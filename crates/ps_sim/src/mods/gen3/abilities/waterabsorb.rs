//! Water Absorb: a Water move aimed at the holder heals it a quarter of its max
//! HP instead (`-heal ... [from] ability: Water Absorb|[of] SOURCE`), or shows
//! `-immune` at full HP; the move stops either way (`null`).
//!
//! No ability state.
//!
//! PS: data/abilities.ts waterabsorb

use ps_dex::Type;

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

use super::{add_immune, event_move};

/// `onTryHit(target, source, move)`: `this.heal(target.baseMaxhp / 4)`.
// PS: data/abilities.ts waterabsorb.onTryHit
fn on_try_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let source = a.source?;
    let mv = event_move(b, a)?;
    if target != source && mv.type_ == Type::Water {
        if !b.heal(super::quarter_base_maxhp(b, target), None, None, EffectId::None).truthy() {
            add_immune(b, target);
        }
        return Some(Relay::Null);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::TryHit, on_try_hit)]);
