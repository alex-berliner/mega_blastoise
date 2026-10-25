//! Recover: heals half the user's maximum HP, unless the cartridge's
//! recovery glitch makes the move fail.
//!
//! The move has no `heal` data in gen 1 (`heal: undefined, // no inherit`
//! in the gen 1 mod), so the healing is the handler's own.
//!
//! No effect state.
//!
//! PS: data/mods/gen1/moves.ts recover.onHit (line 639; gen 1's own handler)

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_protocol::TextId;

/// `onHit(target)`.
// PS: data/mods/gen1/moves.ts recover.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    // The move targets the user, so the event's target is the user.
    let target = a.target.mon()?;
    let hp = i32::from(b.mon(target).hp);
    let maxhp = i32::from(b.mon(target).maxhp);
    if hp == maxhp {
        // Nothing to heal: the move fails without a hint.
        return Some(Relay::FAIL);
    }
    // Fail when health is 255 or 511 less than max, unless it is
    // divisible by 256.
    if (hp == maxhp - 255 || hp == maxhp - 511) && hp % 256 != 0 {
        b.hint(TextId::HintGen1Recovery, false, None);
        return Some(Relay::FAIL);
    }
    // `this.heal(Math.floor(target.maxhp / 2), target, target)`: the user
    // heals itself, and the move is the effect of the healing
    // (`this.effect`), so the `-heal` line carries no kwarg.
    b.heal(maxhp / 2, Some(target), Some(target), EffectId::None);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Hit, on_hit)]);
