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
//! - every `>` line (a PS command: `>start`, `>player`, `>pN ...`,
//!   `>forcewin`, `>forcetie`; or one of the recorder's block separators:
//!   `>update`, `>sideupdate pN`) decodes to a [`CmdLine`] that re-encodes
//!   byte for byte, and every team a `>player` command carries unpacks and
//!   packs back to the same string;
//! - the transcript's `#battle gen=G index=I` labels are metadata, not
//!   protocol lines, and the empty lines are nothing; every other line is
//!   checked.
//!
//! The dex name tables come from `fixtures/names.tsv`. With
//! `PS_PROTOCOL_FULL_LOGS=DIR` (`record-logs.js --full DIR` writes it), the
//! `full_corpus_round_trips` test also checks all 60 battles per gen.
//!
//! [`Msg`]: ps_protocol::Msg
//! [`TextId`]: ps_protocol::TextId
//! [`Request`]: ps_protocol::Request
//! [`CmdLine`]: ps_protocol::text::command::CmdLine

#![cfg(feature = "text")]

use std::io::Read;
use std::path::{Path, PathBuf};

use ps_protocol::text::command::{decode_cmd_line, encode_cmd_line, CmdLine};
use ps_protocol::text::json::{decode_request, encode_request};
use ps_protocol::text::line::{decode_line, encode_msg, CodecState};
use ps_protocol::text::names::Tables;
use ps_protocol::text::team::{pack, unpack};
use ps_protocol::{Command, Msg, SideId, Text, TextArg, TextId};

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
    /// Every `>` line that round-trips: the PS commands (`>start`,
    /// `>player`, `>pN`) and the recorder's block separators (`>update`,
    /// `>sideupdate pN`).
    commands: u64,
    /// The `>start` lines.
    starts: u64,
    /// The `>player` lines.
    players: u64,
    /// The `>pN <choice>` lines.
    choices: u64,
    /// The `>update` and `>sideupdate pN` block separators.
    blocks: u64,
    skipped_empty: u64,
    /// A `>#battle ...` label that begins a battle in the recorded corpus.
    /// It is not a protocol line and not a `>` line: the transcript's
    /// table of contents.
    battle_label: u64,
    failures: Vec<String>,
}

#[test]
fn gen1_log_round_trips() {
    let c = roundtrip("gen1.log.gz", &read_gz(&fixture_path("gen1.log.gz")));
    assert!(c.lines > 10_000 && c.requests > 1_000 && c.teams == 40, "gen 1 corpus shrank");
    // Every `>` line of the corpus round-trips; only the empty lines and
    // the `#battle` labels are not protocol lines.
    assert_eq!(c.starts, 20, "gen 1 >start lines");
    assert_eq!(c.players, 40, "gen 1 >player lines");
    assert_eq!(c.choices, 2164, "gen 1 >pN choice lines");
    assert_eq!(c.blocks, 1186 + 2436, "gen 1 block separators");
    assert_eq!(c.commands, 20 + 40 + 2164 + 1186 + 2436, "gen 1 `>` lines");
}

#[test]
fn gen3_log_round_trips() {
    let c = roundtrip("gen3.log.gz", &read_gz(&fixture_path("gen3.log.gz")));
    assert!(c.lines > 10_000 && c.requests > 1_000 && c.teams == 40, "gen 3 corpus shrank");
    // The 10 Unown switch lines and the 5 Trace lines of the committed battles.
    assert_eq!(c.forme_or_old_ability, 15, "forme / Trace lines round-tripped");
    assert_eq!(c.starts, 20, "gen 3 >start lines");
    assert_eq!(c.players, 40, "gen 3 >player lines");
    assert_eq!(c.choices, 1948, "gen 3 >pN choice lines");
    assert_eq!(c.blocks, 1078 + 2158, "gen 3 block separators");
    assert_eq!(c.commands, 20 + 40 + 1948 + 1078 + 2158, "gen 3 `>` lines");
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
        } else if raw.starts_with('#') {
            // Each battle has its own players and nicknames.
            state = CodecState::new(tables.clone());
            c.battle_label += 1;
        } else if raw.starts_with('>') {
            // Every `>` line: a PS command (`>start`, `>player`, `>pN`) or
            // one of the recorder's block separators (`>update`,
            // `>sideupdate pN`).
            match decode_cmd_line(raw) {
                Ok(decoded) => {
                    let re = encode_cmd_line(&decoded);
                    if re == raw {
                        c.commands += 1;
                        match &decoded {
                            CmdLine::Command(Command::Start { .. }) => c.starts += 1,
                            CmdLine::Command(Command::Player { name, team, .. }) => {
                                c.players += 1;
                                c.teams += 1;
                                // The packed team the line carries packs
                                // back to the same string.
                                assert_eq!(
                                    pack(team),
                                    json_field(raw, "team").expect(">player without a team"),
                                    "team mismatch in line {}",
                                    line_no
                                );
                                assert_eq!(
                                    name.as_str(),
                                    json_field(raw, "name").unwrap_or_default(),
                                    "name mismatch in line {}",
                                    line_no
                                );
                            }
                            CmdLine::Command(Command::Choose { .. }) => c.choices += 1,
                            CmdLine::Block(_) => c.blocks += 1,
                            CmdLine::Command(_) => {}
                        }
                    } else {
                        fail(format!("re-encodes as\n    {:?}", re));
                    }
                }
                Err(e) => fail(format!("decode: {}", e)),
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
        "{}: {} lines, {} requests, {} teams, {} `>` lines round-tripped ({} start, {} player, {} choice, {} block); skipped {} empty and {} battle labels; {} failures",
        name,
        c.lines,
        c.requests,
        c.teams,
        c.commands,
        c.starts,
        c.players,
        c.choices,
        c.blocks,
        c.skipped_empty,
        c.battle_label,
        c.failures.len()
    );
    for f in c.failures.iter().take(20) {
        eprintln!("  {}", f);
    }
    assert!(c.failures.is_empty(), "round-trip failures in {}", name);
    c
}

/// The value of a JSON string field of a `>player` line, with PS's escapes
/// resolved.
fn json_field(line: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\":\"");
    let start = line.find(&needle)? + needle.len();
    let mut out = String::new();
    let mut chars = line[start..].chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(out),
            '\\' => match chars.next()? {
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                '/' => out.push('/'),
                'b' => out.push('\u{8}'),
                'f' => out.push('\u{c}'),
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                'u' => {
                    let hex: String = chars.by_ref().take(4).collect();
                    out.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?);
                }
                _ => return None,
            },
            _ => out.push(c),
        }
    }
    None
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
