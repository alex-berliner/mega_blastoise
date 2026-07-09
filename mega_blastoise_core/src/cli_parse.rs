extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use gen1_battle::{MonData, MoveSlot};

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

/// Help lines for the lobby commands recognised by [`parse_lobby_cmd`].
/// Kept adjacent to the parser so the two stay in sync. The firmware prints
/// this in response to `:help` / `:h` / `?`, making the device the source of
/// truth for its own command set — keep every arm below represented here.
pub const LOBBY_HELP: &[&str] = &[
    ":ready            both players ready (human)",
    ":ready p1|p2      one player ready (human)",
    ":ready p1 ai      P1 is AI (play as P2)",
    ":ready p2 ai      P2 is AI (play as P1)",
    ":ready ai         AI vs AI (also: :ready both ai)",
    ":unready [p1|p2]  unready both / one player",
    ":demo             start a demo (AI vs AI) battle",
    ":s / :stop        cancel demo countdown / stop a running demo",
    ":team pN SPEC     upload team, e.g. :team p1 species:move:move,species:...",
];

pub fn parse_lobby_cmd(line: &str) -> LobbyCmd {
    match line.trim() {
        ":ready"                                         => LobbyCmd::ReadyBoth,
        ":ready p1"                                      => LobbyCmd::ReadyP1,
        ":ready p2"                                      => LobbyCmd::ReadyP2,
        ":ready p1 ai"                                   => LobbyCmd::P1Ai,
        ":ready p2 ai"                                   => LobbyCmd::P2Ai,
        ":ready ai" | ":ready both ai"                   => LobbyCmd::VsAi,
        ":unready p1"                                    => LobbyCmd::UnreadyP1,
        ":unready p2"                                    => LobbyCmd::UnreadyP2,
        ":unready"                                       => LobbyCmd::UnreadyBoth,
        ":demo"                                          => LobbyCmd::Demo,
        ":s" | ":stop"                                   => LobbyCmd::StopDemo,
        l if l.starts_with(":team ")                     => LobbyCmd::UploadTeam,
        _                                                => LobbyCmd::Unknown,
    }
}

/// Parse a `:team` upload line into `(player_index, team)`.
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
/// Species/move strings are passed through verbatim; `gen1_battle::update_team`
/// canonicalizes them (lowercases, strips non-alphanumerics) and rejects
/// anything it can't resolve.
pub fn parse_team_spec(line: &str) -> Option<(u8, Vec<MonData>)> {
    let rest = line.trim().strip_prefix(":team ")?.trim();
    let (who, spec) = rest.split_once(char::is_whitespace)?;
    let player = match who.trim() {
        "p1" => 0u8,
        "p2" => 1u8,
        _ => return None,
    };

    let mut team: Vec<MonData> = Vec::new();
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

        let make_slot = |id: &str| MoveSlot {
            name: id.to_string(),
            id: id.to_string(),
            typ: String::new(),
            pp: 0,
            max_pp: 0,
            disabled: false,
            target: 0,
        };
        let moves: Vec<MoveSlot> = if move_strs.is_empty() {
            alloc::vec![make_slot("tackle")]
        } else {
            move_strs.iter().map(|m| make_slot(m)).collect()
        };

        team.push(MonData {
            // Leave the name empty: the engine then uses the species' proper
            // display name ("Ditto", "Mr. Mime"), which the sprite table and
            // narration are keyed on. Typed input here is lowercase ids.
            name: String::new(),
            species: species.to_string(),
            level: 100,
            moves,
            ..Default::default()
        });
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
