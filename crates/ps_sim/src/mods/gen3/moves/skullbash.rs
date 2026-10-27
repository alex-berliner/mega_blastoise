//! Skull Bash: a two-turn move. The first turn raises the user's Defense and charges (a handler-less `skullbash` volatile, from `twoturnmove`); the second hits. No effect state.
//!
//! PS: data/moves.ts skullbash.onTryMove

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::mods::gen3::moves::{two_turn_try_move, ChargeExtra};
use crate::relay::Relay;
use crate::state::Battle;

/// `onTryMove(attacker, defender, move)`.
// PS: data/moves.ts skullbash.onTryMove
fn on_try_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    two_turn_try_move(b, a, ChargeExtra::SkullBash)
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::TryMove, on_try_move)]);
