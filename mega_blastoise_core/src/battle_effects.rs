//! Presentation hooks: typed [`crate::board_event::BoardEvent`] only — parse logs at the edge,
//! queue, then dispatch.

extern crate alloc;

use alloc::collections::VecDeque;

use crate::board_event::{parse_log_line, BoardEvent, ParsedBattleLogLine};

/// Shared animation delay durations (ms). Identical on all targets for presentation parity.
pub mod anim {
    // Uniform 2500ms for every battle screen except the win screen.
    pub const MOVE_MS:      u32 = 2500;
    pub const DAMAGE_MS:    u32 = 2500;
    pub const SWITCH_IN_MS: u32 = 2500;
    pub const FAINT_MS:     u32 = 2500;
    pub const WIN_MS:       u32 = 7500;
    pub const EFFECT_MS:    u32 = 2500; // super-effective, crit, stat change
    pub const BRIEF_MS:     u32 = 2500; // miss, immune, resist, fail
    pub const CANT_MS:      u32 = 2500; // can't move (slp/par/frz/trapped/…)
    pub const STATUS_MS:    u32 = 2500; // was paralyzed / burned / poisoned / …
}

/// Reacts to [`BoardEvent`] (sound, LEDs, prompts). Same trait on host and firmware.
#[allow(async_fn_in_trait)]
pub trait BoardEffects {
    async fn on_event(&mut self, event: BoardEvent);
}

/// Default no-op sink (stub hardware).
#[derive(Debug, Clone, Copy, Default)]
pub struct NoopBoardEffects;

impl BoardEffects for NoopBoardEffects {
    async fn on_event(&mut self, _event: BoardEvent) {}
}

/// Single FIFO queue for board effects (log-derived + injected prompts / scripted tests).
///
/// The battler often emits `split|side:N`, then a **private** log row, then a **public** row with
/// the same title (`switch`, `damage`, …). Each `split` becomes a [`BoardEvent::Split`]; the
/// private row is still skipped so each moment yields one gameplay cue from the public row.
#[derive(Debug)]
pub struct BoardEventQueue {
    inner: VecDeque<BoardEvent>,
    pending_skip_private_after_split: bool,
}

impl Default for BoardEventQueue {
    fn default() -> Self {
        Self {
            inner: VecDeque::new(),
            pending_skip_private_after_split: false,
        }
    }
}

impl BoardEventQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_event(&mut self, event: BoardEvent) {
        self.inner.push_back(event);
    }

    /// Parse engine log lines and enqueue recognized events (unknown titles are skipped).
    pub fn push_log_lines<'a, I>(&mut self, lines: I)
    where
        I: IntoIterator<Item = &'a str>,
    {
        for line in lines {
            let p = ParsedBattleLogLine::parse(line);
            if p.title() == "split" {
                if let Some(e) = parse_log_line(line) {
                    self.inner.push_back(e);
                }
                self.pending_skip_private_after_split = true;
                continue;
            }
            if self.pending_skip_private_after_split {
                self.pending_skip_private_after_split = false;
                continue;
            }
            if let Some(e) = parse_log_line(line) {
                self.inner.push_back(e);
            }
        }
    }

    pub async fn dispatch_all(&mut self, sink: &mut impl BoardEffects) {
        while let Some(e) = self.inner.pop_front() {
            sink.on_event(e).await;
        }
    }

    /// True if nothing pending.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Drain all pending events into a `Vec`, leaving the queue empty.
    /// Used by the battle runner to post-process events before dispatch.
    pub fn drain_pending(&mut self) -> alloc::vec::Vec<BoardEvent> {
        self.inner.drain(..).collect()
    }
}

/// Enqueue parsed log events and dispatch them in order.
pub async fn process_new_log_lines<'a, I>(
    lines: I,
    queue: &mut BoardEventQueue,
    sink: &mut impl BoardEffects,
) where
    I: IntoIterator<Item = &'a str>,
{
    queue.push_log_lines(lines);
    queue.dispatch_all(sink).await;
}
