//! Fuzz parity: randomly generated scenarios against the REAL Pokémon
//! Showdown simulator, both engines.
//!
//! Same machinery as `showdown_parity.rs`, but the scenarios are drawn from a
//! seeded generator instead of a hand-written table: random species, levels,
//! natures, spreads, stat stages, statuses, screens, and a scripted RNG
//! (hit/miss, crit, exact roll) per case. Every mismatch prints the seed and
//! the full scenario JSON, so a finding replays with
//! `FUZZ_SEED=<n> cargo test ...` and can be pinned into the static table.
//!
//! The move pool is the harness's "vanilla" list — fixed base-power damage
//! with the effects the engines model (secondaries, drain, recoil,
//! multi-hit), derived from Showdown's own dex properties. Anything needing
//! a bespoke sim hook stays out so the fuzz measures mechanics, not stubs.
//!
//! Volume via `FUZZ_N` (default 200 per suite). Skips loudly when the harness
//! is unarmed, like the parity suite.
//!
//! `FUZZ_CASE=k` narrows a suite to ONE of its cases: the suite still runs
//! every case's own side, so the generator's stream (and the choices it
//! draws while the engine plays) is exactly what it would have been, but
//! only case `k` is sent to the reference and only case `k` is compared.
//! The case that fails is the same battle at the same index as when the
//! finding was made, which is what makes a pinned seed replayable.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{json, Value};

fn harness_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/showdown-parity")
}

fn armed() -> bool {
    if harness_dir().join("node_modules").exists() {
        return true;
    }
    eprintln!("SKIP: showdown fuzz not armed — npm install in tools/showdown-parity");
    false
}

/// Where cached reference answers live. Overridable, and removable at any
/// time — a cold cache only costs time.
fn cache_dir() -> PathBuf {
    match std::env::var("FUZZ_CACHE_DIR") {
        Ok(d) => PathBuf::from(d),
        Err(_) => std::env::temp_dir().join("mb-showdown-cache"),
    }
}

fn fnv1a(bytes: &[u8], mut hash: u64) -> u64 {
    for b in bytes {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
    }
    hash
}

/// The harness itself is part of the key: change how we drive the reference
/// and every previous answer is void.
fn harness_stamp() -> u64 {
    static STAMP: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    *STAMP.get_or_init(|| {
        let bytes = std::fs::read(harness_dir().join("harness.js")).unwrap_or_default();
        fnv1a(&bytes, 0xcbf2_9ce4_8422_2325)
    })
}

/// Run a batch of scenarios through the reference, answering from disk
/// whatever has been asked before.
///
/// The reference is deterministic for everything the suites compare — HP, PP,
/// status, faints, who is active, move order. The ONE thing that varies run to
/// run is each mon's gender letter in the battle log, which is cosmetic and
/// only ever read by a human looking at a failure. So a scenario's answer can
/// be kept and reused.
///
/// The key is the scenario itself, not the fuzz seed. That matters: the battle
/// suites RECORD the choices our own engine made and hand that list to the
/// reference, so changing the engine changes the scenario — and a changed
/// scenario is simply a different key, which misses and runs live. There is no
/// way for a stale answer to be served against new behaviour.
///
/// `only` names the one case the caller cares about: that case is the only
/// one asked for live, and the only one answered. The others come back
/// empty (a JSON null) and the caller skips them. The suites still run
/// every case's own side, so the generator's stream does not move.
fn showdown(scenarios: &Value, only: Option<usize>) -> Vec<Value> {
    let list = scenarios.as_array().cloned().unwrap_or_default();
    let caching = std::env::var("FUZZ_NO_CACHE").is_err();
    let dir = cache_dir();

    // Each scenario's own text is the key; the stored copy is checked against
    // it on read, so a hash collision cannot serve the wrong answer.
    let texts: Vec<String> = list.iter().map(|s| s.to_string()).collect();
    let keys: Vec<String> = texts
        .iter()
        .map(|t| format!("{:016x}", fnv1a(t.as_bytes(), harness_stamp())))
        .collect();

    let mut answers: Vec<Option<Value>> = vec![None; list.len()];
    if caching {
        for (i, key) in keys.iter().enumerate() {
            let path = dir.join(&key[..2]).join(format!("{key}.json"));
            let Ok(raw) = std::fs::read(&path) else { continue };
            let Ok(entry) = serde_json::from_slice::<Value>(&raw) else { continue };
            if entry.get("scenario").map(|s| s.to_string()).as_deref() == Some(&texts[i]) {
                if let Some(result) = entry.get("result") {
                    answers[i] = Some(result.clone());
                }
            }
        }
    }

    // With `only` set, the cases that are not named are not even asked
    // about: the reference does the work for one case, and the rest of the
    // list comes back empty for the caller to skip.
    let misses: Vec<usize> = (0..list.len())
        .filter(|i| answers[*i].is_none())
        .filter(|i| only.is_none_or(|k| k == *i))
        .collect();
    if !misses.is_empty() {
        let batch = Value::Array(misses.iter().map(|i| list[*i].clone()).collect());
        let fresh = run_harness(&batch);
        assert_eq!(fresh.len(), misses.len(), "harness returned the wrong count");
        for (slot, result) in misses.iter().zip(fresh) {
            if caching {
                let key = &keys[*slot];
                let shard = dir.join(&key[..2]);
                let _ = std::fs::create_dir_all(&shard);
                let entry = json!({"scenario": list[*slot], "result": result});
                // Write beside the target and rename, so a dozen sweep
                // workers racing on the same scenario cannot tear a file.
                let tmp = shard.join(format!("{key}.{}.tmp", std::process::id()));
                if std::fs::write(&tmp, entry.to_string()).is_ok() {
                    let _ = std::fs::rename(&tmp, shard.join(format!("{key}.json")));
                }
            }
            answers[*slot] = Some(result);
        }
    }

    answers
        .into_iter()
        .enumerate()
        .map(|(i, a)| match a {
            Some(v) => v,
            None if only == Some(i) => {
                panic!("the case the filter named was not answered")
            }
            None => Value::Null,
        })
        .collect()
}

fn run_harness(scenarios: &Value) -> Vec<Value> {
    let mut child = Command::new("node")
        .arg("harness.js")
        .current_dir(harness_dir())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn node");
    child.stdin.take().unwrap().write_all(scenarios.to_string().as_bytes()).unwrap();
    let out = child.wait_with_output().expect("harness run");
    assert!(out.status.success(), "harness failed: {}", String::from_utf8_lossy(&out.stderr));
    serde_json::from_slice(&out.stdout).expect("harness emitted JSON")
}

// ── Deterministic generator ──────────────────────────────────────────────────

struct Fuzz(u64);

impl Fuzz {
    fn new(suite: &str) -> Fuzz {
        let seed = fuzz_seed();
        eprintln!("[{suite}] FUZZ_SEED={seed} FUZZ_N={}", fuzz_n());
        // Mix the suite name in so the suites do not mirror each other.
        let mix = suite.bytes().fold(seed, |a: u64, b| a.rotate_left(7) ^ b as u64);
        Fuzz(if mix == 0 { 1 } else { mix })
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    fn below(&mut self, n: u64) -> u64 {
        (self.next() >> 16) % n.max(1)
    }

    fn chance(&mut self, pct: u64) -> bool {
        self.below(100) < pct
    }

    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u64) as usize]
    }
}

fn fuzz_seed() -> u64 {
    std::env::var("FUZZ_SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(0xF00D_2026)
}

fn fuzz_n() -> usize {
    std::env::var("FUZZ_N").ok().and_then(|s| s.parse().ok()).unwrap_or(200)
}

/// The one case to compare, when `FUZZ_CASE` names it.
///
/// The filter narrows the reference's work and the comparison, never the
/// generator: the suite still runs every case's own side, so the stream of
/// draws (and the choices drawn while the engine plays) is exactly what it
/// would have been, and case `k` is the same battle it would have been.
///
/// A `FUZZ_CASE` that is not a number is an error, not a shrug: a typo in a
/// pinned corpus must not quietly turn into "compare everything".
fn fuzz_case() -> Option<usize> {
    match std::env::var("FUZZ_CASE") {
        Err(_) => None,
        Ok(raw) => {
            let raw = raw.trim();
            if raw.is_empty() {
                return None;
            }
            Some(
                raw.parse()
                    .unwrap_or_else(|_| panic!("FUZZ_CASE={raw:?} is not a case number")),
            )
        }
    }
}

/// `FUZZ_CASE` checked against the suite's own case count: a stale number
/// must not turn the run into a vacuous pass.
fn checked_case(n_cases: usize, suite: &str) -> Option<usize> {
    match fuzz_case() {
        Some(k) => {
            assert!(
                k < n_cases,
                "FUZZ_CASE={k} is past the end of {suite}: it has {n_cases} cases"
            );
            Some(k)
        }
        None => None,
    }
}

/// `FUZZ_WIDE=1` widens the battle suite past the shape it was built in:
/// 1-4 moves a mon, 1-6 mons a side, a level, nature and spread per mon,
/// 2-30 turns and drawn multi-hit counts (drawn faint replacements come at
/// level 2, below). Unset, the generators draw exactly what
/// they always drew, so an old seed still names the same battles: every
/// extra draw is made only with the knob on, and the narrow path's own draws
/// keep their order and count.
///
/// `FUZZ_WIDE=2` is the same stream with Gen 3's drawn replacements switched
/// on as well. They go through `player_picks_replacement` and `send_out`,
/// the path the game runner uses: `step_with` parks the rest of the turn at
/// the faint, the way the sim asks for the switch mid-turn in this era, and
/// `send_out` resumes it with the pick on the field. Level 1 keeps the
/// engine's own lowest-slot replacement, so its seeds name the battles they
/// named when the chosen path still skipped that turn's residuals.
fn fuzz_wide() -> bool {
    std::env::var("FUZZ_WIDE").is_ok_and(|v| v == "1" || v == "2")
}

/// Whether the reference died of its own internal assertion rather than
/// refusing one of our choices. Showdown words those "shouldn't happen":
/// gen 1's `commitChoices` throws "moveSlot is null which shouldn't happen"
/// when a Transformed mon wakes on a slot index the copied foe never had.
/// Past that point the reference has no answer to compare against, so the
/// battle suites compare only the turns it did play.
fn sim_crashed(got: &Value) -> bool {
    got["errors"].as_array().is_some_and(|es| {
        es.iter().any(|e| e.as_str().is_some_and(|s| s.contains("shouldn't happen")))
    })
}

fn fuzz_wide_replace() -> bool {
    std::env::var("FUZZ_WIDE").is_ok_and(|v| v == "2")
}

/// `n` distinct moves from the pool. Two copies of one move is not a legal
/// set, and the sim folds them into a single slot with shared PP. With
/// `n == 2` this is draw-for-draw the old "pick, then re-pick while equal".
fn distinct_moves(fz: &mut Fuzz, pool: &[String], n: usize) -> Vec<String> {
    let n = n.min(pool.len());
    let mut set: Vec<String> = Vec::new();
    while set.len() < n {
        let m = fz.pick(pool).clone();
        if !set.contains(&m) {
            set.push(m);
        }
    }
    set
}

/// A legal spread, in the sim's stat order (hp, atk, def, spa, spd, spe):
/// every IV 0-31, every EV 0-255 with at most 510 in all. The EVs are dealt
/// from a random starting stat so the budget does not always run dry on Speed.
fn wide_spread(fz: &mut Fuzz) -> ([u8; 6], [u8; 6]) {
    let mut ivs = [0u8; 6];
    for iv in ivs.iter_mut() {
        *iv = fz.below(32) as u8;
    }
    let mut evs = [0u8; 6];
    let start = fz.below(6) as usize;
    let mut budget = 510u64;
    for k in 0..6 {
        let ev = fz.below(budget.min(255) + 1);
        evs[(start + k) % 6] = ev as u8;
        budget -= ev;
    }
    (ivs, evs)
}

fn stat_json(xs: [u8; 6]) -> Value {
    json!({"hp": xs[0], "atk": xs[1], "def": xs[2], "spa": xs[3], "spd": xs[4], "spe": xs[5]})
}

/// Vanilla move ids for a gen, intersected with our table so both sides can
/// run every sampled move. Returns (id, priority). `multihit` keeps or drops
/// the multi-strike moves: the single-hit suites compare one strike's damage,
/// so a move the sim lands twice would only measure the miscount.
fn vanilla_moves(gen: u8, multihit: bool, ours: impl Fn(&str) -> bool) -> Vec<(String, i8)> {
    // `FUZZ_ONLY=return,flail` narrows the pool to those ids, for asking one
    // move at a time whether it matches the sim. Widening the pool turned up
    // a backlog of moves that had never been fuzzed at all, and a sweep that
    // fails on all of them at once says nothing about any one of them.
    let only = std::env::var("FUZZ_ONLY").unwrap_or_default();
    let mut only: Vec<&str> = only.split(',').filter(|s| !s.is_empty()).collect();
    // Two copies of one move is not a legal set — the sim folds them into a
    // single slot with shared PP — so a narrowed pool always keeps two
    // ordinary moves to fill the other slot with. TWO, not one: the id under
    // test may not exist in this generation, and a pool of one move sends
    // the team builder's search for a second, distinct move round forever.
    if !only.is_empty() {
        only.push("tackle");
        only.push("scratch");
    }
    let results = showdown(&json!([{"kind": "movelist", "gen": gen}]), None);
    results[0]["moves"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|m| multihit || !m["multihit"].as_bool().unwrap_or(false))
        .map(|m| (m["id"].as_str().unwrap().to_string(), m["priority"].as_i64().unwrap() as i8))
        .filter(|(id, _)| ours(id))
        .filter(|(id, _)| only.is_empty() || only.contains(&id.as_str()))
        .collect()
}

// ── Gen 3: single scripted hits, the full damage surface ─────────────────────

/// The Gen 3 abilities the engine models, and so the ones the fuzzer is
/// allowed to hand out. Everything outside this list stays off both sides:
/// the reference sim would play it and we would not, and every battle it
/// touched would diverge for a reason we already know about.
///
const GEN3_ABILITIES: &[&str] = &[
    "airlock",
    "arenatrap",
    "battlearmor",
    "blaze",
    "chlorophyll",
    "clearbody",
    "cloudnine",
    "colorchange",
    "compoundeyes",
    "cutecharm",
    "damp",
    "drizzle",
    "drought",
    "earlybird",
    "effectspore",
    "flamebody",
    "flashfire",
    "forecast",
    "guts",
    "hugepower",
    "hustle",
    "hypercutter",
    "illuminate",
    "immunity",
    "innerfocus",
    "insomnia",
    "intimidate",
    "keeneye",
    "levitate",
    "lightningrod",
    "limber",
    "liquidooze",
    "magmaarmor",
    "magnetpull",
    "marvelscale",
    "minus",
    "naturalcure",
    "oblivious",
    "overgrow",
    "owntempo",
    "pickup",
    "plus",
    "poisonpoint",
    "pressure",
    "purepower",
    "raindish",
    "rockhead",
    "roughskin",
    "runaway",
    "sandstream",
    "sandveil",
    "serenegrace",
    "shadowtag",
    "shedskin",
    "shellarmor",
    "shielddust",
    "soundproof",
    "speedboost",
    "static",
    "stench",
    "stickyhold",
    "sturdy",
    "suctioncups",
    "swarm",
    "swiftswim",
    "synchronize",
    "thickfat",
    "torrent",
    "trace",
    "truant",
    "vitalspirit",
    "voltabsorb",
    "waterabsorb",
    "waterveil",
    "whitesmoke",
    "wonderguard",
];


/// The Gen 3 held items the engine models, and so the ones the fuzzer is
/// allowed to hand out. Same rule as the abilities: anything outside this
/// list stays off both sides.
///
/// Still to come: Quick Claw and Focus Band, whose rolls the harness has no
/// knob for yet.
const GEN3_ITEMS: &[&str] = &[
    "apicotberry",
    "aspearberry",
    "aguavberry",
    "blackbelt",
    "blackglasses",
    "brightpowder",
    "charcoal",
    "cheriberry",
    "chestoberry",
    "choiceband",
    "deepseascale",
    "deepseatooth",
    "dragonfang",
    "figyberry",
    "focusband",
    "ganlonberry",
    "hardstone",
    "lansatberry",
    "mentalherb",
    "leppaberry",
    "kingsrock",
    "laxincense",
    "leftovers",
    "iapapaberry",
    "liechiberry",
    "lightball",
    "lumberry",
    "luckypunch",
    "magoberry",
    "machobrace",
    "magnet",
    "mail",
    "metalcoat",
    "metalpowder",
    "miracleseed",
    "mysticwater",
    "nevermeltice",
    "oranberry",
    "pechaberry",
    "persimberry",
    "petayaberry",
    "poisonbarb",
    "quickclaw",
    "rawstberry",
    "salacberry",
    "scopelens",
    "seaincense",
    "sharpbeak",
    "shellbell",
    "silkscarf",
    "silverpowder",
    "sitrusberry",
    "softsand",
    "souldew",
    "spelltag",
    "starfberry",
    "stick",
    "thickclub",
    "twistedspoon",
    "whiteherb",
    "wikiberry",
];

/// What this mon is holding. Most mons hold nothing, the way most sets do.
fn pick_item(fz: &mut Fuzz) -> &'static str {
    let pick = *fz.pick(GEN3_ITEMS);
    if fz.chance(60) {
        ""
    } else {
        pick
    }
}

/// Which ability this mon walks in with. The draw always happens so the
/// generator's stream does not depend on which species came up, and it comes
/// out empty whenever the species has nothing we model.
/// A gender the species can actually have. Attract is the only thing that
/// reads it, and it reads it as a plain comparison — but the scenario states
/// it outright either way, since the sim otherwise rolls its own.
fn pick_gender(fz: &mut Fuzz, sp: &'static gen3_battle::data::SpeciesEntry) -> &'static str {
    match sp.gender {
        "" => {
            if fz.chance(50) {
                "F"
            } else {
                "M"
            }
        }
        fixed => fixed,
    }
}

fn pick_ability(fz: &mut Fuzz, sp: &'static gen3_battle::data::SpeciesEntry) -> &'static str {
    let slot = fz.below(2) as usize;
    let none = fz.chance(30);
    let known = |id: &'static str| GEN3_ABILITIES.contains(&id).then_some(id);
    let pair = [sp.abilities.0, sp.abilities.1];
    if none {
        return "";
    }
    known(pair[slot])
        .or_else(|| known(pair[1 - slot]))
        .unwrap_or("")
}

#[test]
fn fuzz_gen3_single_hits() {
    if !armed() {
        return;
    }
    use gen3_battle::{
        damage, hp_stat, other_stat, species_by_id, Attacker, Defender, Invest, MoveUse, Nature,
        Roll, Stat, SPECIES,
    };

    let moves = vanilla_moves(3, false, |id| {
        // One scripted hit only: charge moves spend the single turn
        // charging, a bind's end-of-turn chip would smear the number, and
        // the id-conditional powers resolve outside plain damage().
        gen3_battle::move_by_id(id)
            .map(|m| {
                m.power > 0
                    && !m.charge
                    && !m.trap
                    && m.self_drop.is_none()
                    && !matches!(
                        m.id,
                        "facade" | "smellingsalts" | "revenge" | "focuspunch" | "falseswipe"
                            | "eruption" | "waterspout" | "return" | "frustration" | "triattack"
                            | "brickbreak" | "endeavor" | "flail" | "reversal"
                            | "weatherball" | "spitup" | "highjumpkick" | "jumpkick"
                            | "secretpower" | "snore" | "uproar" | "bide" | "rollout"
                            | "iceball" | "hiddenpower" | "furycutter" | "rage"
                            | "thrash" | "petaldance" | "outrage" | "counter"
                            | "mirrorcoat" | "futuresight" | "doomdesire" | "magnitude"
                            | "psywave" | "dreameater" | "fakeout" | "present"
                            | "triplekick"
                    )
            })
            .unwrap_or(false)
    });
    assert!(
        moves.len() > 80 || std::env::var("FUZZ_ONLY").is_ok(),
        "gen3 vanilla pool too small: {}",
        moves.len()
    );
    // A narrowed pool (FUZZ_ONLY) can leave a suite with nothing of its own
    // generation to fuzz; that suite has nothing to say, so it stands down.
    if moves.is_empty() {
        return;
    }

    let mut fz = Fuzz::new("gen3-single-hits");
    let natures: Vec<Nature> = (0..25).map(Nature::from_index).collect();

    let mut scenarios = Vec::new();
    let mut cases = Vec::new();
    for _ in 0..fuzz_n() {
        let atk_sp = fz.pick(SPECIES);
        let def_sp = fz.pick(SPECIES);
        let level = 5 + fz.below(96) as u8;
        let (a_nat, d_nat) = (*fz.pick(&natures), *fz.pick(&natures));
        let (a_iv, a_ev) = (fz.below(32) as u8, (fz.below(64) * 4) as u8);
        let (d_iv, d_ev) = (fz.below(32) as u8, (fz.below(64) * 4) as u8);
        let (mv, _prio) = fz.pick(&moves).clone();
        // The sim refuses to burn a Fire type, so the scenario must too.
        let fire = {
            use gen3_battle::Type;
            atk_sp.types.0 == Type::Fire || atk_sp.types.1 == Type::Fire
        };
        let burned = !fire && fz.chance(20);
        let reflect = fz.chance(20);
        let light_screen = fz.chance(20);
        let stage = |fz: &mut Fuzz| (fz.below(13) as i8) - 6;
        let (atk_stage, spa_stage) = (stage(&mut fz), stage(&mut fz));
        let (def_stage, spd_stage) = (stage(&mut fz), stage(&mut fz));
        let hit = !fz.chance(10);
        let crit = fz.chance(25);
        let roll = 85 + fz.below(16) as u8;


        let mut conds: Vec<&str> = Vec::new();
        if reflect {
            conds.push("reflect");
        }
        if light_screen {
            conds.push("lightscreen");
        }
        scenarios.push(json!({
            "kind": "turn", "gen": 3,
            "p1": {
                "species": atk_sp.id, "level": level, "nature": a_nat.name(),
                "ivs": {"hp": a_iv, "atk": a_iv, "def": a_iv, "spa": a_iv, "spd": a_iv, "spe": a_iv},
                "evs": {"hp": a_ev, "atk": a_ev, "def": a_ev, "spa": a_ev, "spd": a_ev, "spe": a_ev},
                "status": if burned { Some("brn") } else { None },
                "boosts": {"atk": atk_stage, "spa": spa_stage},
            },
            "p2": {
                "species": def_sp.id, "level": level, "nature": d_nat.name(),
                "ivs": {"hp": d_iv, "atk": d_iv, "def": d_iv, "spa": d_iv, "spd": d_iv, "spe": d_iv},
                "evs": {"hp": d_ev, "atk": d_ev, "def": d_ev, "spa": d_ev, "spd": d_ev, "spe": d_ev},
                "boosts": {"def": def_stage, "spd": spd_stage},
                "sideConditions": conds,
            },
            "moves": [mv, "splash"],
            "script": {"p1": {"hit": hit, "crit": crit, "roll": roll},
                       "p2": {"hit": true, "crit": false, "roll": 100}},
        }));
        cases.push((
            atk_sp.id, def_sp.id, level, a_nat, d_nat, (a_iv, a_ev), (d_iv, d_ev), mv, burned,
            reflect, light_screen, [atk_stage, spa_stage, def_stage, spd_stage], hit, crit, roll,
        ));
    }

    let only = checked_case(cases.len(), "gen3-single-hits");
    let results = showdown(&Value::Array(scenarios.clone()), only);
    let mut bad = 0;
    for (i, (case, got)) in cases.iter().zip(&results).enumerate() {
        let (atk_id, def_id, level, a_nat, d_nat, (a_iv, a_ev), (d_iv, d_ev), mv, burned,
             reflect, light_screen, stages, hit, crit, roll) = case;
        if got.get("error").is_some() {
            eprintln!("HARNESS ERROR case {i}: {got}\n  scenario: {}", scenarios[i]);
            bad += 1;
            continue;
        }
        let a_sp = species_by_id(atk_id).unwrap();
        let d_sp = species_by_id(def_id).unwrap();
        let a_inv = Invest { iv: *a_iv, ev: *a_ev };
        let d_inv = Invest { iv: *d_iv, ev: *d_ev };
        let entry = gen3_battle::move_by_id(mv).unwrap();

        let mut attacker = Attacker {
            level: *level,
            atk: other_stat(a_sp.base.atk, a_inv, *level, *a_nat, Stat::Atk),
            sp_atk: other_stat(a_sp.base.spa, a_inv, *level, *a_nat, Stat::SpAtk),
            atk_stage: stages[0],
            sp_atk_stage: stages[1],
            types: a_sp.types,
            burned: *burned,
            stat_mod: gen3_battle::ability::Chain::new(),
            stat_pre: gen3_battle::ability::Chain::new(),
            ignores_burn: false,
        };
        let mut defender = Defender {
            def: other_stat(d_sp.base.def, d_inv, *level, *d_nat, Stat::Def),
            sp_def: other_stat(d_sp.base.spd, d_inv, *level, *d_nat, Stat::SpDef),
            def_stage: stages[2],
            sp_def_stage: stages[3],
            types: d_sp.types,
            reflect: *reflect,
            light_screen: *light_screen,
            stat_mod: gen3_battle::ability::Chain::new(),
        };
        // Beat Up is its own calc: each healthy ally strikes typeless with
        // its BASE Attack against the target's BASE Defence — no stages, no
        // burn — and Light Screen is the screen that counts. A statused
        // user (the only party member here) leaves nobody, and it fails.
        let mut move_type = entry.move_type;
        if *mv == "beatup" {
            attacker.sp_atk = a_sp.base.atk as u16;
            attacker.sp_atk_stage = 0;
            defender.sp_def = d_sp.base.def as u16;
            defender.sp_def_stage = 0;
            move_type = gen3_battle::Type::None;
        }
        let max_hp = hp_stat(d_sp.base.hp, d_inv, *level);
        // A never-miss move (accuracy 0) ignores the scripted miss, exactly
        // as the sim's accuracy step never runs for it.
        let hit = (*hit || entry.accuracy == 0) && !(*mv == "beatup" && *burned);
        let dealt = if hit {
            damage(&attacker, &defender, &MoveUse { move_type, power: entry.power, halve_def: entry.selfdestruct, late_mult: 1, special: *mv == "beatup", weather: 0, phase1: gen3_battle::ability::Chain::new() },
                   Roll { crit: *crit, random: *roll })
        } else {
            0
        };
        let ours_hp = max_hp.saturating_sub(dealt as u16);

        // Our side has run; the reference's answer is only wanted for the
        // case the filter named.
        if only.is_some_and(|k| k != i) {
            continue;
        }

        let theirs_hp = got["p2"]["hp"].as_u64().unwrap() as u16;
        let their_max = got["p2"]["maxhp"].as_u64().unwrap() as u16;
        if max_hp != their_max || ours_hp != theirs_hp {
            eprintln!(
                "GEN3 FUZZ MISMATCH case {i}: ours {ours_hp}/{max_hp} — showdown {theirs_hp}/{their_max}\n  scenario: {}\n  log: {}",
                scenarios[i], got["log"],
            );
            bad += 1;
        }
    }
    assert_eq!(bad, 0, "{bad}/{} gen3 fuzz cases disagree — replay with the seed above", cases.len());
}

// ── Gen 3: whole scripted turns — order, PP, faints, the win ─────────────────

#[test]
fn fuzz_gen3_turns() {
    if !armed() {
        return;
    }
    use gen3_battle::{
        battle::{SeatScript, Side, TurnScript},
        Battle, Choice, Event, Invest, Mon, Nature, SPECIES,
    };

    let moves = vanilla_moves(3, true, |id| {
        gen3_battle::move_by_id(id)
            .map(|m| {
                // Damaging is DATA, not "the power column is non-zero".
                // This filter used to make the engine's own mistake, so the
                // moves that compute their power in a callback (Return,
                // Hidden Power, Flail, Reversal, Low Kick, Magnitude,
                // Frustration, Present, Spit Up) were never once fuzzed —
                // and every one of them silently did nothing in play.
                m.damaging || m.status_action.is_some()
            })
            .unwrap_or(false)
    });
    let moves: Vec<String> = moves.into_iter().map(|(id, _)| id).collect();

    if moves.is_empty() {
        return;
    }
    let mut fz = Fuzz::new("gen3-turns");
    let mut scenarios = Vec::new();
    let mut cases = Vec::new();
    let statuses: [Option<&str>; 7] =
        [None, None, Some("brn"), Some("psn"), Some("par"), Some("frz"), Some("slp")];
    for _ in 0..fuzz_n() / 2 {
        let (s1, s2) = (fz.pick(SPECIES), fz.pick(SPECIES));
        let level = 40 + fz.below(61) as u8;
        let (m1, m2) = (fz.pick(&moves).clone(), fz.pick(&moves).clone());
        // A pre-set status must be legal for the species, or the sim refuses
        // it and the two sides diverge on setup rather than mechanics.
        let legal_status = |fz: &mut Fuzz, sp: &gen3_battle::SpeciesEntry| {
            use gen3_battle::{data::Status, Type};
            let pick = *fz.pick(&statuses);
            let has = |t: Type| sp.types.0 == t || sp.types.1 == t;
            // A Castform's type at the opening is decided by the sky, not
            // by its own entry in the dex: Forecast fires at switch-in, so a
            // Drought opposite makes it Fire before the harness gets round to
            // handing out the scenario's statuses — and the sim then refuses
            // a burn our engine has already applied. The type-dependent
            // statuses are simply not offered to it.
            let weather_forme = sp.id.starts_with("castform");
            match pick {
                Some("brn") | Some("frz") | Some("psn") if weather_forme => None,
                Some("brn") if has(Type::Fire) => None,
                Some("frz") if has(Type::Ice) => None,
                Some("psn") if has(Type::Poison) || has(Type::Steel) => None,
                other => other.map(|s| match s {
                    "brn" => (s, Status::Burn),
                    "psn" => (s, Status::Poison),
                    "par" => (s, Status::Paralysis),
                    "frz" => (s, Status::Freeze),
                    "slp" => (s, Status::Sleep),
                    _ => unreachable!(),
                }),
            }
        };
        let st1 = legal_status(&mut fz, s1);
        let st2 = legal_status(&mut fz, s2);
        let (ab1, ab2) = (pick_ability(&mut fz, s1), pick_ability(&mut fz, s2));
        let (it1, it2) = (pick_item(&mut fz), pick_item(&mut fz));
        let (g1, g2) = (pick_gender(&mut fz, s1), pick_gender(&mut fz, s2));
        // A pre-set PARALYSIS is not safe to combine with an opening that
        // cares about Speed order. The harness hands the statuses out after
        // `setPlayer` has already started the battle and sorted the opening
        // switch-ins, so the reference greets the field at unquartered Speed
        // while the engine, which is handed a paralysed mon to begin with,
        // greets it slow. The limitation is written up where it happens; here
        // the pair is simply not generated, because the mismatch it produces
        // says nothing about either engine.
        let order_sensitive = |ab: &str, item: &str| {
            matches!(
                ab,
                "intimidate" | "trace" | "drizzle" | "drought" | "sandstream" | "forecast"
            ) || item == "whiteherb"
        };
        let touchy = order_sensitive(ab1, it1) || order_sensitive(ab2, it2);
        let par = |st: Option<(&'static str, gen3_battle::data::Status)>| {
            st.filter(|s| !(touchy && s.0 == "par"))
        };
        let (st1, mut st2) = (par(st1), par(st2));
        // Transform copies the target's stats, Speed included, so the two
        // mons end the turn EXACTLY tied — and a tie makes the residual order
        // arbitrary. The reference resolves it by the swaps its selection
        // sort happens to make, with its tie-shuffle pinned to a no-op; the
        // real game shuffles. Neither answer is the rule, so the pairing that
        // exposes it — a Transform with both sides carrying a status to tick
        // — is not generated. One side statused is still fair game.
        if (m1 == "transform" || m2 == "transform") && st1.is_some() {
            st2 = None;
        }
        // 1-3 whole turns, each seat scripted per turn: hit, crit, roll,
        // secondary, immobile, hits, selfhit. The conditional knobs only
        // fire when their condition holds (paralysis, a 2-5 multi-hit move,
        // confusion), so generating them unconditionally is harmless and
        // covers conditions that land mid-battle.
        let n_turns = 1 + fz.below(3) as usize;
        let mut turns: Vec<(bool, [(bool, bool, u8, bool, bool, u8, bool, bool, bool); 2])> = Vec::new();
        for _ in 0..n_turns {
            let mut pair = [(true, false, 100u8, false, false, 0u8, false, false, false); 2];
            for (seat, slot) in pair.iter_mut().enumerate() {
                *slot = (
                    !fz.chance(10),
                    fz.chance(25),
                    85 + fz.below(16) as u8,
                    fz.chance(40),
                    fz.chance(15),
                    0,
                    fz.chance(50),
                    fz.chance(50),
                    fz.chance(50),
                );
                let mv = if seat == 0 { &m1 } else { &m2 };
                if let Some(e) = gen3_battle::move_by_id(mv) {
                    if e.multihit.is_some_and(|(lo, hi)| lo != hi) {
                        slot.5 = 2 + fz.below(4) as u8;
                    }
                }
            }
            // The turn's Quick Claw coin, one for the whole field.
            turns.push((fz.chance(30), pair));
        }

        let turn_json: Vec<Value> = turns
            .iter()
            .map(|(claw, pair)| {
                let seat = |t: &(bool, bool, u8, bool, bool, u8, bool, bool, bool)| {
                    json!({"hit": t.0, "crit": t.1, "roll": t.2, "secondary": t.3, "immobile": t.4, "hits": t.5, "selfhit": t.6, "stall": t.7, "band": t.8})
                };
                json!({"p1": seat(&pair[0]), "p2": seat(&pair[1]), "claw": claw})
            })
            .collect();
        scenarios.push(json!({
            "kind": "turn", "gen": 3,
            "p1": {"species": s1.id, "level": level, "status": st1.map(|s| s.0), "ability": ab1, "item": it1, "gender": g1},
            "p2": {"species": s2.id, "level": level, "status": st2.map(|s| s.0), "ability": ab2, "item": it2, "gender": g2},
            "moves": [m1, m2],
            "turns": turn_json,
        }));
        cases.push((
            s1.id,
            s2.id,
            level,
            m1,
            m2,
            turns,
            [st1.map(|s| s.1), st2.map(|s| s.1)],
            [ab1, ab2],
            [it1, it2],
            [g1, g2],
        ));
    }

    let only = checked_case(cases.len(), "gen3-turns");
    let results = showdown(&Value::Array(scenarios.clone()), only);
    let inv = Invest { iv: 31, ev: 0 };
    let mut bad = 0;
    for (i, ((s1, s2, level, m1, m2, turns, statuses, abilities, items, genders), got)) in
        cases.iter().zip(&results).enumerate()
    {
        if got.get("error").is_some() {
            eprintln!("HARNESS ERROR case {i}: {got}\n  scenario: {}", scenarios[i]);
            bad += 1;
            continue;
        }
        // Ability and status go on BEFORE the battle exists: constructing it
        // runs the opening switch-ins, and those read both.
        let mk = |id: &str,
                  mv: &str,
                  ability: &'static str,
                  item: &'static str,
                  gender: &'static str,
                  st: Option<_>| {
            let mut mon = Mon::new(id, *level, Nature::Hardy, inv, &[mv]).unwrap();
            mon.ability = ability;
            mon.item = item;
            mon.gender = gender;
            mon.status = st;
            if st == Some(gen3_battle::data::Status::Sleep) {
                // The sim's pinned duration roll: asleep for one skipped
                // action, awake on the second.
                mon.sleep_n = 2;
            }
            mon
        };
        let mut battle = Battle::new(
            Side::new(vec![mk(s1, m1, abilities[0], items[0], genders[0], statuses[0])]),
            Side::new(vec![mk(s2, m2, abilities[1], items[1], genders[1], statuses[1])]),
            1,
        );
        // Skip speed ties: the tie-break is each side's own RNG.
        if battle.sides[0].mon().spe == battle.sides[1].mon().spe {
            continue;
        }
        let mut our_order: Vec<&str> = Vec::new();
        // What our engine actually did, turn by turn. The battle suite has
        // carried this since it was written, and every turn case diagnosed
        // without it was diagnosed by guesswork.
        let mut our_log: Vec<String> = Vec::new();
        for (turn_i, (claw, pair)) in turns.iter().enumerate() {
            if battle.over() {
                break;
            }
            let seat = |t: &(bool, bool, u8, bool, bool, u8, bool, bool, bool)| SeatScript {
                hit: t.0,
                crit: t.1,
                random: t.2,
                secondary: t.3,
                immobile: t.4,
                hits: t.5,
                selfhit: t.6,
                stall: t.7,
                band: t.8,
            };
            let ts = TurnScript {
                seats: [Some(seat(&pair[0])), Some(seat(&pair[1]))],
                claw: *claw,
            };
            let events = battle.step_with([Choice::Move(0), Choice::Move(0)], &ts);
            our_log.push(format!("-- turn {turn_i}"));
            our_log.extend(events.iter().map(|e| format!("   {e:?}")));
            our_order.extend(events.iter().filter_map(|e| match e {
                Event::Used { side: 1, .. } => Some("p1"),
                Event::Used { side: 2, .. } => Some("p2"),
                _ => None,
            }));
        }
        // Our side has played the battle; the reference's answer is only
        // wanted for the case the filter named.
        if only.is_some_and(|k| k != i) {
            continue;
        }
        let their_order: Vec<&str> =
            got["order"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();

        let ok = ["p1", "p2"].iter().enumerate().all(|(s, _)| {
            let ours = &battle.sides[s];
            let want = &got[if s == 0 { "p1" } else { "p2" }];
            let our_status = ours.mon().status.map(|st| st.abbr());
            let their_status = want["status"].as_str();
            ours.mon().hp == want["hp"].as_u64().unwrap() as u16
                && ours.mon().max_hp == want["maxhp"].as_u64().unwrap() as u16
                && ours.mon().moves[0].pp as u64 == want["pp"][0].as_u64().unwrap()
                && ours.mon().fainted() == want["fainted"].as_bool().unwrap()
                && our_status == their_status
        }) && our_order == their_order;
        if !ok {
            eprintln!(
                "GEN3 TURN FUZZ MISMATCH case {i}: ours p1 {}/{} pp{} p2 {}/{} pp{} order {:?} — showdown {} / {} order {:?}\n  scenario: {}\n  log: {}",
                battle.sides[0].mon().hp, battle.sides[0].mon().max_hp, battle.sides[0].mon().moves[0].pp,
                battle.sides[1].mon().hp, battle.sides[1].mon().max_hp, battle.sides[1].mon().moves[0].pp,
                our_order, got["p1"], got["p2"], their_order, scenarios[i], got["log"],
            );
            eprintln!("  ours-log:\n{}", our_log.join("\n"));
            bad += 1;
        }
    }
    assert_eq!(bad, 0, "{bad} gen3 turn fuzz cases disagree — replay with the seed above");
}

/// One side-by-side-comparable line per party member, in the same shape the
/// harness reports, so a turn-by-turn diff is a string compare.
fn snapshot_of(battle: &gen3_battle::Battle) -> Vec<Vec<String>> {
    (0..2)
        .map(|seat| {
            let side = &battle.sides[seat];
            side.party
                .iter()
                .enumerate()
                .map(|(j, m)| {
                    format!(
                        "{}/{} {}{}",
                        m.hp,
                        m.max_hp,
                        m.status.map(|s| s.abbr()).unwrap_or("-"),
                        if side.active == j && !m.fainted() { " *" } else { "" },
                    )
                })
                .collect()
        })
        .collect()
}

// ── Gen 3: whole battles — teams, switching, played to a winner ──────────────

/// The first suite that exercises a party rather than a lone mon: two teams,
/// voluntary switches, faints, replacements, and the end of the battle.
///
/// The engine runs FIRST and records the concrete choice it made each turn;
/// only then does the reference sim replay that exact choice list. Generating
/// choices blind would have both sides guessing at legality from states that
/// may already have diverged, and every such guess would read as a mismatch.
#[test]
fn fuzz_gen3_battles() {
    if !armed() {
        return;
    }
    use gen3_battle::{
        battle::{SeatScript, Side, TurnScript},
        Battle, Choice, Event, Invest, Mon, Nature, SPECIES,
    };

    let moves = vanilla_moves(3, true, |id| {
        // A party changes what these moves MEAN. Each is currently modelled
        // as a no-op, which was indistinguishable from the truth while every
        // battle was one mon against one mon: Assist and Sleep Talk had no
        // other move to call, Roar and Whirlwind had nobody to drag in,
        // Baton Pass had nowhere to pass to. With a bench they all do real
        // work, and they are stubs again until their own pass. Follow Me
        // needs a partner to draw fire away from, so it waits on doubles.
        const PENDING: &[&str] = &["followme"];
        gen3_battle::move_by_id(id)
            .map(|m| {
                !PENDING.contains(&m.id) && (m.damaging || m.status_action.is_some())
            })
            .unwrap_or(false)
    });
    let mut moves: Vec<String> = moves.into_iter().map(|(id, _)| id).collect();

    if moves.is_empty() {
        return;
    }
    let mut fz = Fuzz::new("gen3-battles");
    let wide = fuzz_wide();
    // The sim's movelist carries Hidden Power once per type and every entry
    // comes back as `hiddenpower`, so the pool holds it seventeen times. The
    // narrow run keeps that weighting, as its stream depends on the pool's
    // length; the wide run draws each move once.
    if wide {
        let mut seen = std::collections::HashSet::new();
        moves.retain(|m| seen.insert(m.clone()));
    }
    let wide_replace = fuzz_wide_replace();
    let inv = Invest { iv: 31, ev: 0 };
    let statuses: [Option<&str>; 7] =
        [None, None, None, Some("brn"), Some("psn"), Some("par"), Some("slp")];
    // The wide run adds freeze, which the narrow list leaves out with no
    // reason given; the turn suite has always offered it.
    let wide_statuses: [Option<&str>; 7] =
        [None, None, Some("brn"), Some("psn"), Some("par"), Some("slp"), Some("frz")];
    // Hidden Power's type AND power now live on the mon, derived from the
    // IVs it was built with, so the test computes neither.
    let mut scenarios = Vec::new();
    let mut expected = Vec::new();
    let mut skipped = 0usize;

    for _ in 0..fuzz_n() / 4 {
        let team_n = 2 + fz.below(3) as usize; // 2..=4 per side
        let level = 40 + fz.below(61) as u8;
        let sizes = if wide {
            [1 + fz.below(6) as usize, 1 + fz.below(6) as usize]
        } else {
            [team_n; 2]
        };

        // Build both teams, then reject any battle where two mons could tie
        // on Speed: the tie-break is each side's own RNG and cannot be pinned.
        let mut teams: [Vec<Mon>; 2] = [Vec::new(), Vec::new()];
        let mut specs: [Vec<Value>; 2] = [Vec::new(), Vec::new()];
        for seat in 0..2 {
            for _ in 0..sizes[seat] {
                let sp = fz.pick(SPECIES);
                let n_moves = if wide { 1 + fz.below(4) as usize } else { 2 };
                let set = distinct_moves(&mut fz, &moves, n_moves);
                let st = {
                    use gen3_battle::Type;
                    let pick = *fz.pick(if wide { &wide_statuses } else { &statuses });
                    let has = |t: Type| sp.types.0 == t || sp.types.1 == t;
                    // See the turn suite: a Forecast Castform's type at the
                    // opening is the sky's, not the dex's, and the harness
                    // hands the statuses out after the opening has run.
                    let weather_forme = sp.id.starts_with("castform");
                    match pick {
                        Some("brn") | Some("frz") | Some("psn") if weather_forme => None,
                        Some("brn") if has(Type::Fire) => None,
                        Some("frz") if has(Type::Ice) => None,
                        Some("psn") if has(Type::Poison) || has(Type::Steel) => None,
                        other => other,
                    }
                };
                let ability = pick_ability(&mut fz, sp);
                let item = pick_item(&mut fz);
                let gender = pick_gender(&mut fz, sp);
                let level = if wide { 40 + fz.below(61) as u8 } else { level };
                let nature = if wide { Nature::from_index(fz.below(25) as u8) } else { Nature::Hardy };
                let spread = wide.then(|| wide_spread(&mut fz));
                let refs: Vec<&str> = set.iter().map(|m| m.as_str()).collect();
                let mut mon = match spread {
                    // The engine's own order for a per-stat spread is the
                    // sim's stat order hp, atk, def, spe, spa, spd;
                    // `wide_spread` deals in the spec order hp, atk, def,
                    // spa, spd, spe, so spe and spa swap on the way in.
                    // Stats, Hidden Power type and power all come out of it.
                    Some((ivs, evs)) => Mon::with_spread(
                        sp.id,
                        level,
                        nature,
                        &[
                            Invest { iv: ivs[0], ev: evs[0] },
                            Invest { iv: ivs[1], ev: evs[1] },
                            Invest { iv: ivs[2], ev: evs[2] },
                            Invest { iv: ivs[5], ev: evs[5] },
                            Invest { iv: ivs[3], ev: evs[3] },
                            Invest { iv: ivs[4], ev: evs[4] },
                        ],
                        &refs,
                    )
                    .unwrap(),
                    None => Mon::new(sp.id, level, nature, inv, &refs).unwrap(),
                };
                mon.ability = ability;
                mon.item = item;
                mon.gender = gender;
                mon.status = match st {
                    Some("brn") => Some(gen3_battle::data::Status::Burn),
                    Some("psn") => Some(gen3_battle::data::Status::Poison),
                    Some("par") => Some(gen3_battle::data::Status::Paralysis),
                    Some("slp") => Some(gen3_battle::data::Status::Sleep),
                    Some("frz") => Some(gen3_battle::data::Status::Freeze),
                    _ => None,
                };
                if mon.status == Some(gen3_battle::data::Status::Sleep) {
                    mon.sleep_n = 2;
                }
                let mut spec = json!({
                    "species": sp.id, "level": level, "moves": set,
                    "status": st, "ability": ability, "item": item, "gender": gender,
                });
                if let Some((ivs, evs)) = spread {
                    spec["nature"] = json!(nature.name());
                    spec["ivs"] = stat_json(ivs);
                    spec["evs"] = stat_json(evs);
                }
                specs[seat].push(spec);
                teams[seat].push(mon);
            }
        }
        // Same pre-set-status limitation the turn suite already dodges, and
        // for the same reason: the harness applies the statuses after
        // `setPlayer` has started the battle, so the two OPENERS greet the
        // field at unquartered Speed while the engine greets them already
        // paralysed. Only slot 0 is exposed — a benched mon is statused long
        // before its own `runSwitch` runs. The status was picked before the
        // ability and the item, so the pair is broken up afterwards rather
        // than declined at pick time, which would shift the RNG stream.
        let order_sensitive = |m: &Mon| {
            matches!(
                m.ability,
                "intimidate" | "trace" | "drizzle" | "drought" | "sandstream" | "forecast"
            ) || m.item == "whiteherb"
        };
        if teams.iter().any(|t| order_sensitive(&t[0])) {
            for seat in 0..2 {
                if teams[seat][0].status == Some(gen3_battle::data::Status::Paralysis) {
                    teams[seat][0].status = None;
                    specs[seat][0]["status"] = Value::Null;
                }
            }
        }
        let mut speeds: Vec<u16> =
            teams.iter().flat_map(|t| t.iter().map(|m| m.spe)).collect();
        speeds.sort_unstable();
        let before = speeds.len();
        speeds.dedup();
        if speeds.len() != before {
            skipped += 1;
            continue;
        }

        let [t0, t1] = teams;
        let mut battle = Battle::new(Side::new(t0), Side::new(t1), 1);
        // `FUZZ_WIDE=2` answers forced replacements itself, through the same
        // `send_out` a real game uses, rather than always sending the lowest
        // living slot. See `fuzz_wide` for why level 1 does not.
        battle.player_picks_replacement = wide_replace;

        // Play the engine, recording what it actually chose each turn.
        let n_turns = if wide { 2 + fz.below(29) as usize } else { 2 + fz.below(6) as usize };
        let mut turn_json: Vec<Value> = Vec::new();
        let mut our_order: Vec<&str> = Vec::new();
        let mut our_log: Vec<String> = Vec::new();
        let mut our_states: Vec<Vec<Vec<String>>> = Vec::new();
        for _ in 0..n_turns {
            if battle.over() {
                break;
            }
            let mut choices = [Choice::Move(0); 2];
            let mut seats: [Value; 2] = [Value::Null, Value::Null];
            for seat in 0..2 {
                // A living, benched party member to switch to, if any.
                let bench: Vec<usize> = (0..battle.sides[seat].party.len())
                    .filter(|&i| i != battle.sides[seat].active)
                    .filter(|&i| !battle.sides[seat].party[i].fainted())
                    .collect();
                // Only offer a switch the sim would let a player pick: a
                // locked or held mon has `trapped` set on its request and
                // the choice is rejected outright.
                // Draw the coin unconditionally so the generator's stream
                // does not depend on battle state.
                let want_switch = fz.chance(25);
                let switching =
                    !bench.is_empty() && battle.can_switch(seat) && want_switch;
                let pick = fz.below(2) as usize;
                let wide_pick = if wide { fz.below(24) as usize } else { 0 };
                let slot = if switching {
                    *fz.pick(&bench)
                } else {
                    // Disable, Taunt and Torment grey a move out in the
                    // sim's request; choosing it is rejected, not
                    // reinterpreted.
                    let usable = battle.selectable_moves(seat);
                    if wide && !usable.is_empty() {
                        usable[wide_pick % usable.len()]
                    } else if usable.contains(&pick) {
                        pick
                    } else {
                        // With nothing usable at all the sim's request
                        // carries a single entry, Struggle, so the only
                        // choice a player can send back is the first one.
                        usable.first().copied().unwrap_or(0)
                    }
                };
                choices[seat] =
                    if switching { Choice::Switch(slot) } else { Choice::Move(slot) };
                // The 2-5 strike count, drawn per seat rather than per move
                // so Mimic or Metronome changing what lands cannot desync it.
                // Both sides only read it for a 2-5 move.
                let hits = if wide { 2 + fz.below(4) } else { 0 };
                seats[seat] = json!({
                    "action": if switching { "switch" } else { "move" },
                    "slot": slot,
                    "hit": !fz.chance(10),
                    "crit": fz.chance(20),
                    "roll": 85 + fz.below(16) as u8,
                    "secondary": fz.chance(40),
                    "immobile": fz.chance(15),
                    "hits": hits,
                    "selfhit": fz.chance(50),
                    "stall": fz.chance(50),
                    "band": fz.chance(50),
                });
            }
            // One Quick Claw coin serves the whole field for the turn.
            let claw = fz.chance(30);
            // Who comes in after a faint: one draw per seat, every turn, so
            // the stream does not depend on whether anything fainted. Each
            // ask in the turn (Spikes can ask twice) reads its own byte.
            let replace_draw = if wide { [fz.next(), fz.next()] } else { [0, 0] };
            // Who a Baton Pass brings in, drawn the same way but only at
            // level 2, the one level where the engine asks for it: levels 0
            // and 1 keep their streams.
            let pass_draw = if wide_replace { [fz.next(), fz.next()] } else { [0, 0] };
            let seat_script = |v: &Value| SeatScript {
                hit: v["hit"].as_bool().unwrap(),
                crit: v["crit"].as_bool().unwrap(),
                random: v["roll"].as_u64().unwrap() as u8,
                secondary: v["secondary"].as_bool().unwrap(),
                immobile: v["immobile"].as_bool().unwrap(),
                hits: v["hits"].as_u64().unwrap() as u8,
                selfhit: v["selfhit"].as_bool().unwrap(),
                stall: v["stall"].as_bool().unwrap(),
                band: v["band"].as_bool().unwrap(),
            };
            let ts = TurnScript {
                seats: [Some(seat_script(&seats[0])), Some(seat_script(&seats[1]))],
                claw,
            };
            let mut events = battle.step_with(choices, &ts);
            let mut replaced: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
            let mut passed: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
            let mut guard = 0;
            while wide_replace && !battle.over() && guard < 12 {
                guard += 1;
                let pending = battle.pending_replacements();
                if pending.iter().all(|p| p.is_none()) {
                    break;
                }
                let pass = battle.pending_pass();
                let mut picks: [Option<usize>; 2] = [None, None];
                for seat in 0..2 {
                    if let Some(live) = &pending[seat] {
                        // A seat moves once a turn, so it passes at most once.
                        let (draw, book) = if pass[seat] {
                            (pass_draw[seat], &mut passed[seat])
                        } else {
                            (replace_draw[seat], &mut replaced[seat])
                        };
                        let byte = draw >> (8 * (book.len() % 8)) & 0xff;
                        let j = live[byte as usize % live.len()];
                        book.push(j);
                        picks[seat] = Some(j);
                    }
                }
                battle.send_out(picks, &mut events);
            }
            if wide_replace {
                for seat in 0..2 {
                    seats[seat]["replace"] = json!(replaced[seat]);
                    seats[seat]["pass"] = json!(passed[seat]);
                }
            }
            our_states.push(snapshot_of(&battle));
            our_log.push(format!("-- turn {}: p1 {:?} p2 {:?}", turn_json.len(), choices[0], choices[1]));
            our_log.extend(events.iter().map(|e| format!("   {e:?}")));
            our_order.extend(events.iter().filter_map(|e| match e {
                Event::Used { side: 1, .. } => Some("p1"),
                Event::Used { side: 2, .. } => Some("p2"),
                _ => None,
            }));
            turn_json.push(json!({"p1": seats[0], "p2": seats[1], "claw": claw}));
        }

        scenarios.push(json!({
            "kind": "battle", "gen": 3,
            "p1": {"team": specs[0]},
            "p2": {"team": specs[1]},
            "turns": turn_json,
        }));
        expected.push((battle, our_order, our_log, our_states));
    }

    if skipped > 0 {
        eprintln!("[gen3-battles] {skipped} cases skipped on a Speed tie");
    }
    let only = checked_case(expected.len(), "gen3-battles");
    let results = showdown(&Value::Array(scenarios.clone()), only);
    let mut bad = 0;
    let mut crashed = 0;
    for (i, ((battle, our_order, our_log, our_states), got)) in expected.iter().zip(&results).enumerate() {
        // Our side has played every case's battle; only the case the filter
        // named is compared.
        if only.is_some_and(|k| k != i) {
            continue;
        }
        if got.get("error").is_some() {
            eprintln!("HARNESS ERROR case {i}: {got}\n  scenario: {}", scenarios[i]);
            bad += 1;
            continue;
        }
        // Collect every disagreement by name rather than folding them into
        // one boolean: a battle has forty-odd comparable numbers and "these
        // two states differ" is not a finding.
        let mut diffs: Vec<String> = Vec::new();
        for e in got["errors"].as_array().map(|a| a.as_slice()).unwrap_or(&[]) {
            diffs.push(format!("harness rejected a choice: {e}"));
        }
        for seat in 0..2 {
            let who = if seat == 0 { "p1" } else { "p2" };
            let side = &battle.sides[seat];
            let theirs = got[who].as_array().unwrap();
            if theirs.len() != side.party.len() {
                diffs.push(format!("{who} party size {} vs {}", side.party.len(), theirs.len()));
                continue;
            }
            for (j, mon) in side.party.iter().enumerate() {
                let want = &theirs[j];
                let mut cmp = |field: &str, ours: String, theirs: String| {
                    if ours != theirs {
                        diffs.push(format!("{who}[{j}].{field}: ours {ours} vs sim {theirs}"));
                    }
                };
                cmp("hp", mon.hp.to_string(), want["hp"].to_string());
                cmp("maxhp", mon.max_hp.to_string(), want["maxhp"].to_string());
                cmp("fainted", mon.fainted().to_string(), want["fainted"].to_string());
                cmp(
                    "status",
                    format!("{:?}", mon.status.map(|s| s.abbr())),
                    format!("{:?}", want["status"].as_str()),
                );
                // The sim drops `isActive` the moment a mon faints; the
                // engine's `active` index just stays put when there is
                // nobody left to send in, so compare what both mean.
                cmp(
                    "active",
                    (side.active == j && !mon.fainted()).to_string(),
                    want["active"].to_string(),
                );
                let their_pp = want["pp"].as_array().unwrap();
                for (k, ms) in mon.moves.iter().enumerate() {
                    cmp(
                        &format!("pp[{}]", ms.entry.id),
                        ms.pp.to_string(),
                        their_pp.get(k).map(|v| v.to_string()).unwrap_or_else(|| "-".into()),
                    );
                }
            }
        }
        let their_order: Vec<&str> =
            got["order"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
        if *our_order != their_order {
            diffs.push(format!("order: ours {our_order:?} vs sim {their_order:?}"));
        }
        // Everything after the first parting of ways is an echo of it, so
        // name the turn it happened on.
        if let Some(states) = got["states"].as_array() {
            for (t, theirs) in states.iter().enumerate() {
                let Some(ours) = our_states.get(t) else { break };
                let mut turn_diff = Vec::new();
                for (seat, who) in ["p1", "p2"].iter().enumerate() {
                    let side = theirs[*who].as_array().unwrap();
                    for (j, mon) in side.iter().enumerate() {
                        let sim = format!(
                            "{}/{} {}{}",
                            mon["hp"], mon["maxhp"],
                            mon["status"].as_str().unwrap_or("-"),
                            if mon["active"].as_bool().unwrap_or(false) { " *" } else { "" },
                        );
                        if ours[seat].get(j) != Some(&sim) {
                            turn_diff.push(format!(
                                "{who}[{j}] ours {:?} vs sim {sim:?}",
                                ours[seat].get(j)
                            ));
                        }
                    }
                }
                if !turn_diff.is_empty() {
                    diffs.push(format!("FIRST DIVERGENCE on turn {t}: {}", turn_diff.join("; ")));
                    break;
                }
            }
        }
        // The reference crashed mid-battle: keep only what it did play.
        if sim_crashed(got) {
            diffs.retain(|d| d.starts_with("FIRST DIVERGENCE"));
            if diffs.is_empty() {
                crashed += 1;
                eprintln!("[gen3-battles] case {i}: the sim crashed ({}); compared up to the crash", got["errors"]);
                continue;
            }
        }
        let ok = diffs.is_empty();
        if !ok {
            let ours: Vec<String> = (0..2)
                .map(|s| {
                    let side = &battle.sides[s];
                    format!(
                        "[{}]",
                        side.party
                            .iter()
                            .enumerate()
                            .map(|(j, m)| format!(
                                "{}{}/{}{}",
                                if side.active == j { "*" } else { "" },
                                m.hp,
                                m.max_hp,
                                m.status.map(|st| format!(" {}", st.abbr())).unwrap_or_default()
                            ))
                            .collect::<Vec<_>>()
                            .join(" ")
                    )
                })
                .collect();
            eprintln!(
                "GEN3 BATTLE FUZZ MISMATCH case {i}: ours {} {} order {:?}\n  showdown {} / {} order {:?} errors {}\n  scenario: {}\n  log: {}",
                ours[0], ours[1], our_order,
                got["p1"], got["p2"], their_order, got["errors"],
                scenarios[i], got["log"],
            );
            eprintln!("  diffs:\n    {}", diffs.join("\n    "));
            eprintln!("  ours-log:\n{}", our_log.join("\n"));
            bad += 1;
        }
    }
    if crashed > 0 {
        eprintln!("[gen3-battles] {crashed} cases where the sim crashed, compared up to the crash");
    }
    assert_eq!(bad, 0, "{bad} gen3 battle fuzz cases disagree — replay with the seed above");
}
