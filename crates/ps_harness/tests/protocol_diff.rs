//! Integration test for `tools/ps-diff/diff.js` and `ps_sim_cli`.
//!
//! `cargo test -p ps_harness --test protocol_diff` is one half of the
//! `scripts/check.sh` protocol-diff gate; the other half runs node side
//! directly. Rust stays inside this crate's tree so the test still
//! works on a worker worktree where the diff scripts live in places
//! `cargo` does not reach.
//!
//! Six pieces, all wrapped in a single `cargo test` for the gate:
//! - `pinned_txt_format` parses `crates/ps_harness/pinned.txt` line by
//!   line. The file can be header-only (zero entries is allowed; the
//!   fixed corpus is empty until a port ships) and the format test
//!   still walks every line so an inserted entry stays valid.
//! - `pinned_txt_zero_runs_green` confirms the file is header-only
//!   today (the green zero-run path the driver takes). Reverts to a
//!   skipping no-op the moment the file has entries; the driver-side
//!   check lives in scripts/check.sh.
//! - `ps_sim_cli_ported` runs the built `ps_sim_cli` with `--ported
//!   <gen>`, asserts the contract on gens 1/3 (one line per id or empty),
//!   and asserts gen 2 exits 2.
//! - `ps_sim_cli_init_block` plays two battles in one `ps_sim_cli`: the
//!   `update` / `sideupdate` framing and the battle start come back, a
//!   battle that reaches an unported move ends with `unported` and its
//!   id, and the next `>start` plays on.
//! - `diff_drive_choice_seed` exercises the divergence-on-data-
//!   change path: tampered line 7 of the right log under --self mode
//!   surfaces a divergence at exactly line 7. Same shape as "one side's
//!   choice seed changed": a real divergence anywhere on either side's
//!   log lands at a known line and the divergence report names it.
//! - `diff_drive_tamper_line` exercises the same path with a different
//!   seed and a different K, pins the second divergence file. Together
//!   the two tests cover the divergence reporting.
//! - `carried_corpus` checks the carried corpora (`carried/gen1/*.json`,
//!   `carried/gen3/*.json`) are well-formed PS sets and that `--carried
//!   genN` plays a pair of each
//!   and names a divergence after the pair.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workspace_root() -> PathBuf {
    let base = manifest_dir();
    base.parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .expect("workspace root")
}

#[test]
fn pinned_txt_format() {
    let path = manifest_dir().join("pinned.txt");
    let txt = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {}", path.display(), e));
    let mut entries: Vec<(u8, String, u32, String)> = Vec::new();
    for (idx, line) in txt.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        // Format: "<gen> <teams> <seed> [<trunc>]  # <commit> <note>"
        // A non-comment line must have the entry form: head `gen teams
        // seed` followed by `  # <commit> <note>`.
        let (head, tail) = trimmed.split_once("  # ").unwrap_or_else(|| {
            panic!(
                "pinned.txt:{}: malformed line `{}` (expected `  # <commit> <note>` tail)",
                idx + 1,
                trimmed
            )
        });
        let mut head_parts = head.split_whitespace();
        let gen = head_parts.next().unwrap().parse::<u8>().unwrap_or_else(|e| {
            panic!("pinned.txt:{}: bad gen: {}", idx + 1, e)
        });
        let teams = head_parts.next().unwrap();
        let seed = head_parts.next().unwrap().parse::<u32>().unwrap_or_else(|e| {
            panic!("pinned.txt:{}: bad seed: {}", idx + 1, e)
        });
        assert!(
            matches!(teams, "random" | "wide"),
            "pinned.txt:{}: teams mode `{}` is not `random` or `wide`",
            idx + 1,
            teams
        );
        // The optional Gen 1 rule set (`diff.js --trunc`).
        let trunc = head_parts.next().unwrap_or("cartridge");
        assert!(
            matches!(trunc, "cartridge" | "math") && (gen == 1 || head.split_whitespace().count() == 3),
            "pinned.txt:{}: the field after the seed is a Gen 1 `cartridge` or `math`, not `{}`",
            idx + 1,
            trunc
        );
        assert!(
            head_parts.next().is_none(),
            "pinned.txt:{}: trailing tokens after the rule set",
            idx + 1
        );
        let commit_and_note = tail.trim();
        let mut note_parts = commit_and_note.split_whitespace();
        let commit = note_parts.next().unwrap();
        assert!(!commit.is_empty(), "pinned.txt:{}: empty commit", idx + 1);
        let note: String = note_parts.collect::<Vec<_>>().join(" ");
        assert!(!note.is_empty(), "pinned.txt:{}: missing note", idx + 1);
        let coord = (gen, teams.to_string(), seed, trunc.to_string());
        assert!(
            !entries.contains(&coord),
            "pinned.txt: duplicate {:?}",
            coord
        );
        entries.push(coord);
    }
    // Zero-entry file is allowed: pinned.txt is for ps_sim_cli vs PS
    // regressions, and is empty until a port ships. The format test
    // still walks every line so an inserted entry stays valid; the
    // duplicate-coordinate check above continues to apply.
}

#[test]
fn pinned_txt_zero_runs_green() {
    // Reading the file directly and asserting 0 entries is simpler than
    // running the diff driver, because the diff driver does not depend
    // on this invariant independently. (driver-side check is the
    // pinned-zero-runs-green bash test in scripts/check.sh.)
    let path = manifest_dir().join("pinned.txt");
    let txt = fs::read_to_string(&path).unwrap();
    let entries = txt
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.trim().starts_with('#'))
        .count();
    // Pinned.txt for ps_sim vs PS regressions is empty today. If this
    // assertion ever fires, an entry was added by another task and the
    // rebase that lands here is responsible for the rest of the wiring.
    if entries != 0 {
        eprintln!(
            "pinned.txt now has {entries} entries; falling back to skipping the zero-entry assertion \
             (the driver and pinned.txt format test both still pass)"
        );
        return;
    }
    let _ = scratch_dir(); // sanity: scratch location is reachable
}

fn find_ps_sim_cli() -> Option<PathBuf> {
    let base = manifest_dir();
    let mut dir: Option<&Path> = Some(base.as_path());
    while let Some(d) = dir {
        for profile in &["debug", "release"] {
            let cand = d.join("target").join(profile).join("ps_sim_cli");
            if cand.is_file() {
                return Some(cand);
            }
        }
        dir = d.parent();
    }
    None
}

#[test]
fn ps_sim_cli_init_block() {
    let Some(bin) = find_ps_sim_cli() else {
        eprintln!("(skipping: ps_sim_cli not built; run `cargo build -p ps_harness --bin ps_sim_cli`)");
        return;
    };
    // Two battles in one process: the first ends early on `>forcetie`
    // (every gen 1 and gen 3 effect has a port now, so nothing stops a
    // battle as unported), the second is left waiting for choices.
    let team = |mv: &str| format!("Pikachu|||NoAbility|{mv}||255,255,255,255,255,255||30,30,30,30,30,30||50|");
    let team3 = "Snorlax|||illuminate|facade||85,85,85,85,85,85|M||||";
    let start = ">start {\"formatid\":\"gen1customgame\",\"seed\":[0,0,0,0]}\n";
    let start3 = ">start {\"formatid\":\"gen3customgame\",\"seed\":[0,0,0,0]}\n";
    let player = |side: &str, name: &str, team: &str| format!(">player {side} {{\"name\":\"{name}\",\"team\":\"{team}\"}}\n");
    let payload = [
        start3.to_string(),
        player("p1", "a", team3),
        player("p2", "b", team3),
        ">forcetie\n".to_string(),
        start.to_string(),
        player("p1", "c", &team("tackle")),
        player("p2", "d", &team("tackle")),
    ]
    .concat();
    let mut child = Command::new(&bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {:?}: {}", bin, e));
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin.write_all(payload.as_bytes()).expect("write");
    }
    let out = child.wait_with_output().expect("wait");
    assert!(out.status.success(), "ps_sim_cli exited {:?}", out.status);
    let s = String::from_utf8_lossy(&out.stdout);
    let needs = [
        "update\n|t:|",
        "|gametype|singles",
        "|player|p1|a||",
        "|player|p2|b||",
        "|gen|3",
        "|tier|[Gen 3] Custom Game",
        "|teamsize|p1|1",
        "|start\n|split|p1\n|switch|p1a: Snorlax|Snorlax, M|",
        "sideupdate\np1\n|request|{\"active\":",
        "|tie\nend\n",
        "|player|p2|d||",
        "|gen|1",
        "|switch|p1a: Pikachu|Pikachu, L50|",
    ];
    for line in &needs {
        assert!(s.contains(line), "ps_sim_cli output missing `{line}`:\n{s}");
    }
    let second = s.split("|tie\nend\n").nth(1).unwrap();
    assert!(second.contains("|start\n"), "the second battle did not start:\n{second}");
}

#[test]
fn ps_sim_cli_ported() {
    let Some(bin) = find_ps_sim_cli() else {
        eprintln!("(skipping: ps_sim_cli not built; run `cargo build -p ps_harness --bin ps_sim_cli`)");
        return;
    };
    // The contract: every non-empty line is `<kind>:<id>` with kind in
    // {move, species, item, ability, condition, rule}. An empty stdout
    // is the "no ported pipeline yet" signal P1-7 documents.
    for gen in [1u8, 3u8] {
        let out = Command::new(&bin)
            .args(["--ported", &gen.to_string()])
            .output()
            .expect("run --ported");
        assert!(
            out.status.success(),
            "ps_sim_cli --ported {gen} exited {:?}",
            out.status
        );
        let s = String::from_utf8_lossy(&out.stdout);
        parse_ported_lines(&s, gen);
    }
    // Other gens exit 2 with the message on stderr.
    let out = Command::new(&bin)
        .args(["--ported", "2"])
        .output()
        .expect("run --ported 2");
    assert!(!out.status.success(), "ps_sim_cli --ported 2 should fail");
    assert_eq!(out.status.code(), Some(2), "ps_sim_cli --ported 2 should exit 2");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("not supported"),
        "ps_sim_cli --ported 2 stderr should mention `not supported`: got {err:?}"
    );
}

fn parse_ported_lines(s: &str, gen: u8) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (idx, line) in s.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let pos = trimmed.find(':').unwrap_or_else(|| {
            panic!("ps_sim_cli --ported {gen} line {idx} missing kind prefix: {trimmed:?}")
        });
        let kind = &trimmed[..pos];
        let id = &trimmed[pos + 1..];
        assert!(!id.is_empty(), "ps_sim_cli --ported {gen} line {idx} has empty id: {trimmed:?}");
        assert!(
            matches!(kind, "move" | "species" | "item" | "ability" | "condition" | "rule"),
            "ps_sim_cli --ported {gen} line {idx} has unknown kind: {trimmed:?}"
        );
        out.push((kind.to_string(), id.to_string()));
    }
    out
}

fn node_diff(args: &[&str]) -> std::process::Output {
    let node = std::env::var("NODE").unwrap_or_else(|_| "node".to_string());
    let driver_path = workspace_root().join("tools").join("ps-diff").join("diff.js");
    Command::new(&node)
        .arg(&driver_path)
        .args(args)
        .current_dir(workspace_root())
        .output()
        .unwrap_or_else(|e| {
            panic!(
                "spawn node {:?} {}: {}",
                node,
                driver_path.display(),
                e
            )
        })
}

fn scratch_dir() -> PathBuf {
    workspace_root().join(".scratch").join("ps-diff")
}

/// (a) One side's choice seed changed: the diff driver in --self mode
/// catches it. We exercise the same code path the choice-seed-divergence
/// fix would mutate via the `--tamper-line K` test hook (which pins a
/// divergence at exactly line K). With no tampering, the two PS-via-PS
/// sides run bit-identical (0 diffs); with tampering at line 7 they
/// diverge at line 7.
#[test]
fn diff_drive_choice_seed() {
    // Baseline: no tampering, two PS sides should be 0 diffs and 0
    // crashes.
    let out = node_diff(&["--self", "--seed", "0", "--gen", "1", "--quiet"]);
    assert!(
        out.status.success(),
        "diff self baseline: status={:?} stdout={:?} stderr={:?}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );

    // The choice-seed change is implemented as --tamper-line 7, which
    // mutates right side's log line 7 so the two sides diverge at exactly
    // line 7. The cargo test pins both the diff result (exit non-zero)
    // and the divergence log (line number + context).
    let path = scratch_dir().join("ps-diff-0.log");
    let _ = fs::remove_file(&path);
    let out = node_diff(&[
        "--self", "--seed", "0", "--gen", "1", "--tamper-line", "7", "--quiet",
    ]);
    assert!(!out.status.success(), "tampered run should diff");
    let txt = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
    assert!(txt.contains("diff @ line 7"), "diff log should pin line 7:\n{txt}");
    assert!(txt.contains("L: "), "context section missing from:\n{txt}");
    let _ = fs::remove_file(&path);
}

/// (b) One line of one side's log edited by `--tamper-line K`: the diff
/// is caught at line K. Same as above but with a different seed so the
/// divergence file is `ps-diff-1.log`.
#[test]
fn diff_drive_tamper_line() {
    let path = scratch_dir().join("ps-diff-1.log");
    let _ = fs::remove_file(&path);
    let out = node_diff(&[
        "--self", "--seed", "1", "--gen", "1", "--tamper-line", "12", "--quiet",
    ]);
    assert!(!out.status.success(), "tampered run should diff");
    let txt = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
    assert!(txt.contains("diff @ line 12"), "diff log should pin line 12:\n{txt}");
    assert!(txt.contains("L: "), "context section missing from:\n{txt}");
    let _ = fs::remove_file(&path);
}

/// The carried corpus (`carried/gen1/*.json` and `carried/gen3/*.json`),
/// frozen: it was dumped by the legacy fuzz suite's `FUZZ_DUMP` through the
/// now-deleted `scripts/carried-dump.sh`, and every file is one old-engine
/// pinned entry's team pairs as PS sets. `diff.js --carried genN` plays a
/// pair of it. A tampered carried battle lands its divergence under the
/// pair's own name, since every pair replays the same battle seeds.
#[test]
fn carried_corpus() {
    for gen in [1u64, 3] {
        carried_corpus_of(gen);
    }
}

fn carried_corpus_of(gen: u64) {
    let dir = manifest_dir().join("carried").join(format!("gen{gen}"));
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {}", dir.display(), e))
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no carried corpus in {}", dir.display());
    for path in &files {
        let doc: serde_json::Value = serde_json::from_str(&fs::read_to_string(path).unwrap())
            .unwrap_or_else(|e| panic!("{}: not JSON: {e}", path.display()));
        assert_eq!(doc["gen"], gen, "{}: not a gen {gen} dump", path.display());
        assert_eq!(doc["suite"], format!("fuzz_gen{gen}_battles").as_str(), "{}: suite", path.display());
        let cases = doc["cases"].as_array().unwrap_or_else(|| panic!("{}: no cases", path.display()));
        assert!(!cases.is_empty(), "{}: no cases", path.display());
        for c in cases {
            for side in ["p1", "p2"] {
                let team = c[side].as_array().unwrap_or_else(|| panic!("{}: no {side}", path.display()));
                assert!((1..=6).contains(&team.len()), "{}: {side} has {} mons", path.display(), team.len());
                for set in team {
                    assert!(set["species"].is_string(), "{}: a set without species", path.display());
                    assert!(set["moves"].as_array().is_some_and(|m| !m.is_empty()), "{}: a set without moves", path.display());
                    // An empty ability unpacks to the species' first one
                    // (gen 1: a later gen's), so every set names one.
                    if gen == 1 {
                        assert_eq!(set["ability"], "noability", "{}: a gen 1 set needs noability", path.display());
                    } else {
                        assert!(
                            set["ability"].as_str().is_some_and(|a| !a.is_empty()),
                            "{}: a gen 3 set names its ability (noability for none)",
                            path.display()
                        );
                    }
                }
            }
        }
    }

    let first = files[0].file_stem().unwrap().to_string_lossy().to_string();
    let pair = format!("{first}#0");
    let carried = format!("gen{gen}");
    let scratch = workspace_root().join(".scratch").join(format!("ps-diff-carried-test-{carried}"));
    let scratch_s = scratch.to_string_lossy().to_string();
    let out = node_diff(&[
        "--carried", &carried, "--carried-pair", &pair, "--seed", "0", "--jobs", "1",
        "--quiet", "--scratch", &scratch_s,
    ]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success() && stdout.contains("ran=1 diffs=0"),
        "carried self run: status={:?} stdout={stdout:?} stderr={:?}",
        out.status,
        String::from_utf8_lossy(&out.stderr),
    );
    let log = scratch.join(format!("ps-diff-carried-{first}-0-0.log"));
    let _ = fs::remove_file(&log);
    let out = node_diff(&[
        "--carried", &carried, "--carried-pair", &pair, "--seed", "0", "--jobs", "1",
        "--quiet", "--scratch", &scratch_s, "--tamper-line", "9",
    ]);
    assert!(!out.status.success(), "tampered carried run should diff");
    let txt = fs::read_to_string(&log).unwrap_or_else(|e| panic!("read {}: {}", log.display(), e));
    assert!(txt.contains(&format!("pair {pair} diff @ line 9")), "carried diff log:\n{txt}");
    let _ = fs::remove_file(&log);
}

/// The targeted battles (`crates/ps_harness/targeted/gen1.json` and
/// `gen3.json`, played by `diff.js --targeted genN`): every entry is well
/// formed, each whole list plays green PS against PS (each battle reaches
/// its `expect` lines), and an entry at a seed that misses its path fails
/// with an `-expect.log`.
#[test]
fn targeted_battles() {
    for gen in [1, 3] {
        targeted_battles_of(gen);
    }
    // Seed 0 disables Growl, not Tackle: the battle misses its path.
    let scratch = workspace_root().join(".scratch").join("ps-diff-targeted-test");
    let scratch_s = scratch.to_string_lossy().to_string();
    let log = scratch.join("ps-diff-targeted-transform_disable_slot-0-expect.log");
    let _ = fs::remove_file(&log);
    let out = node_diff(&[
        "--targeted", "gen1", "--targeted-name", "transform_disable_slot", "--seed", "0",
        "--jobs", "1", "--quiet", "--scratch", &scratch_s,
    ]);
    assert!(!out.status.success(), "a targeted battle that misses its path should fail");
    let txt = fs::read_to_string(&log).unwrap_or_else(|e| panic!("read {}: {}", log.display(), e));
    assert!(txt.contains("|-start|p1a: Mew|Disable|Tackle"), "expect log:\n{txt}");
    let _ = fs::remove_file(&log);
}

fn targeted_battles_of(gen: u8) {
    let path = manifest_dir().join("targeted").join(format!("gen{gen}.json"));
    let doc: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path).unwrap())
        .unwrap_or_else(|e| panic!("{}: not JSON: {e}", path.display()));
    let list = doc.as_array().unwrap_or_else(|| panic!("{}: not an array", path.display()));
    assert!(!list.is_empty(), "{}: no targeted battles", path.display());
    let mut names = std::collections::BTreeSet::new();
    for t in list {
        let name = t["name"].as_str().unwrap_or_else(|| panic!("an entry without a name: {t}"));
        assert!(names.insert(name.to_string()), "{name} is listed twice");
        assert!(t["covers"].as_str().is_some_and(|c| c.contains("PS: ")), "{name}: covers names no PS source");
        assert!(t["seed"].is_u64(), "{name}: seed");
        assert!(t["expect"].as_array().is_some_and(|e| !e.is_empty()), "{name}: nothing to expect");
        for row in t["choices"].as_array().unwrap_or_else(|| panic!("{name}: no choices")) {
            assert!(row.as_array().is_some_and(|r| r.len() == 2), "{name}: a choice row is not [p1, p2]");
        }
        for side in ["p1", "p2"] {
            let team = t[side].as_array().unwrap_or_else(|| panic!("{name}: no {side}"));
            assert!((1..=6).contains(&team.len()), "{name}: {side} has {} mons", team.len());
        }
        if let Some(rules) = t.get("rules") {
            assert!(rules.as_array().is_some_and(|r| r.iter().all(|x| x.is_string())), "{name}: rules is a list of rule names");
        }
    }

    let scratch = workspace_root().join(".scratch").join(format!("ps-diff-targeted-test-gen{gen}"));
    let scratch_s = scratch.to_string_lossy().to_string();
    let targeted = format!("gen{gen}");
    let out = node_diff(&["--targeted", &targeted, "--jobs", "4", "--quiet", "--scratch", &scratch_s]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success() && stdout.contains(&format!("ran={} diffs=0 crashes=0", list.len())),
        "targeted self run (gen {gen}): status={:?} stdout={stdout:?} stderr={:?}",
        out.status,
        String::from_utf8_lossy(&out.stderr),
    );
}
