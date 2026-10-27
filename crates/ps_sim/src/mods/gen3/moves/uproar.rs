//! Uproar: wakes every sleeping active Pokemon, then locks the user in
//! (the `uproar` volatile, its `self.volatileStatus`). No effect state.
//!
//! PS: data/moves.ts uproar.onTryHit

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;

/// `onTryHit(target)`: the target's side and then the foe's, slot by slot,
/// lose their sleep.
// PS: data/moves.ts uproar.onTryHit
fn on_try_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let (side, foe) = (target.side, 1 - target.side);
    for pos in 0..b.sides[side as usize].active.len() as u8 {
        if let Some(ally) = b.active_at(side, pos) {
            if b.mon(ally).status == conditions::SLP {
                b.cure_status(ally, false);
            }
        }
        if let Some(foe_active) = b.active_at(foe, pos) {
            if b.mon(foe_active).status == conditions::SLP {
                b.cure_status(foe_active, false);
            }
        }
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::TryHit, on_try_hit)]);
