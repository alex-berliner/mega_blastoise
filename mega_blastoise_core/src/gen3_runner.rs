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

use gen3_battle::{
    battle::{Battle, Choice, Event},
    Type,
};

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

/// What a seat may choose from this turn. Platforms use it to bound their
/// cursor before blocking on a press.
#[derive(Clone, Copy, Debug)]
pub struct SeatPrompt {
    /// 1 or 2.
    pub player: u8,
    pub n_moves: u8,
    pub n_party: u8,
    /// The active mon fainted: this seat must switch, not attack.
    pub forced_switch: bool,
}

/// One committed press from a seat, already classified by the platform's
/// cursor UI: a move slot or a party index. This is ALL a platform supplies —
/// the turn's lifecycle (who is prompted, when an AI commits, when a seat
/// locks) lives in [`run_battle`], so the two platforms cannot order it
/// differently.
pub enum PadChoice {
    /// A seat committed to an action.
    Pick { player: u8, choice: Choice },
    /// A seat took its committed action back.
    Cancel { player: u8 },
}

pub trait PadSource {
    /// Throw away anything pressed before this turn opened. Presses made
    /// while the narration was still running belong to the text the player
    /// was skipping, not to the choice they have not been asked for yet —
    /// and without this they arrive the instant the turn starts, lock the
    /// seat in immediately, and the player never sees a menu at all. Gen 1
    /// has always dropped them; this is the same rule, said out loud.
    fn flush(&mut self);
    async fn next(&mut self) -> PadChoice;
}

/// The per-seat UI state the runner drives as the turn progresses. The web
/// backs this with its `DeviceSession`; the firmware will back it with the
/// same one.
pub trait UiHook {
    fn begin_turn(&mut self, prompt: SeatPrompt);
    fn set_locked(&mut self, player: u8, locked: bool);
}

fn prompts_for(battle: &Battle) -> [SeatPrompt; 2] {
    [1u8, 2].map(|player| {
        let side = &battle.sides[(player - 1) as usize];
        SeatPrompt {
            player,
            n_moves: side.mon().moves.len().max(1) as u8,
            n_party: side.party.len().max(1) as u8,
            forced_switch: side.mon().fainted(),
        }
    })
}

/// The stock policy for a robot seat: a random legal pick, forced switches
/// honoured. Core owns it for the same reason `RandomAi` is core on the Gen 1
/// side — an AI that picked differently per platform would be drift.
fn ai_choice(rng: &mut gen3_battle::Rng, p: SeatPrompt) -> Choice {
    if p.forced_switch {
        Choice::Switch(rng.below(p.n_party as u32) as usize)
    } else {
        Choice::Move(rng.below(p.n_moves as u32) as usize)
    }
}

/// Run a Gen 3 battle to its end.
///
/// The counterpart of the Gen 1 runner: same responsibilities, same place in
/// the stack, so a caller picks an engine rather than a shape. Per turn: both
/// seats' cursors are re-bounded, AI seats commit instantly and lock, human
/// seats are prompted, and each human commit locks its seat.
pub async fn run_battle<E: BoardEffects, P: PadSource, U: UiHook>(
    battle: &mut Battle,
    ai: [bool; 2],
    ai_rng: &mut gen3_battle::Rng,
    pads: &mut P,
    ui: &mut U,
    effects: &mut E,
) {
    announce_start(battle, effects).await;
    while !battle.over() {
        // The turn is about to be asked for, so nothing pressed before now
        // counts as an answer to it.
        pads.flush();
        let prompts = prompts_for(battle);
        let mut chosen: [Option<Choice>; 2] = [None, None];
        for p in prompts {
            let i = (p.player - 1) as usize;
            ui.begin_turn(p);
            if ai[i] {
                chosen[i] = Some(ai_choice(ai_rng, p));
                ui.set_locked(p.player, true);
            } else {
                // Cue the human seat. This is what takes a display off the
                // narration and back to the choice screen.
                effects
                    .on_event(BoardEvent::Prompt {
                        player_id: player_id(p.player),
                        kind: PromptKind::ChooseMove,
                    })
                    .await;
            }
        }
        while chosen[0].is_none() || chosen[1].is_none() {
            let ev = pads.next().await;
            let player = match ev {
                PadChoice::Pick { player, .. } | PadChoice::Cancel { player } => player,
            };
            if !(1..=2).contains(&player) {
                continue;
            }
            let i = (player - 1) as usize;
            if ai[i] {
                continue;
            }
            match ev {
                PadChoice::Pick { choice, .. } if chosen[i].is_none() => {
                    chosen[i] = Some(choice);
                    ui.set_locked(player, true);
                }
                // Taking it back puts the seat on its menu again, which needs
                // the same cue the turn opened with — otherwise the display
                // sits on the waiting screen with nothing to press.
                PadChoice::Cancel { .. } if chosen[i].is_some() => {
                    chosen[i] = None;
                    ui.set_locked(player, false);
                    effects
                        .on_event(BoardEvent::Prompt {
                            player_id: player_id(player),
                            kind: PromptKind::ChooseMove,
                        })
                        .await;
                }
                _ => {}
            }
        }
        if play_turn(battle, [chosen[0].unwrap(), chosen[1].unwrap()], effects).await {
            break;
        }
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

/// A move's type, for callers that colour a badge without reaching into the
/// engine's types themselves.
pub fn move_type_name(t: Type) -> &'static str {
    t.name()
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
