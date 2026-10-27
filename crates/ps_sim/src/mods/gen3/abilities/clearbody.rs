//! Clear Body: none of the holder's stats can be lowered by another
//! Pokemon.
//!
//! The ability keeps no state of its own.
//!
//! PS: data/abilities.ts clearbody (onTryBoost)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

use super::block_all_drops;

/// `onTryBoost(boost, target, source, effect)`.
fn on_try_boost(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    block_all_drops(b, a)
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::TryBoost, on_try_boost)]);
