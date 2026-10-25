//! `ps_sim_cli`: `ps_sim` behind Pokemon Showdown's `BattleStream` text
//! protocol, on stdin / stdout. `tools/ps-diff/diff.js --ps-cli` drives it
//! against PS line for line.
//!
//! Input, one command per line, as `diff.js` writes them:
//! - `>start {"formatid":"gen1customgame","seed":[a,b,c,d]}` (or a seed string,
//!   `sodium,<hex>` included) makes a new
//!   battle (dropping any battle still running);
//! - `>player pN {"name":"...","team":"PACKED"}` (`avatar` and `rating`
//!   optional);
//! - `>pN CHOICE` (`move N`, `switch N`, `pass`, `default`, `undo`).
//!
//! The process is long-lived: it plays any number of battles, one after
//! the other, until stdin closes, then exits 0.
//!
//! Output is what PS's `BattleStream` pushes, one message after another,
//! each as its type line, its data, and a newline:
//! - `update` and the log lines since the last flush, with `|split|pN`
//!   framing (`ps_protocol::text::encode_log`);
//! - `sideupdate`, `pN`, and an `|error|` or `|request|` line
//!   (`encode_msg`, `encode_request`);
//! - `end` and a JSON summary once the battle is over (PS's `end` carries
//!   the whole input log as well; here only `winner`, `turns`, `p1`, `p2`).
//!
//! `BattleStream` flushes once per write; this flushes after every line,
//! which splits the same lines over more `update` messages without
//! changing them or their order.
//!
//! Two more messages end a battle early, in place of `end`, after which
//! the commands for that battle are ignored until the next `>start`:
//! - `unported` and the id, when `ps_sim` panics `unported: <id>` (the
//!   pipeline reached an effect that has no port yet);
//! - `crash` and the panic message (on one line), for any other panic or
//!   a command `ps_sim` cannot take.
//!
//! `--ported <gen>` prints `ps_sim::mods::ported(gen)`, one `kind:id` per
//! line (gens 1 and 3; others exit 2).
//!
//! `--handlers <gen>` prints every handler the gen's registry holds, one
//! per line: `kind:id`, the handler key (`onResidual`,
//! `basePowerCallback`) and the ported moves whose data can reach it (an
//! item: the item itself), comma separated (the `--focus` ids
//! `scripts/ps-coverage.sh` prints for an under-hit handler; `-` when the
//! data names none).
//!
//! `--hits FILE` (built with the `hit-count` feature) writes the handler
//! calls of every battle played to FILE, rewritten after each battle and at
//! exit; a `{pid}` in FILE becomes the process id and start time, so the
//! processes of a `--jobs` run (and any respawned one) each write their own
//! file. One `battles <gen> <n>` line per
//! gen, then one `hit <gen> <kind:id> <key> <calls> <battles>` line per
//! handler that ran (tab separated), `battles` being the number of battles
//! it ran in at least once.

use std::cell::RefCell;
#[cfg(feature = "hit-count")]
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::io::{self, BufRead, Write};
use std::panic::{self, AssertUnwindSafe};
use std::sync::OnceLock;

use ps_protocol::text::json::encode_request;
use ps_protocol::text::line::encode_msg;
use ps_protocol::text::names::Tables;
use ps_protocol::text::team::unpack;
use ps_protocol::text::encode_log;
use ps_protocol::{Choice, Command, FormatSpec, Msg, Name, PrngSeed, Request, SideId};
use ps_sim::text::{dex_tables, Names};
use ps_sim::effect::Rule;
use ps_sim::{Battle, EffectId, EventId, Prefix, Sink};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(|s| s.as_str()) == Some("--ported") {
        let gen = args.get(1).and_then(|s| s.parse::<u8>().ok());
        match gen {
            Some(g @ (1 | 3)) => run_ported(g),
            Some(g) => {
                eprintln!("ps_sim_cli: --ported not supported for gen {g}");
                std::process::exit(2);
            }
            None => {
                eprintln!("ps_sim_cli: --ported expects a generation (1, 3, ...)");
                std::process::exit(2);
            }
        }
        return;
    }
    if args.first().map(|s| s.as_str()) == Some("--handlers") {
        match args.get(1).and_then(|s| s.parse::<u8>().ok()) {
            Some(g @ (1 | 3)) => run_handlers(g),
            _ => {
                eprintln!("ps_sim_cli: --handlers expects a generation (1 or 3)");
                std::process::exit(2);
            }
        }
        return;
    }
    let mut hits_path = None;
    if let Some(i) = args.iter().position(|a| a == "--hits") {
        let Some(path) = args.get(i + 1) else {
            eprintln!("ps_sim_cli: --hits expects a file");
            std::process::exit(2);
        };
        if !cfg!(feature = "hit-count") {
            eprintln!("ps_sim_cli: --hits needs a build with `--features hit-count`");
            std::process::exit(2);
        }
        // The process id and start time: a respawned process never reuses
        // an earlier one's file.
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        hits_path = Some(path.replace("{pid}", &format!("{}-{nanos}", std::process::id())));
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        eprintln!(
            "ps_sim_cli: ps_sim behind PS's BattleStream protocol\n  \
             Run on stdin/stdout; reads `>start` / `>player pN` / `>pN <choice>`.\n  \
             --ported <gen>   print the ported effect ids for `gen`, one per line.\n  \
             --handlers <gen> print the registered handlers for `gen` and their focus moves.\n  \
             --hits FILE      write per-handler hit counts to FILE (`hit-count` builds)."
        );
        return;
    }
    run_stream(hits_path);
}

/// `--ported <gen>`: one `<kind>:<id>` per line.
fn run_ported(gen: u8) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for entry in ps_sim::mods::ported(gen) {
        let _ = writeln!(out, "{entry}");
    }
    let _ = out.flush();
}

// ---- handler names and hit counts ------------------------------------------

/// `kind:id` for a registered effect.
fn effect_name(dex: &ps_dex::Dex, id: EffectId) -> String {
    use ps_sim::mods::PsId;
    match id {
        EffectId::Move(m) => format!("move:{}", PsId(dex.move_def(m).name.as_str())),
        EffectId::Condition(c) => format!("condition:{}", dex.condition_def(c).key.as_str()),
        EffectId::Item(i) => format!("item:{}", PsId(dex.item_def(i).name.as_str())),
        EffectId::Ability(a) => format!("ability:{}", PsId(dex.ability_def(a).name.as_str())),
        EffectId::Rule(r) => format!("rule:{}", format!("{r:?}").to_ascii_lowercase()),
        other => format!("{other:?}"),
    }
}

/// The PS data key of a handler: `on{prefix}{event}`, or the callback key
/// (`basePowerCallback`).
fn handler_key(prefix: Prefix, event: EventId) -> String {
    let n = event.name();
    if event.is_callback_key() {
        format!("{}{}", n[..1].to_ascii_lowercase(), &n[1..])
    } else {
        format!("on{}{n}", prefix.name())
    }
}

/// Conditions no move's data names, which a handler adds: the moves whose
/// use reaches the handler that adds each.
const HANDLER_ADDED: &[(u8, &str, &[&str])] = &[
    // PS data/mods/gen2/conditions.ts tox.onStart (gen 1 inherits it).
    (1, "residualdmg", &["toxic"]),
    // PS data/mods/gen1/moves.ts dig / fly / the charge moves' onTryMove.
    (1, "twoturnmove", &["dig", "fly", "razorwind", "skullbash", "skyattack", "solarbeam"]),
    (1, "invulnerability", &["dig", "fly"]),
    // PS data/mods/gen1/conditions.ts partialtrappinglock.onBeforeMove.
    (1, "fakepartiallytrapped", &["bind", "clamp", "firespin", "wrap"]),
];

/// The ported moves whose data can reach `id`'s handlers: the move itself,
/// the moves that name a condition (`ps_sim::mods::move_conditions`), the
/// moves of [`HANDLER_ADDED`], and for Sleep and Freeze Clause Mod (whose
/// `onSetStatus` sees every status) the moves that inflict `slp` / `frz`.
/// An item's focus is the item itself (`--focus` takes item ids).
/// Empty when none can (gen 1's `trapped`: no gen 1 move adds it).
fn focus_moves(gen: u8, dex: &'static ps_dex::Dex, id: EffectId) -> Vec<String> {
    if let EffectId::Condition(c) = id {
        let key = dex.condition_def(c).key.as_str();
        if let Some((_, _, moves)) = HANDLER_ADDED.iter().find(|(g, k, _)| *g == gen && *k == key) {
            return moves.iter().map(|m| m.to_string()).collect();
        }
    }
    if let EffectId::Item(i) = id {
        // teams.js `splitFocus` reads an item id as an item focus.
        return vec![format!("{}", ps_sim::mods::PsId(dex.item_def(i).name.as_str()))];
    }
    let ported: BTreeSet<&str> = ps_sim::mods::ported(gen).iter().filter_map(|e| e.strip_prefix("move:")).collect();
    let cond = match id {
        EffectId::Condition(c) => Some(c),
        EffectId::Rule(Rule::SleepClauseMod) => dex.condition_id("slp"),
        EffectId::Rule(Rule::FreezeClauseMod) => dex.condition_id("frz"),
        _ => None,
    };
    let mut out = Vec::new();
    for i in 0..dex.moves.len() {
        let m = ps_dex::MoveId(i as u16);
        let name = format!("{}", ps_sim::mods::PsId(dex.move_def(m).name.as_str()));
        if !ported.contains(name.as_str()) {
            continue;
        }
        let hit = match (id, cond) {
            (EffectId::Move(x), _) => x == m,
            (_, Some(c)) => ps_sim::mods::move_conditions(dex, m).any(|x| x == c),
            _ => false,
        };
        if hit {
            out.push(name);
        }
    }
    out
}

fn dex_of(gen: u8) -> &'static ps_dex::Dex {
    if gen == 1 {
        &ps_dex::GEN1
    } else {
        &ps_dex::GEN3
    }
}

/// `--handlers <gen>`: `kind:id`, key and focus moves, tab separated.
fn run_handlers(gen: u8) {
    let dex = dex_of(gen);
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for (id, def) in ps_sim::mods::registry_for(dex).entries {
        let focus = focus_moves(gen, dex, *id);
        let focus = if focus.is_empty() { "-".to_string() } else { focus.join(",") };
        for h in def.handlers {
            let _ = writeln!(out, "{}\t{}\t{focus}", effect_name(dex, *id), handler_key(h.prefix, h.event));
        }
    }
    let _ = out.flush();
}

/// The sweep's handler calls so far (`--hits`).
#[cfg(feature = "hit-count")]
struct HitTotals {
    path: String,
    battles: BTreeMap<u8, u64>,
    /// (gen, effect, prefix, event) -> (calls, battles).
    calls: BTreeMap<(u8, EffectId, Prefix, EventId), (u64, u64)>,
}

#[cfg(feature = "hit-count")]
impl HitTotals {
    /// Adds one battle's calls (taken out of the battle).
    fn add(&mut self, b: &mut Battle) {
        *self.battles.entry(b.gen).or_insert(0) += 1;
        for ((effect, prefix, event), n) in std::mem::take(&mut b.hits.calls) {
            let e = self.calls.entry((b.gen, effect, prefix, event)).or_insert((0, 0));
            e.0 += u64::from(n);
            e.1 += 1;
        }
    }

    fn write(&self) {
        let mut text = String::new();
        for (gen, n) in &self.battles {
            text.push_str(&format!("battles\t{gen}\t{n}\n"));
        }
        for ((gen, effect, prefix, event), (calls, battles)) in &self.calls {
            let name = effect_name(dex_of(*gen), *effect);
            text.push_str(&format!("hit\t{gen}\t{name}\t{}\t{calls}\t{battles}\n", handler_key(*prefix, *event)));
        }
        let tmp = format!("{}.tmp", self.path);
        if std::fs::write(&tmp, text).and_then(|_| std::fs::rename(&tmp, &self.path)).is_err() {
            eprintln!("ps_sim_cli: cannot write {}", self.path);
        }
    }
}

#[cfg(feature = "hit-count")]
thread_local! {
    static HITS: RefCell<Option<HitTotals>> = const { RefCell::new(None) };
}

// ---- output ------------------------------------------------------------------

/// The codec's dex names, built once per dex.
fn tables(dex: &'static ps_dex::Dex) -> &'static Tables {
    static GEN1: OnceLock<Tables> = OnceLock::new();
    static GEN3: OnceLock<Tables> = OnceLock::new();
    let cell = if dex.gen == 1 { &GEN1 } else { &GEN3 };
    cell.get_or_init(|| dex_tables(dex))
}

thread_local! {
    /// The lines a full log spilled since the last `update`, as text.
    static SPILLED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    /// The message of the last panic, taken by the command loop.
    static PANIC: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// The battle's [`ps_sim::state::LogSpill`]: the lines go in front of the
/// next `update`.
fn spill(b: &Battle, lines: &[Msg]) {
    let text = encode_log(lines, tables(b.dex), &Names(b));
    SPILLED.with(|s| s.borrow_mut().push(text));
}

/// One `send_updates` block, typed; rendered once the call returns (the
/// rendering reads the battle `send_updates` holds mutably).
enum Block {
    /// The block's lines, after the text of those a full log spilled first.
    Update(Vec<String>, Vec<Msg>),
    Line(SideId, Msg),
    Request(SideId, Request),
}

#[derive(Default)]
struct Collect {
    blocks: Vec<Block>,
}

impl Sink for Collect {
    fn update(&mut self, lines: &[Msg]) {
        let spilled = SPILLED.with(|s| std::mem::take(&mut *s.borrow_mut()));
        self.blocks.push(Block::Update(spilled, lines.to_vec()));
    }
    fn side_line(&mut self, side: SideId, line: &Msg) {
        self.blocks.push(Block::Line(side, *line));
    }
    fn request(&mut self, side: SideId, request: &Request) {
        self.blocks.push(Block::Request(side, request.clone()));
    }
}

/// Appends one `BattleStream` message.
fn message(out: &mut String, kind: &str, data: &str) {
    out.push_str(kind);
    out.push('\n');
    out.push_str(data);
    out.push('\n');
}

/// PS `sendUpdates` as `BattleStream` messages, then `end` once the battle
/// is over.
fn flush(b: &mut Battle, ended_sent: &mut bool) -> String {
    let mut c = Collect::default();
    b.send_updates(&mut c);
    let (dex, names) = (tables(b.dex), Names(b));
    let mut out = String::new();
    for block in c.blocks {
        match block {
            Block::Update(mut parts, lines) => {
                if !lines.is_empty() || parts.is_empty() {
                    parts.push(encode_log(&lines, dex, &names));
                }
                message(&mut out, "update", &parts.join("\n"));
            }
            Block::Line(side, m) => {
                message(&mut out, "sideupdate", &format!("{}\n{}", side.as_str(), encode_msg(&m, dex, &names)));
            }
            Block::Request(side, r) => {
                let json = encode_request(&r, dex, &names);
                message(&mut out, "sideupdate", &format!("{}\n|request|{json}", side.as_str()));
            }
        }
    }
    if b.ended && !*ended_sent {
        *ended_sent = true;
        let name = |s: usize| serde_json::to_string(b.sides[s].name.as_str()).unwrap();
        let winner = match b.winner {
            Some(Some(side)) => name(side.index()),
            _ => "\"\"".to_string(),
        };
        let summary = format!("{{\"winner\":{winner},\"turns\":{},\"p1\":{},\"p2\":{}}}", b.turn, name(0), name(1));
        message(&mut out, "end", &summary);
    }
    out
}

// ---- input -------------------------------------------------------------------

/// The battle being played, if any.
#[derive(Default)]
struct Stream {
    battle: Option<Box<Battle>>,
    ended_sent: bool,
    /// The battle's hit counts are in the totals (`--hits`).
    counted: bool,
}

impl Stream {
    /// `--hits`: adds the battle's handler calls to the totals once (when it
    /// ends, stops early or is dropped) and rewrites the file.
    fn count_hits(&mut self) {
        #[cfg(feature = "hit-count")]
        if let Some(b) = self.battle.as_mut() {
            if !self.counted {
                self.counted = true;
                HITS.with(|h| {
                    if let Some(t) = h.borrow_mut().as_mut() {
                        t.add(b);
                        t.write();
                    }
                });
            }
        }
    }
}

/// What went wrong with a command (not a panic).
fn bad(what: impl std::fmt::Display) -> String {
    format!("ps_sim_cli: {what}")
}

impl Stream {
    /// One `>` line. Returns the text to write.
    fn command(&mut self, line: &str) -> Result<String, String> {
        let rest = &line[1..];
        let (kind, arg) = rest.split_once(' ').unwrap_or((rest, ""));
        match kind {
            "start" => {
                let v: serde_json::Value = serde_json::from_str(arg).map_err(|e| bad(format!(">start: {e}")))?;
                let formatid = v["formatid"].as_str().ok_or_else(|| bad(">start: no formatid"))?;
                let spec = FormatSpec::parse(formatid).ok_or_else(|| bad(format!("unknown format {formatid}")))?;
                // An array of four numbers (the harness) or a seed string
                // (`a,b,c,d`, `gen5,<hex>`, `sodium,<hex>`), as PS reads it.
                let seed = match &v["seed"] {
                    serde_json::Value::String(text) => {
                        PrngSeed::parse(text).ok_or_else(|| bad(format!(">start: bad seed {text:?}")))?
                    }
                    other => {
                        let seed: Vec<u16> = other
                            .as_array()
                            .ok_or_else(|| bad(">start: no seed"))?
                            .iter()
                            .map(|x| x.as_u64().map(|n| n as u16))
                            .collect::<Option<_>>()
                            .ok_or_else(|| bad(">start: bad seed"))?;
                        let seed: [u16; 4] = seed.try_into().map_err(|_| bad(">start: the seed is not four numbers"))?;
                        PrngSeed::Gen5(seed)
                    }
                };
                let clock = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as u32)
                    .unwrap_or(0);
                self.count_hits();
                let mut b = Box::new(Battle::from_start(spec, seed, clock));
                b.spill = Some(spill);
                SPILLED.with(|s| s.borrow_mut().clear());
                self.battle = Some(b);
                self.ended_sent = false;
                self.counted = false;
            }
            "player" => {
                let Some(b) = self.battle.as_mut() else { return Ok(String::new()) };
                let (side, json) = arg.split_once(' ').ok_or_else(|| bad(">player: no options"))?;
                let side = SideId::parse(side).ok_or_else(|| bad(format!(">player: side {side}")))?;
                let v: serde_json::Value = serde_json::from_str(json).map_err(|e| bad(format!(">player: {e}")))?;
                let field = |k: &str| v[k].as_str().unwrap_or("");
                let team = match field("team") {
                    "" => None,
                    t => Some(unpack(t).map_err(|e| bad(format!(">player: team: {e}")))?),
                };
                b.set_player(side, field("name"), field("avatar"), field("rating"), team.as_ref())
                    .map_err(|e| bad(format!(">player: {e:?}")))?;
            }
            "p1" | "p2" => {
                let Some(b) = self.battle.as_mut() else { return Ok(String::new()) };
                let side = SideId::parse(kind).unwrap();
                let choice = parse_choice(arg).ok_or_else(|| bad(format!("choice {arg:?}")))?;
                b.step(&Command::Choose { side, choice }).map_err(|e| bad(format!("{e:?}")))?;
            }
            "forcelose" => {
                // PS `>forcelose pN`: the side loses, the battle ends.
                let Some(b) = self.battle.as_mut() else { return Ok(String::new()) };
                let side = SideId::parse(arg).ok_or_else(|| bad(format!(">forcelose: side {arg}")))?;
                b.lose(side.index() as u8);
            }
            "forcetie" => {
                // PS `>forcetie`: the battle ends in a tie.
                let Some(b) = self.battle.as_mut() else { return Ok(String::new()) };
                b.step(&Command::ForceTie).map_err(|e| bad(format!("{e:?}")))?;
            }
            "forcewin" => {
                // PS `>forcewin pN`: that side wins.
                let Some(b) = self.battle.as_mut() else { return Ok(String::new()) };
                let side = SideId::parse(arg).ok_or_else(|| bad(format!(">forcewin: side {arg}")))?;
                b.step(&Command::ForceWin(side)).map_err(|e| bad(format!("{e:?}")))?;
            }
            other => return Err(bad(format!("command >{other}"))),
        }
        let b = self.battle.as_mut().unwrap();
        let text = flush(b, &mut self.ended_sent);
        if self.ended_sent {
            // Counted before the `end` is written, so the file is current
            // by the time the driver sees the battle over.
            self.count_hits();
        }
        Ok(text)
    }
}

/// A singles choice, PS grammar (`move 1`, `move NAME`, `switch 2`,
/// `switch NAME`, `pass`, `default`, `undo`).
///
/// The names are PS's: the replay corpus records the choice the player
/// typed (`move submission`), and PS resolves it against the request's
/// moves (by move id) or the team's Pokemon (by nickname or species id).
/// `ps_sim`'s choice layer implements that resolution, so the name goes
/// through unchanged.
fn parse_choice(text: &str) -> Option<Choice> {
    let (kind, arg) = text.trim().split_once(' ').unwrap_or((text.trim(), ""));
    let arg = arg.trim();
    Some(match kind {
        "move" => match arg.parse::<u8>() {
            Ok(n) => Choice::Move(n),
            Err(_) => Choice::MoveName(Name::from_str(arg).ok()?),
        },
        "switch" => match arg.parse::<u8>() {
            Ok(n) => Choice::Switch(n),
            Err(_) => Choice::SwitchName(Name::from_str(arg).ok()?),
        },
        "pass" => Choice::Pass,
        "default" => Choice::Default,
        "undo" => Choice::Undo,
        _ => return None,
    })
}

fn run_stream(hits_path: Option<String>) {
    #[cfg(feature = "hit-count")]
    if let Some(path) = hits_path {
        let t = HitTotals { path, battles: BTreeMap::new(), calls: BTreeMap::new() };
        t.write();
        HITS.with(|h| *h.borrow_mut() = Some(t));
    }
    #[cfg(not(feature = "hit-count"))]
    let _ = hits_path;
    // A panic ends the battle, not the process; keep its message for the
    // `unported` / `crash` report and print nothing (the harness would
    // read stderr as output).
    panic::set_hook(Box::new(|info| {
        let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "panic".to_string()
        };
        let at = info.location().map(|l| format!(" at {}:{}", l.file(), l.line())).unwrap_or_default();
        PANIC.with(|p| *p.borrow_mut() = Some(format!("{msg}{at}")));
    }));
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stream = Stream::default();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if !line.starts_with('>') {
            continue;
        }
        let text = match panic::catch_unwind(AssertUnwindSafe(|| stream.command(&line))) {
            Ok(Ok(text)) => text,
            Ok(Err(e)) => {
                stream.count_hits();
                stream.battle = None;
                format!("crash\n{e}\n")
            }
            Err(_) => {
                stream.count_hits();
                stream.battle = None;
                let msg = PANIC.with(|p| p.borrow_mut().take()).unwrap_or_default();
                match msg.strip_prefix("unported: ") {
                    Some(rest) => {
                        let id = rest.split(" at ").next().unwrap_or(rest);
                        format!("unported\n{id}\n")
                    }
                    None => format!("crash\n{}\n", msg.replace('\n', " | ")),
                }
            }
        };
        if !text.is_empty() {
            let mut out = stdout.lock();
            let _ = out.write_all(text.as_bytes());
            let _ = out.flush();
        }
    }
    stream.count_hits();
}
