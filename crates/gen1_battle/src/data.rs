//! Team / Mon / Player data types.
//!
//! These mirror the structural shape of the corresponding battler types so
//! that existing callers (`prompt_fmt.rs`, `display.rs`, `random_ai.rs`) work
//! without modification.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use hashbrown::HashMap;

use crate::types::{Stat, Type};

#[derive(Clone, Debug, Default)]
pub struct MoveSlot {
    pub name: String,
    pub id: String,
    pub typ: String,
    pub pp: u8,
    pub max_pp: u8,
    pub disabled: bool,
    pub target: u8, // placeholder; refined when move targeting is wired in
}

/// Stat-stage boost record. Atk/Def/Spe used by Gen 1; spa is "Special";
/// spd/acc/eva exist for API compat (always 0 in Gen 1).
#[derive(Clone, Copy, Debug, Default)]
pub struct BoostTable {
    pub atk: i8,
    pub def: i8,
    pub spa: i8,
    pub spd: i8,
    pub spe: i8,
    pub acc: i8,
    pub eva: i8,
}

#[derive(Clone, Debug, Default)]
pub struct MonSummary {
    pub name: String,
    pub species: String,
    pub level: u8,
}

/// Input team-member descriptor (what the caller hands in via `update_team`).
#[derive(Clone, Debug, Default)]
pub struct MonData {
    pub name: String,
    pub species: String,
    pub level: u8,
    pub moves: Vec<MoveSlot>,
    pub ivs: BoostTable,    // reusing the layout; only 4 fields meaningful
    pub evs: BoostTable,    // same
    pub gender: Option<String>,
    pub nature: Option<String>,
    pub ability: Option<String>,
    pub item: Option<String>,
}

/// Per-mon battle snapshot returned via `player_data()`. Wraps the live state
/// in fields the display + prompt code expects to see.
#[derive(Clone, Debug, Default)]
pub struct MonBattleData {
    pub active: bool,
    pub player_team_position: u8,
    pub hp: u16,
    pub max_hp: u16,
    pub status: Option<String>,
    pub species: String,
    pub ability: Option<String>,
    pub types: Vec<Type>,
    pub item: Option<String>,
    pub summary: MonSummary,
    pub stats: HashMap<Stat, u16>,
    pub boosts: BoostTable,
    pub moves: Vec<MoveSlot>,
}

#[derive(Clone, Debug, Default)]
pub struct PlayerBattleData {
    pub id: String,
    pub name: String,
    pub mons: Vec<MonBattleData>,
}

#[derive(Clone, Debug, Default)]
pub struct TeamData {
    pub members: Vec<MonData>,
}

#[derive(Clone, Debug, Default)]
pub struct PlayerOptions;

#[derive(Clone, Debug, Default)]
pub struct PlayerDex;

#[derive(Clone, Debug, Default)]
pub enum PlayerType {
    #[default]
    Trainer,
}

#[derive(Clone, Debug, Default)]
pub struct PlayerData {
    pub id: String,
    pub name: String,
    pub player_type: PlayerType,
    pub player_options: PlayerOptions,
    pub team: TeamData,
    pub dex: PlayerDex,
}

#[derive(Clone, Debug, Default)]
pub struct SideData {
    pub name: String,
    pub players: Vec<PlayerData>,
}
