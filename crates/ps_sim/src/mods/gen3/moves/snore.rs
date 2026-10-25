//! Snore: a 40 BP attack that works only while the user sleeps
//! (`sleepUsable`, and the shared `onTry`). No effect state.
//!
//! PS: data/moves.ts snore.onTry

use crate::effect::{EffectDef, EventId, Handler};

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Try, super::try_asleep)]);
