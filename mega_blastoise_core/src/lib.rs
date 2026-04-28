#![no_std]

extern crate alloc;

pub mod battle_effects;
pub mod battle_input;
pub mod data_store;
pub use battle_effects::{
    for_each_new_log_line, NoopBattleEffects, ParsedBattleLogLine, BattleEffects,
};
pub use battle_input::{
    format_move_choice, format_switch_choice, join_choice_parts, switch_choice_from_team_indices,
    turn_choice_from_move_slots, BattleInput,
};
pub use data_store::FlashDataStore;
