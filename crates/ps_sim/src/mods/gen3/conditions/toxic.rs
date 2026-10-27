//! `toxic`: Toxic's own condition (`duration: 1`, `noCopy`): from gen 6 a
//! Poison-type user's Toxic cannot miss. Only gen 6's `onPrepareHit` adds
//! it, which gen 3's Toxic does not have, so it never starts in gen 3; it
//! is ported because the move names it.
//!
//! Effect state: `target` is the user.
//!
//! PS: data/mods/gen7/moves.ts toxic.condition

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler, Prefix};
use crate::relay::Relay;
use crate::state::{Battle, Holder};

/// `move && source === this.effectState.target`.
fn own_move(b: &mut Battle, a: &EventArgs) -> bool {
    let holder = b.effect_state().target;
    matches!(a.effect, EffectId::Move(_)) && a.source.is_some_and(|s| holder == Holder::Mon(s))
}

/// `onSourceInvulnerability(target, source, move)`: `0`.
// PS: data/mods/gen7/moves.ts toxic.condition.onSourceInvulnerability
fn on_source_invulnerability(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    own_move(b, a).then_some(Relay::Int(0))
}

/// `onSourceAccuracy(accuracy, target, source, move)`: `true`.
// PS: data/mods/gen7/moves.ts toxic.condition.onSourceAccuracy
fn on_source_accuracy(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    own_move(b, a).then_some(Relay::TRUE)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Invulnerability, on_source_invulnerability).prefix(Prefix::Source).priority(1),
    Handler::on(EventId::Accuracy, on_source_accuracy).prefix(Prefix::Source),
]);
