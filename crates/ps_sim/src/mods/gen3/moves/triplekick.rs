//! Triple Kick: each strike is 10 power stronger (`multihit: 3`,
//! `multiaccuracy`).
//!
//! PS: data/moves.ts triplekick.basePowerCallback

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `basePowerCallback(pokemon, target, move)`: `10 * move.hit`.
// PS: data/moves.ts triplekick.basePowerCallback
fn base_power_callback(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    let hit = b.active_move().expect("basePowerCallback without a move").hit;
    Some(Relay::Int(10 * i32::from(hit)))
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::BasePowerCallback, base_power_callback)]);
