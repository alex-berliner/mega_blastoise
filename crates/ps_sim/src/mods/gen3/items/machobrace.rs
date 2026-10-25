//! `machobrace`: Macho Brace, the holder's speed halved.
//!
//! PS: data/items.ts machobrace

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onModifySpe(spe)`: `return this.chainModify(0.5)`.
// PS: data/items.ts machobrace.onModifySpe
fn on_modify_spe(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    b.chain_modify_f(0.5);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::ModifySpe, on_modify_spe)]);
