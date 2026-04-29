//! Shared battle setup and harness entrypoints (interactive run vs scripted effect smoke test).

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
    SerializedRuleSet,
    SideData,
    TeamData,
};
use mega_blastoise_core::{
    board_prompt_event, process_new_log_lines, BattleInput, BoardEvent, BoardEventQueue,
    FlashDataStore, PromptKind,
};

use crate::board_game_effects::BoardGameEffects;
use crate::stdin_input::StdinBattleInput;

// --- Shared wiring so interactive + tests stay aligned ---

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
        .update_team("p1", TeamData { members: vec![charizard()], ..Default::default() })
        .expect("set p1 team");
    battle
        .update_team("p2", TeamData { members: vec![blastoise()], ..Default::default() })
        .expect("set p2 team");

    battle.start().expect("battle start");
    println!("=== Charizard vs Blastoise (interactive) ===\n");
    println!("On each turn, both players pick a move. Forced switches: bench slot 1–6.\n");

    process_new_log_lines(battle.new_log_entries(), &mut queue, &mut board_effects);

    while !battle.ended() {
        let requests: Vec<(String, battler::Request)> = battle.active_requests().collect();

        if requests.is_empty() {
            process_new_log_lines(battle.new_log_entries(), &mut queue, &mut board_effects);
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

        process_new_log_lines(battle.new_log_entries(), &mut queue, &mut board_effects);
    }

    println!("\n=== Battle over ===");
}

/// Feed canned [`BoardEvent`]s through the same sink (board “drives itself” / regression smoke).
pub fn run_self_test_effects() {
    let mut queue = BoardEventQueue::new();
    let mut sink = BoardGameEffects::new();

    let script = [
        BoardEvent::BattleStart,
        BoardEvent::Prompt {
            player_id: "p1".into(),
            kind: PromptKind::ChooseMove,
        },
        BoardEvent::Move {
            name: "Flamethrower".into(),
        },
        BoardEvent::Damage {
            mon: "b:0 Blastoise".into(),
            health: "120/201".into(),
        },
        BoardEvent::Turn { n: 2 },
        BoardEvent::Win { side: Some("0".into()) },
    ];

    for e in script {
        queue.push_event(e);
    }
    queue.dispatch_all(&mut sink);
    println!("(end of --self-test scripted events)");
}
