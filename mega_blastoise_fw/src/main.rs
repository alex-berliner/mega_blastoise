#![no_std]
#![no_main]

extern crate alloc;

use alloc::boxed::Box;

mod board_effects;
mod pico_battle_input;
mod pn532;

use alloc::{string::ToString, vec::Vec};

use battler::{
    BattleType,
    CoreBattleEngineOptions,
    CoreBattleOptions,
    FormatData,
    MonData,
    PlayerData,
    PlayerDex,
    PlayerOptions,
    PlayerType,
    Request,
    SerializedRuleSet,
    SideData,
    TeamData,
};
use defmt::info;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Input, Pull};
use embassy_rp::i2c::{Async, InterruptHandler, I2c};
use embassy_rp::peripherals::{I2C0, I2C1};
use embedded_alloc::Heap;
use board_effects::DefmtBattleEffects;
use mega_blastoise_core::{for_each_new_log_line, BattleInput, FlashDataStore};
use pico_battle_input::PicoBattleInput;
use {defmt_rtt as _, panic_probe as _};

#[global_allocator]
static HEAP: Heap = Heap::empty();

fn init_heap() {
    const HEAP_SIZE: usize = 128 * 1024;
    static mut HEAP_MEM: [u8; HEAP_SIZE] = [0u8; HEAP_SIZE];
    unsafe { HEAP.init(core::ptr::addr_of!(HEAP_MEM) as usize, HEAP_SIZE) }
}

fn charizard() -> MonData {
    MonData {
        name: "Charizard".to_string(),
        species: "Charizard".to_string(),
        ability: "No Ability".to_string(),
        moves: ["Flamethrower", "Earthquake", "Slash", "Wing Attack"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        level: 50,
        ..Default::default()
    }
}

fn blastoise() -> MonData {
    MonData {
        name: "Blastoise".to_string(),
        species: "Blastoise".to_string(),
        ability: "No Ability".to_string(),
        moves: ["Surf", "Ice Beam", "Body Slam", "Submission"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        level: 50,
        ..Default::default()
    }
}

bind_interrupts!(struct Irqs {
    I2C0_IRQ => InterruptHandler<I2C0>;
    I2C1_IRQ => InterruptHandler<I2C1>;
});

#[embassy_executor::task]
async fn pn532_task_i2c0(bus: &'static mut I2c<'static, I2C0, Async>) {
    pn532::reader_loop(0, bus).await
}

#[embassy_executor::task]
async fn pn532_task_i2c1(bus: &'static mut I2c<'static, I2C1, Async>) {
    pn532::reader_loop(1, bus).await
}

fn player(id: &str, name: &str) -> PlayerData {
    PlayerData {
        id: id.to_string(),
        name: name.to_string(),
        player_type: PlayerType::Trainer,
        player_options: PlayerOptions::default(),
        team: TeamData::default(),
        dex: PlayerDex::default(),
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    init_heap();

    // Embassy order: SCL, then SDA (see `embassy_rp::i2c::I2c::new_async`).
    let i2c0 = I2c::new_async(p.I2C0, p.PIN_17, p.PIN_16, Irqs, pn532::i2c_config());
    let i2c1 = I2c::new_async(p.I2C1, p.PIN_19, p.PIN_18, Irqs, pn532::i2c_config());
    let i2c0: &'static mut I2c<'static, I2C0, Async> = Box::leak(Box::new(i2c0));
    let i2c1: &'static mut I2c<'static, I2C1, Async> = Box::leak(Box::new(i2c1));
    spawner.spawn(pn532_task_i2c0(i2c0)).unwrap();
    spawner.spawn(pn532_task_i2c1(i2c1)).unwrap();
    info!("PN532 tasks started (I2C0: GP16/GP17, I2C1: GP18/GP19, addr 0x24)");

    // Move buttons GPIO 6–9 (protocol slots 0–3); switch GPIO 10–15 (party 0–5).
    let move_pins = [
        Input::new(p.PIN_6, Pull::Up),
        Input::new(p.PIN_7, Pull::Up),
        Input::new(p.PIN_8, Pull::Up),
        Input::new(p.PIN_9, Pull::Up),
    ];
    let switch_pins = [
        Input::new(p.PIN_10, Pull::Up),
        Input::new(p.PIN_11, Pull::Up),
        Input::new(p.PIN_12, Pull::Up),
        Input::new(p.PIN_13, Pull::Up),
        Input::new(p.PIN_14, Pull::Up),
        Input::new(p.PIN_15, Pull::Up),
    ];
    let mut input = PicoBattleInput::new(move_pins, switch_pins);
    let mut effects = DefmtBattleEffects::new();

    info!("=== mega-blastoise PoC (GPIO + 2× PN532 I²C) ===");
    info!("Initialising data store...");

    let data = FlashDataStore::new();

    let options = CoreBattleOptions {
        seed: Some(12345),
        format: FormatData {
            battle_type: BattleType::Singles,
            rules: SerializedRuleSet::new(),
        },
        field: Default::default(),
        side_1: SideData {
            name: "Red".to_string(),
            players: alloc::vec![player("p1", "Red")],
        },
        side_2: SideData {
            name: "Blue".to_string(),
            players: alloc::vec![player("p2", "Blue")],
        },
    };

    let engine_opts = CoreBattleEngineOptions {
        validate_teams: false,
        auto_continue: true,
        reveal_actual_health: true,
        log_time: false,
        ..Default::default()
    };

    let mut battle =
        battler::PublicCoreBattle::new(options, &data, engine_opts).expect("battle init");

    battle
        .update_team("p1", TeamData { members: alloc::vec![charizard()], ..Default::default() })
        .expect("set p1 team");

    battle
        .update_team("p2", TeamData { members: alloc::vec![blastoise()], ..Default::default() })
        .expect("set p2 team");

    battle.start().expect("battle start");
    info!("Battle started — press GPIO move/switch buttons when prompted in logs.");

    for_each_new_log_line(battle.new_log_entries(), &mut effects);

    while !battle.ended() {
        let requests: Vec<(alloc::string::String, Request)> =
            battle.active_requests().collect();

        if requests.is_empty() {
            for_each_new_log_line(battle.new_log_entries(), &mut effects);
            continue;
        }

        for (player_id, request) in &requests {
            match request {
                Request::Turn(_) => info!("Player {}: press move button [GPIO 6-9]", player_id),
                Request::Switch(_) => info!("Player {}: press switch [GPIO 10-15]", player_id),
                _ => {}
            }
            let line = input.read_choice(player_id, request);
            if let Err(e) = battle.set_player_choice(player_id, &line) {
                info!(
                    "choice error for {}: {}",
                    player_id.as_str(),
                    defmt::Display2Format(&e)
                );
            }
        }

        for_each_new_log_line(battle.new_log_entries(), &mut effects);
    }

    info!("=== Battle over ===");
    loop {
        cortex_m::asm::wfi();
    }
}
