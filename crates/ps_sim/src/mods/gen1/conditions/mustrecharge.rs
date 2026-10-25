//! `mustrecharge`: Hyper Beam's recharge turn. Gen 1 gives it no duration
//! and no `onStart`; the `-mustrecharge` line comes from `onAfterMove`
//! instead, and not at all when the target fainted (no recharge after a
//! KO).
//!
//! `onLockMove: 'recharge'` locks the request; gen 1's `commitChoices`
//! replays `lastSelectedMove` (Hyper Beam), and `onBeforeMove` stops it.
//! Partial trapping moves (`onTryMove`) and sleep (`moveHit`) clear it.
//! No effect state.
//!
//! PS: data/mods/gen1/conditions.ts mustrecharge (lines 276-288), inheriting
//! data/mods/gen4/conditions.ts mustrecharge (line 130) for `onBeforeMove`
//! and data/conditions.ts mustrecharge (line 364) for `onLockMove`

use ps_protocol::{Arg, Msg, Word};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen1::conditions;

/// `onBeforeMove(pokemon)`: the recharge turn. Gen 4's version, base's
/// without `pokemon.removeVolatile('truant')` (PS 104641c84).
// PS: data/mods/gen4/conditions.ts mustrecharge.onBeforeMove
fn on_before_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let mon = b.ident(pokemon);
    b.add(Msg::Cant { mon, reason: Arg::word(Word::Recharge), mv: None, kw: ps_protocol::Kw::NONE });
    b.remove_volatile(pokemon, conditions::MUSTRECHARGE);
    Some(Relay::Null)
}

/// `onAfterMove(pokemon, target, move)`.
// PS: data/mods/gen1/conditions.ts mustrecharge.onAfterMove
fn on_after_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    if let Some(t) = a.source {
        if b.mon(t).hp == 0 {
            // A plain `delete`: no End event.
            b.mon_mut(pokemon).volatiles.remove(EffectId::Condition(conditions::MUSTRECHARGE));
            return None;
        }
    }
    let mon = b.ident(pokemon);
    b.add(Msg::MustRecharge(mon));
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::BeforeMove, on_before_move).priority(7),
    Handler::value(EventId::LockMove, Relay::Recharge),
    Handler::on(EventId::AfterMove, on_after_move),
]);
