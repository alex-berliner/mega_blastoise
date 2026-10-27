//! `leftovers`: Leftovers, a sixteenth of the holder's base max HP back
//! each turn.
//!
//! PS: data/items.ts leftovers

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onResidual(pokemon)`: `this.heal(pokemon.baseMaxhp / 16)`. PS's
/// amount is a float; `heal` raises a positive amount at most 1 to 1 and
/// truncates the rest, so the amount here is the floor, at least 1.
// PS: data/items.ts leftovers.onResidual
fn on_residual(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let amount = (i32::from(b.mon(pokemon).base_maxhp) / 16).max(1);
    b.heal(amount, None, None, EffectId::None);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Residual, on_residual).order(10).sub_order(4)]);
