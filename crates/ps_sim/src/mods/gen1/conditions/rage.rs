//! `rage`: Rage's lock (the move's `condition`, added by its `self`
//! effect). The user repeats Rage until it switches out or faints, and every
//! damaging hit it takes raises its Attack. Disable and the exploding moves
//! build Rage even when they miss; gen 1's `useMoveInner` does that part.
//! `accuracy` is the Rage accuracy bug, read and written by `tryMoveHit`.
//!
//! Effect state:
//! - `move`: `move_` (always Rage);
//! - `accuracy`: `data[1]`, as `tryMoveHit` reads it.
//!
//! PS: data/mods/gen1/moves.ts rage.condition (lines 604-618)

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen1::moves;
use ps_dex::MoveCategory;

/// `onStart(target, source, effect)`.
// PS: data/mods/gen1/moves.ts rage.condition.onStart
fn on_start(b: &mut Battle, _: &EventArgs) -> Option<Relay> {
    let st = b.effect_state();
    st.move_ = moves::RAGE;
    st.data[1] = 255;
    None
}

/// `onHit(target, source, move)`: a damaging hit builds Rage.
// PS: data/mods/gen1/moves.ts rage.condition.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let EffectId::Move(m) = a.effect else { return None };
    let def = b.dex.move_def(m);
    // `move.category` of the move object in flight.
    let category = b.active_move().filter(|am| am.id == m).map_or(def.category, |am| am.category);
    if b.mon(target).boosts[0] < 6 && category != MoveCategory::Status && def.selfdestruct().is_none() {
        // `this.boost({atk: 1})`: the event's target and source, and this
        // effect (`[from] Rage`).
        b.boost([1, 0, 0, 0, 0, 0, 0], target, None, EffectId::None);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::value(EventId::LockMove, Relay::Move(moves::RAGE)),
    Handler::on(EventId::Hit, on_hit),
]);
