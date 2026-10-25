//! Gen 3 items with handlers: one file per effect (or per shared
//! handler group; the file names match PS's ids so a `git grep` and a
//! `show-effect.js query` both fit).

pub mod brightpowder;
pub mod choiceband;
pub mod deepseascale;
pub mod deepseatooth;
pub mod flavor;
pub mod focusband;
pub mod heal_berries;
pub mod kingsrock;
pub mod laxincense;
pub mod leftovers;
pub mod leppa;
pub mod lightball;
pub mod luckypunch;
pub mod lum;
pub mod machobrace;
pub mod mail;
pub mod mentalherb;
pub mod metalpowder;
pub mod persim;
pub mod scopelens;
pub mod shellbell;
pub mod souldew;
pub mod stat_berries;
pub mod status_cure;
pub mod stick;
pub mod thickclub;
pub mod type_boost;
pub mod whiteherb;

#[cfg(test)]
mod held_tests;
#[cfg(test)]
mod tests;

use crate::effect::EventArgs;
use crate::relay::Relay;
use crate::state::{Battle, MonRef};

/// `pokemon.species.name`: the species the holder has now (Transform
/// changes it).
pub fn species_name(b: &Battle, pokemon: MonRef) -> &'static str {
    b.dex.species_def(b.mon(pokemon).species).name.as_str()
}

/// `critRatio`, the number `ModifyCritRatio` relays.
pub fn crit_ratio(a: &EventArgs) -> i32 {
    match a.relay {
        Some(Relay::Int(n)) => n,
        other => panic!("ModifyCritRatio relay {other:?}"),
    }
}
