//! Teleport: `onTry: false`, so the move fails silently after its
//! `|move|` line (gen 1 has no wild battles to flee). No effect state.
//!
//! PS: data/moves.ts teleport (gen 1 resolves `onTry` to `false`)

use crate::effect::{EffectDef, EventId, Handler};
use crate::relay::Relay;

pub static DEF: EffectDef = EffectDef::new(&[Handler::value(EventId::Try, Relay::FAIL)]);
