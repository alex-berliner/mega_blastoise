//! Sleep Clause Mod: a foe's second sleeping Pokemon is refused.
//!
//! PS has one definition for every gen (`data/rulesets.ts`; no gen 3 or
//! gen 4 mod overrides it), so gen 3 registers gen 1's port of it.
//!
//! PS: data/rulesets.ts sleepclausemod

pub use crate::mods::gen1::rules::sleepclausemod::DEF;
