//! Dig: a two-turn move. The first turn burrows the user (its `dig` condition, from `twoturnmove`); the second hits. No effect state.
//!
//! PS: data/moves.ts dig.onTryMove

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::mods::gen3::moves::{two_turn_try_move, ChargeExtra};
use crate::relay::Relay;
use crate::state::Battle;

/// `onTryMove(attacker, defender, move)`.
// PS: data/moves.ts dig.onTryMove
fn on_try_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    two_turn_try_move(b, a, ChargeExtra::None)
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::TryMove, on_try_move)]);
