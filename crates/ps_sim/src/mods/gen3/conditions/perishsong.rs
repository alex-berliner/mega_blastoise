//! `perishsong`: Perish Song's volatile. The count goes down each residual
//! and the holder faints when it ends.
//!
//! Effect state: `duration` (4, counted down by the residual before
//! `onResidual` reads it).
//!
//! PS: data/moves.ts perishsong.condition

use ps_protocol::{Arg, ArgValue, Kw, Msg, Word};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;

fn perish_word(n: i8) -> Word {
    match n {
        0 => Word::Perish0,
        1 => Word::Perish1,
        2 => Word::Perish2,
        3 => Word::Perish3,
        other => panic!("perish{other}"),
    }
}

/// `onEnd(target)`: `-start|MON|perish0`, then `target.faint()`.
// PS: data/moves.ts perishsong.condition.onEnd
fn on_end(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let mon = b.ident(target);
    b.add(Msg::StartEffect { mon, effect: Arg::new(ArgValue::Word(Word::Perish0)), a: None, b: None, kw: Kw::NONE });
    b.faint(target, None, EffectId::None);
    None
}

/// `onResidual(pokemon)`: `-start|MON|perishN` with the turns left.
// PS: data/moves.ts perishsong.condition.onResidual
fn on_residual(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let duration = b.volatile_state(pokemon, conditions::PERISHSONG).expect("perishsong").duration;
    let mon = b.ident(pokemon);
    b.add(Msg::StartEffect { mon, effect: Arg::new(ArgValue::Word(perish_word(duration))), a: None, b: None, kw: Kw::NONE });
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::End, on_end),
    Handler::on(EventId::Residual, on_residual).order(12),
]);
