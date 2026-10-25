//! Synthesis: heals half the user's HP, two thirds in the sun, a quarter in
//! rain, sand or hail. No effect state.
//!
//! PS: data/mods/gen4/moves.ts synthesis.onHit

use crate::effect::{EffectDef, EventId, Handler};

use super::weather_heal;

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Hit, weather_heal)]);
