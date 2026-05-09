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
    // ── phase 1: drain log entries out of the battle engine ───────────────────
    #[cfg(feature = "timing")]
    let t_drain = embassy_time::Instant::now();

    let entries: alloc::vec::Vec<alloc::string::String> =
        battle.new_log_entries().map(alloc::string::String::from).collect();
    let n_entries = entries.len();

    #[cfg(feature = "timing")]
    let drain_ms = t_drain.elapsed().as_millis();

    // ── phase 2: parse log lines into BoardEvents ─────────────────────────────
    #[cfg(feature = "timing")]
    let t_parse = embassy_time::Instant::now();

    queue.push_log_lines(entries.iter().map(alloc::string::String::as_str));

    #[cfg(feature = "timing")]
    let parse_ms = t_parse.elapsed().as_millis();

    // ── phase 3: enrich events + query_active_moves ───────────────────────────
    #[cfg(feature = "timing")]
    let t_enrich = embassy_time::Instant::now();

    let raw = queue.drain_pending();
    for event in raw {
        let inject = match &event {
            BoardEvent::Move { player_id: Some(pid), .. } => {
                #[cfg(feature = "timing")]
                let t_qam = embassy_time::Instant::now();

                let moves = query_active_moves(battle, data, pid.as_str());

                #[cfg(feature = "timing")]
                defmt::info!("  query_active_moves({}): {}ms", pid.as_str(), t_qam.elapsed().as_millis());

                Some((pid.clone(), moves))
            }
            _ => None,
        };

        let enriched = match event {
            BoardEvent::SwitchIn { name, species, player_id, moves } if moves.is_empty() => {
                #[cfg(feature = "timing")]
                let t_qam = embassy_time::Instant::now();

                let new_moves = player_id
                    .as_deref()
                    .map(|pid| query_active_moves(battle, data, pid))
                    .unwrap_or_default();

                #[cfg(feature = "timing")]
                defmt::info!("  query_active_moves(SwitchIn): {}ms", t_qam.elapsed().as_millis());

                BoardEvent::SwitchIn { name, species, player_id, moves: new_moves }
            }
            other => other,
        };

        queue.push_event(enriched);

        if let Some((pid, moves)) = inject {
            queue.push_event(BoardEvent::MovesUpdate { player_id: pid, moves });
        }
    }

    #[cfg(feature = "timing")]
    let enrich_ms = t_enrich.elapsed().as_millis();

    // ── phase 4: dispatch BoardEvents to OLED / LED / buzzer ─────────────────
    #[cfg(feature = "timing")]
    let t_dispatch = embassy_time::Instant::now();

    queue.dispatch_all(effects);

    #[cfg(feature = "timing")]
    let dispatch_ms = t_dispatch.elapsed().as_millis();

    #[cfg(feature = "timing")]
    if n_entries > 0 || drain_ms + parse_ms + enrich_ms + dispatch_ms > 0 {
        defmt::info!(
            "enrich_and_dispatch: entries={} drain={}ms parse={}ms enrich={}ms dispatch={}ms",
            n_entries as u32,
            drain_ms,
            parse_ms,
            enrich_ms,
            dispatch_ms,
        );
    }
}

/// Drive a battle to completion, running `inputs` concurrently for the duration.
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

            #[cfg(feature = "timing")]
            let t0 = embassy_time::Instant::now();

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

            #[cfg(feature = "timing")]
            defmt::info!(
                "set_player_choice({}): {}ms",
                player_id.as_str(),
                t0.elapsed().as_millis()
            );

            #[cfg(feature = "timing")]
            let t_ead = embassy_time::Instant::now();

            enrich_and_dispatch(battle, data, queue, effects);

            #[cfg(feature = "timing")]
            defmt::info!(
                "enrich_and_dispatch after set_player_choice({}): {}ms",
                player_id.as_str(),
                t_ead.elapsed().as_millis()
            );
        }

        if !had_request {
            enrich_and_dispatch(battle, data, queue, effects);
            continue;
        }

        enrich_and_dispatch(battle, data, queue, effects);
        on_turn(battle);
    }
}
