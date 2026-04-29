//! Typed events for the physical board. Engine **log lines** and **input prompts** are converted
//! here; strings are only for human-readable descriptions ([`BoardEvent::description`]).

use alloc::format;
use alloc::string::String;
use core::fmt;

use battler::Request;

/// Borrowed view of `title|key:value|…` committed log lines from the engine.
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

/// Why the engine is waiting on a player (maps to lights / whose turn on the board).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptKind {
    ChooseMove,
    ChooseSwitch,
    TeamPreview,
    LearnMove,
}

/// Something the board should represent (sound, LEDs, prompts).
#[derive(Debug, Clone)]
pub enum BoardEvent {
    /// Engine `split|side:N` — marks whose private/public log pair follows (see battler
    /// `log_private_public`).
    Split {
        side: String,
    },
    Damage {
        mon: String,
        health: String,
    },
    Heal {
        mon: String,
        health: String,
    },
    Faint {
        mon: String,
    },
    /// A move was announced in the log (`move` / `animatemove`).
    Move {
        name: String,
    },
    /// `switch` / `drag` / `appear` — lead or bench coming in (parsed from public log row).
    SwitchIn {
        /// Nickname / label (`name` field in the battler log).
        name: String,
        species: Option<String>,
        player_id: Option<String>,
    },
    SwitchOut,
    Turn {
        n: u32,
    },
    BattleStart,
    Win {
        side: Option<String>,
    },
    Tie,
    /// Waiting for input — board should cue this player before blocking on buttons/NFC/stdin.
    Prompt {
        player_id: String,
        kind: PromptKind,
    },
}

/// Short trainer label for messages (`p1` → Red in the stock demo).
pub fn player_display_name(player_id: &str) -> &'static str {
    match player_id {
        "p1" => "Red",
        "p2" => "Blue",
        _ => "?",
    }
}

/// Friendly label for battler **side index** in the stock 1v1 demo (`0` / `1`).
pub fn side_display_name(side: &str) -> &'static str {
    match side {
        "0" => "Red",
        "1" => "Blue",
        _ => "?",
    }
}

/// Build a prompt event from an engine [`Request`] (emit **before** collecting input).
pub fn board_prompt_event(player_id: &str, request: &Request) -> BoardEvent {
    let kind = match request {
        Request::Turn(_) => PromptKind::ChooseMove,
        Request::Switch(_) => PromptKind::ChooseSwitch,
        Request::TeamPreview(_) => PromptKind::TeamPreview,
        Request::LearnMove(_) => PromptKind::LearnMove,
    };
    BoardEvent::Prompt {
        player_id: String::from(player_id),
        kind,
    }
}

/// Parse one committed log line into a typed event, if recognized.
pub fn parse_log_line(line: &str) -> Option<BoardEvent> {
    let p = ParsedBattleLogLine::parse(line);
    match p.title() {
        "damage" => Some(BoardEvent::Damage {
            mon: p.get("mon").unwrap_or("?").into(),
            health: p.get("health").unwrap_or("?").into(),
        }),
        "heal" => Some(BoardEvent::Heal {
            mon: p.get("mon").unwrap_or("?").into(),
            health: p.get("health").unwrap_or("?").into(),
        }),
        "faint" => Some(BoardEvent::Faint {
            mon: p.get("mon").unwrap_or("?").into(),
        }),
        "move" | "animatemove" => Some(BoardEvent::Move {
            name: p.get("name").unwrap_or("?").into(),
        }),
        "switch" | "drag" | "appear" => Some(BoardEvent::SwitchIn {
            name: p.get("name").unwrap_or("?").into(),
            species: p.get("species").map(String::from),
            player_id: p.get("player").map(String::from),
        }),
        "switchout" => Some(BoardEvent::SwitchOut),
        "turn" => {
            let n = p
                .get("turn")
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(1);
            Some(BoardEvent::Turn { n })
        }
        "battlestart" => Some(BoardEvent::BattleStart),
        "split" => Some(BoardEvent::Split {
            side: p.get("side").unwrap_or("?").into(),
        }),
        "win" => Some(BoardEvent::Win {
            side: p.get("side").map(String::from),
        }),
        "tie" => Some(BoardEvent::Tie),
        _ => None,
    }
}

impl BoardEvent {
    /// Plain-language line for hosts (println / defmt). Hardware code should branch on `BoardEvent`,
    /// not on this string.
    pub fn description(&self) -> String {
        match self {
            BoardEvent::Split { side } => {
                let who = side_display_name(side.as_str());
                format!(
                    "Split for side {side} ({who}) — engine sends an owner-only row next, then the public row; board uses the public row as the next cue"
                )
            }
            BoardEvent::Damage { mon, health } => {
                format!("{mon}: took damage → hit noise, HP light shows {health}")
            }
            BoardEvent::Heal { mon, health } => {
                format!("{mon}: healed → soft blip, HP light shows {health}")
            }
            BoardEvent::Faint { mon } => {
                format!("{mon}: fainted → KO sound, that Pokémon’s lights off")
            }
            BoardEvent::Move { name } => format!(
                "uses {name} → quick move sound + flash that player’s strip"
            ),
            BoardEvent::SwitchIn {
                name,
                species,
                player_id,
            } => {
                let mon_label = match species {
                    Some(sp) if !sp.is_empty() && sp.as_str() != name.as_str() => {
                        format!("{name} ({sp})")
                    }
                    _ => name.clone(),
                };
                let trainer = player_id
                    .as_deref()
                    .map(player_display_name)
                    .unwrap_or("Trainer");
                format!("{trainer} sends in {mon_label} → switch sound + light that slot")
            }
            BoardEvent::SwitchOut => "Pokémon out → dim its lights".into(),
            BoardEvent::Turn { n } => {
                format!("Turn {n} → optional blink on the turn marker")
            }
            BoardEvent::BattleStart => {
                "Fight starts → short beep / lights at full HP".into()
            }
            BoardEvent::Win { side } => match side {
                Some(s) => format!("Match over — side {s} wins → win sound + that side lights up"),
                None => "Someone won → win sound + winner side lights up".into(),
            },
            BoardEvent::Tie => "Draw → short neutral tone".into(),
            BoardEvent::Prompt { player_id, kind } => {
                let label = player_display_name(player_id.as_str());
                match kind {
                    PromptKind::ChooseMove => format!(
                        "{label}: pick a move — light that player’s move buttons / cue"
                    ),
                    PromptKind::ChooseSwitch => format!(
                        "{label}: must switch — light bench / switch controls"
                    ),
                    PromptKind::TeamPreview => format!("{label}: team preview prompt (demo uses random)"),
                    PromptKind::LearnMove => format!("{label}: learn-move prompt (demo passes)"),
                }
            }
        }
    }
}

impl fmt::Display for BoardEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.description())
    }
}
