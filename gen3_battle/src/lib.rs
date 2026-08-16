//! Gen 3 (Ruby / Sapphire / Emerald) battle mechanics.
//!
//! A sibling to [`gen1_battle`], not a replacement for it. Gen 1 combat is
//! preserved exactly as it is: this crate is additive, and a device running a
//! Gen 1 battle never enters it. Which engine a battle uses is chosen once, at
//! setup, from the ruleset the players picked in the menu.
//!
//! What is here so far is the arithmetic core, which is where Gen 3 actually
//! departs from Gen 1:
//!
//!   * [`types`] — seventeen types (Dark and Steel are new, Fairy does not
//!     exist yet), the chart, and the physical/special split, which in Gen 3
//!     is still a property of the move's TYPE.
//!   * [`data`] — the dex and move list, generated at build time by merging
//!     the vendored gen1, gen2 and gen3 layers.
//!   * [`stats`] — Special split into Sp.Atk and Sp.Def, IVs, EVs and the 25
//!     natures.
//!   * [`damage`] — the Gen 3 damage formula, with its modifiers applied in
//!     the order the games apply them, since each step floors.
//!   * [`battle`] — a singles turn loop over those: Speed order, damage,
//!     faints, switches and the win condition.
//!
//! Still to come, in rough order of how much they change play: abilities,
//! held items, weather, and the move set beyond the shared Gen 1 entries.
//!
//! The mechanics are implemented from the published Gen 3 formulas. Upstream
//! `battler` is a useful cross-check for behaviour, but no code is taken from
//! it: this crate is `no_std` and allocation-free on the hot path, which that
//! engine is not.

#![cfg_attr(not(test), no_std)]

pub mod ability;
pub mod item;
pub mod battle;
pub mod damage;
pub mod data;
pub mod draft;
pub mod stats;
pub mod types;

pub use battle::{Battle, Choice, Event, Mon, MoveSlot, Rng, SeatScript, Side, TurnScript};
pub use damage::{crit_denominator, damage, Attacker, Defender, MoveUse, Roll};
pub use data::{
    move_by_id, species_by_id, BaseStats, Boost, MoveEntry, Secondary, SecondaryEffect,
    SideCondition, SpeciesEntry, Status, Weather, MOVES, SPECIES, TYPE_COUNT,
};
pub use draft::{draft_mon, draft_team};
pub use stats::{apply_stage, hp_stat, other_stat, Invest, Nature, Stat};
pub use types::{category_of, effectiveness, effectiveness_against, Category, Type};

/// Which generation's rules a battle runs under.
///
/// Lives here rather than in the menu so that both engines and the device
/// agree on one spelling of the question, and so a caller holding a
/// `Ruleset` cannot forget that Gen 1 is still a live option.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Ruleset {
    #[default]
    Gen1,
    Gen3,
}

impl Ruleset {
    pub fn as_str(self) -> &'static str {
        match self {
            Ruleset::Gen1 => "Gen 1",
            Ruleset::Gen3 => "Gen 3",
        }
    }

    /// True when the Special stat is split into Sp.Atk and Sp.Def, which is
    /// the single most load-bearing difference for anything reading stats.
    pub fn has_special_split(self) -> bool {
        matches!(self, Ruleset::Gen3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ruleset_says_which_engine_and_which_stat_model() {
        assert!(
            !Ruleset::default().has_special_split(),
            "Gen 1 stays the default"
        );
        assert!(Ruleset::Gen3.has_special_split());
        assert_eq!(Ruleset::Gen3.as_str(), "Gen 3");
    }
}
