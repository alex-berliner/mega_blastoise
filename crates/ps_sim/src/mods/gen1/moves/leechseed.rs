//! Leech Seed: Grass types are immune. Gen 1 drops the base `onHit`.
//!
//! PS: data/moves.ts leechseed.onTryImmunity (lines 10232-10234)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::Type;

/// `onTryImmunity(target)`: `return !target.hasType('Grass');`.
// PS: data/moves.ts leechseed.onTryImmunity
fn on_try_immunity(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    Some(Relay::Bool(!b.has_type(target, Type::Grass)))
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::TryImmunity, on_try_immunity)]);
