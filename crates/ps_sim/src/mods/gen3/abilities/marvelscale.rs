//! Marvel Scale: the holder's Defense is 1.5 times as high while it is
//! statused.
//!
//! The ability keeps no state of its own.
//!
//! PS: data/abilities.ts marvelscale (onModifyDef)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onModifyDef(def, pokemon)`: `this.chainModify(1.5)` while the holder
/// has a status.
fn on_modify_def(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    if b.mon(pokemon).status.is_none() {
        return None;
    }
    b.chain_modify_f(1.5);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::ModifyDef, on_modify_def).priority(6)]);
