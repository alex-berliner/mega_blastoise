//! Typed events for the physical board. Engine **log lines** and **input prompts** are converted
//! here; strings are only for human-readable descriptions ([`BoardEvent::description`]).

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use gen1_battle::Request;

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

/// One move slot carried in [`BoardEvent::SwitchIn`] and [`BoardEvent::MovesUpdate`].
///
/// Populated by the battle runner from live battle state; the OLED and move-detail
/// screen render directly from these fields without any further battle queries.
#[derive(Debug, Clone)]
pub struct MoveSlot {
    pub name: String,
    /// Display type name, e.g. `"Electric"`.
    pub type_name: String,
    /// `"Physical"`, `"Special"`, or `"Status"`.
    pub category: String,
    /// Base power; `None` for status moves (base_power == 0).
    pub power: Option<u32>,
    /// Accuracy 0–100; `None` for moves that always hit.
    pub accuracy: Option<u8>,
    pub pp: u8,
    pub max_pp: u8,
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
        /// Team slot index (0-based) populated by the battle runner; None if unavailable.
        team_slot: Option<u8>,
    },
    /// A move was announced in the log (`move` / `animatemove`).
    Move {
        /// The Pokémon that used the move (name extracted from `mon:name,player,pos`).
        user: Option<String>,
        /// The player that used the move (`"p1"` / `"p2"`), extracted from the mon field.
        player_id: Option<String>,
        name: String,
    },
    /// `switch` / `drag` / `appear` — lead or bench coming in (parsed from public log row).
    ///
    /// `moves` is empty when produced by [`parse_log_line`]; the battle runner enriches it
    /// with the full move list before dispatching.
    SwitchIn {
        /// Nickname / label (`name` field in the battler log).
        name: String,
        species: Option<String>,
        player_id: Option<String>,
        /// Team slot index (0-based) populated by the battle runner; None if unavailable.
        team_slot: Option<u8>,
        moves: Vec<MoveSlot>,
        /// Active mon's Speed stat, populated by the battle runner — drives
        /// the battle-screen sprite bob rate.
        speed: Option<u16>,
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
    SuperEffective { mon: String },
    Resisted { mon: String },
    Immune { mon: String },
    Miss { mon: String },
    CriticalHit { mon: String },
    /// Status condition inflicted (`status|mon:...|status:<name>`).
    SetStatus { mon: String, status: String },
    /// Status condition cured (`curestatus|mon:...|status:<name>`).
    CureStatus { mon: String, status: String },
    /// Can't move this turn (`cant|mon:...|from:<reason>`).
    Cant { mon: String, reason: String },
    /// Stat stage change (`boost|mon:...|stat:<name>|delta:<i8>`).
    StatChange { mon: String, stat: String, delta: i8 },
    /// A volatile effect began (`start|mon:...|what:<id>[|move:<name>]`) —
    /// seeded, confusion, substitute, wrap, disable, reflect, lightscreen,
    /// mist, focusenergy, conversion, transform, mimic, charging, bide, haze.
    EffectStart { mon: String, what: String, detail: Option<String> },
    /// A volatile effect ended (`end|mon:...|what:<id>`).
    EffectEnd { mon: String, what: String },
    /// The active mon's display identity changed in place (Transform):
    /// `activemon|mon:...|name:<new>|speed:<spe>`. Display-only, not narrated.
    ActiveMonUpdate { mon: String, name: String, speed: Option<u16> },
    Fail { mon: String },
    /// Active-mon moves refreshed — emitted after every move (PP change) and at switch-in.
    /// Internal signal; not narrated to USB/stdout.
    MovesUpdate {
        player_id: String,
        moves: Vec<MoveSlot>,
    },
    /// Any log line not matched by a specific variant — preserved so nothing is silently dropped.
    Raw(String),
}

/// Short trainer label for messages (`p1` → Red in the stock demo).
/// Convert `"p1"` / `"p2"` to 1 / 2.  Any other value returns 2.
pub fn player_id_to_num(player_id: &str) -> u8 {
    if player_id == "p1" { 1 } else { 2 }
}

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
pub fn mon_display_name(position_details: &str) -> &str {
    position_details.split(',').next().unwrap_or(position_details)
}

/// Extract the player id (`"p1"` or `"p2"`) from a battler `mon` position field.
pub fn mon_player_id(mon: &str) -> Option<&str> {
    let id = mon.split(',').nth(1)?.trim();
    if id == "p1" || id == "p2" { Some(id) } else { None }
}

/// Extract the player number (1 or 2) from a battler `mon` position field.
pub fn mon_player_num(mon: &str) -> Option<u8> {
    mon_player_id(mon).map(player_id_to_num)
}

/// Convert a battler status ID to its display abbreviation.
pub fn status_abbrev(s: &str) -> &str {
    match s {
        "par" => "PAR",
        "brn" => "BRN",
        "frz" => "FRZ",
        "psn" => "PSN",
        "tox" => "TOX",
        "slp" => "SLP",
        other => other,
    }
}

/// Build `"Red's Golduck"` from a `mon` position field.
fn player_mon_label(mon: &str) -> String {
    let name = mon_display_name(mon);
    let player = mon.split(',').nth(1).unwrap_or("");
    let trainer = player_display_name(player);
    format!("{trainer}'s {name}")
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
            team_slot: None,
        }),
        "move" | "animatemove" => {
            let mon_str = p.get("mon");
            Some(BoardEvent::Move {
                user: mon_str.map(|s| mon_display_name(s).into()),
                player_id: mon_str.and_then(|s| s.split(',').nth(1)).map(String::from),
                name: p.get("name").unwrap_or("?").into(),
            })
        }
        "switch" | "drag" | "appear" => Some(BoardEvent::SwitchIn {
            name: p.get("name").unwrap_or("?").into(),
            species: p.get("species").map(String::from),
            player_id: p.get("player").map(String::from),
            team_slot: None,
            moves: Vec::new(),
            speed: None,
        }),
        "switchout" => Some(BoardEvent::SwitchOut {
            name: p.get("mon").map(|s| mon_display_name(s).into()).unwrap_or_default(),
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
        "supereffective" => Some(BoardEvent::SuperEffective {
            mon: p.get("mon").unwrap_or("?").into(),
        }),
        "resisted" => Some(BoardEvent::Resisted {
            mon: p.get("mon").unwrap_or("?").into(),
        }),
        "immune" => Some(BoardEvent::Immune {
            mon: p.get("mon").unwrap_or("?").into(),
        }),
        "miss" => Some(BoardEvent::Miss {
            mon: p.get("mon").unwrap_or("?").into(),
        }),
        "crit" => Some(BoardEvent::CriticalHit {
            mon: p.get("mon").unwrap_or("?").into(),
        }),
        "status" => Some(BoardEvent::SetStatus {
            mon: p.get("mon").unwrap_or("?").into(),
            status: p.get("status").unwrap_or("?").into(),
        }),
        "curestatus" => Some(BoardEvent::CureStatus {
            mon: p.get("mon").unwrap_or("?").into(),
            status: p.get("status").unwrap_or("?").into(),
        }),
        "cant" => Some(BoardEvent::Cant {
            mon: p.get("mon").unwrap_or("?").into(),
            reason: p.get("from").unwrap_or("?").into(),
        }),
        "fail" => Some(BoardEvent::Fail {
            mon: p.get("mon").map(String::from).unwrap_or_default(),
        }),
        "boost" => Some(BoardEvent::StatChange {
            mon: p.get("mon").unwrap_or("?").into(),
            stat: p.get("stat").unwrap_or("stat").into(),
            delta: p.get("delta").and_then(|d| d.parse::<i8>().ok()).unwrap_or(0),
        }),
        "start" => Some(BoardEvent::EffectStart {
            mon: p.get("mon").unwrap_or("?").into(),
            what: p.get("what").unwrap_or("?").into(),
            detail: p.get("move").map(String::from),
        }),
        "end" => Some(BoardEvent::EffectEnd {
            mon: p.get("mon").unwrap_or("?").into(),
            what: p.get("what").unwrap_or("?").into(),
        }),
        "activemon" => Some(BoardEvent::ActiveMonUpdate {
            mon: p.get("mon").unwrap_or("?").into(),
            name: p.get("name").unwrap_or("?").into(),
            speed: p.get("speed").and_then(|v| v.parse::<u16>().ok()),
        }),
        // Pure engine bookkeeping — not gameplay events, not narrated.
        "residual" | "continue" | "info" | "side" | "player" | "teamsize" => None,
        _ => Some(BoardEvent::Raw(String::from(line))),
    }
}

impl BoardEvent {
    /// Convert `BoardEvent::Win { side }` to a 1-based player number (1 or 2),
    /// or 0 for a tie / unknown side.  Call at the `Win` arm; panics on other variants.
    pub fn win_player_num(side: &Option<String>) -> u8 {
        match side.as_deref() {
            Some("0") => 1,
            Some("1") => 2,
            _ => 0,
        }
    }

    /// Returns the two OLED win-screen messages `(p1_msg, p2_msg)` for a given winner.
    /// `winner` is 1-based (from [`win_player_num`](Self::win_player_num)); 0 means tie.
    pub fn win_messages(winner: u8) -> (&'static str, &'static str) {
        match winner {
            1 => ("WINNER!", "GG!"),
            2 => ("GG!", "WINNER!"),
            _ => ("TIE!", "TIE!"),
        }
    }

    /// Returns the canonical animation delay (ms) for this event, using the
    /// shared `anim::*` constants.  Returns 0 for events that need no delay.
    pub fn anim_delay_ms(&self) -> u32 {
        use crate::anim;
        match self {
            Self::Move { .. }                                  => anim::MOVE_MS,
            Self::Damage { .. } | Self::Heal { .. }           => anim::DAMAGE_MS,
            Self::SwitchIn { .. }                              => anim::SWITCH_IN_MS,
            Self::Faint { .. }                                 => anim::FAINT_MS,
            Self::Win { .. } | Self::Tie                      => anim::WIN_MS,
            Self::SuperEffective { .. }
            | Self::CriticalHit { .. }
            | Self::SetStatus { .. }
            | Self::CureStatus { .. }
            | Self::StatChange { .. }
            | Self::EffectStart { .. }
            | Self::EffectEnd { .. }                          => anim::EFFECT_MS,
            Self::Miss { .. }
            | Self::Immune { .. }
            | Self::Resisted { .. }
            | Self::Cant { .. }
            | Self::Fail { .. }                               => anim::BRIEF_MS,
            _ => 0,
        }
    }

    /// Returns `true` if this event should be forwarded to USB / log output.
    /// `Split`, `Prompt`, and `MovesUpdate` are internal engine signals that
    /// all targets suppress.
    pub fn should_narrate(&self) -> bool {
        !matches!(
            self,
            Self::Split { .. }
                | Self::Prompt { .. }
                | Self::MovesUpdate { .. }
                | Self::ActiveMonUpdate { .. }
        )
    }

    /// Human-readable battle narrative for display (USB, stdout). Hardware effects code should
    /// branch on the `BoardEvent` variant directly rather than parsing this string.
    pub fn description(&self) -> String {
        match self {
            BoardEvent::Split { side } => {
                format!("[split side:{}]", side_display_name(side.as_str()))
            }
            BoardEvent::Damage { mon, health } => {
                format!("{} took damage!  (HP: {health})", player_mon_label(mon))
            }
            BoardEvent::Heal { mon, health } => {
                format!("{} recovered HP!  (HP: {health})", player_mon_label(mon))
            }
            BoardEvent::Faint { mon, .. } => {
                format!("{} fainted!", player_mon_label(mon))
            }
            BoardEvent::Move { user, name, player_id, .. } => match user.as_deref() {
                Some(u) => {
                    let trainer = player_id.as_deref().map(player_display_name).unwrap_or("");
                    if trainer.is_empty() { format!("{u} used {name}!") }
                    else { format!("{trainer}'s {u} used {name}!") }
                }
                None => format!("Used {name}!"),
            },
            BoardEvent::SwitchIn {
                name,
                species,
                player_id,
                ..
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
            BoardEvent::SuperEffective { mon } => {
                format!("It's super effective on {}!", player_mon_label(mon))
            }
            BoardEvent::Resisted { mon } => {
                format!("It's not very effective on {}...", player_mon_label(mon))
            }
            BoardEvent::Immune { mon } => {
                format!("{} is unaffected!", player_mon_label(mon))
            }
            BoardEvent::Miss { mon } => {
                format!("The attack missed {}!", player_mon_label(mon))
            }
            BoardEvent::CriticalHit { mon } => {
                format!("A critical hit on {}!", player_mon_label(mon))
            }
            BoardEvent::SetStatus { mon, status } => {
                let label = player_mon_label(mon);
                match status.as_str() {
                    "brn" => format!("{label} was burned!"),
                    "psn" => format!("{label} was poisoned!"),
                    "tox" => format!("{label} was badly poisoned!"),
                    "par" => format!("{label} was paralyzed!"),
                    "slp" => format!("{label} fell asleep!"),
                    "frz" => format!("{label} was frozen solid!"),
                    other => format!("{label} was inflicted with {}!", status_abbrev(other)),
                }
            }
            BoardEvent::CureStatus { mon, status } => {
                format!("{}'s {} was cured!", player_mon_label(mon), status_abbrev(status))
            }
            BoardEvent::Cant { mon, reason } => {
                let label = player_mon_label(mon);
                match reason.as_str() {
                    "confusion" => format!("{label} hurt itself in its confusion!"),
                    "disabled" => format!("{label}'s move is disabled!"),
                    _ => format!("{label} can't move! ({reason})"),
                }
            }
            BoardEvent::StatChange { mon, stat, delta } => {
                let label = player_mon_label(mon);
                let dir = match delta {
                    d if *d >= 2 => "rose sharply",
                    d if *d == 1 => "rose",
                    d if *d == -1 => "fell",
                    _ => "fell harshly",
                };
                format!("{label}'s {stat} {dir}!")
            }
            BoardEvent::EffectStart { mon, what, detail } => {
                let label = player_mon_label(mon);
                let mv = detail.as_deref().unwrap_or("?");
                match what.as_str() {
                    "seeded" => format!("{label} was seeded!"),
                    "confusion" => format!("{label} became confused!"),
                    "substitute" => format!("{label} put up a substitute!"),
                    "wrap" => format!("{label} was wrapped!"),
                    "disable" => format!("{label}'s {mv} was disabled!"),
                    "reflect" => format!("{label} put up Reflect!"),
                    "lightscreen" => format!("{label} put up Light Screen!"),
                    "mist" => format!("{label} became shrouded in mist!"),
                    "focusenergy" => format!("{label} is getting pumped!"),
                    "conversion" => format!("{label} converted its type!"),
                    "transform" => {
                        if detail.is_some() {
                            format!("{label} transformed into {mv}!")
                        } else {
                            format!("{label} transformed!")
                        }
                    }
                    "mimic" => format!("{label} mimicked {mv}!"),
                    "charging" => format!("{label} is charging {mv}!"),
                    "bide" => format!("{label} is storing energy!"),
                    "haze" => "All stat changes were eliminated!".into(),
                    other => format!("{label}: {other}"),
                }
            }
            BoardEvent::EffectEnd { mon, what } => {
                let label = player_mon_label(mon);
                match what.as_str() {
                    "substitute" => format!("{label}'s substitute broke!"),
                    "disable" => format!("{label} is no longer disabled!"),
                    "bide" => format!("{label} unleashed its energy!"),
                    other => format!("{label}'s {other} wore off!"),
                }
            }
            BoardEvent::Fail { mon } => {
                if mon.is_empty() {
                    "The move failed!".into()
                } else {
                    format!("But it failed for {}!", player_mon_label(mon))
                }
            }
            BoardEvent::ActiveMonUpdate { .. } => String::new(), // display-only
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
            BoardEvent::MovesUpdate { .. } => String::new(),
        }
    }
}

impl fmt::Display for BoardEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.description())
    }
}
