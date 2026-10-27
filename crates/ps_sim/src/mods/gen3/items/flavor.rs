//! Gen 3 flavor berries (Pomeg, Kelpsy, Qualot, Hondew, Grepa, Tamato,
//! Cornn, Magost, Rabuta, Nomel, Spelon, Pamtre, Watmel, Durin, Belue,
//! Razz, Bluk, Nanab, Wepear, Pinap): each has only `onEat: false`, so
//! the eat returns `false` after printing `-enditem [eat]`. The berry
//! is consumed but no other side effect happens (a flavor berry is just
//! eaten). One shared handler group across the 20 berries.
//!
//! PS: data/items.ts (the flavor berry entries).

use crate::effect::{EffectDef, EventId, Handler};
use crate::relay::Relay;

/// `onEat: false` (PS's `callback: 0`). The berry is consumed but the
/// eat returns false so the pipeline cleans up.
pub static DEF_FLAVOR: EffectDef = EffectDef::new(&[Handler::value(EventId::Eat, Relay::Bool(false))]);
