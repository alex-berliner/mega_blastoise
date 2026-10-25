//! Thunder: its accuracy depends on the weather (rain makes it sure to
//! hit, sun drops it to 50%). Gen 1 has no weather, so the accuracy is
//! the move's own 70%.
//!
//! No effect state.
//!
//! PS: data/moves.ts thunder.onModifyMove (line 19432; gen 1 inherits it: the
//! gen 1 mod, line 907, changes only its data)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onModifyMove(move, pokemon, target)`.
// PS: data/moves.ts thunder.onModifyMove
fn on_modify_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    // PS hands the move to the handler as the relay variable, so the
    // event's target is the user and its source the move's target.
    // `target?.effectiveWeather()`: no target, no weather.
    if a.source.is_none() {
        return None;
    }
    // `target.effectiveWeather()`: gen 1 has no abilities and no
    // weather, so this is the field's weather and nothing else.
    let weather = b.field.weather;
    if weather.is_none() {
        return None;
    }
    let key = b.dex.condition_def(weather).key.as_str();
    match key {
        "raindance" | "primordialsea" => {
            // `move.accuracy = true`.
            if let Some(am) = b.active_move_mut() {
                am.accuracy_x256 = None;
            }
        }
        "sunnyday" | "desolateland" => {
            // `move.accuracy = 50`.
            if let Some(am) = b.active_move_mut() {
                am.accuracy_x256 = Some(50 * 256);
            }
        }
        _ => {}
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::ModifyMove, on_modify_move)]);
