//! Minus: the holder's Special Attack is 1.5x while any active Pokemon has
//! Plus (gen 3 counts the foe too: every active Pokemon, not only allies).
//!
//! No ability state.
//!
//! PS: data/mods/gen3/abilities.ts minus

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `onModifySpA(spa, pokemon)`: `chainModify(1.5)` once an unfainted
/// active Pokemon has Plus.
// PS: data/mods/gen3/abilities.ts minus.onModifySpA
fn on_modify_spa(b: &mut Battle, _a: &EventArgs) -> Option<Relay> {
    let plus = b.dex.ability_id("plus").expect("Plus in the dex");
    let (actives, n) = b.all_active(false);
    for &active in &actives[..n] {
        if !b.mon(active).fainted && b.has_ability(active, plus) {
            b.chain_modify_f(1.5);
            return None;
        }
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::ModifySpA, on_modify_spa)]);
