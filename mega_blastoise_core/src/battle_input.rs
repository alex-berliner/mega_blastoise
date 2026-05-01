//! Unified input channel for the battle engine.
//!
//! [`InputBus`] is the single point through which all choice strings reach
//! [`run_battle`](crate::run_battle).  Any number of input sources — USB serial,
//! physical buttons, NFC readers — obtain a sender and run concurrently (e.g. via
//! `embassy_futures::join`).  The runner signals [`ActivePrompt`] before blocking on
//! the channel so that "smart" sources can display rich prompts (move names, PP, …).

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use battler::Request;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::{Channel, Sender};
use embassy_sync::signal::Signal;

/// The engine request the runner is currently waiting on.
#[derive(Clone)]
pub struct ActivePrompt {
    pub player_id: String,
    pub request: Request,
}

/// Shared bus between the battle runner and all input sources.
///
/// Create one per battle (stack-allocated, no heap), split off senders for each
/// input source, then run them alongside [`run_battle`](crate::run_battle) with
/// `embassy_futures::join`.
pub struct InputBus {
    /// Choice strings produced by input sources, consumed by the runner.
    pub choices: Channel<NoopRawMutex, String, 4>,
    /// Set by the runner before it blocks; sources subscribe to show the right prompt.
    pub prompt: Signal<NoopRawMutex, ActivePrompt>,
    /// Battle event descriptions pushed by BoardEffects; drained by output sinks (e.g. USB).
    pub log: Channel<NoopRawMutex, String, 8>,
}

impl InputBus {
    pub const fn new() -> Self {
        Self {
            choices: Channel::new(),
            prompt: Signal::new(),
            log: Channel::new(),
        }
    }

    pub fn sender(&self) -> Sender<'_, NoopRawMutex, String, 4> {
        self.choices.sender()
    }
}

// ── choice string helpers (used by all input sources) ────────────────────────

/// `move 0` … `move 3` (0-based slot).
pub fn format_move_choice(slot: usize) -> String {
    alloc::format!("move {slot}")
}

/// `switch 0` … `switch 5` (0-based team index).
pub fn format_switch_choice(team_index: usize) -> String {
    alloc::format!("switch {team_index}")
}

/// Join multiple sub-choices with `;` (doubles / multi-switch).
pub fn join_choice_parts(parts: &[String]) -> String {
    parts.join(";")
}

pub fn turn_choice_from_move_slots(slots: &[usize]) -> String {
    let parts: Vec<String> = slots.iter().map(|s| format_move_choice(*s)).collect();
    join_choice_parts(&parts)
}

pub fn switch_choice_from_team_indices(indices: &[usize]) -> String {
    let parts: Vec<String> = indices.iter().map(|i| format_switch_choice(*i)).collect();
    join_choice_parts(&parts)
}
