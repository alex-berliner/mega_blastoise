#![no_std]
#![no_main]

extern crate alloc;

mod board_effects;
mod pico_battle_input;
mod pn532;
mod subsystems;
mod usb_input;

use battler::TeamData;
use defmt::info;
use embassy_executor::Spawner;
use mega_blastoise_core::{
    demo_battle_options, demo_engine_opts, demo_team_blue, demo_team_red, run_battle,
    BoardEventQueue, FlashDataStore, InputBus, NoInput,
};
use mega_blastoise_fw::mem_profile::{heap_snapshot, init_heap};
use mega_blastoise_fw as _;
use rtt_target::{rtt_init, set_defmt_channel};

use board_effects::BattleEffects;

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
        info!("USB ready. Connect with: picocom -b 115200 /dev/ttyACM1");
        #[cfg(feature = "mem-profile")]
        heap_snapshot("after_usb_init");
        input
    };

    // ── NFC readers (PN532 over I²C) ─────────────────────────────────────────
    #[cfg(feature = "nfc")]
    {
        subsystems::nfc::init(
            p.I2C0, p.I2C1,
            p.PIN_16, p.PIN_17, p.PIN_18, p.PIN_19,
            &spawner,
        );
        info!("NFC readers started (I2C0: GP16/17, I2C1: GP18/19, addr 0x24)");
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

    info!("Initialising data store...");
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

    info!("Battle started.");

    // ── Run ───────────────────────────────────────────────────────────────────
    #[cfg(feature = "usb")]
    let mut input = usb_input;
    #[cfg(not(feature = "usb"))]
    let mut input = NoInput;

    run_battle(&mut battle, &bus, &mut input, &mut queue, &mut effects, |_| {
        #[cfg(feature = "mem-profile")]
        heap_snapshot("after_turn");
    })
    .await;

    info!("=== Battle over ===");
    loop { cortex_m::asm::wfi(); }
}
