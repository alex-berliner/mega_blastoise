//! `deepseascale`: Deep Sea Scale, Clamperl's special defense doubled.
//!
//! PS: data/mods/gen4/items.ts deepseascale

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::mods::gen3::items::species_name;
use crate::relay::Relay;
use crate::state::Battle;

/// `onModifySpD(spd, pokemon)`: `if (pokemon.species.name === "Clamperl")
/// return this.chainModify(2)`. The holder is the defender.
// PS: data/mods/gen4/items.ts deepseascale.onModifySpD
fn on_modify_spd(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    if species_name(b, pokemon) == "Clamperl" {
        b.chain_modify(2, 1);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::ModifySpD, on_modify_spd).priority(2)]);
