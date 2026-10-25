//! `iceball`: Ice Ball's lock and hit counter, Rollout's condition under
//! Ice Ball's name (see `rollout.rs`). `duration: 1`, `onLockMove:
//! 'iceball'`.
//!
//! Effect state: `hitCount` is `hit_count`; `contactHitCount` is `data[0]`.
//!
//! PS: data/moves.ts iceball.condition

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::mods::gen3::conditions::rollout;
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::{conditions, moves};

/// `onResidual(target)`.
// PS: data/moves.ts iceball.condition.onResidual
fn on_residual(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    rollout::residual(b, a, conditions::ICEBALL)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::value(EventId::LockMove, Relay::Move(moves::ICEBALL)),
    Handler::on(EventId::Start, rollout::on_start),
    Handler::on(EventId::Residual, on_residual),
]);
