//! Drizzle: on switch-in, rain starts, for good (an ability's weather has no
//! duration in gens <= 5, `raindance.onFieldStart`). In gen 3 the ability
//! answers `SwitchIn` only on turn 0 (`getCallback` takes its
//! `onAfterEntryHazard`); later `switchIn` runs `AfterEntryHazard` itself,
//! before the other switch-in effects. `onSwitchInPriority: 3`.
//!
//! No ability state.
//!
//! PS: data/abilities.ts drizzle (onStart), data/mods/gen3/abilities.ts drizzle
//! (onAfterEntryHazard)

use ps_dex::gen3::conditions as c;

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onStart(source)`: `this.field.setWeather('raindance')`; Kyogre holding a Blue Orb
/// (gen 6) is skipped.
// PS: data/abilities.ts drizzle.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let source = a.target.mon()?;
    if b.dex.species_id("kyogre") == Some(b.mon(source).species)
        && b.dex.item_id("blueorb").is_some_and(|orb| b.mon(source).item == orb)
    {
        return None;
    }
    b.set_weather(c::RAINDANCE, None, EffectId::None);
    None
}

/// `onAfterEntryHazard(pokemon)`: `this.effect.onStart.call(this, pokemon)`.
// PS: data/mods/gen3/abilities.ts drizzle.onAfterEntryHazard
fn on_after_entry_hazard(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    // The call by name runs `onStart` (counted as one of its hits).
    #[cfg(feature = "hit-count")]
    b.count_named(EventId::Start);
    on_start(b, a)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::AfterEntryHazard, on_after_entry_hazard),
]);
