//! Rain Dish: the holder heals a sixteenth of its maximum HP at the end
//! of every turn in rain.
//!
//! The ability keeps no state of its own.
//!
//! PS: data/mods/gen3/abilities.ts raindish (onResidual)

use ps_dex::gen3::conditions as c;

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

use super::super::conditions::fraction_of_base_maxhp;

/// `onResidual(pokemon)`: in rain (`["raindance", "primordialsea"]`;
/// Primordial Sea is gen 6), `this.heal(pokemon.baseMaxhp / 16)`: the
/// event's target, no source, the ability as the effect.
fn on_residual(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    if b.pokemon_effective_weather(pokemon) != c::RAINDANCE {
        return None;
    }
    let amount = fraction_of_base_maxhp(b, pokemon, 16);
    b.heal(amount, Some(pokemon), None, EffectId::None);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Residual, on_residual).order(10).sub_order(3)]);
