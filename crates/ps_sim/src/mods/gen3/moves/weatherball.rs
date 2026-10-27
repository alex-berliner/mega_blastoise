//! Weather Ball: gen 3's `onModifyMove` sets the type (and, but in a
//! sandstorm, the Special category) by the weather and doubles the power.
//! The base `onModifyType` is registered too, but gen 3's `useMoveInner`
//! runs no `ModifyType`.
//!
//! PS: data/moves.ts weatherball.onModifyType;
//! data/mods/gen3/moves.ts weatherball.onModifyMove

use ps_dex::{MoveCategory, Type};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;

/// `onModifyType(move, pokemon)`: the type by `pokemon.effectiveWeather()`
/// (Desolate Land, Primordial Sea and Snowscape are later gens).
// PS: data/moves.ts weatherball.onModifyType
fn on_modify_type(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let t = match b.pokemon_effective_weather(pokemon) {
        conditions::SUNNYDAY => Type::Fire,
        conditions::RAINDANCE => Type::Water,
        conditions::SANDSTORM => Type::Rock,
        conditions::HAIL => Type::Ice,
        _ => return None,
    };
    b.active_move_mut().expect("onModifyType without a move").type_ = t;
    None
}

/// `onModifyMove(move)`: by `this.field.effectiveWeather()`, sun Fire, rain
/// Water, hail Ice (each Special), sandstorm Rock; any weather doubles
/// `move.basePower`.
// PS: data/mods/gen3/moves.ts weatherball.onModifyMove
fn on_modify_move(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    let weather = b.effective_weather();
    let am = b.active_move_mut().expect("onModifyMove without a move");
    match weather {
        conditions::SUNNYDAY => (am.type_, am.category) = (Type::Fire, MoveCategory::Special),
        conditions::RAINDANCE => (am.type_, am.category) = (Type::Water, MoveCategory::Special),
        conditions::SANDSTORM => am.type_ = Type::Rock,
        conditions::HAIL => (am.type_, am.category) = (Type::Ice, MoveCategory::Special),
        _ => {}
    }
    if !weather.is_none() {
        am.base_power *= 2;
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::ModifyType, on_modify_type),
    Handler::on(EventId::ModifyMove, on_modify_move),
]);
