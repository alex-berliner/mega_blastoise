//! Endure: the `endure` volatile for the turn, with Protect's `stall`
//! handlers. No effect state.
//!
//! PS: data/moves.ts endure (onPrepareHit, onHit)

use crate::effect::{EffectDef, EventId, Handler};

use super::protect::{stall_hit, stall_prepare_hit};

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::PrepareHit, stall_prepare_hit),
    Handler::on(EventId::Hit, stall_hit),
]);
