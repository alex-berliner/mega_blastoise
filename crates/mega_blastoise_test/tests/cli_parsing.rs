//! Regression tests for the battle CLI command parsing (shared by USB and web).
//!
//! These are the canonical contracts for what the firmware and web UI accept.
//! If a refactor changes any of these, this file will catch it.

use mega_blastoise_core::{
    parse_lobby_cmd, parse_switch_line, parse_team_spec, parse_turn_line, LobbyCmd, TurnChoice,
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
fn usb_switch_s2_is_idx_1() {
    assert_eq!(parse_turn_line("s2", 4), Ok(TurnChoice::Switch(1)));
}

#[test]
fn usb_switch_uppercase_s3_is_idx_2() {
    assert_eq!(parse_turn_line("S3", 4), Ok(TurnChoice::Switch(2)));
}

#[test]
fn usb_switch_s0_rejected() {
    assert!(parse_turn_line("s0", 4).is_err());
}

#[test]
fn usb_switch_s7_rejected() {
    assert!(parse_turn_line("s7", 4).is_err());
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
fn lobby_ready_p2_ai() {
    assert_eq!(parse_lobby_cmd(":ready p2 ai"), LobbyCmd::P2Ai);
}

#[test]
fn lobby_vs_ai_all_aliases() {
    assert_eq!(parse_lobby_cmd(":ready ai"),      LobbyCmd::VsAi);
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

// ── :team upload ──────────────────────────────────────────────────────────────

#[test]
fn team_cmd_classified_as_upload() {
    assert_eq!(parse_lobby_cmd(":team p1 pikachu"), LobbyCmd::UploadTeam);
    assert_eq!(parse_lobby_cmd(":team p2 snorlax:bodyslam"), LobbyCmd::UploadTeam);
}

#[test]
fn team_spec_player_index() {
    let (p, _) = parse_team_spec(":team p1 pikachu").unwrap();
    assert_eq!(p, 0);
    let (p, _) = parse_team_spec(":team p2 pikachu").unwrap();
    assert_eq!(p, 1);
}

#[test]
fn team_spec_leaves_name_for_engine_canonicalization() {
    // The engine fills empty names with the species display name ("Ditto"),
    // which the sprite table is keyed on — a raw typed name here would break
    // sprite lookup.
    let (_, team) = parse_team_spec(":team p1 ditto:transform").unwrap();
    assert!(team[0].name.is_empty());
}

#[test]
fn team_spec_species_only_gets_default_move() {
    let (_, team) = parse_team_spec(":team p1 snorlax").unwrap();
    assert_eq!(team.len(), 1);
    assert_eq!(team[0].species, "snorlax");
    assert_eq!(team[0].level, 100);
    assert_eq!(team[0].moves.len(), 1);
    assert_eq!(team[0].moves[0].id, "tackle");
}

#[test]
fn team_spec_with_moves() {
    let (_, team) =
        parse_team_spec(":team p1 snorlax:bodyslam:earthquake:rest:hyperbeam").unwrap();
    assert_eq!(team.len(), 1);
    let ids: Vec<&str> = team[0].moves.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids, ["bodyslam", "earthquake", "rest", "hyperbeam"]);
}

#[test]
fn team_spec_multiple_mons() {
    let (_, team) =
        parse_team_spec(":team p2 pikachu:thunderbolt,charizard:flamethrower,snorlax").unwrap();
    assert_eq!(team.len(), 3);
    assert_eq!(team[0].species, "pikachu");
    assert_eq!(team[1].species, "charizard");
    assert_eq!(team[2].species, "snorlax");
}

#[test]
fn team_spec_caps_at_six_mons() {
    let (_, team) = parse_team_spec(
        ":team p1 a,b,c,d,e,f,g,h",
    )
    .unwrap();
    assert_eq!(team.len(), 6);
}

#[test]
fn team_spec_caps_moves_at_four() {
    let (_, team) =
        parse_team_spec(":team p1 snorlax:m1:m2:m3:m4:m5:m6").unwrap();
    assert_eq!(team[0].moves.len(), 4);
}

#[test]
fn team_spec_rejects_bad_player() {
    assert!(parse_team_spec(":team p3 pikachu").is_none());
    assert!(parse_team_spec(":team xx pikachu").is_none());
}

#[test]
fn team_spec_rejects_missing_parts() {
    assert!(parse_team_spec(":team p1").is_none());
    assert!(parse_team_spec(":team p1 ").is_none());
    assert!(parse_team_spec(":team").is_none());
    assert!(parse_team_spec("team p1 pikachu").is_none());
}
