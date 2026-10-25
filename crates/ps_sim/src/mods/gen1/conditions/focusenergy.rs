//! `focusenergy`: Focus Energy's volatile. Gen 1's `getDamage` reads it
//! (the crit chance bug); gen 1 drops the base `onModifyCritRatio`, so only
//! the base `onStart` is left.
//!
//! Effect state: none.
//!
//! PS: data/moves.ts focusenergy.condition.onStart (lines 5983-5992), with
//! data/mods/gen1/moves.ts focusenergy (lines 343-350)

use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen1::moves;

/// `onStart(target, source, effect)`: `-start|MON|move: Focus Energy`.
/// The `[zeffect]` and `[silent]` branches cannot run in gen 1: the effect
/// is never Z-Power, Costar, Imposter, Psych Up or Transform (Transform
/// copies no volatiles in gen 1).
// PS: data/moves.ts focusenergy.condition.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    // `if (target.volatiles['dragoncheer']) return false;`
    if b.has_volatile_key(target, "dragoncheer") {
        return Some(Relay::FAIL);
    }
    let mon = b.ident(target);
    let effect = Arg::new(ArgValue::MoveEffect(MoveNum(moves::FOCUSENERGY.0)));
    b.add(Msg::StartEffect { mon, effect, a: None, b: None, kw: Kw::NONE });
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Start, on_start)]);
