//! `ps_sim_cli`: the long-lived Pokemon Showdown BattleStream proxy.
//!
//! P1-7 builds the protocol-diff harness (`tools/ps-diff/diff.js`) and
//! this binary; together they are the parity fixture the rest of
//! `ps_sim` is measured against. P1-5 and P1-6 wire
//! `ps_sim::Battle::step` and the `|request|` emitter behind the
//! existing `>` command line wrap-up. Until those land, this binary:
//!
//! - Reads `>start`, `>player`, `>pN <choice>` lines from stdin, one
//!   per line, and runs as long as it sees end-of-file. The
//!   `docs/architecture/showdown-split-plan.md` section 4 lifecycle
//!   (re-running many battles per process) is the same wire framing;
//!   the parser here already keeps that contract so P1-5 lands as a
//!   drop-in for the stub body.
//! - Writes PS-shaped lines to stdout: at least the `|init|battle`
//!   preamble on `>start`, an `|error|[Invalid choice]` so the harness
//!   can pin the failure. None of this looks like a real battle: the
//!   harness flags `--ps-cli` runs as `unported` until P1-8 / P2-*
//!   start producing real bytes.
//! - `--ported <gen>` lists the ported effect ids for `gen`, one per
//!   line, on stdout. Reads them from `ps_sim::mods::ported` so the
//!   Rust side is the single source of truth (the protocol-diff
//!   harness shells out to this instead of carrying its own whitelist).
//!   The first token is the kind word (`moves`, `species`, ...); the
//!   rest are ids. An empty list is a valid output (P1-7's PinnedTxt
//!   reports a single summary line and exits 0 on the empty case too).
//!
//! Errors: invalid parse and an unclean close exit 2; a graceful EOF
//! after `>start` exits 0. The diff harness reports exit 2 as a crash,
//! exit 0 with no init block as `unported`, and exit 0 with partial
//! output as a diff (what we expect during porting).
//!
//! All `stdin / stdout / stderr` buffering is line-oriented so the
//! node driver can split on `\n`; we never write partial lines.
//!
//! Source of truth for the on-wire format: `PS/sim/SIM-PROTOCOL.md`,
//! `PS/sim/battle-stream.ts`. PS uses `\n`-separated lines and the
//! same text envelope the typed `Msg` codec in `ps_protocol` will
//! encode / decode. This binary uses the same framing because PS is
//! the protocol.
use std::io::{self, BufRead, Write};

/// One battle's worth of state the CLI keeps between commands.
#[derive(Default)]
struct BattleCtx {
    /// Whether `>start` already ran this process lifetime. New `>start`
    /// resets the state for a new battle.
    started: bool,
}

/// Top-level CLI entrypoint: `--ported <gen>` prints the ported list and
/// exits; otherwise we fall through to the long-lived BattleStream proxy.
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(|s| s.as_str()) == Some("--ported") {
        let gen = args.get(1).and_then(|s| s.parse::<u8>().ok());
        match gen {
            Some(1) => run_ported(1),
            Some(3) => run_ported(3),
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
            "ps_sim_cli: long-lived PS BattleStream proxy (P1-7 scaffold)\n  \
             Run on stdin/stdout; reads `>start` / `>player pN` / `>pN <choice>`.\n  \
             --ported <gen>   print the ported effect ids for `gen`, one per line."
        );
        return;
    }
    run_proxy();
}

/// Print the ported effect ids for `gen`, one per line, in the format
/// `<kind>:<id>`. The harness groups lines by the `kind` prefix to build
/// the wide pools (species, moves, items, abilities, conditions, rules).
/// Empty output means "no ported pipeline yet for this gen"; the
/// harness treats an empty list as the green light in `--self` (where
/// the wide generator uses the whole showdown_fuzz space) and as a
/// forced exit 2 in `--ps-cli` mode.
fn run_ported(gen: u8) {
    let ported = ps_sim::mods::ported(gen);
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for entry in ported {
        let _ = writeln!(out, "{entry}");
    }
    let _ = out.flush();
}

fn run_proxy() {
    eprintln!("ps_sim_cli: scaffold of P1-7 (ps_sim engine not yet wired)");
    let stdin = io::stdin();
    let mut ctx = BattleCtx::default();
    let mut saw_start = false;
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.is_empty() {
            continue;
        }
        if parse_command(&line, &mut ctx).is_some() {
            if line.starts_with(">start ") {
                saw_start = true;
            }
        }
    }
    if !saw_start {
        // Without a battle, the harness will mark this as `unported`;
        // exit cleanly so it doesn't read as a crash.
        eprintln!("ps_sim_cli: no >start seen; exiting");
    }
    // PS pushes `end` to its BattleStream after `|win|`. We don't run
    // a battle, so we just exit. The harness treats that as `unported`.
    std::process::exit(0);
}

fn parse_command(line: &str, ctx: &mut BattleCtx) -> Option<&'static str> {
    if !line.starts_with('>') {
        return None;
    }
    let rest = &line[1..];
    if let Some(args) = rest.strip_prefix("start ") {
        ctx.started = true;
        // Mirror PS's |init|battle / |gametype| / |gen| / |tier| lines
        // for whichever format the JSON encodes. The full list lives in
        // `PS/sim/battle.ts:startBattle` (P1-5 will replace this
        // stub with that call).
        let mut out = String::new();
        out.push_str("|init|battle\n");
        out.push_str("|gametype|singles\n");
        if let Some(fmt) = quick_get_field(args, "formatid") {
            let gen_num = fmt
                .strip_prefix("gen")
                .and_then(|s| s
                    .strip_suffix("randombattle")
                    .or_else(|| s.strip_suffix("customgame"))
                    .or_else(|| s.strip_suffix("battle"))
                )
                .unwrap_or("3");
            out.push_str(&format!("|gen|{gen_num}\n"));
            out.push_str(&format!("|tier|[Gen {gen_num}] Custom Game\n"));
        }
        out.push_str("|teamsize|p1|6\n");
        out.push_str("|teamsize|p2|6\n");
        out.push_str("|start\n");
        print(&out);
        return Some("start");
    }
    if let Some(after) = rest.strip_prefix("player ") {
        // ">player pN {json}":
        let mut parts = after.splitn(2, ' ');
        let side = parts.next().unwrap_or("").trim();
        let payload = parts.next().unwrap_or("");
        let mut out = String::new();
        if let Some(name) = quick_get_field(payload, "name") {
            out.push_str(&format!("|player|{side}|{name}|\n"));
        }
        print(&out);
        return Some("player");
    }
    // Anything else in this scaffold is treated as a choose: `>pN …`.
    // We acknowledge it, refuse it, and emit an |error| the harness can
    // pin so the parser-side contract is honest while the engine is
    // not yet wired.
    let mut out = String::new();
    out.push_str("|error|[Invalid choice] Stub ps_sim_cli: no engine wired yet\n");
    print(&out);
    Some("choose")
}

fn print(s: &str) {
    let stdout = io::stdout();
    let mut h = stdout.lock();
    let _ = h.write_all(s.as_bytes());
    let _ = h.flush();
}

/// Pull one JSON string field out of a flat object. We only need a
/// hammer here; serde stays out of this scaffold because the full
/// engine uses the typed `ps_protocol::text` codec, not `serde_json`.
fn quick_get_field(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\":");
    let pos = json.find(&needle)?;
    let after = &json[pos + needle.len()..];
    let after = after.trim_start();
    if let Some(stripped) = after.strip_prefix('"') {
        let end = stripped.find('"')?;
        return Some(stripped[..end].to_owned());
    }
    if let Some(stripped) = after.strip_prefix(',') {
        return quick_get_field(stripped, key);
    }
    None
}
