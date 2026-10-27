//! The turn loop against recorded PS 0.11.11 battles.
//!
//! `tests/fixtures/battles_gen{1,3}.txt.gz` come from
//! `tools/ps-diff/battle-fixtures.js` (see its header for the format and the
//! battle kinds). Each battle is replayed command chunk by command chunk;
//! after each chunk the server's `sendUpdates` output, rendered as PS text,
//! must equal the recorded `update` and `sideupdate` blocks, line for line
//! (blocks recorded by hash compare by FNV-1a 64 of the same text).
//!
//! The lines and the request JSON go through the `ps_protocol` text codec
//! (`encode_log`, `encode_request`), the teams through its `unpack`, with
//! the dex name tables and battle names of [`crate::text`].
//!
//! The hook battles replace PS's `runMove` with a stub, on both sides:
//! [`stub_run_move`] here, `STUB` in the generator.
//!
//! `tests/fixtures/moves_gen{1,3}.txt.gz` (`tools/ps-diff/move-fixtures.js`,
//! same format) hold whole battles with the real pipeline: teams of ported
//! moves (and, in gen 3, handler-free abilities) only, played to the end.

extern crate std;

use std::boxed::Box;
use std::format;
use std::string::{String, ToString};
use std::sync::OnceLock;
use std::vec::Vec;

use ps_protocol::text::json::encode_request;
use ps_protocol::text::encode_log;
use ps_protocol::text::names::Tables;
use ps_protocol::text::team::unpack;
use ps_protocol::{Msg, PackedTeam, Request, SideId};

use super::*;
use crate::emit::Sink;
use crate::text::Names;

const CLOCK: u32 = 1_700_000_000;

// ---- the stub move pipeline ------------------------------------------------

/// The generator's `STUB`: slots 1 and 4 deal a third of the target's max
/// HP, slot 2 knocks it out, slot 3 makes the user faint and then knocks
/// the target out. No PRNG call.
fn stub_run_move(b: &mut Battle, mv: ActionMove, pokemon: MonRef, _target_loc: i8) {
    let target = b.active_at(1 - pokemon.side, 0);
    let id = mv.id().expect("a dex move");
    let slot = b.mon(pokemon).move_slots.iter().position(|s| s.id == id).unwrap_or(0);
    let user = b.ident(pokemon);
    let tgt = target.map(|t| b.ident(t));
    b.add(Msg::Move { user, mv: MoveNum(id.0), target: tgt, kw: Kw::NONE });
    if slot == 2 {
        b.faint(pokemon, None, EffectId::None);
    }
    let Some(target) = target else { return };
    if b.mon(target).fainted {
        return;
    }
    let mon = b.mon(target);
    let damage = if slot == 1 || slot == 2 { i32::from(mon.hp) } else { (i32::from(mon.maxhp) / 3).max(1) };
    b.direct_damage(damage, Some(target), Some(pokemon), EffectId::None);
}

// ---- fixtures ----------------------------------------------------------------

#[derive(Debug)]
enum Expected {
    Full { header: String, body: String },
    Hash { header: String, hash: u64 },
}

#[derive(Debug)]
struct Chunk {
    commands: Vec<String>,
    expected: Vec<Expected>,
}

#[derive(Debug)]
struct Fixture {
    kind: String,
    gen: u8,
    index: u32,
    chunks: Vec<Chunk>,
}

fn load(gen: u8) -> Vec<Fixture> {
    load_file(&format!("battles_gen{gen}.txt.gz"))
}

/// The text of a gzipped fixture.
fn fixture_text(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    let raw = std::fs::read(&path).expect("fixture readable");
    let mut text = String::new();
    std::io::Read::read_to_string(&mut flate2::read::GzDecoder::new(&raw[..]), &mut text).expect("gzip");
    text
}

fn load_file(name: &str) -> Vec<Fixture> {
    let text = fixture_text(name);
    let mut out = Vec::new();
    let mut lines = text.split('\n').peekable();
    while let Some(line) = lines.next() {
        let Some(rest) = line.strip_prefix("#battle ") else { continue };
        let mut parts = rest.split(' ');
        let kind = parts.next().unwrap().to_string();
        let gen: u8 = parts.next().unwrap().trim_start_matches("gen=").parse().unwrap();
        let index: u32 = parts.next().unwrap().trim_start_matches("index=").parse().unwrap();
        let mut chunks: Vec<Chunk> = Vec::new();
        loop {
            let line = lines.next().expect("unterminated battle");
            if line == "#end" {
                break;
            }
            if let Some(h) = line.strip_prefix(">hash ") {
                let f: Vec<&str> = h.split(' ').collect();
                let (header, hash) = if f[0] == "update" {
                    ("update".to_string(), f[1])
                } else {
                    (format!("{} {}", f[0], f[1]), f[2])
                };
                chunks.last_mut().unwrap().expected.push(Expected::Hash { header, hash: u64::from_str_radix(hash, 16).unwrap() });
            } else if line == ">update" || line.starts_with(">sideupdate ") {
                let mut body = Vec::new();
                while let Some(&next) = lines.peek() {
                    if next.starts_with('>') || next.starts_with('#') {
                        break;
                    }
                    body.push(lines.next().unwrap());
                }
                chunks
                    .last_mut()
                    .unwrap()
                    .expected
                    .push(Expected::Full { header: line[1..].to_string(), body: body.join("\n") });
            } else if let Some(cmd) = line.strip_prefix('>') {
                let new_chunk = chunks.last().is_none_or(|c| !c.expected.is_empty());
                if new_chunk {
                    chunks.push(Chunk { commands: Vec::new(), expected: Vec::new() });
                }
                chunks.last_mut().unwrap().commands.push(cmd.to_string());
            }
        }
        out.push(Fixture { kind, gen, index, chunks });
    }
    out
}

fn fnv64(text: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in text.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

// ---- choices ---------------------------------------------------------------

/// One choice of a recorded battle, through the `ps_protocol` command
/// codec: the fixture writes the choice as PS's `>pN` line writes it
/// (`move 1`, `switch 3`), so the codec reads it as one.
fn parse_choice(text: &str) -> ps_protocol::Choice {
    match ps_protocol::text::command::decode_command(&format!(">p1 {text}"))
        .unwrap_or_else(|e| panic!("choice {text:?}: {e}"))
    {
        ps_protocol::Command::Choose { choice, .. } => choice,
        other => panic!("choice {text:?}: not a choice but {other:?}"),
    }
}

fn parse_team(text: &str) -> PackedTeam {
    unpack(text).unwrap_or_else(|e| panic!("team {text:?}: {e}"))
}

// ---- rendering, through the `ps_protocol` text codec -----------------------

/// [`crate::text::dex_tables`], built once per dex.
fn tables(dex: &'static Dex) -> &'static Tables {
    static GEN1: OnceLock<Tables> = OnceLock::new();
    static GEN3: OnceLock<Tables> = OnceLock::new();
    let cell = if dex.gen == 1 { &GEN1 } else { &GEN3 };
    cell.get_or_init(|| crate::text::dex_tables(dex))
}

/// A log as `BattleStream` writes it ([`encode_log`]).
fn lines(b: &Battle, msgs: &[Msg]) -> String {
    encode_log(msgs, tables(b.dex), &Names(b))
}

std::thread_local! {
    /// The lines a full log spilled since the last `update` block.
    static SPILLED: core::cell::RefCell<Vec<Msg>> = const { core::cell::RefCell::new(Vec::new()) };
}

/// The tests' [`crate::state::LogSpill`]: [`Collect::update`] puts these
/// lines in front of the rest of the block.
fn spill_to_test(_: &Battle, lines: &[Msg]) {
    SPILLED.with(|s| s.borrow_mut().extend_from_slice(lines));
}

/// The blocks `send_updates` produced: `(header, lines or request)`.
#[derive(Default)]
struct Collect {
    blocks: Vec<(String, Vec<Msg>, Option<Request>)>,
}

impl Sink for Collect {
    fn update(&mut self, lines: &[Msg]) {
        let mut block = SPILLED.with(|s| core::mem::take(&mut *s.borrow_mut()));
        block.extend_from_slice(lines);
        self.blocks.push(("update".into(), block, None));
    }
    fn side_line(&mut self, side: SideId, line: &Msg) {
        self.blocks.push((format!("sideupdate {}", side.as_str()), std::vec![*line], None));
    }
    fn request(&mut self, side: SideId, request: &Request) {
        self.blocks.push((format!("sideupdate {}", side.as_str()), Vec::new(), Some(request.clone())));
    }
}

fn flush(b: &mut Battle) -> Vec<(String, String)> {
    let mut c = Collect::default();
    b.send_updates(&mut c);
    c.blocks
        .into_iter()
        .map(|(h, msgs, req)| {
            let body = match req {
                Some(r) => format!("|request|{}", encode_request(&r, tables(b.dex), &Names(b))),
                None => lines(b, &msgs),
            };
            (h, body)
        })
        .collect()
}

// ---- the replay --------------------------------------------------------------

#[derive(Default, Debug)]
struct Tally {
    battles: usize,
    blocks: usize,
    requests: usize,
    move_requests: usize,
    switch_requests: usize,
    wait_requests: usize,
    errors: usize,
}

/// `>start` and the `>player`s of a fixture's first chunk, unchecked, with
/// the stub pipeline installed.
fn setup(cmds: &[String]) -> Box<Battle> {
    let v: serde_json::Value = serde_json::from_str(cmds[0].strip_prefix("start ").unwrap()).unwrap();
    let s: Vec<u16> = v["seed"].as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as u16).collect();
    let spec = FormatSpec::parse(v["formatid"].as_str().unwrap()).unwrap();
    let mut b = Box::new(Battle::from_start(spec, [s[0], s[1], s[2], s[3]], CLOCK));
    b.move_hook = Some(stub_run_move);
    b.lax_handlers = true;
    b.spill = Some(spill_to_test);
    for c in &cmds[1..] {
        let (side, json) = c.strip_prefix("player ").unwrap().split_once(' ').unwrap();
        let v: serde_json::Value = serde_json::from_str(json).unwrap();
        let team = parse_team(v["team"].as_str().unwrap());
        b.set_player(SideId::parse(side).unwrap(), v["name"].as_str().unwrap(), "", "", Some(&team)).unwrap();
    }
    b
}

/// Replays one fixture; `Err` describes the first divergence.
fn replay(f: &Fixture, tally: &mut Tally) -> Result<Box<Battle>, String> {
    let mut battle: Option<Box<Battle>> = None;
    for (ci, chunk) in f.chunks.iter().enumerate() {
        for cmd in &chunk.commands {
            let (kind, rest) = cmd.split_once(' ').unwrap();
            match kind {
                "start" => {
                    let v: serde_json::Value = serde_json::from_str(rest).unwrap();
                    let spec = FormatSpec::parse(v["formatid"].as_str().unwrap()).unwrap();
                    let s: Vec<u16> = v["seed"].as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as u16).collect();
                    let mut b = Box::new(Battle::from_start(spec, [s[0], s[1], s[2], s[3]], CLOCK));
                    if f.kind != "moves" {
                        b.move_hook = Some(stub_run_move);
                        b.lax_handlers = true;
                    }
                    b.spill = Some(spill_to_test);
                    battle = Some(b);
                }
                "player" => {
                    let (side, json) = rest.split_once(' ').unwrap();
                    let v: serde_json::Value = serde_json::from_str(json).unwrap();
                    let team = parse_team(v["team"].as_str().unwrap());
                    let side = SideId::parse(side).unwrap();
                    let b = battle.as_mut().unwrap();
                    b.set_player(side, v["name"].as_str().unwrap(), "", "", Some(&team)).map_err(|e| format!("{e:?}"))?;
                }
                "p1" | "p2" => {
                    let b = battle.as_mut().unwrap();
                    let cmd = Command::Choose { side: SideId::parse(kind).unwrap(), choice: parse_choice(rest) };
                    b.step(&cmd).unwrap();
                }
                other => panic!("command {other}"),
            }
        }
        let b = battle.as_mut().unwrap();
        let ours = flush(b);
        let ctx = |i: usize| format!("{} gen{} #{} chunk {ci} ({:?}) block {i}", f.kind, f.gen, f.index, chunk.commands);
        if ours.len() != chunk.expected.len() {
            let got: Vec<String> = ours.iter().map(|(h, b)| format!(">{h}\n{b}")).collect();
            return Err(format!("{}: {} blocks, PS has {}\nours:\n{}", ctx(0), ours.len(), chunk.expected.len(), got.join("\n")));
        }
        for (i, ((h, body), exp)) in ours.iter().zip(&chunk.expected).enumerate() {
            tally.blocks += 1;
            if body.starts_with("|request|") {
                tally.requests += 1;
                if body.contains("\"forceSwitch\"") {
                    tally.switch_requests += 1;
                } else if body.starts_with("|request|{\"wait\"") {
                    tally.wait_requests += 1;
                } else {
                    tally.move_requests += 1;
                }
            }
            if body.starts_with("|error|") {
                tally.errors += 1;
            }
            match exp {
                Expected::Full { header, body: want } => {
                    if h != header || body != want {
                        let (a, b) = first_diff(want, body);
                        return Err(format!("{}: >{header} differs\n  PS:   {a}\n  ours: {b}\nours (whole):\n>{h}\n{body}", ctx(i)));
                    }
                }
                Expected::Hash { header, hash } => {
                    if h != header || fnv64(body) != *hash {
                        return Err(format!("{}: >{header} hash differs\nours:\n>{h}\n{body}", ctx(i)));
                    }
                }
            }
        }
    }
    tally.battles += 1;
    Ok(battle.unwrap())
}

fn first_diff(a: &str, b: &str) -> (String, String) {
    for (x, y) in a.split('\n').zip(b.split('\n')) {
        if x != y {
            return (x.to_string(), y.to_string());
        }
    }
    (format!("({} lines)", a.split('\n').count()), format!("({} lines)", b.split('\n').count()))
}

/// The opening lines the server writes with no handler ported: anything
/// else between `|start` and `|turn|1` comes from a lead's switch-in
/// ability.
fn opening_is_handler_free(f: &Fixture) -> bool {
    let Some(Expected::Full { body, .. }) = f.chunks[0].expected.first() else { return true };
    let mut in_start = false;
    for line in body.split('\n') {
        if line == "|start" {
            in_start = true;
            continue;
        }
        if line.starts_with("|turn|") {
            break;
        }
        if in_start && !line.starts_with("|split|") && !line.starts_with("|switch|") {
            return false;
        }
    }
    true
}

/// A lead's ability acts in the opening through a handler PS's data has
/// and the registry lacks: a switch-in line (Intimidate, Pressure, Sand
/// Stream, Trace...) or a trap that shows in the request (Magnet Pull,
/// Arena Trap, Shadow Tag).
fn lead_has_unported_opening_ability(b: &Battle) -> bool {
    lead_has_opening_ability(b, false)
}

/// A lead's ability has a handler that acts in the opening (see
/// [`lead_has_unported_opening_ability`]); `ported` picks the leads whose
/// ability the registry carries, or those whose ability it lacks.
fn lead_has_opening_ability(b: &Battle, ported: bool) -> bool {
    (0..2u8).any(|s| {
        // The set's ability: the battle has started, and Trace may have
        // replaced it already.
        let m = b.active_at(s, 0).unwrap();
        let a = b.mon(m).base_ability;
        !a.is_none()
            && b.registry.get(EffectId::Ability(a)).is_some() == ported
            && b.dex.ability_def(a).handlers.iter().any(|h| {
                matches!(
                    h.name(),
                    "onStart"
                        | "onSwitchIn"
                        | "onAfterEntryHazard"
                        | "onFoeTrapPokemon"
                        | "onFoeMaybeTrapPokemon"
                        | "onAnyTrapPokemon"
                        | "onAnyMaybeTrapPokemon"
                )
            })
    })
}

fn run_kind(gen: u8, kinds: &[&str]) -> Tally {
    let mut tally = Tally::default();
    let mut failures = Vec::new();
    for f in load(gen).iter().filter(|f| kinds.contains(&f.kind.as_str())) {
        if f.kind == "prefix" && !opening_is_handler_free(f) {
            continue;
        }
        if let Err(e) = replay(f, &mut tally) {
            failures.push(e);
        }
    }
    for e in failures.iter().take(3) {
        std::eprintln!("{e}\n");
    }
    assert!(failures.is_empty(), "{} of {} gen {gen} {kinds:?} battles diverge", failures.len(), failures.len() + tally.battles);
    std::eprintln!("gen {gen} {kinds:?}: {tally:?}");
    tally
}

// ---- the tests ---------------------------------------------------------------

/// The P1-4 corpus up to the first move: header, leads, `|turn|1`, both
/// requests. Gen 3 battles whose leads announce an ability on switch-in
/// wait for the ability ports; the skip is checked to be exactly that.
#[test]
fn opening_matches_ps_gen1() {
    let t = run_kind(1, &["prefix"]);
    assert_eq!(t.battles, 60);
    assert_eq!(t.move_requests, 120);
}

#[test]
fn opening_matches_ps_gen3() {
    let mut skipped_ability = 0;
    let mut compared = 0;
    let mut with_forme = 0;
    let mut with_ability = 0;
    let mut tally = Tally::default();
    for f in load(3).iter().filter(|f| f.kind == "prefix") {
        if lead_has_unported_opening_ability(&setup(&f.chunks[0].commands)) {
            skipped_ability += 1;
            continue;
        }
        // Everything else in the opening is handler-free, or a ported lead
        // ability's.
        let b = setup(&f.chunks[0].commands);
        assert!(
            opening_is_handler_free(f) || lead_has_opening_ability(&b, true),
            "gen 3 #{}: an opening line no lead ability explains",
            f.index
        );
        with_ability += usize::from(lead_has_opening_ability(&b, true));
        replay(f, &mut tally).unwrap_or_else(|e| panic!("{e}"));
        compared += 1;
        with_forme += usize::from(team_has_cosmetic_forme(f));
    }
    std::eprintln!(
        "gen 3 openings: {compared} match ({with_forme} with an Unown forme, {with_ability} with a ported lead ability), {skipped_ability} wait for lead abilities"
    );
    assert_eq!(compared + skipped_ability, 60);
    // Every lead ability of the corpus that acts in the opening is ported
    // (G3-A2: Intimidate, Pressure, Trace, the weather abilities, the
    // trapping abilities).
    assert_eq!(compared, 60);
    assert_eq!(with_ability, 14);
    // An `Unown-X` details string in the requests (and the switch line
    // when it leads).
    assert_eq!(with_forme, 5);
    assert_eq!(tally.move_requests, 2 * compared);
}

/// A set whose species is a cosmetic forme (Unown's letters).
fn team_has_cosmetic_forme(f: &Fixture) -> bool {
    let b = Battle::new(&ps_dex::GEN3, crate::mods::registry_for(&ps_dex::GEN3), [1, 2, 3, 4], Trunc::Math);
    f.chunks[0].commands[1..].iter().any(|c| {
        let (_, json) = c.strip_prefix("player ").unwrap().split_once(' ').unwrap();
        let v: serde_json::Value = serde_json::from_str(json).unwrap();
        parse_team(v["team"].as_str().unwrap()).iter().any(|p| b.unpack_set(p).unwrap().forme != 0)
    })
}

/// Every turn a switch, to PS's turn limit: `|message|It is turn 1000...`
/// and `|tie|` after turn 1000, with the auto-tie warnings on the way.
#[test]
fn switch_only_battles_tie_at_the_turn_limit() {
    for gen in [1, 3] {
        let t = run_kind(gen, &["switch"]);
        assert_eq!(t.battles, 2);
        for f in load(gen).iter().filter(|f| f.kind == "switch") {
            let b = replay(f, &mut Tally::default()).unwrap();
            assert!(b.ended && b.winner == Some(None), "gen {gen} #{}: no tie", f.index);
            assert_eq!(b.turn, 1001);
        }
    }
}

/// Gen 1's no-progress check: every Pokemon knows only Transform, at the
/// start and after the last other Pokemon faints.
#[test]
fn gen1_no_progress_ties() {
    let t = run_kind(1, &["noprog"]);
    assert_eq!(t.battles, 2);
    for f in load(1).iter().filter(|f| f.kind == "noprog") {
        let b = replay(f, &mut Tally::default()).unwrap();
        assert_eq!(b.winner, Some(None));
    }
}

/// Faints, forced switches, wait requests, the gen 1 queue clear and the
/// gen 3 cancelAction, wins, double KOs, choice errors and undo, through
/// the stub pipeline; every request byte for byte.
#[test]
fn stub_battles_match_ps() {
    for gen in [1, 3] {
        let t = run_kind(gen, &["hook", "dko"]);
        assert_eq!(t.battles, 25);
        assert!(t.switch_requests > 10 && t.wait_requests > 10 && t.errors > 0, "gen {gen}: {t:?}");
    }
}

/// The double KO of both last Pokemon is a tie in gens 1-4 (`checkWin`
/// with no side), whichever of the tied Pokemon the PRNG lets move first.
#[test]
fn double_ko_ties_either_way() {
    for gen in [1, 3] {
        let f = load(gen).into_iter().find(|f| f.kind == "dko").unwrap();
        // Replay the setup, then force the speed tie each way.
        for first in [0u32, u32::MAX] {
            let mut b = setup(&f.chunks[0].commands);
            flush(&mut b);
            b.prng.push_overrides([first]);
            b.step(&Command::Choose { side: SideId::P1, choice: ps_protocol::Choice::Move(3) }).unwrap();
            b.step(&Command::Choose { side: SideId::P2, choice: ps_protocol::Choice::Move(3) }).unwrap();
            assert!(b.prng.override_is_empty());
            let out = flush(&mut b);
            let log = &out[0].1;
            let mover = if first == 0 { "p1a" } else { "p2a" };
            assert!(log.contains(&format!("|move|{mover}: Electrode|Explosion")), "gen {gen}: {log}");
            assert!(log.ends_with("|faint|p1a: Electrode\n|faint|p2a: Electrode\n|\n|tie") ||
                log.ends_with("|faint|p2a: Electrode\n|faint|p1a: Electrode\n|\n|tie"), "gen {gen}: {log}");
            assert_eq!(b.winner, Some(None));
        }
    }
}

/// A command whose log outgrows [`crate::state::LOG_CAP`] several times
/// over: the spill takes the oldest lines as the log fills, and the
/// `update` block comes out whole and in order. The hook writes the move,
/// 300 numbered `|debug|` lines, then the stub's damage.
#[test]
fn a_long_update_block_spills_in_order() {
    fn chatty_run_move(b: &mut Battle, mv: ActionMove, pokemon: MonRef, _target_loc: i8) {
        let user = b.ident(pokemon);
        b.add(Msg::Move { user, mv: MoveNum(mv.id().unwrap().0), target: None, kw: Kw::NONE });
        for i in 0..300 {
            b.debug(Text::with1(TextId::BasePower, TextArg::int(i)));
        }
        let target = b.active_at(1 - pokemon.side, 0).unwrap();
        b.direct_damage(1, Some(target), Some(pokemon), EffectId::None);
    }
    let f = load(1).into_iter().find(|f| f.kind == "hook").unwrap();
    let mut b = setup(&f.chunks[0].commands);
    flush(&mut b);
    b.move_hook = Some(chatty_run_move);
    b.step(&Command::Choose { side: SideId::P1, choice: ps_protocol::Choice::Move(1) }).unwrap();
    b.step(&Command::Choose { side: SideId::P2, choice: ps_protocol::Choice::Move(1) }).unwrap();
    assert!(b.spilled);
    let spilled = SPILLED.with(|s| s.borrow().len());
    assert!(spilled >= 600 - crate::state::LOG_CAP, "{spilled} lines spilled");
    let out = flush(&mut b);
    assert!(!b.spilled && SPILLED.with(|s| s.borrow().is_empty()));
    assert_eq!(out[0].0, "update");
    let lines: Vec<&str> = out[0].1.split('\n').collect();
    assert_eq!(lines[0], "|");
    assert!(lines.last().unwrap().starts_with("|turn|2"), "{}", lines.last().unwrap());
    // Each move, then its 300 lines in order, then its damage.
    let mut at = 0;
    for _ in 0..2 {
        at += lines[at..].iter().position(|l| l.starts_with("|move|")).expect("a move line");
        for i in 0..300 {
            assert_eq!(lines[at + 1 + i], format!("|debug|BP: {i}"));
        }
        at += 301;
        assert!(lines[at].starts_with("|split|p"), "{}", lines[at]);
        assert!(lines[at + 1].starts_with("|-damage|"), "{}", lines[at + 1]);
    }
    // Then both requests, as after any turn.
    assert_eq!(out.len(), 3);
    assert!(out[1].1.starts_with("|request|") && out[2].1.starts_with("|request|"));
}

/// A full log keeps the lines from the last `|move|` on (`attrLastMove`
/// may still edit it) and spills the ones before.
#[test]
fn a_full_log_keeps_the_last_move_line() {
    let f = load(1).into_iter().find(|f| f.kind == "hook").unwrap();
    let mut b = setup(&f.chunks[0].commands);
    flush(&mut b);
    let user = b.ident(MonRef { side: 0, index: 0 });
    let debug = |i| Msg::Debug(Text::with1(TextId::BasePower, TextArg::int(i)));
    for i in 0..10 {
        b.add(debug(i));
    }
    b.last_move_line = Some(10);
    b.add(Msg::Move { user, mv: MoveNum(1), target: None, kw: Kw::NONE });
    for i in 11..crate::state::LOG_CAP as i32 + 1 {
        b.add(debug(i));
    }
    assert_eq!(SPILLED.with(|s| s.borrow().clone()), (0..10).map(debug).collect::<Vec<_>>());
    assert_eq!(b.last_move_line, Some(0));
    assert!(matches!(b.log.as_slice()[0], Msg::Move { .. }));
    assert_eq!(b.log.as_slice().len(), crate::state::LOG_CAP - 9);
    flush(&mut b);
}

/// Replays `moves_gen<gen>.txt.gz` (`tools/ps-diff/move-fixtures.js`):
/// whole battles through the real move pipeline, to the end, every line
/// and every request as PS has them. The teams carry only `ported(gen)`
/// moves and abilities and no items, and the recorder's list (the first
/// line, `#ported`) is that list less its species. Gen 1's header predates the `kind:`
/// prefix: its bare ids are the list's moves. Returns the number of
/// battles.
fn move_battles_match_ps(gen: u8) -> usize {
    let file = format!("moves_gen{gen}.txt.gz");
    let text = fixture_text(&file);
    let header = text.lines().next().unwrap().strip_prefix("#ported ").expect("#ported header");
    let legacy = !header.contains(':');
    let listed: Vec<String> =
        header.split(',').map(|e| if legacy { format!("move:{e}") } else { e.to_string() }).collect();
    // Species are in the list for the harness's wide teams; the recorder
    // lists only what it derives (moves, and gen 3's abilities).
    let ported: Vec<&str> = crate::mods::ported(gen)
        .iter()
        .copied()
        .filter(|e| !e.starts_with("species:") && (!legacy || e.starts_with("move:")))
        .collect();
    assert_eq!(listed, ported, "move-fixtures.js derives another ported list");

    let fixtures = load_file(&file);
    let mut tally = Tally::default();
    let mut failures = Vec::new();
    let (mut wins, mut ties, mut turns) = (0, 0, 0u32);
    for f in &fixtures {
        // Every team move and ability is a ported one; no items.
        for c in &f.chunks[0].commands[1..] {
            let (_, json) = c.strip_prefix("player ").unwrap().split_once(' ').unwrap();
            let v: serde_json::Value = serde_json::from_str(json).unwrap();
            for set in parse_team(v["team"].as_str().unwrap()).iter() {
                for m in set.moves.iter().filter(|m| !m.is_empty()) {
                    let id = format!("move:{}", crate::mods::PsId(m.as_str()));
                    assert!(listed.contains(&id), "#{}: {id} is not ported", f.index);
                }
                assert!(set.item.is_empty(), "#{}: an item", f.index);
                let ability = crate::mods::PsId(set.ability.as_str()).to_string();
                if gen == 3 {
                    assert!(listed.contains(&format!("ability:{ability}")), "#{}: ability {ability} is not ported", f.index);
                }
            }
        }
        match replay(f, &mut tally) {
            Ok(b) => {
                assert!(b.ended, "#{}: the battle did not end", f.index);
                match b.winner {
                    Some(Some(_)) => wins += 1,
                    _ => ties += 1,
                }
                turns += u32::from(b.turn);
            }
            Err(e) => failures.push(e),
        }
    }
    for e in failures.iter().take(3) {
        std::eprintln!("{e}\n");
    }
    assert!(failures.is_empty(), "{} of {} gen {gen} move battles diverge", failures.len(), fixtures.len());
    std::eprintln!("gen {gen} move battles: {tally:?}, {wins} wins, {ties} ties, {turns} turns");
    assert_eq!(tally.battles, fixtures.len());
    fixtures.len()
}

/// Whole Gen 1 battles: 320 random battles and two for gen 1's 999 / 1
/// stat bounce.
#[test]
fn move_battles_match_ps_gen1() {
    assert_eq!(move_battles_match_ps(1), 322);
}

/// Whole Gen 3 battles through gen 3's pipeline (P1-9).
#[test]
fn move_battles_match_ps_gen3() {
    assert_eq!(move_battles_match_ps(3), GEN3_MOVE_BATTLES);
}

/// The battles in `moves_gen3.txt.gz`.
const GEN3_MOVE_BATTLES: usize = 330;

/// A move with handlers that are not ported panics `unported: <id>`, the
/// line the CLI reports. The battle runs on gen 1's registry minus
/// Soft-Boiled, so the test does not depend on which batches have landed.
#[test]
#[should_panic(expected = "unported: softboiled")]
fn unported_moves_panic() {
    let team = "Chansey|||NoAbility|softboiled||255,255,255,255,255,255||30,30,30,30,30,30||70|";
    let cmds = [
        String::from(r#"start {"formatid":"gen1customgame@@@Sleep Clause Mod,Freeze Clause Mod","seed":[1,2,3,4]}"#),
        format!(r#"player p1 {{"name":"Player 1","team":"{team}"}}"#),
        format!(r#"player p2 {{"name":"Player 2","team":"{team}"}}"#),
    ];
    let mut b = setup(&cmds);
    b.move_hook = None;
    let softboiled = EffectId::Move(ps_dex::gen1::moves::SOFTBOILED);
    let entries: Vec<_> = b.registry.entries.iter().copied().filter(|(id, _)| *id != softboiled).collect();
    b.registry = Box::leak(Box::new(crate::effect::Registry { entries: entries.leak() }));
    b.step(&Command::Choose { side: SideId::P1, choice: ps_protocol::Choice::Move(1) }).unwrap();
    b.step(&Command::Choose { side: SideId::P2, choice: ps_protocol::Choice::Move(1) }).unwrap();
}

/// The same in gen 3, on gen 3's registry minus Facade (which has an
/// `onBasePower` handler): every gen 3 effect has a port now.
#[test]
#[should_panic(expected = "unported: facade")]
fn unported_moves_panic_gen3() {
    let team = "Snorlax|||illuminate|facade||85,85,85,85,85,85|M||||";
    let cmds = [
        String::from(r#"start {"formatid":"gen3customgame","seed":[1,2,3,4]}"#),
        format!(r#"player p1 {{"name":"Player 1","team":"{team}"}}"#),
        format!(r#"player p2 {{"name":"Player 2","team":"{team}"}}"#),
    ];
    let mut b = setup(&cmds);
    b.move_hook = None;
    let facade = EffectId::Move(ps_dex::gen3::moves::FACADE);
    let entries: Vec<_> = b.registry.entries.iter().copied().filter(|(id, _)| *id != facade).collect();
    b.registry = Box::leak(Box::new(crate::effect::Registry { entries: entries.leak() }));
    b.step(&Command::Choose { side: SideId::P1, choice: ps_protocol::Choice::Move(1) }).unwrap();
    b.step(&Command::Choose { side: SideId::P2, choice: ps_protocol::Choice::Move(1) }).unwrap();
}

/// Plays a 1v1 gen 1 battle with the real pipeline, both sides choosing
/// `move 1` until it ends (or 60 turns), and returns the whole log.
fn play_gen1(p1: &str, p2: &str, seed: [u16; 4]) -> String {
    let start = format!(
        r#"start {{"formatid":"gen1customgame@@@Sleep Clause Mod,Freeze Clause Mod","seed":[{},{},{},{}]}}"#,
        seed[0], seed[1], seed[2], seed[3]
    );
    let cmds = [
        start,
        format!(r#"player p1 {{"name":"Player 1","team":"{p1}"}}"#),
        format!(r#"player p2 {{"name":"Player 2","team":"{p2}"}}"#),
    ];
    let mut b = setup(&cmds);
    b.move_hook = None;
    let mut log = String::new();
    let mut take = |b: &mut Battle| {
        for (h, body) in flush(b) {
            if h == "update" {
                log.push_str(&body);
                log.push('\n');
            }
        }
    };
    take(&mut b);
    while !b.ended && b.turn < 60 {
        b.step(&Command::Choose { side: SideId::P1, choice: ps_protocol::Choice::Move(1) }).unwrap();
        b.step(&Command::Choose { side: SideId::P2, choice: ps_protocol::Choice::Move(1) }).unwrap();
        take(&mut b);
    }
    log
}

/// G1-2's conditions run through the pipeline: the partial trap and its
/// lock, Hyper Beam's recharge, Rage's build-up and Bide.
#[test]
fn gen1_trapping_recharge_rage_and_bide() {
    let set = |species: &str, mv: &str| format!("{species}|||NoAbility|{mv}||255,255,255,255,255,255||30,30,30,30,30,30||90|");
    let log = play_gen1(&set("Arbok", "wrap"), &set("Snorlax", "tackle"), [1, 2, 3, 4]);
    assert!(log.contains("|cant|p2a: Snorlax|partiallytrapped"), "{log}");
    assert!(log.contains("|move|p1a: Arbok|Wrap|p2a: Snorlax|[from] Wrap"), "{log}");

    let log = play_gen1(&set("Tauros", "hyperbeam"), &set("Chansey", "rage"), [5, 6, 7, 8]);
    assert!(log.contains("|-mustrecharge|p1a: Tauros"), "{log}");
    assert!(log.contains("|cant|p1a: Tauros|recharge"), "{log}");
    assert!(log.contains("|-boost|p2a: Chansey|atk|1|[from] Rage"), "{log}");
    assert!(log.contains("|move|p2a: Chansey|Rage|p1a: Tauros|[from] Rage"), "{log}");

    let log = play_gen1(&set("Chansey", "bide"), &set("Rattata", "tackle"), [9, 10, 11, 12]);
    assert!(log.contains("|-start|p1a: Chansey|Bide"), "{log}");
    assert!(log.contains("|-activate|p1a: Chansey|Bide"), "{log}");
    assert!(log.contains("|-end|p1a: Chansey|Bide"), "{log}");
}

/// G1-1's effects run through the pipeline: Toxic's counter and its
/// `[from] psn` line, confusion, Rest's sleep and silent heal, paralysis,
/// and Dream Eater against an awake target.
#[test]
fn gen1_statuses_confusion_and_rest() {
    let set = |species: &str, mv: &str| format!("{species}|||NoAbility|{mv}||255,255,255,255,255,255||30,30,30,30,30,30||90|");
    let log = play_gen1(&set("Lapras", "toxic"), &set("Chansey", "tackle"), [1, 2, 3, 4]);
    assert!(log.contains("|-status|p2a: Chansey|tox"), "{log}");
    // `this.damage(damage, pokemon, pokemon)`: no `[of]`.
    assert!(log.contains(" tox|[from] psn\n"), "{log}");
    let log = play_gen1(&set("Weezing", "poisongas"), &set("Chansey", "tackle"), [1, 2, 3, 4]);
    // Plain poison's `this.damage(damage, pokemon)` takes the foe as its
    // source.
    assert!(log.contains(" psn|[from] psn|[of] p1a: Weezing\n"), "{log}");

    let log = play_gen1(&set("Gengar", "confuseray"), &set("Tauros", "bodyslam"), [5, 6, 7, 8]);
    assert!(log.contains("|-start|p2a: Tauros|confusion"), "{log}");
    assert!(log.contains("|-activate|p2a: Tauros|confusion"), "{log}");

    let log = play_gen1(&set("Snorlax", "rest"), &set("Tauros", "tackle"), [9, 10, 11, 12]);
    assert!(log.contains("|-status|p1a: Snorlax|slp|[from] move: Rest"), "{log}");
    assert!(log.contains("|-heal|p1a: Snorlax|") && log.contains(" slp|[silent]\n"), "{log}");
    assert!(log.contains("|cant|p1a: Snorlax|slp"), "{log}");

    let log = play_gen1(&set("Jolteon", "thunderwave"), &set("Persian", "slash"), [13, 14, 15, 16]);
    assert!(log.contains("|-status|p2a: Persian|par"), "{log}");

    let log = play_gen1(&set("Hypno", "dreameater"), &set("Tauros", "tackle"), [17, 18, 19, 20]);
    assert!(log.contains("|-immune|p2a: Tauros"), "{log}");
}

/// G1-5's handlers run through the pipeline: Conversion's `typechange`
/// line with both of the target's types, Splash's `-nothing`, Pay Day's
/// `-fieldactivate`, Soft-Boiled's plain `-heal`, Super Fang's half-HP
/// damage, and High Jump Kick's 1 HP crash against a Ghost. The expected
/// lines are PS's for the same teams and seeds.
#[test]
fn gen1_simple_handlers() {
    let set = |species: &str, mv: &str| format!("{species}|||NoAbility|{mv}||255,255,255,255,255,255||30,30,30,30,30,30||90|");
    let log = play_gen1(&set("Gastly", "conversion"), &set("Charizard", "splash"), [1, 2, 3, 4]);
    assert!(log.contains("|move|p2a: Charizard|Splash|p2a: Charizard\n|-nothing\n"), "{log}");
    assert!(
        log.contains("|-start|p1a: Gastly|typechange|Fire/Flying|[from] move: Conversion|[of] p2a: Charizard\n"),
        "{log}"
    );

    let log = play_gen1(&set("Persian", "payday"), &set("Chansey", "softboiled"), [5, 6, 7, 8]);
    assert!(log.contains("|-damage|p2a: Chansey|545/633\n|-fieldactivate|move: Pay Day\n"), "{log}");
    assert!(log.contains("|move|p2a: Chansey|Soft-Boiled|p2a: Chansey\n|split|p2\n|-heal|p2a: Chansey|633/633\n"), "{log}");

    let log = play_gen1(&set("Hitmonlee", "highjumpkick"), &set("Gengar", "superfang"), [9, 10, 11, 12]);
    assert!(log.contains("|-damage|p1a: Hitmonlee|137/273\n"), "{log}");
    assert!(log.contains("|-immune|p2a: Gengar\n|split|p1\n|-damage|p1a: Hitmonlee|136/273\n"), "{log}");
}

/// `>forcetie` and `>forcewin`.
#[test]
fn force_win_and_tie() {
    let f = load(3).into_iter().find(|f| f.kind == "dko").unwrap();
    let mut b = setup(&f.chunks[0].commands);
    flush(&mut b);
    b.step(&Command::ForceTie).unwrap();
    assert_eq!(flush(&mut b), std::vec![("update".to_string(), "|\n|tie".to_string())]);
    // The game is over: choices are refused.
    b.step(&Command::Choose { side: SideId::P1, choice: ps_protocol::Choice::Move(1) }).unwrap();
    assert_eq!(
        flush(&mut b),
        std::vec![("sideupdate p1".to_string(), "|error|[Invalid choice] Can't do anything: The game is over".to_string())]
    );
    b.step(&Command::ForceWin(SideId::P1)).unwrap();
    assert!(flush(&mut b).is_empty(), "win after the end does nothing");
}

/// Gen 1's clauses refuse a second sleeping / frozen foe.
#[test]
fn sleep_and_freeze_clauses() {
    let f = load(1).into_iter().find(|f| f.kind == "prefix").unwrap();
    let mut b = replay(&f, &mut Tally::default()).unwrap();
    let slp = b.dex.condition_id("slp").unwrap();
    let frz = b.dex.condition_id("frz").unwrap();
    let a = b.active_at(0, 0).unwrap();
    let bench = b.pokemon_at(1, 1);
    let foe = b.active_at(1, 0).unwrap();
    // A foe already asleep (put to sleep by p1): the next sleep fails.
    b.mon_mut(bench).status = slp;
    b.mon_mut(bench).status_state.source = Some(a);
    assert_eq!(b.set_status(foe, slp, Some(a), EffectId::None, false), Relay::FAIL);
    let out = flush(&mut b);
    assert!(out[0].1.contains("|-message|Sleep Clause Mod activated.\n|-hint|Sleep Clause Mod prevents"), "{out:?}");
    // Self-inflicted sleep (Rest) does not count.
    b.mon_mut(bench).status_state.source = Some(bench);
    assert_eq!(b.set_status(foe, slp, Some(a), EffectId::None, false), Relay::TRUE);
    // Freeze: any frozen foe blocks the next.
    b.mon_mut(bench).status = frz;
    let foe2 = b.pokemon_at(1, 2);
    b.mon_mut(foe).status = ps_dex::ConditionId::NONE;
    let _ = foe2;
    assert_eq!(b.set_status(foe, frz, Some(a), EffectId::None, false), Relay::FAIL);
    let out = flush(&mut b);
    assert!(out.iter().any(|(_, l)| l.contains("|-message|Freeze Clause activated.")), "{out:?}");
}

#[test]
fn format_rules_and_trunc() {
    let b = Box::new(Battle::from_start(FormatSpec::gen1_harness(), [1, 2, 3, 4], CLOCK));
    assert!(b.has_rule(Rule::CartridgeTrunc) && b.has_rule(Rule::SleepClauseMod) && b.has_rule(Rule::DesyncClauseMod));
    assert_eq!(b.trunc, Trunc::Cartridge);
    // The two clauses are on the field, sleep first (rule table order).
    assert_eq!(
        b.field.pseudo_weather.ids(),
        &[EffectId::Rule(Rule::SleepClauseMod), EffectId::Rule(Rule::FreezeClauseMod)]
    );
    // `!Cartridge Trunc` turns the plain math back on and nothing else.
    let math = FormatSpec::gen1_harness().with_repeal(ps_protocol::RuleId::CartridgeTrunc).unwrap();
    let b = Box::new(Battle::from_start(math, [1, 2, 3, 4], CLOCK));
    assert!(!b.has_rule(Rule::CartridgeTrunc) && b.has_rule(Rule::SleepClauseMod) && b.has_rule(Rule::DesyncClauseMod));
    assert_eq!(b.trunc, Trunc::Math);
    let rb = FormatSpec::parse("gen1randombattle").unwrap();
    let b = Box::new(Battle::from_start(rb, [1, 2, 3, 4], CLOCK));
    assert!(b.has_rule(Rule::CartridgeTrunc));
    let rb = FormatSpec::parse("gen1randombattle@@@!Cartridge Trunc").unwrap();
    let b = Box::new(Battle::from_start(rb, [1, 2, 3, 4], CLOCK));
    assert!(!b.has_rule(Rule::CartridgeTrunc) && b.has_rule(Rule::EndlessBattleClause));
    assert_eq!(b.trunc, Trunc::Math);
    let b = Box::new(Battle::from_start(FormatSpec::gen3_harness(), [1, 2, 3, 4], CLOCK));
    assert!(!b.has_rule(Rule::CartridgeTrunc) && !b.has_rule(Rule::DesyncClauseMod) && b.has_rule(Rule::CancelMod));
    assert_eq!(b.trunc, Trunc::Math);
    assert!(b.field.pseudo_weather.is_empty());
}

/// PS throws when a repeal finds nothing to repeal (`getRuleTable`).
#[test]
#[should_panic(expected = "did nothing because \"Cartridge Trunc\" is not in effect")]
fn repealing_a_rule_not_in_effect_throws() {
    let spec = FormatSpec::gen3_harness().with_repeal(ps_protocol::RuleId::CartridgeTrunc).unwrap();
    let _ = Box::new(Battle::from_start(spec, [1, 2, 3, 4], CLOCK));
}

/// Design section 6: at most 16 KB per battle. The host's pointers are
/// wider than the device's, so the host size bounds the device's (14,632
/// bytes on thumbv8m when this was written).
#[test]
fn battle_fits_the_ram_budget() {
    let size = core::mem::size_of::<Battle>();
    std::eprintln!("size_of::<Battle>() = {size}");
    assert!(size <= 16 * 1024, "{size}");
}

// ---- the ActiveMove arena's longest chains (crate::actions::MOVE_CAP) ------

/// A started battle of `format` with the real pipeline, its opening
/// flushed.
fn chain_battle(format: &str, p1: &str, p2: &str) -> Box<Battle> {
    let cmds = [
        format!(r#"start {{"formatid":"{format}","seed":[1,2,3,4]}}"#),
        format!(r#"player p1 {{"name":"Player 1","team":"{p1}"}}"#),
        format!(r#"player p2 {{"name":"Player 2","team":"{p2}"}}"#),
    ];
    let mut b = setup(&cmds);
    b.move_hook = None;
    b.lax_handlers = false;
    flush(&mut b);
    b
}

/// The forced `next()` that makes `sample` of an `n`-long list pick `i`
/// (`random(n)` is `floor(next * n / 2^32)`).
fn pick(i: usize, n: usize) -> u32 {
    ((i as u64) << 32).div_ceil(n as u64) as u32
}

/// Metronome's pool (`flags.metronome`, sorted by `num`).
fn metronome_pool(dex: &'static Dex) -> Vec<MoveId> {
    let mut pool: Vec<MoveId> =
        (0..dex.moves.len()).map(|i| MoveId(i as u16)).filter(|&m| dex.move_def(m).flags.has("metronome")).collect();
    pool.sort_by_key(|&m| dex.move_def(m).num);
    pool
}

/// Gen 1's unbounded chain, `k` turns of the cycle deep: Clefable's Mirror
/// Move copies the foe's `lastMove`, Metronome; that Metronome rolls
/// Mirror Move `k` times, each copying Metronome again, then Haze. Returns
/// the battle and the action's log.
fn gen1_mirror_metronome_cycle(k: usize) -> (Box<Battle>, String) {
    use ps_dex::gen1::moves;
    let set = |species: &str, mv: &str| format!("{species}|||NoAbility|{mv}||255,255,255,255,255,255||30,30,30,30,30,30||90|");
    let mut b = chain_battle("gen1customgame@@@Sleep Clause Mod,Freeze Clause Mod", &set("Clefable", "mirrormove"), &set("Snorlax", "splash"));
    let (p1, p2) = (b.active_at(0, 0).unwrap(), b.active_at(1, 0).unwrap());
    // The foe's Metronome called a two-turn move: its charging turn left
    // `lastMove` on Metronome.
    b.mon_mut(p2).last_move = moves::METRONOME;
    let pool = metronome_pool(b.dex);
    let at = |m: MoveId| pool.iter().position(|&x| x == m).unwrap();
    let mut forced: Vec<u32> = (0..k).map(|_| pick(at(moves::MIRRORMOVE), pool.len())).collect();
    forced.push(pick(at(moves::HAZE), pool.len()));
    b.prng.push_overrides(forced);
    b.run_move(ActionMove::Move(moves::MIRRORMOVE), p1, 0, EffectId::None);
    assert!(b.prng.override_is_empty(), "every forced roll used");
    let log = flush(&mut b).into_iter().filter(|(h, _)| h == "update").map(|(_, body)| body).collect::<Vec<_>>().join("\n");
    (b, log)
}

/// MOVE_CAP's gen 1 count: the cycle two turns deep (two Metronome rolls
/// of Mirror Move, 1 in 163^2) makes 3 + 2 * 2 = 7 objects, the cap, and
/// plays out; one turn (seed 6592751) makes 5.
#[test]
fn gen1_mirror_move_metronome_cycle_fits_the_arena() {
    for k in 0..=2 {
        let (b, log) = gen1_mirror_metronome_cycle(k);
        assert_eq!(usize::from(b.move_count), 3 + 2 * k, "{log}");
        assert_eq!(log.matches("|move|p1a: Clefable|Mirror Move|").count(), 1 + k, "{log}");
        assert_eq!(log.matches("|move|p1a: Clefable|Metronome|p1a: Clefable|[from] Mirror Move").count(), 1 + k, "{log}");
        assert_eq!(log.matches("|move|p1a: Clefable|Mirror Move|p1a: Clefable|[from] Metronome").count(), k, "{log}");
        assert!(log.contains("|move|p1a: Clefable|Haze|p1a: Clefable|[from] Metronome\n"), "{log}");
    }
    assert_eq!(crate::actions::MOVE_CAP, 7);
}

/// Three turns of the cycle (1 in 163^3) would need a ninth object: the
/// documented limit of [`crate::actions::MOVE_CAP`].
#[test]
#[should_panic(expected = "more than 7 ActiveMove objects in one action")]
fn gen1_cycle_three_rolls_deep_overflows() {
    gen1_mirror_metronome_cycle(3);
}

/// MOVE_CAP's gen 3 count, its longest chain: Encore turns the chosen
/// Tackle into a new Metronome object, which rolls Assist, which draws
/// the teammate's Nature Power, which calls Swift: 5 objects.
#[test]
fn gen3_longest_call_chain_fits_the_arena() {
    use ps_dex::gen3::{conditions, moves};
    let p1 = "Mew|||Synchronize|metronome,tackle||255,255,255,255,255,255||||90|]\
              Smeargle|||OwnTempo|naturepower||255,255,255,255,255,255||||90|";
    let p2 = "Snorlax|||Immunity|splash||255,255,255,255,255,255||||90|";
    let mut b = chain_battle("gen3customgame", p1, p2);
    let p1 = b.active_at(0, 0).unwrap();
    b.mon_mut(p1).last_move = moves::METRONOME;
    b.add_volatile(p1, conditions::ENCORE, None, EffectId::None);
    let pool = metronome_pool(b.dex);
    let assist = pool.iter().position(|&x| x == moves::ASSIST).unwrap();
    // Metronome rolls Assist; Assist's pool is Nature Power alone.
    b.prng.push_overrides([pick(assist, pool.len()), 0]);
    b.run_move(ActionMove::Move(moves::TACKLE), p1, 1, EffectId::None);
    assert!(b.prng.override_is_empty(), "every forced roll used");
    let log = flush(&mut b).into_iter().filter(|(h, _)| h == "update").map(|(_, body)| body).collect::<Vec<_>>().join("\n");
    assert_eq!(b.move_count, 5, "{log}");
    assert!(log.contains("|move|p1a: Mew|Assist|p1a: Mew|[from] Metronome"), "{log}");
    assert!(log.contains("|move|p1a: Mew|Nature Power|"), "{log}");
    assert!(log.contains("|move|p1a: Mew|Swift|p2a: Snorlax|[from] Nature Power"), "{log}");
}

/// Replays `scripted_gen1.json` (`tools/ps-diff/scripted-fixtures.js`):
/// short battles with fixed teams, seed and choices, written to reach one
/// group of handlers each, against PS's whole log.
#[test]
fn scripted_battles_match_ps_gen1() {
    let path = format!("{}/tests/fixtures/scripted_gen1.json", env!("CARGO_MANIFEST_DIR"));
    let all: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).expect("fixture readable")).unwrap();
    let scenarios = all.as_array().unwrap();
    assert!(scenarios.len() >= 3);
    for sc in scenarios {
        let name = sc["name"].as_str().unwrap();
        let seed: Vec<u16> = sc["seed"].as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as u16).collect();
        let cmds = [
            format!(r#"start {{"formatid":"{}","seed":[{},{},{},{}]}}"#, sc["format"].as_str().unwrap(), seed[0], seed[1], seed[2], seed[3]),
            format!(r#"player p1 {{"name":"Player 1","team":"{}"}}"#, sc["p1"].as_str().unwrap()),
            format!(r#"player p2 {{"name":"Player 2","team":"{}"}}"#, sc["p2"].as_str().unwrap()),
        ];
        let mut b = setup(&cmds);
        b.move_hook = None;
        b.lax_handlers = false;
        let mut log = Vec::new();
        let mut take = |b: &mut Battle| {
            for (h, body) in flush(b) {
                if h == "update" {
                    log.push(body);
                }
            }
        };
        take(&mut b);
        for pair in sc["choices"].as_array().unwrap() {
            for (side, c) in [SideId::P1, SideId::P2].into_iter().zip(pair.as_array().unwrap()) {
                if let Some(c) = c.as_str() {
                    b.step(&Command::Choose { side, choice: parse_choice(c) }).unwrap();
                }
            }
            take(&mut b);
        }
        let ours: Vec<String> =
            log.join("\n").split('\n').filter(|l| !l.starts_with("|t:|")).map(String::from).collect();
        let ps: Vec<&str> = sc["log"].as_str().unwrap().split('\n').collect();
        for (i, (o, p)) in ours.iter().zip(&ps).enumerate() {
            assert_eq!(o, p, "{name}: line {i} differs\n--- ours ---\n{}", ours.join("\n"));
        }
        assert_eq!(ours.len(), ps.len(), "{name}: log length\n--- ours ---\n{}", ours.join("\n"));
    }
}

/// Gen 1's `deductPP` at 0 PP: the cartridge wraps to 63, plain math
/// (`!Cartridge Trunc`) leaves -1, which `getMoves` disables (`pp <= 0`).
/// The ps-diff wide seeds 1003459, 1004914, 1006883 and 1008774 under
/// `--trunc math` play it through a binding move.
#[test]
fn gen1_pp_underflow_follows_the_rule_set() {
    use crate::actions::GenScripts;
    use crate::mods::gen1::scripts::Gen1Scripts;
    let math = FormatSpec::gen1_harness().with_repeal(ps_protocol::RuleId::CartridgeTrunc).unwrap();
    for (spec, want) in [(FormatSpec::gen1_harness(), 63), (math, -1)] {
        let mut b = Box::new(Battle::from_start(spec, [1, 2, 3, 4], CLOCK));
        for side in [SideId::P1, SideId::P2] {
            let team = parse_team("Venomoth|||noability|wrap,psychic|||||||");
            b.set_player(side, "p", "", "", Some(&team)).unwrap();
        }
        let _ = flush(&mut b);
        let mon = b.pokemon_at(0, 0);
        b.sides[0].last_selected_move_slot = 0;
        b.mon_mut(mon).move_slots[0].pp = 0;
        let wrap = b.mon(mon).move_slots[0].id;
        Gen1Scripts::deduct_pp(&mut b, mon, wrap, 0);
        assert_eq!(b.mon(mon).move_slots[0].pp, want);
        let moves = b.get_moves(mon, None, false);
        assert_eq!(moves[0].pp, Some(want));
        assert_eq!(moves[0].disabled, Some(want <= 0));
    }
}

/// `runSwitch` (PS 437926906): both leads switch in under one `SwitchIn`
/// field event after `speedSort(getAllActive(true))`, and gens <= 4 skip the
/// `Update` after `start` and after a switch. Through `start` of a Gen 1
/// mirror match PS draws 3 PRNG calls (the `insertChoice` tie, the
/// `runSwitch` sort, one `Update` sort; 0.11.11 drew 4); a pairing without
/// a speed tie draws none. Counted on PS's `Battle` for these teams.
#[test]
fn switch_in_prng_calls_follow_ps() {
    for (foe, want) in [("Wartortle", 3), ("Pidgey", 0)] {
        let mut b = Box::new(Battle::from_start(FormatSpec::gen1_harness(), [1, 2, 3, 4], CLOCK));
        for (side, species) in [(SideId::P1, "Wartortle"), (SideId::P2, foe)] {
            let team = parse_team(&format!("{species}|||noability|tackle|||||||"));
            b.set_player(side, "p", "", "", Some(&team)).unwrap();
        }
        let _ = flush(&mut b);
        assert_eq!(b.prng.calls, want, "Wartortle vs {foe}");
        // Both leads are started, the order of the last sort kept.
        for s in 0..2u8 {
            assert!(b.mon(b.active_at(s, 0).unwrap()).is_started);
        }
        assert!(b.speed_order.iter().all(|&p| p < 2));
    }
}

/// `queue.cancelAction` keeps a Pokemon's queued `runSwitch`, and
/// `getSwitches` lists the queued switches of one side or both.
#[test]
fn cancel_action_keeps_run_switch() {
    use crate::queue::{Action, ActionKind};
    let mut b = Box::new(Battle::from_start(FormatSpec::gen1_harness(), [1, 2, 3, 4], CLOCK));
    for side in [SideId::P1, SideId::P2] {
        let team = parse_team("Wartortle|||noability|tackle|||||||]Pidgey|||noability|tackle|||||||");
        b.set_player(side, "p", "", "", Some(&team)).unwrap();
    }
    let _ = flush(&mut b);
    let m = b.pokemon_at(0, 0);
    b.queue.clear();
    b.queue.push(Action::new(ActionKind::RunSwitch).with_pokemon(m));
    let mut sw = Action::new(ActionKind::Switch).with_pokemon(m);
    sw.target = Some(b.pokemon_at(0, 1));
    b.queue.push(sw);
    b.queue.push(Action::new(ActionKind::Move).with_pokemon(m));
    assert_eq!(b.queue.get_switches(Some(0)).count(), 1);
    assert_eq!(b.queue.get_switches(Some(1)).count(), 0);
    assert_eq!(b.queue.get_switches(None).count(), 1);
    // The queued switch's target is no longer a possible switch-in.
    assert_eq!(b.can_switch(0), 0);
    assert!(b.queue.cancel_action(m));
    assert_eq!(b.queue.len(), 1);
    assert_eq!(b.queue.peek().unwrap().choice, ActionKind::RunSwitch);
}

/// The variant formats' `ModifySpecies` rules give each Pokemon the base
/// stats PS's rule computes (`data/rulesets.ts`, gen 1's own in
/// `data/mods/gen1/rulesets.ts`), and the stats follow from them.
#[test]
fn species_rules_modify_base_stats() {
    let base_of = |format: &str, team: &str| -> ([u8; 6], u16, [u16; 5]) {
        let mut b = Box::new(Battle::from_start(FormatSpec::parse(format).unwrap(), [1, 2, 3, 4], CLOCK));
        for side in [SideId::P1, SideId::P2] {
            b.set_player(side, "p", "", "", Some(&parse_team(team))).unwrap();
        }
        let m = b.pokemon_at(0, 0);
        (b.mon(m).species_base_stats, b.mon(m).species_bst, b.mon(m).base_stored_stats)
    };
    // Machop 70/80/50/35/35/35: every stat of 70 or lower doubles.
    let (base, bst, _) = base_of("gen3customgame@@@Bad 'n Boosted Mod", "Machop|||noability|tackle|||||||");
    assert_eq!((base, bst), ([140, 80, 100, 70, 70, 70], 530));
    // Pichu 20/40/15/35/35/60 (BST 205): doubled.
    let (base, bst, _) = base_of("gen3customgame@@@350 Cup Mod", "Pichu|||noability|tackle|||||||");
    assert_eq!((base, bst), ([40, 80, 30, 70, 70, 120], 410));
    // Snorlax (BST 540) is left alone.
    let (base, _, _) = base_of("gen3customgame@@@350 Cup Mod", "Snorlax|||noability|tackle|||||||");
    assert_eq!(base, [160, 110, 65, 65, 110, 30]);
    // Gen 3 Scalemons: (600 - hp) / (bst - hp), Snorlax 440/380.
    let (base, bst, _) = base_of("gen3customgame@@@Scalemons Mod", "Snorlax|||noability|tackle|||||||");
    assert_eq!((base, bst), ([160, 127, 75, 75, 127, 34], 598));
    // Gen 1 Scalemons: toward 500, the Special counted once and copied.
    // Slowbro 95/75/110/80/80/30, gen 1 BST 390: 405/295.
    let (base, bst, _) = base_of("gen1customgame@@@Scalemons Mod", "Slowbro|||noability|tackle|||||||");
    assert_eq!((base, bst), ([95, 102, 151, 109, 109, 41], 498));
    // Gen 1 350 Cup: Rattata (BST 218) doubled, Special copied.
    let (base, bst, _) = base_of("gen1customgame@@@350 Cup Mod", "Rattata|||noability|tackle|||||||");
    assert_eq!((base, bst), ([60, 112, 70, 50, 50, 144], 436));
}

/// A Picked Team Size format asks for a team preview pick before the
/// battle: `|clearpoke|`, the one-sided `|poke|` lines, `|teampreview|N`,
/// then the picked Pokemon form the team in the picked order.
#[test]
fn picked_team_size_runs_a_team_preview() {
    let spec = FormatSpec::parse("gen3customgame@@@pickedteamsize=2").unwrap();
    let mut b = Box::new(Battle::from_start(spec, [1, 2, 3, 4], CLOCK));
    let team = "Snorlax|||noability|tackle|||||||]Pikachu|||noability|tackle|||||||]Gengar|||noability|tackle|||||||";
    for side in [SideId::P1, SideId::P2] {
        b.set_player(side, "p", "", "", Some(&parse_team(team))).unwrap();
    }
    assert_eq!(b.request_state, crate::request::RequestState::TeamPreview);
    let text: Vec<String> = flush(&mut b).into_iter().map(|(_, body)| body).collect();
    let text = text.join("\n");
    assert!(text.contains("|clearpoke\n|split|p1\n|poke|p1|Snorlax"), "{text}");
    assert!(text.contains("|teampreview|2"), "{text}");
    assert!(text.contains(r#"|request|{"teamPreview":true,"maxChosenTeamSize":2,"side":"#), "{text}");
    // A bad pick is refused; `team 3` fills in the rest in team order.
    assert!(!b.choose(0, ps_protocol::Choice::Team(ps_protocol::Name::from_str("9").unwrap())));
    assert!(b.choose(0, ps_protocol::Choice::Team(ps_protocol::Name::from_str("3").unwrap())));
    assert!(b.choose(1, ps_protocol::Choice::Default));
    assert_eq!(b.sides[0].pokemon(), &[2, 0]);
    assert_eq!(b.sides[1].pokemon(), &[0, 1]);
    assert_eq!(b.sides[0].pokemon_left, 2);
    assert!(b.started && b.turn == 1);
}
