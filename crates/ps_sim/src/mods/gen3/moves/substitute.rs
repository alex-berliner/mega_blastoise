//! Substitute: fails with a doll already up or at a quarter of the user's
//! HP or less, and costs a quarter of its max HP; the volatile does the
//! rest (`conditions::substitute`).
//!
//! PS: data/moves.ts substitute (onTryHit, onHit)

use ps_protocol::{Arg, ArgValue, Kw, KwFlags, MoveNum, Msg};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::{conditions, moves};

/// `onTryHit(source)`: `-fail|MON|move: Substitute` (with `[weak]` for too
/// little HP) and `NOT_FAIL`.
// PS: data/moves.ts substitute.onTryHit
fn on_try_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let source = a.target.mon()?;
    let what = Some(Arg::new(ArgValue::MoveEffect(MoveNum(moves::SUBSTITUTE.0))));
    let (hp, maxhp) = (u32::from(b.mon(source).hp), u32::from(b.mon(source).maxhp));
    let kw = if b.has_vol(source, conditions::SUBSTITUTE) {
        Kw::NONE
    } else if hp * 4 <= maxhp || maxhp == 1 {
        // `source.hp <= source.maxhp / 4 || source.maxhp === 1`.
        Kw::flags(KwFlags::WEAK)
    } else {
        return None;
    };
    let mon = b.ident(source);
    b.add(Msg::Fail { mon, what, what2: None, kw });
    Some(Relay::NOT_FAIL)
}

/// `onHit(target)`: `directDamage(target.maxhp / 4)`, whose
/// `clampIntRange(damage, 1)` floors the quarter and makes it at least 1.
// PS: data/moves.ts substitute.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let d = (i32::from(b.mon(target).maxhp) / 4).max(1);
    b.direct_damage(d, Some(target), None, EffectId::None);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::TryHit, on_try_hit),
    Handler::on(EventId::Hit, on_hit),
]);
