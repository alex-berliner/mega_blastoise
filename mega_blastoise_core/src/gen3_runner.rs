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

/// The active mon as a board position string: `"Name,p1"`.
///
/// The display and LED layers route by parsing the player id back out of this
/// field ([`crate::board_event::mon_player_num`]), so a bare species name
/// would leave HP updates, faints and party syncing silently doing nothing.
fn active_name(battle: &Battle, side: u8) -> String {
    let name = battle.sides[(side - 1) as usize].mon().species.name;
    format!("{name},{}", player_id(side))
}

/// `"cur/max"`, the health string the display layer already parses.
fn health(battle: &Battle, side: u8) -> String {
    let m = battle.sides[(side - 1) as usize].mon();
    format!("{}/{}", m.hp, m.max_hp)
}

/// The active mon's moves in the shape the board speaks.
fn move_slots(battle: &Battle, side: u8) -> Vec<MoveSlot> {
    battle.sides[(side - 1) as usize]
        .mon()
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

async fn switch_in<E: BoardEffects>(battle: &Battle, side: u8, effects: &mut E) {
    let i = (side - 1) as usize;
    let mon = battle.sides[i].mon();
    effects
        .on_event(BoardEvent::SwitchIn {
            name: format!("{},{}", mon.species.name, player_id(side)),
            species: Some(mon.species.name.to_string()),
            player_id: Some(player_id(side)),
            team_slot: Some(battle.sides[i].active as u8),
            moves: move_slots(battle, side),
            speed: Some(mon.spe),
        })
        .await;
}

/// Open the battle: both leads out, both move lists published.
pub async fn announce_start<E: BoardEffects>(battle: &Battle, effects: &mut E) {
    effects.on_event(BoardEvent::BattleStart).await;
    for side in [1u8, 2] {
        switch_in(battle, side, effects).await;
    }
}

/// Resolve one turn and narrate it. Returns true once the battle is over.
pub async fn play_turn<E: BoardEffects>(
    battle: &mut Battle,
    choices: [Choice; 2],
    effects: &mut E,
) -> bool {
    let events = battle.step(choices);
    effects.on_event(BoardEvent::Turn { n: battle.turn }).await;

    for event in events {
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
                        user: Some(active_name(battle, side)),
                        player_id: Some(player_id(side)),
                        name,
                    })
                    .await;
                // PP moved, so the seat's move list is stale.
                effects
                    .on_event(BoardEvent::MovesUpdate {
                        player_id: player_id(side),
                        moves: move_slots(battle, side),
                    })
                    .await;
            }
            Event::Damage { side, effectiveness, crit, .. } => {
                let mon = active_name(battle, side);
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
                        mon: active_name(battle, side),
                        team_slot: Some(battle.sides[(side - 1) as usize].active as u8),
                    })
                    .await;
            }
            Event::Statused { side, status } => {
                effects
                    .on_event(BoardEvent::SetStatus {
                        mon: active_name(battle, side),
                        status: status.abbr().to_string(),
                    })
                    .await;
            }
            Event::Boosted { side, boost, delta } => {
                effects
                    .on_event(BoardEvent::StatChange {
                        mon: active_name(battle, side),
                        stat: boost.label().into(),
                        delta,
                    })
                    .await;
            }
            Event::Flinched { side } => {
                effects
                    .on_event(BoardEvent::Cant { mon: active_name(battle, side), reason: "flinch".into() })
                    .await;
            }
            Event::FullyParalyzed { side } => {
                effects
                    .on_event(BoardEvent::Cant { mon: active_name(battle, side), reason: "par".into() })
                    .await;
            }
            Event::Cant { side, status } => {
                effects
                    .on_event(BoardEvent::Cant {
                        mon: active_name(battle, side),
                        reason: status.abbr().to_string(),
                    })
                    .await;
            }
            Event::Residual { side, .. } => {
                effects
                    .on_event(BoardEvent::Damage {
                        mon: active_name(battle, side),
                        health: health(battle, side),
                    })
                    .await;
            }
            Event::Switched { side, .. } => switch_in(battle, side, effects).await,
            Event::Failed { side } => {
                effects.on_event(BoardEvent::Fail { mon: active_name(battle, side) }).await;
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
pub trait PadSource {
    async fn next(&mut self) -> (u8, Choice);
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
            let (player, choice) = pads.next().await;
            if !(1..=2).contains(&player) {
                continue;
            }
            let i = (player - 1) as usize;
            if chosen[i].is_none() && !ai[i] {
                chosen[i] = Some(choice);
                ui.set_locked(player, true);
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
                BoardEvent::SwitchIn { name, .. } => Some(name),
                _ => None,
            };
            if let Some(mon) = mon {
                assert!(mon_player_num(mon).is_some(), "{mon:?} does not name a player");
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
