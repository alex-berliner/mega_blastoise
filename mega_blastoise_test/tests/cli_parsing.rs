//! Regression tests for USB and web CLI command parsing.
//!
//! These are the canonical contracts for what the firmware and web UI accept.
//! If a refactor changes any of these, this file will catch it.

use mega_blastoise_core::{
    parse_lobby_cmd, parse_switch_line, parse_turn_line, parse_web_game_cmd,
    LobbyCmd, TurnChoice, WebGameInput,
};

// ── USB turn prompt — move slot ───────────────────────────────────────────────

#[test]
fn usb_move_slot_1_is_zero_indexed() {
    assert_eq!(parse_turn_line("1", 4), Ok(TurnChoice::Move(0)));
}

#[test]
fn usb_move_slot_4_is_three_indexed() {
    assert_eq!(parse_turn_line("4", 4), Ok(TurnChoice::Move(3)));
}

#[test]
fn usb_move_slot_upper_bound_matches_n() {
    assert_eq!(parse_turn_line("2", 2), Ok(TurnChoice::Move(1)));
}

#[test]
fn usb_move_slot_0_rejected() {
    assert!(parse_turn_line("0", 4).is_err());
}

#[test]
fn usb_move_slot_exceeds_n_rejected() {
    assert!(parse_turn_line("5", 4).is_err());
}

#[test]
fn usb_move_slot_n_plus_1_rejected() {
    assert!(parse_turn_line("3", 2).is_err());
}

#[test]
fn usb_move_garbage_rejected() {
    assert!(parse_turn_line("move1", 4).is_err());
    assert!(parse_turn_line("", 4).is_err());
    assert!(parse_turn_line("!", 4).is_err());
}

// ── USB turn prompt — in-turn switch ─────────────────────────────────────────

#[test]
fn usb_switch_1_is_zero_indexed() {
    assert_eq!(parse_turn_line("switch 1", 4), Ok(TurnChoice::Switch(0)));
}

#[test]
fn usb_switch_6_is_five_indexed() {
    assert_eq!(parse_turn_line("switch 6", 4), Ok(TurnChoice::Switch(5)));
}

#[test]
fn usb_switch_0_rejected() {
    assert!(parse_turn_line("switch 0", 4).is_err());
}

#[test]
fn usb_switch_7_rejected() {
    assert!(parse_turn_line("switch 7", 4).is_err());
}

#[test]
fn usb_switch_missing_number_rejected() {
    assert!(parse_turn_line("switch", 4).is_err());
    assert!(parse_turn_line("switch abc", 4).is_err());
}

// ── USB forced-switch prompt ──────────────────────────────────────────────────

#[test]
fn usb_forced_switch_1_is_zero_indexed() {
    assert_eq!(parse_switch_line("1"), Ok(0));
}

#[test]
fn usb_forced_switch_6_is_five_indexed() {
    assert_eq!(parse_switch_line("6"), Ok(5));
}

#[test]
fn usb_forced_switch_0_rejected() {
    assert!(parse_switch_line("0").is_err());
}

#[test]
fn usb_forced_switch_7_rejected() {
    assert!(parse_switch_line("7").is_err());
}

#[test]
fn usb_forced_switch_garbage_rejected() {
    assert!(parse_switch_line("abc").is_err());
    assert!(parse_switch_line("").is_err());
    assert!(parse_switch_line("s1").is_err());
}

// ── USB lobby commands ────────────────────────────────────────────────────────

#[test]
fn lobby_ready_both() {
    assert_eq!(parse_lobby_cmd(":ready"), LobbyCmd::ReadyBoth);
}

#[test]
fn lobby_ready_p1() {
    assert_eq!(parse_lobby_cmd(":ready p1"), LobbyCmd::ReadyP1);
}

#[test]
fn lobby_ready_p2() {
    assert_eq!(parse_lobby_cmd(":ready p2"), LobbyCmd::ReadyP2);
}

#[test]
fn lobby_ready_p1_ai() {
    assert_eq!(parse_lobby_cmd(":ready p1 ai"), LobbyCmd::P1Ai);
}

#[test]
fn lobby_vs_ai_all_aliases() {
    assert_eq!(parse_lobby_cmd(":ready ai"),      LobbyCmd::VsAi);
    assert_eq!(parse_lobby_cmd(":ready p2 ai"),   LobbyCmd::VsAi);
    assert_eq!(parse_lobby_cmd(":ready both ai"), LobbyCmd::VsAi);
}

#[test]
fn lobby_unready_commands() {
    assert_eq!(parse_lobby_cmd(":unready"),    LobbyCmd::UnreadyBoth);
    assert_eq!(parse_lobby_cmd(":unready p1"), LobbyCmd::UnreadyP1);
    assert_eq!(parse_lobby_cmd(":unready p2"), LobbyCmd::UnreadyP2);
}

#[test]
fn lobby_demo() {
    assert_eq!(parse_lobby_cmd(":demo"), LobbyCmd::Demo);
}

#[test]
fn lobby_stop_aliases() {
    assert_eq!(parse_lobby_cmd(":s"),    LobbyCmd::StopDemo);
    assert_eq!(parse_lobby_cmd(":stop"), LobbyCmd::StopDemo);
}

#[test]
fn lobby_unknown_is_unknown() {
    assert_eq!(parse_lobby_cmd("hello"), LobbyCmd::Unknown);
    assert_eq!(parse_lobby_cmd(":ready p3"), LobbyCmd::Unknown);
    assert_eq!(parse_lobby_cmd(""), LobbyCmd::Unknown);
}

#[test]
fn lobby_cmd_trims_whitespace() {
    assert_eq!(parse_lobby_cmd("  :ready  "), LobbyCmd::ReadyBoth);
}

// ── Web in-game commands ──────────────────────────────────────────────────────

#[test]
fn web_move_1_defaults_to_p2() {
    assert_eq!(parse_web_game_cmd("1"), WebGameInput::Move { player: 2, slot: 0 });
}

#[test]
fn web_move_4_is_slot_3() {
    assert_eq!(parse_web_game_cmd("4"), WebGameInput::Move { player: 2, slot: 3 });
}

#[test]
fn web_move_p1_prefix() {
    assert_eq!(parse_web_game_cmd("p1:1"), WebGameInput::Move { player: 1, slot: 0 });
    assert_eq!(parse_web_game_cmd("p1:4"), WebGameInput::Move { player: 1, slot: 3 });
}

#[test]
fn web_move_p2_prefix_explicit() {
    assert_eq!(parse_web_game_cmd("p2:2"), WebGameInput::Move { player: 2, slot: 1 });
}

#[test]
fn web_switch_s1_is_idx_0() {
    assert_eq!(parse_web_game_cmd("s1"), WebGameInput::Switch { player: 2, idx: 0 });
}

#[test]
fn web_switch_s3_is_idx_2() {
    assert_eq!(parse_web_game_cmd("s3"), WebGameInput::Switch { player: 2, idx: 2 });
}

#[test]
fn web_switch_uppercase_s() {
    assert_eq!(parse_web_game_cmd("S2"), WebGameInput::Switch { player: 2, idx: 1 });
}

#[test]
fn web_switch_p1_prefix() {
    assert_eq!(parse_web_game_cmd("p1:s2"), WebGameInput::Switch { player: 1, idx: 1 });
}

#[test]
fn web_move_0_rejected() {
    assert_eq!(parse_web_game_cmd("0"), WebGameInput::Unknown);
}

#[test]
fn web_move_5_rejected() {
    assert_eq!(parse_web_game_cmd("5"), WebGameInput::Unknown);
}

#[test]
fn web_switch_s0_rejected() {
    assert_eq!(parse_web_game_cmd("s0"), WebGameInput::Unknown);
}

#[test]
fn web_switch_s4_rejected() {
    assert_eq!(parse_web_game_cmd("s4"), WebGameInput::Unknown);
}

#[test]
fn web_garbage_is_unknown() {
    assert_eq!(parse_web_game_cmd(""), WebGameInput::Unknown);
    assert_eq!(parse_web_game_cmd("move"), WebGameInput::Unknown);
    assert_eq!(parse_web_game_cmd(":ready"), WebGameInput::Unknown);
}
