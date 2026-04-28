//! Physical outputs (RGB / buzzer / PWM audio) — wire [`BattleEffects`] here as drivers appear.

use defmt::info;
use mega_blastoise_core::{BattleEffects, ParsedBattleLogLine};

/// Logs each line over RTT (same as before). Extend `match` arms for buzzer / NeoPixel / I2S.
pub struct DefmtBattleEffects;

impl DefmtBattleEffects {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefmtBattleEffects {
    fn default() -> Self {
        Self::new()
    }
}

impl BattleEffects for DefmtBattleEffects {
    fn on_log_line(&mut self, line: &str) {
        info!("{}", line);

        let p = ParsedBattleLogLine::parse(line);
        match p.title() {
            "damage" => {
                let _health = p.get("health");
                let _mon = p.get("mon");
                // Future: map `health` (public % / fraction string) → LED color; one-shot SFX.
            }
            "faint" => {}
            "move" | "animatemove" => {}
            _ => {}
        }
    }
}
