//! Trace: on switch-in, copies a random foe's ability. In gen 3 the copied
//! ability does not start (`setAbility` runs `Start` only in gens 4+), so a
//! traced Intimidate or Drizzle does nothing until the next switch-in.
//! `onSwitchInPriority: 1`.
//!
//! No ability state.
//!
//! PS: data/mods/gen3/abilities.ts trace

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onStart(pokemon)`: `pokemon.side.randomFoe()` (one PRNG call), then
/// `pokemon.setAbility(target.getAbility(), target)`, whose `-ability`
/// line says `[from] ability: Trace|[of] TARGET`.
// PS: data/mods/gen3/abilities.ts trace.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let target = b.random_foe(pokemon.side)?;
    if b.mon(target).fainted {
        return None;
    }
    let ability = b.mon(target).ability;
    b.set_ability(pokemon, ability, Some(target), EffectId::None, false, false);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Start, on_start)]);
