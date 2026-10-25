//! `stockpile`: Stockpile's volatile, up to three layers, which Spit Up
//! and Swallow spend (`noCopy`: Baton Pass leaves it).
//!
//! Effect state: `layers` is `layers`.
//!
//! PS: data/mods/gen3/moves.ts stockpile.condition

use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg, Word};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::moves;

/// `-start|MON|stockpileN`.
fn start_line(b: &mut Battle, a: &EventArgs, layers: u8) {
    let Some(target) = a.target.mon() else { return };
    let word = match layers {
        1 => Word::Stockpile1,
        2 => Word::Stockpile2,
        3 => Word::Stockpile3,
        n => panic!("stockpile layer {n}"),
    };
    let mon = b.ident(target);
    b.add(Msg::StartEffect { mon, effect: Arg::new(ArgValue::Word(word)), a: None, b: None, kw: Kw::NONE });
}

/// `onStart(target)`: one layer.
// PS: data/mods/gen3/moves.ts stockpile.condition.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    b.effect_state().layers = 1;
    start_line(b, a, 1);
    None
}

/// `onRestart(target)`: another layer, up to three.
// PS: data/mods/gen3/moves.ts stockpile.condition.onRestart
fn on_restart(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    if b.effect_state().layers >= 3 {
        return Some(Relay::FAIL);
    }
    let st = b.effect_state();
    st.layers += 1;
    let layers = st.layers;
    start_line(b, a, layers);
    None
}

/// `onEnd(target)`: `-end|MON|Stockpile`.
// PS: data/mods/gen3/moves.ts stockpile.condition.onEnd
fn on_end(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    b.effect_state().layers = 0;
    let mon = b.ident(target);
    b.add(Msg::EndEffect { mon, effect: Arg::new(ArgValue::Move(MoveNum(moves::STOCKPILE.0))), kw: Kw::NONE });
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::Restart, on_restart),
    Handler::on(EventId::End, on_end),
]);
