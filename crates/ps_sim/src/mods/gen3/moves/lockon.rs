//! Lock-On: the user's next move against the target cannot miss (the
//! `lockon` volatile on the user). Mind Reader is the same move.
//!
//! PS: data/moves.ts lockon (onTryHit, onHit)

use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::{conditions, moves};
use ps_dex::MoveId;

/// `onTryHit(target, source)`: `false` while the user already has a lock.
// PS: data/moves.ts lockon.onTryHit
pub fn on_try_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let source = a.source.expect("lockon without a source");
    if b.has_vol(source, conditions::LOCKON) {
        return Some(Relay::FAIL);
    }
    None
}

/// `onHit(target, source)` of Lock-On or Mind Reader: the volatile on the
/// user, then `-activate|USER|move: NAME|[of] TARGET`.
// PS: data/moves.ts lockon.onHit, mindreader.onHit
pub fn hit(b: &mut Battle, a: &EventArgs, mv: MoveId) -> Option<Relay> {
    let target = a.target.mon()?;
    let source = a.source.expect("lockon without a source");
    b.add_volatile(source, conditions::LOCKON, Some(target), EffectId::None);
    let mon = b.ident(source);
    let of = b.ident(target);
    let effect = Arg::new(ArgValue::MoveEffect(MoveNum(mv.0)));
    b.add(Msg::Activate { mon: Some(mon), effect, a: None, b: None, kw: Kw { of: Some(of), ..Kw::NONE } });
    None
}

fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    hit(b, a, moves::LOCKON)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::TryHit, on_try_hit),
    Handler::on(EventId::Hit, on_hit),
]);
