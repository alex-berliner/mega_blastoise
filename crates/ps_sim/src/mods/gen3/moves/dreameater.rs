//! Dream Eater: only a sleeping target without a Substitute can be eaten;
//! otherwise `-immune`. The drain is the move's data (`drain: [1, 2]`).
//! No effect state.
//!
//! PS: data/mods/gen4/moves.ts dreameater.onTryImmunity

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;

/// `onTryImmunity(target)`: `target.status === 'slp' &&
/// !target.volatiles['substitute']`.
// PS: data/mods/gen4/moves.ts dreameater.onTryImmunity
fn on_try_immunity(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let asleep = b.mon(target).status == conditions::SLP;
    let sub = b.mon(target).volatiles.contains(EffectId::Condition(conditions::SUBSTITUTE));
    Some(Relay::Bool(asleep && !sub))
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::TryImmunity, on_try_immunity)]);
