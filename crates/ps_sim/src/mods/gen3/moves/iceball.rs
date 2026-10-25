//! Ice Ball: Rollout under its own name and volatile (`iceball`); see
//! `rollout.rs`. No effect state of its own.
//!
//! PS: data/moves.ts iceball

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::mods::gen3::moves::rollout;
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;

/// `basePowerCallback(pokemon, target, move)`.
// PS: data/moves.ts iceball.basePowerCallback
fn base_power_callback(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    rollout::base_power_callback_of(b, a, conditions::ICEBALL)
}

/// `onModifyMove(move, pokemon, target)`.
// PS: data/moves.ts iceball.onModifyMove
fn on_modify_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    rollout::on_modify_move_of(b, a, conditions::ICEBALL)
}

/// `onAfterMove(source, target, move)`.
// PS: data/moves.ts iceball.onAfterMove
fn on_after_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    rollout::on_after_move_of(b, a, conditions::ICEBALL)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::BasePowerCallback, base_power_callback),
    Handler::on(EventId::ModifyMove, on_modify_move),
    Handler::on(EventId::AfterMove, on_after_move),
]);
