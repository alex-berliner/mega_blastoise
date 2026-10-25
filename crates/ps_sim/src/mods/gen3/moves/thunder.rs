//! Thunder: never misses in rain, 50% accurate in sun.
//!
//! PS: data/moves.ts thunder.onModifyMove

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;

/// `onModifyMove(move, pokemon, target)`: by `target?.effectiveWeather()`,
/// rain makes `move.accuracy = true`, sun `50` (Primordial Sea and
/// Desolate Land are gen 6).
// PS: data/moves.ts thunder.onModifyMove
fn on_modify_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let Some(target) = a.source else { return None };
    let weather = b.pokemon_effective_weather(target);
    let am = b.active_move_mut().expect("onModifyMove without a move");
    match weather {
        conditions::RAINDANCE => am.accuracy_x256 = None,
        conditions::SUNNYDAY => am.accuracy_x256 = Some(50 * 256),
        _ => {}
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::ModifyMove, on_modify_move)]);
