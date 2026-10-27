//! Brick Break: shatters the foe side's Reflect and Light Screen before
//! the hit.
//!
//! PS: data/mods/gen3/moves.ts brickbreak.onTryHit

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;

/// `onTryHit(target, source)`: `source.side.foe.removeSideCondition(...)`
/// for `reflect`, then `lightscreen`.
// PS: data/mods/gen3/moves.ts brickbreak.onTryHit
fn on_try_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let source = a.source.expect("brickbreak.onTryHit without a user");
    let foe = 1 - source.side;
    b.remove_side_condition(foe, conditions::REFLECT);
    b.remove_side_condition(foe, conditions::LIGHTSCREEN);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::TryHit, on_try_hit)]);
