//! `fly`: Fly's turn in the air (the move's `condition`, added by
//! `twoturnmove`'s `onStart`). Only Gust, Twister, Sky Uppercut and
//! Thunder reach the user (Hurricane, Smack Down and Thousand Arrows are
//! later gens); Gust and Twister deal double damage. `duration: 2`. No
//! effect state.
//!
//! PS: data/moves.ts fly.condition

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler, Prefix};
use crate::mods::gen3::conditions::semi_invulnerable;
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::moves;

/// `onSourceModifyDamage(damage, source, target, move)`: Gust and Twister
/// double.
// PS: data/moves.ts fly.condition.onSourceModifyDamage
fn on_source_modify_damage(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    if matches!(a.effect, EffectId::Move(m) if m == moves::GUST || m == moves::TWISTER) {
        b.chain_modify(2, 1);
    }
    None
}

/// `onInvulnerability(target, source, move)`: other moves miss.
// PS: data/moves.ts fly.condition.onInvulnerability
fn on_invulnerability(_b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    semi_invulnerable::invulnerability(a, semi_invulnerable::IN_THE_AIR)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Invulnerability, on_invulnerability),
    Handler::on(EventId::ModifyDamage, on_source_modify_damage).prefix(Prefix::Source),
]);
