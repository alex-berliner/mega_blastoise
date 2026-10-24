//! Psywave: the damage is a random roll below one and a half times the
//! user's level, and the levels the cartridge would soft-lock on do no
//! damage at all.
//!
//! No effect state.
//!
//! PS: data/mods/gen1/moves.ts psywave.damageCallback (line 587; gen 1's own
//! handler; the base move has none)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_protocol::TextId;

/// `damageCallback(pokemon)`.
// PS: data/mods/gen1/moves.ts psywave.damageCallback
fn damage_callback(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    // PS calls the callback with the move's user first, so the event's
    // target is the user.
    let pokemon = a.target.mon()?;
    let level = i32::from(b.mon(pokemon).level);
    // `((pokemon.level + (pokemon.level >> 1)) & 0xff) < 2`: the levels
    // the game soft-locks on (0, 1, 171, ...).
    if ((level + (level >> 1)) & 0xff) < 2 {
        b.hint(TextId::HintGen1PsywaveSoftlock, false, None);
        return Some(Relay::FAIL);
    }
    // `this.random(1, this.trunc(1.5 * pokemon.level))`: an integer in
    // `[1, upper)`, the upper bound exclusive.
    let upper = b.trunc.float(f64::from(level) * 1.5) as u32;
    let damage = b.prng.random_range(1, upper);
    Some(Relay::Int(damage as i32))
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::DamageCallback, damage_callback)]);
