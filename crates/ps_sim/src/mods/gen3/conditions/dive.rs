//! `dive`: Dive's underwater turn (the move's `condition`, added by
//! `twoturnmove`'s `onStart`). Only Surf and Whirlpool reach the user, and
//! they deal double damage; sandstorm and hail do not hurt it.
//! `duration: 2`. No effect state.
//!
//! PS: data/moves.ts dive.condition

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler, Prefix};
use crate::mods::gen3::conditions::semi_invulnerable;
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::moves;

const HITTERS: &[ps_dex::MoveId] = &[moves::SURF, moves::WHIRLPOOL];

/// `onImmunity(type, pokemon)`: no sandstorm or hail damage underwater.
// PS: data/moves.ts dive.condition.onImmunity
fn on_immunity(_b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    semi_invulnerable::weather_immunity(a)
}

/// `onInvulnerability(target, source, move)`: other moves miss.
// PS: data/moves.ts dive.condition.onInvulnerability
fn on_invulnerability(_b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    semi_invulnerable::invulnerability(a, HITTERS)
}

/// `onSourceModifyDamage(damage, source, target, move)`: Surf and
/// Whirlpool double.
// PS: data/moves.ts dive.condition.onSourceModifyDamage
fn on_source_modify_damage(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    if matches!(a.effect, EffectId::Move(m) if HITTERS.contains(&m)) {
        b.chain_modify(2, 1);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Immunity, on_immunity),
    Handler::on(EventId::Invulnerability, on_invulnerability),
    Handler::on(EventId::ModifyDamage, on_source_modify_damage).prefix(Prefix::Source),
]);
