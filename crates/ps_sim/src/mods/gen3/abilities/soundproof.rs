//! Soundproof: sound moves (`flags.sound`) do not affect the holder:
//! `-immune ... [from] ability: Soundproof`, and the move stops (`null`).
//!
//! No ability state.
//!
//! PS: data/mods/gen7/abilities.ts soundproof

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

use super::{add_immune, event_move};

/// `onTryHit(target, source, move)`.
// PS: data/mods/gen7/abilities.ts soundproof.onTryHit
fn on_try_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let mv = event_move(b, a)?;
    if b.dex.move_def(mv.id).flags.has("sound") {
        add_immune(b, target);
        return Some(Relay::Null);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::TryHit, on_try_hit)]);
