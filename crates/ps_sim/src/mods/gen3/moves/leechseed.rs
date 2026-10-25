//! Leech Seed: Grass types are immune; the volatile does the rest
//! (`conditions::leechseed`).
//!
//! PS: data/moves.ts leechseed.onTryImmunity

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::Type;

/// `onTryImmunity(target)`: `!target.hasType('Grass')`.
// PS: data/moves.ts leechseed.onTryImmunity
fn on_try_immunity(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    Some(Relay::Bool(!b.has_type(target, Type::Grass)))
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::TryImmunity, on_try_immunity)]);
