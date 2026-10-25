//! Helping Hand: its target is an adjacent ally, so in singles
//! `useMoveInner` finds none (`-notarget`) and nothing here runs; the
//! `onTryHit` is ported for completeness.
//!
//! PS: data/moves.ts helpinghand.onTryHit

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onTryHit(target)`: `false` once the ally has moved (and did not just
/// switch in).
// PS: data/moves.ts helpinghand.onTryHit
fn on_try_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    if !b.mon(target).newly_switched && b.will_move(target).is_none() {
        return Some(Relay::FAIL);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::TryHit, on_try_hit)]);
