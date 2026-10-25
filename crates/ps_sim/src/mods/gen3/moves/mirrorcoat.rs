//! Mirror Coat: Counter for special moves other than Hidden Power, through
//! the `mirrorcoat` volatile (`conditions/mirrorcoat.rs`). No effect state
//! of its own.
//!
//! PS: data/moves.ts mirrorcoat (`damageCallback`, `beforeTurnCallback`,
//! `onTry`)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;

/// `beforeTurnCallback(pokemon)`: `pokemon.addVolatile('mirrorcoat')`.
// PS: data/moves.ts mirrorcoat.beforeTurnCallback
fn before_turn_callback(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    super::counter_before_turn(b, a, conditions::MIRRORCOAT)
}

/// `onTry(source)`: `false` until a special hit set `slot`.
// PS: data/moves.ts mirrorcoat.onTry
fn on_try(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    super::counter_try(b, a, conditions::MIRRORCOAT)
}

/// `damageCallback(pokemon)`: the stored damage, or 1.
// PS: data/moves.ts mirrorcoat.damageCallback
fn damage_callback(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    super::counter_damage(b, a, conditions::MIRRORCOAT)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::BeforeTurnCallback, before_turn_callback),
    Handler::on(EventId::Try, on_try),
    Handler::on(EventId::DamageCallback, damage_callback),
]);
