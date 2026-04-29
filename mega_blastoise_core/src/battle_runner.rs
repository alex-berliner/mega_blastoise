extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use battler::{
    BattleType, CoreBattleEngineOptions, CoreBattleOptions, FormatData, PlayerData, PlayerDex,
    PlayerOptions, PlayerType, SerializedRuleSet, SideData, TeamData,
};

use crate::battle_effects::{process_new_log_lines, BoardEffects, BoardEventQueue};
use crate::battle_input::BattleInput;
use crate::board_event::board_prompt_event;

pub fn make_player(id: &str, name: &str) -> PlayerData {
    PlayerData {
        id: id.to_string(),
        name: name.to_string(),
        player_type: PlayerType::Trainer,
        player_options: PlayerOptions::default(),
        team: TeamData::default(),
        dex: PlayerDex::default(),
    }
}

pub fn demo_battle_options() -> CoreBattleOptions {
    CoreBattleOptions {
        seed: Some(12345),
        format: FormatData {
            battle_type: BattleType::Singles,
            rules: SerializedRuleSet::new(),
        },
        field: Default::default(),
        side_1: SideData {
            name: "Red".to_string(),
            players: alloc::vec![make_player("p1", "Red")],
        },
        side_2: SideData {
            name: "Blue".to_string(),
            players: alloc::vec![make_player("p2", "Blue")],
        },
    }
}

pub fn demo_engine_opts() -> CoreBattleEngineOptions {
    CoreBattleEngineOptions {
        validate_teams: false,
        auto_continue: true,
        reveal_actual_health: true,
        log_time: false,
        ..Default::default()
    }
}

/// Drive a battle to completion: dispatch board events, read choices, advance engine.
///
/// Each side's prompt + log events go through `queue` → `effects` so both firmware
/// (defmt) and the host test harness (println) see the same event stream.
pub fn run_battle<I, E>(
    battle: &mut battler::PublicCoreBattle<'_>,
    input: &mut I,
    queue: &mut BoardEventQueue,
    effects: &mut E,
) where
    I: BattleInput,
    E: BoardEffects,
{
    process_new_log_lines(battle.new_log_entries(), queue, effects);

    while !battle.ended() {
        let requests: Vec<(String, battler::Request)> = battle.active_requests().collect();

        if requests.is_empty() {
            process_new_log_lines(battle.new_log_entries(), queue, effects);
            continue;
        }

        for (player_id, request) in &requests {
            queue.push_event(board_prompt_event(player_id, request));
            queue.dispatch_all(effects);
            let line = input.read_choice(player_id, request);
            let _ = battle.set_player_choice(player_id, &line);
        }

        process_new_log_lines(battle.new_log_entries(), queue, effects);
    }
}
