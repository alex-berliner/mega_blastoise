extern crate alloc;

use alloc::string::ToString;

use battler::{
    BattleType, CoreBattleEngineOptions, CoreBattleOptions, FormatData, PlayerData, PlayerDex,
    PlayerOptions, PlayerType, SerializedRuleSet, SideData, TeamData,
};

use crate::battle_effects::{process_new_log_lines, BoardEffects, BoardEventQueue};
use crate::battle_input::{ActivePrompt, InputBus};
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

/// Drive a battle to completion: dispatch board events, collect choices via [`InputBus`], advance engine.
///
/// Before blocking on each choice the runner signals [`ActivePrompt`] on `bus.prompt`
/// so that concurrent input sources can display the right prompt (move names, PP, …).
/// Run input sources alongside this with `embassy_futures::join`.
pub async fn run_battle<E, T>(
    battle: &mut battler::PublicCoreBattle<'_>,
    bus: &InputBus,
    queue: &mut BoardEventQueue,
    effects: &mut E,
    mut on_turn: T,
) where
    E: BoardEffects,
    T: FnMut(&mut battler::PublicCoreBattle<'_>),
{
    process_new_log_lines(battle.new_log_entries(), queue, effects);

    while !battle.ended() {
        let mut had_request = false;
        loop {
            let next_request = battle.active_requests().next();
            let Some((player_id, request)) = next_request else {
                break;
            };
            had_request = true;
            queue.push_event(board_prompt_event(&player_id, &request));
            queue.dispatch_all(effects);
            bus.prompt.signal(ActivePrompt {
                player_id: player_id.clone(),
                request: request.clone(),
            });
            let line = bus.choices.receive().await;
            if let Err(e) = battle.set_player_choice(&player_id, &line) {
                #[cfg(feature = "defmt")]
                defmt::error!(
                    "set_player_choice failed ({}): {}",
                    player_id.as_str(),
                    defmt::Display2Format(&e.to_string())
                );
                #[cfg(not(feature = "defmt"))]
                let _ = e;
            }
        }

        if !had_request {
            process_new_log_lines(battle.new_log_entries(), queue, effects);
            continue;
        }

        process_new_log_lines(battle.new_log_entries(), queue, effects);
        on_turn(battle);
    }
}
