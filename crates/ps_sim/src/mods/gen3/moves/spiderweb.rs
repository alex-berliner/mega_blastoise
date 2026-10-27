//! Spider Web: traps the target while the user stays in (`trapped` on the
//! target, linked to `trapper` on the user).
//!
//! PS: data/moves.ts spiderweb.onHit

use crate::effect::{EffectDef, EventId, Handler};

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Hit, super::trap_on_hit)]);
