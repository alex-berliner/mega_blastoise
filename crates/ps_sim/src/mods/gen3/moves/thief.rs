//! Thief: after the hit an empty-handed user takes the target's item.
//! No effect state.
//!
//! PS: data/moves.ts thief.onAfterHit

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::moves;

/// `onAfterHit(target, source, move)`: `-enditem|TARGET|ITEM|[silent]|[from]
/// move: Thief|[of] SOURCE`, then `-item|SOURCE|ITEM|[from] move:
/// Thief|[of] TARGET`.
// PS: data/moves.ts thief.onAfterHit
fn on_after_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    super::steal_item_after_hit(b, a, moves::THIEF, true)
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::AfterHit, on_after_hit)]);
