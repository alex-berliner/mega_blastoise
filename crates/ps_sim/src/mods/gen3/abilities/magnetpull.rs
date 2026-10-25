//! Magnet Pull (gen 3): Steel-type Pokemon next to the holder cannot
//! switch. `onAny...`, so the holder's own events answer too (a Steel
//! holder is not adjacent to itself).
//!
//! No ability state.
//!
//! PS: data/mods/gen3/abilities.ts magnetpull

use ps_dex::Type;

use crate::effect::{EffectDef, EventArgs, EventId, Handler, Prefix};
use crate::relay::Relay;
use crate::state::Battle;

/// `onAnyTrapPokemon(pokemon)`.
// PS: data/mods/gen3/abilities.ts magnetpull.onAnyTrapPokemon
fn on_any_trap_pokemon(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let holder = super::holder(b)?;
    if b.has_type(pokemon, Type::Steel) && b.is_adjacent(pokemon, holder) {
        b.try_trap(pokemon, true);
    }
    None
}

/// `onAnyMaybeTrapPokemon(pokemon, source)`: `!pokemon.knownType ||
/// pokemon.hasType('Steel')`.
// PS: data/mods/gen3/abilities.ts magnetpull.onAnyMaybeTrapPokemon
fn on_any_maybe_trap_pokemon(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let source = match a.source {
        Some(s) => s,
        None => super::holder(b)?,
    };
    if !b.is_adjacent(pokemon, source) {
        return None;
    }
    if !b.mon(pokemon).known_type || b.has_type(pokemon, Type::Steel) {
        b.mon_mut(pokemon).maybe_trapped = true;
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::TrapPokemon, on_any_trap_pokemon).prefix(Prefix::Any),
    Handler::on(EventId::MaybeTrapPokemon, on_any_maybe_trap_pokemon).prefix(Prefix::Any),
]);
