//! Typed events for the physical board. Engine **log lines** and **input prompts** are converted
//! here; strings are only for human-readable descriptions ([`BoardEventExt::description`]).

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

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

// The board vocabulary has one definition, in `ps_client` (the core sits
// above it and the PS client yields these directly). What the UI does with
// an event, its narration and pacing, stays here as [`BoardEventExt`].
pub use ps_client::board::{BoardEvent, MoveSlot, PromptKind};

/// Short trainer label for messages (`p1` → Red in the stock demo).
/// Convert `"p1"` / `"p2"` to 1 / 2.  Any other value returns 2.
pub fn player_id_to_num(player_id: &str) -> u8 {
    if player_id == "p1" { 1 } else { 2 }
}

pub fn player_display_name(player_id: &str) -> &'static str {
    match player_id {
        // Trainer names match the players' LED identity colors:
        // P1 glows white, P2 glows red.
        "p1" => "White",
        "p2" => "Red",
        _ => "?",
    }
}

/// Friendly label for battler **side index** in the stock 1v1 demo (`0` / `1`).
pub fn side_display_name(side: &str) -> &'static str {
    match side {
        "0" => "White",
        "1" => "Red",
        _ => "?",
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

/// The trainer of a `mon` position field (`"Red"`), for team-wide lines.
fn mon_trainer(mon: &str) -> &'static str {
    player_display_name(mon.split(',').nth(1).unwrap_or(""))
}

/// PS `toID`: lowercase letters and digits only (`"Focus Band"` is
/// `focusband`), how the PS client looks an effect's text up.
fn effect_id(name: &str) -> String {
    name.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_lowercase()).collect()
}

/// The words of an [`BoardEvent::Ability`], [`BoardEvent::ItemGained`],
/// [`BoardEvent::ItemEnded`] or [`BoardEvent::Activate`]: Showdown's
/// client text (`data/text/{default,moves,abilities,items}.ts`, as its
/// `BattleTextParser` picks the template), shortened where it would not
/// fit the 48-byte narration box.
fn effect_line(event: &BoardEvent) -> String {
    let name_of = |m: &Option<String>| -> String { m.as_deref().map(mon_display_name).unwrap_or("its foe").into() };
    match event {
        BoardEvent::Ability { mon, ability, how, other } => {
            let label = player_mon_label(mon);
            let other = name_of(other);
            match (how.as_str(), effect_id(ability).as_str()) {
                ("trace", _) => format!("{label} traced {other}'s {ability}!"),
                ("roleplay", _) => format!("{label} copied {other}'s {ability}!"),
                ("", "pressure") => format!("{label} is exerting its pressure!"),
                // PS's `abilityActivation`: "[NAME's ABILITY]".
                _ => format!("[{label}'s {ability}]"),
            }
        }
        BoardEvent::ItemGained { mon, item, how, .. } => {
            let label = player_mon_label(mon);
            match how.as_str() {
                "thief" | "covet" => format!("{label} stole {item}!"),
                "recycle" => format!("{label} found one {item}!"),
                _ => format!("{label} obtained {item}."),
            }
        }
        BoardEvent::ItemEnded { mon, item, how, .. } => {
            let label = player_mon_label(mon);
            match how.as_str() {
                "eat" => format!("{label} ate its {item}!"),
                "knockoff" => format!("Knocked off {label}'s {item}!"),
                "" => format!("{label} used its {item}!"),
                _ => format!("{label} lost its {item}!"),
            }
        }
        BoardEvent::Activate { mon, what, detail, other, n } => {
            let label = player_mon_label(mon);
            let other = name_of(other);
            let mv = detail.as_deref().unwrap_or("its move");
            let n = n.unwrap_or(0);
            match effect_id(what).as_str() {
                "confusion" => format!("{label} is confused!"),
                "struggle" => format!("{label} has no moves left!"),
                "substitute" => format!("{label}'s substitute took damage!"),
                "protect" | "detect" => format!("{label} protected itself!"),
                "endure" => format!("{label} endured the hit!"),
                "focusband" => format!("{label} hung on using its Focus Band!"),
                "quickclaw" => format!("{label}'s Quick Claw let it act first!"),
                "leppaberry" => format!("{label} restored PP to {mv}!"),
                "spite" => format!("{label}'s {mv} lost {n} PP!"),
                "grudge" => format!("{label}'s {mv} lost all its PP!"),
                "attract" => format!("{label} is in love with {other}!"),
                "trick" => format!("{label} switched items!"),
                "lockon" | "mindreader" => format!("{label} took aim at {other}!"),
                "skillswap" => format!("{label} swapped Abilities!"),
                "snatch" => format!("{label} snatched {other}'s move!"),
                "destinybond" => format!("{label} took its attacker down with it!"),
                "pursuit" => format!("({label} is being withdrawn...)"),
                "beatup" => format!("{other}'s attack!"),
                "bide" => format!("{label} is storing energy!"),
                "mist" => format!("{label} is protected by the mist!"),
                "uproar" => format!("{label} is making an uproar!"),
                "brickbreak" => format!("{label} shattered the barriers!"),
                "healbell" => "A bell chimed!".into(),
                "aromatherapy" => "A soothing aroma wafted through the area!".into(),
                "perishsong" => "All who heard the song will faint in 3 turns!".into(),
                "payday" => "Coins were scattered everywhere!".into(),
                "hitcount" if n == 1 => "Hit 1 time!".into(),
                "hitcount" => format!("Hit {n} times!"),
                "ohko" => "It's a one-hit KO!".into(),
                "nothing" => "But nothing happened!".into(),
                "unboost" => match detail {
                    Some(stat) => format!("{label}'s {stat} was not lowered!"),
                    None => format!("{label}'s stats were not lowered!"),
                },
                // PS's default `activate`: "(EFFECT activated!)".
                _ => format!("({what} activated!)"),
            }
        }
        _ => String::new(),
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

/// What the UI does with a [`BoardEvent`]: its narration, animation pacing
/// and result-screen words. A trait because the type lives in `ps_client`;
/// `use mega_blastoise_core::BoardEventExt` (or the crate root re-export)
/// brings these into scope.
pub trait BoardEventExt {
    fn win_player_num(side: &Option<String>) -> u8;
    fn win_messages(winner: u8) -> (&'static str, &'static str);
    fn anim_delay_ms(&self) -> u32;
    fn should_narrate(&self) -> bool;
    fn description(&self) -> String;
}

impl BoardEventExt for BoardEvent {
    /// Convert `BoardEvent::Win { side }` to a 1-based player number (1 or 2),
    /// or 0 for a tie / unknown side.  Call at the `Win` arm; panics on other variants.
    fn win_player_num(side: &Option<String>) -> u8 {
        match side.as_deref() {
            Some("0") => 1,
            Some("1") => 2,
            _ => 0,
        }
    }

    /// Returns the two OLED win-screen messages `(p1_msg, p2_msg)` for a given winner.
    /// `winner` is 1-based (from [`win_player_num`](BoardEventExt::win_player_num)); 0 means tie.
    fn win_messages(winner: u8) -> (&'static str, &'static str) {
        match winner {
            1 => ("WINNER!", "GG!"),
            2 => ("GG!", "WINNER!"),
            _ => ("TIE!", "TIE!"),
        }
    }

    /// Returns the canonical animation delay (ms) for this event, using the
    /// shared `anim::*` constants.  Returns 0 for events that need no delay.
    fn anim_delay_ms(&self) -> u32 {
        use crate::anim;
        match self {
            Self::Move { .. }                                  => anim::MOVE_MS,
            Self::Damage { .. } | Self::Heal { .. }           => anim::DAMAGE_MS,
            Self::SwitchIn { .. }                              => anim::SWITCH_IN_MS,
            Self::Faint { .. }                                 => anim::FAINT_MS,
            Self::Win { .. } | Self::Tie                      => anim::WIN_MS,
            Self::SetStatus { .. }                             => anim::STATUS_MS,
            Self::SuperEffective { .. }
            | Self::CriticalHit { .. }
            | Self::CureStatus { .. }
            | Self::StatChange { .. }
            | Self::EffectStart { .. }
            | Self::EffectEnd { .. }
            | Self::Ability { .. }
            | Self::ItemGained { .. }
            | Self::ItemEnded { .. }
            | Self::Activate { .. }                           => anim::EFFECT_MS,
            Self::Cant { .. }                                  => anim::CANT_MS,
            Self::Miss { .. }
            | Self::Immune { .. }
            | Self::Resisted { .. }
            | Self::Fail { .. }                               => anim::BRIEF_MS,
            _ => 0,
        }
    }

    /// Returns `true` if this event should be forwarded to USB / log output.
    /// `Split`, `Prompt`, and `MovesUpdate` are internal engine signals that
    /// all targets suppress.
    fn should_narrate(&self) -> bool {
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
    fn description(&self) -> String {
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
                    // `user` is a position field, so the seat id has to come
                    // off before it is read out — every sibling arm goes
                    // through `player_mon_label` for exactly this reason, and
                    // this one printed "Blaziken,p1 used Ember!".
                    let u = mon_display_name(u);
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
                    "par" => format!("{label} is fully paralyzed!"),
                    "frz" => format!("{label} is frozen solid!"),
                    "slp" => format!("{label} is sound asleep!"),
                    "trapped" => format!("{label} is trapped!"),
                    "flinch" => format!("{label} flinched!"),
                    "recharge" => format!("{label} must recharge!"),
                    "love" => format!("{label} is immobilized by love!"),
                    "truant" => format!("{label} is loafing around!"),
                    "focuspunch" => format!("{label} lost its focus!"),
                    "imprison" => format!("{label} can't use its sealed move!"),
                    "taunt" => format!("{label} can't act after the taunt!"),
                    "nopp" => format!("{label} has no PP left!"),
                    _ => format!("{label} can't move!"),
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
                    "haze" | "Haze" => "All stat changes were eliminated!".into(),
                    // The Gen 3 runner's words and PS's effect names, with
                    // Showdown's client text (`data/text/moves.ts` `start`).
                    "Substitute" => format!("{label} put up a substitute!"),
                    "Leech Seed" => format!("{label} was seeded!"),
                    "Focus Energy" => format!("{label} is getting pumped!"),
                    "Reflect" => format!("{label} put up Reflect!"),
                    "Light Screen" => format!("{label} put up Light Screen!"),
                    "Mist" => format!("{label} became shrouded in mist!"),
                    "Safeguard" => format!("{}'s team is cloaked in a veil!", mon_trainer(mon)),
                    "Spikes" => format!("Spikes were scattered around {}'s team!", mon_trainer(mon)),
                    "bind" => format!("{label} was wrapped!"),
                    "charge" => format!("{label} is charging up!"),
                    "drowsy" => format!("{label} grew drowsy!"),
                    "trapped" => format!("{label} can no longer escape!"),
                    "Protect" => format!("{label} protected itself!"),
                    "Endure" => format!("{label} braced itself!"),
                    "Focus Punch" => format!("{label} is tightening its focus!"),
                    "Magic Coat" => format!("{label} put up a Magic Coat!"),
                    "Snatch" => format!("{label} waits for a move to snatch!"),
                    "Destiny Bond" => format!("{label} wants to take its foe down!"),
                    "Grudge" => format!("{label} bears a grudge!"),
                    "Encore" => format!("{label} must do an encore!"),
                    "Taunt" => format!("{label} fell for the taunt!"),
                    "Torment" => format!("{label} was subjected to torment!"),
                    "Uproar" => format!("{label} caused an uproar!"),
                    "Charge" => format!("{label} began charging power!"),
                    "Foresight" | "Odor Sleuth" => format!("{label} was identified!"),
                    "Imprison" => format!("{label} sealed the foe's moves!"),
                    "Curse" => format!("{label} was put under a curse!"),
                    "Future Sight" => format!("{label} foresaw an attack!"),
                    "Doom Desire" => format!("{label} chose Doom Desire!"),
                    "Attract" => format!("{label} fell in love!"),
                    "Ingrain" => format!("{label} planted its roots!"),
                    "Nightmare" => format!("{label} began having a nightmare!"),
                    "Flash Fire" => format!("{label}'s Fire moves grew stronger!"),
                    "rain" => "It started to rain!".into(),
                    "harsh sunlight" => "The sunlight turned harsh!".into(),
                    "sandstorm" => "A sandstorm kicked up!".into(),
                    "hail" => "It started to hail!".into(),
                    other => {
                        if let Some(n) = other.strip_prefix("perish") {
                            format!("{label}'s perish count fell to {n}!")
                        } else if let Some(n) = other.strip_prefix("stockpile") {
                            format!("{label} stockpiled {n}!")
                        } else {
                            format!("{label}: {other}")
                        }
                    }
                }
            }
            BoardEvent::EffectEnd { mon, what } => {
                let label = player_mon_label(mon);
                let trainer = mon_trainer(mon);
                match what.as_str() {
                    "substitute" | "Substitute" => format!("{label}'s substitute broke!"),
                    "disable" | "Disable" => format!("{label} is no longer disabled!"),
                    "bide" => format!("{label} unleashed its energy!"),
                    // Showdown's client text (`data/text/moves.ts` `end`).
                    "confusion" => format!("{label} snapped out of confusion!"),
                    "seeded" | "Leech Seed" => format!("{label} was freed from Leech Seed!"),
                    "wrap" | "bind" => format!("{label} was freed from the trap!"),
                    "Encore" => format!("{label}'s encore ended!"),
                    "Taunt" => format!("{label} shook off the taunt!"),
                    "Torment" => format!("{label} is no longer tormented!"),
                    "Uproar" => format!("{label} calmed down."),
                    "Attract" => format!("{label} got over its infatuation!"),
                    "Future Sight" => format!("Future Sight struck {label}!"),
                    "Doom Desire" => format!("Doom Desire struck {label}!"),
                    "stockpile" => format!("{label}'s stockpile wore off!"),
                    "Spikes" => format!("The spikes around {trainer}'s team are gone!"),
                    "reflect" | "Reflect" => format!("{trainer}'s Reflect wore off!"),
                    "lightscreen" | "Light Screen" => format!("{trainer}'s Light Screen wore off!"),
                    "mist" | "Mist" => format!("{trainer}'s mist faded!"),
                    "Safeguard" => format!("{trainer}'s Safeguard wore off!"),
                    "rain" => "The rain stopped.".into(),
                    "harsh sunlight" => "The harsh sunlight faded.".into(),
                    "sandstorm" => "The sandstorm subsided.".into(),
                    "hail" => "The hail stopped.".into(),
                    other => format!("{label}'s {other} wore off!"),
                }
            }
            BoardEvent::Ability { .. }
            | BoardEvent::ItemGained { .. }
            | BoardEvent::ItemEnded { .. }
            | BoardEvent::Activate { .. } => effect_line(self),
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


#[cfg(test)]
mod tests {
    //! The words the board says for the lines the PS client added (UX-2):
    //! Showdown's client text, as `description` gives it and both seats'
    //! narration box shows it.

    use super::*;
    use alloc::string::ToString;
    use alloc::vec;
    use alloc::vec::Vec;

    fn s(x: &str) -> String {
        x.to_string()
    }

    fn act(mon: &str, what: &str, detail: Option<&str>, other: Option<&str>, n: Option<i16>) -> BoardEvent {
        BoardEvent::Activate { mon: s(mon), what: s(what), detail: detail.map(s), other: other.map(s), n }
    }

    #[test]
    fn item_ability_and_activate_lines_read_as_showdown_says_them() {
        let gold = "Golduck,p2,0";
        let gard = "Gardevoir,p1,0";
        let cases: Vec<(BoardEvent, &str)> = vec![
            (BoardEvent::Ability { mon: s(gold), ability: s("Intimidate"), how: s(""), other: None }, "[Red's Golduck's Intimidate]"),
            (BoardEvent::Ability { mon: s(gold), ability: s("Pressure"), how: s(""), other: None }, "Red's Golduck is exerting its pressure!"),
            (BoardEvent::Ability { mon: s(gold), ability: s("Water Absorb"), how: s("activate"), other: None }, "[Red's Golduck's Water Absorb]"),
            (BoardEvent::Ability { mon: s(gard), ability: s("Levitate"), how: s("trace"), other: Some(s(gold)) }, "White's Gardevoir traced Golduck's Levitate!"),
            (BoardEvent::Ability { mon: s(gard), ability: s("Cloud Nine"), how: s("roleplay"), other: Some(s(gold)) }, "White's Gardevoir copied Golduck's Cloud Nine!"),
            (BoardEvent::ItemGained { mon: s(gard), item: s("Choice Band"), how: s("trick"), other: None }, "White's Gardevoir obtained Choice Band."),
            (BoardEvent::ItemGained { mon: s(gard), item: s("Leftovers"), how: s("thief"), other: Some(s(gold)) }, "White's Gardevoir stole Leftovers!"),
            (BoardEvent::ItemGained { mon: s(gard), item: s("Leppa Berry"), how: s("recycle"), other: None }, "White's Gardevoir found one Leppa Berry!"),
            (BoardEvent::ItemEnded { mon: s(gard), item: s("Salac Berry"), how: s("eat"), other: None }, "White's Gardevoir ate its Salac Berry!"),
            (BoardEvent::ItemEnded { mon: s(gard), item: s("Leftovers"), how: s("knockoff"), other: Some(s(gold)) }, "Knocked off White's Gardevoir's Leftovers!"),
            (BoardEvent::ItemEnded { mon: s(gard), item: s("White Herb"), how: s(""), other: None }, "White's Gardevoir used its White Herb!"),
            (act(gard, "confusion", None, None, None), "White's Gardevoir is confused!"),
            (act(gold, "Protect", None, None, None), "Red's Golduck protected itself!"),
            (act(gold, "Substitute", None, None, None), "Red's Golduck's substitute took damage!"),
            (act(gold, "Focus Band", None, None, None), "Red's Golduck hung on using its Focus Band!"),
            (act(gold, "Spite", Some("Psychic"), None, Some(4)), "Red's Golduck's Psychic lost 4 PP!"),
            (act(gold, "Leppa Berry", Some("Psychic"), None, None), "Red's Golduck restored PP to Psychic!"),
            (act(gard, "Attract", None, Some(gold), None), "White's Gardevoir is in love with Golduck!"),
            (act(gard, "Lock-On", None, Some(gold), None), "White's Gardevoir took aim at Golduck!"),
            (act(gard, "Beat Up", None, Some("Snorlax,p1,0"), None), "Snorlax's attack!"),
            (act("", "Perish Song", None, None, None), "All who heard the song will faint in 3 turns!"),
            (act("", "Heal Bell", None, None, None), "A bell chimed!"),
            (act(gold, "hitcount", None, None, Some(3)), "Hit 3 times!"),
            (act(gold, "Uproar", Some("upkeep"), None, None), "Red's Golduck is making an uproar!"),
            (act(gold, "unboost", Some("Attack"), None, None), "Red's Golduck's Attack was not lowered!"),
            (act(gold, "Mystery", None, None, None), "(Mystery activated!)"),
        ];
        for (event, want) in cases {
            assert_eq!(event.description(), want, "{event:?}");
            assert!(want.len() <= 48, "{want:?} does not fit the narration box");
            assert!(event.should_narrate() && event.anim_delay_ms() > 0, "{event:?}");
        }
    }

    /// Both engines' effect words read as sentences, not `"label: word"`;
    /// the weather words have no mon, and read without one.
    #[test]
    fn shared_effect_words_read_as_sentences() {
        let start = |mon: &str, what: &str| BoardEvent::EffectStart { mon: s(mon), what: s(what), detail: None };
        let end = |mon: &str, what: &str| BoardEvent::EffectEnd { mon: s(mon), what: s(what) };
        let cases: Vec<(BoardEvent, &str)> = vec![
            (start("", "rain"), "It started to rain!"),
            (end("", "harsh sunlight"), "The harsh sunlight faded."),
            (start("Golduck,p2,0", "Spikes"), "Spikes were scattered around Red's team!"),
            (start("Golduck,p2,0", "drowsy"), "Red's Golduck grew drowsy!"),
            (start("Golduck,p2,0", "perish2"), "Red's Golduck's perish count fell to 2!"),
            (start("Golduck,p2,0", "Encore"), "Red's Golduck must do an encore!"),
            (start("Golduck,p2,0", "Protect"), "Red's Golduck protected itself!"),
            (end("Golduck,p2,0", "confusion"), "Red's Golduck snapped out of confusion!"),
            (end("Golduck,p2,0", "Leech Seed"), "Red's Golduck was freed from Leech Seed!"),
            (
                BoardEvent::Cant { mon: s("Slaking,p1,0"), reason: s("truant") },
                "White's Slaking is loafing around!",
            ),
        ];
        for (event, want) in cases {
            assert_eq!(event.description(), want, "{event:?}");
        }
    }
}
