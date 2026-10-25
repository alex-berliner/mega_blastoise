//! What Dig, Dive, Fly and Bounce's conditions share: their
//! `onInvulnerability` (only the listed moves reach the user) and Dig and
//! Dive's `onImmunity` (no sandstorm or hail damage). Not an effect.
//!
//! PS: data/moves.ts dig.condition, dive.condition, fly.condition,
//! bounce.condition

use crate::effect::{EffectId, EventArgs};
use crate::relay::Relay;
use ps_dex::gen3::{conditions, moves};
use ps_dex::MoveId;

/// The moves that reach a Pokemon in the air (Fly, Bounce): `gust`,
/// `twister`, `skyuppercut`, `thunder` (the list's `hurricane`,
/// `smackdown` and `thousandarrows` are later gens).
pub const IN_THE_AIR: &[MoveId] = &[moves::GUST, moves::TWISTER, moves::SKYUPPERCUT, moves::THUNDER];

/// `onInvulnerability(target, source, move)`: `undefined` for `hitters`,
/// `false` for every other move.
pub fn invulnerability(a: &EventArgs, hitters: &[MoveId]) -> Option<Relay> {
    if matches!(a.effect, EffectId::Move(m) if hitters.contains(&m)) {
        return None;
    }
    Some(Relay::FAIL)
}

/// `onImmunity(type, pokemon) { if (type === 'sandstorm' || type ===
/// 'hail') return false; }`
pub fn weather_immunity(a: &EventArgs) -> Option<Relay> {
    let weathers = [conditions::SANDSTORM, conditions::HAIL].map(|c| Relay::Effect(EffectId::Condition(c)));
    if a.relay.is_some_and(|r| weathers.contains(&r)) {
        return Some(Relay::FAIL);
    }
    None
}
