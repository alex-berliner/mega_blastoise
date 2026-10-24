//! Shell Armor: `onCriticalHit: false`, as Battle Armor. No effect state.
//!
//! PS: data/abilities.ts shellarmor

use crate::effect::{EffectDef, EventId, Handler};
use crate::relay::Relay;

pub static DEF: EffectDef = EffectDef::new(&[Handler::value(EventId::CriticalHit, Relay::FAIL)]);
