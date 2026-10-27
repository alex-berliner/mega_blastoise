//! Rain Dance: the weather. `duration: 5` (8 with Damp Rock), endless from an
//! ability in gens <= 5. Water moves hit harder and Fire moves softer.
//!
//! The field's weather state has no fields of its own beyond `duration`.
//!
//! PS: data/conditions.ts raindance

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;
use ps_protocol::{Kw, KwFlags, Msg, Weather};
use ps_dex::Type;
use ps_protocol::{Text, TextId};

/// `durationCallback(source, effect)`: 8 turns with Damp Rock (a gen 4 item),
/// else 5.
// PS: data/conditions.ts raindance.durationCallback
fn duration_callback(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let rock = b.dex.item_id("damprock");
    if let (Some(r), Some(source)) = (rock, a.target.mon()) {
        if b.has_item(source, r) {
            return Some(Relay::Int(8));
        }
    }
    Some(Relay::Int(5))
}

/// `onFieldStart(field, source, effect)`: the `-weather` line; an
/// ability's weather lasts (gens <= 5).
// PS: data/conditions.ts raindance.onFieldStart
fn on_field_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    if matches!(a.effect, EffectId::Ability(_)) && b.gen <= 5 {
        b.effect_state().duration = 0;
    }
    let kw = super::weather_start_kw(b, a);
    b.add(Msg::Weather { weather: Weather::RainDance, kw });
    None
}

/// `onFieldResidual()`: the `[upkeep]` line, then the `Weather` event.
// PS: data/conditions.ts raindance.onFieldResidual
fn on_field_residual(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    b.add(Msg::Weather { weather: Weather::RainDance, kw: Kw::flags(KwFlags::UPKEEP) });
    b.each_event(EventId::Weather, EffectId::None, None);
    None
}

/// `onFieldEnd()`: `-weather|none`.
// PS: data/conditions.ts raindance.onFieldEnd
fn on_field_end(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    b.add(Msg::Weather { weather: Weather::None, kw: Kw::NONE });
    None
}

/// `onWeatherModifyDamage(damage, attacker, defender, move)`: Water moves
/// 1.5x, Fire moves 0.5x, under the defender's rain.
// PS: data/conditions.ts raindance.onWeatherModifyDamage
fn on_weather_modify_damage(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let defender = a.source?;
    if b.pokemon_effective_weather(defender) != conditions::RAINDANCE {
        return None;
    }
    match super::event_move_type(b, a) {
        Type::Water => {
            b.debug(Text::new(TextId::RainWaterBoost));
            b.chain_modify_f(1.5);
        }
        Type::Fire => {
            b.debug(Text::new(TextId::RainFireSuppress));
            b.chain_modify_f(0.5);
        }
        _ => {}
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::DurationCallback, duration_callback),
    Handler::on(EventId::WeatherModifyDamage, on_weather_modify_damage),
    Handler::on(EventId::FieldStart, on_field_start),
    Handler::on(EventId::FieldResidual, on_field_residual).order(8),
    Handler::on(EventId::FieldEnd, on_field_end),
]);
