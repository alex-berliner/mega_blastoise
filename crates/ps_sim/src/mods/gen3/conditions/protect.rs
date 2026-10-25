//! `protect`: Protect's and Detect's volatile. It stops the moves PS's
//! `checkMoveBypassesProtect` refuses (a move with the `protect` flag that
//! the `HitProtect` event does not let through) and lasts one turn.
//!
//! Effect state: none.
//!
//! PS: data/conditions.ts protect (onStart), data/mods/gen4/conditions.ts
//! protect (onTryHit)

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::{conditions, moves};
use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg};

/// `onStart(target)`: `-singleturn|MON|Protect`.
// PS: data/conditions.ts protect.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let mon = b.ident(target);
    let effect = Arg::new(ArgValue::Move(MoveNum(moves::PROTECT.0)));
    b.add(Msg::SingleTurn { mon, effect, kw: Kw::NONE });
    None
}

/// `onTryHit(target, source, move)`: the move is stopped unless it
/// bypasses Protect.
// PS: data/mods/gen4/conditions.ts protect.onTryHit
fn on_try_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let source = a.source?;
    let move_id = match a.effect {
        EffectId::Move(m) => m,
        other => panic!("protect against {other:?}"),
    };
    // `if (this.checkMoveBypassesProtect(move, source, target)) return;`
    if b.check_move_bypasses_protect(move_id, source, target, true) {
        return None;
    }
    let mon = b.ident(target);
    let effect = Arg::new(ArgValue::Move(MoveNum(moves::PROTECT.0)));
    b.add(Msg::Activate { mon: Some(mon), effect, a: None, b: None, kw: Kw::NONE });
    // `const lockedmove = source.getVolatile('lockedmove')`: a locked
    // move (Thrash, Outrage) keeps its lock when Protect stops the hit.
    let lockedmove = EffectId::Condition(conditions::LOCKEDMOVE);
    if let Some(st) = b.mon_mut(source).volatiles.get_mut(lockedmove) {
        if st.data[super::lockedmove::TRUE_DURATION] >= 2 {
            st.duration = 2;
        }
    }
    // `return null`: the hit is stopped.
    Some(Relay::Null)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::TryHit, on_try_hit).priority(3),
]);
