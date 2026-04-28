//! Stand-in for sound + LEDs on the real board. Here we print short plain-English notes.

use mega_blastoise_core::{BattleEffects, ParsedBattleLogLine};

#[derive(Debug, Clone)]
pub struct BoardGameEffects {
    /// Also print the raw engine line (debug).
    pub echo_raw: bool,
}

impl Default for BoardGameEffects {
    fn default() -> Self {
        Self { echo_raw: false }
    }
}

impl BoardGameEffects {
    pub fn new() -> Self {
        Self::default()
    }
}

impl BattleEffects for BoardGameEffects {
    fn on_log_line(&mut self, line: &str) {
        let p = ParsedBattleLogLine::parse(line);

        let msg = match p.title() {
            "damage" => {
                let who = p.get("mon").unwrap_or("?");
                let hp = p.get("health").unwrap_or("?");
                Some(format!("{who}: took damage → hit noise, HP light shows {hp}"))
            }
            "heal" => {
                let who = p.get("mon").unwrap_or("?");
                let hp = p.get("health").unwrap_or("?");
                Some(format!("{who}: healed → soft blip, HP light shows {hp}"))
            }
            "faint" => {
                let who = p.get("mon").unwrap_or("?");
                Some(format!("{who}: fainted → KO sound, that Pokémon’s lights off"))
            }
            "move" | "animatemove" => {
                let name = p.get("name").unwrap_or("?");
                Some(format!("uses {name} → quick move sound + flash that player’s strip"))
            }
            "switch" | "drag" | "appear" => Some("new Pokémon in → switch sound + light that bench slot".into()),
            "switchout" => Some("Pokémon out → dim its lights".into()),
            "turn" => Some(format!(
                "Turn {} → optional blink on the turn marker",
                p.get("turn").unwrap_or("?")
            )),
            "battlestart" => Some("Fight starts → short beep / lights at full HP".into()),
            "win" => Some("Someone won → win sound + winner side lights up".into()),
            "tie" => Some("Draw → short neutral tone".into()),
            _ => None,
        };

        if let Some(m) = msg {
            println!("{m}");
        }

        if self.echo_raw {
            println!("  engine: {line}");
        }
    }
}
