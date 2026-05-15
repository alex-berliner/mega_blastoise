//! Shared battle setup and harness entrypoints.

use gen1_battle::TeamData;
use mega_blastoise_core::{
    demo_battle_options, demo_engine_opts, draw_two_randbat_teams, format_active_state, run_battle,
    BoardEventQueue, FlashDataStore, InputBus, InputSource,
};

use crate::host_battle_controller::HostBattleController;
use crate::host_battle_effects::HostBattleEffects;

fn print_active_pokemon_state(battle: &mut gen1_battle::PublicCoreBattle<'_>) {
    print!("{}", format_active_state(battle));
}

pub fn run_interactive() {
    let data = FlashDataStore::new();
    let mut controller = HostBattleController::new();
    let mut queue = BoardEventQueue::new();

    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0xdeadbeef);

    let (team_red, team_blue) = draw_two_randbat_teams(seed, 3);

    let mut battle =
        gen1_battle::PublicCoreBattle::new(demo_battle_options(), &data, demo_engine_opts())
            .expect("battle init");

    battle
        .update_team("p1", TeamData { members: team_red, ..Default::default() })
        .expect("set p1 team");
    battle
        .update_team("p2", TeamData { members: team_blue, ..Default::default() })
        .expect("set p2 team");

    battle.start().expect("battle start");
    println!("=== Randbat (3v3 singles) ===\n");
    println!("Each side has three random Gen 1 Pokémon. Pick moves each turn; switches use bench slots.\n");

    let bus = InputBus::new();
    let mut effects = HostBattleEffects::new(Some(&bus));
    pollster::block_on(run_battle(
        &mut battle,
        &data,
        &bus,
        controller.run(&bus),
        &mut queue,
        &mut effects,
        |b| print_active_pokemon_state(b),
    ));

    println!("\n=== Battle over ===");
}
