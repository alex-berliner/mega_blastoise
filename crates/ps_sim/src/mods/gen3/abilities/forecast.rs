//! Forecast: Castform takes the forme of the weather (Sunny, Rainy, Snowy,
//! or back to plain Castform) on switch-in and whenever the weather
//! changes (`WeatherChange`, from `setWeather`, `clearWeather` and Air Lock
//! or Cloud Nine ending). A temporary forme change: `-formechange|MON|
//! FORME|[msg]|[from] ability: Forecast`. In gen 3 it answers `SwitchIn`
//! only on turn 0 (`onAfterEntryHazard`); `onSwitchInPriority: 3`.
//!
//! No ability state.
//!
//! PS: data/abilities.ts forecast (onStart, onWeatherChange),
//! data/mods/gen3/abilities.ts forecast (onAfterEntryHazard)

use ps_protocol::KwFlags;

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::{Battle, Holder};

/// `onStart(pokemon)`: `this.singleEvent('WeatherChange', this.effect,
/// this.effectState, pokemon)`.
// PS: data/abilities.ts forecast.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let frame = b.frame;
    b.single_event(EventId::WeatherChange, frame.effect, frame.state, frame.serial, Holder::Mon(pokemon), None, crate::effect::EffectId::None, None);
    None
}

/// `onWeatherChange(pokemon)`.
// PS: data/abilities.ts forecast.onWeatherChange
fn on_weather_change(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let base = b.mon(pokemon).base_species;
    if b.dex.species_def(base).base_species().as_str() != "Castform" || b.mon(pokemon).transformed {
        return None;
    }
    let current = b.mon(pokemon).species;
    let is = |b: &Battle, id: &str| b.dex.species_id(id) == Some(current);
    let weather = b.pokemon_effective_weather(pokemon);
    let weather = if weather.is_none() { "" } else { b.dex.condition_def(weather).key.as_str() };
    let forme = match weather {
        "sunnyday" | "desolateland" => (!is(b, "castformsunny")).then_some("castformsunny"),
        "raindance" | "primordialsea" => (!is(b, "castformrainy")).then_some("castformrainy"),
        "hail" | "snowscape" => (!is(b, "castformsnowy")).then_some("castformsnowy"),
        _ => (!is(b, "castform")).then_some("castform"),
    };
    if let Some(forme) = forme {
        if b.mon(pokemon).is_active {
            let species = b.dex.species_id(forme).unwrap_or_else(|| panic!("no species {forme}"));
            let effect = b.frame.effect;
            b.forme_change(pokemon, species, effect, false, KwFlags::MSG);
        }
    }
    None
}

/// `onAfterEntryHazard(pokemon)`: `this.effect.onStart.call(this, pokemon)`.
// PS: data/mods/gen3/abilities.ts forecast.onAfterEntryHazard
fn on_after_entry_hazard(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    on_start(b, a)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::WeatherChange, on_weather_change),
    Handler::on(EventId::AfterEntryHazard, on_after_entry_hazard),
]);
