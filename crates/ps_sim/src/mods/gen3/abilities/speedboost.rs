//! Speed Boost: the holder's Speed rises by one stage at the end of every
//! turn it has been in.
//!
//! The ability keeps no state of its own.
//!
//! PS: data/abilities.ts speedboost (onResidual)

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onResidual(pokemon)`: `this.boost({spe: 1})`.
fn on_residual(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    if b.mon(pokemon).active_turns == 0 {
        return None;
    }
    // `this.boost({spe: 1})`: the target is the event's target, the effect
    // is the ability.
    b.base_boost([0, 0, 0, 0, 1, 0, 0], pokemon, None, EffectId::None, false, false);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Residual, on_residual).order(10).sub_order(3)]);
