//! Hi Jump Kick: a miss (or any failure) hurts the user for half the
//! damage it would have dealt, capped at half the target's max HP. Jump
//! Kick shares the body.
//!
//! PS: data/mods/gen3/moves.ts highjumpkick.onMoveFail

use crate::actions::{GenScripts, MoveResult};
use crate::actions::MoveData;
use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::mods::gen3::scripts::Gen3Scripts;
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::Type;

/// `onMoveFail(target, source, move)`: if `target.runImmunity('Fighting')`,
/// `getDamage(source, target, move, true)` (a number, or PS throws), then
/// `this.damage(clampIntRange(damage / 2, 1, floor(target.maxhp / 2)),
/// source, source, move)`.
// PS: data/mods/gen3/moves.ts highjumpkick.onMoveFail
pub fn on_move_fail(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon().expect("onMoveFail without a target");
    let source = a.source.expect("onMoveFail without a user");
    if !b.run_type_immunity(target, Type::Fighting, false) {
        return None;
    }
    let mv = b.active_move.expect("onMoveFail without a move");
    let damage = match Gen3Scripts::get_damage(b, source, target, mv, MoveData::Move, true) {
        MoveResult::Int(n) => n,
        other => panic!("crash damage of {other:?}"),
    };
    let crash = (damage / 2).max(1).min(i32::from(b.mon(target).maxhp) / 2);
    let effect = b.mv(mv).effect();
    b.damage(crash, Some(source), Some(source), effect);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::MoveFail, on_move_fail)]);
