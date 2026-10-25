//! `lightball`: Light Ball, Pikachu's special attack doubled. Gen 3 keeps
//! only `onModifySpA` (the `onModifyAtkPriority` has no handler).
//!
//! PS: data/mods/gen3/items.ts lightball

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::mods::gen3::items::species_name;
use crate::relay::Relay;
use crate::state::Battle;

/// `onModifySpA(spa, pokemon)`: `if (pokemon.species.name === "Pikachu")
/// return this.chainModify(2)`.
// PS: data/mods/gen3/items.ts lightball.onModifySpA
fn on_modify_spa(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    if species_name(b, pokemon) == "Pikachu" {
        b.chain_modify(2, 1);
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::ModifySpA, on_modify_spa).priority(1)]);
