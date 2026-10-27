//! Counter: at -5 priority, twice the damage the last physical move (or
//! Hidden Power) dealt the user this turn, back at its attacker. Before
//! the turn the user readies the `counter` volatile, which records that
//! damage (`conditions/counter.rs`). No effect state of its own.
//!
//! PS: data/moves.ts counter (`damageCallback`, `beforeTurnCallback`,
//! `onTry`)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;

/// `beforeTurnCallback(pokemon)`: `pokemon.addVolatile('counter')`.
// PS: data/moves.ts counter.beforeTurnCallback
fn before_turn_callback(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    super::counter_before_turn(b, a, conditions::COUNTER)
}

/// `onTry(source)`: `false` until a counterable hit set `slot`.
// PS: data/moves.ts counter.onTry
fn on_try(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    super::counter_try(b, a, conditions::COUNTER)
}

/// `damageCallback(pokemon)`: the stored damage, or 1.
// PS: data/moves.ts counter.damageCallback
fn damage_callback(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    super::counter_damage(b, a, conditions::COUNTER)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::BeforeTurnCallback, before_turn_callback),
    Handler::on(EventId::Try, on_try),
    Handler::on(EventId::DamageCallback, damage_callback),
]);
