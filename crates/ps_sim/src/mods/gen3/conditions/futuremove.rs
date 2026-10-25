//! `futuremove`: Future Sight and Doom Desire's delayed hit, a slot
//! condition on the target's slot. The moves' `onTry` adds it and fills in
//! `duration: 3`, `move`, `source` and `moveData` (whose `damage` they
//! compute when used).
//!
//! Effect state:
//! - `targetSlot`: the slot's side and position in `data[0]` and `data[1]`;
//! - `endingTurn`: `data[2]`;
//! - `moveData.damage`: `data[3]` (the moves' port writes it);
//! - `move`: `move_`; `source`: `source`.
//!
//! PS: data/conditions.ts futuremove

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;
use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg, Text, TextArg, TextArgValue, TextId};

/// `this.effectState.targetSlot`: side, then position.
pub const TARGET_SIDE: usize = 0;
pub const TARGET_POSITION: usize = 1;
/// `this.effectState.endingTurn`.
pub const ENDING_TURN: usize = 2;
/// `this.effectState.moveData.damage`.
pub const MOVE_DAMAGE: usize = 3;

/// `onStart(target)`: the slot and the turn it lands (the turn-256 hint is
/// gen 8).
// PS: data/conditions.ts futuremove.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let (side, position) = (target.side, b.mon(target).position);
    let ending_turn = b.turn as i16 - 1 + 2;
    let st = b.effect_state();
    st.data[TARGET_SIDE] = i16::from(side);
    st.data[TARGET_POSITION] = i16::from(position);
    st.data[ENDING_TURN] = ending_turn;
    None
}

/// `onRestart()`: a second future move on the slot fails (its hint is
/// gen 8).
// PS: data/conditions.ts futuremove.onRestart
fn on_restart(_b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    Some(Relay::FAIL)
}

/// `onResidual(target)`: on the ending turn the condition leaves the slot
/// (and `onEnd` hits).
// PS: data/conditions.ts futuremove.onResidual
fn on_residual(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    // `getOverflowedTurnCount()`: `turn - 1` before gen 8.
    let (side, position, ending_turn) = {
        let st = b.effect_state();
        (st.data[TARGET_SIDE] as u8, st.data[TARGET_POSITION] as u8, st.data[ENDING_TURN])
    };
    if (b.turn as i16 - 1) < ending_turn {
        return None;
    }
    // `target.side.removeSlotCondition(this.getAtSlot(targetSlot), ...)`.
    b.remove_slot_condition(side, position, conditions::FUTUREMOVE);
    None
}

/// `onEnd(target)`: no hit on a fainted target or the user itself;
/// otherwise the `-end` line and the hit.
// PS: data/conditions.ts futuremove.onEnd
fn on_end(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let (m, source) = {
        let st = b.effect_state();
        (st.move_, st.source)
    };
    let fainted = b.mon(target).fainted;
    if fainted || Some(target) == source {
        let id = if fainted { TextId::HintFutureMoveTargetFainted } else { TextId::HintFutureMoveTargetUser };
        b.hint_text(Text::with1(id, TextArg::new(TextArgValue::Move(MoveNum(m.0)))), false, None);
        return None;
    }
    let mon = b.ident(target);
    b.add(Msg::EndEffect { mon, effect: Arg::new(ArgValue::MoveEffect(MoveNum(m.0))), kw: Kw::NONE });
    // Infiltrator and Normalize (gen 6) do not apply. `const hitMove = new
    // this.dex.Move(data.moveData); this.actions.trySpreadMoveHit([target],
    // data.source, hitMove, true)`, then Life Orb (gen 5), `activeMove =
    // null` and `checkWin()`: gen 3 reaches base `trySpreadMoveHit` and its
    // `hitStep*` only here, with a handler-less move object built from
    // `moveData`, and only Future Sight and Doom Desire (G3-M4) add this
    // condition. The hit is theirs to port.
    panic!("unported: trySpreadMoveHit of a future move ({:?})", b.dex.move_def(m).name.as_str())
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::Restart, on_restart),
    Handler::on(EventId::Residual, on_residual).order(11),
    Handler::on(EventId::End, on_end),
]);
