//! Shared battle setup and harness entrypoints.

use battler::TeamData;
use mega_blastoise_core::{
    demo_battle_options, demo_engine_opts, draw_randbat_team, run_battle,
    BoardEventQueue, FlashDataStore, InputBus, InputSource,
};

use crate::host_battle_controller::HostBattleController;
use crate::host_battle_effects::HostBattleEffects;

fn print_active_pokemon_state(battle: &mut battler::PublicCoreBattle<'_>) {
    println!("── Active Pokémon ──");
    for pid in ["p1", "p2"] {
        let Ok(data) = battle.player_data(pid) else {
            continue;
        };
        let actives: Vec<_> = data.mons.iter().filter(|m| m.active).collect();
        if actives.is_empty() {
            println!("  {}: (none on field)", data.name);
            continue;
        }
        for m in actives {
            let status = m.status.clone().unwrap_or_else(|| "—".into());
            let types = m
                .types
                .iter()
                .map(|t| format!("{t:?}"))
                .collect::<Vec<_>>()
                .join("/");
            println!(
                "  {} — {} ({})  HP {}/{} ({})  status: {}  types: [{}]",
                data.name, m.summary.name, m.species, m.hp, m.max_hp, m.health, status, types
            );
            println!("    ability: {}  item: {}", m.ability, m.item.as_deref().unwrap_or("—"));
            for mv in &m.moves {
                let dis = if mv.disabled { " (disabled)" } else { "" };
                println!("    • {}  {}/{} PP{}", mv.name, mv.pp, mv.max_pp, dis);
            }
        }
    }
    println!();
}

pub fn run_interactive() {
    let data = FlashDataStore::new();
    let mut controller = HostBattleController::new();
    let mut queue = BoardEventQueue::new();

    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0xdeadbeef);

    let team_red = draw_randbat_team(seed, 3);
    let team_blue = draw_randbat_team(seed.wrapping_add(0x9e3779b97f4a7c15), 3);

    let mut battle =
        battler::PublicCoreBattle::new(demo_battle_options(), &data, demo_engine_opts())
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
