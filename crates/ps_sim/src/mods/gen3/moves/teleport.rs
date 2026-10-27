//! Teleport: `onTry: false`, so the move fails silently after its
//! `|move|` line (no wild battles to flee). No effect state.
//!
//! PS: data/mods/gen7/moves.ts teleport (gen 3 inherits `onTry: false`)

use crate::effect::{EffectDef, EventId, Handler};
use crate::relay::Relay;

pub static DEF: EffectDef = EffectDef::new(&[Handler::value(EventId::Try, Relay::FAIL)]);
