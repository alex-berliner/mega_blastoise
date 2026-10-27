//! Arena Trap: grounded foes (not Flying, no Levitate) cannot switch.
//!
//! No ability state.
//!
//! PS: data/abilities.ts arenatrap

use crate::effect::{EffectDef, EventArgs, EventId, Handler, Prefix};
use crate::relay::Relay;
use crate::state::Battle;

/// `onFoeTrapPokemon(pokemon)`: `pokemon.tryTrap(true)` when it is
/// adjacent to the holder and grounded.
// PS: data/abilities.ts arenatrap.onFoeTrapPokemon
fn on_foe_trap_pokemon(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    // `isAdjacent(undefined)` would throw; the holder is always set here.
    let holder = super::holder(b)?;
    if !b.is_adjacent(pokemon, holder) {
        return None;
    }
    if b.is_grounded(pokemon, false) == Some(true) {
        b.try_trap(pokemon, true);
    }
    None
}

/// `onFoeMaybeTrapPokemon(pokemon, source)`: `maybeTrapped` when it might
/// be grounded (`isGrounded(!pokemon.knownType)`: an unknown type negates
/// the Flying immunity).
// PS: data/abilities.ts arenatrap.onFoeMaybeTrapPokemon
fn on_foe_maybe_trap_pokemon(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let source = match a.source {
        Some(s) => s,
        None => super::holder(b)?,
    };
    if !b.is_adjacent(pokemon, source) {
        return None;
    }
    let negate_immunity = !b.mon(pokemon).known_type;
    if b.is_grounded(pokemon, negate_immunity) == Some(true) {
        b.mon_mut(pokemon).maybe_trapped = true;
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::TrapPokemon, on_foe_trap_pokemon).prefix(Prefix::Foe),
    Handler::on(EventId::MaybeTrapPokemon, on_foe_maybe_trap_pokemon).prefix(Prefix::Foe),
]);
