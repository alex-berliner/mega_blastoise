//! Jump Kick: the user takes 1 damage when the move fails (it missed,
//! or it had no effect).
//!
//! No effect state.
//!
//! PS: data/mods/gen1/moves.ts jumpkick.onMoveFail (line 425; gen 1's own
//! handler; the base move has none)

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onMoveFail(target, source, move)`.
// PS: data/mods/gen1/moves.ts jumpkick.onMoveFail
fn on_move_fail(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    // The event's target is the move's target and its source the user.
    let target = a.target.mon()?;
    let source = a.source?;
    // `this.directDamage(1, source, target)`: the user takes the damage,
    // and the move is the effect of it (`this.effect`).
    b.direct_damage(1, Some(source), Some(target), EffectId::None);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::MoveFail, on_move_fail)]);
