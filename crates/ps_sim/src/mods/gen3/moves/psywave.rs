//! Psywave: a random damage from half to one and a half times the user's
//! level, as the fraction PS's JS number holds ([`Relay::Float`]):
//! `spreadDamage` floors it, a Substitute's HP keeps it.
//!
//! PS: data/moves.ts psywave.damageCallback (gen 3 resolves to base, not
//! gen 2's override)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `damageCallback(pokemon)`: `this.random(50, 151) * pokemon.level / 100`.
// PS: data/moves.ts psywave.damageCallback
fn damage_callback(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let roll = b.prng.random_range(50, 151);
    Some(Relay::float(f64::from(roll) * f64::from(b.mon(pokemon).level) / 100.0))
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::DamageCallback, damage_callback)]);
