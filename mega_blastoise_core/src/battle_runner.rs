extern crate alloc;

use alloc::string::ToString;

use battler::{
    BattleType, CoreBattleEngineOptions, CoreBattleOptions, FormatData, PlayerData, PlayerDex,
    PlayerOptions, PlayerType, SerializedRuleSet, SideData, TeamData,
};
use battler_data::DataStore;

use core::future::Future;

use embassy_futures::select::{select, Either};

use crate::battle_effects::{BoardEffects, BoardEventQueue};
use crate::battle_input::{ActivePrompt, InputBus};
use crate::board_event::{board_prompt_event, BoardEvent, MoveSlot};

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

/// Like [`demo_battle_options`] but uses `seed` for the battle-engine PRNG.
pub fn battle_options_with_seed(seed: u64) -> CoreBattleOptions {
    CoreBattleOptions {
        seed: Some(seed),
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

/// Query the full move list for the currently active Pokémon of `player_id`.
///
/// Pulls live PP from battle state and base stats (power, accuracy, category) from the
/// data store — the same source of truth the engine uses.
fn query_active_moves<DS: DataStore>(
    battle: &mut battler::PublicCoreBattle<'_>,
    data: &DS,
    player_id: &str,
) -> alloc::vec::Vec<MoveSlot> {
    let Ok(player_data) = battle.player_data(player_id) else {
        return alloc::vec::Vec::new();
    };
    let Some(active_mon) = player_data.mons.into_iter().find(|m| m.active) else {
        return alloc::vec::Vec::new();
    };
    active_mon
        .moves
        .into_iter()
        .map(|mv| {
            let (power, accuracy, category) = data
                .get_move(&mv.id)
                .ok()
                .flatten()
                .map(|md| {
                    (
                        if md.base_power > 0 { Some(md.base_power) } else { None },
                        md.accuracy.percentage(),
                        alloc::format!("{:?}", md.category),
                    )
                })
                .unwrap_or((None, None, alloc::string::String::from("?")));
            MoveSlot {
                name: mv.name,
                type_name: alloc::format!("{:?}", mv.typ),
                category,
                power,
                accuracy,
                pp: mv.pp,
                max_pp: mv.max_pp,
            }
        })
        .collect()
}

/// Drain new log entries, enrich `SwitchIn` with move data, inject `MovesUpdate`
/// after every `Move` event, then dispatch everything to `effects`.
fn enrich_and_dispatch<E, DS>(
    battle: &mut battler::PublicCoreBattle<'_>,
    data: &DS,
    queue: &mut BoardEventQueue,
    effects: &mut E,
) where
    E: BoardEffects,
    DS: DataStore,
{
    // Collect into owned strings first so the &mut borrow on battle ends.
    let entries: alloc::vec::Vec<alloc::string::String> =
        battle.new_log_entries().map(alloc::string::String::from).collect();
    queue.push_log_lines(entries.iter().map(alloc::string::String::as_str));

    let raw = queue.drain_pending();
    for event in raw {
        // Decide whether to inject a MovesUpdate after this event.
        let inject = match &event {
            BoardEvent::Move { player_id: Some(pid), .. } => {
                let moves = query_active_moves(battle, data, pid.as_str());
                Some((pid.clone(), moves))
            }
            _ => None,
        };

        // Enrich SwitchIn with move list from live battle state.
        let enriched = match event {
            BoardEvent::SwitchIn { name, species, player_id, moves } if moves.is_empty() => {
                let new_moves = player_id
                    .as_deref()
                    .map(|pid| query_active_moves(battle, data, pid))
                    .unwrap_or_default();
                BoardEvent::SwitchIn { name, species, player_id, moves: new_moves }
            }
            other => other,
        };

        queue.push_event(enriched);

        if let Some((pid, moves)) = inject {
            queue.push_event(BoardEvent::MovesUpdate { player_id: pid, moves });
        }
    }

    queue.dispatch_all(effects);
}

/// Drive a battle to completion, running `inputs` concurrently for the duration.
///
/// `data` is the game data store used to enrich move slots with power/accuracy/type.
///
/// `inputs` is any future that drives your input sources — a single `source.run(&bus)`,
/// or multiple sources composed with `embassy_futures::join`:
///
/// ```ignore
/// run_battle(&mut battle, &data, &bus,
///     join(usb.run(&bus), buttons.run(&bus)),
///     ...).await;
/// ```
///
/// Pass `async {}` when no interactive source is needed (the runner will auto-continue).
/// The function returns as soon as the battle ends; `inputs` is dropped at that point even
/// if it is still pending (input sources typically loop forever, so `select` is correct here
/// rather than `join`).
pub async fn run_battle<E, T, F, DS>(
    battle: &mut battler::PublicCoreBattle<'_>,
    data: &DS,
    bus: &InputBus,
    inputs: F,
    queue: &mut BoardEventQueue,
    effects: &mut E,
    on_turn: T,
) where
    E: BoardEffects,
    T: FnMut(&mut battler::PublicCoreBattle<'_>),
    F: Future<Output = ()>,
    DS: DataStore,
{
    match select(battle_loop(battle, data, bus, queue, effects, on_turn), inputs).await {
        Either::First(()) | Either::Second(()) => {}
    }
}

async fn battle_loop<E, T, DS>(
    battle: &mut battler::PublicCoreBattle<'_>,
    data: &DS,
    bus: &InputBus,
    queue: &mut BoardEventQueue,
    effects: &mut E,
    mut on_turn: T,
) where
    E: BoardEffects,
    T: FnMut(&mut battler::PublicCoreBattle<'_>),
    DS: DataStore,
{
    enrich_and_dispatch(battle, data, queue, effects);

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
            let player_data = battle.player_data(&player_id).ok();
            bus.prompt.send(ActivePrompt {
                player_id: player_id.clone(),
                request: request.clone(),
                player_data,
            }).await;
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
            // With auto_continue=true the engine runs the turn synchronously inside
            // set_player_choice once all players have chosen, immediately making the
            // next turn's requests available. Flush here so events reach bus.log before
            // the next prompt is sent rather than piling up until the battle ends.
            enrich_and_dispatch(battle, data, queue, effects);
        }

        if !had_request {
            enrich_and_dispatch(battle, data, queue, effects);
            continue;
        }

        enrich_and_dispatch(battle, data, queue, effects);
        on_turn(battle);
    }
}
