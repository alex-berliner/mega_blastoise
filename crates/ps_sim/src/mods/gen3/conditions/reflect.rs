//! `reflect`: Reflect's side condition. Physical damage to the side's
//! Pokemon is halved, except a critical hit. Five turns (eight with Light
//! Clay, which is gen 4). No effect state.
//!
//! PS: data/moves.ts reflect.condition (durationCallback, onSideStart,
//! onSideEnd), data/mods/gen4/moves.ts reflect.condition
//! (onAnyModifyDamagePhase1)

use ps_dex::MoveCategory;
use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg};

use crate::actions::{ActiveMove, FakeMove};
use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler, Prefix};
use crate::relay::Relay;
use crate::state::{Battle, Holder};
use ps_dex::gen3::moves;

/// The `durationCallback(target, source, effect)` of Reflect and Light
/// Screen: 8 with Light Clay, else 5. Light Clay is gen 4, so no gen 3
/// Pokemon holds it.
// PS: data/moves.ts reflect.condition.durationCallback,
// lightscreen.condition.durationCallback
pub fn screen_duration(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let lightclay = b.dex.item_id("lightclay");
    if let (Some(source), Some(clay)) = (a.source, lightclay) {
        if b.has_item(source, clay) {
            return Some(Relay::Int(8));
        }
    }
    Some(Relay::Int(5))
}

/// The `onAnyModifyDamagePhase1(damage, source, target, move)` of Reflect
/// and Light Screen. `ModifyDamagePhase1` runs on the attacker with the
/// defender as its source, so PS's `source` is the event's target and its
/// `target` the event's source. `move.infiltrates` is never set in gen 3
/// (Infiltrator's is gen 5), and `alliesAndSelf()` is one Pokemon in
/// singles, so the modifier is 0.5.
// PS: data/mods/gen4/moves.ts reflect.condition.onAnyModifyDamagePhase1,
// lightscreen.condition.onAnyModifyDamagePhase1
pub fn screen_modify(b: &mut Battle, a: &EventArgs, category: MoveCategory, debug: ps_protocol::TextId) -> Option<Relay> {
    let source = a.target.mon()?;
    let target = a.source?;
    let Holder::Side(side) = b.effect_state().target else { panic!("a screen not on a side") };
    if target == source || target.side != side {
        return None;
    }
    // `this.getCategory(move)`: the move object's category.
    let mv = damage_move(b, a);
    if mv.category == category && !mv.crit {
        b.debug(ps_protocol::Text::new(debug));
        b.chain_modify(1, 2);
    }
    None
}

/// The move object whose damage is being modified: the active move, or a
/// move object with no id (so the event's effect is none), the newest in
/// the arena: a future move's damage roll on a plain `moveData`, or
/// `getDamage`'s object for a number (confusion's self-hit, which never
/// gets here: its source is its target). Both have `willCrit: false`.
fn damage_move(b: &Battle, a: &EventArgs) -> ActiveMove {
    match a.effect {
        EffectId::Move(id) => b.active_move().filter(|am| am.id == id).copied().expect("a damage event of another move"),
        EffectId::None => {
            let newest = b.moves[usize::from(b.move_count) - 1];
            assert!(matches!(newest.fake, FakeMove::Data | FakeMove::BasePower), "a damage event with no move object");
            newest
        }
        other => panic!("a damage event of {other:?}"),
    }
}

/// `onSideStart(side)`: `-sidestart|SIDE|Reflect`.
// PS: data/moves.ts reflect.condition.onSideStart
fn on_side_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let Holder::Side(side) = a.target else { return None };
    let side = b.sides[side as usize].id();
    b.add(Msg::SideStart { side, effect: Arg::new(ArgValue::Move(MoveNum(moves::REFLECT.0))), kw: Kw::NONE });
    None
}

/// `onSideEnd(side)`: `-sideend|SIDE|Reflect`.
// PS: data/moves.ts reflect.condition.onSideEnd
fn on_side_end(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let Holder::Side(side) = a.target else { return None };
    let side = b.sides[side as usize].id();
    b.add(Msg::SideEnd { side, effect: Arg::new(ArgValue::Move(MoveNum(moves::REFLECT.0))), kw: Kw::NONE });
    None
}

/// `onAnyModifyDamagePhase1`: physical moves.
// PS: data/mods/gen4/moves.ts reflect.condition.onAnyModifyDamagePhase1
fn on_any_modify_damage_phase1(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    screen_modify(b, a, MoveCategory::Physical, ps_protocol::TextId::ReflectWeaken)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::DurationCallback, screen_duration),
    Handler::on(EventId::SideStart, on_side_start),
    Handler::on(EventId::SideEnd, on_side_end),
    Handler::on(EventId::ModifyDamagePhase1, on_any_modify_damage_phase1).prefix(Prefix::Any),
]);
