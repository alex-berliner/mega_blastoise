//! Frustration: power from the user's unhappiness.
//!
//! PS: data/moves.ts frustration.basePowerCallback

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `basePowerCallback(pokemon)`: `Math.floor((255 - pokemon.happiness) *
/// 10 / 25) || 1`.
// PS: data/moves.ts frustration.basePowerCallback
fn base_power_callback(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let bp = (255 - i32::from(b.mon(pokemon).happiness)) * 10 / 25;
    Some(Relay::Int(if bp == 0 { 1 } else { bp }))
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::BasePowerCallback, base_power_callback)]);
