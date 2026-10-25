//! Every line of every cached replay log, through the client.
//!
//! The corpus is the replay cache the fetcher wrote
//! (`/root/replay-cache/<format>/<id>.json` for the numeric-seed era and
//! `/root/replay-cache/current/<format>/<id>.json` for the `sodium,` era,
//! untracked, present on the machine that fetched it) plus the committed
//! sample (`crates/ps_harness/replays/`, laid out the same way). Both hold the
//! server's own replay log: the exact lines the battle room kept, chat,
//! joins and badges included.
//!
//! Every line of every log goes through the client: the battle lines
//! decode into a `Msg` the view applies, and the server's own lines (see
//! [`server_only`]) are recognised and ignored. No line may make the
//! client fail, and the corpus must not be vacuous: the battle lines are
//! decoded and applied, not skipped.

mod common;

use std::path::{Path, PathBuf};

use common::tables;

use ps_client::BattleView;
use ps_dex::Dex;
use ps_protocol::text::line::{decode_line, CodecState};

/// One replay: the log the server kept for that battle.
struct Replay {
    path: PathBuf,
    format: String,
    log: String,
}

/// The server's own lines: not part of the battle protocol, so the
/// client ignores them.
fn server_only(line: &str) -> bool {
    let Some(rest) = line.strip_prefix('|') else {
        // A chat message that ran over several lines.
        return true;
    };
    let kind = rest.split('|').next().unwrap_or("");
    match kind {
        // Chat and the room's roster.
        "c" | "c:" | "j" | "l" | "n" | "inactive" | "inactiveoff" => true,
        // The room's decorations.
        "badge" | "badgeget" | "uhtml" | "uhtmlchange" | "html" => true,
        // The battle's metadata, not an event.
        "rated" => true,
        // The team preview (gen 3): the client's own view of the teams
        // comes from the requests, not from these.
        "poke" | "clearpoke" | "teampreview" | "showteam" => true,
        // A server-side error (a bad name in a team), not a choice error.
        "error" => !line.starts_with("|error|[Invalid choice] ")
            && !line.starts_with("|error|[Unavailable choice] "),
        _ => false,
    }
}

/// The kinds the client does not read: the server's own text, which the
/// pinned tables (the tier names, the rule wordings, the notice
/// templates) do not have to know. A line of one of these kinds that the
/// codec cannot read is not an error; a line of any other kind is.
fn not_read(kind: &str) -> bool {
    matches!(kind, "tier" | "rule" | "raw" | "-message" | "-hint")
}

fn dex_for(format: &str) -> &'static Dex {
    if format.starts_with("gen3") {
        &ps_dex::GEN3
    } else {
        &ps_dex::GEN1
    }
}

/// The replay logs under `dir`: one subdirectory per format, and the
/// `current` era's formats one level down.
fn read_replays(dir: &Path) -> Vec<Replay> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let format = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        if format == "current" {
            out.extend(read_replays(&path));
            continue;
        }
        let Ok(files) = std::fs::read_dir(&path) else {
            continue;
        };
        for file in files.flatten() {
            let file = file.path();
            if file.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&file) else {
                continue;
            };
            let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
                continue;
            };
            let Some(log) = value.get("log").and_then(|l| l.as_str()) else {
                continue;
            };
            out.push(Replay { path: file, format: format.clone(), log: log.to_string() });
        }
    }
    out
}

/// What one log did to the client.
#[derive(Default)]
struct Run {
    /// The battle lines the client decoded and applied.
    decoded: usize,
    /// The server's own lines the client ignored.
    ignored: usize,
    /// The lines the client could not read, by the kind they carry.
    unread: std::collections::BTreeMap<String, usize>,
    /// The lines the client could not read and had no reason to.
    failures: Vec<String>,
}

fn play(dex: &'static Dex, replay: &Replay) -> Run {
    let mut codec = CodecState::new(tables(dex));
    let mut view = BattleView::new(dex);
    let mut run = Run::default();
    for (n, raw) in replay.log.lines().enumerate() {
        if raw.is_empty() {
            continue;
        }
        if server_only(raw) {
            run.ignored += 1;
            continue;
        }
        match decode_line(raw, &mut codec) {
            Ok(msg) => {
                run.decoded += 1;
                view.apply(&msg);
            }
            Err(e) => {
                let kind = raw.strip_prefix('|').unwrap_or(raw).split('|').next().unwrap_or("");
                if not_read(kind) {
                    *run.unread.entry(kind.to_string()).or_default() += 1;
                    continue;
                }
                run.failures
                    .push(format!("{} line {}: {e}\n    {raw}", replay.path.display(), n + 1));
            }
        }
    }
    run
}

fn corpus() -> Vec<Replay> {
    let mut out = Vec::new();
    let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("../ps_harness/replays");
    out.extend(read_replays(&sample));
    // The untracked cache, when the fetcher has written it.
    out.extend(read_replays(Path::new("/root/replay-cache")));
    out
}

#[test]
fn every_cached_log_line_parses() {
    let all = corpus();
    assert!(!all.is_empty(), "no replay logs found");
    let mut decoded = 0usize;
    let mut ignored = 0usize;
    let mut failures = Vec::new();
    let mut unread: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for replay in &all {
        let run = play(dex_for(&replay.format), replay);
        decoded += run.decoded;
        ignored += run.ignored;
        failures.extend(run.failures);
        for (k, n) in run.unread {
            *unread.entry(k).or_default() += n;
        }
    }
    eprintln!(
        "{} replays, {decoded} battle lines decoded, {ignored} server lines ignored, {} failures",
        all.len(),
        failures.len()
    );
    for (k, n) in &unread {
        eprintln!("  {n} {k} lines the client does not read");
    }
    for f in failures.iter().take(20) {
        eprintln!("  {f}");
    }
    assert!(failures.is_empty(), "{} lines failed", failures.len());
    // Not vacuous: the client read the battle lines of the corpus.
    assert!(decoded > 1000, "only {decoded} battle lines decoded");
}
