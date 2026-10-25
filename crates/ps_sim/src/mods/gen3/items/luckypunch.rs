//! `luckypunch`: Lucky Punch, Chansey's critical hit ratio two stages up.
//!
//! PS: data/mods/gen4/items.ts luckypunch

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::mods::gen3::items::{crit_ratio, species_name};
use crate::relay::Relay;
use crate::state::Battle;

/// `onModifyCritRatio(critRatio, user)`: `if (user.species.name ===
/// "Chansey") return critRatio + 2`.
// PS: data/mods/gen4/items.ts luckypunch.onModifyCritRatio
fn on_modify_crit_ratio(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let user = a.target.mon()?;
    (species_name(b, user) == "Chansey").then(|| Relay::Int(crit_ratio(a) + 2))
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::ModifyCritRatio, on_modify_crit_ratio)]);
