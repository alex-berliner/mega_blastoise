//! Splash: does nothing (`-nothing`). No effect state.
//!
//! PS: data/moves.ts splash (onTry, onTryHit)

use ps_protocol::Msg;

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onTry(source, target, move)`: Gravity stops it. Gravity is gen 4, so
/// the gen 3 field never has it.
// PS: data/moves.ts splash.onTry
fn on_try(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    assert!(b.dex.condition_id("gravity").is_none(), "unported: Splash under Gravity");
    None
}

/// `onTryHit(target, source)`: `-nothing`.
// PS: data/moves.ts splash.onTryHit
fn on_try_hit(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    b.add(Msg::Nothing);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Try, on_try),
    Handler::on(EventId::TryHit, on_try_hit),
]);
