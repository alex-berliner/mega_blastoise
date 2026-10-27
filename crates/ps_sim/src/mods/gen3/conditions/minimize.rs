//! `minimize`: Minimize's volatile. Moves with the `minimize` flag deal
//! double damage to the holder. Using Minimize again keeps it (`onRestart`
//! answers `null`). No effect state.
//!
//! PS: data/moves.ts minimize.condition

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler, Prefix};
use crate::relay::Relay;
use crate::state::Battle;

/// `onRestart: () => null`: the volatile stays, and adding it again fails
/// silently.
// PS: data/moves.ts minimize.condition.onRestart
fn on_restart(_b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    Some(Relay::Null)
}

/// `onSourceModifyDamage(damage, source, target, move)`: double for a
/// `minimize` move.
// PS: data/moves.ts minimize.condition.onSourceModifyDamage
fn on_source_modify_damage(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    // A future move's damage roll is on a plain `moveData` (no id, so the
    // event effect is none) whose flags are `metronome` and `futuremove`.
    let EffectId::Move(m) = a.effect else { return None };
    if b.dex.move_def(m).flags.has("minimize") {
        b.chain_modify(2, 1);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Restart, on_restart),
    Handler::on(EventId::ModifyDamage, on_source_modify_damage).prefix(Prefix::Source),
]);
