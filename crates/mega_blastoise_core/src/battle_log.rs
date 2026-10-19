//! Scrollable battle log — the teaching surface behind the `?` button.
//!
//! Every narration line is kept so a player can look back at what actually
//! happened, with the numbers, rather than trying to read a 2.5 second flash.
//! Bounded so a long match cannot grow without limit on the firmware.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

/// Lines retained. Older lines fall off the front.
pub const LOG_CAPACITY: usize = 50;

/// Rows one 240x160 half can show at once.
pub const LOG_ROWS: usize = 11;

/// A bounded, append-only record of a battle's narration.
#[derive(Default)]
pub struct BattleLog {
    lines: Vec<String>,
}

impl BattleLog {
    pub const fn new() -> Self {
        Self { lines: Vec::new() }
    }

    /// Append a line, dropping the oldest once at capacity. Blank lines are
    /// ignored so the log does not fill with spacing from the event stream.
    pub fn push(&mut self, line: &str) {
        let line = line.trim();
        if line.is_empty() {
            return;
        }
        if self.lines.len() == LOG_CAPACITY {
            self.lines.remove(0);
        }
        self.lines.push(String::from(line));
    }

    pub fn clear(&mut self) {
        self.lines.clear();
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// Largest valid scroll offset, so the last page is exactly full.
    pub fn max_scroll(&self) -> usize {
        self.lines.len().saturating_sub(LOG_ROWS)
    }

    /// Clamp a requested offset into range.
    pub fn clamp_scroll(&self, offset: usize) -> usize {
        offset.min(self.max_scroll())
    }

    /// Offset that shows the newest lines — where the log should open.
    pub fn bottom(&self) -> usize {
        self.max_scroll()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_oldest_at_capacity() {
        let mut log = BattleLog::new();
        for i in 0..(LOG_CAPACITY + 5) {
            let mut s = String::from("line ");
            s.push((b'0' + (i % 10) as u8) as char);
            log.push(&s);
        }
        assert_eq!(log.len(), LOG_CAPACITY);
    }

    #[test]
    fn blank_lines_are_not_recorded() {
        let mut log = BattleLog::new();
        log.push("   ");
        log.push("");
        assert!(log.is_empty());
    }

    #[test]
    fn opens_on_the_newest_page() {
        let mut log = BattleLog::new();
        for _ in 0..20 {
            log.push("hit");
        }
        assert_eq!(log.bottom(), 20 - LOG_ROWS);
        assert_eq!(log.clamp_scroll(999), log.bottom());
    }

    #[test]
    fn a_short_log_never_scrolls() {
        let mut log = BattleLog::new();
        log.push("Turn 1");
        assert_eq!(log.bottom(), 0);
        assert_eq!(log.clamp_scroll(5), 0);
    }
}
