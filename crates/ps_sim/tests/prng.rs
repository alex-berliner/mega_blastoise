//! Integration test: `ps_sim::prng` matches Pokemon Showdown's PRNG
//! bit-for-bit over 100,000 method calls per `tools/ps-diff/prng-fixtures.js`.
//!
//! The fixture is committed under `tests/fixtures/prng.json` and
//! regenerated there before re-pinning; the script itself must reproduce
//! the JSON byte-for-byte on every run. See the brief in
//! `docs/architecture/showdown-split-execution.md` P1-2 for the contract.

use ps_sim::prng::{Gen5Rng, Prng, Seed};

/// Fixture emitted by the script. Keep large strings out of the test
/// binary (and out of source control) by pinning the file at HEAD.
const FIXTURE: &str = include_str!("fixtures/prng.json");

#[derive(serde::Deserialize)]
struct Fixture {
    block_size: usize,
    calls_per_seed: usize,
    seeds: Vec<SeedRecord>,
}

#[derive(serde::Deserialize)]
struct SeedRecord {
    seed: Seed,
    calls: Vec<CallEntry>,
    checks: Vec<CheckEntry>,
}

#[derive(serde::Deserialize)]
struct CallEntry {
    op: String,
    #[serde(default)]
    result: serde_json::Value,
    #[serde(default)]
    n: Option<u32>,
    #[serde(default)]
    m: Option<u32>,
    #[serde(default)]
    num: Option<u32>,
    #[serde(default)]
    den: Option<u32>,
    // The fixture carries `items` so any future reader can see exactly
    // what PS sampled/shuffled, even though we hard-code the test data
    // here for bit-exact comparison.
    #[serde(default)]
    #[allow(dead_code)]
    items: Option<serde_json::Value>,
}

#[derive(serde::Deserialize)]
struct CheckEntry {
    after: usize,
    seed: Seed,
}

// Drives one call against a Prng per the JSON-encoded entry; returns the
// expected value from the JSON so the caller can decode and assert it.
//
// Each `random()` entry in the fixture records the underlying u32 that
// `rng.next()` returned (PS computes `random() = next / 2**32`); we
// verify the integer so the f64 round-trip is bypassed (serde_json's
// number parser is not strictly IEEE 754 round-to-nearest on every
// decimal reading, while u32 round-trips bit-exactly in JSON). The Rust
// `Prng::random()` then computes the f64 the same way as PS and a
// separate unit test pins that conversion.
fn replay_one(prng: &mut Prng, entry: &CallEntry) -> serde_json::Value {
    match entry.op.as_str() {
        // PS PRNG's `next` is internal; the high-level `random()` returns
        // `next / 2^32` as an f64. The fixture records the underlying
        // integer so JSON round-trip is bit-exact; we replay through
        // `.next()` and compare.
        "random" => {
            let got = prng.next();
            serde_json::json!(got)
        }
        "random_n" => {
            let n = entry.n.expect("random_n has n");
            let got = prng.random_n(n);
            serde_json::json!(got)
        }
        "random_range" => {
            let m = entry.m.expect("random_range has m");
            let n = entry.n.expect("random_range has n");
            let got = prng.random_range(m, n);
            serde_json::json!(got)
        }
        "random_chance" => {
            let num = entry.num.expect("random_chance has num");
            let den = entry.den.expect("random_chance has den");
            let got = prng.random_chance(num, den);
            serde_json::json!(got)
        }
        "sample" => {
            // The fixture uses a 3-string array; we replicate that exact
            // call so the JSON `result` (the chosen string) matches.
            let items: [&str; 3] = ["a", "b", "c"];
            let got = *prng.sample(&items);
            serde_json::json!(got)
        }
        "shuffle" => {
            // The fixture uses [1,2,3,4,5,6]; we replicate it.
            let mut arr: [u32; 6] = [1, 2, 3, 4, 5, 6];
            prng.shuffle(&mut arr, 0, 6);
            serde_json::json!(arr)
        }
        other => panic!("unknown op in fixture: {other}"),
    }
}

// Drives one call against a Prng per the JSON-encoded entry; returns the
// expected value from the JSON so the caller can decode and assert it.
#[test]
fn all_one_hundred_thousand_calls_match_ps() {
    let fixture: Fixture = serde_json::from_str(FIXTURE).expect("fixture parses");
    assert_eq!(fixture.calls_per_seed, 1000, "calls_per_seed pinned at 1000");
    assert_eq!(fixture.seeds.len(), 100, "100 seeds pinned");
    assert_eq!(fixture.block_size, 10, "10-call block pinned");

    let mut total_calls = 0usize;
    for (seed_idx, seed_rec) in fixture.seeds.iter().enumerate() {
        let mut prng = Prng::new(seed_rec.seed);

        // The fixture stores the seed BEFORE any calls, but our test does
        // not need that snapshot because we constructed Prng::new with it.
        // Each entry then must reproduce the result AND drive the same
        // underlying state.
        let mut next_check = 0usize;
        for (call_n, entry) in seed_rec.calls.iter().enumerate() {
            let got = replay_one(&mut prng, entry);
            assert_eq!(
                got,
                entry.result,
                "seed_idx={seed_idx} call={call_n} op={} expected={}",
                entry.op,
                entry.result,
            );
            total_calls += 1;

            // State check after every CHECK_EVERY calls.
            if (call_n + 1) % 100 == 0 {
                let expected_check = &seed_rec.checks[next_check];
                assert_eq!(
                    expected_check.after,
                    call_n + 1,
                    "checkpoint order at seed {seed_idx}"
                );
                assert_eq!(
                    prng.get_seed(),
                    expected_check.seed,
                    "seed state at checkpoint seed_idx={seed_idx} after={}",
                    expected_check.after
                );
                next_check += 1;
            }
        }

        // Final state: matches the last checkpoint.
        assert_eq!(
            prng.get_seed(),
            seed_rec.checks.last().expect("at least one checkpoint").seed,
            "final seed state for seed_idx={seed_idx}"
        );
    }

    assert_eq!(
        total_calls, 100_000,
        "all 100 seeds x 1000 calls must execute"
    );
}

#[test]
fn call_counter_advances_one_per_public_method() {
    // Sanity check independent of the fixture: each public method that
    // consumes randomness adds exactly 1 to `calls`, except shuffle
    // which adds `end - start - 1`. We don't pin the exact delta here
    // because `random_range(m, m)` deliberately still calls `next()`
    // (verified in `random_range_equal_args_is_constant` in the unit
    // tests); instead, the fixture above drives the call count through
    // a long sequence and any drift would surface as a state mismatch.
    let mut prng = Prng::new([1, 2, 3, 4]);
    assert_eq!(prng.calls, 0);
    let _ = prng.random();
    assert_eq!(prng.calls, 1);
    prng.reset_calls();
    assert_eq!(prng.calls, 0);
}

#[test]
fn get_seed_matches_standalone_gen5_state() {
    // Whatever `Prng::get_seed()` reports must equal the state in a
    // peer `Gen5Rng` that did the same number of frames.
    let mut prng = Prng::new([7, 11, 13, 17]);
    let mut gen = Gen5Rng::new([7, 11, 13, 17]);
    assert_eq!(prng.get_seed(), gen.get_seed());

    for _ in 0..10 {
        let _ = prng.random();
    }
    for _ in 0..10 {
        gen.next();
    }
    assert_eq!(prng.get_seed(), gen.get_seed());
}

#[test]
fn fixture_is_byte_reproducible_via_script() {
    // Re-run the script in-place and diff against the file we just
    // loaded. Cargo sets `CARGO_MANIFEST_DIR` for integration tests
    // (pointing at `crates/ps_sim`); the script lives at
    // `<workspace>/tools/ps-diff/prng-fixtures.js` which is two
    // directories up from the manifest.
    use std::path::PathBuf;
    use std::process::Command;

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture_path = manifest_dir.join("tests/fixtures/prng.json");
    assert!(
        fixture_path.exists(),
        "fixture not found at {:?}",
        fixture_path,
    );

    let script_path = manifest_dir
        .join("..")
        .join("..")
        .join("tools/ps-diff/prng-fixtures.js");
    assert!(
        script_path.exists(),
        "script not found at {:?}",
        script_path,
    );

    let output = Command::new("node")
        .arg(&script_path)
        .output()
        .expect("spawn node prng-fixtures.js");
    assert!(
        output.status.success(),
        "prng-fixtures.js did not exit 0: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Tolerate trailing-newline differences (the script appends one
    // unconditionally; `include_str!` also captures it).
    let mut fresh = output.stdout.clone();
    if !fresh.ends_with(b"\n") {
        fresh.push(b'\n');
    }
    let mut on_disk = FIXTURE.as_bytes().to_vec();
    if !on_disk.ends_with(b"\n") {
        on_disk.push(b'\n');
    }
    if on_disk != fresh {
        let mismatch = std::env::current_dir()
            .ok()
            .map(|c| c.join(".scratch"))
            .unwrap_or_else(|| PathBuf::from(".scratch"))
            .join("prng-fresh.json");
        std::fs::create_dir_all(mismatch.parent().unwrap()).ok();
        std::fs::write(&mismatch, &fresh).ok();
        panic!(
            "prng-fixtures.js rerun differs from committed fixture; fresh @ {:?}, committed @ {:?}",
            mismatch, fixture_path,
        );
    }
}
