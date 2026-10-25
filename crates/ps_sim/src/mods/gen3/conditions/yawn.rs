//! `yawn`: Yawn's volatile. At the end of the next turn the holder falls
//! asleep (if it can).
//!
//! Effect state: `source` is the Yawn user. Its end comes from its
//! `duration` in the residual, ordered by `onResidualOrder` 10 /
//! `onResidualSubOrder` 19 (keys with no `onResidual`: the dex's
//! `residual_order`).
//!
//! PS: data/moves.ts yawn.condition

use ps_protocol::{Arg, ArgValue, Kw, KwFlags, MoveNum, Msg};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::{conditions, moves};

fn yawn_arg() -> Arg {
    Arg::new(ArgValue::MoveEffect(MoveNum(moves::YAWN.0)))
}

/// `onStart(target, source)`: `-start|MON|move: Yawn|[of] SOURCE`.
// PS: data/moves.ts yawn.condition.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let source = a.source.expect("yawn without a source");
    let mon = b.ident(target);
    let of = b.ident(source);
    b.add(Msg::StartEffect { mon, effect: yawn_arg(), a: None, b: None, kw: Kw { of: Some(of), ..Kw::NONE } });
    None
}

/// `onEnd(target)`: `-end|MON|move: Yawn|[silent]`, then
/// `trySetStatus('slp', source)`.
// PS: data/moves.ts yawn.condition.onEnd
fn on_end(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let mon = b.ident(target);
    b.add(Msg::EndEffect { mon, effect: yawn_arg(), kw: Kw::flags(KwFlags::SILENT) });
    let source = b.effect_state().source;
    b.try_set_status(target, conditions::SLP, source, EffectId::None);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::End, on_end),
]);
