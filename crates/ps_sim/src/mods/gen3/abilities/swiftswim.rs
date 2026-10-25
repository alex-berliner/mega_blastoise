//! Swift Swim: the holder's Speed is doubled in rain.
//!
//! The ability keeps no state of its own.
//!
//! PS: data/abilities.ts swiftswim (onModifySpe)

use ps_dex::gen3::conditions as c;

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onModifySpe(spe, pokemon)`: `chainModify(2)` when
/// `pokemon.effectiveWeather()` is one of `["raindance", "primordialsea"]`
/// (Primordial Sea is gen 6).
fn on_modify_spe(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    if b.pokemon_effective_weather(pokemon) != c::RAINDANCE {
        return None;
    }
    b.chain_modify_f(2.0);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::ModifySpe, on_modify_spe)]);
