//! Battle Armor: `onCriticalHit: false`, so a hit on the holder never
//! crits (the crit roll still happens). No effect state.
//!
//! PS: data/abilities.ts battlearmor

use crate::effect::{EffectDef, EventId, Handler};
use crate::relay::Relay;

pub static DEF: EffectDef = EffectDef::new(&[Handler::value(EventId::CriticalHit, Relay::FAIL)]);
