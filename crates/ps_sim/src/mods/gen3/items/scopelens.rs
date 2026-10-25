//! `scopelens`: Scope Lens, the critical hit ratio one stage up.
//!
//! PS: data/items.ts scopelens

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::mods::gen3::items::crit_ratio;
use crate::relay::Relay;
use crate::state::Battle;

/// `onModifyCritRatio(critRatio)`: `return critRatio + 1`.
// PS: data/items.ts scopelens.onModifyCritRatio
fn on_modify_crit_ratio(_b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    Some(Relay::Int(crit_ratio(a) + 1))
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::ModifyCritRatio, on_modify_crit_ratio)]);
