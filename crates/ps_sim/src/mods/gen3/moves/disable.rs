//! Disable: fails against a target that has not moved yet or last used
//! Struggle; the volatile does the rest (`conditions::disable`).
//!
//! PS: data/moves.ts disable.onTryHit

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::moves;

/// `onTryHit(target)`: `false` for no last move or Struggle (no Z or Max
/// moves in gen 3).
// PS: data/moves.ts disable.onTryHit
fn on_try_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let last = b.mon(target).last_move;
    if last.is_none() || last == moves::STRUGGLE {
        return Some(Relay::FAIL);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::TryHit, on_try_hit)]);
