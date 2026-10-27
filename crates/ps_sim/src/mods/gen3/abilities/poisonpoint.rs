//! Poison Point: a Pokemon that touches the holder has a chance of being
//! poisoned.
//!
//! The ability keeps no state of its own.
//!
//! PS: data/abilities.ts poisonpoint (onDamagingHit), data/mods/gen3/abilities.ts
//! poisonpoint (onDamagingHit)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;

use super::contact_hit;

/// `onDamagingHit(damage, target, source, move)`.
fn on_damaging_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let (target, source) = contact_hit(b, a)?;
    if !b.prng.random_chance(1, 3) {
        return None;
    }
    // `source.trySetStatus("psn", target)`.
    b.try_set_status(source, conditions::PSN, Some(target), b.frame.effect);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::DamagingHit, on_damaging_hit)]);
