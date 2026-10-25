//! Gen 1's format rules with event handlers (the custom rules the harness
//! plays, and those of the variant formats public servers run): each lives
//! on the field as a pseudo-weather, as PS's `Battle` constructor adds
//! them. The rules PS defines once for every gen live here and gen 3
//! registers them too.

pub mod freezeclausemod;
pub mod sleepclausemod;
pub mod guaranteedsecondarymod;
pub mod species_mods;
