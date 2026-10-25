//! Spit Up: 100 power per `stockpile` layer, then the layers go. Fails
//! without them. No effect state.
//!
//! PS: data/moves.ts spitup

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::mods::gen3::moves::stockpile::layers;
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;

/// `basePowerCallback(pokemon)`: `false` without layers.
// PS: data/moves.ts spitup.basePowerCallback
fn base_power_callback(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    match layers(b, pokemon) {
        0 => Some(Relay::FAIL),
        n => Some(Relay::Int(i32::from(n) * 100)),
    }
}

/// `onTry(source)`: `!!source.volatiles['stockpile']`.
// PS: data/moves.ts spitup.onTry
fn on_try(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let source = a.target.mon()?;
    Some(Relay::Bool(b.mon(source).volatiles.contains(EffectId::Condition(conditions::STOCKPILE))))
}

/// `onAfterMove(pokemon)`: the layers go.
// PS: data/moves.ts spitup.onAfterMove
fn on_after_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    b.remove_volatile(pokemon, conditions::STOCKPILE);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::BasePowerCallback, base_power_callback),
    Handler::on(EventId::Try, on_try),
    Handler::on(EventId::AfterMove, on_after_move),
]);
