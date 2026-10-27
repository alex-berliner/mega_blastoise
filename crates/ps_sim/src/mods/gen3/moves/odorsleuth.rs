//! Odor Sleuth: Foresight under another name (same `onTryHit`, same
//! `foresight` volatile).
//!
//! PS: data/moves.ts odorsleuth.onTryHit

use crate::effect::{EffectDef, EventId, Handler};

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::TryHit, super::foresight::on_try_hit)]);
