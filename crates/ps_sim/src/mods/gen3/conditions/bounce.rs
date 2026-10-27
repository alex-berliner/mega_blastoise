//! `bounce`: Bounce's turn in the air (the move's `condition`, added by
//! `twoturnmove`'s `onStart`). Only Gust, Twister, Sky Uppercut and
//! Thunder reach the user (Hurricane, Smack Down and Thousand Arrows are
//! later gens); Gust and Twister hit with double power. `duration: 2`. No
//! effect state.
//!
//! PS: data/moves.ts bounce.condition

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler, Prefix};
use crate::mods::gen3::conditions::semi_invulnerable;
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::moves;

/// `onInvulnerability(target, source, move)`: other moves miss.
// PS: data/moves.ts bounce.condition.onInvulnerability
fn on_invulnerability(_b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    semi_invulnerable::invulnerability(a, semi_invulnerable::IN_THE_AIR)
}

/// `onSourceBasePower(basePower, target, source, move)`: Gust and Twister
/// double.
// PS: data/moves.ts bounce.condition.onSourceBasePower
fn on_source_base_power(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    if matches!(a.effect, EffectId::Move(m) if m == moves::GUST || m == moves::TWISTER) {
        b.chain_modify(2, 1);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Invulnerability, on_invulnerability),
    Handler::on(EventId::BasePower, on_source_base_power).prefix(Prefix::Source),
]);
