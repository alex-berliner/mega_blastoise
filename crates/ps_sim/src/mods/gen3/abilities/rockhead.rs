//! Rock Head: the holder takes no recoil damage (Struggle's recoil still
//! hurts).
//!
//! The ability keeps no state of its own.
//!
//! PS: data/abilities.ts rockhead (onDamage)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::moves;

/// `onDamage(damage, target, source, effect)`: `null` (no damage) when
/// the effect is a recoil and the move in flight is not Struggle.
fn on_damage(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    if b.effect_key(a.effect) != "recoil" {
        return None;
    }
    let Some(active) = b.active_move() else {
        panic!("unported: Rock Head with no move in flight");
    };
    if active.id != moves::STRUGGLE {
        return Some(Relay::Null);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Damage, on_damage)]);
