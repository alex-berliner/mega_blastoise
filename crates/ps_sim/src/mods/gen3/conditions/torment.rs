//! `torment`: Torment's volatile. The holder cannot pick its last move
//! (Struggle aside) on the next turn. No effect state of its own.
//!
//! PS: data/moves.ts torment.condition

use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::moves;

fn torment_arg() -> Arg {
    Arg::new(ArgValue::Move(MoveNum(moves::TORMENT.0)))
}

/// `onStart(pokemon, source, effect)`: `-start|MON|Torment` (no Dynamax or
/// G-Max Meltdown in gen 3).
// PS: data/moves.ts torment.condition.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let mon = b.ident(pokemon);
    b.add(Msg::StartEffect { mon, effect: torment_arg(), a: None, b: None, kw: Kw::NONE });
    None
}

/// `onEnd(pokemon)`: `-end|MON|Torment`.
// PS: data/moves.ts torment.condition.onEnd
fn on_end(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let mon = b.ident(pokemon);
    b.add(Msg::EndEffect { mon, effect: torment_arg(), kw: Kw::NONE });
    None
}

/// `onDisableMove(pokemon)`: the last move, unless it was Struggle.
// PS: data/moves.ts torment.condition.onDisableMove
fn on_disable_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let last = b.mon(pokemon).last_move;
    if !last.is_none() && last != moves::STRUGGLE {
        b.disable_move(pokemon, last, false, EffectId::None);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::End, on_end),
    Handler::on(EventId::DisableMove, on_disable_move),
]);
