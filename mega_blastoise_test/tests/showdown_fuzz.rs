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

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{json, Value};

fn harness_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/showdown-parity")
}

fn armed() -> bool {
    if harness_dir().join("node_modules").exists() {
        return true;
    }
    eprintln!("SKIP: showdown fuzz not armed — npm install in tools/showdown-parity");
    false
}

fn showdown(scenarios: &Value) -> Vec<Value> {
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
        let seed = std::env::var("FUZZ_SEED")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0xF00D_2026);
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

fn fuzz_n() -> usize {
    std::env::var("FUZZ_N").ok().and_then(|s| s.parse().ok()).unwrap_or(200)
}

/// Vanilla move ids for a gen, intersected with our table so both sides can
/// run every sampled move. Returns (id, priority). `multihit` keeps or drops
/// the multi-strike moves: the single-hit suites compare one strike's damage,
/// so a move the sim lands twice would only measure the miscount.
fn vanilla_moves(gen: u8, multihit: bool, ours: impl Fn(&str) -> bool) -> Vec<(String, i8)> {
    let results = showdown(&json!([{"kind": "movelist", "gen": gen}]));
    results[0]["moves"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|m| multihit || !m["multihit"].as_bool().unwrap_or(false))
        .map(|m| (m["id"].as_str().unwrap().to_string(), m["priority"].as_i64().unwrap() as i8))
        .filter(|(id, _)| ours(id))
        .collect()
}

// ── Gen 3: single scripted hits, the full damage surface ─────────────────────

/// The Gen 3 abilities the engine models, and so the ones the fuzzer is
/// allowed to hand out. Everything outside this list stays off both sides:
/// the reference sim would play it and we would not, and every battle it
/// touched would diverge for a reason we already know about.
///
/// Still to come: Forecast, which needs Castform's weather formes.
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
/// Still to come: King's Rock, Quick Claw, White Herb, Mental Herb, Leppa
/// Berry, and the two moves that move items about — Trick and Recycle.
const GEN3_ITEMS: &[&str] = &[
    "apicotberry",
    "aspearberry",
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
    "ganlonberry",
    "hardstone",
    "lansatberry",
    "laxincense",
    "leftovers",
    "liechiberry",
    "lightball",
    "luckypunch",
    "machobrace",
    "magnet",
    "metalcoat",
    "metalpowder",
    "miracleseed",
    "mysticwater",
    "nevermeltice",
    "oranberry",
    "pechaberry",
    "petayaberry",
    "poisonbarb",
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
    assert!(moves.len() > 80, "gen3 vanilla pool too small: {}", moves.len());

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

    let results = showdown(&Value::Array(scenarios.clone()));
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
                m.power > 0
                    || m.status_action.is_some()
                    || m.fixed.is_some()
                    || m.ohko
                    || matches!(m.id, "counter" | "mirrorcoat")
            })
            .unwrap_or(false)
    });
    let moves: Vec<String> = moves.into_iter().map(|(id, _)| id).collect();

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
            match pick {
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
        // 1-3 whole turns, each seat scripted per turn: hit, crit, roll,
        // secondary, immobile, hits, selfhit. The conditional knobs only
        // fire when their condition holds (paralysis, a 2-5 multi-hit move,
        // confusion), so generating them unconditionally is harmless and
        // covers conditions that land mid-battle.
        let n_turns = 1 + fz.below(3) as usize;
        let mut turns: Vec<[(bool, bool, u8, bool, bool, u8, bool, bool); 2]> = Vec::new();
        for _ in 0..n_turns {
            let mut pair = [(true, false, 100u8, false, false, 0u8, false, false); 2];
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
                );
                let mv = if seat == 0 { &m1 } else { &m2 };
                if let Some(e) = gen3_battle::move_by_id(mv) {
                    if e.multihit.is_some_and(|(lo, hi)| lo != hi) {
                        slot.5 = 2 + fz.below(4) as u8;
                    }
                }
            }
            turns.push(pair);
        }

        let turn_json: Vec<Value> = turns
            .iter()
            .map(|pair| {
                let seat = |t: &(bool, bool, u8, bool, bool, u8, bool, bool)| {
                    json!({"hit": t.0, "crit": t.1, "roll": t.2, "secondary": t.3, "immobile": t.4, "hits": t.5, "selfhit": t.6, "stall": t.7})
                };
                json!({"p1": seat(&pair[0]), "p2": seat(&pair[1])})
            })
            .collect();
        scenarios.push(json!({
            "kind": "turn", "gen": 3,
            "p1": {"species": s1.id, "level": level, "status": st1.map(|s| s.0), "ability": ab1, "item": it1},
            "p2": {"species": s2.id, "level": level, "status": st2.map(|s| s.0), "ability": ab2, "item": it2},
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
        ));
    }

    let results = showdown(&Value::Array(scenarios.clone()));
    let inv = Invest { iv: 31, ev: 0 };
    let mut bad = 0;
    for (i, ((s1, s2, level, m1, m2, turns, statuses, abilities, items), got)) in
        cases.iter().zip(&results).enumerate()
    {
        if got.get("error").is_some() {
            eprintln!("HARNESS ERROR case {i}: {got}\n  scenario: {}", scenarios[i]);
            bad += 1;
            continue;
        }
        // Ability and status go on BEFORE the battle exists: constructing it
        // runs the opening switch-ins, and those read both.
        let mk = |id: &str, mv: &str, ability: &'static str, item: &'static str, st: Option<_>| {
            let mut mon = Mon::new(id, *level, Nature::Hardy, inv, &[mv]).unwrap();
            mon.ability = ability;
            mon.item = item;
            mon.status = st;
            if st == Some(gen3_battle::data::Status::Sleep) {
                // The sim's pinned duration roll: asleep for one skipped
                // action, awake on the second.
                mon.sleep_n = 2;
            }
            mon
        };
        let mut battle = Battle::new(
            Side::new(vec![mk(s1, m1, abilities[0], items[0], statuses[0])]),
            Side::new(vec![mk(s2, m2, abilities[1], items[1], statuses[1])]),
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
        for (turn_i, pair) in turns.iter().enumerate() {
            if battle.over() {
                break;
            }
            let seat = |t: &(bool, bool, u8, bool, bool, u8, bool, bool)| SeatScript {
                hit: t.0,
                crit: t.1,
                random: t.2,
                secondary: t.3,
                immobile: t.4,
                hits: t.5,
                selfhit: t.6,
                stall: t.7,
            };
            let ts = TurnScript { seats: [Some(seat(&pair[0])), Some(seat(&pair[1]))] };
            let events = battle.step_with([Choice::Move(0), Choice::Move(0)], &ts);
            our_log.push(format!("-- turn {turn_i}"));
            our_log.extend(events.iter().map(|e| format!("   {e:?}")));
            our_order.extend(events.iter().filter_map(|e| match e {
                Event::Used { side: 1, .. } => Some("p1"),
                Event::Used { side: 2, .. } => Some("p2"),
                _ => None,
            }));
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

// ── Gen 1: single scripted hits at the engine's fixed spread ─────────────────

#[test]
fn fuzz_gen1_single_hits() {
    if !armed() {
        return;
    }
    use gen1_battle::testing::{compute_damage_scripted, Mon};

    let moves = vanilla_moves(1, false, |id| {
        use gen1_battle::MoveEffectKind::{
            Counter, CrashOnMiss, DreamEater, FlatDamage, HalfHp, LevelDamage, Ohko, Psywave,
            Rage, ThrashLock, TwoTurn, Wrap,
        };
        gen1_battle::move_by_id(id)
            .map(|m| {
                // Formula damage only: fixed-damage and OHKO moves resolve
                // outside compute_damage, which is all this suite runs — and
                // charge moves spend the suite's single turn charging.
                m.power > 0
                    && !matches!(
                        m.effect_kind,
                        FlatDamage | LevelDamage | HalfHp | Ohko | TwoTurn | Counter | DreamEater
                            | Wrap | ThrashLock | CrashOnMiss | Rage | Psywave
                    )
            })
            .unwrap_or(false)
    });
    assert!(moves.len() > 30, "gen1 vanilla pool too small: {}", moves.len());

    // Sample species from our own dex ids via the randbat pool's species and
    // the classic 151 by trying dex ids straight off Showdown's list is
    // unnecessary: our SPECIES table is authoritative for what we can build.
    let species: Vec<&'static str> = gen1_battle::SPECIES.iter().map(|s| s.id).collect();

    let mut fz = Fuzz::new("gen1-single-hits");
    let mut scenarios = Vec::new();
    let mut cases = Vec::new();
    for _ in 0..fuzz_n() {
        let atk = *fz.pick(&species);
        let def = *fz.pick(&species);
        let level = 5 + fz.below(96) as u8;
        let (mv, _) = fz.pick(&moves).clone();
        let hit = !fz.chance(10);
        let crit = fz.chance(25);
        let roll = 217 + fz.below(39) as u8;

        scenarios.push(json!({
            "kind": "turn", "gen": 1,
            "p1": {"species": atk, "level": level,
                   "ivs": {"hp": 30, "atk": 30, "def": 30, "spa": 30, "spd": 30, "spe": 30},
                   "evs": {"hp": 255, "atk": 255, "def": 255, "spa": 255, "spd": 255, "spe": 255}},
            "p2": {"species": def, "level": level,
                   "ivs": {"hp": 30, "atk": 30, "def": 30, "spa": 30, "spd": 30, "spe": 30},
                   "evs": {"hp": 255, "atk": 255, "def": 255, "spa": 255, "spd": 255, "spe": 255}},
            "moves": [mv, "splash"],
            "script": {"p1": {"hit": hit, "crit": crit, "roll": roll},
                       "p2": {"hit": true, "crit": false, "roll": 255}},
        }));
        cases.push((atk, def, level, mv, hit, crit, roll));
    }

    let results = showdown(&Value::Array(scenarios.clone()));
    let mut bad = 0;
    for (i, ((atk, def, level, mv, hit, crit, roll), got)) in cases.iter().zip(&results).enumerate() {
        if got.get("error").is_some() {
            eprintln!("HARNESS ERROR case {i}: {got}\n  scenario: {}", scenarios[i]);
            bad += 1;
            continue;
        }
        // The engine's constructor wants 'static ids and move names; the
        // case tuple only borrows them, so resolve back to the tables.
        let mv_entry = gen1_battle::move_by_id(mv).unwrap();
        let attacker = Mon::from_species(*atk, *level, &[mv_entry.id]).unwrap();
        let defender = Mon::from_species(*def, *level, &["splash"]).unwrap();
        let hit = *hit || mv_entry.accuracy == 0;
        let boom = matches!(mv_entry.effect_kind, gen1_battle::MoveEffectKind::SelfDestruct);
        let dealt = if hit {
            compute_damage_scripted(&attacker, &defender, mv_entry, boom, *crit, *roll).dmg
        } else {
            0
        };
        let ours_hp = defender.hp_max.saturating_sub(dealt);

        let theirs_hp = got["p2"]["hp"].as_u64().unwrap() as u16;
        let their_max = got["p2"]["maxhp"].as_u64().unwrap() as u16;
        if defender.hp_max != their_max || ours_hp != theirs_hp {
            eprintln!(
                "GEN1 FUZZ MISMATCH case {i}: ours {ours_hp}/{} — showdown {theirs_hp}/{their_max}\n  scenario: {}\n  log: {}",
                defender.hp_max, scenarios[i], got["log"],
            );
            bad += 1;
        }
    }
    assert_eq!(bad, 0, "{bad}/{} gen1 fuzz cases disagree — replay with the seed above", cases.len());
}

// ── Gen 1: whole scripted turns through the full cartridge engine ────────────

#[test]
fn fuzz_gen1_turns() {
    if !armed() {
        return;
    }
    use gen1_battle::testing::{apply_status_drop, SeatForce, Status as G1Status};
    use gen1_battle::{MonData, MoveSlot as G1MoveSlot, PublicCoreBattle, TeamData};
    use mega_blastoise_core::{battle_options_with_seed, demo_engine_opts, FlashDataStore};

    let moves = vanilla_moves(1, false, |id| gen1_battle::move_by_id(id).is_some());
    assert!(moves.len() > 30, "gen1 turn pool too small: {}", moves.len());
    let species: Vec<&'static str> = gen1_battle::SPECIES.iter().map(|s| s.id).collect();

    let mut fz = Fuzz::new("gen1-turns");
    let mut scenarios = Vec::new();
    let mut cases = Vec::new();
    let statuses: [Option<&str>; 7] =
        [None, None, Some("brn"), Some("psn"), Some("par"), Some("frz"), Some("slp")];
    for _ in 0..fuzz_n() / 2 {
        let (s1, s2) = (*fz.pick(&species), *fz.pick(&species));
        let level = 40 + fz.below(61) as u8;
        let (m1, _) = fz.pick(&moves).clone();
        let (m2, _) = fz.pick(&moves).clone();
        // Transform against Transform-only is the sim's Endless Battle
        // Clause: it ties at turn zero, which measures the clause, not the
        // engine. Mimic-of-Transform manufactures the same dead end one
        // turn later. Skip those matchups.
        let ebc = |a: &str, b: &str| a == "transform" && (b == "transform" || b == "mimic");
        if ebc(&m1, &m2) || ebc(&m2, &m1) {
            continue;
        }
        // Same legality rule as gen 3, minus Steel which does not exist yet.
        let legal_status = |fz: &mut Fuzz, id: &str| {
            let sp = gen1_battle::SPECIES.iter().find(|s| s.id == id).unwrap();
            let pick = *fz.pick(&statuses);
            let has = |t: gen1_battle::Type| sp.primary_type == t || sp.secondary_type == t;
            match pick {
                Some("brn") if has(gen1_battle::Type::Fire) => None,
                Some("frz") if has(gen1_battle::Type::Ice) => None,
                Some("psn") if has(gen1_battle::Type::Poison) => None,
                other => other,
            }
        };
        let mut st1 = legal_status(&mut fz, s1);
        let mut st2 = legal_status(&mut fz, s2);
        // The RBY thaw glitch — a mon thawed mid-turn by a Fire move attacks
        // with a 102 BP ???-typed glitch move — is not modelled yet, so
        // freeze setups stay away from Fire moves until it is.
        // Mimic, Mirror Move and Metronome can DELIVER a Fire move (even
        // the frozen mon's own, mimicked back at it), so they count too.
        let fire = |m: &str| {
            gen1_battle::move_by_id(m)
                .map(|e| e.move_type == gen1_battle::Type::Fire)
                .unwrap_or(false)
                || matches!(m, "mimic" | "mirrormove" | "metronome")
        };
        if st1 == Some("frz") && (fire(&m2) || fire(&m1)) {
            st1 = None;
        }
        if st2 == Some("frz") && (fire(&m1) || fire(&m2)) {
            st2 = None;
        }
        let n_turns = 1 + fz.below(3) as usize;
        let mut turns: Vec<[(bool, bool, u8, bool, bool, u8, bool); 2]> = Vec::new();
        for _ in 0..n_turns {
            let mut pair = [(true, false, 255u8, false, false, 0u8, false); 2];
            for (seat, slot) in pair.iter_mut().enumerate() {
                *slot = (
                    !fz.chance(10),
                    fz.chance(25),
                    217 + fz.below(39) as u8,
                    fz.chance(40),
                    fz.chance(15),
                    0,
                    fz.chance(50),
                );
                // Script the 2-5 strike count so both sides land the same.
                let mv = if seat == 0 { &m1 } else { &m2 };
                if gen1_battle::move_by_id(mv)
                    .is_some_and(|e| e.effect_kind == gen1_battle::MoveEffectKind::MultiHit2to5)
                {
                    slot.5 = 2 + fz.below(4) as u8;
                }
            }
            turns.push(pair);
        }

        let dvs = json!({"hp": 30, "atk": 30, "def": 30, "spa": 30, "spd": 30, "spe": 30});
        let exp = json!({"hp": 255, "atk": 255, "def": 255, "spa": 255, "spd": 255, "spe": 255});
        let turn_json: Vec<Value> = turns
            .iter()
            .map(|pair| {
                let seat = |t: &(bool, bool, u8, bool, bool, u8, bool)| {
                    json!({"hit": t.0, "crit": t.1, "roll": t.2, "secondary": t.3, "immobile": t.4, "hits": t.5, "selfhit": t.6})
                };
                json!({"p1": seat(&pair[0]), "p2": seat(&pair[1])})
            })
            .collect();
        scenarios.push(json!({
            "kind": "turn", "gen": 1,
            "p1": {"species": s1, "level": level, "status": st1, "ivs": dvs, "evs": exp},
            "p2": {"species": s2, "level": level, "status": st2, "ivs": dvs, "evs": exp},
            "moves": [m1, m2],
            "turns": turn_json,
        }));
        cases.push((s1, s2, level, m1, m2, turns, [st1, st2]));
    }

    let results = showdown(&Value::Array(scenarios.clone()));
    let data = FlashDataStore::new();
    let mut bad = 0;
    for (i, ((s1, s2, level, m1, m2, turns, statuses), got)) in cases.iter().zip(&results).enumerate() {
        if got.get("error").is_some() {
            eprintln!("HARNESS ERROR case {i}: {got}\n  scenario: {}", scenarios[i]);
            bad += 1;
            continue;
        }
        let mon_data = |id: &str, mv: &str| MonData {
            name: String::new(),
            species: gen1_battle::SPECIES.iter().find(|s| s.id == id).unwrap().name.to_string(),
            level: *level,
            moves: vec![G1MoveSlot {
                name: String::from(mv),
                id: String::from(mv),
                typ: String::new(),
                pp: 0,
                max_pp: 0,
                disabled: false,
                target: 0,
            }],
            ivs: Default::default(),
            evs: Default::default(),
            gender: None,
            nature: None,
            ability: Some(String::from("No Ability")),
            item: None,
        };
        let mut battle =
            PublicCoreBattle::new(battle_options_with_seed(1), &data, demo_engine_opts()).unwrap();
        battle.update_team("p1", TeamData { members: vec![mon_data(s1, m1)], ..Default::default() }).unwrap();
        battle.update_team("p2", TeamData { members: vec![mon_data(s2, m2)], ..Default::default() }).unwrap();
        battle.start().unwrap();
        {
            let sides = battle.testing_sides_mut();
            for (seat, st) in statuses.iter().enumerate() {
                let mon = sides[seat].active_mut();
                mon.status = match st {
                    None => G1Status::None,
                    Some("brn") => G1Status::Burn,
                    Some("psn") => G1Status::Poison,
                    Some("par") => G1Status::Paralysis,
                    Some("frz") => G1Status::Freeze,
                    // The sim's pinned duration roll: random(1,8) bottoms at 1.
                    Some("slp") => G1Status::Sleep(1),
                    _ => unreachable!(),
                };
                apply_status_drop(mon);
            }
            // Skip speed ties on the post-status modified stat: the
            // tie-break is each side's own RNG.
            if sides[0].active().modified[4] == sides[1].active().modified[4] {
                continue;
            }
        }
        let _ = battle.new_log_entries().count(); // drop the setup log

        let mut our_order: Vec<&str> = Vec::new();
        for pair in turns {
            if battle.ended() {
                break;
            }
            let seat = |t: &(bool, bool, u8, bool, bool, u8, bool)| SeatForce {
                hit: Some(t.0),
                crit: Some(t.1),
                roll: Some(t.2),
                secondary: Some(t.3),
                immobile: Some(t.4),
                hits: (t.5 > 0).then_some(t.5),
                selfhit: Some(t.6),
            };
            battle.set_turn_force(Some([seat(&pair[0]), seat(&pair[1])]));
            battle.set_player_choice("p1", "move 0").unwrap();
            if !battle.ended() {
                let _ = battle.set_player_choice("p2", "move 0");
            }
            for line in battle.new_log_entries() {
                if line.starts_with("move|mon:") {
                    our_order.push(if line.contains(",p1,") { "p1" } else { "p2" });
                }
            }
        }

        let their_order: Vec<&str> =
            got["order"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
        let ok = ["p1", "p2"].iter().enumerate().all(|(seat, pid)| {
            let ours = battle.player_data(pid).unwrap();
            let mon = &ours.mons[0];
            let pp = battle.active_mon_move_pp(pid);
            let want = &got[if seat == 0 { "p1" } else { "p2" }];
            mon.hp as u64 == want["hp"].as_u64().unwrap()
                && mon.max_hp as u64 == want["maxhp"].as_u64().unwrap()
                && mon.status.as_deref() == want["status"].as_str()
                && (mon.hp == 0) == want["fainted"].as_bool().unwrap()
                && pp.as_ref().and_then(|v| v.first().map(|(p, _)| *p as u64))
                    == want["pp"][0].as_u64()
        }) && our_order == their_order;
        if !ok {
            let p1 = battle.player_data("p1").unwrap();
            let p2 = battle.player_data("p2").unwrap();
            eprintln!(
                "GEN1 TURN FUZZ MISMATCH case {i}: ours p1 {}/{} {:?} p2 {}/{} {:?} order {:?} — showdown {} / {} order {:?}\n  scenario: {}\n  log: {}",
                p1.mons[0].hp, p1.mons[0].max_hp, p1.mons[0].status,
                p2.mons[0].hp, p2.mons[0].max_hp, p2.mons[0].status,
                our_order, got["p1"], got["p2"], their_order, scenarios[i], got["log"],
            );
            bad += 1;
        }
    }
    assert_eq!(bad, 0, "{bad} gen1 turn fuzz cases disagree — replay with the seed above");
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
                !PENDING.contains(&m.id)
                    && (m.power > 0
                        || m.status_action.is_some()
                        || m.fixed.is_some()
                        || m.ohko
                        || matches!(m.id, "counter" | "mirrorcoat"))
            })
            .unwrap_or(false)
    });
    let moves: Vec<String> = moves.into_iter().map(|(id, _)| id).collect();

    let mut fz = Fuzz::new("gen3-battles");
    let inv = Invest { iv: 31, ev: 0 };
    let statuses: [Option<&str>; 7] =
        [None, None, None, Some("brn"), Some("psn"), Some("par"), Some("slp")];

    let mut scenarios = Vec::new();
    let mut expected = Vec::new();
    let mut skipped = 0usize;

    for _ in 0..fuzz_n() / 4 {
        let team_n = 2 + fz.below(3) as usize; // 2..=4 per side
        let level = 40 + fz.below(61) as u8;

        // Build both teams, then reject any battle where two mons could tie
        // on Speed: the tie-break is each side's own RNG and cannot be pinned.
        let mut teams: [Vec<Mon>; 2] = [Vec::new(), Vec::new()];
        let mut specs: [Vec<Value>; 2] = [Vec::new(), Vec::new()];
        for seat in 0..2 {
            for _ in 0..team_n {
                let sp = fz.pick(SPECIES);
                let m1 = fz.pick(&moves).clone();
                // Two copies of one move is not a legal set, and the sim
                // folds them into a single slot with shared PP.
                let mut m2 = fz.pick(&moves).clone();
                while m2 == m1 {
                    m2 = fz.pick(&moves).clone();
                }
                let st = {
                    use gen3_battle::Type;
                    let pick = *fz.pick(&statuses);
                    let has = |t: Type| sp.types.0 == t || sp.types.1 == t;
                    match pick {
                        Some("brn") if has(Type::Fire) => None,
                        Some("psn") if has(Type::Poison) || has(Type::Steel) => None,
                        other => other,
                    }
                };
                let ability = pick_ability(&mut fz, sp);
                let item = pick_item(&mut fz);
                let mut mon = Mon::new(sp.id, level, Nature::Hardy, inv, &[&m1, &m2]).unwrap();
                mon.ability = ability;
                mon.item = item;
                mon.status = match st {
                    Some("brn") => Some(gen3_battle::data::Status::Burn),
                    Some("psn") => Some(gen3_battle::data::Status::Poison),
                    Some("par") => Some(gen3_battle::data::Status::Paralysis),
                    Some("slp") => Some(gen3_battle::data::Status::Sleep),
                    _ => None,
                };
                if mon.status == Some(gen3_battle::data::Status::Sleep) {
                    mon.sleep_n = 2;
                }
                specs[seat].push(json!({
                    "species": sp.id, "level": level, "moves": [m1, m2],
                    "status": st, "ability": ability, "item": item,
                }));
                teams[seat].push(mon);
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

        // Play the engine, recording what it actually chose each turn.
        let n_turns = 2 + fz.below(6) as usize;
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
                let slot = if switching {
                    *fz.pick(&bench)
                } else {
                    // Disable, Taunt and Torment grey a move out in the
                    // sim's request; choosing it is rejected, not
                    // reinterpreted.
                    let usable = battle.selectable_moves(seat);
                    if usable.contains(&pick) {
                        pick
                    } else {
                        usable.first().copied().unwrap_or(pick)
                    }
                };
                choices[seat] =
                    if switching { Choice::Switch(slot) } else { Choice::Move(slot) };
                seats[seat] = json!({
                    "action": if switching { "switch" } else { "move" },
                    "slot": slot,
                    "hit": !fz.chance(10),
                    "crit": fz.chance(20),
                    "roll": 85 + fz.below(16) as u8,
                    "secondary": fz.chance(40),
                    "immobile": fz.chance(15),
                    "hits": 0,
                    "selfhit": fz.chance(50),
                    "stall": fz.chance(50),
                });
            }
            let seat_script = |v: &Value| SeatScript {
                hit: v["hit"].as_bool().unwrap(),
                crit: v["crit"].as_bool().unwrap(),
                random: v["roll"].as_u64().unwrap() as u8,
                secondary: v["secondary"].as_bool().unwrap(),
                immobile: v["immobile"].as_bool().unwrap(),
                hits: v["hits"].as_u64().unwrap() as u8,
                selfhit: v["selfhit"].as_bool().unwrap(),
                stall: v["stall"].as_bool().unwrap(),
            };
            let ts = TurnScript {
                seats: [Some(seat_script(&seats[0])), Some(seat_script(&seats[1]))],
            };
            let events = battle.step_with(choices, &ts);
            our_states.push(snapshot_of(&battle));
            our_log.push(format!("-- turn {}: p1 {:?} p2 {:?}", turn_json.len(), choices[0], choices[1]));
            our_log.extend(events.iter().map(|e| format!("   {e:?}")));
            our_order.extend(events.iter().filter_map(|e| match e {
                Event::Used { side: 1, .. } => Some("p1"),
                Event::Used { side: 2, .. } => Some("p2"),
                _ => None,
            }));
            turn_json.push(json!({"p1": seats[0], "p2": seats[1]}));
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
    let results = showdown(&Value::Array(scenarios.clone()));
    let mut bad = 0;
    for (i, ((battle, our_order, our_log, our_states), got)) in expected.iter().zip(&results).enumerate() {
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
    assert_eq!(bad, 0, "{bad} gen3 battle fuzz cases disagree — replay with the seed above");
}

// ── Gen 1: whole battles — teams, switching, played to a winner ──────────────

/// The Gen 1 counterpart of [`fuzz_gen3_battles`]: two parties, voluntary
/// switches, faints and replacements, checked against the reference sim.
///
/// Same shape as the Gen 3 suite — the cartridge engine plays first and its
/// own choices are what the sim replays — but the era brings its own rules.
/// The one that shows up immediately is that ANY faint clears the whole queue
/// here, not merely the actions of the mons still standing.
#[test]
fn fuzz_gen1_battles() {
    if !armed() {
        return;
    }
    use gen1_battle::testing::{apply_status_drop, SeatForce, Status as G1Status};
    use gen1_battle::{MonData, MoveSlot as G1MoveSlot, PublicCoreBattle, Request, TeamData};
    use mega_blastoise_core::{battle_options_with_seed, demo_engine_opts, FlashDataStore};

    let moves = vanilla_moves(1, true, |id| gen1_battle::move_by_id(id).is_some());
    let moves: Vec<String> = moves.into_iter().map(|(id, _)| id).collect();
    assert!(moves.len() > 30, "gen1 battle pool too small: {}", moves.len());
    let species: Vec<&'static str> = gen1_battle::SPECIES.iter().map(|s| s.id).collect();

    let mut fz = Fuzz::new("gen1-battles");
    let data = FlashDataStore::new();
    let statuses: [Option<&str>; 6] =
        [None, None, None, Some("brn"), Some("psn"), Some("par")];

    let mut scenarios = Vec::new();
    let mut expected = Vec::new();
    let mut skipped = 0usize;

    for _ in 0..fuzz_n() / 4 {
        let team_n = 2 + fz.below(3) as usize;
        let level = 40 + fz.below(61) as u8;

        // Two teams. Statuses stay off sleep and freeze: both stall the
        // battle on clocks the two engines pin differently, and the freeze
        // side would also need the RBY thaw glitch the engine does not model.
        let mut specs: [Vec<Value>; 2] = [Vec::new(), Vec::new()];
        let mut members: [Vec<MonData>; 2] = [Vec::new(), Vec::new()];
        for seat in 0..2 {
            for _ in 0..team_n {
                let id = *fz.pick(&species);
                let sp = gen1_battle::SPECIES.iter().find(|s| s.id == id).unwrap();
                let m1 = fz.pick(&moves).clone();
                let mut m2 = fz.pick(&moves).clone();
                while m2 == m1 {
                    m2 = fz.pick(&moves).clone();
                }
                let st = {
                    let pick = *fz.pick(&statuses);
                    let has =
                        |t: gen1_battle::Type| sp.primary_type == t || sp.secondary_type == t;
                    match pick {
                        Some("brn") if has(gen1_battle::Type::Fire) => None,
                        Some("psn") if has(gen1_battle::Type::Poison) => None,
                        other => other,
                    }
                };
                // The cartridge engine builds every mon at 15 DVs and full
                // stat experience; the sim needs telling, in its own units.
                specs[seat].push(json!({
                    "species": id, "level": level, "moves": [&m1, &m2], "status": st,
                    "ivs": {"hp": 30, "atk": 30, "def": 30, "spa": 30, "spd": 30, "spe": 30},
                    "evs": {"hp": 255, "atk": 255, "def": 255, "spa": 255, "spd": 255, "spe": 255},
                }));
                members[seat].push(MonData {
                    name: String::new(),
                    species: sp.name.to_string(),
                    level,
                    moves: vec![
                        G1MoveSlot { name: m1.clone(), id: m1, typ: String::new(), pp: 0, max_pp: 0, disabled: false, target: 0 },
                        G1MoveSlot { name: m2.clone(), id: m2, typ: String::new(), pp: 0, max_pp: 0, disabled: false, target: 0 },
                    ],
                    ivs: Default::default(),
                    evs: Default::default(),
                    gender: None,
                    nature: None,
                    ability: Some(String::from("No Ability")),
                    item: None,
                });
            }
        }

        let mut battle =
            PublicCoreBattle::new(battle_options_with_seed(1), &data, demo_engine_opts()).unwrap();
        let [t0, t1] = members;
        battle.update_team("p1", TeamData { members: t0, ..Default::default() }).unwrap();
        battle.update_team("p2", TeamData { members: t1, ..Default::default() }).unwrap();
        battle.start().unwrap();
        {
            let sides = battle.testing_sides_mut();
            for seat in 0..2 {
                for (j, spec) in specs[seat].iter().enumerate() {
                    let mon = &mut sides[seat].team[j];
                    mon.status = match spec["status"].as_str() {
                        Some("brn") => G1Status::Burn,
                        Some("psn") => G1Status::Poison,
                        Some("par") => G1Status::Paralysis,
                        _ => G1Status::None,
                    };
                    apply_status_drop(mon);
                }
            }
            // No Speed ties anywhere in the two parties: the tie-break is
            // each side's own RNG and cannot be pinned.
            let mut speeds: Vec<u16> = (0..2)
                .flat_map(|s| (0..team_n).map(move |j| (s, j)))
                .map(|(s, j)| sides[s].team[j].modified[4])
                .collect();
            speeds.sort_unstable();
            let before = speeds.len();
            speeds.dedup();
            if speeds.len() != before {
                skipped += 1;
                continue;
            }
            // The drop above was only needed to compare Speeds. On the bench
            // it has to come back off: the engine re-applies it to fresh
            // stats when the mon actually walks on (which is what makes a
            // switch out and back a no-op), so leaving it here would halve
            // a burned Attack twice over.
            for seat in 0..2 {
                for j in 1..team_n {
                    let mon = &mut sides[seat].team[j];
                    mon.modified = mon.stats;
                }
            }
        }
        let _ = battle.new_log_entries().count(); // drop the setup log

        let n_turns = 2 + fz.below(5) as usize;
        let mut turn_json: Vec<Value> = Vec::new();
        let mut our_order: Vec<&str> = Vec::new();
        let mut our_states: Vec<Vec<Vec<String>>> = Vec::new();
        let mut our_log: Vec<String> = Vec::new();
        for _ in 0..n_turns {
            if battle.ended() {
                break;
            }
            let mut seats: [Value; 2] = [Value::Null, Value::Null];
            let mut lines: [String; 2] = [String::new(), String::new()];
            for seat in 0..2 {
                let pid = if seat == 0 { "p1" } else { "p2" };
                let (active, alive): (usize, Vec<usize>) = {
                    let d = battle.player_data(pid).unwrap();
                    let active = d
                        .mons
                        .iter()
                        .position(|m| m.active)
                        .unwrap_or(0);
                    let alive = d
                        .mons
                        .iter()
                        .enumerate()
                        .filter(|(j, m)| *j != active && m.hp > 0)
                        .map(|(j, _)| j)
                        .collect();
                    (active, alive)
                };
                // What the engine's own request says is legal — the same
                // view the sim builds: a mon locked into Rage or a charge
                // move is trapped, and Disable greys its slot out.
                let (trapped, locked, usable): (bool, bool, Vec<usize>) = {
                    let mut t = false;
                    let mut l = false;
                    let mut u: Vec<usize> = Vec::new();
                    for (rid, req) in battle.active_requests() {
                        if rid != pid {
                            continue;
                        }
                        if let Request::Turn(tr) = req {
                            if let Some(a) = tr.active.first() {
                                t = a.trapped;
                                l = a.locked_into_move;
                                u = a
                                    .moves
                                    .iter()
                                    .enumerate()
                                    .filter(|(_, m)| !m.disabled)
                                    .map(|(j, _)| j)
                                    .collect();
                            }
                        }
                    }
                    if u.is_empty() {
                        u.push(0);
                    }
                    (t, l, u)
                };
                let want_switch = fz.chance(25);
                let switching = !alive.is_empty() && !trapped && want_switch;
                let pick = fz.below(2) as usize;
                let slot = if switching {
                    *fz.pick(&alive)
                } else if locked {
                    // A locked mon's request offers exactly one move, and
                    // that is what the harness will send.
                    0
                } else if usable.contains(&pick) {
                    pick
                } else {
                    usable[0]
                };
                let _ = active;
                lines[seat] =
                    if switching { format!("switch {slot}") } else { format!("move {slot}") };
                seats[seat] = json!({
                    "action": if switching { "switch" } else { "move" },
                    "slot": slot,
                    "hit": !fz.chance(10),
                    "crit": fz.chance(20),
                    "roll": 217 + fz.below(39) as u8,
                    "secondary": fz.chance(40),
                    "immobile": fz.chance(15),
                    "hits": 0,
                    "selfhit": fz.chance(50),
                });
            }
            let seat_force = |v: &Value| SeatForce {
                hit: Some(v["hit"].as_bool().unwrap()),
                crit: Some(v["crit"].as_bool().unwrap()),
                roll: Some(v["roll"].as_u64().unwrap() as u8),
                secondary: Some(v["secondary"].as_bool().unwrap()),
                immobile: Some(v["immobile"].as_bool().unwrap()),
                // Pinned to the table floor, matching the harness's `hits ||
                // 2`. It cannot be drawn per move here the way the turn
                // suite does it, because Mimic can change what a slot holds
                // between the draw and the use.
                hits: Some(2),
                selfhit: Some(v["selfhit"].as_bool().unwrap()),
            };
            battle.set_turn_force(Some([seat_force(&seats[0]), seat_force(&seats[1])]));
            battle.set_player_choice("p1", &lines[0]).unwrap();
            if !battle.ended() {
                let _ = battle.set_player_choice("p2", &lines[1]);
            }
            // Answer any forced replacement the same way the harness does:
            // the lowest-numbered living team slot that is not already out.
            let mut guard = 0;
            while !battle.ended() && guard < 12 {
                guard += 1;
                let pending: Vec<String> = battle
                    .active_requests()
                    .filter(|(_, r)| matches!(r, Request::Switch(_)))
                    .map(|(pid, _)| pid.to_string())
                    .collect();
                if pending.is_empty() {
                    break;
                }
                for pid in pending {
                    let d = battle.player_data(&pid).unwrap();
                    let next = d
                        .mons
                        .iter()
                        .enumerate()
                        .find(|(_, m)| !m.active && m.hp > 0)
                        .map(|(j, _)| j);
                    match next {
                        Some(j) => {
                            let _ = battle.set_player_choice(&pid, &format!("switch {j}"));
                        }
                        None => break,
                    }
                }
            }
            our_log.push(format!(
                "-- turn {}: p1 {} p2 {}",
                turn_json.len(),
                lines[0],
                lines[1]
            ));
            for line in battle.new_log_entries() {
                if line.starts_with("move|mon:") {
                    our_order.push(if line.contains(",p1,") { "p1" } else { "p2" });
                }
                our_log.push(format!("   {line}"));
            }
            our_states.push(
                ["p1", "p2"]
                    .iter()
                    .map(|pid| {
                        battle
                            .player_data(pid)
                            .unwrap()
                            .mons
                            .iter()
                            .map(|m| {
                                format!(
                                    "{}/{} {}{} pp[{}]",
                                    m.hp,
                                    m.max_hp,
                                    m.status.clone().unwrap_or_else(|| "-".into()),
                                    if m.active && m.hp > 0 { " *" } else { "" },
                                    m.moves
                                        .iter()
                                        .map(|s| format!("{}:{}", s.id, s.pp))
                                        .collect::<Vec<_>>()
                                        .join(","),
                                )
                            })
                            .collect()
                    })
                    .collect(),
            );
            turn_json.push(json!({"p1": seats[0], "p2": seats[1]}));
        }

        scenarios.push(json!({
            "kind": "battle", "gen": 1,
            "p1": {"team": specs[0]},
            "p2": {"team": specs[1]},
            "turns": turn_json,
        }));
        expected.push((battle, our_order, our_states, our_log));
    }

    if skipped > 0 {
        eprintln!("[gen1-battles] {skipped} cases skipped on a Speed tie");
    }
    let results = showdown(&Value::Array(scenarios.clone()));
    let mut bad = 0;
    for (i, ((battle, our_order, our_states, our_log), got)) in expected.iter().zip(&results).enumerate() {
        if got.get("error").is_some() {
            eprintln!("HARNESS ERROR case {i}: {got}\n  scenario: {}", scenarios[i]);
            bad += 1;
            continue;
        }
        let mut diffs: Vec<String> = Vec::new();
        for e in got["errors"].as_array().map(|a| a.as_slice()).unwrap_or(&[]) {
            diffs.push(format!("harness rejected a choice: {e}"));
        }
        for (seat, who) in ["p1", "p2"].iter().enumerate() {
            let ours = battle.player_data(who).unwrap();
            let theirs = got[*who].as_array().unwrap();
            let _ = seat;
            if theirs.len() != ours.mons.len() {
                diffs.push(format!("{who} party size {} vs {}", ours.mons.len(), theirs.len()));
                continue;
            }
            for (j, mon) in ours.mons.iter().enumerate() {
                let want = &theirs[j];
                let mut cmp = |field: &str, a: String, b: String| {
                    if a != b {
                        diffs.push(format!("{who}[{j}].{field}: ours {a} vs sim {b}"));
                    }
                };
                cmp("hp", mon.hp.to_string(), want["hp"].to_string());
                cmp("maxhp", mon.max_hp.to_string(), want["maxhp"].to_string());
                cmp("fainted", (mon.hp == 0).to_string(), want["fainted"].to_string());
                cmp(
                    "active",
                    (mon.active && mon.hp > 0).to_string(),
                    want["active"].to_string(),
                );
                let their_pp = want["pp"].as_array().unwrap();
                for (k, ms) in mon.moves.iter().enumerate() {
                    cmp(
                        &format!("pp[{}]", ms.id),
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
        if let Some(states) = got["states"].as_array() {
            for (t, theirs) in states.iter().enumerate() {
                let Some(ours) = our_states.get(t) else { break };
                let mut turn_diff = Vec::new();
                for (seat, who) in ["p1", "p2"].iter().enumerate() {
                    for (j, mon) in theirs[*who].as_array().unwrap().iter().enumerate() {
                        // The sim reports PP positionally; pair it with our
                        // own slot ids so a divergence names the move.
                        let ours_ids: Vec<String> = our_states[t][seat]
                            .get(j)
                            .and_then(|s| s.split("pp[").nth(1))
                            .map(|s| s.trim_end_matches(']'))
                            .map(|s| s.split(',').map(|x| x.split(':').next().unwrap_or("").to_string()).collect())
                            .unwrap_or_default();
                        let sim_pp: Vec<String> = mon["pp"]
                            .as_array()
                            .map(|a| {
                                a.iter()
                                    .enumerate()
                                    .map(|(k, v)| {
                                        format!("{}:{}", ours_ids.get(k).cloned().unwrap_or_default(), v)
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();
                        let sim = format!(
                            "{}/{} {}{} pp[{}]",
                            mon["hp"], mon["maxhp"],
                            mon["status"].as_str().unwrap_or("-"),
                            if mon["active"].as_bool().unwrap_or(false) { " *" } else { "" },
                            sim_pp.join(","),
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
        if !diffs.is_empty() {
            eprintln!(
                "GEN1 BATTLE FUZZ MISMATCH case {i}:\n  diffs:\n    {}\n  scenario: {}\n  log: {}",
                diffs.join("\n    "),
                scenarios[i],
                got["log"],
            );
            eprintln!("  ours-log:\n{}", our_log.join("\n"));
            bad += 1;
        }
    }
    assert_eq!(bad, 0, "{bad} gen1 battle fuzz cases disagree — replay with the seed above");
}
