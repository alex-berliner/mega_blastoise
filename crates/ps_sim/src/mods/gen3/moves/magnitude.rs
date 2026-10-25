//! Magnitude: `onModifyMove` rolls the magnitude and its power;
//! `onUseMoveMessage` prints the magnitude (kept on the move in flight,
//! [`crate::actions::ActiveMove::magnitude`]).
//!
//! PS: data/moves.ts magnitude.onModifyMove, magnitude.onUseMoveMessage

use ps_protocol::{Arg, Kw, MoveNum, Msg};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onModifyMove(move, pokemon)`: `i = this.random(100)` picks magnitude 4
/// to 10 (base power 10 to 150).
// PS: data/moves.ts magnitude.onModifyMove
fn on_modify_move(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    let i = b.prng.random_n(100);
    let (magnitude, base_power) = if i < 5 {
        (4, 10)
    } else if i < 15 {
        (5, 30)
    } else if i < 35 {
        (6, 50)
    } else if i < 65 {
        (7, 70)
    } else if i < 85 {
        (8, 90)
    } else if i < 95 {
        (9, 110)
    } else {
        (10, 150)
    };
    let am = b.active_move_mut().expect("ModifyMove without an active move");
    am.magnitude = magnitude;
    am.base_power = base_power;
    None
}

/// `onUseMoveMessage(pokemon, target, move)`: `-activate|MON|move:
/// Magnitude|N`.
// PS: data/moves.ts magnitude.onUseMoveMessage
fn on_use_move_message(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let magnitude = b.active_move().expect("onUseMoveMessage without a move").magnitude;
    let mon = b.ident(pokemon);
    let effect = Arg::move_effect(MoveNum(ps_dex::gen3::moves::MAGNITUDE.0));
    b.add(Msg::Activate { mon: Some(mon), effect, a: Some(Arg::int(i16::from(magnitude))), b: None, kw: Kw::NONE });
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::ModifyMove, on_modify_move),
    Handler::on(EventId::UseMoveMessage, on_use_move_message),
]);
