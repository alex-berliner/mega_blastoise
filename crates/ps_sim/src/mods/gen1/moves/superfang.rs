//! Super Fang: the damage is half of the target's current HP (at least
//! 1). Gen 1 keeps the base `damageCallback`; the gen 1 mod only sets
//! `basePower: 1` and `ignoreImmunity: true`.
//!
//! No effect state.
//!
//! PS: data/moves.ts superfang.damageCallback (line 18455; gen 1 inherits it:
//! data/mods/gen1/moves.ts superfang, line 898)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `damageCallback(pokemon, target)`.
// PS: data/moves.ts superfang.damageCallback
fn damage_callback(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    // PS calls the callback with the move's user first, so the event's
    // target is the user and its source the move's target.
    let target = a.source?;
    // `this.clampIntRange(target.getUndynamaxedHP() / 2, 1)`: half the
    // current HP, truncated, and never below 1. Gen 1 has no Dynamax, so
    // `getUndynamaxedHP()` is the HP itself.
    let damage = (i32::from(b.mon(target).hp) / 2).max(1);
    Some(Relay::Int(damage))
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::DamageCallback, damage_callback)]);
