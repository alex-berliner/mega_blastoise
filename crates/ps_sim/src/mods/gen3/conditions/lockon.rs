//! `lockon`: Lock-On's and Mind Reader's volatile on the user. Until the
//! end of the next turn the user's moves against its target cannot miss,
//! even against a semi-invulnerable target.
//!
//! Effect state: `source` is the locked target (`addVolatile('lockon',
//! target)`), `target` the user.
//!
//! PS: data/moves.ts lockon.condition

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler, Prefix};
use crate::relay::Relay;
use crate::state::{Battle, Holder};

/// `move && source === this.effectState.target && target ===
/// this.effectState.source`.
fn locked(b: &mut Battle, a: &EventArgs) -> bool {
    let st = *b.effect_state();
    matches!(a.effect, EffectId::Move(_))
        && a.source.is_some_and(|s| st.target == Holder::Mon(s))
        && a.target.mon().is_some_and(|t| st.source == Some(t))
}

/// `onSourceInvulnerability(target, source, move)`: `0`, so the target is
/// hittable.
// PS: data/moves.ts lockon.condition.onSourceInvulnerability
fn on_source_invulnerability(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    locked(b, a).then_some(Relay::Int(0))
}

/// `onSourceAccuracy(accuracy, target, source, move)`: `true`.
// PS: data/moves.ts lockon.condition.onSourceAccuracy
fn on_source_accuracy(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    locked(b, a).then_some(Relay::TRUE)
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Invulnerability, on_source_invulnerability).prefix(Prefix::Source).priority(1),
    Handler::on(EventId::Accuracy, on_source_accuracy).prefix(Prefix::Source),
]);
