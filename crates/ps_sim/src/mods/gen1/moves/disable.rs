//! Disable: gen 1's fails only when the target has no move with PP left.
//!
//! PS: data/mods/gen1/moves.ts disable.onTryHit (lines 252-254)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onTryHit(target)`: `target.moveSlots.some(ms => ms.pp > 0)`.
// PS: data/mods/gen1/moves.ts disable.onTryHit
fn on_try_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let some = b.mon(target).move_slots.iter().any(|ms| !ms.id.is_none() && ms.pp > 0);
    Some(Relay::Bool(some))
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::TryHit, on_try_hit)]);
