//! Rough Skin: a Pokemon that touches the holder loses a sixteenth of its
//! maximum HP.
//!
//! The ability keeps no state of its own.
//!
//! PS: data/abilities.ts roughskin (onDamagingHit), data/mods/gen3/abilities.ts
//! roughskin (onDamagingHit)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

use super::contact_hit;
use super::super::conditions::fraction_of_base_maxhp;

/// `onDamagingHit(damage, target, source, move)`.
fn on_damaging_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let (target, source) = contact_hit(b, a)?;
    // `this.damage(source.baseMaxhp / 16, source, target)`: the attacker
    // takes the damage, the holder is its source.
    let damage = fraction_of_base_maxhp(b, source, 16);
    b.damage(damage, Some(source), Some(target), b.frame.effect);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::DamagingHit, on_damaging_hit).order(1)]);
