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
use embassy_futures::join::join;
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
    /// Total number of prompts in this batch (e.g. 2 when both players need to choose
    /// simultaneously). Input sources use this to wait for all prompts with `receive().await`
    /// instead of `try_receive()`, eliminating the race where the second prompt hasn't been
    /// sent yet when the first is processed.
    pub batch_total: usize,
}

/// Shared bus between the battle runner and all input sources.
///
/// Create one per battle (stack-allocated, no heap).  Pass `&bus` to every input
/// source's `run(&bus)` call, then compose those futures and hand them to
/// [`run_battle`](crate::run_battle).
pub struct InputBus {
    /// Choice strings produced by input sources, consumed by the runner.
    pub choices: Channel<NoopRawMutex, String, 4>,
    /// Sent by the runner before it blocks on choices.  Capacity-2 lets the runner
    /// queue both players' prompts simultaneously so ButtonController can fire
    /// on_prompt for everyone before any wait_action blocks.
    pub prompt: Channel<NoopRawMutex, ActivePrompt, 2>,
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

    /// Called after the player makes a provisional choice.
    /// Override to show a "waiting / press to unready" screen.  Default is a no-op.
    fn on_choice_pending(&mut self, _player_id: &str) {}

    /// Wait until the cancel window expires (returns `false` = committed) or the
    /// player presses any button (returns `true` = cancelled).  Default always
    /// proceeds immediately — override to add an undo window.
    async fn wait_cancel_window(&mut self, _player_id: &str) -> bool { false }

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

impl<BS: ButtonSource> ButtonController<BS> {
    /// Collect the player's choice for a given prompt (move/switch/etc.).
    async fn collect_choice(&mut self, prompt: &ActivePrompt) -> String {
        let ActivePrompt { player_id, request, .. } = prompt;
        match request {
            Request::Turn(turn) => {
                let mut parts = Vec::new();
                for mon_req in &turn.active {
                    let n = mon_req.moves.len().min(4);
                    if n == 0 {
                        parts.push(String::from("pass"));
                        continue;
                    }
                    if mon_req.locked_into_move {
                        parts.push(format_move_choice(0));
                        continue;
                    }
                    loop {
                        match self.source.wait_action(player_id, n).await {
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
                    let idx = self.source.wait_switch(player_id).await;
                    parts.push(format_switch_choice(idx));
                }
                join_choice_parts(&parts)
            }
            Request::TeamPreview(_) => String::from("random"),
            Request::LearnMove(_) => String::from("pass"),
        }
    }

    /// Collect a choice with undo support: shows the waiting screen and allows
    /// the player to cancel within the cancel window.
    async fn collect_choice_with_unready(&mut self, prompt: &ActivePrompt) -> String {
        loop {
            let choice = self.collect_choice(prompt).await;
            self.source.on_choice_pending(&prompt.player_id);
            if self.source.wait_cancel_window(&prompt.player_id).await {
                self.source.on_prompt(&prompt.player_id, &prompt.request, &prompt.player_data);
            } else {
                return choice;
            }
        }
    }
}

impl<BS: ButtonSource + Clone> ButtonController<BS> {
    /// Like `run`, but collects all players' choices in parallel when `batch_total > 1`.
    /// Requires `BS: Clone` so each player gets an independent source instance.
    /// Choices are always sent to `bus.choices` in prompt order (p1 before p2).
    pub async fn run_parallel(&mut self, bus: &InputBus) {
        loop {
            let first_prompt = loop {
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

            self.source.on_prompt(
                &first_prompt.player_id,
                &first_prompt.request,
                &first_prompt.player_data,
            );

            let extra_count = first_prompt.batch_total.saturating_sub(1);
            let mut extra_prompts: Vec<ActivePrompt> = Vec::with_capacity(extra_count);
            for _ in 0..extra_count {
                let p = bus.prompt.receive().await;
                self.source.on_prompt(&p.player_id, &p.request, &p.player_data);
                extra_prompts.push(p);
            }

            if extra_prompts.len() == 1 {
                // Two-player batch: collect both choices simultaneously so neither player
                // blocks on the other's pick or cancel window.
                let extra = extra_prompts.remove(0);
                let mut source2 = self.source.clone();
                let mut ctrl2 = ButtonController { source: source2, log_sink: self.log_sink };
                let (c1, c2) = join(
                    self.collect_choice_with_unready(&first_prompt),
                    ctrl2.collect_choice_with_unready(&extra),
                ).await;
                // Send in prompt order so battle_runner applies them to the right players.
                bus.choices.send(c1).await;
                bus.choices.send(c2).await;
            } else {
                // Single player (or unsupported batch size) — serial.
                let choice = self.collect_choice_with_unready(&first_prompt).await;
                bus.choices.send(choice).await;
                for extra in &extra_prompts {
                    let choice = self.collect_choice_with_unready(extra).await;
                    bus.choices.send(choice).await;
                }
            }

            while let Ok(line) = bus.log.try_receive() {
                (self.log_sink)(&line);
            }
        }
    }
}

impl<BS: ButtonSource> InputSource for ButtonController<BS> {
    async fn run(&mut self, bus: &InputBus) {
        loop {
            // Drain bus.log while waiting for the next prompt.
            let first_prompt = loop {
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

            // Fire on_prompt for the first player immediately.
            self.source.on_prompt(
                &first_prompt.player_id,
                &first_prompt.request,
                &first_prompt.player_data,
            );

            // Receive all remaining prompts in this batch using blocking receive().
            // Using try_receive() here would race: the battle_runner sends prompts
            // in a loop with dispatch_all().await between sends, so the second
            // prompt may not be in the channel yet when the first is processed.
            // batch_total tells us exactly how many to expect.
            let extra_count = first_prompt.batch_total.saturating_sub(1);
            let mut extra_prompts: Vec<ActivePrompt> = Vec::with_capacity(extra_count);
            for _ in 0..extra_count {
                let p = bus.prompt.receive().await;
                self.source.on_prompt(&p.player_id, &p.request, &p.player_data);
                extra_prompts.push(p);
            }

            // Collect and submit choices in prompt order.
            let choice = self.collect_choice_with_unready(&first_prompt).await;
            bus.choices.send(choice).await;

            for extra in &extra_prompts {
                let choice = self.collect_choice_with_unready(extra).await;
                bus.choices.send(choice).await;
            }

            while let Ok(line) = bus.log.try_receive() {
                (self.log_sink)(&line);
            }
        }
    }
}
