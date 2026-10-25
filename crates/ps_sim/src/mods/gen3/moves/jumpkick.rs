//! Jump Kick: Hi Jump Kick's gen 3 `onMoveFail`, word for word (only the
//! error text differs).
//!
//! PS: data/mods/gen3/moves.ts jumpkick.onMoveFail

use crate::effect::{EffectDef, EventId, Handler};

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::MoveFail, super::highjumpkick::on_move_fail)]);
