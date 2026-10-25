//! Curse: a non-Ghost raises Attack and Defense and lowers Speed (its
//! `self` effect); a Ghost pays half its max HP to put the `curse`
//! volatile on the target.
//!
//! PS: data/mods/gen4/moves.ts curse.onModifyMove; data/moves.ts curse
//! (onTryHit, onHit)

use crate::actions::MoveRef;
use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;
use ps_dex::{Boosts, ConditionId, MoveTarget, SelfEffect, Type};

/// `move.self = {boosts: ...}`: +1 Attack, +1 Defense, -1 Speed.
static BOOSTS: Boosts = Boosts { atk: 1, def: 1, spa: 0, spd: 0, spe: -1, accuracy: 0, evasion: 0 };
static SELF: SelfEffect = SelfEffect {
    chance: None,
    status: ConditionId::NONE,
    volatile_status: ConditionId::NONE,
    side_condition: ConditionId::NONE,
    pseudo_weather: ConditionId::NONE,
    boosts: Some(&BOOSTS),
    handlers: &[],
};
/// `onModifyMove`'s key order, `{atk: 1, def: 1, spe: -1}`.
const MODIFY_MOVE_ORDER: &[u8] = &[0, 1, 4];
/// `onTryHit`'s key order, `{spe: -1, atk: 1, def: 1}`: the one the boost
/// lines come out in, since `TryHit` runs after `ModifyMove`.
const TRY_HIT_ORDER: &[u8] = &[4, 0, 1];

/// The move in flight, as the event's `move`.
fn this_move(b: &Battle, a: &EventArgs) -> MoveRef {
    let EffectId::Move(id) = a.effect else { panic!("curse handler from {:?}", a.effect) };
    b.active_move.filter(|&r| b.mv(r).id == id).expect("curse: the move in flight")
}

/// `delete move.volatileStatus; delete move.onHit`.
fn delete_volatile_and_on_hit(b: &mut Battle, mv: MoveRef) {
    let am = b.mv_mut(mv);
    am.volatile_status_deleted = true;
    am.on_hit_deleted = true;
}

/// `onModifyMove(move, source, target)`: a non-Ghost's Curse becomes a
/// self-targeted boost; a Ghost's against a Substitute does nothing.
// PS: data/mods/gen4/moves.ts curse.onModifyMove
fn on_modify_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let source = a.target.mon()?;
    let mv = this_move(b, a);
    if !b.has_type(source, Type::Ghost) {
        delete_volatile_and_on_hit(b, mv);
        let am = b.mv_mut(mv);
        am.self_effect = Some((&SELF, MODIFY_MOVE_ORDER));
        am.target = MoveTarget::Self_;
    } else if a.source.is_some_and(|t| b.has_vol(t, conditions::SUBSTITUTE)) {
        delete_volatile_and_on_hit(b, mv);
    }
    None
}

/// `onTryHit(target, source, move)`: the non-Ghost rewrite again (with
/// the boosts in another key order); a Ghost's fails against a target
/// already cursed.
// PS: data/moves.ts curse.onTryHit
fn on_try_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let source = a.source.expect("curse without a source");
    let mv = this_move(b, a);
    if !b.has_type(source, Type::Ghost) {
        delete_volatile_and_on_hit(b, mv);
        b.mv_mut(mv).self_effect = Some((&SELF, TRY_HIT_ORDER));
    } else if !b.mv(mv).volatile_status_deleted && b.has_vol(target, conditions::CURSE) {
        return Some(Relay::FAIL);
    }
    None
}

/// `onHit(target, source)`: `directDamage(source.maxhp / 2, source,
/// source)`, whose `clampIntRange(damage, 1)` floors it and makes it at
/// least 1.
// PS: data/moves.ts curse.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let source = a.source.expect("curse without a source");
    let d = (i32::from(b.mon(source).maxhp) / 2).max(1);
    b.direct_damage(d, Some(source), Some(source), EffectId::None);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::ModifyMove, on_modify_move),
    Handler::on(EventId::TryHit, on_try_hit),
    Handler::on(EventId::Hit, on_hit),
]);
