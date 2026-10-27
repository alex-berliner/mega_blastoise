//! Reversal: Flail's gen 3 `basePowerCallback`, word for word.
//!
//! PS: data/mods/gen3/moves.ts reversal.basePowerCallback

use crate::effect::{EffectDef, EventId, Handler};

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::BasePowerCallback, super::flail::base_power_callback)]);
