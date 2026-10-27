//! Fly: a two-turn move. The first turn lifts the user out of reach, which
//! makes it semi-invulnerable; the second one hits from the air.
//!
//! PS: data/mods/gen1/moves.ts fly, `onTryMove` (lines 329-341);
//! data/mods/gen2/moves.ts fly, `onPrepareHit` (lines 192-196).

use ps_dex::gen1::conditions;

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::mods::gen1::conditions::twoturnmove;
use crate::relay::Relay;
use crate::state::Battle;

/// `onTryMove(attacker, defender, move)`.
// PS: data/mods/gen1/moves.ts fly.onTryMove (lines 332-341)
fn on_try_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    // Fly lifts the user out of reach while it charges.
    twoturnmove::charge(b, a, true)
}

/// `onPrepareHit(target, source)`: the move fails while the user is
/// asleep. Gen 1's pipeline does not run the `PrepareHit` event, so this
/// handler never fires in gen 1; PS's data has it, so it is here.
// PS: data/mods/gen2/moves.ts fly.onPrepareHit (lines 194-196)
fn on_prepare_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    // `return source.status !== 'slp';`
    let asleep = a.source.is_some_and(|s| b.mon(s).status == conditions::SLP);
    Some(Relay::Bool(!asleep))
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::TryMove, on_try_move),
    Handler::on(EventId::PrepareHit, on_prepare_hit),
]);
