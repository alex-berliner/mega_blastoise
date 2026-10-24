//! Substitute: fails with a doll already up or under a quarter of the
//! user's HP, and costs a quarter of its max HP (nothing at 3 max HP or
//! less; exactly a quarter left faints the user).
//!
//! PS: data/mods/gen1/moves.ts substitute (`onTryHit` lines 805-816,
//! `onHit` lines 817-822)

use ps_protocol::{Arg, ArgValue, Kw, KwFlags, MoveNum, Msg};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen1::{conditions, moves};

/// `onTryHit(target)`: `-fail|MON|move: Substitute` (with `[weak]` for
/// too little HP) and `null`.
// PS: data/mods/gen1/moves.ts substitute.onTryHit
fn on_try_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let what = Some(Arg::new(ArgValue::MoveEffect(MoveNum(moves::SUBSTITUTE.0))));
    let kw = if b.has_vol(target, conditions::SUBSTITUTE) {
        Kw::NONE
    } else if u32::from(b.mon(target).hp) * 4 < u32::from(b.mon(target).maxhp) {
        // `target.hp < target.maxhp / 4`.
        Kw::flags(KwFlags::WEAK)
    } else {
        return None;
    };
    let mon = b.ident(target);
    b.add(Msg::Fail { mon, what, what2: None, kw });
    Some(Relay::Null)
}

/// `onHit(target)`: `directDamage(target.maxhp / 4, target, target)`,
/// which floors the quarter.
// PS: data/mods/gen1/moves.ts substitute.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let maxhp = i32::from(b.mon(target).maxhp);
    if maxhp > 3 {
        b.direct_damage(maxhp / 4, Some(target), Some(target), EffectId::None);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::TryHit, on_try_hit),
    Handler::on(EventId::Hit, on_hit),
]);
