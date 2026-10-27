//! Sturdy: the holder is immune to OHKO moves. (Gen 3's Sturdy has no
//! `onDamage`: the `onDamagePriority` key the dex keeps has no handler.)
//!
//! The ability keeps no state of its own.
//!
//! PS: data/abilities.ts sturdy (onTryHit)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

use super::{add_immune, event_move};

/// `onTryHit(pokemon, target, move)`: for an OHKO move, `-immune|MON|[from]
/// ability: Sturdy` and `null`.
fn on_try_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    if !event_move(b, a)?.ohko {
        return None;
    }
    add_immune(b, pokemon);
    Some(Relay::Null)
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::TryHit, on_try_hit)]);
