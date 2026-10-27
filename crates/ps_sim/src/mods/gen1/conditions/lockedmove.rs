//! `lockedmove`: Thrash and Petal Dance, 2 or 3 turns locked into the
//! move. Gen 1's `tryMoveHit` adds it (the moves' `self` effect is skipped
//! in `moveHit`), counts `time` down before each hit and confuses the user
//! when it reaches 0; `onAfterMove` then removes it. `accuracy` is the
//! Thrash accuracy bug: the last turn's final accuracy carries over.
//!
//! Effect state:
//! - `move`: `move_`;
//! - `time`: `data[0]`, as `tryMoveHit` reads it;
//! - `accuracy`: `data[1]`, as `tryMoveHit` reads it.
//!
//! PS: data/mods/gen1/conditions.ts lockedmove (lines 289-305)

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen1::conditions;

/// `onStart(target, source, effect)`.
// PS: data/mods/gen1/conditions.ts lockedmove.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let time = b.prng.random_range(2, 4) as i16;
    let st = b.effect_state();
    st.move_ = match a.effect {
        EffectId::Move(m) => m,
        other => panic!("lockedmove from {other:?}"),
    };
    st.data[0] = time;
    st.data[1] = 255;
    None
}

/// `onLockMove()`: the move it is locked into.
// PS: data/mods/gen1/conditions.ts lockedmove.onLockMove
fn on_lock_move(b: &mut Battle, _: &EventArgs) -> Option<Relay> {
    Some(Relay::Move(b.effect_state().move_))
}

/// `onAfterMove(pokemon)`: the lock ends once `time` has run out.
// PS: data/mods/gen1/conditions.ts lockedmove.onAfterMove
fn on_after_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let key = EffectId::Condition(conditions::LOCKEDMOVE);
    let time = b.mon(pokemon).volatiles.get(key).expect("pokemon.volatiles['lockedmove']").data[0];
    if time <= 0 {
        b.remove_volatile(pokemon, conditions::LOCKEDMOVE);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::LockMove, on_lock_move),
    Handler::on(EventId::AfterMove, on_after_move),
]);
