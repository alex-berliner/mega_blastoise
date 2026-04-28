#![no_std]
#![no_main]

extern crate alloc;

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
use embedded_alloc::Heap;
use {defmt_rtt as _, panic_probe as _};

mod data_store;
use data_store::FlashDataStore;

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
async fn main(_spawner: Spawner) {
    let _p = embassy_rp::init(Default::default());
    init_heap();

    info!("=== mega-blastoise PoC ===");
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
        .update_team(
            "p1",
            TeamData {
                members: alloc::vec![charizard()],
                ..Default::default()
            },
        )
        .expect("set p1 team");

    battle
        .update_team(
            "p2",
            TeamData {
                members: alloc::vec![blastoise()],
                ..Default::default()
            },
        )
        .expect("set p2 team");

    battle.start().expect("battle start");

    info!("Battle started: Charizard vs Blastoise");

    // Drain the initial log so we see the opening state.
    for entry in battle.new_log_entries() {
        info!("{}", entry);
    }

    while !battle.ended() {
        // Collect pending requests — auto_continue means the battle will advance
        // as soon as every player submits a choice.
        let requests: Vec<(alloc::string::String, Request)> =
            battle.active_requests().collect();

        if requests.is_empty() {
            // All choices submitted; engine is mid-turn processing. Keep draining.
            for entry in battle.new_log_entries() {
                info!("{}", entry);
            }
            continue;
        }

        for (player_id, request) in &requests {
            match request {
                Request::Turn(_) => {
                    // Both players always pick their first move for this PoC.
                    if let Err(e) = battle.set_player_choice(player_id, "move 1") {
                        info!("choice error for {}: {}", player_id.as_str(), defmt::Display2Format(&e));
                    }
                }
                Request::Switch(_) => {
                    // Won't happen with single-Pokémon teams in singles.
                    info!("unexpected switch request for {}", player_id.as_str());
                }
                _ => {}
            }
        }

        for entry in battle.new_log_entries() {
            info!("{}", entry);
        }
    }

    info!("=== Battle over ===");
    loop {
        // Halt — probe-rs will report the defmt output then detach.
        cortex_m::asm::wfi();
    }
}
