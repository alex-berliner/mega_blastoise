//! Shadow Tag (gen 3): every foe is trapped, Shadow Tag holders included
//! (gen 4 frees them). A foe that might be facing it is `maybeTrapped`
//! unless it has Shadow Tag itself.
//!
//! No ability state.
//!
//! PS: data/mods/gen3/abilities.ts shadowtag (onFoeTrapPokemon),
//! data/abilities.ts shadowtag (onFoeMaybeTrapPokemon)

use crate::effect::{EffectDef, EventArgs, EventId, Handler, Prefix};
use crate::relay::Relay;
use crate::state::{Battle, Trapped};

/// `onFoeTrapPokemon(pokemon)`: `pokemon.trapped = true`.
// PS: data/mods/gen3/abilities.ts shadowtag.onFoeTrapPokemon
fn on_foe_trap_pokemon(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    b.mon_mut(pokemon).trapped = Trapped::Yes;
    None
}

/// `onFoeMaybeTrapPokemon(pokemon, source)`.
// PS: data/abilities.ts shadowtag.onFoeMaybeTrapPokemon
fn on_foe_maybe_trap_pokemon(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let source = match a.source {
        Some(s) => s,
        None => super::holder(b)?,
    };
    if !b.is_adjacent(pokemon, source) {
        return None;
    }
    let shadowtag = b.dex.ability_id("shadowtag").expect("Shadow Tag in the dex");
    if !b.has_ability(pokemon, shadowtag) {
        b.mon_mut(pokemon).maybe_trapped = true;
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::TrapPokemon, on_foe_trap_pokemon).prefix(Prefix::Foe),
    Handler::on(EventId::MaybeTrapPokemon, on_foe_maybe_trap_pokemon).prefix(Prefix::Foe),
]);
