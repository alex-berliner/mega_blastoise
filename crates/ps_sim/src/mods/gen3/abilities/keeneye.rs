//! Keen Eye: the holder's Accuracy cannot be lowered by another Pokemon.
//!
//! The ability keeps no state of its own.
//!
//! PS: data/abilities.ts keeneye (onTryBoost)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_protocol::Stat;

use super::block_stat_drop;

/// `onTryBoost(boost, target, source, effect)`.
fn on_try_boost(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    block_stat_drop(b, a, Stat::Accuracy)
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::TryBoost, on_try_boost)]);
