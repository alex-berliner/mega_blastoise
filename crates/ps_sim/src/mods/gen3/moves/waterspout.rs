//! Water Spout: Eruption's `basePowerCallback`, word for word.
//!
//! PS: data/moves.ts waterspout.basePowerCallback

use crate::effect::{EffectDef, EventId, Handler};

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::BasePowerCallback, super::eruption::base_power_callback)]);
