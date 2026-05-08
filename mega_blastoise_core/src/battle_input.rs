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
use embassy_futures::select::{select, Either};
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

// ── Button-event input interface ─────────────────────────────────────────────

/// Result of [`ButtonSource::wait_action`] — the player pressed a move button or a party button.
pub enum PlayerAction {
    /// Move button pressed; value is the 0-based slot index.
    Move(usize),
    /// Party button pressed; value is the 0-based team index.
    Switch(usize),
}

/// A source of raw button-press events — one per physical (or simulated) button.
///
/// Implementors only need to know *which* button was pressed; all battle-protocol
/// logic lives in [`ButtonController`].  Both the firmware GPIO matrix and the USB
/// serial mock implement this trait so they share an identical input pipeline.
pub trait ButtonSource {
    /// Called once when the engine sends a new prompt, before waiting for input.
    /// Override to show a display (terminal menu, OLED update, …).  Default is a no-op.
    fn on_prompt(
        &mut self,
        _player_id: &str,
        _request: &Request,
        _player_data: &Option<PlayerBattleData>,
    ) {
    }

    /// Wait for the player to press either a move button or a party button.
    /// Used during `Request::Turn` where either is valid (unless trapped).
    async fn wait_action(&mut self, player_id: &str, n_moves: usize) -> PlayerAction;

    /// Wait for the player to press a party button only (forced switch after faint).
    /// Returns a 0-based team index.
    async fn wait_switch(&mut self, player_id: &str) -> usize;
}

/// Drives the battle engine's choice loop using a [`ButtonSource`].
///
/// Reads [`ActivePrompt`]s from `bus.prompt`, calls the source for each choice, validates
/// (disabled/no-PP, trapped) and retries silently, then sends the final choice string to
/// `bus.choices`.  Log events are forwarded to `log_sink` while waiting for prompts.
pub struct ButtonController<BS: ButtonSource> {
    pub source: BS,
    log_sink: fn(&str),
}

impl<BS: ButtonSource> ButtonController<BS> {
    pub fn new(source: BS) -> Self {
        Self { source, log_sink: |_| {} }
    }

    pub fn with_log_sink(source: BS, log_sink: fn(&str)) -> Self {
        Self { source, log_sink }
    }
}

impl<BS: ButtonSource> InputSource for ButtonController<BS> {
    async fn run(&mut self, bus: &InputBus) {
        loop {
            // Drain bus.log while waiting for the next prompt.
            let prompt = loop {
                match select(bus.prompt.receive(), bus.log.receive()).await {
                    Either::First(p) => {
                        while let Ok(line) = bus.log.try_receive() {
                            (self.log_sink)(&line);
                        }
                        break p;
                    }
                    Either::Second(line) => (self.log_sink)(&line),
                }
            };

            let ActivePrompt { player_id, request, player_data } = prompt;
            self.source.on_prompt(&player_id, &request, &player_data);

            let choice = match &request {
                Request::Turn(turn) => {
                    let mut parts = Vec::new();
                    for mon_req in &turn.active {
                        let n = mon_req.moves.len().min(4);
                        if n == 0 {
                            parts.push(String::from("pass"));
                            continue;
                        }
                        loop {
                            match self.source.wait_action(&player_id, n).await {
                                PlayerAction::Move(slot) if slot < n => {
                                    let mv = &mon_req.moves[slot];
                                    if !mv.disabled && mv.pp > 0 {
                                        parts.push(format_move_choice(slot));
                                        break;
                                    }
                                }
                                PlayerAction::Switch(idx) if !mon_req.trapped => {
                                    parts.push(format_switch_choice(idx));
                                    break;
                                }
                                _ => {}
                            }
                        }
                    }
                    join_choice_parts(&parts)
                }
                Request::Switch(sw) => {
                    let mut parts = Vec::new();
                    for _ in &sw.needs_switch {
                        let idx = self.source.wait_switch(&player_id).await;
                        parts.push(format_switch_choice(idx));
                    }
                    join_choice_parts(&parts)
                }
                Request::TeamPreview(_) => String::from("random"),
                Request::LearnMove(_) => String::from("pass"),
            };

            bus.choices.send(choice).await;

            while let Ok(line) = bus.log.try_receive() {
                (self.log_sink)(&line);
            }
        }
    }
}
