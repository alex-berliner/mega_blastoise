//! Minimize's volatile: using Minimize again while the volatile is
//! already up does nothing (the `onRestart` handler returns `null`, so
//! the move's `didSomething` stays false).
//!
//! The move itself has no handlers: its boost (`evasion`) and its
//! `volatileStatus` are the move's data, which the gen 1 pipeline
//! applies. The volatile's own `onRestart` is the only handler PS's
//! resolved gen 1 condition has: the base condition's
//! `onSourceModifyDamage` is dropped by
//! `data/mods/gen1/moves.ts` and its `onAccuracy` by
//! `data/mods/gen5/moves.ts`.
//!
//! No effect state.
//!
//! PS: data/moves.ts minimize.condition (line 11920, `onRestart`); gen 1 keeps it
//! (data/mods/gen1/moves.ts minimize, line 520)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onRestart()`.
// PS: data/moves.ts minimize.condition.onRestart
fn on_restart(_b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    // `() => null`: PS's `null` ends the move silently.
    Some(Relay::Null)
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Restart, on_restart)]);
