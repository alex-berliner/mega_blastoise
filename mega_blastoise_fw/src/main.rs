#![no_std]
#![no_main]

extern crate alloc;

mod battle_controller;
mod battle_effects;
mod pico_battle_input;
mod pn532;
mod subsystems;
mod usb_input;

use battler::TeamData;
use defmt::debug;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Input, Pull};
use mega_blastoise_core::{
    demo_battle_options, demo_engine_opts, demo_team_blue, demo_team_red, run_battle,
    BoardEventQueue, FlashDataStore, InputBus, InputSource,
};
use mega_blastoise_fw::mem_profile::{heap_snapshot, init_heap};
use mega_blastoise_fw as _;
use rtt_target::{rtt_init, set_defmt_channel};

use battle_controller::BattleController;
use battle_effects::BattleEffects;
use pico_battle_input::PicoBattleInput;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    init_heap();
    #[cfg(feature = "mem-profile")]
    heap_snapshot("boot");

    let channels = rtt_init! {
        up: { 0: { size: 1024, name: "defmt" } }
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

    // ── Physical buttons (GPIO 6–9 moves, 10–15 switch) ──────────────────────
    let buttons = PicoBattleInput::new(
        [
            Input::new(p.PIN_6,  Pull::Up),
            Input::new(p.PIN_7,  Pull::Up),
            Input::new(p.PIN_8,  Pull::Up),
            Input::new(p.PIN_9,  Pull::Up),
        ],
        [
            Input::new(p.PIN_10, Pull::Up),
            Input::new(p.PIN_11, Pull::Up),
            Input::new(p.PIN_12, Pull::Up),
            Input::new(p.PIN_13, Pull::Up),
            Input::new(p.PIN_14, Pull::Up),
            Input::new(p.PIN_15, Pull::Up),
        ],
    );
    debug!("Buttons ready: move GPIO 6-9, switch GPIO 10-15");

    // ── NFC readers (PN532 over I²C) ─────────────────────────────────────────
    #[cfg(feature = "nfc")]
    {
        subsystems::nfc::init(
            p.I2C0, p.I2C1,
            p.PIN_16, p.PIN_17, p.PIN_18, p.PIN_19,
            &spawner,
        );
        debug!("NFC readers started (I2C0: GP16/17, I2C1: GP18/19, addr 0x24)");
        #[cfg(feature = "mem-profile")]
        heap_snapshot("after_nfc_init");
    }

    // ── Battle engine ─────────────────────────────────────────────────────────
    let bus = InputBus::new();
    let mut queue = BoardEventQueue::new();

    let mut effects = BattleEffects::new(
        #[cfg(feature = "usb")] Some(&bus),
        #[cfg(not(feature = "usb"))] None,
    );

    debug!("Initialising data store...");
    let data = FlashDataStore::new();
    #[cfg(feature = "mem-profile")]
    heap_snapshot("after_datastore");

    let mut battle =
        battler::PublicCoreBattle::new(demo_battle_options(), &data, demo_engine_opts())
            .expect("battle init");
    #[cfg(feature = "mem-profile")]
    heap_snapshot("after_battle_new");

    battle.update_team("p1", TeamData { members: demo_team_red(),  ..Default::default() }).expect("p1");
    #[cfg(feature = "mem-profile")]
    heap_snapshot("after_team_p1");

    battle.update_team("p2", TeamData { members: demo_team_blue(), ..Default::default() }).expect("p2");
    #[cfg(feature = "mem-profile")]
    heap_snapshot("after_team_p2");

    battle.start().expect("battle start");
    #[cfg(feature = "mem-profile")]
    heap_snapshot("after_battle_start");

    debug!("Battle started.");

    // ── Run ───────────────────────────────────────────────────────────────────
    #[cfg(feature = "usb")]
    {
        let mut controller = BattleController::new(usb_input, buttons);
        run_battle(&mut battle, &bus, controller.run(&bus), &mut queue, &mut effects, |_| {
            #[cfg(feature = "mem-profile")]
            heap_snapshot("after_turn");
        })
        .await;
    }

    #[cfg(not(feature = "usb"))]
    run_battle(&mut battle, &bus, buttons.run(&bus), &mut queue, &mut effects, |_| {
        #[cfg(feature = "mem-profile")]
        heap_snapshot("after_turn");
    })
    .await;

    debug!("=== Battle over ===");
    loop { cortex_m::asm::wfi(); }
}
