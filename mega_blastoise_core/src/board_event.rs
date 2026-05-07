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
        /// The Pokémon that used the move (name extracted from `mon:name,player,pos`).
        user: Option<String>,
        name: String,
    },
    /// `switch` / `drag` / `appear` — lead or bench coming in (parsed from public log row).
    SwitchIn {
        /// Nickname / label (`name` field in the battler log).
        name: String,
        species: Option<String>,
        player_id: Option<String>,
    },
    SwitchOut {
        /// Pokémon that left the field (name extracted from `mon:name,player,pos`).
        name: String,
    },
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
    /// Any log line not matched by a specific variant — preserved so nothing is silently dropped.
    Raw(String),
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

/// Extract the display name from a battler `mon` position field (`"name,player_id,pos"`).
/// Returns the whole string unchanged if no comma is present (e.g. synthetic test values).
fn extract_mon_name(position_details: &str) -> &str {
    position_details.split(',').next().unwrap_or(position_details)
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
            user: p.get("mon").map(|s| extract_mon_name(s).into()),
            name: p.get("name").unwrap_or("?").into(),
        }),
        "switch" | "drag" | "appear" => Some(BoardEvent::SwitchIn {
            name: p.get("name").unwrap_or("?").into(),
            species: p.get("species").map(String::from),
            player_id: p.get("player").map(String::from),
        }),
        "switchout" => Some(BoardEvent::SwitchOut {
            name: p.get("mon").map(|s| extract_mon_name(s).into()).unwrap_or_default(),
        }),
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
        _ => Some(BoardEvent::Raw(String::from(line))),
    }
}

impl BoardEvent {
    /// Human-readable battle narrative for display (USB, stdout). Hardware effects code should
    /// branch on the `BoardEvent` variant directly rather than parsing this string.
    pub fn description(&self) -> String {
        match self {
            BoardEvent::Split { side } => {
                format!("[split side:{}]", side_display_name(side.as_str()))
            }
            BoardEvent::Damage { mon, health } => {
                let name = extract_mon_name(mon);
                format!("{name} took damage!  (HP: {health})")
            }
            BoardEvent::Heal { mon, health } => {
                let name = extract_mon_name(mon);
                format!("{name} recovered HP!  (HP: {health})")
            }
            BoardEvent::Faint { mon } => {
                let name = extract_mon_name(mon);
                format!("{name} fainted!")
            }
            BoardEvent::Move { user, name } => match user.as_deref() {
                Some(u) => format!("{u} used {name}!"),
                None => format!("Used {name}!"),
            },
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
                format!("{trainer} sent out {mon_label}!")
            }
            BoardEvent::SwitchOut { name } => {
                format!("{name} was recalled!")
            }
            BoardEvent::Turn { n } => format!("--- Turn {n} ---"),
            BoardEvent::BattleStart => "=== Battle start! ===".into(),
            BoardEvent::Win { side } => match side {
                Some(s) => format!("=== {} wins! ===", side_display_name(s.as_str())),
                None => "=== Battle over! ===".into(),
            },
            BoardEvent::Tie => "=== Draw! ===".into(),
            BoardEvent::Raw(line) => format!("[event] {line}"),
            BoardEvent::Prompt { player_id, kind } => {
                let label = player_display_name(player_id.as_str());
                match kind {
                    PromptKind::ChooseMove => format!("{label}: choosing a move..."),
                    PromptKind::ChooseSwitch => format!("{label}: must switch!"),
                    PromptKind::TeamPreview => format!("{label}: team preview"),
                    PromptKind::LearnMove => format!("{label}: learn move"),
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
