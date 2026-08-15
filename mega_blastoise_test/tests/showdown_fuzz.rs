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
                            | "secretpower"
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

        let attacker = Attacker {
            level: *level,
            atk: other_stat(a_sp.base.atk, a_inv, *level, *a_nat, Stat::Atk),
            sp_atk: other_stat(a_sp.base.spa, a_inv, *level, *a_nat, Stat::SpAtk),
            atk_stage: stages[0],
            sp_atk_stage: stages[1],
            types: a_sp.types,
            burned: *burned,
        };
        let defender = Defender {
            def: other_stat(d_sp.base.def, d_inv, *level, *d_nat, Stat::Def),
            sp_def: other_stat(d_sp.base.spd, d_inv, *level, *d_nat, Stat::SpDef),
            def_stage: stages[2],
            sp_def_stage: stages[3],
            types: d_sp.types,
            reflect: *reflect,
            light_screen: *light_screen,
        };
        let max_hp = hp_stat(d_sp.base.hp, d_inv, *level);
        // A never-miss move (accuracy 0) ignores the scripted miss, exactly
        // as the sim's accuracy step never runs for it.
        let hit = *hit || entry.accuracy == 0;
        let dealt = if hit {
            damage(&attacker, &defender, &MoveUse { move_type: entry.move_type, power: entry.power, halve_def: entry.selfdestruct, weather: 0 },
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
        // 1-3 whole turns, each seat scripted per turn: hit, crit, roll,
        // secondary, immobile, hits, selfhit. The conditional knobs only
        // fire when their condition holds (paralysis, a 2-5 multi-hit move,
        // confusion), so generating them unconditionally is harmless and
        // covers conditions that land mid-battle.
        let n_turns = 1 + fz.below(3) as usize;
        let mut turns: Vec<[(bool, bool, u8, bool, bool, u8, bool); 2]> = Vec::new();
        for _ in 0..n_turns {
            let mut pair = [(true, false, 100u8, false, false, 0u8, false); 2];
            for (seat, slot) in pair.iter_mut().enumerate() {
                *slot = (
                    !fz.chance(10),
                    fz.chance(25),
                    85 + fz.below(16) as u8,
                    fz.chance(40),
                    fz.chance(15),
                    0,
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
                let seat = |t: &(bool, bool, u8, bool, bool, u8, bool)| {
                    json!({"hit": t.0, "crit": t.1, "roll": t.2, "secondary": t.3, "immobile": t.4, "hits": t.5, "selfhit": t.6})
                };
                json!({"p1": seat(&pair[0]), "p2": seat(&pair[1])})
            })
            .collect();
        scenarios.push(json!({
            "kind": "turn", "gen": 3,
            "p1": {"species": s1.id, "level": level, "status": st1.map(|s| s.0)},
            "p2": {"species": s2.id, "level": level, "status": st2.map(|s| s.0)},
            "moves": [m1, m2],
            "turns": turn_json,
        }));
        cases.push((s1.id, s2.id, level, m1, m2, turns, [st1.map(|s| s.1), st2.map(|s| s.1)]));
    }

    let results = showdown(&Value::Array(scenarios.clone()));
    let inv = Invest { iv: 31, ev: 0 };
    let mut bad = 0;
    for (i, ((s1, s2, level, m1, m2, turns, statuses), got)) in cases.iter().zip(&results).enumerate() {
        if got.get("error").is_some() {
            eprintln!("HARNESS ERROR case {i}: {got}\n  scenario: {}", scenarios[i]);
            bad += 1;
            continue;
        }
        let mk = |id: &str, mv: &str| Mon::new(id, *level, Nature::Hardy, inv, &[mv]).unwrap();
        let mut battle =
            Battle::new(Side::new(vec![mk(s1, m1)]), Side::new(vec![mk(s2, m2)]), 1);
        for (seat, st) in statuses.iter().enumerate() {
            battle.sides[seat].party[0].status = *st;
            if *st == Some(gen3_battle::data::Status::Sleep) {
                // The sim's pinned duration roll: asleep for one skipped
                // action, awake on the second.
                battle.sides[seat].party[0].sleep_n = 2;
            }
        }
        // Skip speed ties: the tie-break is each side's own RNG.
        if battle.sides[0].mon().spe == battle.sides[1].mon().spe {
            continue;
        }
        let mut our_order: Vec<&str> = Vec::new();
        for pair in turns {
            if battle.over() {
                break;
            }
            let seat = |t: &(bool, bool, u8, bool, bool, u8, bool)| SeatScript {
                hit: t.0,
                crit: t.1,
                random: t.2,
                secondary: t.3,
                immobile: t.4,
                hits: t.5,
                selfhit: t.6,
            };
            let ts = TurnScript { seats: [Some(seat(&pair[0])), Some(seat(&pair[1]))] };
            let events = battle.step_with([Choice::Move(0), Choice::Move(0)], &ts);
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
            Counter, DreamEater, FlatDamage, HalfHp, LevelDamage, Ohko, TwoTurn,
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
        let fire = |m: &str| {
            gen1_battle::move_by_id(m)
                .map(|e| e.move_type == gen1_battle::Type::Fire)
                .unwrap_or(false)
        };
        if st1 == Some("frz") && fire(&m2) {
            st1 = None;
        }
        if st2 == Some("frz") && fire(&m1) {
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
