//! Air Lock: while it is out, every weather is suppressed
//! (`suppressWeather: true`, read by `field.suppressingWeather`). Gen 3
//! writes no `-ability` line for it (that is gen 5's `onSwitchIn`).
//! In gen 3 it answers `SwitchIn` only on turn 0 (`onAfterEntryHazard`,
//! as the weather abilities); `onSwitchInPriority: 4`.
//!
//! Ability state: `ending` (PS `abilityState.ending`), set while the
//! ability is being removed so `suppressingWeather` no longer counts it.
//!
//! PS: data/mods/gen4/abilities.ts airlock (onStart), data/abilities.ts airlock
//! (onEnd), data/mods/gen3/abilities.ts airlock (onAfterEntryHazard)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onStart(pokemon)`: `pokemon.abilityState.ending = false`.
// PS: data/mods/gen4/abilities.ts airlock.onStart
fn on_start(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    b.effect_state().ending = false;
    None
}

/// `onEnd(pokemon)`: `ending = true`, then `eachEvent('WeatherChange',
/// this.effect)` so Forecast sees the weather come back.
// PS: data/abilities.ts airlock.onEnd
fn on_end(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    b.effect_state().ending = true;
    let effect = b.frame.effect;
    b.each_event(EventId::WeatherChange, effect, None);
    None
}

/// `onAfterEntryHazard(pokemon)`: `this.effect.onStart.call(this, pokemon)`.
// PS: data/mods/gen3/abilities.ts airlock.onAfterEntryHazard
fn on_after_entry_hazard(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    on_start(b, a)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::End, on_end),
    Handler::on(EventId::AfterEntryHazard, on_after_entry_hazard),
]);
