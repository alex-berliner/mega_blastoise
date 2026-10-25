//! `defensecurl`: Defense Curl's volatile, a marker (gen 3 Rollout and Ice
//! Ball read it). Using Defense Curl again keeps it (`onRestart` answers
//! `null`). No effect state.
//!
//! PS: data/moves.ts defensecurl.condition

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onRestart: () => null`: the volatile stays, and adding it again fails
/// silently.
// PS: data/moves.ts defensecurl.condition.onRestart
fn on_restart(_b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    Some(Relay::Null)
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Restart, on_restart)]);
