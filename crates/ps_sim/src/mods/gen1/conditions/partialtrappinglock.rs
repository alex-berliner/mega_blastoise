//! `partialtrappinglock`: the user of Wrap, Bind, Clamp or Fire Spin,
//! locked into the move for 2 to 5 turns. Every turn it repeats the first
//! turn's damage (`getDamage` reads `damage`) and renews the target's
//! `partiallytrapped`; on the last turn both Pokemon get
//! `fakepartiallytrapped`.
//!
//! The move's `self` effect adds it, so `effect` in `onStart` is the move.
//! The pipeline drops it with a plain `delete` on a miss (`tryMoveHit`),
//! and gen 1's `endTurn` once the target has fainted or is no longer
//! trapped (`Battle::gen1_partial_trapping_cleanup`).
//!
//! Effect state:
//! - `move`: `move_`;
//! - `locked`: `data[0]` as a Pokemon (`EffectState::data_mon`), as
//!   `endTurn` and `getDamage` read it;
//! - `damage`: `data[1]`, as `getDamage` reads it;
//! - `totalDuration`: `data[2]`;
//! - `duration`: `duration`, from `durationCallback`.
//!
//! PS: data/mods/gen1/conditions.ts partialtrappinglock (lines 228-275)

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen1::conditions;

/// `durationCallback()`: 2, 3, 4 or 5 turns, as the cartridge draws them.
// PS: data/mods/gen1/conditions.ts partialtrappinglock.durationCallback
fn duration_callback(b: &mut Battle, _: &EventArgs) -> Option<Relay> {
    Some(Relay::Int(*b.prng.sample(&[2, 2, 2, 3, 3, 3, 4, 5])))
}

/// `onStart(target, source, effect)`: lock onto the foe and trap it.
// PS: data/mods/gen1/conditions.ts partialtrappinglock.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let (foes, n) = b.allies(1 - target.side);
    if n == 0 {
        return Some(Relay::FAIL);
    }
    let foe = foes[0];
    let last_damage = b.last_damage;
    let st = b.effect_state();
    st.move_ = match a.effect {
        EffectId::Move(m) => m,
        other => panic!("partialtrappinglock from {other:?}"),
    };
    st.data[2] = i16::from(st.duration);
    st.data[1] = last_damage as i16;
    st.set_data_mon(0, Some(foe));
    b.add_volatile(foe, conditions::PARTIALLYTRAPPED, Some(target), a.effect);
    None
}

/// `onBeforeMove(pokemon, target, move)`: a new target ends the lock.
// PS: data/mods/gen1/conditions.ts partialtrappinglock.onBeforeMove
fn on_before_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    if a.source != b.effect_state().data_mon(0) {
        b.remove_volatile(pokemon, conditions::PARTIALTRAPPINGLOCK);
    }
    None
}

/// `onAfterMove(pokemon, target, move)`: renew the trap, or end it with the
/// fake traps on the last turn.
// PS: data/mods/gen1/conditions.ts partialtrappinglock.onAfterMove
fn on_after_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let target = a.source;
    if let Some(t) = target {
        if b.mon(t).hp == 0 {
            // A plain `delete`: no End event.
            b.mon_mut(pokemon).volatiles.remove(EffectId::Condition(conditions::PARTIALTRAPPINGLOCK));
            return None;
        }
    }
    let target = target.expect("partialtrappinglock.onAfterMove without a target");
    let st = b.effect_state();
    let (duration, total_duration) = (i16::from(st.duration), st.data[2]);
    let fake = EffectId::Condition(conditions::FAKEPARTIALLYTRAPPED);
    if duration == 1 {
        if total_duration != 5 {
            b.add_volatile(pokemon, conditions::FAKEPARTIALLYTRAPPED, None, EffectId::None);
            b.mon_mut(pokemon).volatiles.get_mut(fake).expect("fakepartiallytrapped").set_data_mon(0, Some(target));
            b.add_volatile(target, conditions::FAKEPARTIALLYTRAPPED, None, EffectId::None);
            b.mon_mut(target).volatiles.get_mut(fake).expect("fakepartiallytrapped").set_data_mon(0, Some(pokemon));
        }
    } else {
        b.add_volatile(target, conditions::PARTIALLYTRAPPED, Some(pokemon), a.effect);
        if total_duration - duration > 0 {
            let trapped = EffectId::Condition(conditions::PARTIALLYTRAPPED);
            // `maybeLocked` (data[0]).
            b.mon_mut(target).volatiles.get_mut(trapped).expect("partiallytrapped").data[0] = 1;
        }
    }
    None
}

/// `onSemiLockMove()`: the move it is locked into.
// PS: data/mods/gen1/conditions.ts partialtrappinglock.onSemiLockMove
fn on_semi_lock_move(b: &mut Battle, _: &EventArgs) -> Option<Relay> {
    Some(Relay::Move(b.effect_state().move_))
}

/// `onDisableMove(pokemon)`: past its second turn the lock may have ended
/// for all the player knows.
// PS: data/mods/gen1/conditions.ts partialtrappinglock.onDisableMove
fn on_disable_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let st = b.effect_state();
    if st.data[2] - i16::from(st.duration) > 1 {
        b.mon_mut(pokemon).maybe_locked = true;
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::DurationCallback, duration_callback),
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::BeforeMove, on_before_move),
    Handler::on(EventId::AfterMove, on_after_move),
    Handler::on(EventId::SemiLockMove, on_semi_lock_move),
    Handler::on(EventId::DisableMove, on_disable_move),
]);
