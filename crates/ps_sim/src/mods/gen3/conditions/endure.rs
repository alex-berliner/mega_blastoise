//! `endure`: Endure's volatile. A move's damage that would faint the
//! holder leaves it at 1 HP (a future move's does not). `duration: 1`. No
//! effect state.
//!
//! PS: data/moves.ts endure.condition (onStart), data/mods/gen4/moves.ts
//! endure.condition (onDamage)

use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::moves;

fn endure_arg() -> Arg {
    Arg::new(ArgValue::MoveEffect(MoveNum(moves::ENDURE.0)))
}

/// `onStart(target)`: `-singleturn|MON|move: Endure`.
// PS: data/moves.ts endure.condition.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let mon = b.ident(target);
    b.add(Msg::SingleTurn { mon, effect: endure_arg(), kw: Kw::NONE });
    None
}

/// `onDamage(damage, target, source, effect)`: `effect?.effectType ===
/// 'Move' && !effect?.flags['futuremove'] && damage >= target.hp`.
// PS: data/mods/gen4/moves.ts endure.condition.onDamage
fn on_damage(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let EffectId::Move(m) = a.effect else { return None };
    let Some(Relay::Int(damage)) = a.relay else { return None };
    let hp = i32::from(b.mon(target).hp);
    if !b.dex.move_def(m).flags.has("futuremove") && damage >= hp {
        let mon = b.ident(target);
        b.add(Msg::Activate { mon: Some(mon), effect: endure_arg(), a: None, b: None, kw: Kw::NONE });
        return Some(Relay::Int(hp - 1));
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::Damage, on_damage).priority(-10),
]);
