//! Shared battle setup and harness entrypoints (interactive run vs scripted effect smoke test).

use battler::{
    BattleType,
    CoreBattleEngineOptions,
    CoreBattleOptions,
    FormatData,
    PlayerData,
    PlayerDex,
    PlayerOptions,
    PlayerType,
    SerializedRuleSet,
    SideData,
    TeamData,
};
use mega_blastoise_core::{
    board_prompt_event, demo_team_blue, demo_team_red, parse_log_line, BattleInput, BoardEvent,
    BoardEventQueue, FlashDataStore,
};

use crate::board_game_effects::BoardGameEffects;
use crate::stdin_input::StdinBattleInput;

/// After log lines are drained: if a **Turn** event was in this batch, print each side’s active Pokémon.
fn process_logs_and_turn_snapshot(
    battle: &mut battler::PublicCoreBattle<'_>,
    queue: &mut BoardEventQueue,
    effects: &mut BoardGameEffects,
) {
    let lines: Vec<&str> = battle.new_log_entries().collect();
    let saw_turn = lines.iter().any(|line| {
        matches!(
            parse_log_line(line),
            Some(BoardEvent::Turn { .. })
        )
    });
    queue.push_log_lines(lines.into_iter());
    queue.dispatch_all(effects);
    if saw_turn {
        print_active_pokemon_state(battle);
    }
}

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
                data.name,
                m.summary.name,
                m.species,
                m.hp,
                m.max_hp,
                m.health,
                status,
                types
            );
            println!(
                "    ability: {}  item: {}",
                m.ability,
                m.item.as_deref().unwrap_or("—")
            );
            for mv in &m.moves {
                let dis = if mv.disabled { " (disabled)" } else { "" };
                println!(
                    "    • {}  {}/{} PP{}",
                    mv.name, mv.pp, mv.max_pp, dis
                );
            }
        }
    }
    println!();
}

// --- Shared wiring so interactive + tests stay aligned ---

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

/// stdin vs GPIO battle loop (same event path as firmware).
pub fn run_interactive() {
    let data = FlashDataStore::new();
    let mut input = StdinBattleInput;
    let mut board_effects = BoardGameEffects::new();
    let mut queue = BoardEventQueue::new();

    let options = CoreBattleOptions {
        seed: Some(12345),
        format: FormatData {
            battle_type: BattleType::Singles,
            rules: SerializedRuleSet::new(),
        },
        field: Default::default(),
        side_1: SideData {
            name: "Red".to_string(),
            players: vec![player("p1", "Red")],
        },
        side_2: SideData {
            name: "Blue".to_string(),
            players: vec![player("p2", "Blue")],
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
                members: demo_team_red(),
                ..Default::default()
            },
        )
        .expect("set p1 team");
    battle
        .update_team(
            "p2",
            TeamData {
                members: demo_team_blue(),
                ..Default::default()
            },
        )
        .expect("set p2 team");

    battle.start().expect("battle start");
    println!("=== Demo battle (4v4 teams, singles field) ===\n");
    println!(
        "Each side has four Pokémon — slot 1 is your lead. Pick moves each turn; switches use bench slots 1–6.\n"
    );

    process_logs_and_turn_snapshot(&mut battle, &mut queue, &mut board_effects);

    while !battle.ended() {
        let requests: Vec<(String, battler::Request)> = battle.active_requests().collect();

        if requests.is_empty() {
            process_logs_and_turn_snapshot(&mut battle, &mut queue, &mut board_effects);
            continue;
        }

        for (player_id, request) in &requests {
            queue.push_event(board_prompt_event(player_id, request));
            queue.dispatch_all(&mut board_effects);

            let line = input.read_choice(player_id, request);
            if let Err(e) = battle.set_player_choice(player_id, &line) {
                eprintln!("choice error for {player_id}: {e}");
            }
        }

        process_logs_and_turn_snapshot(&mut battle, &mut queue, &mut board_effects);
    }

    println!("\n=== Battle over ===");
}
