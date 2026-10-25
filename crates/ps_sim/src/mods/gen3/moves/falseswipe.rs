//! False Swipe: leaves the target at 1 HP at least. Its `onDamage` runs as
//! `spreadDamage`'s `onEffect` handler (the move is the damage's effect).
//!
//! PS: data/moves.ts falseswipe.onDamage (`onDamagePriority: -20`)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onDamage(damage, target, source, effect)`: `if (damage >= target.hp)
/// return target.hp - 1`.
// PS: data/moves.ts falseswipe.onDamage
fn on_damage(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let Some(Relay::Int(damage)) = a.relay else { panic!("falseswipe.onDamage of {:?}", a.relay) };
    let hp = i32::from(b.mon(target).hp);
    if damage >= hp {
        return Some(Relay::Int(hp - 1));
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Damage, on_damage).priority(-20)]);
