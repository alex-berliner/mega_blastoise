//! Abstract interface for collecting battler choice strings (`set_player_choice` format).
//!
//! Move slots are **0-based** in the protocol (`move 0` … `move 3`).  
//! Switch targets are **team positions** (`switch 0` … `switch 5` for six party slots).

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use battler::Request;

/// Collects [`Request`] responses as battler choice strings (e.g. `"move 2"`, `"switch 4"`).
pub trait BattleInput {
    /// Returns the full choice line for this player (may contain `;`-separated sub-choices).
    async fn read_choice(&mut self, player_id: &str, request: &Request) -> String;
}

/// One move slot — `slot` is 0-based (`move 0` = first move).
pub fn format_move_choice(slot: usize) -> String {
    alloc::format!("move {slot}")
}

/// Switch to a party slot — `team_index` is 0-based (`switch 0` = lead / first bench slot per engine).
pub fn format_switch_choice(team_index: usize) -> String {
    alloc::format!("switch {team_index}")
}

/// Combine active positions / multiple commands (doubles, forced switches).
pub fn join_choice_parts(parts: &[String]) -> String {
    parts.join(";")
}

/// Build a turn choice from one move slot per active [`MonMoveRequest`](battler::MonMoveRequest) line.
pub fn turn_choice_from_move_slots(slots: &[usize]) -> String {
    let parts: Vec<String> = slots.iter().map(|s| format_move_choice(*s)).collect();
    join_choice_parts(&parts)
}

/// Build a forced-switch choice string (`needs_switch` length).
pub fn switch_choice_from_team_indices(indices: &[usize]) -> String {
    let parts: Vec<String> = indices.iter().map(|i| format_switch_choice(*i)).collect();
    join_choice_parts(&parts)
}
