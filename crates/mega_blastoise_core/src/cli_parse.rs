extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use ps_protocol::{Name, PackedSet, PackedTeam};

// ── USB battle prompt parsing ─────────────────────────────────────────────────

/// Result of parsing a USB CLI turn-prompt line.
#[derive(Debug, PartialEq)]
pub enum TurnChoice {
    Move(usize),   // 0-based move slot
    Switch(usize), // 0-based party slot
}

/// Parse a USB CLI turn-prompt line. `n` is the number of available moves.
/// Accepts:
///   "1"–"N"       → Move(slot - 1)
///   "switch 1"–"switch 6" → Switch(slot - 1)
pub fn parse_turn_line(trimmed: &str, n: usize) -> Result<TurnChoice, String> {
    if let Some(rest) = trimmed.strip_prefix("switch ") {
        let idx: usize = rest.trim().parse().map_err(|_| {
            alloc::format!("expected slot number after 'switch', got '{}'", rest.trim())
        })?;
        if idx == 0 || idx > 6 {
            return Err(alloc::format!("switch slot must be 1-6, got {}", idx));
        }
        return Ok(TurnChoice::Switch(idx - 1));
    }
    // "s2" / "S2" — switch to team slot 2, matching the prompt's [sN] labels.
    if let Some(rest) = trimmed.strip_prefix(['s', 'S']) {
        if let Ok(idx) = rest.trim().parse::<usize>() {
            if idx == 0 || idx > 6 {
                return Err(alloc::format!("switch slot must be s1-s6, got s{}", idx));
            }
            return Ok(TurnChoice::Switch(idx - 1));
        }
    }
    let slot: usize = trimmed.parse().map_err(|_| {
        alloc::format!("expected move number 1-{} or switch s1-s6, got '{}'", n, trimmed)
    })?;
    if slot == 0 || slot > n {
        return Err(alloc::format!("move slot must be 1-{}, got {}", n, slot));
    }
    Ok(TurnChoice::Move(slot - 1))
}

/// Parse a USB CLI forced-switch prompt line. Returns 0-based party slot.
/// Accepts "1"–"6".
pub fn parse_switch_line(trimmed: &str) -> Result<usize, String> {
    let idx: usize = trimmed.parse().map_err(|_| {
        alloc::format!("expected slot number 1-6, got '{}'", trimmed)
    })?;
    if idx == 0 || idx > 6 {
        return Err(alloc::format!("switch slot must be 1-6, got {}", idx));
    }
    Ok(idx - 1)
}

// ── USB lobby command parsing ─────────────────────────────────────────────────

/// A command from the USB CLI lobby phase.
#[derive(Debug, PartialEq)]
pub enum LobbyCmd {
    ReadyP1,
    ReadyP2,
    ReadyBoth,
    UnreadyP1,
    UnreadyP2,
    UnreadyBoth,
    P1Ai,
    P2Ai,
    VsAi,
    Demo,
    StopDemo,
    /// `:team p1 …` / `:team p2 …` — caller must then parse the team via
    /// [`parse_team_spec`] (the team payload isn't carried in this enum so
    /// `LobbyCmd` stays `Copy`-cheap and `PartialEq`).
    UploadTeam,
    Unknown,
}

pub fn parse_lobby_cmd(line: &str) -> LobbyCmd {
    match line.trim() {
        ":ready"                                         => LobbyCmd::ReadyBoth,
        ":ready p1"                                      => LobbyCmd::ReadyP1,
        ":ready p2"                                      => LobbyCmd::ReadyP2,
        ":ready p1 ai"                                   => LobbyCmd::P1Ai,
        ":ready p2 ai"                                   => LobbyCmd::P2Ai,
        ":ready ai" | ":ready both ai"                   => LobbyCmd::VsAi,
        ":cancel p1" | ":unready p1"                     => LobbyCmd::UnreadyP1,
        ":cancel p2" | ":unready p2"                     => LobbyCmd::UnreadyP2,
        ":cancel" | ":unready"                           => LobbyCmd::UnreadyBoth,
        ":demo"                                          => LobbyCmd::Demo,
        ":s" | ":stop"                                   => LobbyCmd::StopDemo,
        l if l.starts_with(":team ")                     => LobbyCmd::UploadTeam,
        _                                                => LobbyCmd::Unknown,
    }
}

/// Parse a `:team` upload line into `(player_index, team)`, the team in the
/// protocol's packed form (what `>player` carries).
///
/// Syntax:
/// ```text
/// :team p1 SPECIES[:MOVE[:MOVE[:MOVE[:MOVE]]]][,SPECIES...]
/// ```
/// - `p1` → player index 0, `p2` → 1.
/// - Up to 6 comma-separated mons; up to 4 colon-separated moves each.
/// - Level defaults to 100. If no moves are given, the mon gets `tackle`
///   so it can still act.
///
/// Species/move strings are passed through verbatim (a name too long for
/// the protocol's [`Name`] rejects the line); the sim resolves them the way
/// PS does (`toID`) and rejects anything it can't.
pub fn parse_team_spec(line: &str) -> Option<(u8, PackedTeam)> {
    let rest = line.trim().strip_prefix(":team ")?.trim();
    let (who, spec) = rest.split_once(char::is_whitespace)?;
    let player = match who.trim() {
        "p1" => 0u8,
        "p2" => 1u8,
        _ => return None,
    };

    let mut team = PackedTeam::new();
    for mon_spec in spec.split(',') {
        let mon_spec = mon_spec.trim();
        if mon_spec.is_empty() {
            continue;
        }
        let mut parts = mon_spec.split(':');
        let species = parts.next()?.trim();
        if species.is_empty() {
            return None;
        }
        let move_strs: Vec<&str> =
            parts.map(|m| m.trim()).filter(|m| !m.is_empty()).take(4).collect();

        let mut set = PackedSet {
            // Leave the nickname empty: PS then uses the species' proper
            // display name ("Ditto", "Mr. Mime"), which the sprite table and
            // narration are keyed on. Typed input here is lowercase ids.
            name: Name::default(),
            species: Name::from_str(species).ok()?,
            level: 100,
            happiness: 255,
            ..Default::default()
        };
        for m in if move_strs.is_empty() { &["tackle"][..] } else { &move_strs[..] } {
            set.moves.push(Name::from_str(m).ok()?).ok()?;
        }
        team.push(set).ok()?;
        if team.len() == 6 {
            break;
        }
    }

    if team.is_empty() {
        None
    } else {
        Some((player, team))
    }
}
