//! Effect Spore: a Pokemon that touches the holder has a chance of
//! falling asleep, being paralyzed or being poisoned.
//!
//! The ability keeps no state of its own.
//!
//! PS: data/abilities.ts effectspore (onDamagingHit), data/mods/gen3/abilities.ts
//! effectspore (onDamagingHit)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;

use super::contact_hit;

/// `onDamagingHit(damage, target, source, move)`.
fn on_damaging_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let (target, source) = contact_hit(b, a)?;
    if !b.prng.random_chance(1, 10) {
        return None;
    }
    // `const status = this.sample(["slp", "par", "psn"]);`.
    let status = *b.prng.sample(&[conditions::SLP, conditions::PAR, conditions::PSN]);
    b.try_set_status(source, status, Some(target), b.frame.effect);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::DamagingHit, on_damaging_hit)]);
