//! Guts: the holder's Attack is 1.5 times as high while it is statused.
//!
//! The ability keeps no state of its own.
//!
//! PS: data/abilities.ts guts (onModifyAtk)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onModifyAtk(atk, pokemon)`: `this.chainModify(1.5)` while the holder
/// has a status.
fn on_modify_atk(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    if b.mon(pokemon).status.is_none() {
        return None;
    }
    b.chain_modify_f(1.5);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::ModifyAtk, on_modify_atk).priority(5)]);
