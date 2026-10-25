//! Poison: the `-status` line and the poison damage at the end of the turn.
//!
//! The poison keeps no state of its own.
//!
//! PS: data/conditions.ts psn

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_protocol::Status;

use super::{add_status_line, fraction_of_base_maxhp};

/// `onStart(target, source, sourceEffect)`: the `-status` line.
// PS: data/conditions.ts psn.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    add_status_line(b, a, Status::Psn)
}

/// `onResidual(pokemon)`: `this.damage(pokemon.baseMaxhp / 8)`.
// PS: data/conditions.ts psn.onResidual
fn on_residual(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let damage = fraction_of_base_maxhp(b, pokemon, 8);
    b.damage(damage, None, None, EffectId::None);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::Residual, on_residual).order(10).sub_order(6),
]);
