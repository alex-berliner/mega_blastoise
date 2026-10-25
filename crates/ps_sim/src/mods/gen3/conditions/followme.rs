//! `followme`: Follow Me's slot condition. Its move fails in singles
//! (`onTry`: `activePerHalf > 1`), so it never starts there; it is ported
//! for completeness.
//!
//! Effect state: `slot` (the user's position) is `data[SLOT]`.
//!
//! PS: data/mods/gen3/moves.ts followme.condition

use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg};

use crate::effect::{EffectDef, EventArgs, EventId, Handler, Prefix};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::moves;

/// `this.effectState.slot`, the user's position.
pub const SLOT: usize = 0;

/// `onStart(target, source, effect)`: `-singleturn|MON|move: Follow Me`
/// and the user's slot.
// PS: data/mods/gen3/moves.ts followme.condition.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let mon = b.ident(target);
    let effect = Arg::new(ArgValue::MoveEffect(MoveNum(moves::FOLLOWME.0)));
    b.add(Msg::SingleTurn { mon, effect, kw: Kw::NONE });
    let position = b.mon(target).position;
    b.effect_state().data[SLOT] = i16::from(position);
    None
}

/// `onFoeRedirectTarget(target, source, source2, move)`: the user's slot,
/// when valid. PS runs `RedirectTarget` only with `activePerHalf > 1`.
// PS: data/mods/gen3/moves.ts followme.condition.onFoeRedirectTarget
fn on_foe_redirect_target(_b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    unreachable!("RedirectTarget runs only in doubles")
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::RedirectTarget, on_foe_redirect_target).prefix(Prefix::Foe).priority(1),
]);
