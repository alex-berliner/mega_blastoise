//! `thickclub`: Thick Club, Cubone's and Marowak's attack doubled.
//!
//! PS: data/mods/gen4/items.ts thickclub

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::mods::gen3::items::species_name;
use crate::relay::Relay;
use crate::state::Battle;

/// `onModifyAtk(atk, pokemon)`: `if (pokemon.species.name === "Cubone" ||
/// pokemon.species.name === "Marowak") return this.chainModify(2)`.
// PS: data/mods/gen4/items.ts thickclub.onModifyAtk
fn on_modify_atk(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let name = species_name(b, pokemon);
    if name == "Cubone" || name == "Marowak" {
        b.chain_modify(2, 1);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::ModifyAtk, on_modify_atk).priority(1)]);
