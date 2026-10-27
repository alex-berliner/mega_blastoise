//! Attract: the target must be of the opposite gender (an immunity, so
//! `-immune`); the volatile does the rest (`conditions::attract`).
//!
//! PS: data/moves.ts attract.onTryImmunity

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::Gender;

/// `onTryImmunity(target, source)`: male against female or the reverse.
// PS: data/moves.ts attract.onTryImmunity
fn on_try_immunity(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let source = a.source.expect("attract without a source");
    let (tg, sg) = (b.mon(target).gender, b.mon(source).gender);
    Some(Relay::Bool((tg == Gender::Male && sg == Gender::Female) || (tg == Gender::Female && sg == Gender::Male)))
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::TryImmunity, on_try_immunity)]);
