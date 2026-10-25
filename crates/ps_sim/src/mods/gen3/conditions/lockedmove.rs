//! `lockedmove`: Thrash, Outrage and Petal Dance's 2-3 turns locked into
//! the move (their `self.volatileStatus` adds or restarts it on every use),
//! then confusion from fatigue.
//!
//! Effect state: `move` is `move_`; `trueDuration` (the turns of the lock
//! left; JS number, may count below 1 only past the end) is `data[0]`.
//!
//! PS: data/conditions.ts lockedmove

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;

/// `this.effectState.trueDuration`.
const TRUE_DURATION: usize = 0;

/// `onResidual(target)`: a sleeping Pokemon drops the lock (no End, so no
/// confusion); the true duration counts down.
// PS: data/conditions.ts lockedmove.onResidual
fn on_residual(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    if b.mon(target).status == conditions::SLP {
        // `delete target.volatiles['lockedmove']`; the handler goes on with
        // its (now detached) state.
        if let Some(gone) = b.mon_mut(target).volatiles.remove(EffectId::Condition(conditions::LOCKEDMOVE)) {
            b.bury(gone);
        }
    }
    b.effect_state().data[TRUE_DURATION] -= 1;
    None
}

/// `onStart(target, source, effect)`: 2-3 turns, and the move.
// PS: data/conditions.ts lockedmove.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let true_duration = b.prng.random_range(2, 4) as i16;
    let m = match a.effect {
        EffectId::Move(m) => m,
        other => panic!("lockedmove from {other:?}"),
    };
    let st = b.effect_state();
    st.data[TRUE_DURATION] = true_duration;
    st.move_ = m;
    None
}

/// `onRestart()`: the next use keeps the lock going while turns are left.
// PS: data/conditions.ts lockedmove.onRestart
fn on_restart(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    let st = b.effect_state();
    if st.data[TRUE_DURATION] >= 2 {
        st.duration = 2;
    }
    None
}

/// `onEnd(target)`: confusion when the lock ran its course.
// PS: data/conditions.ts lockedmove.onEnd
fn on_end(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    if b.effect_state().data[TRUE_DURATION] > 1 {
        return None;
    }
    b.add_volatile(target, conditions::CONFUSION, None, EffectId::None);
    None
}

/// `onLockMove(pokemon)`: the move (no Dynamax in gen 3).
// PS: data/conditions.ts lockedmove.onLockMove
fn on_lock_move(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    Some(Relay::Move(b.effect_state().move_))
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Residual, on_residual),
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::Restart, on_restart),
    Handler::on(EventId::End, on_end),
    Handler::on(EventId::LockMove, on_lock_move),
]);
