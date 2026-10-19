//! Battle initialization options.

extern crate alloc;

use crate::data::SideData;

#[derive(Clone, Debug, Default)]
pub enum BattleType {
    #[default]
    Singles,
    Doubles,
}

#[derive(Clone, Debug, Default)]
pub struct SerializedRuleSet;

impl SerializedRuleSet {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Clone, Debug, Default)]
pub struct FormatData {
    pub battle_type: BattleType,
    pub rules: SerializedRuleSet,
}

#[derive(Clone, Debug, Default)]
pub struct CoreBattleOptions {
    pub seed: Option<u64>,
    pub format: FormatData,
    pub field: (),
    pub side_1: SideData,
    pub side_2: SideData,
}

#[derive(Clone, Debug, Default)]
pub struct CoreBattleEngineOptions {
    pub validate_teams: bool,
    pub auto_continue: bool,
    pub reveal_actual_health: bool,
    pub log_time: bool,
}
