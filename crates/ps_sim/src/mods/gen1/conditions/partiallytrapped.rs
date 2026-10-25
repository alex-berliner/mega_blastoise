//! `partiallytrapped`: the target of Wrap, Bind, Clamp or Fire Spin. It
//! cannot move while it lasts (the trapper's `partialtrappinglock` renews
//! it every turn), but it may switch out.
//!
//! Duration 2 (the dex), so it runs out one turn after the trapper stops.
//! Gen 1's `endTurn` also drops it once the trapper's lock is gone
//! (`Battle::gen1_partial_trapping_cleanup`).
//!
//! Effect state:
//! - `source`: the trapper (PS's `this.effectState.source`);
//! - `maybeLocked`: `data[0]` (1 for `true`), set by the trapper's
//!   `partialtrappinglock.onAfterMove`.
//!
//! PS: data/mods/gen1/conditions.ts partiallytrapped (lines 192-217)

use ps_protocol::{Arg, Msg, Word};

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onBeforeMove(pokemon)`: the trapped Pokemon cannot move.
// PS: data/mods/gen1/conditions.ts partiallytrapped.onBeforeMove
fn on_before_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let mon = b.ident(pokemon);
    b.add(Msg::Cant { mon, reason: Arg::word(Word::PartiallyTrapped), mv: None, kw: ps_protocol::Kw::NONE });
    Some(Relay::FAIL)
}

/// `onRestart()`: another turn of the trap.
// PS: data/mods/gen1/conditions.ts partiallytrapped.onRestart
fn on_restart(b: &mut Battle, _: &EventArgs) -> Option<Relay> {
    b.effect_state().duration = 2;
    None
}

/// `onAccuracy(accuracy, target, source, move)`: the trapper never misses
/// it.
// PS: data/mods/gen1/conditions.ts partiallytrapped.onAccuracy
fn on_accuracy(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    if a.source.is_some() && a.source == b.effect_state().source {
        return Some(Relay::TRUE);
    }
    None
}

/// `onDisableMove(target)`: the player may not know the trap goes on.
// PS: data/mods/gen1/conditions.ts partiallytrapped.onDisableMove
fn on_disable_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    if b.effect_state().data[0] != 0 {
        b.mon_mut(target).maybe_locked = true;
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::BeforeMove, on_before_move).priority(9),
    Handler::on(EventId::Restart, on_restart),
    Handler::on(EventId::Accuracy, on_accuracy),
    Handler::on(EventId::DisableMove, on_disable_move).priority(1),
]);
