mod board_game_effects;
mod stdin_input;

use board_game_effects::BoardGameEffects;
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
use mega_blastoise_core::{for_each_new_log_line, BattleInput, FlashDataStore};
use stdin_input::StdinBattleInput;

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

fn main() {
    let data = FlashDataStore::new();
    let mut input = StdinBattleInput;
    let mut board_effects = BoardGameEffects::new();

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
    println!("On each turn, both players pick a move. For forced switches, pick bench slot 1-6.\n");

    for_each_new_log_line(battle.new_log_entries(), &mut board_effects);

    while !battle.ended() {
        let requests: Vec<(String, Request)> = battle.active_requests().collect();

        if requests.is_empty() {
            for_each_new_log_line(battle.new_log_entries(), &mut board_effects);
            continue;
        }

        for (player_id, request) in &requests {
            let line = input.read_choice(player_id, request);
            if let Err(e) = battle.set_player_choice(player_id, &line) {
                eprintln!("choice error for {player_id}: {e}");
            }
        }

        for_each_new_log_line(battle.new_log_entries(), &mut board_effects);
    }

    println!("\n=== Battle over ===");
}
