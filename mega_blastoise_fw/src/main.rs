#![no_std]
#![no_main]

extern crate alloc;

mod battle_controller;
mod battle_effects;
mod lobby;
mod pico_battle_input;
mod subsystems;
mod usb_input;

use gen1_battle::TeamData;
use defmt::debug;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Flex};
use embassy_time::{Instant, Timer};
use mega_blastoise_core::{
    battle_options_with_seed, demo_engine_opts, draw_two_randbat_teams, format_active_state, run_battle,
    BoardEventQueue, FlashDataStore, InputBus, InputSource,
};
use mega_blastoise_fw::mem_profile::init_heap;
#[cfg(feature = "mem-profile")]
use mega_blastoise_fw::mem_profile::heap_snapshot;
use mega_blastoise_fw as _;
use rtt_target::{rtt_init, set_defmt_channel};

use battle_controller::BattleController;
use battle_effects::BattleEffects;
use lobby::{run_lobby, LobbyResult};
use pico_battle_input::PicoBattleInput;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    init_heap();
    #[cfg(feature = "mem-profile")]
    heap_snapshot("boot");

    // RTT up-buffer sized to capture a whole AI battle's OLED-framebuffer dump
    // (`oledfb|pN|<2 KB hex>` per frame change). gen1_battle's real heap peak is
    // ~3 KB (measured) so the 64 KB heap is far oversized; RAM is 256 KB, so we
    // spend the abundant free RAM on a large RTT buffer instead. Default
    // NoBlockSkip mode is kept deliberately: BlockIfFull throttles the firmware
    // to a crawl (battle can't finish in real time over RTT). A 96 KB buffer
    // absorbs the OLED burst between probe-rs drains so few/no frames drop.
    let channels = rtt_init! {
        up: { 0: { size: 96 * 1024, name: "defmt" } }
    };
    set_defmt_channel(channels.up.0);

    // ── USB CDC battle CLI ────────────────────────────────────────────────────
    #[cfg(feature = "usb")]
    let mut usb_input = {
        let input = subsystems::usb::init(p.USB, &spawner);
        debug!("USB ready. Connect with: picocom --echo -b 115200 /dev/ttyACM1");
        #[cfg(feature = "mem-profile")]
        heap_snapshot("after_usb_init");
        input
    };

    // ── Button matrix (see pico_battle_input's board tables) ─────────────────
    // Default: partner PCB — drives GP6/7/8/9, senses GP10-12 + GP9
    // (GP9/GP13 are one net on that board; GP13 stays unused).
    // `stripboard` feature: the hand-wired board — drives GP5/7/8/9,
    // senses GP10-13.
    #[cfg(feature = "stripboard")]
    let mut buttons = PicoBattleInput::new([
        Flex::new(p.PIN_5),
        Flex::new(p.PIN_7),
        Flex::new(p.PIN_8),
        Flex::new(p.PIN_9),
        Flex::new(p.PIN_10),
        Flex::new(p.PIN_11),
        Flex::new(p.PIN_12),
        Flex::new(p.PIN_13),
    ]);
    #[cfg(not(feature = "stripboard"))]
    let mut buttons = PicoBattleInput::new([
        Flex::new(p.PIN_6),
        Flex::new(p.PIN_7),
        Flex::new(p.PIN_8),
        Flex::new(p.PIN_9),
        Flex::new(p.PIN_10),
        Flex::new(p.PIN_11),
        Flex::new(p.PIN_12),
    ]);
    debug!("Button matrix ready");

    // ── Piezo buzzer (GP21 = PWM slice 2 channel B) ──────────────────────────
    #[cfg(feature = "buzzer")]
    {
        spawner.spawn(subsystems::buzzer::task(p.PWM_SLICE2, p.PIN_21))
            .expect("buzzer task spawn");
        debug!("Buzzer ready: GP21 / PWM2B");
    }

    // ── NeoPixel LED strips (WS2812B: P1 GP20/SM0/DMA0, P2 GP22/SM1/DMA1) ────
    #[cfg(feature = "leds")]
    {
        spawner.spawn(subsystems::led::task(
            p.PIO0, p.PIN_20, p.PIN_22, p.DMA_CH0, p.DMA_CH1,
        ))
        .expect("led task spawn");
        debug!("LEDs ready: P1 GP20, P2 GP22 / PIO0 SM0+SM1 / DMA0+DMA1");
    }

    // ── OLED displays (SSD1306 on I2C0 + I2C1) ───────────────────────────────
    #[cfg(feature = "oled")]
    {
        spawner.spawn(subsystems::oled::task(
            p.I2C0, p.PIN_17, p.PIN_16,  // I2C0: SCL=GP17, SDA=GP16 → P1 OLED
            p.I2C1, p.PIN_19, p.PIN_18,  // I2C1: SCL=GP19, SDA=GP18 → P2 OLED
        ))
        .expect("oled task spawn");
        debug!("OLEDs ready: I2C0 GP16/17, I2C1 GP18/19");
        #[cfg(feature = "mem-profile")]
        heap_snapshot("after_oled_init");
    }

    // ── Shared battle infrastructure ──────────────────────────────────────────
    let bus = InputBus::new();
    let mut queue = BoardEventQueue::new();

    debug!("Initialising data store...");
    let data = FlashDataStore::new();
    #[cfg(feature = "mem-profile")]
    heap_snapshot("after_datastore");

    // ── Game loop: lobby → battle → lobby → … ────────────────────────────────
    loop {
        // Lobby: demo AI battle plays until a player presses ready, then countdown.
        #[cfg(feature = "usb")]
        let LobbyResult { ai_players, team_p1: up_p1, team_p2: up_p2 } =
            run_lobby(&mut buttons, &mut usb_input, &data, &mut queue).await;
        #[cfg(not(feature = "usb"))]
        let LobbyResult { ai_players, team_p1: up_p1, team_p2: up_p2 } =
            run_lobby(&mut buttons, &data, &mut queue).await;

        queue.drain_pending(); // discard any demo events still queued

        #[cfg(feature = "usb")]
        {
            let seed = Instant::now().as_ticks();
            usb_input.set_ai_players(ai_players, seed);
        }

        let _ = ai_players; // used above under #[cfg(feature = "usb")]

        // Draw teams from timing jitter (fresh entropy each round).
        let seed = Instant::now().as_ticks();

        let mut effects = BattleEffects::new(
            #[cfg(feature = "usb")] Some(&bus),
            #[cfg(not(feature = "usb"))] None,
            true,
        );

        let mut battle =
            gen1_battle::PublicCoreBattle::new(battle_options_with_seed(seed), &data, demo_engine_opts())
                .expect("battle init");
        #[cfg(feature = "mem-profile")]
        heap_snapshot("after_battle_new");

        // Use uploaded test teams when provided, else draw random ones.
        let (rand_p1, rand_p2) = draw_two_randbat_teams(seed, 3);
        let team_p1 = up_p1.unwrap_or(rand_p1);
        let team_p2 = up_p2.unwrap_or(rand_p2);
        #[cfg(feature = "leds")]
        let (p1_team_size, p2_team_size) = (team_p1.len() as u8, team_p2.len() as u8);
        battle.update_team("p1", TeamData { members: team_p1, ..Default::default() }).expect("p1");
        #[cfg(feature = "mem-profile")]
        heap_snapshot("after_team_p1");

        battle.update_team("p2", TeamData { members: team_p2, ..Default::default() }).expect("p2");
        #[cfg(feature = "mem-profile")]
        heap_snapshot("after_team_p2");

        battle.start().expect("battle start");
        #[cfg(feature = "mem-profile")]
        heap_snapshot("after_battle_start");

        debug!("Battle started.");

        // Light each player's full team green for the new battle (resets any
        // stale per-member state from the previous round).
        #[cfg(feature = "leds")]
        {
            use subsystems::led::{send as led_send, LedCmd};
            led_send(LedCmd::TeamInit { player: 1, size: p1_team_size });
            led_send(LedCmd::TeamInit { player: 2, size: p2_team_size });
        }

        #[cfg(feature = "usb")]
        {
            let mut controller = BattleController::new(usb_input, buttons);
            run_battle(&mut battle, &data, &bus, controller.run(&bus), &mut queue, &mut effects, |b| {
                for line in format_active_state(b).lines() {
                    let _ = bus.log.try_send(alloc::string::String::from(line));
                }
                #[cfg(feature = "mem-profile")]
                heap_snapshot("after_turn");
                for (action_type, ms) in b.drain_action_timings() {
                    defmt::info!("  action[{}]: {}ms", action_type, ms);
                }
            })
            .await;
            // Recover ownership of usb_input and buttons from the controller.
            let (u, b) = controller.into_parts();
            usb_input = u;
            buttons = b;
            usb_input.set_ai_players([false, false], 0);
        }

        #[cfg(not(feature = "usb"))]
        run_battle(&mut battle, &data, &bus, buttons.run(&bus), &mut queue, &mut effects, |b| {
            #[cfg(feature = "mem-profile")]
            heap_snapshot("after_turn");
            for (action_type, ms) in b.drain_action_timings() {
                defmt::info!("  action[{}]: {}ms", action_type, ms);
            }
        })
        .await;

        debug!("=== Battle over ===");
        // Brief pause so win effects finish before the lobby resets the LEDs.
        Timer::after_secs(4).await;
    }
}
