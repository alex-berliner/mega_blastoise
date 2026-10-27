//! `rollout`: Rollout's lock and hit counter (its `onModifyMove` adds it;
//! its `basePowerCallback` counts the hits and keeps it, and so the lock,
//! for five). `duration: 1`, `onLockMove: 'rollout'`. Ice Ball's
//! `iceball` is the same condition under its own name and shares these
//! functions.
//!
//! Effect state: `hitCount` is `hit_count`; `contactHitCount` is `data[0]`.
//!
//! PS: data/moves.ts rollout.condition

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::{conditions, moves};
use ps_dex::ConditionId;

/// `this.effectState.contactHitCount`.
pub const CONTACT_HIT_COUNT: usize = 0;

/// `onStart()`: both counts at 0.
// PS: data/moves.ts rollout.condition.onStart
pub fn on_start(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    let st = b.effect_state();
    st.hit_count = 0;
    st.data[CONTACT_HIT_COUNT] = 0;
    None
}

/// `onResidual(target)`: Struggle drops the volatile without its `End`
/// (`delete target.volatiles[id]`).
// PS: data/moves.ts rollout.condition.onResidual
pub fn residual(b: &mut Battle, a: &EventArgs, id: ConditionId) -> Option<Relay> {
    let target = a.target.mon()?;
    if b.mon(target).last_move == moves::STRUGGLE {
        if let Some(gone) = b.mon_mut(target).volatiles.remove(EffectId::Condition(id)) {
            b.bury(gone);
        }
    }
    None
}

fn on_residual(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    residual(b, a, conditions::ROLLOUT)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::value(EventId::LockMove, Relay::Move(moves::ROLLOUT)),
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::Residual, on_residual),
]);
