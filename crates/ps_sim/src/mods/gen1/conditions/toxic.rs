//! The `toxic` condition of the Toxic move: the semi-invulnerability a
//! poisoned attacker gets (gen 7's Toxic). Gen 1's Toxic has no
//! `onPrepareHit`, so the volatile is never created and these handlers
//! never run; the move's own condition still has to be ported for the
//! move to be.
//!
//! `this.effectState.target` is the Pokemon holding the volatile.
//!
//! PS: data/mods/gen7/moves.ts toxic (line 1031)

use crate::effect::{EffectDef, EventArgs, EventId, Handler, Prefix};
use crate::relay::Relay;
use crate::state::Battle;

/// `onSourceInvulnerability(target, source, move)`: `if (move && source ===
/// this.effectState.target) return 0;`.
// PS: data/mods/gen7/moves.ts toxic.condition.onSourceInvulnerability
fn on_source_invulnerability(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    if a.effect.is_none() {
        return None;
    }
    if b.effect_state().target.mon() == a.source {
        return Some(Relay::Int(0));
    }
    None
}

/// `onSourceAccuracy(accuracy, target, source, move)`: `if (move && source
/// === this.effectState.target) return true;`.
// PS: data/mods/gen7/moves.ts toxic.condition.onSourceAccuracy
fn on_source_accuracy(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    if a.effect.is_none() {
        return None;
    }
    if b.effect_state().target.mon() == a.source {
        return Some(Relay::Bool(true));
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Invulnerability, on_source_invulnerability).prefix(Prefix::Source).priority(1),
    Handler::on(EventId::Accuracy, on_source_accuracy).prefix(Prefix::Source),
]);
