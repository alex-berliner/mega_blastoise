//! Differential tests: the core engines against the REAL Pokémon Showdown
//! simulator (tools/showdown-parity/harness.js).
//!
//! Randomness is scripted on both sides — hit/miss, crit, and the exact
//! damage roll — so the whole random surface is compared branch by branch
//! rather than being disabled or averaged: a crit's stage-ignore rules, a
//! miss still spending PP, both roll extremes.
//!
//! Skips (passing, loudly) when the harness's node_modules is absent, so a
//! checkout without node keeps a green suite. Run `npm install` in
//! tools/showdown-parity to arm it.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{json, Value};

fn harness_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/showdown-parity")
}

/// Run a batch of scenarios through the real simulator.
fn showdown(scenarios: &Value) -> Vec<Value> {
    let dir = harness_dir();
    let mut child = Command::new("node")
        .arg("harness.js")
        .current_dir(&dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn node");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(scenarios.to_string().as_bytes())
        .unwrap();
    let out = child.wait_with_output().expect("harness run");
    assert!(out.status.success(), "harness failed: {}", String::from_utf8_lossy(&out.stderr));
    serde_json::from_slice(&out.stdout).expect("harness emitted JSON")
}

fn armed() -> bool {
    if harness_dir().join("node_modules").exists() {
        return true;
    }
    eprintln!("SKIP: showdown parity not armed — npm install in tools/showdown-parity");
    false
}

// ── Stats ────────────────────────────────────────────────────────────────────

#[test]
fn gen3_stats_match_showdown() {
    if !armed() {
        return;
    }
    use gen3_battle::{hp_stat, other_stat, species_by_id, Invest, Nature, Stat};

    // Species across stat ranges, levels the drafter actually uses, natures
    // covering raised/lowered/neutral, and spreads covering IV and EV maths.
    let species = ["blaziken", "swampert", "gardevoir", "shedinja", "blissey", "rattata", "deoxys"];
    let natures = [Nature::Hardy, Nature::Adamant, Nature::Modest, Nature::Timid, Nature::Sassy];
    let spreads = [(31u8, 0u8), (31, 252), (0, 0), (15, 100)];

    let mut scenarios = Vec::new();
    let mut keys = Vec::new();
    for id in species {
        for level in [5u8, 50, 66, 100] {
            for nature in natures {
                for (iv, ev) in spreads {
                    scenarios.push(json!({
                        "kind": "stats", "gen": 3, "species": id, "level": level,
                        "nature": format!("{nature:?}"),
                        "ivs": {"hp": iv, "atk": iv, "def": iv, "spa": iv, "spd": iv, "spe": iv},
                        "evs": {"hp": ev, "atk": ev, "def": ev, "spa": ev, "spd": ev, "spe": ev},
                    }));
                    keys.push((id, level, nature, iv, ev));
                }
            }
        }
    }
    let results = showdown(&Value::Array(scenarios));

    let mut bad = 0;
    for ((id, level, nature, iv, ev), got) in keys.iter().zip(&results) {
        assert!(got.get("error").is_none(), "{id}: {got}");
        let sp = species_by_id(id).expect(id);
        let inv = Invest { iv: *iv, ev: *ev };
        let ours = [
            hp_stat(sp.base.hp, inv, *level),
            other_stat(sp.base.atk, inv, *level, *nature, Stat::Atk),
            other_stat(sp.base.def, inv, *level, *nature, Stat::Def),
            other_stat(sp.base.spa, inv, *level, *nature, Stat::SpAtk),
            other_stat(sp.base.spd, inv, *level, *nature, Stat::SpDef),
            other_stat(sp.base.spe, inv, *level, *nature, Stat::Spe),
        ];
        let theirs: Vec<u16> = ["hp", "atk", "def", "spa", "spd", "spe"]
            .iter()
            .map(|k| got[k].as_u64().unwrap() as u16)
            .collect();
        if ours.to_vec() != theirs {
            eprintln!("MISMATCH {id} L{level} {nature:?} iv{iv} ev{ev}: ours {ours:?} showdown {theirs:?}");
            bad += 1;
        }
    }
    assert_eq!(bad, 0, "{bad} stat spreads disagree with Showdown");
}

// ── Type chart ───────────────────────────────────────────────────────────────

#[test]
fn gen3_type_chart_matches_showdown() {
    if !armed() {
        return;
    }
    use gen3_battle::{effectiveness, Type, TYPE_COUNT};

    let results = showdown(&json!([{"kind": "chart", "gen": 3}]));
    let chart = &results[0];
    let names: Vec<&str> = chart["types"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
    assert_eq!(names.len(), TYPE_COUNT, "type count disagrees: showdown has {names:?}");

    let by_name = |n: &str| -> Type {
        (0..TYPE_COUNT as u8)
            .map(|i| unsafe { core::mem::transmute::<u8, Type>(i) })
            .find(|t| t.name() == n)
            .unwrap_or_else(|| panic!("showdown type {n} unknown to us"))
    };

    let mut bad = 0;
    for (ai, atk) in names.iter().enumerate() {
        for (di, def) in names.iter().enumerate() {
            let theirs = chart["mult"][ai][di].as_u64().unwrap() as u8;
            let ours = effectiveness(by_name(atk), by_name(def));
            if ours != theirs {
                eprintln!("CHART MISMATCH {atk} -> {def}: ours x{ours} showdown x{theirs} (x10)");
                bad += 1;
            }
        }
    }
    assert_eq!(bad, 0, "{bad} chart cells disagree with Showdown");
}

// ── Whole turns, scripted randomness ─────────────────────────────────────────

struct TurnCase {
    name: &'static str,
    p1: (&'static str, &'static str), // species, move
    p2: (&'static str, &'static str),
    p1_status: Option<&'static str>,
    p2_conditions: &'static [&'static str],
    script: [(bool, bool, u8); 2], // (hit, crit, roll) per seat
}

#[test]
fn gen3_turns_match_showdown() {
    if !armed() {
        return;
    }
    use gen3_battle::{
        battle::{Side, TurnScript},
        Battle, Choice, Invest, Mon, Nature, SeatScript,
    };

    let cases = [
        TurnCase { name: "stab resisted", p1: ("blaziken", "ember"), p2: ("swampert", "splash"),
                   p1_status: None, p2_conditions: &[], script: [(true, false, 100), (true, true, 100)] },
        TurnCase { name: "super effective min roll", p1: ("sceptile", "leafblade"), p2: ("swampert", "splash"),
                   p1_status: None, p2_conditions: &[], script: [(true, false, 85), (true, true, 100)] },
        TurnCase { name: "crit ignores reflect", p1: ("slaking", "bodyslam"), p2: ("registeel", "splash"),
                   p1_status: None, p2_conditions: &["reflect"], script: [(true, true, 100), (true, true, 100)] },
        TurnCase { name: "reflect halves without crit", p1: ("slaking", "bodyslam"), p2: ("registeel", "splash"),
                   p1_status: None, p2_conditions: &["reflect"], script: [(true, false, 100), (true, true, 100)] },
        TurnCase { name: "burn halves physical", p1: ("slaking", "bodyslam"), p2: ("registeel", "splash"),
                   p1_status: Some("brn"), p2_conditions: &[], script: [(true, false, 100), (true, true, 100)] },
        TurnCase { name: "special ignores burn", p1: ("gardevoir", "psychic"), p2: ("swampert", "splash"),
                   p1_status: Some("brn"), p2_conditions: &[], script: [(true, false, 92), (true, true, 100)] },
        TurnCase { name: "miss spends pp deals nothing", p1: ("blaziken", "ember"), p2: ("swampert", "splash"),
                   p1_status: None, p2_conditions: &[], script: [(false, false, 100), (true, true, 100)] },
        TurnCase { name: "immune", p1: ("gengar", "lick"), p2: ("slaking", "splash"),
                   p1_status: None, p2_conditions: &[], script: [(true, false, 100), (true, true, 100)] },
        TurnCase { name: "light screen vs special", p1: ("gardevoir", "psychic"), p2: ("swampert", "splash"),
                   p1_status: None, p2_conditions: &["lightscreen"], script: [(true, false, 100), (true, true, 100)] },
        TurnCase { name: "dark is special in gen3", p1: ("absol", "crunch"), p2: ("regice", "splash"),
                   p1_status: None, p2_conditions: &[], script: [(true, false, 100), (true, true, 100)] },
    ];

    let mut scenarios = Vec::new();
    for c in &cases {
        scenarios.push(json!({
            "kind": "turn", "gen": 3,
            "p1": {"species": c.p1.0, "level": 100, "status": c.p1_status},
            "p2": {"species": c.p2.0, "level": 100, "sideConditions": c.p2_conditions},
            "moves": [c.p1.1, c.p2.1],
            "script": {
                "p1": {"hit": c.script[0].0, "crit": c.script[0].1, "roll": c.script[0].2},
                "p2": {"hit": c.script[1].0, "crit": c.script[1].1, "roll": c.script[1].2},
            },
        }));
    }
    let results = showdown(&Value::Array(scenarios));

    let mut bad = 0;
    for (c, got) in cases.iter().zip(&results) {
        assert!(got.get("error").is_none(), "{}: {got}", c.name);

        // Mirror the setup on our engine. Levels and investment match the
        // harness defaults (level 100, 31/0, Hardy).
        let inv = Invest { iv: 31, ev: 0 };
        let mk = |species: &str, mv: &str| {
            Mon::new(species, 100, Nature::Hardy, inv, &[mv])
                .unwrap_or_else(|| panic!("{}: {species}/{mv} not in our tables", c.name))
        };
        let mut p1 = mk(c.p1.0, c.p1.1);
        let p2 = mk(c.p2.0, c.p2.1);
        p1.burned = c.p1_status == Some("brn");
        let mut battle = Battle::new(Side::new(vec![p1]), Side::new(vec![p2]), 1);

        // Screens are defender state in our damage model; the engine has no
        // screen-setting move yet, so the parity applies them directly.
        let reflect = c.p2_conditions.contains(&"reflect");
        let light_screen = c.p2_conditions.contains(&"lightscreen");
        let script = TurnScript {
            seats: [
                Some(SeatScript { hit: c.script[0].0, crit: c.script[0].1, random: c.script[0].2 }),
                Some(SeatScript { hit: c.script[1].0, crit: c.script[1].1, random: c.script[1].2 }),
            ],
        };
        let (ours_hp, ours_pp) = {
            // Fold the screens in through the damage call the turn makes: the
            // engine's Defender is built from the target side, so model them
            // by computing damage directly when screens are involved.
            if reflect || light_screen {
                use gen3_battle::{damage, Attacker, Defender, MoveUse, Roll};
                let a = &battle.sides[0].party[0];
                let d = &battle.sides[1].party[0];
                let slot = a.moves[0];
                let dealt = damage(
                    &Attacker {
                        level: a.level, atk: a.atk, sp_atk: a.spa,
                        atk_stage: 0, sp_atk_stage: 0, types: a.types(), burned: a.burned,
                    },
                    &Defender {
                        def: d.def, sp_def: d.spd, def_stage: 0, sp_def_stage: 0,
                        types: d.types(), reflect, light_screen,
                    },
                    &MoveUse { move_type: slot.move_type(), power: slot.entry.power },
                    Roll { crit: c.script[0].1, random: c.script[0].2 },
                );
                (d.hp.saturating_sub(dealt as u16), a.moves[0].pp - 1)
            } else {
                battle.step_with([Choice::Move(0), Choice::Move(0)], &script);
                (battle.sides[1].mon().hp, battle.sides[0].mon().moves[0].pp)
            }
        };

        let theirs_hp = got["p2"]["hp"].as_u64().unwrap() as u16;
        let theirs_pp = got["p1"]["pp"][0].as_u64().unwrap() as u8;
        let our_max = battle.sides[1].mon().max_hp;
        let their_max = got["p2"]["maxhp"].as_u64().unwrap() as u16;

        if our_max != their_max || ours_hp != theirs_hp || ours_pp != theirs_pp {
            eprintln!(
                "TURN MISMATCH [{}]: ours hp {ours_hp}/{our_max} pp {ours_pp} — showdown hp {theirs_hp}/{their_max} pp {theirs_pp}\n  log: {}",
                c.name, got["log"],
            );
            bad += 1;
        }
    }
    assert_eq!(bad, 0, "{bad} turns disagree with Showdown");
}

// ── Gen 1 ────────────────────────────────────────────────────────────────────

#[test]
fn gen1_type_chart_matches_showdown() {
    if !armed() {
        return;
    }
    use gen1_battle::{type_effectiveness, Type};

    let by_name = |n: &str| -> Type {
        match n {
            "Normal" => Type::Normal, "Fire" => Type::Fire, "Water" => Type::Water,
            "Electric" => Type::Electric, "Grass" => Type::Grass, "Ice" => Type::Ice,
            "Fighting" => Type::Fighting, "Poison" => Type::Poison, "Ground" => Type::Ground,
            "Flying" => Type::Flying, "Psychic" => Type::Psychic, "Bug" => Type::Bug,
            "Rock" => Type::Rock, "Ghost" => Type::Ghost, "Dragon" => Type::Dragon,
            other => panic!("showdown gen1 type {other} unknown to us"),
        }
    };

    let results = showdown(&json!([{"kind": "chart", "gen": 1}]));
    let chart = &results[0];
    let names: Vec<&str> =
        chart["types"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
    assert_eq!(names.len(), 15, "gen 1 has fifteen types, showdown says {names:?}");

    let mut bad = 0;
    for (ai, atk) in names.iter().enumerate() {
        for (di, def) in names.iter().enumerate() {
            let theirs = chart["mult"][ai][di].as_u64().unwrap() as u8;
            let ours = type_effectiveness(by_name(atk), by_name(def));
            if ours != theirs {
                eprintln!("GEN1 CHART MISMATCH {atk} -> {def}: ours x{ours} showdown x{theirs} (x10)");
                bad += 1;
            }
        }
    }
    assert_eq!(bad, 0, "{bad} gen1 chart cells disagree with Showdown");
}

#[test]
fn gen1_stats_match_showdown() {
    if !armed() {
        return;
    }
    // The engine assumes max DVs and max stat experience for every mon, so the
    // harness asks Showdown for the same spread: ivs 30 (dv 15), evs 255.
    let species = ["snorlax", "alakazam", "chansey", "electrode", "rattata", "mewtwo"];
    let mut scenarios = Vec::new();
    let mut keys = Vec::new();
    for id in species {
        for level in [5u8, 55, 90, 100] {
            scenarios.push(json!({
                "kind": "stats", "gen": 1, "species": id, "level": level,
                "ivs": {"hp": 30, "atk": 30, "def": 30, "spa": 30, "spd": 30, "spe": 30},
                "evs": {"hp": 255, "atk": 255, "def": 255, "spa": 255, "spd": 255, "spe": 255},
            }));
            keys.push((id, level));
        }
    }
    let results = showdown(&Value::Array(scenarios));

    let mut bad = 0;
    for ((id, level), got) in keys.iter().zip(&results) {
        assert!(got.get("error").is_none(), "{id}: {got}");
        let mon = gen1_battle::testing::Mon::from_species(id, *level, &["tackle"])
            .unwrap_or_else(|| panic!("{id} not in our gen1 dex"));
        // Gen 1 has one Special: Showdown reports it as both spa and spd.
        let ours = [mon.hp_max, mon.stats[1], mon.stats[2], mon.stats[3], mon.stats[4]];
        let theirs: Vec<u16> = ["hp", "atk", "def", "spa", "spe"]
            .iter()
            .map(|k| got[k].as_u64().unwrap() as u16)
            .collect();
        if ours.to_vec() != theirs {
            eprintln!("GEN1 STAT MISMATCH {id} L{level}: ours {ours:?} showdown {theirs:?}");
            bad += 1;
        }
    }
    assert_eq!(bad, 0, "{bad} gen1 stat lines disagree with Showdown");
}

#[test]
fn gen1_damage_matches_showdown() {
    if !armed() {
        return;
    }
    use gen1_battle::testing::{compute_damage_scripted, Mon};
    use gen1_battle::move_by_id;

    // (name, attacker, move, defender, crit, roll 217..=255)
    let cases = [
        ("stab super effective", "blastoise", "hydropump", "charizard", false, 255),
        ("min roll", "blastoise", "hydropump", "charizard", false, 217),
        ("crit doubles the level term", "snorlax", "bodyslam", "chansey", true, 255),
        ("resisted", "snorlax", "bodyslam", "gengar", false, 255), // immune, in fact
        ("special uses one stat", "alakazam", "psychic", "snorlax", false, 234),
        ("physical into high def", "rattata", "tackle", "cloyster", false, 255),
    ];

    let mut scenarios = Vec::new();
    for (_, atk, mv, def, crit, roll) in &cases {
        scenarios.push(json!({
            "kind": "turn", "gen": 1,
            "p1": {"species": atk, "level": 90,
                   "ivs": {"hp": 30, "atk": 30, "def": 30, "spa": 30, "spd": 30, "spe": 30},
                   "evs": {"hp": 255, "atk": 255, "def": 255, "spa": 255, "spd": 255, "spe": 255}},
            "p2": {"species": def, "level": 90,
                   "ivs": {"hp": 30, "atk": 30, "def": 30, "spa": 30, "spd": 30, "spe": 30},
                   "evs": {"hp": 255, "atk": 255, "def": 255, "spa": 255, "spd": 255, "spe": 255}},
            "moves": [mv, "splash"],
            "script": {"p1": {"hit": true, "crit": crit, "roll": roll},
                       "p2": {"hit": true, "crit": false, "roll": 255}},
        }));
    }
    let results = showdown(&Value::Array(scenarios));

    let mut bad = 0;
    for ((name, atk, mv, def, crit, roll), got) in cases.iter().zip(&results) {
        assert!(got.get("error").is_none(), "{name}: {got}");
        let attacker = Mon::from_species(atk, 90, &[mv]).unwrap();
        let defender = Mon::from_species(def, 90, &["splash"]).unwrap();
        let entry = move_by_id(mv).unwrap();
        let dealt = compute_damage_scripted(&attacker, &defender, entry, false, *crit, *roll).dmg;
        let ours_hp = defender.hp_max.saturating_sub(dealt);

        let theirs_hp = got["p2"]["hp"].as_u64().unwrap() as u16;
        let their_max = got["p2"]["maxhp"].as_u64().unwrap() as u16;
        if defender.hp_max != their_max || ours_hp != theirs_hp {
            eprintln!(
                "GEN1 DAMAGE MISMATCH [{name}]: ours {ours_hp}/{} — showdown {theirs_hp}/{their_max}\n  log: {}",
                defender.hp_max, got["log"],
            );
            bad += 1;
        }
    }
    assert_eq!(bad, 0, "{bad} gen1 damage cases disagree with Showdown");
}
