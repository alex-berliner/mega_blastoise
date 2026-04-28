//! Hooks for **physical board presentation**: LEDs, sound, displays — separate from battle rules.
//!
//! The battler engine exposes human-readable log lines (`PublicCoreBattle::new_log_entries`).
//! Implement [`BattleEffects::on_log_line`] to react on-device; use [`ParsedBattleLogLine`] to
//! branch on stable `title|key:value|…` fields (e.g. `damage` + `health` for RGB health rings).

/// Receives each **new** committed log line since the last drain — same feed you already print.
pub trait BattleEffects {
    fn on_log_line(&mut self, line: &str);
}

/// Default for firmware that does not yet drive WS2812 / DAC (implements [`BattleEffects`] as no-ops).
#[derive(Debug, Clone, Copy, Default)]
pub struct NoopBattleEffects;

impl BattleEffects for NoopBattleEffects {
    fn on_log_line(&mut self, _line: &str) {}
}

/// Calls [`BattleEffects::on_log_line`] once per iterator item (typically `battle.new_log_entries()`).
pub fn for_each_new_log_line<'a, I>(lines: I, effects: &mut impl BattleEffects)
where
    I: IntoIterator<Item = &'a str>,
{
    for line in lines {
        effects.on_log_line(line);
    }
}

/// Borrowed view of `title|key:value|key:value|…` log lines from the engine.
#[derive(Debug, Clone, Copy)]
pub struct ParsedBattleLogLine<'a> {
    title: &'a str,
    rest: &'a str,
}

impl<'a> ParsedBattleLogLine<'a> {
    pub fn parse(line: &'a str) -> Self {
        let mut parts = line.splitn(2, '|');
        let title = parts.next().unwrap_or("");
        let rest = parts.next().unwrap_or("");
        Self { title, rest }
    }

    pub fn title(&self) -> &'a str {
        self.title
    }

    pub fn get(&self, key: &str) -> Option<&'a str> {
        if self.rest.is_empty() {
            return None;
        }
        for segment in self.rest.split('|') {
            if let Some((k, v)) = segment.split_once(':') {
                if k == key {
                    return Some(v);
                }
            }
        }
        None
    }
}
