//! The PRNG trace on both sides of the protocol diff.
//!
//! `ps_sim_cli --rng-trace` (the `rng-trace` feature of `ps_sim`)
//! sends the battle's PRNG calls as an `rngtrace` message: the call
//! index, the method, the arguments, the result and the caller
//! location. On the PS side `tools/ps-diff/diff.js` wraps PS's PRNG
//! methods (without changing what they return) and records the same
//! thing. When the two sides diverge, the driver writes the two traces
//! side by side around the first differing call, with both call sites.
//!
//! The test plays one Gen 1 battle twice:
//! - as it is: the two traces are the same;
//! - with one extra `random()` injected on the PS side
//!   (`PS_RNG_TRACE_EXTRA=5`): the report must name the injected call
//!   as the first differing call (index 5), with the PS call site and
//!   the `ps_sim` call site of that call.
//!
//! The traced `ps_sim_cli` is built here (into `.scratch/`, so the
//! workspace's own build stays untouched): the feature is off by
//! default, so the plain binary cannot trace.

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

/// The `ps_sim_cli` built with the `rng-trace` feature.
fn build_traced_cli() -> PathBuf {
    let target = workspace_root()
        .join(".scratch")
        .join("ps-harness-rng-trace")
        .join("target");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let out = Command::new(&cargo)
        .args([
            "build",
            "-p",
            "ps_harness",
            "--features",
            "rng-trace",
            "--bin",
            "ps_sim_cli",
            "--target-dir",
        ])
        .arg(&target)
        .current_dir(workspace_root())
        .stdin(Stdio::null())
        .output()
        .unwrap_or_else(|e| panic!("spawn {cargo}: {e}"));
    assert!(
        out.status.success(),
        "cargo build --features rng-trace failed: {}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let bin = target.join("debug").join("ps_sim_cli");
    assert!(bin.is_file(), "the traced ps_sim_cli is missing at {:?}", bin);
    bin
}

/// The diff driver, with the given arguments and environment.
fn node_diff(bin: &Path, args: &[&str], extra_random: Option<u32>) -> std::process::Output {
    let node = std::env::var("NODE").unwrap_or_else(|_| "node".to_string());
    let driver = workspace_root().join("tools").join("ps-diff").join("diff.js");
    let mut cmd = Command::new(&node);
    cmd.arg(&driver)
        .arg("--ps-cli")
        .arg(bin)
        .args(args)
        .current_dir(workspace_root())
        .stdin(Stdio::null())
        .env("PS_RNG_TRACE", "1");
    match extra_random {
        Some(n) => {
            cmd.env("PS_RNG_TRACE_EXTRA", n.to_string());
        }
        None => {
            cmd.env_remove("PS_RNG_TRACE_EXTRA");
        }
    }
    cmd.output()
        .unwrap_or_else(|e| panic!("spawn {node} {}: {e}", driver.display()))
}

fn scratch_dir() -> PathBuf {
    workspace_root().join(".scratch").join("ps-diff-rng-trace")
}

/// The divergence report of the battle at `seed`.
fn divergence_log(seed: u32) -> String {
    let path = scratch_dir().join(format!("ps-diff-{seed}.log"));
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

#[test]
fn rng_trace_finds_the_injected_extra_random() {
    let bin = build_traced_cli();
    let scratch = scratch_dir();
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch).expect("scratch dir");
    let scratch_s = scratch.to_string_lossy().to_string();

    // 1. The battle as it is: the two traces are the same. The tamper
    //    hook forces a divergence so that the report has a trace
    //    section to write.
    let out = node_diff(
        &bin,
        &[
            "--gen", "1", "--seed", "0", "--n", "1", "--jobs", "1", "--scratch", &scratch_s,
            "--tamper-line", "3",
        ],
        None,
    );
    assert!(
        !out.status.success(),
        "the tampered run should diff: {:?}",
        String::from_utf8_lossy(&out.stdout),
    );
    let txt = divergence_log(0);
    assert!(
        txt.contains("--- PRNG trace ---"),
        "the report has no trace section:\n{txt}",
    );
    assert!(
        txt.contains("the two traces are the same"),
        "the two traces should be the same:\n{txt}",
    );

    // 2. The same battle with one extra `random()` on the PS side: the
    //    report must name the injected call as the first differing
    //    one, and show both call sites.
    let out = node_diff(
        &bin,
        &[
            "--gen", "1", "--seed", "0", "--n", "1", "--jobs", "1", "--scratch", &scratch_s,
        ],
        Some(5),
    );
    assert!(
        !out.status.success(),
        "the run with an extra random should diff: {:?}",
        String::from_utf8_lossy(&out.stdout),
    );
    let txt = divergence_log(0);
    assert!(
        txt.contains("first differing call: 5"),
        "the report should name call 5 as the first difference:\n{txt}",
    );
    // The row of the differing call: the PS side (the injected call)
    // and the ps_sim side (the call that the injection pushed aside).
    let row = txt
        .lines()
        .find(|l| l.starts_with(">>") && l.contains(" 5 |"))
        .unwrap_or_else(|| panic!("no row for call 5 in:\n{txt}"));
    assert!(
        row.contains("diff.js (injected extra random)"),
        "the PS row should be the injected call:\n{row}",
    );
    assert!(
        row.contains("crates/ps_sim/src/"),
        "the ps_sim row should carry its call site:\n{row}",
    );
    // The calls before the injection are the same on both sides.
    assert!(
        !txt.contains("the two traces are the same"),
        "the traces differ, so the report must not say they are the same:\n{txt}",
    );
}
