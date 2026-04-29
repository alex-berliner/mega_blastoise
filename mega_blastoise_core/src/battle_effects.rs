//! Presentation hooks: typed [`crate::board_event::BoardEvent`] only — parse logs at the edge,
//! queue, then dispatch.

extern crate alloc;

use alloc::collections::VecDeque;

use crate::board_event::{parse_log_line, BoardEvent};

/// Reacts to [`BoardEvent`] (sound, LEDs, prompts). Same trait on host and firmware.
pub trait BoardEffects {
    fn on_event(&mut self, event: BoardEvent);
}

/// Default no-op sink (stub hardware).
#[derive(Debug, Clone, Copy, Default)]
pub struct NoopBoardEffects;

impl BoardEffects for NoopBoardEffects {
    fn on_event(&mut self, _event: BoardEvent) {}
}

/// Single FIFO queue for board effects (log-derived + injected prompts / scripted tests).
#[derive(Debug, Default)]
pub struct BoardEventQueue {
    inner: VecDeque<BoardEvent>,
}

impl BoardEventQueue {
    pub fn new() -> Self {
        Self {
            inner: VecDeque::new(),
        }
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
            if let Some(e) = parse_log_line(line) {
                self.inner.push_back(e);
            }
        }
    }

    pub fn dispatch_all(&mut self, sink: &mut impl BoardEffects) {
        while let Some(e) = self.inner.pop_front() {
            sink.on_event(e);
        }
    }

    /// True if nothing pending.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

/// Enqueue parsed log events and dispatch them in order.
pub fn process_new_log_lines<'a, I>(
    lines: I,
    queue: &mut BoardEventQueue,
    sink: &mut impl BoardEffects,
) where
    I: IntoIterator<Item = &'a str>,
{
    queue.push_log_lines(lines);
    queue.dispatch_all(sink);
}
