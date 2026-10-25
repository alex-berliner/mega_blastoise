//! Yawn: fails against a target with a status or one that cannot sleep;
//! the volatile does the rest (`conditions::yawn`).
//!
//! PS: data/moves.ts yawn.onTryHit

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;

/// `onTryHit(target)`: `false` for `target.status ||
/// !target.runStatusImmunity('slp')`.
// PS: data/moves.ts yawn.onTryHit
fn on_try_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    if !b.mon(target).status.is_none() || !b.run_status_immunity(target, conditions::SLP, false) {
        return Some(Relay::FAIL);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::TryHit, on_try_hit)]);
