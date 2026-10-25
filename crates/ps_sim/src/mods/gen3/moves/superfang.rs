//! Super Fang: half the target's HP.
//!
//! PS: data/moves.ts superfang.damageCallback

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `damageCallback(pokemon, target)`:
/// `clampIntRange(target.getUndynamaxedHP() / 2, 1)` (no Dynamax in gen
/// 3: the HP).
// PS: data/moves.ts superfang.damageCallback
fn damage_callback(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.source.expect("damageCallback without a target");
    Some(Relay::Int((i32::from(b.mon(target).hp) / 2).max(1)))
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::DamageCallback, damage_callback)]);
