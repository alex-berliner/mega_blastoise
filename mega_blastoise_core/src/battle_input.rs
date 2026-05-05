//! Unified input channel for the battle engine.
//!
//! [`InputBus`] is the single point through which all choice strings reach
//! [`run_battle`](crate::run_battle).  Any number of input sources — USB serial,
//! physical buttons, NFC readers — can send to `choices` concurrently; all race and
//! the first answer wins.  The runner sends [`ActivePrompt`] on a capacity-1 channel
//! before blocking on `choices`, so the display source (e.g. USB) can show rich
//! prompts (move names, PP, …).  Sources that don't need prompt details (buttons,
//! NFC) can ignore `bus.prompt` and post choices directly.
//!
//! To run multiple sources together, compose their futures before passing to
//! [`run_battle`](crate::run_battle):
//! ```ignore
//! run_battle(&mut battle, &bus,
//!     embassy_futures::join::join(usb.run(&bus), buttons.run(&bus)),
//!     ...).await;
//! ```

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use battler::{PlayerBattleData, Request};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::{Channel, Sender};

/// The engine request the runner is currently waiting on.
#[derive(Clone)]
pub struct ActivePrompt {
    pub player_id: String,
    pub request: Request,
    /// Full battle state for the requesting player — used by display sources to show HP, moves,
    /// and bench. `None` only when the battle engine can't supply data (shouldn't happen).
    pub player_data: Option<PlayerBattleData>,
}

/// Shared bus between the battle runner and all input sources.
///
/// Create one per battle (stack-allocated, no heap).  Pass `&bus` to every input
/// source's `run(&bus)` call, then compose those futures and hand them to
/// [`run_battle`](crate::run_battle).
pub struct InputBus {
    /// Choice strings produced by input sources, consumed by the runner.
    pub choices: Channel<NoopRawMutex, String, 4>,
    /// Sent by the runner before it blocks; consumed once by the active display source.
    /// Capacity-1: the runner cannot send a second prompt until the first is taken.
    pub prompt: Channel<NoopRawMutex, ActivePrompt, 1>,
    /// Battle event descriptions pushed by BoardEffects; drained by output sinks (e.g. USB).
    /// Capacity 32 handles a full turn's worth of events (move, damage, faint, switch-in, etc.)
    /// without dropping any before USB drains them.
    pub log: Channel<NoopRawMutex, String, 32>,
}

impl InputBus {
    pub const fn new() -> Self {
        Self {
            choices: Channel::new(),
            prompt: Channel::new(),
            log: Channel::new(),
        }
    }

    pub fn sender(&self) -> Sender<'_, NoopRawMutex, String, 4> {
        self.choices.sender()
    }
}

/// Anything that can produce choices for the battle runner.
///
/// Implement this on your input driver (USB, buttons, …) and pass it to [`run_battle`](crate::run_battle).
/// The runner joins the battle loop with `input.run(bus)` so both progress cooperatively.
pub trait InputSource {
    async fn run(&mut self, bus: &InputBus);
}

/// Placeholder input source that never produces choices (pends forever).
///
/// Use this when no interactive input is needed — e.g. when running without USB.
pub struct NoInput;

impl InputSource for NoInput {
    async fn run(&mut self, _bus: &InputBus) {
        // core::future::pending().await
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
