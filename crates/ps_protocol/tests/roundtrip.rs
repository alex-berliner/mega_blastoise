//! Round-trip the recorded PS battles.
//!
//! Reads `crates/ps_protocol/tests/fixtures/{gen1,gen3}.log.gz` (20 battles
//! per gen recorded by `tools/ps-diff/record-logs.js` from PS 0.11.11's
//! `BattleStream`) and checks every line of them:
//!
//! - every protocol line decodes to a [`Msg`] that re-encodes to the same
//!   bytes, text lines (`raw`, `debug`, `error`, `-message`, `-hint`) through
//!   their [`TextId`] templates;
//! - every `|request|` JSON decodes to a [`Request`] that re-encodes to the
//!   same bytes;
//! - every team a `>player` command carries unpacks and packs back to the
//!   same string.
//!
//! The only lines not checked are empty lines and the other `>` lines (the
//! transcript's block headers and the choices sent). Anything else that does
//! not round-trip, an unknown kind included, fails the test.
//!
//! The dex name tables come from `fixtures/names.tsv`. With
//! `PS_PROTOCOL_FULL_LOGS=DIR` (`record-logs.js --full DIR` writes it), the
//! `full_corpus_round_trips` test also checks all 60 battles per gen.
//!
//! [`Msg`]: ps_protocol::Msg
//! [`TextId`]: ps_protocol::TextId
//! [`Request`]: ps_protocol::Request

#![cfg(feature = "text")]

use std::io::Read;
use std::path::{Path, PathBuf};

use ps_protocol::text::json::{decode_request, encode_request};
use ps_protocol::text::line::{decode_line, encode_msg, CodecState};
use ps_protocol::text::names::Tables;
use ps_protocol::text::team::{pack, unpack};
use ps_protocol::{Msg, SideId, Text, TextArg, TextId};

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures").join(name)
}

fn load_names() -> Tables {
    let s = std::fs::read_to_string(fixture_path("names.tsv")).expect("names.tsv readable");
    Tables::from_tsv(&s)
}

fn read_gz(path: &Path) -> String {
    let raw = std::fs::read(path).unwrap_or_else(|e| panic!("{}: {}", path.display(), e));
    let mut s = String::new();
    flate2::read::GzDecoder::new(&raw[..]).read_to_string(&mut s).expect("gzip");
    s
}

/// What one log round-tripped.
#[derive(Default)]
struct Counts {
    lines: u64,
    requests: u64,
    teams: u64,
    /// Lines of the kinds the old types could not hold: a details string
    /// with a cosmetic forme, and `-ability` with the old ability.
    forme_or_old_ability: u64,
    skipped_empty: u64,
    skipped_commands: u64,
    failures: Vec<String>,
}

#[test]
fn gen1_log_round_trips() {
    let c = roundtrip("gen1.log.gz", &read_gz(&fixture_path("gen1.log.gz")));
    assert!(c.lines > 10_000 && c.requests > 1_000 && c.teams == 40, "gen 1 corpus shrank");
}

#[test]
fn gen3_log_round_trips() {
    let c = roundtrip("gen3.log.gz", &read_gz(&fixture_path("gen3.log.gz")));
    assert!(c.lines > 10_000 && c.requests > 1_000 && c.teams == 40, "gen 3 corpus shrank");
    // The 10 Unown switch lines and the 5 Trace lines of the committed battles.
    assert_eq!(c.forme_or_old_ability, 15, "forme / Trace lines round-tripped");
}

/// All 60 recorded battles per gen, when `PS_PROTOCOL_FULL_LOGS` names the
/// directory `record-logs.js --full` wrote.
#[test]
fn full_corpus_round_trips() {
    let Some(dir) = std::env::var_os("PS_PROTOCOL_FULL_LOGS") else {
        eprintln!("PS_PROTOCOL_FULL_LOGS not set; the committed fixtures are checked by the other tests");
        return;
    };
    for name in ["gen1.log", "gen3.log"] {
        let path = Path::new(&dir).join(name);
        let body = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {}", path.display(), e));
        roundtrip(name, &body);
    }
}

fn roundtrip(name: &str, body: &str) -> Counts {
    let tables = load_names();
    let mut c = Counts::default();
    let mut state = CodecState::new(tables.clone());
    for (i, raw) in body.lines().enumerate() {
        let line_no = i + 1;
        let mut fail = |what: String| c.failures.push(format!("line {}: {}\n    {:?}", line_no, what, raw));
        if raw.is_empty() {
            c.skipped_empty += 1;
        } else if raw.starts_with("#battle ") {
            // Each battle has its own players and nicknames.
            state = CodecState::new(tables.clone());
        } else if let Some(cmd) = raw.strip_prefix('>') {
            if cmd.starts_with("player ") {
                match team_of(cmd) {
                    Some(team) => match unpack(&team) {
                        Ok(t) if pack(&t) == team => c.teams += 1,
                        Ok(t) => fail(format!("team re-packs as {:?}", pack(&t))),
                        Err(e) => fail(format!("team: {}", e)),
                    },
                    None => fail("no team in >player".into()),
                }
            } else {
                c.skipped_commands += 1;
            }
        } else if let Some(json) = raw.strip_prefix("|request|") {
            let (dex, battle) = state.split_mut();
            match decode_request(json, dex, battle) {
                Ok(req) => {
                    let re = encode_request(&req, state.dex(), state.battle());
                    if re == json {
                        c.requests += 1;
                    } else {
                        fail(format!("request re-encodes as\n    {:?}", re));
                    }
                }
                Err(e) => fail(format!("request: {}", e)),
            }
        } else if raw.starts_with('|') {
            match decode_line(raw, &mut state) {
                Ok(msg) => {
                    let re = encode_msg(&msg, state.dex(), state.battle());
                    if re == raw {
                        c.lines += 1;
                        if is_forme_or_old_ability(&msg) {
                            c.forme_or_old_ability += 1;
                        }
                    } else {
                        fail(format!("re-encodes as\n    {:?}", re));
                    }
                }
                Err(e) => fail(format!("decode: {}", e)),
            }
        } else {
            fail("not a protocol line, a request or a command".into());
        }
    }
    eprintln!(
        "{}: {} lines, {} requests, {} teams round-tripped; skipped {} empty and {} `>` lines; {} failures",
        name,
        c.lines,
        c.requests,
        c.teams,
        c.skipped_empty,
        c.skipped_commands,
        c.failures.len()
    );
    for f in c.failures.iter().take(20) {
        eprintln!("  {}", f);
    }
    assert!(c.failures.is_empty(), "round-trip failures in {}", name);
    c
}

fn is_forme_or_old_ability(msg: &Msg) -> bool {
    match msg {
        Msg::Switch { details, .. } | Msg::Drag { details, .. } | Msg::DetailsChange { details, .. } => {
            details.forme != 0
        }
        Msg::Ability { old, .. } => old.is_some(),
        _ => false,
    }
}

/// The `team` string of a `>player p1 {"name":...,"team":"..."}` command.
fn team_of(cmd: &str) -> Option<String> {
    let start = cmd.find("\"team\":\"")? + "\"team\":\"".len();
    let mut out = String::new();
    let mut chars = cmd[start..].chars();
    loop {
        match chars.next()? {
            '"' => return Some(out),
            '\\' => match chars.next()? {
                'u' => {
                    let hex: String = chars.by_ref().take(4).collect();
                    out.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?);
                }
                'n' => out.push('\n'),
                c => out.push(c),
            },
            c => out.push(c),
        }
    }
}

#[test]
fn kecleon_typechange_round_trips() {
    let mut state = CodecState::new(load_names());
    let line = "|-start|p2a: Kecleon|typechange|Flying|[from] ability: Color Change";
    let msg = decode_line(line, &mut state).unwrap();
    assert_eq!(encode_msg(&msg, state.dex(), state.battle()), line);
}

#[test]
fn text_lines_decode_to_their_templates() {
    let mut state = CodecState::new(load_names());
    let cases: [(&str, TextId); 5] = [
        ("|debug|BP: 70", TextId::BasePower),
        ("|debug|before turn callback: focuspunch", TextId::BeforeTurnCallback),
        ("|debug|set status [psn] interrupted", TextId::SetStatusInterrupted),
        ("|-hint|In Gen 1, moves with 100% accuracy can still miss 1/256 of the time.", TextId::HintGen1Miss),
        (
            "|raw|<div class=\"infobox\"><details class=\"readmore\" open><summary><strong>2 custom rules:</strong></summary> Sleep Clause Mod, Freeze Clause Mod</details></div>",
            TextId::CustomRules2,
        ),
    ];
    for (line, id) in cases {
        let msg = decode_line(line, &mut state).unwrap();
        let text = match msg {
            Msg::Debug(t) | Msg::Raw(t) | Msg::Message(t) | Msg::Hint { text: t, .. } => t,
            other => panic!("{:?}", other),
        };
        assert_eq!(text.id, id, "{}", line);
        assert_eq!(encode_msg(&msg, state.dex(), state.battle()), line);
    }
    // A number must read back as written; text no template makes fails.
    assert!(decode_line("|debug|BP: 070", &mut state).is_err());
    assert!(decode_line("|debug|something PS never says", &mut state).is_err());
}

#[test]
fn error_lines_keep_their_prefix() {
    let mut state = CodecState::new(load_names());
    for line in [
        "|error|[Invalid choice] Can't switch: The active Pokémon is trapped",
        "|error|[Unavailable choice] Can't switch: You do not have a Pokémon in slot 7 to switch to",
    ] {
        let msg = decode_line(line, &mut state).unwrap();
        assert!(matches!(msg, Msg::Error { side: SideId::P1, .. }));
        assert_eq!(encode_msg(&msg, state.dex(), state.battle()), line);
    }
    let disabled = Msg::Error {
        side: SideId::P2,
        unavailable: false,
        text: Text::with1(TextId::CantSwitchNoSlot, TextArg::int(3)),
    };
    assert_eq!(
        encode_msg(&disabled, state.dex(), state.battle()),
        "|error|[Invalid choice] Can't switch: You do not have a Pokémon in slot 3 to switch to"
    );
    assert!(decode_line("|error|Can't switch: The active Pokémon is trapped", &mut state).is_err());
}

#[test]
fn details_carry_cosmetic_formes_and_reject_unknown_species() {
    let mut state = CodecState::new(load_names());
    let line = "|switch|p1a: Unown|Unown-Question, L80, shiny|212/212";
    let msg = decode_line(line, &mut state).unwrap();
    let Msg::Switch { details, .. } = msg else { panic!("{:?}", msg) };
    assert_eq!({ details.forme }, 27);
    assert_eq!(encode_msg(&msg, state.dex(), state.battle()), line);
    // No silent collapse onto the base species.
    assert!(decode_line("|switch|p1a: Unown|Unown-Zz|212/212", &mut state).is_err());
}

#[test]
fn packed_sets_with_misc_round_trip() {
    for team in [
        "Pika|Pikachu|LightBall|Static|thunderbolt,surf|Timid|,,,252,4,252|F|,0,,,,|S|50|128,Fire,ultraball,G,5,Water",
        "Mew||||transform|||||||",
        "A|||||||||||]B|||||||||||",
        "Snorlax||Leftovers|ThickFat|bodyslam||85,85,85,85,85,85||||71|,,pokeball,,,",
    ] {
        let t = unpack(team).unwrap_or_else(|e| panic!("{}: {}", team, e));
        assert_eq!(pack(&t), team);
    }
    // Text PS's pack never writes is refused rather than normalised.
    assert!(unpack("A||||||||||100|").is_err());
    assert!(unpack("A||||||||,,,,,31|||").is_err());
}
