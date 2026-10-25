//! `deepseatooth`: Deep Sea Tooth, Clamperl's special attack doubled.
//!
//! PS: data/mods/gen4/items.ts deepseatooth

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::mods::gen3::items::species_name;
use crate::relay::Relay;
use crate::state::Battle;

/// `onModifySpA(spa, pokemon)`: `if (pokemon.species.name === "Clamperl")
/// return this.chainModify(2)`.
// PS: data/mods/gen4/items.ts deepseatooth.onModifySpA
fn on_modify_spa(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    if species_name(b, pokemon) == "Clamperl" {
        b.chain_modify(2, 1);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::ModifySpA, on_modify_spa).priority(1)]);
