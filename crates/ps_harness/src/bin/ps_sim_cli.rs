//! `ps_sim_cli`: `ps_sim` behind Pokemon Showdown's `BattleStream` text
//! protocol, on stdin / stdout. `tools/ps-diff/diff.js --ps-cli` drives it
//! against PS line for line.
//!
//! Input, one command per line, as `diff.js` writes them:
//! - `>start {"formatid":"gen1customgame","seed":[a,b,c,d]}` makes a new
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

use std::cell::RefCell;
use std::io::{self, BufRead, Write};
use std::panic::{self, AssertUnwindSafe};
use std::sync::OnceLock;

use ps_protocol::text::json::encode_request;
use ps_protocol::text::line::encode_msg;
use ps_protocol::text::names::Tables;
use ps_protocol::text::team::unpack;
use ps_protocol::text::encode_log;
use ps_protocol::{Choice, Command, FormatSpec, Msg, Request, SideId};
use ps_sim::text::{dex_tables, Names};
use ps_sim::{Battle, Sink};

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
    if args.iter().any(|a| a == "--help" || a == "-h") {
        eprintln!(
            "ps_sim_cli: ps_sim behind PS's BattleStream protocol\n  \
             Run on stdin/stdout; reads `>start` / `>player pN` / `>pN <choice>`.\n  \
             --ported <gen>   print the ported effect ids for `gen`, one per line."
        );
        return;
    }
    run_stream();
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
                let seed: Vec<u16> = v["seed"]
                    .as_array()
                    .ok_or_else(|| bad(">start: no seed"))?
                    .iter()
                    .map(|x| x.as_u64().map(|n| n as u16))
                    .collect::<Option<_>>()
                    .ok_or_else(|| bad(">start: bad seed"))?;
                let seed: [u16; 4] = seed.try_into().map_err(|_| bad(">start: the seed is not four numbers"))?;
                let clock = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as u32)
                    .unwrap_or(0);
                let mut b = Box::new(Battle::from_start(spec, seed, clock));
                b.spill = Some(spill);
                SPILLED.with(|s| s.borrow_mut().clear());
                self.battle = Some(b);
                self.ended_sent = false;
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
            other => return Err(bad(format!("command >{other}"))),
        }
        let b = self.battle.as_mut().unwrap();
        Ok(flush(b, &mut self.ended_sent))
    }
}

/// A singles choice, PS grammar (`move 1`, `switch 2`, `pass`, `default`,
/// `undo`).
fn parse_choice(text: &str) -> Option<Choice> {
    let (kind, arg) = text.trim().split_once(' ').unwrap_or((text.trim(), ""));
    Some(match kind {
        "move" => Choice::Move(arg.trim().parse().ok()?),
        "switch" => Choice::Switch(arg.trim().parse().ok()?),
        "pass" => Choice::Pass,
        "default" => Choice::Default,
        "undo" => Choice::Undo,
        _ => return None,
    })
}

fn run_stream() {
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
                stream.battle = None;
                format!("crash\n{e}\n")
            }
            Err(_) => {
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
}
