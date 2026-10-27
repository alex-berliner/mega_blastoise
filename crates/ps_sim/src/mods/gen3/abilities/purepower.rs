//! Pure Power: the holder's Attack is doubled.
//!
//! The ability keeps no state of its own.
//!
//! PS: data/abilities.ts purepower (onModifyAtk)

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onModifyAtk(atk)`: `this.chainModify(2)`.
fn on_modify_atk(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    b.chain_modify_f(2.0);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::ModifyAtk, on_modify_atk).priority(5)]);
