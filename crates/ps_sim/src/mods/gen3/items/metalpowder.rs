//! `metalpowder`: Metal Powder, an untransformed Ditto's defense doubled.
//!
//! PS: data/items.ts metalpowder

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::mods::gen3::items::species_name;
use crate::relay::Relay;
use crate::state::Battle;

/// `onModifyDef(def, pokemon)`: `if (pokemon.species.name === "Ditto" &&
/// !pokemon.transformed) return this.chainModify(2)`.
// PS: data/items.ts metalpowder.onModifyDef
fn on_modify_def(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    if species_name(b, pokemon) == "Ditto" && !b.mon(pokemon).transformed {
        b.chain_modify(2, 1);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::ModifyDef, on_modify_def).priority(2)]);
