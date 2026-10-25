//! Gen 3's format rules with event handlers (`gen3randombattle`'s Sleep
//! Clause Mod, and the variant formats' stat and move mods): each lives on
//! the field as a pseudo-weather, as PS's `Battle` constructor adds them.
//! PS defines them once for every gen and gen 3 overrides none, so gen 3
//! registers gen 1's ports (`crate::mods::gen1::rules`), except Scalemons
//! Mod and 350 Cup Mod, which gen 1 overrides: their base versions are here
//! (`species_mods`).

pub mod sleepclausemod;
pub mod species_mods;
