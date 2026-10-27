//! Fake Out: works only on the user's first action since it came in
//! (`activeMoveActions`). No effect state.
//!
//! PS: data/moves.ts fakeout.onTry

use ps_protocol::TextId;

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onTry(source)`: past the first action, the hint and `false`.
// PS: data/moves.ts fakeout.onTry
fn on_try(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let source = a.target.mon()?;
    if b.mon(source).active_move_actions > 1 {
        b.hint(TextId::HintFakeOut, false, None);
        return Some(Relay::FAIL);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::Try, on_try)]);
