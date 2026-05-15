#![no_std]

//! Gen 1 cartridge-accurate Pokémon battle engine.
//!
//! Designed as a drop-in replacement for the `battler` crate's API surface used
//! by `mega_blastoise_core`. Memory target: ~800 B of RAM per 6v6 battle, all
//! data tables (species/moves/type chart) flash-resident.
//!
//! See `GEN1_SPEC.md` at the workspace root for mechanics details.

extern crate alloc;

mod battle;
mod combat;
mod data;
mod dispatch;
mod log;
mod options;
mod request;
mod rng;
mod state;
mod tables;
mod types;

/// Internal types exposed for unit tests. Not part of the stable API surface
/// (everything in here can change shape without a SemVer bump).
#[doc(hidden)]
pub mod testing {
    pub use crate::combat::*;
    pub use crate::dispatch::*;
    pub use crate::rng::Rng;
    pub use crate::state::*;
}

pub use battle::{Battle, PublicCoreBattle};
pub use data::{
    BoostTable, MonBattleData, MonData, MonSummary, MoveSlot, PlayerBattleData, PlayerData,
    PlayerDex, PlayerOptions, PlayerType, SideData, TeamData,
};
pub use options::{
    BattleType, CoreBattleEngineOptions, CoreBattleOptions, FormatData, SerializedRuleSet,
};
pub use request::{LearnMoveRequest, MonTurnRequest, Request, SwitchRequest, TeamPreviewRequest, TurnRequest};
pub use tables::{
    move_by_id, species_by_id, type_effectiveness, MoveCategory, MoveEffectKind, MoveEntry,
    SpeciesEntry, MOVES, SPECIES, TYPE_CHART, TYPE_COUNT,
};
pub use types::{Stat, Type};
