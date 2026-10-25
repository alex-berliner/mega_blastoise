//! Hail: the weather. `duration: 5` (8 with Icy Rock), endless from an
//! ability in gens <= 5. A sixteenth of their HP off every non-Ice Pokemon each turn.
//!
//! The field's weather state has no fields of its own beyond `duration`.
//!
//! PS: data/conditions.ts hail

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;
use ps_protocol::{Kw, KwFlags, Msg, Weather};

/// `durationCallback(source, effect)`: 8 turns with Icy Rock (a gen 4 item),
/// else 5.
// PS: data/conditions.ts hail.durationCallback
fn duration_callback(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let rock = b.dex.item_id("icyrock");
    if let (Some(r), Some(source)) = (rock, a.target.mon()) {
        if b.has_item(source, r) {
            return Some(Relay::Int(8));
        }
    }
    Some(Relay::Int(5))
}

/// `onFieldStart(field, source, effect)`: the `-weather` line; an
/// ability's weather lasts (gens <= 5).
// PS: data/conditions.ts hail.onFieldStart
fn on_field_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    if matches!(a.effect, EffectId::Ability(_)) && b.gen <= 5 {
        b.effect_state().duration = 0;
    }
    let kw = super::weather_start_kw(b, a);
    b.add(Msg::Weather { weather: Weather::Hail, kw });
    None
}

/// `onFieldResidual()`: the `[upkeep]` line, then the `Weather` event while the hail is not suppressed.
// PS: data/conditions.ts hail.onFieldResidual
fn on_field_residual(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    b.add(Msg::Weather { weather: Weather::Hail, kw: Kw::flags(KwFlags::UPKEEP) });
    if b.is_weather(&[conditions::HAIL]) {
        b.each_event(EventId::Weather, EffectId::None, None);
    }
    None
}

/// `onFieldEnd()`: `-weather|none`.
// PS: data/conditions.ts hail.onFieldEnd
fn on_field_end(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    b.add(Msg::Weather { weather: Weather::None, kw: Kw::NONE });
    None
}

/// `onWeather(target)`: `this.damage(target.baseMaxhp / 16)` (the
/// weather's type immunity is `spreadDamage`'s).
// PS: data/conditions.ts hail.onWeather
fn on_weather(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let damage = super::fraction_of_base_maxhp(b, target, 16);
    b.damage(damage, None, None, EffectId::None);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::DurationCallback, duration_callback),
    Handler::on(EventId::Weather, on_weather),
    Handler::on(EventId::FieldStart, on_field_start),
    Handler::on(EventId::FieldResidual, on_field_residual).order(8),
    Handler::on(EventId::FieldEnd, on_field_end),
]);
