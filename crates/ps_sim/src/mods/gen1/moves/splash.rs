//! Splash: nothing happens (`-nothing`).
//!
//! No effect state.
//!
//! PS: data/moves.ts splash.onTry and splash.onTryHit (line 17679;
//! gen 1 inherits them whole: the gen 1 mod does not override Splash)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_protocol::Msg;

/// `onTry(source, target, move)`.
// PS: data/moves.ts splash.onTry
fn on_try(_b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    // `if (this.field.getPseudoWeather('Gravity'))`: gen 1 has no Gravity,
    // so the move is never refused and the handler returns `undefined`.
    None
}

/// `onTryHit(target, source)`.
// PS: data/moves.ts splash.onTryHit
fn on_try_hit(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    // `this.add('-nothing')`.
    b.add(Msg::Nothing);
    None
}

pub static DEF: EffectDef =
    EffectDef::new(&[Handler::on(EventId::Try, on_try), Handler::on(EventId::TryHit, on_try_hit)]);
