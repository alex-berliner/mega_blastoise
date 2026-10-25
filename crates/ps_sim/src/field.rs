//! Weather as a field effect: `field.setWeather`, `effectiveWeather`,
//! `isWeather`, and the Pokemon's `effectiveWeather`. (`clearWeather` and
//! `suppressingWeather` are in `state.rs`; the weather's handlers run from
//! the field like a pseudo-weather's, gathered by `findFieldEventHandlers`.)
//!
//! The weather state is [`crate::state::Field::weather_state`]. PS builds it
//! with `initEffectState({id})` and no target, so its `effectOrder` is 0;
//! `runEvent` sets its `target` to the field when a handler runs.
//!
//! PS: sim/field.ts setWeather, effectiveWeather, isWeather;
//! sim/pokemon.ts effectiveWeather.

use ps_dex::ConditionId;

use crate::effect::{EffectId, EffectType, EventId};
use crate::relay::Relay;
use crate::state::{Battle, Holder, MonRef, StateRef};

impl Battle {
    /// PS `field.setWeather(status, source, sourceEffect)`: `true`, `false`
    /// or `null` ([`Relay::Null`]).
    // PS: sim/field.ts setWeather
    pub fn set_weather(&mut self, status: ConditionId, source: Option<MonRef>, source_effect: EffectId) -> Relay {
        let source_effect = if source_effect.is_none() { self.frame.effect } else { source_effect };
        // `this.battle.event?.target`: a Pokemon when a handler sets it.
        let source = source.or(self.event.target.mon());

        if self.field.weather == status {
            if self.effect_type(source_effect) == EffectType::Ability {
                if self.gen > 5 || self.field.weather_state.duration == 0 {
                    return Relay::FAIL;
                }
            } else if self.gen > 2 || self.dex.condition_def(status).key.as_str() == "sandstorm" {
                return Relay::FAIL;
            }
        }
        let key = EffectId::Condition(status);
        if let Some(src) = source {
            let result = self.run_event(EventId::SetWeather, Holder::Mon(src), Some(src), key, Some(Relay::Effect(key)));
            // No gen 1-3 effect answers `SetWeather` (the primal weathers
            // are gen 6), so its `-fail ... [from] WEATHER` and `-ability
            // ... [fail]` lines are never written.
            assert!(result.truthy(), "unported: a SetWeather handler refused {key:?}");
        }
        // The weather's handlers run from here on: it must be ported.
        self.require_ported(key);
        let prev_weather = self.field.weather;
        let prev_state = self.field.weather_state;
        let mut st = self.init_effect_state(true, Holder::None);
        if let Some(src) = source {
            st.source = Some(src);
            st.source_slot = Some(self.slot_of(src));
        }
        if let Some(d) = self.dex.condition_def(status).duration {
            st.duration = d as i8;
        }
        self.field.weather = status;
        self.field.weather_state = st;
        self.bury(prev_state);
        if self.has_duration_callback(status) {
            let src = source.expect("setting weather without a source");
            if let Some(d) = self.duration_callback(status, Holder::Mon(src), Some(src), source_effect) {
                self.field.weather_state.duration = d;
            }
        }
        let serial = self.field.weather_state.serial;
        let started = self.single_event(EventId::FieldStart, key, StateRef::Weather, serial, Holder::Field, source, source_effect, None);
        if !started.truthy() {
            self.field.weather = prev_weather;
            self.field.weather_state = prev_state;
            return Relay::FAIL;
        }
        self.each_event(EventId::WeatherChange, source_effect, None);
        Relay::TRUE
    }

    /// PS `field.effectiveWeather()`: the weather, unless Air Lock or Cloud
    /// Nine suppresses it (`ConditionId::NONE` for `''`).
    // PS: sim/field.ts effectiveWeather
    pub fn effective_weather(&mut self) -> ConditionId {
        if self.suppressing_weather() {
            return ConditionId::NONE;
        }
        self.field.weather
    }

    /// PS `field.isWeather(weather)` over a list of weathers (one for the
    /// string form).
    // PS: sim/field.ts isWeather
    pub fn is_weather(&mut self, weathers: &[ConditionId]) -> bool {
        let ours = self.effective_weather();
        weathers.contains(&ours)
    }

    /// PS `pokemon.effectiveWeather(sourceEffect, message)`. Mega Sol (gen
    /// 9) is no gen 1-3 ability, so that branch never applies; Utility
    /// Umbrella (gen 8) hides sun and rain from its holder.
    // PS: sim/pokemon.ts effectiveWeather
    pub fn pokemon_effective_weather(&mut self, m: MonRef) -> ConditionId {
        let weather = self.effective_weather();
        if weather.is_none() {
            return weather;
        }
        let key = self.dex.condition_def(weather).key.as_str();
        if matches!(key, "sunnyday" | "raindance" | "desolateland" | "primordialsea") {
            if let Some(umbrella) = self.dex.item_id("utilityumbrella") {
                if self.has_item(m, umbrella) {
                    return ConditionId::NONE;
                }
            }
        }
        weather
    }
}

#[cfg(test)]
#[path = "field_tests.rs"]
mod tests;
