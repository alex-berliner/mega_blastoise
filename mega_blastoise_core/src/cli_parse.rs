extern crate alloc;

use alloc::string::String;

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
    let slot: usize = trimmed.parse().map_err(|_| {
        alloc::format!("expected move number 1-{}, got '{}'", n, trimmed)
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
    VsAi,
    Demo,
    StopDemo,
    Unknown,
}

pub fn parse_lobby_cmd(line: &str) -> LobbyCmd {
    match line.trim() {
        ":ready"                                         => LobbyCmd::ReadyBoth,
        ":ready p1"                                      => LobbyCmd::ReadyP1,
        ":ready p2"                                      => LobbyCmd::ReadyP2,
        ":ready p1 ai"                                   => LobbyCmd::P1Ai,
        ":ready ai" | ":ready both ai" | ":ready p2 ai" => LobbyCmd::VsAi,
        ":unready p1"                                    => LobbyCmd::UnreadyP1,
        ":unready p2"                                    => LobbyCmd::UnreadyP2,
        ":unready"                                       => LobbyCmd::UnreadyBoth,
        ":demo"                                          => LobbyCmd::Demo,
        ":s" | ":stop"                                   => LobbyCmd::StopDemo,
        _                                                => LobbyCmd::Unknown,
    }
}

// ── Web in-game command parsing ───────────────────────────────────────────────

/// A command parsed from the web UI text input during a battle.
#[derive(Debug, PartialEq)]
pub enum WebGameInput {
    /// Move slot (0-based). Default player is 2 (Blue); use `p1:` prefix for Red.
    Move { player: u8, slot: u8 },
    /// Switch to party slot (0-based). `s1`–`s3` input.
    Switch { player: u8, idx: u8 },
    Unknown,
}

/// Parse a web in-game CLI line (already trimmed).
/// Format: `[p1:|p2:]<cmd>` where cmd is:
///   "1"–"4"   → Move, slot 0-based
///   "s1"–"s3" (or "S1"–"S3") → Switch, idx 0-based
/// Default player is 2 (Blue) when no prefix is given.
pub fn parse_web_game_cmd(cmd: &str) -> WebGameInput {
    let (player, rest) = if let Some(r) = cmd.strip_prefix("p1:") {
        (1u8, r)
    } else if let Some(r) = cmd.strip_prefix("p2:") {
        (2u8, r)
    } else {
        (2u8, cmd)
    };

    if let Some(n) = rest.strip_prefix('s').or_else(|| rest.strip_prefix('S')) {
        if let Ok(idx) = n.parse::<u8>() {
            if idx >= 1 && idx <= 3 {
                return WebGameInput::Switch { player, idx: idx - 1 };
            }
        }
    }

    if let Ok(slot) = rest.parse::<u8>() {
        if slot >= 1 && slot <= 4 {
            return WebGameInput::Move { player, slot: slot - 1 };
        }
    }

    WebGameInput::Unknown
}
