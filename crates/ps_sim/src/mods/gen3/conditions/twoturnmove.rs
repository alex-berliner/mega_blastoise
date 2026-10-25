//! `twoturnmove`: the charge turn of Razor Wind, Solar Beam, Skull Bash,
//! Sky Attack, Dig, Fly, Dive and Bounce (their `onTryMove` adds it). It
//! adds the move's own volatile (Dig's and Fly's semi-invulnerability; a
//! handler-less one for the others, which their `onTryMove` removes on the
//! second turn) and locks the user into the move. `duration: 2`.
//!
//! Effect state: `move` is `move_`. The move's own volatile keeps
//! `targetLoc` (the target of the second turn) in `data[0]`.
//!
//! PS: data/conditions.ts twoturnmove

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::{Battle, Holder};
use ps_dex::gen3::conditions;

/// `attacker.volatiles[move.id].targetLoc`.
pub const TARGET_LOC: usize = 0;

/// `onStart(attacker, defender, effect)`: the move, its volatile, the
/// second turn's target, `[still]` and `PrepareHit`.
// PS: data/conditions.ts twoturnmove.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let attacker = a.target.mon()?;
    let EffectId::Move(m) = a.effect else { panic!("twoturnmove from {:?}", a.effect) };
    b.effect_state().move_ = m;
    let key = b.move_volatile_key(m);
    b.add_volatile_key(attacker, key, None, EffectId::None);
    let mut move_target_loc = b.mon(attacker).last_move_target_loc;
    // `effect.sourceEffect`: the move object's (Metronome, Sleep Talk...).
    let called = b.active_move().is_some_and(|am| am.id == m && !am.source_effect.is_none());
    let mut defender = a.source;
    if called && b.dex.move_def(m).target != ps_dex::MoveTarget::Self_ {
        // A called move needs its random target now.
        if defender.is_some_and(|d| b.mon(d).fainted) {
            // `this.sample(attacker.foes(true))`: the foe's actives, fainted
            // ones included.
            let foe = 1 - attacker.side;
            let mut foes = [attacker; 1];
            let mut n = 0;
            for pos in 0..b.sides[foe as usize].active.len() as u8 {
                if let Some(f) = b.active_at(foe, pos) {
                    foes[n] = f;
                    n += 1;
                }
            }
            defender = Some(*b.prng.sample(&foes[..n]));
        }
        let d = defender.expect("a called two-turn move without a defender");
        move_target_loc = b.get_loc_of(attacker, d);
    }
    if let Some(st) = b.mon_mut(attacker).volatiles.get_mut(key) {
        st.data[TARGET_LOC] = i16::from(move_target_loc);
    }
    b.attr_last_move_still();
    // Side effects of hitting (Protean, Libero: gen 6).
    b.run_event(EventId::PrepareHit, Holder::Mon(attacker), defender, a.effect, None);
    None
}

/// `onEnd(target)`: the move's own volatile goes too.
// PS: data/conditions.ts twoturnmove.onEnd
fn on_end(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let m = b.effect_state().move_;
    let key = b.move_volatile_key(m);
    b.remove_volatile_key(target, key);
    None
}

/// `onLockMove()`: the move.
// PS: data/conditions.ts twoturnmove.onLockMove
fn on_lock_move(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    Some(Relay::Move(b.effect_state().move_))
}

/// `onMoveAborted(pokemon)`: a move that did not happen drops the charge.
// PS: data/conditions.ts twoturnmove.onMoveAborted
fn on_move_aborted(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    b.remove_volatile(pokemon, conditions::TWOTURNMOVE);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::End, on_end),
    Handler::on(EventId::LockMove, on_lock_move),
    Handler::on(EventId::MoveAborted, on_move_aborted),
]);
