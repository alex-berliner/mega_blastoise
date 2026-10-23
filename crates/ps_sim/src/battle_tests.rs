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
//! (`encode_msg`, `encode_request`), the teams through its `unpack`; this
//! file adds only `BattleStream`'s `|split|` framing and the dex name
//! tables the codec reads.
//!
//! The hook battles replace PS's `runMove` with a stub, on both sides:
//! [`stub_run_move`] here, `STUB` in the generator.

extern crate std;

use std::boxed::Box;
use std::format;
use std::string::{String, ToString};
use std::sync::OnceLock;
use std::vec::Vec;

use ps_protocol::text::json::encode_request;
use ps_protocol::text::line::encode_msg;
use ps_protocol::text::names::{Cosmetic, Tables};
use ps_protocol::text::team::unpack;
use ps_protocol::{BattleNames, Health, HpStatus, Msg, PackedTeam, Request, SideId};

use super::*;
use crate::emit::Sink;

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
    let path = format!("{}/tests/fixtures/battles_gen{gen}.txt.gz", env!("CARGO_MANIFEST_DIR"));
    let raw = std::fs::read(&path).expect("fixture readable");
    let mut text = String::new();
    std::io::Read::read_to_string(&mut flate2::read::GzDecoder::new(&raw[..]), &mut text).expect("gzip");
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

// ---- choices (P1-4b's `decode_command` is still a stub) ---------------------

fn parse_choice(text: &str) -> ps_protocol::Choice {
    let (kind, arg) = text.split_once(' ').unwrap_or((text, ""));
    match kind {
        "move" => ps_protocol::Choice::Move(arg.parse().unwrap()),
        "switch" => ps_protocol::Choice::Switch(arg.parse().unwrap()),
        "pass" => ps_protocol::Choice::Pass,
        "default" => ps_protocol::Choice::Default,
        "undo" => ps_protocol::Choice::Undo,
        other => panic!("choice {other}"),
    }
}

fn parse_team(text: &str) -> PackedTeam {
    unpack(text).unwrap_or_else(|e| panic!("team {text:?}: {e}"))
}

// ---- rendering, through the `ps_protocol` text codec -----------------------

/// The codec's dex names from a `ps_dex` dex: index `i` of each column is
/// the dex's id `i` (`MoveNum(i)` is `MoveId(i)`, `TypeNum(i)` is
/// `Type::ALL[i]`), and the cosmetic formes in `cosmeticFormes` order.
fn tables(dex: &'static Dex) -> &'static Tables {
    static GEN1: OnceLock<Tables> = OnceLock::new();
    static GEN3: OnceLock<Tables> = OnceLock::new();
    let cell = if dex.gen == 1 { &GEN1 } else { &GEN3 };
    cell.get_or_init(|| {
        let mut t = Tables::new();
        let id = |name: &str| -> String { name.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_lowercase()).collect() };
        for m in dex.moves {
            t.moves.push(&id(m.name.as_str()), m.name.as_str());
        }
        for (i, s) in dex.species.iter().enumerate() {
            t.species.push(&id(s.name.as_str()), s.name.as_str());
            let base = ps_protocol::SpeciesNum(i as u16);
            for (k, name) in dex.cosmetic_formes(SpeciesId(i as u16)).iter().enumerate() {
                let (cid, name) = (id(name.as_str()), name.as_str().to_string());
                t.cosmetic.push(Cosmetic { id: cid, name, base, forme: k as u8 + 1 });
            }
        }
        for i in dex.items {
            t.items.push(&id(i.name.as_str()), i.name.as_str());
        }
        for a in dex.abilities {
            t.abilities.push(&id(a.name.as_str()), a.name.as_str());
        }
        for ty in Type::ALL {
            t.types.push(&id(ty.name()), ty.name());
        }
        t
    })
}

/// The codec's battle names: what `>player` and the teams set.
struct Names<'a>(&'a Battle);

impl BattleNames for Names<'_> {
    fn player_name(&self, side: SideId) -> &str {
        self.0.sides[side.index()].name.as_str()
    }
    fn player_avatar(&self, side: SideId) -> &str {
        self.0.sides[side.index()].avatar.as_str()
    }
    fn player_rating(&self, side: SideId) -> &str {
        self.0.sides[side.index()].rating.as_str()
    }
    fn nickname(&self, side: SideId, slot: u8) -> &str {
        self.0.sides[side.index()].mons[slot as usize].name.as_str()
    }
}

/// The HP of a line that carries one.
fn health_mut(m: &mut Msg) -> Option<&mut Health> {
    match m {
        Msg::Switch { health, .. }
        | Msg::Drag { health, .. }
        | Msg::Damage { health, .. }
        | Msg::Heal { health, .. }
        | Msg::SetHp { health, .. } => Some(health),
        _ => None,
    }
}

/// `m` with its HP in one form, as one viewer gets it.
fn with_health(m: &Msg, form: impl Fn(&Health) -> HpStatus) -> Msg {
    let mut m = *m;
    let h = health_mut(&mut m).unwrap();
    *h = Health::plain(form(h));
    m
}

/// A log as `BattleStream` writes it: a split line as `|split|pN`, the
/// secret form, the shared form; a one-sided hint as `|split|pN`, the hint,
/// an empty line (PS sim/battle.ts `addSplit`).
fn lines(b: &Battle, msgs: &[Msg]) -> String {
    let (dex, names) = (tables(b.dex), Names(b));
    let enc = |m: &Msg| encode_msg(m, dex, &names);
    let mut out: Vec<String> = Vec::new();
    for m in msgs {
        match m.split_side() {
            Some(side) => {
                out.push(format!("|split|{}", side.as_str()));
                match m {
                    Msg::Hint { text, .. } => {
                        out.push(enc(&Msg::Hint { text: *text, split: None }));
                        out.push(String::new());
                    }
                    _ => {
                        out.push(enc(&with_health(m, Health::secret)));
                        out.push(enc(&with_health(m, Health::shared)));
                    }
                }
            }
            None => out.push(enc(m)),
        }
    }
    out.join("\n")
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
                    b.move_hook = Some(stub_run_move);
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
    (0..2u8).any(|s| {
        let m = b.active_at(s, 0).unwrap();
        let a = b.mon(m).ability;
        !a.is_none()
            && b.registry.get(EffectId::Ability(a)).is_none()
            && b.dex.ability_def(a).handlers.iter().any(|h| {
                matches!(h.name(), "onStart" | "onSwitchIn" | "onFoeTrapPokemon" | "onFoeMaybeTrapPokemon")
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
    let mut tally = Tally::default();
    for f in load(3).iter().filter(|f| f.kind == "prefix") {
        if lead_has_unported_opening_ability(&setup(&f.chunks[0].commands)) {
            skipped_ability += 1;
            continue;
        }
        // Everything else in the opening is handler-free.
        assert!(opening_is_handler_free(f), "gen 3 #{}: an opening line no lead ability explains", f.index);
        replay(f, &mut tally).unwrap_or_else(|e| panic!("{e}"));
        compared += 1;
        with_forme += usize::from(team_has_cosmetic_forme(f));
    }
    std::eprintln!(
        "gen 3 openings: {compared} match ({with_forme} with an Unown forme), {skipped_ability} wait for lead abilities"
    );
    assert_eq!(compared + skipped_ability, 60);
    assert_eq!(compared, 45);
    // An `Unown-X` details string in the requests (and the switch line
    // when it leads).
    assert_eq!(with_forme, 3);
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

/// Without the hook, a move is unported: the panic the CLI reports.
#[test]
#[should_panic(expected = "unported")]
fn moves_are_unported_without_the_hook() {
    let f = load(1).into_iter().find(|f| f.kind == "prefix").unwrap();
    let mut b = setup(&f.chunks[0].commands);
    b.move_hook = None;
    b.step(&Command::Choose { side: SideId::P1, choice: ps_protocol::Choice::Move(1) }).unwrap();
    b.step(&Command::Choose { side: SideId::P2, choice: ps_protocol::Choice::Move(1) }).unwrap();
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
    let b = Box::new(Battle::from_start(FormatSpec::gen3_harness(), [1, 2, 3, 4], CLOCK));
    assert!(!b.has_rule(Rule::CartridgeTrunc) && !b.has_rule(Rule::DesyncClauseMod) && b.has_rule(Rule::CancelMod));
    assert_eq!(b.trunc, Trunc::Math);
    assert!(b.field.pseudo_weather.is_empty());
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
