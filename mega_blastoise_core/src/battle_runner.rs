extern crate alloc;

use alloc::string::ToString;

use battler::{
    BattleType, CoreBattleEngineOptions, CoreBattleOptions, FormatData, PlayerData, PlayerDex,
    PlayerOptions, PlayerType, Request, SerializedRuleSet, SideData, TeamData,
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

// ── Move slot cache ───────────────────────────────────────────────────────────

// Caches per-player MoveSlot lists for the active Pokémon.  Populated once on
// SwitchIn (full CBOR decode via query_active_moves).  On subsequent Move
// events we only refresh PP from battle RAM — no CBOR decode.
struct MoveSlotCache {
    entries: alloc::vec::Vec<(alloc::string::String, alloc::vec::Vec<MoveSlot>)>,
}

impl MoveSlotCache {
    fn new() -> Self {
        Self { entries: alloc::vec::Vec::new() }
    }

    fn store(&mut self, player_id: &str, slots: alloc::vec::Vec<MoveSlot>) {
        if let Some(e) = self.entries.iter_mut().find(|(id, _)| id == player_id) {
            e.1 = slots;
        } else {
            self.entries.push((player_id.to_string(), slots));
        }
    }

    // Read current PP from battle state (no CBOR, no full PlayerBattleData), update
    // cached slots, return a clone.  Returns empty vec on cache miss.
    fn refresh_pp(
        &mut self,
        player_id: &str,
        battle: &battler::PublicCoreBattle<'_>,
    ) -> alloc::vec::Vec<MoveSlot> {
        let pp_list = battle.active_mon_move_pp(player_id).unwrap_or_default();

        let Some(entry) = self.entries.iter_mut().find(|(id, _)| id == player_id) else {
            return alloc::vec::Vec::new();
        };
        for (slot, (pp, max_pp)) in entry.1.iter_mut().zip(pp_list.iter()) {
            slot.pp = *pp;
            slot.max_pp = *max_pp;
        }
        entry.1.clone()
    }
}

// ── DataStore lookup (full CBOR decode, called only on SwitchIn) ──────────────

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

// ── Event enrichment ──────────────────────────────────────────────────────────

async fn enrich_and_dispatch<E, DS>(
    battle: &mut battler::PublicCoreBattle<'_>,
    data: &DS,
    queue: &mut BoardEventQueue,
    effects: &mut E,
    cache: &mut MoveSlotCache,
) where
    E: BoardEffects,
    DS: DataStore,
{
    #[cfg(feature = "timing")]
    let t_drain = embassy_time::Instant::now();

    let entries: alloc::vec::Vec<alloc::string::String> =
        battle.new_log_entries().map(alloc::string::String::from).collect();
    let n_entries = entries.len();

    #[cfg(feature = "timing")]
    let drain_ms = t_drain.elapsed().as_millis();

    #[cfg(feature = "timing")]
    let t_parse = embassy_time::Instant::now();

    queue.push_log_lines(entries.iter().map(alloc::string::String::as_str));

    #[cfg(feature = "timing")]
    let parse_ms = t_parse.elapsed().as_millis();

    #[cfg(feature = "timing")]
    let t_enrich = embassy_time::Instant::now();

    let raw = queue.drain_pending();
    for event in raw {
        let inject = match &event {
            BoardEvent::Move { player_id: Some(pid), .. } => {
                // PP-only refresh from battle RAM — no CBOR decode.
                #[cfg(feature = "timing")]
                let t_pp = embassy_time::Instant::now();

                let moves = cache.refresh_pp(pid.as_str(), battle);

                #[cfg(feature = "timing")]
                defmt::info!("  refresh_pp({}): {}ms", pid.as_str(), t_pp.elapsed().as_millis());

                if moves.is_empty() {
                    // Cache miss (shouldn't happen after SwitchIn) — fall back.
                    #[cfg(feature = "timing")]
                    defmt::info!("  refresh_pp cache miss for {}, falling back", pid.as_str());
                    let moves = query_active_moves(battle, data, pid.as_str());
                    cache.store(pid.as_str(), moves.clone());
                    Some((pid.clone(), moves))
                } else {
                    Some((pid.clone(), moves))
                }
            }
            _ => None,
        };

        let enriched = match event {
            BoardEvent::SwitchIn { name, species, player_id, moves } if moves.is_empty() => {
                // Full CBOR decode — only happens once per switch-in.
                let new_moves = player_id.as_deref().map(|pid| {
                    #[cfg(feature = "timing")]
                    let t_qam = embassy_time::Instant::now();

                    let moves = query_active_moves(battle, data, pid);
                    cache.store(pid, moves.clone());

                    #[cfg(feature = "timing")]
                    defmt::info!("  query_active_moves(SwitchIn {}): {}ms", pid, t_qam.elapsed().as_millis());

                    moves
                }).unwrap_or_default();
                BoardEvent::SwitchIn { name, species, player_id, moves: new_moves }
            }
            BoardEvent::SwitchIn { name, species, player_id, moves } => {
                // Pre-populated — cache for future PP refreshes.
                if let Some(pid) = &player_id {
                    cache.store(pid.as_str(), moves.clone());
                }
                BoardEvent::SwitchIn { name, species, player_id, moves }
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

    #[cfg(feature = "timing")]
    let t_dispatch = embassy_time::Instant::now();

    queue.dispatch_all(effects).await;

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

// ── Battle runner ─────────────────────────────────────────────────────────────

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
    let mut cache = MoveSlotCache::new();
    enrich_and_dispatch(battle, data, queue, effects, &mut cache).await;

    while !battle.ended() {
        let mut had_request = false;
        loop {
            // Collect ALL active requests before sending any prompts, so both
            // players are prompted simultaneously and can inspect their screens
            // in parallel without waiting on each other.
            let requests: alloc::vec::Vec<(alloc::string::String, Request)> = battle
                .active_requests()
                .map(|(pid, req)| (pid.to_string(), req.clone()))
                .collect();

            if requests.is_empty() { break; }
            had_request = true;

            // Send all prompts to bus.prompt before collecting any choices.
            let batch_total = requests.len();
            for (player_id, request) in &requests {
                queue.push_event(board_prompt_event(player_id, request));
                queue.dispatch_all(effects).await;
                let player_data = battle.player_data(player_id).ok();
                bus.prompt.send(ActivePrompt {
                    player_id: player_id.clone(),
                    request: request.clone(),
                    player_data,
                    batch_total,
                }).await;
            }

            // Collect choices in the same order as prompts, then apply them.
            for (player_id, _) in &requests {
                let line = bus.choices.receive().await;

                #[cfg(feature = "timing")]
                let t0 = embassy_time::Instant::now();

                if let Err(e) = battle.set_player_choice(player_id, &line) {
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

                enrich_and_dispatch(battle, data, queue, effects, &mut cache).await;

                #[cfg(feature = "timing")]
                defmt::info!(
                    "enrich_and_dispatch after set_player_choice({}): {}ms",
                    player_id.as_str(),
                    t_ead.elapsed().as_millis()
                );
            }
        }

        if !had_request {
            enrich_and_dispatch(battle, data, queue, effects, &mut cache).await;
            continue;
        }

        enrich_and_dispatch(battle, data, queue, effects, &mut cache).await;
        on_turn(battle);
    }
}
