//! Drives a [`gen3_battle`] battle and narrates it as [`BoardEvent`]s.
//!
//! This is the Gen 3 half of the engine seam. The Gen 1 half is
//! [`crate::battle_runner`] and is deliberately untouched: a Gen 1 battle runs
//! exactly the code it always ran, and picking a ruleset picks which of the
//! two a caller drives.
//!
//! The two runners differ in who owns the loop. Gen 1's engine hands out
//! requests and wants to be polled, so `run_battle` owns an async loop. The
//! Gen 3 engine resolves a whole turn from two choices, so this side exposes
//! one turn at a time and leaves the loop to the platform, which already has
//! the input plumbing. That keeps this module free of async generics and, more
//! usefully, testable without a bus.

extern crate alloc;

use alloc::{format, string::String, string::ToString, vec::Vec};

use gen3_battle::{
    battle::{Battle, Choice, Event},
    Type,
};

use crate::battle_effects::BoardEffects;
use crate::board_event::{BoardEvent, MoveSlot};

/// `"p1"` / `"p2"` for a 1-based side.
fn player_id(side: u8) -> String {
    if side == 1 { "p1".to_string() } else { "p2".to_string() }
}

/// The active mon's display name on `side` (1-based).
fn active_name(battle: &Battle, side: u8) -> String {
    battle.sides[(side - 1) as usize].mon().species.name.to_string()
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
            name: mon.species.name.to_string(),
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
