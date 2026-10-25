//! The client's events for the Gen 3 lines the board narrates beyond HP and
//! moves: `-ability`, `-item`, `-enditem`, `-activate`, `-fieldactivate`,
//! the `[from] ability:` banner on other lines, `-start` / `-end` with
//! `[silent]` and `[upkeep]`, `-fail` of a stat drop, `-hitcount`, `-ohko`,
//! `-cureteam`, `-formechange` with `[msg]`.
//!
//! One hand-made script (`tests/narration/gen3_lines.txt`), every line in
//! the form `ps_sim` and PS 0.11.11 write it, is fed through
//! [`BattleView`] line by line; the stream, each line followed by its
//! events, must equal `tests/golden/gen3_narration.txt`
//! (`PS_CLIENT_BLESS=1` rewrites it). P2-1's golden style; the recorded
//! battles' goldens are in `fixtures.rs`.

mod common;

use std::fmt::Write as _;
use std::path::Path;

use common::tables;

use ps_client::{BattleView, BoardEvent};
use ps_protocol::text::json::decode_request;
use ps_protocol::text::line::{decode_line, CodecState};
use ps_protocol::SideId;

const TEAMS: [[&str; 4]; 2] = [["Gardevoir", "Snorlax", "Sneasel", "Castform"], ["Golduck", "Dusclops", "Breloom", "Starmie"]];

/// Plays `script` (PS lines; `#` comments and blank lines skipped) and
/// renders each line that yields events, then its events.
fn stream(script: &str) -> String {
    let dex = &ps_dex::GEN3;
    let mut codec = CodecState::new(tables(dex));
    let mut view = BattleView::new(dex);
    for (i, side) in [SideId::P1, SideId::P2].into_iter().enumerate() {
        for (slot, nick) in TEAMS[i].iter().enumerate() {
            codec.battle_mut().set_nickname(side, nick);
            view.set_nickname(side, slot as u8, nick);
        }
    }
    let mut out = String::new();
    for raw in script.lines() {
        if raw.is_empty() || raw.starts_with('#') {
            continue;
        }
        let events = if let Some(json) = raw.strip_prefix("|request|") {
            let (dex_names, names) = codec.split_mut();
            let req = decode_request(json, dex_names, names).unwrap_or_else(|e| panic!("{raw}: {e}"));
            view.apply_request(&req)
        } else {
            let msg = decode_line(raw, &mut codec).unwrap_or_else(|e| panic!("{raw}: {e}"));
            view.apply(&msg)
        };
        let cause: String = if raw.starts_with("|request|") { "|request|...".into() } else { raw.into() };
        writeln!(out, "{cause}").unwrap();
        if events.is_empty() {
            writeln!(out, "    (nothing)").unwrap();
        }
        for e in &events {
            match e {
                BoardEvent::MovesUpdate { player_id, moves } => {
                    let pp: Vec<String> =
                        moves.iter().map(|m| format!("{} {}/{}", m.name, m.pp, m.max_pp)).collect();
                    writeln!(out, "    MovesUpdate {player_id} [{}]", pp.join("; ")).unwrap();
                }
                BoardEvent::SwitchIn { name, player_id, .. } => {
                    writeln!(out, "    SwitchIn {} {name}", player_id.as_deref().unwrap_or("?")).unwrap();
                }
                e => writeln!(out, "    {e:?}").unwrap(),
            }
        }
    }
    out
}

#[test]
fn golden_gen3_item_ability_activate_lines() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let script = std::fs::read_to_string(dir.join("narration/gen3_lines.txt")).unwrap();
    let text = format!(
        "# BoardEvent stream of tests/narration/gen3_lines.txt: each line, then its events\n\
         # (`(nothing)` when it yields none).\n{}",
        stream(&script)
    );
    let path = dir.join("golden/gen3_narration.txt");
    if std::env::var_os("PS_CLIENT_BLESS").is_some() {
        std::fs::write(&path, &text).unwrap();
        return;
    }
    let want = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e} (PS_CLIENT_BLESS=1 writes it)", path.display()));
    if want != text {
        let first = want.lines().zip(text.lines()).position(|(a, b)| a != b);
        panic!("{} differs at line {:?}", path.display(), first.map(|i| i + 1));
    }
}

/// The rules the golden shows, asserted directly so a re-bless cannot
/// quietly drop them.
#[test]
fn every_line_kind_yields_its_event() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let script = std::fs::read_to_string(dir.join("narration/gen3_lines.txt")).unwrap();
    let out = stream(&script);
    let after = |line: &str| -> String {
        let at = out.find(&format!("{line}\n")).unwrap_or_else(|| panic!("{line} not in the stream"));
        out[at + line.len() + 1..].lines().take_while(|l| l.starts_with("    ")).collect::<Vec<_>>().join("\n")
    };
    let cases: &[(&str, &str)] = &[
        ("|-ability|p2a: Golduck|Intimidate|boost", r#"Ability { mon: "Golduck,p2,0", ability: "Intimidate", how: """#),
        ("|-ability|p2a: Golduck|Pressure|[silent]", "(nothing)"),
        ("|-ability|p1a: Gardevoir|Levitate|Trace|[from] ability: Trace|[of] p2a: Golduck", r#"how: "trace", other: Some("Golduck,p2,0")"#),
        ("|-immune|p2a: Golduck|[from] ability: Water Absorb", r#"Ability { mon: "Golduck,p2,0", ability: "Water Absorb", how: "activate""#),
        ("|-status|p1a: Gardevoir|par|[from] ability: Static|[of] p2a: Golduck", r#"Ability { mon: "Golduck,p2,0", ability: "Static""#),
        ("|-item|p1a: Gardevoir|Choice Band|[from] move: Trick", r#"ItemGained { mon: "Gardevoir,p1,0", item: "Choice Band", how: "trick""#),
        ("|-item|p1a: Gardevoir|Leftovers|[from] move: Thief|[of] p2a: Golduck", r#"how: "thief", other: Some("Golduck,p2,0")"#),
        ("|-enditem|p1a: Gardevoir|Salac Berry|[eat]", r#"ItemEnded { mon: "Gardevoir,p1,0", item: "Salac Berry", how: "eat""#),
        ("|-enditem|p1a: Gardevoir|Leftovers|[from] move: Knock Off|[of] p2a: Golduck", r#"how: "knockoff", other: Some("Golduck,p2,0")"#),
        ("|-enditem|p1a: Gardevoir|Bright Powder|[silent]|[from] move: Trick", "(nothing)"),
        ("|-activate|p2a: Golduck|Protect", r#"Activate { mon: "Golduck,p2,0", what: "Protect""#),
        ("|-activate|p2a: Golduck|move: Spite|psychic|4", "MovesUpdate p2 [Psychic 12/16"),
        ("|-activate|p2a: Golduck|move: Grudge|Surf", "Surf 0/24"),
        ("|-activate|p2a: Golduck|item: Leppa Berry|Psychic|[consumed]", "MovesUpdate p2 [Psychic 16/16"),
        ("|-activate|p1a: Gardevoir|move: Mimic|Surf", r#"EffectStart { mon: "Gardevoir,p1,0", what: "mimic", detail: Some("Surf") }"#),
        ("|-activate|p1a: Gardevoir|ability: Shed Skin", r#"Ability { mon: "Gardevoir,p1,0", ability: "Shed Skin", how: "activate""#),
        ("|-activate|p1a: Gardevoir|trapped", r#"EffectStart { mon: "Gardevoir,p1,0", what: "trapped""#),
        ("|-fieldactivate|move: Perish Song", r#"Activate { mon: "", what: "Perish Song""#),
        ("|-hitcount|p2a: Golduck|3", "n: Some(3)"),
        ("|-start|p1a: Gardevoir|Uproar|[upkeep]", r#"what: "Uproar", detail: Some("upkeep")"#),
        ("|-start|p1a: Gardevoir|perish3|[silent]", "(nothing)"),
        ("|-end|p1a: Gardevoir|move: Taunt|[silent]", "(nothing)"),
        ("|-fail|p2a: Golduck|unboost|Attack|[from] ability: Hyper Cutter|[of] p2a: Golduck", r#"what: "unboost", detail: Some("Attack")"#),
        ("|-mustrecharge|p1a: Gardevoir", "(nothing)"),
    ];
    for (line, want) in cases {
        let got = after(line);
        assert!(got.contains(want), "{line}\n  want: {want}\n  got:\n{got}");
    }
}

