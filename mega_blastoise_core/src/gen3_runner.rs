//! Drives a [`gen3_battle`] battle and narrates it as [`BoardEvent`]s.
//!
//! This is the Gen 3 half of the engine seam. The Gen 1 half is
//! [`crate::battle_runner`] and is deliberately untouched: a Gen 1 battle runs
//! exactly the code it always ran, and picking a ruleset picks which of the
//! two a caller drives.
//!
//! Both runners own their loop, so a platform drives either the same way:
//! hand it a battle, somewhere to read choices from, and somewhere to narrate
//! to. [`run_battle`] here is the counterpart of [`crate::battle_runner`]'s.
//!
//! Narration pacing is deliberately absent: [`BoardEffects`] sinks already
//! delay per event, so both engines inherit identical timing by emitting the
//! same events. A loop that slept as well would pace everything twice.
//!
//! [`play_turn`] stays public underneath, because a turn resolved without a
//! bus is what the tests use.

extern crate alloc;

use alloc::{format, string::String, string::ToString, vec::Vec};

use gen3_battle::battle::{Battle, Choice, Event};

use crate::battle_effects::BoardEffects;
use crate::board_event::{BoardEvent, MoveSlot, PromptKind};

/// `"p1"` / `"p2"` for a 1-based side.
fn player_id(side: u8) -> String {
    if side == 1 { "p1".to_string() } else { "p2".to_string() }
}

/// Who each side had on the field, as party indices. The engine plays a WHOLE
/// turn before handing back its event list, so by the time those events are
/// narrated `Side::mon()` names whoever survived to the end of it — not who
/// actually acted. A mon that attacked and then fainted had every one of its
/// lines read out under its replacement's name. Walking the events with this
/// alongside, updated on each `Switched`, keeps the narration honest.
///
/// Indices stay valid across a switch because `reorder_for_switch` permutes
/// `Side::order` and never `Side::party`.
type Onstage = [usize; 2];

/// The active mon as a board position string: `"Name,p1"`.
///
/// The display and LED layers route by parsing the player id back out of this
/// field ([`crate::board_event::mon_player_num`]), so a bare species name
/// would leave HP updates, faints and party syncing silently doing nothing.
fn active_name(battle: &Battle, side: u8, at: &Onstage) -> String {
    let i = (side - 1) as usize;
    let name = battle.sides[i].party[at[i]].species.name;
    format!("{name},{}", player_id(side))
}

/// `"cur/max"`, the health string the display layer already parses.
fn health(battle: &Battle, side: u8) -> String {
    let m = battle.sides[(side - 1) as usize].mon();
    format!("{}/{}", m.hp, m.max_hp)
}

/// The active mon's moves in the shape the board speaks.
fn move_slots(battle: &Battle, side: u8, at: &Onstage) -> Vec<MoveSlot> {
    let i = (side - 1) as usize;
    battle.sides[i].party[at[i]]
        .moves
        .iter()
        .map(|s| MoveSlot {
            name: s.entry.name.to_string(),
            type_name: s.entry.move_type.name().to_string(),
            category: match s.entry.category() {
                gen3_battle::Category::Physical => "Physical".to_string(),
                gen3_battle::Category::Special => "Special".to_string(),
                gen3_battle::Category::Status => "Status".to_string(),
            },
            power: if s.entry.power == 0 { None } else { Some(s.entry.power as u32) },
            accuracy: if s.entry.accuracy == 0 { None } else { Some(s.entry.accuracy) },
            pp: s.pp,
            max_pp: s.entry.pp,
        })
        .collect()
}

async fn switch_in<E: BoardEffects>(battle: &Battle, side: u8, at: &Onstage, effects: &mut E) {
    let i = (side - 1) as usize;
    let mon = &battle.sides[i].party[at[i]];
    effects
        .on_event(BoardEvent::SwitchIn {
            // The bare species name, NOT a `Name,p1` position string. This
            // event carries `player_id` in its own field, so the name needs no
            // seat glued to it — and gluing one on broke two things at once:
            // the plate printed "Blastoise,p1", and the sprite table, which is
            // keyed by species, missed every lookup and drew nothing. Gen 1
            // fills this field from the battler log's own `name`, which is a
            // bare nickname; Gen 3 now matches. The `mon:` fields on every
            // other event DO carry the suffix, because those have nowhere else
            // to say which seat they mean.
            name: mon.species.name.to_string(),
            species: Some(mon.species.name.to_string()),
            player_id: Some(player_id(side)),
            team_slot: Some(at[i] as u8),
            moves: move_slots(battle, side, at),
            speed: Some(mon.spe),
        })
        .await;
}

/// Open the battle: both leads out, both move lists published.
pub async fn announce_start<E: BoardEffects>(battle: &Battle, effects: &mut E) {
    effects.on_event(BoardEvent::BattleStart).await;
    // Nothing has happened yet, so the leads ARE the active mons.
    let at = &[battle.sides[0].active, battle.sides[1].active];
    for side in [1u8, 2] {
        switch_in(battle, side, at, effects).await;
    }
}

/// Resolve one turn and narrate it. Returns true once the battle is over.
pub async fn play_turn<E: BoardEffects>(
    battle: &mut Battle,
    choices: [Choice; 2],
    effects: &mut E,
) -> bool {
    // Who is on stage NOW, before the engine plays the turn out. The walk
    // below moves this on as it meets each `Switched`, so every line is read
    // in the voice of the mon that was actually standing there.
    let mut on = [battle.sides[0].active, battle.sides[1].active];
    let at = &mut on;
    let events = battle.step(choices);
    effects.on_event(BoardEvent::Turn { n: battle.turn }).await;

    for event in events {
        // A switch changes who the rest of the turn is about — including the
        // switch-in announcement itself, which is about the ARRIVAL.
        if let Event::Switched { side, party_index } = event {
            at[(side - 1) as usize] = party_index;
        }
        let at = &*at;
        match event {
            Event::Used { side, move_index } => {
                let name = battle.sides[(side - 1) as usize]
                    .mon()
                    .moves
                    .get(move_index)
                    .map(|s| s.entry.name.to_string())
                    .unwrap_or_default();
                effects
                    .on_event(BoardEvent::Move {
                        user: Some(active_name(battle, side, at)),
                        player_id: Some(player_id(side)),
                        name,
                    })
                    .await;
                // PP moved, so the seat's move list is stale.
                effects
                    .on_event(BoardEvent::MovesUpdate {
                        player_id: player_id(side),
                        moves: move_slots(battle, side, at),
                    })
                    .await;
            }
            Event::Damage { side, effectiveness, crit, .. } => {
                let mon = active_name(battle, side, at);
                if crit {
                    effects.on_event(BoardEvent::CriticalHit { mon: mon.clone() }).await;
                }
                // The board narrates effectiveness separately from the number.
                match effectiveness {
                    0 => effects.on_event(BoardEvent::Immune { mon: mon.clone() }).await,
                    e if e > 100 => {
                        effects.on_event(BoardEvent::SuperEffective { mon: mon.clone() }).await
                    }
                    e if e < 100 => effects.on_event(BoardEvent::Resisted { mon: mon.clone() }).await,
                    _ => {}
                }
                effects
                    .on_event(BoardEvent::Damage { mon, health: health(battle, side) })
                    .await;
            }
            Event::Fainted { side } => {
                effects
                    .on_event(BoardEvent::Faint {
                        mon: active_name(battle, side, at),
                        team_slot: Some(battle.sides[(side - 1) as usize].active as u8),
                    })
                    .await;
            }
            Event::Statused { side, status } => {
                effects
                    .on_event(BoardEvent::SetStatus {
                        mon: active_name(battle, side, at),
                        status: status.abbr().to_string(),
                    })
                    .await;
            }
            Event::Boosted { side, boost, delta } => {
                effects
                    .on_event(BoardEvent::StatChange {
                        mon: active_name(battle, side, at),
                        stat: boost.label().into(),
                        delta,
                    })
                    .await;
            }
            Event::Drained { side, .. } => {
                effects
                    .on_event(BoardEvent::Heal {
                        mon: active_name(battle, side, at),
                        health: health(battle, side),
                    })
                    .await;
            }
            Event::Healed { side, .. } => {
                effects
                    .on_event(BoardEvent::Heal {
                        mon: active_name(battle, side, at),
                        health: health(battle, side),
                    })
                    .await;
            }
            Event::Recoil { side, .. } => {
                effects
                    .on_event(BoardEvent::Damage {
                        mon: active_name(battle, side, at),
                        health: health(battle, side),
                    })
                    .await;
            }
            Event::ConfusionStarted { side } => {
                effects
                    .on_event(BoardEvent::EffectStart {
                        mon: active_name(battle, side, at),
                        what: "confusion".into(),
                        detail: None,
                    })
                    .await;
            }
            Event::ConfusedHit { side, .. } => {
                effects
                    .on_event(BoardEvent::Damage {
                        mon: active_name(battle, side, at),
                        health: health(battle, side),
                    })
                    .await;
            }
            Event::ConfusionEnded { side } => {
                effects
                    .on_event(BoardEvent::EffectEnd {
                        mon: active_name(battle, side, at),
                        what: "confusion".into(),
                    })
                    .await;
            }
            Event::SideStarted { side, condition } => {
                effects
                    .on_event(BoardEvent::EffectStart {
                        mon: active_name(battle, side, at),
                        what: condition.label().into(),
                        detail: None,
                    })
                    .await;
            }
            Event::SideEnded { side, condition } => {
                effects
                    .on_event(BoardEvent::EffectEnd {
                        mon: active_name(battle, side, at),
                        what: condition.label().into(),
                    })
                    .await;
            }
            Event::WeatherStarted { weather } => {
                effects
                    .on_event(BoardEvent::EffectStart {
                        mon: String::new(),
                        what: weather.label().into(),
                        detail: None,
                    })
                    .await;
            }
            Event::WeatherEnded { weather } => {
                effects
                    .on_event(BoardEvent::EffectEnd {
                        mon: String::new(),
                        what: weather.label().into(),
                    })
                    .await;
            }
            Event::WeatherDamage { side, .. } => {
                effects
                    .on_event(BoardEvent::Damage {
                        mon: active_name(battle, side, at),
                        health: health(battle, side),
                    })
                    .await;
            }
            Event::Drowsy { side } => {
                effects
                    .on_event(BoardEvent::EffectStart {
                        mon: active_name(battle, side, at),
                        what: "drowsy".into(),
                        detail: None,
                    })
                    .await;
            }
            Event::PerishCount { side, n } => {
                effects
                    .on_event(BoardEvent::EffectStart {
                        mon: active_name(battle, side, at),
                        what: alloc::format!("perish{n}"),
                        detail: None,
                    })
                    .await;
            }
            Event::DestinyArmed { side } => {
                effects
                    .on_event(BoardEvent::EffectStart {
                        mon: active_name(battle, side, at),
                        what: "Destiny Bond".into(),
                        detail: None,
                    })
                    .await;
            }
            Event::NoEscape { side } => {
                effects
                    .on_event(BoardEvent::EffectStart {
                        mon: active_name(battle, side, at),
                        what: "trapped".into(),
                        detail: None,
                    })
                    .await;
            }
            Event::SpikesLaid { side } => {
                effects
                    .on_event(BoardEvent::EffectStart {
                        mon: active_name(battle, side, at),
                        what: "Spikes".into(),
                        detail: None,
                    })
                    .await;
            }
            Event::SpikesDamage { side, .. } => {
                effects
                    .on_event(BoardEvent::Damage {
                        mon: active_name(battle, side, at),
                        health: health(battle, side),
                    })
                    .await;
            }
            Event::Protected { side } => {
                effects
                    .on_event(BoardEvent::EffectStart {
                        mon: active_name(battle, side, at),
                        what: "Protect".into(),
                        detail: None,
                    })
                    .await;
            }
            Event::HazeCleared => {
                effects
                    .on_event(BoardEvent::EffectStart {
                        mon: String::new(),
                        what: "Haze".into(),
                        detail: None,
                    })
                    .await;
            }
            Event::SubStarted { side } => {
                effects
                    .on_event(BoardEvent::EffectStart {
                        mon: active_name(battle, side, at),
                        what: "Substitute".into(),
                        detail: None,
                    })
                    .await;
            }
            Event::SubDamage { .. } => {}
            Event::SubBroke { side } => {
                effects
                    .on_event(BoardEvent::EffectEnd {
                        mon: active_name(battle, side, at),
                        what: "Substitute".into(),
                    })
                    .await;
            }
            Event::Focused { side } => {
                effects
                    .on_event(BoardEvent::EffectStart {
                        mon: active_name(battle, side, at),
                        what: "Focus Energy".into(),
                        detail: None,
                    })
                    .await;
            }
            Event::Rested { side } => {
                effects
                    .on_event(BoardEvent::Heal {
                        mon: active_name(battle, side, at),
                        health: health(battle, side),
                    })
                    .await;
                effects
                    .on_event(BoardEvent::SetStatus {
                        mon: active_name(battle, side, at),
                        status: "slp".into(),
                    })
                    .await;
            }
            Event::Trapped { side } => {
                effects
                    .on_event(BoardEvent::EffectStart {
                        mon: active_name(battle, side, at),
                        what: "bind".into(),
                        detail: None,
                    })
                    .await;
            }
            Event::TrapDamage { side, .. } => {
                effects
                    .on_event(BoardEvent::Damage {
                        mon: active_name(battle, side, at),
                        health: health(battle, side),
                    })
                    .await;
            }
            Event::TrapEnded { side } => {
                effects
                    .on_event(BoardEvent::EffectEnd {
                        mon: active_name(battle, side, at),
                        what: "bind".into(),
                    })
                    .await;
            }
            Event::Seeded { side } => {
                effects
                    .on_event(BoardEvent::EffectStart {
                        mon: active_name(battle, side, at),
                        what: "Leech Seed".into(),
                        detail: None,
                    })
                    .await;
            }
            Event::SeedDrain { side, .. } => {
                effects
                    .on_event(BoardEvent::Damage {
                        mon: active_name(battle, side, at),
                        health: health(battle, side),
                    })
                    .await;
            }
            Event::Charging { side } => {
                effects
                    .on_event(BoardEvent::EffectStart {
                        mon: active_name(battle, side, at),
                        what: "charge".into(),
                        detail: None,
                    })
                    .await;
            }
            Event::Recharging { side } => {
                effects
                    .on_event(BoardEvent::Cant {
                        mon: active_name(battle, side, at),
                        reason: "recharge".into(),
                    })
                    .await;
            }
            Event::Flinched { side } => {
                effects
                    .on_event(BoardEvent::Cant { mon: active_name(battle, side, at), reason: "flinch".into() })
                    .await;
            }
            Event::FullyParalyzed { side } => {
                effects
                    .on_event(BoardEvent::Cant { mon: active_name(battle, side, at), reason: "par".into() })
                    .await;
            }
            Event::Infatuated { side } => {
                effects
                    .on_event(BoardEvent::Cant {
                        mon: active_name(battle, side, at),
                        reason: "love".into(),
                    })
                    .await;
            }
            Event::Cant { side, status } => {
                effects
                    .on_event(BoardEvent::Cant {
                        mon: active_name(battle, side, at),
                        reason: status.abbr().to_string(),
                    })
                    .await;
            }
            Event::Residual { side, .. } => {
                effects
                    .on_event(BoardEvent::Damage {
                        mon: active_name(battle, side, at),
                        health: health(battle, side),
                    })
                    .await;
            }
            Event::Switched { side, .. } => switch_in(battle, side, at, effects).await,
            Event::Failed { side } => {
                effects.on_event(BoardEvent::Fail { mon: active_name(battle, side, at) }).await;
            }
            Event::Win { side } => {
                if side == 0 {
                    effects.on_event(BoardEvent::Tie).await;
                } else {
                    effects.on_event(BoardEvent::Win { side: Some(player_id(side)) }).await;
                }
            }
        }
    }
    battle.over()
}

/// Distil one Gen 3 seat into the generation-neutral [`SlotOptions`] the
/// shared collector consumes. The Gen 3 counterpart of
/// [`crate::battle_runner::slot_options_from_request`]: parsing the engine's
/// state is battle logic and lives with the engine; everything past the
/// [`crate::battle_input::InputBus`] is generation-blind.
///
/// The first cut of this runner skipped the collector and read raw pad
/// presses instead, and every behaviour the collector already owned came back
/// as a bug report: presses queued through the narration locked seats in
/// before a menu was drawn, B could not unready, an AI could pick a 0-PP
/// move, and nothing validated a press at all.
pub fn slot_options_for(battle: &Battle, side: u8) -> crate::choice_collect::SlotOptions {
    use crate::choice_collect::SlotOptions;
    use crate::display::PartySlotData;

    let i = (side - 1) as usize;
    let s_side = &battle.sides[i];
    let mon = s_side.mon();
    let mut s = SlotOptions::blank(side, player_id(side));
    s.active_slot = Some(s_side.active);
    for (slot_i, ok) in s.party_ok.iter_mut().enumerate() {
        *ok = slot_i != s_side.active
            && s_side.party.get(slot_i).is_some_and(|m| !m.fainted());
    }
    s.party = s_side
        .party
        .iter()
        .enumerate()
        .map(|(slot_i, m)| PartySlotData {
            name: alloc::string::String::from(m.species.name),
            active: slot_i == s_side.active,
            level: m.level,
            hp: m.hp,
            max_hp: m.max_hp,
            status: m.status.map(|st| alloc::string::String::from(status_id(st))),
            atk: m.atk,
            def: m.def,
            spe: m.spe,
            // The party card was drawn for Gen 1's single Special; showing
            // Sp.Atk there is the least wrong of the two halves.
            spc: m.spa,
            types: {
                let (t1, t2) = m.species.types;
                let mut v = alloc::vec![t1];
                if t2 != battle_types::Type::None {
                    v.push(t2);
                }
                v
            },
            boost_atk: if slot_i == s_side.active { m.stages[0] } else { 0 },
            boost_def: if slot_i == s_side.active { m.stages[1] } else { 0 },
            // Stage order is Atk, Def, Spe, SpAtk, SpDef.
            boost_spc: if slot_i == s_side.active { m.stages[3] } else { 0 },
            boost_spe: if slot_i == s_side.active { m.stages[2] } else { 0 },
            item: if m.item.is_empty() {
                None
            } else {
                Some(alloc::string::String::from(m.item))
            },
            moves: m
                .moves
                .iter()
                .map(|ms| (alloc::string::String::from(ms.entry.name), ms.pp, ms.entry.pp))
                .collect(),
        })
        .collect();
    if mon.fainted() {
        // The engine auto-replaces a mid-turn faint with the lowest living
        // slot, so this only happens when a battle is being poked after it
        // ended; keep the shape honest anyway.
        s.forced_switch = true;
    } else {
        s.n_moves = mon.moves.len().min(4);
        for (mi, u) in s.usable.iter_mut().enumerate().take(s.n_moves) {
            *u = mon.moves[mi].pp > 0;
        }
        s.trapped = !battle.can_switch(i);
        if s.n_moves == 0 {
            // Struggle: the engine substitutes it whatever index arrives.
            s.auto = Some(crate::battle_input::format_move_choice(0));
        }
    }
    s
}

fn status_id(st: gen3_battle::data::Status) -> &'static str {
    use gen3_battle::data::Status;
    match st {
        Status::Burn => "brn",
        Status::Paralysis => "par",
        Status::Poison => "psn",
        Status::Toxic => "tox",
        Status::Sleep => "slp",
        Status::Freeze => "frz",
    }
}

/// The shared choice grammar, read back into an engine [`Choice`]. The
/// collector speaks strings ("move 2" / "switch 4") because that is the Gen 1
/// battler's own protocol; Gen 3 borrows the grammar rather than inventing a
/// second one, and this is the whole cost of the borrowing.
pub fn parse_choice(line: &str) -> Choice {
    let mut parts = line.split_whitespace();
    match (parts.next(), parts.next().and_then(|n| n.parse::<usize>().ok())) {
        (Some("switch"), Some(i)) => Choice::Switch(i),
        (Some("move"), Some(i)) => Choice::Move(i),
        // "pass" and anything unparseable: the engine treats an
        // out-of-range move like slot 0, which is also what a locked or
        // Struggling mon does with any index.
        _ => Choice::Move(0),
    }
}

/// Run a Gen 3 battle to its end over the SAME [`InputBus`] protocol the
/// Gen 1 runner speaks: distil each seat into an [`ActivePrompt`], let the
/// platform's collector pump answer with choice strings, step the engine,
/// narrate. Who is AI, what a press means, unready, validation — none of that
/// is here, because none of it is Gen 3's business.
pub async fn battle_loop<E: BoardEffects>(
    battle: &mut Battle,
    bus: &crate::battle_input::InputBus,
    effects: &mut E,
) {
    use crate::battle_input::ActivePrompt;

    announce_start(battle, effects).await;
    while !battle.over() {
        let mut chosen: [Option<Choice>; 2] = [None, None];
        for side in [1u8, 2] {
            let slot = slot_options_for(battle, side);
            effects
                .on_event(BoardEvent::Prompt {
                    player_id: player_id(side),
                    kind: PromptKind::ChooseMove,
                })
                .await;
            bus.prompt
                .send(ActivePrompt {
                    player_id: player_id(side),
                    slot,
                    batch_total: 2,
                })
                .await;
        }
        while chosen[0].is_none() || chosen[1].is_none() {
            let submitted = bus.choices.receive().await;
            let i = match submitted.player_id.as_str() {
                "p1" => 0,
                "p2" => 1,
                _ => continue,
            };
            chosen[i] = Some(parse_choice(&submitted.choice));
        }
        if play_turn(battle, [chosen[0].unwrap(), chosen[1].unwrap()], effects).await {
            break;
        }
    }
}

/// [`battle_loop`] raced against the platform's input future — the same
/// composition shape as the Gen 1 [`crate::run_battle`].
pub async fn run_battle<E: BoardEffects, F: core::future::Future<Output = ()>>(
    battle: &mut Battle,
    bus: &crate::battle_input::InputBus,
    inputs: F,
    effects: &mut E,
) {
    use embassy_futures::select::{select, Either};
    match select(battle_loop(battle, bus, effects), inputs).await {
        Either::First(()) | Either::Second(()) => {}
    }
}

/// Draft two random-battle teams from one seed and build the battle. The
/// setup half of a Gen 3 game, in core so both platforms construct battles
/// identically; the caller only decides the seed and the team size.
pub fn drafted_battle(seed: u64, six: bool) -> Option<Battle> {
    use gen3_battle::battle::Side;
    let size = if six { 6 } else { 3 };
    let mut rng = gen3_battle::Rng::new(seed);
    let t1 = gen3_battle::draft_team(&mut rng, size);
    let t2 = gen3_battle::draft_team(&mut rng, size);
    if t1.is_empty() || t2.is_empty() {
        return None;
    }
    Some(Battle::new(Side::new(t1), Side::new(t2), seed))
}


#[cfg(test)]
mod tests {
    use super::*;
    use gen3_battle::{battle::Side, Invest, Mon, Nature};

    /// Records what the board was told, so a turn can be asserted on.
    #[derive(Default)]
    struct Recorder(Vec<BoardEvent>);

    impl BoardEffects for Recorder {
        async fn on_event(&mut self, event: BoardEvent) {
            self.0.push(event);
        }
    }

    fn mon(id: &str, level: u8, moves: &[&str]) -> Mon {
        Mon::new(id, level, Nature::Hardy, Invest { iv: 31, ev: 0 }, moves).expect("species")
    }

    fn battle() -> Battle {
        Battle::new(
            Side::new(alloc::vec![mon("blaziken", 100, &["ember"])]),
            Side::new(alloc::vec![mon("treecko", 5, &["pound"])]),
            42,
        )
    }

    /// The engine plays a whole turn before handing back its events, so the
    /// narration has to be told who was standing there when each one fired —
    /// reading `Side::mon()` names whoever is left at the END of the turn. A
    /// mon that attacked and then fainted had its whole turn read out in its
    /// replacement's name, which is what this pins down: given an index that
    /// disagrees with the live `active`, the narration follows the index.
    #[test]
    fn narration_names_the_mon_that_was_on_stage_not_the_one_left_standing() {
        let mut b = Battle::new(
            Side::new(alloc::vec![
                mon("blaziken", 100, &["ember"]),
                mon("wigglytuff", 100, &["pound"]),
            ]),
            Side::new(alloc::vec![mon("treecko", 5, &["pound"])]),
            42,
        );
        // Whoever the turn ENDED with.
        b.sides[0].active = 1;
        assert_eq!(b.sides[0].mon().species.name, "Wigglytuff");
        // Whoever was on stage when the event fired.
        let at = &[0usize, 0usize];
        assert_eq!(active_name(&b, 1, at), "Blaziken,p1");
        assert_eq!(move_slots(&b, 1, at)[0].name, "Ember");
    }

    /// The whole Gen 3 seat protocol, end to end over the real bus: every
    /// turn sends a fresh prompt per seat BEFORE it will accept a choice, and
    /// the collected strings drive the engine. This is the core-level pin for
    /// the "player was never prompted" bug: the old runner read raw pad
    /// events with no prompt/choice protocol at all, so a press queued during
    /// the narration was consumed as the next turn's answer and the seat
    /// never saw a menu. Under the bus protocol that cannot happen — a
    /// choice is only produced by the platform pump in ANSWER to a prompt,
    /// and this test asserts the prompts actually arrive, carrying options
    /// that match the engine's live state.
    #[test]
    fn every_turn_prompts_both_seats_before_taking_choices() {
        let mut b = Battle::new(
            Side::new(alloc::vec![mon("blaziken", 50, &["ember", "doubleedge"])]),
            Side::new(alloc::vec![mon("swampert", 50, &["surf"])]),
            42,
        );
        let bus = crate::battle_input::InputBus::new();
        let mut rec = Recorder::default();
        let mut turns = 0usize;
        block_on(async {
            loop {
                if b.over() || turns >= 8 {
                    break;
                }
                // One turn of the runner's loop, hand-driven: it must send
                // both prompts before it blocks on choices.
                let step = async {
                    for side in [1u8, 2] {
                        let slot = slot_options_for(&b, side);
                        bus.prompt
                            .send(crate::battle_input::ActivePrompt {
                                player_id: player_id(side),
                                slot,
                                batch_total: 2,
                            })
                            .await;
                    }
                };
                step.await;
                // The "platform": drain exactly the prompts that were sent,
                // check their shape, and answer them.
                for _ in 0..2 {
                    let p = bus.prompt.try_receive().expect("a prompt per seat per turn");
                    assert!(p.slot.n_moves > 0, "prompt carries the move count");
                    assert!(
                        (1..=2).contains(&crate::board_event::player_id_to_num(&p.player_id)),
                        "prompt names a seat"
                    );
                    bus.choices
                        .send(crate::battle_input::PlayerChoice {
                            player_id: p.player_id,
                            choice: alloc::string::String::from("move 0"),
                        })
                        .await;
                }
                let mut chosen = [None, None];
                while chosen[0].is_none() || chosen[1].is_none() {
                    let c = bus.choices.try_receive().expect("both answers are in");
                    let i = (c.player_id == "p2") as usize;
                    chosen[i] = Some(parse_choice(&c.choice));
                }
                turns += 1;
                if play_turn(&mut b, [chosen[0].unwrap(), chosen[1].unwrap()], &mut rec).await {
                    break;
                }
            }
        });
        assert!(turns > 0, "the battle played at least one turn");
        assert!(
            rec.0.iter().any(|e| matches!(e, BoardEvent::Move { .. })),
            "choices drove real moves"
        );
    }

    /// The distiller only offers what the engine will accept: no 0-PP move
    /// is usable, only living benched mons are switch targets, and the party
    /// rows carry real HP. The old runner offered raw counts, so its robot
    /// could pick a dead move and its menus could not grey anything out.
    #[test]
    fn distilled_options_match_engine_state() {
        let mut b = Battle::new(
            Side::new(alloc::vec![
                mon("blaziken", 50, &["ember", "doubleedge"]),
                mon("wigglytuff", 50, &["pound"]),
            ]),
            Side::new(alloc::vec![mon("swampert", 50, &["surf"])]),
            42,
        );
        b.sides[0].party[0].moves[1].pp = 0;
        let s = slot_options_for(&b, 1);
        assert_eq!(s.n_moves, 2);
        assert!(s.usable[0], "Ember has PP");
        assert!(!s.usable[1], "the drained slot is not offered");
        assert!(s.party_ok[1], "the benched Wigglytuff is a switch target");
        assert!(!s.party_ok[0], "the active mon is not");
        assert_eq!(s.party.len(), 2);
        assert!(s.party[0].active && s.party[0].hp > 0);
    }

    fn block_on<F: core::future::Future>(f: F) -> F::Output {
        // The runner only awaits the sink, which never yields in these tests,
        // so a trivial executor is enough.
        use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
        fn noop(_: *const ()) {}
        fn clone(p: *const ()) -> RawWaker {
            RawWaker::new(p, &VTABLE)
        }
        static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, noop, noop, noop);
        let waker = unsafe { Waker::from_raw(RawWaker::new(core::ptr::null(), &VTABLE)) };
        let mut cx = Context::from_waker(&waker);
        let mut f = core::pin::pin!(f);
        loop {
            if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
                return v;
            }
        }
    }

    #[test]
    fn the_opening_puts_both_leads_on_the_board() {
        let b = battle();
        let mut rec = Recorder::default();
        block_on(announce_start(&b, &mut rec));
        assert!(matches!(rec.0[0], BoardEvent::BattleStart));
        let switch_ins = rec
            .0
            .iter()
            .filter(|e| matches!(e, BoardEvent::SwitchIn { .. }))
            .count();
        assert_eq!(switch_ins, 2, "both sides lead");
    }

    #[test]
    fn a_turn_narrates_the_move_the_damage_and_the_win() {
        let mut b = battle();
        let mut rec = Recorder::default();
        let over = block_on(play_turn(&mut b, [Choice::Move(0), Choice::Move(0)], &mut rec));

        assert!(matches!(rec.0[0], BoardEvent::Turn { n: 1 }));
        assert!(rec.0.iter().any(|e| matches!(e, BoardEvent::Move { .. })));
        assert!(rec.0.iter().any(|e| matches!(e, BoardEvent::Damage { .. })));
        // Ember on a Grass lead, from a level 100 attacker: super effective,
        // and fatal.
        assert!(rec.0.iter().any(|e| matches!(e, BoardEvent::SuperEffective { .. })));
        assert!(rec.0.iter().any(|e| matches!(e, BoardEvent::Faint { .. })));
        assert!(rec.0.iter().any(|e| matches!(e, BoardEvent::Win { .. })));
        assert!(over);
    }

    #[test]
    fn every_mon_field_routes_back_to_a_player() {
        // The sinks parse the player id out of the mon string. A bare species
        // name parses to nothing and the HP bar never moves.
        use crate::board_event::mon_player_num;
        let mut b = battle();
        let mut rec = Recorder::default();
        block_on(announce_start(&b, &mut rec));
        block_on(play_turn(&mut b, [Choice::Move(0), Choice::Move(0)], &mut rec));
        for e in &rec.0 {
            let mon = match e {
                BoardEvent::Damage { mon, .. }
                | BoardEvent::Faint { mon, .. }
                | BoardEvent::SuperEffective { mon }
                | BoardEvent::CriticalHit { mon } => Some(mon),
                _ => None,
            };
            if let Some(mon) = mon {
                assert!(mon_player_num(mon).is_some(), "{mon:?} does not name a player");
            }
            // A SwitchIn says which seat in its own field, and its `name` is
            // the bare species — the display prints it and the sprite table is
            // keyed by it, so a seat suffix there would break both.
            if let BoardEvent::SwitchIn { name, player_id, .. } = e {
                assert!(player_id.is_some(), "switch-in does not name a player");
                assert!(!name.contains(','), "switch-in name {name:?} carries a suffix");
            }
        }
    }

    #[test]
    fn health_is_reported_as_the_board_expects() {
        let mut b = battle();
        let mut rec = Recorder::default();
        block_on(play_turn(&mut b, [Choice::Move(0), Choice::Move(0)], &mut rec));
        let health = rec.0.iter().find_map(|e| match e {
            BoardEvent::Damage { health, .. } => Some(health.clone()),
            _ => None,
        });
        let health = health.expect("something took damage");
        assert!(health.contains('/'), "health is cur/max, got {health}");
    }

    #[test]
    fn using_a_move_republishes_that_seat_moves() {
        let mut b = battle();
        let mut rec = Recorder::default();
        block_on(play_turn(&mut b, [Choice::Move(0), Choice::Move(0)], &mut rec));
        let update = rec.0.iter().find_map(|e| match e {
            BoardEvent::MovesUpdate { player_id, moves } => Some((player_id.clone(), moves.clone())),
            _ => None,
        });
        let (id, moves) = update.expect("a move list was republished");
        assert!(id == "p1" || id == "p2");
        assert_eq!(moves[0].max_pp, 25, "ember has 25 PP");
        assert_eq!(moves[0].pp, 24, "and one was just spent");
    }
}
