//! `souldew`: Soul Dew, Latias' and Latios' special attack and special
//! defense times 1.5 (the gen 6 mod's stat boost; its
//! `onBasePowerPriority` has no handler).
//!
//! PS: data/mods/gen6/items.ts souldew

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;

/// `if (pokemon.baseSpecies.num === 380 || pokemon.baseSpecies.num ===
/// 381) return this.chainModify(1.5)`: the base species, which Transform
/// does not change.
// PS: data/mods/gen6/items.ts souldew.onModifySpA / onModifySpD
fn on_modify_stat(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let num = b.dex.species_def(b.mon(pokemon).base_species).num;
    if num == 380 || num == 381 {
        b.chain_modify_f(1.5);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::ModifySpA, on_modify_stat).priority(1),
    Handler::on(EventId::ModifySpD, on_modify_stat).priority(2),
]);
