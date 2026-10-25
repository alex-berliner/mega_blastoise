//! Compiles the dex dumps into `&'static` tables.
//!
//! Input: `data/{gen1,gen3}/*.json`, written by `tools/ps-diff/dump-dex.js`
//! from PS's resolved `Dex.mod(genN)`. Output, in `OUT_DIR`:
//! - `pool.rs`: one string pool shared by both gens (every name, key and
//!   constant stored once; a string already inside the pool is reused).
//! - `names.rs`: handler names, handler constants, move flag / bool names
//!   and ability flag names, each table sorted, and the flag constants.
//! - `dex.rs`: `pub static GEN1: Dex` and `GEN3`, and per-gen id constants
//!   (`gen1::moves::BIDE`).
//!
//! The JSON's order is PS's iteration order and is kept: an entry's index is
//! its id. Anything the schema cannot hold exactly (an out-of-range number,
//! a condition name that resolves to nothing) panics here rather than being
//! stored approximately.

use std::collections::{BTreeSet, HashMap};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

const GENS: [&str; 2] = ["gen1", "gen3"];

/// A `MoveExtra` with every field unset, as `emit_gen` spells it.
const EXTRA_NONE: &str = "MoveExtra { secondaries: &[], self_effect: None, boosts: None, side_condition: ConditionId::NONE, \
    slot_condition: ConditionId::NONE, weather: ConditionId::NONE, pseudo_weather: ConditionId::NONE, \
    damage: None, multihit: None, drain: None, recoil: None, heal: None, selfdestruct: None, \
    self_switch: None }";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=data");
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("data");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    let gens: Vec<GenData> = GENS.iter().map(|g| GenData::load(&root, g)).collect();
    let names = Names::collect(&gens);
    let mut pool = Pool::default();
    let mut dex = String::new();
    for g in &gens {
        dex.push_str(&emit_gen(g, &names, &mut pool));
    }

    fs::write(out.join("dex.rs"), dex).unwrap();
    fs::write(out.join("names.rs"), names.emit()).unwrap();
    fs::write(out.join("pool.rs"), pool.emit()).unwrap();
}

// ------------------------------------------------------------------ input

struct GenData {
    name: String,
    gen: u8,
    /// `psVersion`, plus `psCommit` when the pin is an upstream commit.
    ps: String,
    species: Vec<Value>,
    moves: Vec<Value>,
    items: Vec<Value>,
    abilities: Vec<Value>,
    conditions: Vec<Value>,
    typechart: Value,
    move_bools: Vec<String>,
}

fn read(dir: &Path, file: &str) -> Value {
    let path = dir.join(file);
    println!("cargo:rerun-if-changed={}", path.display());
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn arr(v: Value) -> Vec<Value> {
    match v {
        Value::Array(a) => a,
        other => panic!("expected an array, got {other}"),
    }
}

impl GenData {
    fn load(root: &Path, gen: &str) -> GenData {
        let dir = root.join(gen);
        let meta = read(&dir, "meta.json");
        GenData {
            name: gen.to_string(),
            gen: meta["gen"].as_u64().unwrap() as u8,
            ps: match meta["psCommit"].as_str() {
                Some(c) => format!("{} at {c}", meta["psVersion"].as_str().unwrap()),
                None => meta["psVersion"].as_str().unwrap().to_string(),
            },
            species: arr(read(&dir, "species.json")),
            moves: arr(read(&dir, "moves.json")),
            items: arr(read(&dir, "items.json")),
            abilities: arr(read(&dir, "abilities.json")),
            conditions: arr(read(&dir, "conditions.json")),
            typechart: read(&dir, "typechart.json"),
            move_bools: strings(&meta["moveBools"]),
        }
    }

    /// Every handler list in the gen, for collecting names.
    fn all_handlers(&self) -> Vec<&Value> {
        let mut owners: Vec<&Value> = Vec::new();
        for m in &self.moves {
            owners.push(m);
            if !m["self"].is_null() {
                owners.push(&m["self"]);
            }
            for s in m["secondaries"].as_array().into_iter().flatten() {
                owners.push(s);
                if !s["self"].is_null() {
                    owners.push(&s["self"]);
                }
            }
        }
        owners.extend(self.items.iter().chain(&self.abilities).chain(&self.conditions));
        owners.iter().flat_map(|v| v["handlers"].as_array().unwrap()).collect()
    }

    fn condition_index(&self, key: &str, what: &str) -> String {
        let i = self
            .conditions
            .iter()
            .position(|c| c["key"] == key)
            .unwrap_or_else(|| panic!("{}: {what} names condition {key}, not in the table", self.name));
        format!("ConditionId({i})")
    }

    /// A condition reference field: `ConditionId::NONE` for null.
    fn cond(&self, v: &Value, what: &str) -> String {
        match v {
            Value::Null => "ConditionId::NONE".into(),
            Value::String(s) => self.condition_index(s, what),
            other => panic!("{}: {what} = {other}", self.name),
        }
    }
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap_or_else(|| panic!("expected a string array, got {v}"))
        .iter()
        .map(|s| s.as_str().unwrap().to_string())
        .collect()
}

// ------------------------------------------------------------------ names

/// The small vocabularies the tables index into.
struct Names {
    handlers: Vec<String>,
    constants: Vec<String>,
    move_flags: Vec<String>,
    move_bools: Vec<String>,
    ability_flags: Vec<String>,
    abilities: Vec<String>,
}

impl Names {
    fn collect(gens: &[GenData]) -> Names {
        let mut handlers = BTreeSet::new();
        let mut constants = BTreeSet::new();
        let mut move_flags = BTreeSet::new();
        let mut ability_flags = BTreeSet::new();
        let mut abilities = BTreeSet::new();
        let mut bools_seen = BTreeSet::new();
        for g in gens {
            for sp in &g.species {
                for a in sp["abilities"].as_array().unwrap() {
                    if let Some(a) = a.as_str() {
                        abilities.insert(a.to_string());
                    }
                }
            }
            for m in &g.moves {
                bools_seen.extend(strings(&m["bools"]));
            }
            for h in g.all_handlers() {
                handlers.insert(h["name"].as_str().unwrap().to_string());
                if let Value::String(s) = &h["value"] {
                    if s != "function" {
                        constants.insert(s.clone());
                    }
                }
            }
            for m in &g.moves {
                move_flags.extend(strings(&m["flags"]));
            }
            for a in &g.abilities {
                ability_flags.extend(strings(&a["flags"]));
                abilities.insert(a["name"].as_str().unwrap().to_string());
            }
        }
        // The bool list is fixed by the dumper and identical for every gen;
        // the table keeps the ones some move sets, in that order.
        for g in gens {
            assert_eq!(g.move_bools, gens[0].move_bools, "{}: moveBools differ", g.name);
        }
        let move_bools: Vec<String> =
            gens[0].move_bools.iter().filter(|b| bools_seen.contains(*b)).cloned().collect();
        let names = Names {
            handlers: handlers.into_iter().collect(),
            constants: constants.into_iter().collect(),
            move_flags: move_flags.into_iter().collect(),
            move_bools,
            ability_flags: ability_flags.into_iter().collect(),
            abilities: abilities.into_iter().collect(),
        };
        assert!(names.handlers.len() <= 256, "more than 256 handler names");
        assert!(names.constants.len() <= 256, "more than 256 handler constants");
        assert!(names.move_flags.len() <= 64, "more than 64 move flags");
        assert!(names.move_bools.len() <= 16, "more than 16 move bools");
        assert!(names.abilities.len() < 255, "more than 254 ability names");
        assert!(names.ability_flags.len() <= 32, "more than 32 ability flags");
        names
    }

    fn index(list: &[String], name: &str) -> usize {
        list.iter().position(|n| n == name).unwrap()
    }

    fn bits(list: &[String], set: &Value) -> u64 {
        strings(set).iter().fold(0, |acc, n| acc | 1 << Self::index(list, n))
    }

    fn emit(&self) -> String {
        let mut s = String::new();
        let list = |s: &mut String, doc: &str, name: &str, items: &[String]| {
            writeln!(s, "/// {doc}").unwrap();
            writeln!(s, "pub const {name}: [&str; {}] = [", items.len()).unwrap();
            for i in items {
                writeln!(s, "    {i:?},").unwrap();
            }
            writeln!(s, "];").unwrap();
        };
        list(&mut s, "Every handler key either gen's effects carry, sorted.", "HANDLER_NAMES", &self.handlers);
        list(&mut s, "Every constant id a handler holds (`'recharge'`), sorted.", "HANDLER_CONSTANTS", &self.constants);
        list(&mut s, "Every PS move flag either gen's moves set, sorted: the bits of `MoveFlags`.", "MOVE_FLAG_NAMES", &self.move_flags);
        list(&mut s, "The boolean move fields, in dump order: the bits of `MoveBools`.", "MOVE_BOOL_NAMES", &self.move_bools);
        list(&mut s, "Every PS ability flag either gen's abilities set, sorted: the bits of `AbilityFlags`.", "ABILITY_FLAG_NAMES", &self.ability_flags);
        list(&mut s, "Every ability name in either gen's species or ability tables, sorted: what `SpeciesDef::abilities` indexes.", "ABILITY_NAMES", &self.abilities);
        writeln!(s, "impl MoveFlags {{").unwrap();
        for (i, f) in self.move_flags.iter().enumerate() {
            writeln!(s, "    pub const {}: MoveFlags = MoveFlags(1 << {i});", f.to_uppercase()).unwrap();
        }
        writeln!(s, "}}").unwrap();
        writeln!(s, "impl MoveBools {{").unwrap();
        for (i, f) in self.move_bools.iter().enumerate() {
            writeln!(s, "    pub const {}: MoveBools = MoveBools(1 << {i});", screaming(f)).unwrap();
        }
        writeln!(s, "}}").unwrap();
        s
    }
}

/// `ignoreNegativeOffensive` -> `IGNORE_NEGATIVE_OFFENSIVE`.
fn screaming(camel: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = camel.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if c.is_ascii_uppercase() && i > 0 {
            let prev_lower = chars[i - 1].is_ascii_lowercase();
            let next_lower = chars.get(i + 1).is_some_and(|n| n.is_ascii_lowercase());
            if prev_lower || next_lower {
                out.push('_');
            }
        }
        out.push(c.to_ascii_uppercase());
    }
    out
}

// ------------------------------------------------------------------ pool

#[derive(Default)]
struct Pool {
    text: String,
    cache: HashMap<String, (usize, usize)>,
}

impl Pool {
    /// The `Str` expression for `s`, adding it to the pool if needed.
    fn add(&mut self, s: &str) -> String {
        if s.is_empty() {
            return "Str::EMPTY".into();
        }
        let (off, len) = match self.cache.get(s) {
            Some(&hit) => hit,
            None => {
                let off = match self.text.find(s) {
                    Some(off) => off,
                    None => {
                        let off = self.text.len();
                        self.text.push_str(s);
                        off
                    }
                };
                self.cache.insert(s.to_string(), (off, s.len()));
                (off, s.len())
            }
        };
        assert!(off + len <= u16::MAX as usize, "string pool over 64 KB");
        assert!(len <= u8::MAX as usize, "string {s:?} over 255 bytes");
        format!("s({off}, {len})")
    }

    fn opt(&mut self, v: &Value) -> String {
        match v {
            Value::Null => "Str::EMPTY".into(),
            Value::String(s) => self.add(s),
            other => panic!("expected a string or null, got {other}"),
        }
    }

    fn emit(&self) -> String {
        format!(
            "/// Every string of every generation's dex, each stored once.\npub const POOL: &str = {:?};\n",
            self.text
        )
    }
}

// ------------------------------------------------------------------ values

fn int(v: &Value, what: &str) -> i64 {
    v.as_i64().unwrap_or_else(|| panic!("{what}: expected an integer, got {v}"))
}

fn ranged(v: &Value, lo: i64, hi: i64, what: &str) -> i64 {
    let n = int(v, what);
    assert!((lo..=hi).contains(&n), "{what}: {n} outside {lo}..={hi}");
    n
}

fn u8_(v: &Value, what: &str) -> String {
    ranged(v, 0, u8::MAX as i64, what).to_string()
}

fn opt_u8(v: &Value, what: &str) -> String {
    if v.is_null() {
        "None".into()
    } else {
        format!("Some({})", u8_(v, what))
    }
}

fn boolean(v: &Value, what: &str) -> bool {
    v.as_bool().unwrap_or_else(|| panic!("{what}: expected a boolean, got {v}"))
}

fn type_expr(name: &str, what: &str) -> String {
    let variant = match name {
        "???" => "None",
        "Normal" | "Fire" | "Water" | "Electric" | "Grass" | "Ice" | "Fighting" | "Poison"
        | "Ground" | "Flying" | "Psychic" | "Bug" | "Rock" | "Ghost" | "Dragon" | "Dark"
        | "Steel" | "Fairy" => name,
        other => panic!("{what}: unknown type {other}"),
    };
    format!("Type::{variant}")
}

fn target_expr(name: &str, what: &str) -> String {
    let variant = match name {
        "normal" => "Normal",
        "self" => "Self_",
        "allAdjacentFoes" => "AllAdjacentFoes",
        "all" => "All",
        "foeSide" => "FoeSide",
        "allySide" => "AllySide",
        "allyTeam" => "AllyTeam",
        "allies" => "Allies",
        "any" => "Any",
        "randomNormal" => "RandomNormal",
        "scripted" => "Scripted",
        "adjacentFoe" => "AdjacentFoe",
        "adjacentAlly" => "AdjacentAlly",
        "adjacentAllyOrSelf" => "AdjacentAllyOrSelf",
        "allAdjacent" => "AllAdjacent",
        other => panic!("{what}: unknown target {other}"),
    };
    format!("MoveTarget::{variant}")
}

fn boosts(v: &Value, what: &str) -> String {
    if v.is_null() {
        return "None".into();
    }
    let obj = v.as_object().unwrap();
    let mut s = String::from("Some(&Boosts {");
    for stat in ["atk", "def", "spa", "spd", "spe", "accuracy", "evasion"] {
        let n = obj.get(stat).map_or(0, |n| ranged(n, -12, 12, what));
        write!(s, " {stat}: {n},").unwrap();
    }
    for k in obj.keys() {
        assert!(
            ["atk", "def", "spa", "spd", "spe", "accuracy", "evasion"].contains(&k.as_str()),
            "{what}: boost {k}"
        );
        assert_ne!(obj[k], 0, "{what}: a 0 boost would not round-trip");
    }
    s.push_str(" })");
    s
}

fn fraction(v: &Value, what: &str) -> String {
    if v.is_null() {
        return "None".into();
    }
    let a = v.as_array().unwrap();
    format!("Some(Fraction {{ num: {}, den: {} }})", u8_(&a[0], what), u8_(&a[1], what))
}

fn handlers(v: &Value, names: &Names, what: &str) -> String {
    let list = v["handlers"].as_array().unwrap_or_else(|| panic!("{what}: no handlers"));
    if list.is_empty() {
        return "&[]".into();
    }
    let mut s = String::from("&[");
    for h in list {
        let name = h["name"].as_str().unwrap();
        let w = format!("{what}.{name}");
        let value = match &h["value"] {
            Value::String(f) if f == "function" => "HandlerValue::Function".to_string(),
            Value::Bool(false) => "HandlerValue::False".to_string(),
            Value::String(id) => format!("HandlerValue::Id({})", Names::index(&names.constants, id)),
            other => panic!("{w}: handler value {other}"),
        };
        let small = |k: &str| -> i64 {
            let v = &h[k];
            if v.is_null() {
                0
            } else {
                ranged(v, i8::MIN as i64, i8::MAX as i64, &w)
            }
        };
        // PS: `order || false`, and the table's 0 means `false`, so a
        // literal 0 would read the same; none exists, and this keeps it so.
        assert!(h["order"].is_null() || small("order") != 0, "{w}: order 0");
        let order = small("order");
        write!(
            s,
            "Handler {{ name: {}, value: {value}, priority: {}, order: {order}, sub_order: {} }},",
            Names::index(&names.handlers, name),
            small("priority"),
            small("subOrder"),
        )
        .unwrap();
    }
    s.push(']');
    s
}

/// `toID`.
fn to_id(s: &str) -> String {
    s.chars()
        .map(|c| c.to_ascii_lowercase())
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

fn check_key(e: &Value, what: &str) -> String {
    let key = e["key"].as_str().unwrap();
    let name = e["name"].as_str().unwrap();
    assert_eq!(to_id(name), key, "{what}: name {name} is not its key {key}");
    key.to_string()
}

// ------------------------------------------------------------------ tables

fn emit_gen(g: &GenData, names: &Names, pool: &mut Pool) -> String {
    let gn = &g.name;
    let up = gn.to_uppercase();
    let mut s = String::new();

    // Moves.
    let move_key_index: HashMap<String, usize> = g
        .moves
        .iter()
        .enumerate()
        .map(|(i, m)| (m["key"].as_str().unwrap().to_string(), i))
        .collect();
    writeln!(s, "static {up}_MOVES: [MoveDef; {}] = [", g.moves.len()).unwrap();
    for m in &g.moves {
        let key = check_key(m, &format!("{gn} move"));
        let w = format!("{gn} move {key}");
        let id = m["id"].as_str().unwrap();
        let placeholder_for = if id == key {
            "MoveId::NONE".to_string()
        } else {
            format!("MoveId({})", move_key_index.get(id).unwrap_or_else(|| panic!("{w}: id {id}")))
        };
        // 0 is PS's `true`; PS has no 0% move (the dumper checks).
        let accuracy = match &m["accuracy"] {
            Value::Bool(true) => "0".to_string(),
            v => ranged(v, 1, u8::MAX as i64, &w).to_string(),
        };
        let damage = match &m["damage"] {
            Value::Null => "None".to_string(),
            Value::String(l) if l == "level" => "Some(FixedDamage::Level)".to_string(),
            v => format!("Some(FixedDamage::Hp({}))", ranged(v, 0, u16::MAX as i64, &w)),
        };
        let multihit = match &m["multihit"] {
            Value::Null => "None".to_string(),
            v => {
                let a = v.as_array().unwrap();
                format!("Some([{}, {}])", u8_(&a[0], &w), u8_(&a[1], &w))
            }
        };
        let selfdestruct = match m["selfdestruct"].as_str() {
            None => "None",
            Some("always") => "Some(SelfDestruct::Always)",
            Some("ifHit") => "Some(SelfDestruct::IfHit)",
            Some(o) => panic!("{w}: selfdestruct {o}"),
        };
        let self_switch = match &m["selfSwitch"] {
            Value::Null => "None",
            Value::Bool(true) => "Some(SelfSwitch::True)",
            Value::String(c) if c == "copyvolatile" => "Some(SelfSwitch::CopyVolatile)",
            o => panic!("{w}: selfSwitch {o}"),
        };
        let mut secondaries = String::from("&[");
        for (i, sec) in m["secondaries"].as_array().into_iter().flatten().enumerate() {
            let ws = format!("{w}.secondaries[{i}]");
            write!(
                secondaries,
                "Secondary {{ chance: {}, status: {}, volatile_status: {}, boosts: {}, self_effect: {}, handlers: {} }},",
                opt_u8(&sec["chance"], &ws),
                g.cond(&sec["status"], &ws),
                g.cond(&sec["volatileStatus"], &ws),
                boosts(&sec["boosts"], &ws),
                self_effect(g, &sec["self"], names, &format!("{ws}.self")),
                handlers(sec, names, &ws),
            )
            .unwrap();
        }
        secondaries.push(']');
        let condition = match m["condition"].as_str() {
            None => "ConditionId::NONE".to_string(),
            Some(k) => g.condition_index(k, &w),
        };
        let category = match m["category"].as_str().unwrap() {
            c @ ("Physical" | "Special" | "Status") => format!("MoveCategory::{c}"),
            o => panic!("{w}: category {o}"),
        };
        let extra = format!(
            "MoveExtra {{ secondaries: {secondaries}, self_effect: {}, boosts: {}, side_condition: {}, slot_condition: {}, weather: {}, \
             pseudo_weather: {}, damage: {damage}, multihit: {multihit}, drain: {}, recoil: {}, heal: {}, \
             selfdestruct: {selfdestruct}, self_switch: {self_switch} }}",
            self_effect(g, &m["self"], names, &format!("{w}.self")),
            boosts(&m["boosts"], &w),
            g.cond(&m["sideCondition"], &w),
            g.cond(&m["slotCondition"], &w),
            g.cond(&m["weather"], &w),
            g.cond(&m["pseudoWeather"], &w),
            fraction(&m["drain"], &w),
            fraction(&m["recoil"], &w),
            fraction(&m["heal"], &w),
        );
        let extra = if extra == EXTRA_NONE { "None".to_string() } else { format!("Some(&{extra})") };
        writeln!(
            s,
            "    MoveDef {{ name: {name}, handlers: {handlers}, extra: {extra}, \
             num: {num}, placeholder_for: {placeholder_for}, base_power: {bp}, status: {status}, \
             volatile_status: {vs}, condition: {condition}, flags: MoveFlags({flags}), bools: MoveBools({bools}), \
             type_: {ty}, base_move_type: {bty}, category: {category}, target: {target}, priority: {priority}, \
             accuracy: {accuracy}, pp: {pp}, crit_ratio: {crit} }},",
            name = pool.add(m["name"].as_str().unwrap()),
            num = ranged(&m["num"], i16::MIN as i64, i16::MAX as i64, &w),
            ty = type_expr(m["type"].as_str().unwrap(), &w),
            bty = type_expr(m["baseMoveType"].as_str().unwrap(), &w),
            target = target_expr(m["target"].as_str().unwrap(), &w),
            priority = ranged(&m["priority"], i8::MIN as i64, i8::MAX as i64, &w),
            bp = ranged(&m["basePower"], 0, u16::MAX as i64, &w),
            pp = u8_(&m["pp"], &w),
            crit = u8_(&m["critRatio"], &w),
            status = g.cond(&m["status"], &w),
            vs = g.cond(&m["volatileStatus"], &w),
            flags = Names::bits(&names.move_flags, &m["flags"]),
            bools = Names::bits(&names.move_bools, &m["bools"]),
            handlers = handlers(m, names, &w),
        )
        .unwrap();
    }
    writeln!(s, "];").unwrap();

    // Species.
    writeln!(s, "static {up}_SPECIES: [SpeciesDef; {}] = [", g.species.len()).unwrap();
    for sp in &g.species {
        let key = check_key(sp, &format!("{gn} species"));
        let w = format!("{gn} species {key}");
        let types = strings(&sp["types"]);
        assert!(!types.is_empty() && types.len() <= 2, "{w}: types {types:?}");
        let t2 = types.get(1).map_or("Type::None".to_string(), |t| type_expr(t, &w));
        let stats: Vec<String> = sp["baseStats"].as_array().unwrap().iter().map(|v| u8_(v, &w)).collect();
        let ab = sp["abilities"].as_array().unwrap();
        let gender = match sp["gender"].as_str() {
            None => "Gender::Either",
            Some("M") => "Gender::Male",
            Some("F") => "Gender::Female",
            Some("N") => "Gender::Neutral",
            Some(o) => panic!("{w}: gender {o}"),
        };
        let ratio: Vec<String> = sp["genderRatio"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| {
                let eighths = r.as_f64().unwrap() * 8.0;
                assert_eq!(eighths.fract(), 0.0, "{w}: genderRatio {r}");
                (eighths as u8).to_string()
            })
            .collect();
        let max_hp = if sp["maxHP"].is_null() {
            "None".to_string()
        } else {
            format!("Some({})", ranged(&sp["maxHP"], 0, u16::MAX as i64, &w))
        };
        let ability = |v: &Value| match v.as_str() {
            None => "NO_ABILITY".to_string(),
            Some(a) => Names::index(&names.abilities, a).to_string(),
        };
        let forme = if sp["forme"].is_null() {
            assert_eq!(sp["baseSpecies"], sp["name"], "{w}: a base forme is its own baseSpecies");
            assert!(sp["battleOnly"].is_null() && sp["requiredItem"].is_null(), "{w}: forme fields without a forme");
            None
        } else {
            Some(sp["baseSpecies"].as_str().unwrap())
        };
        let forme = if forme.is_none() && sp["maxHP"].is_null() {
            "None".to_string()
        } else {
            format!(
                "Some(&Forme {{ max_hp: {max_hp}, base_species: {}, forme: {}, battle_only: {}, required_item: {} }})",
                forme.map_or("Str::EMPTY".to_string(), |b| pool.add(b)),
                pool.opt(&sp["forme"]),
                pool.opt(&sp["battleOnly"]),
                pool.opt(&sp["requiredItem"]),
            )
        };
        writeln!(
            s,
            "    SpeciesDef {{ name: {}, forme: {forme}, num: {}, weighthg: {}, \
             types: [{}, {t2}], base_stats: [{}], abilities: [{}, {}], gender: {gender}, gender_ratio: [{}], \
             nfe: {} }},",
            pool.add(sp["name"].as_str().unwrap()),
            ranged(&sp["num"], i16::MIN as i64, i16::MAX as i64, &w),
            ranged(&sp["weighthg"], 0, u16::MAX as i64, &w),
            type_expr(&types[0], &w),
            stats.join(", "),
            ability(&ab[0]),
            ability(&ab[1]),
            ratio.join(", "),
            boolean(&sp["nfe"], &w),
        )
        .unwrap();
    }
    writeln!(s, "];").unwrap();

    // Cosmetic formes, by base species.
    writeln!(s, "static {up}_COSMETIC_FORMES: &[CosmeticFormes] = &[").unwrap();
    for (i, sp) in g.species.iter().enumerate() {
        let Some(formes) = sp["cosmeticFormes"].as_array() else { continue };
        let names: Vec<String> = formes.iter().map(|f| pool.add(f.as_str().unwrap())).collect();
        writeln!(s, "    CosmeticFormes {{ base: SpeciesId({i}), names: &[{}] }},", names.join(", ")).unwrap();
    }
    writeln!(s, "];").unwrap();

    // Items.
    writeln!(s, "static {up}_ITEMS: [ItemDef; {}] = [", g.items.len()).unwrap();
    for it in &g.items {
        let key = check_key(it, &format!("{gn} item"));
        let w = format!("{gn} item {key}");
        let natural_gift = match &it["naturalGift"] {
            Value::Null => "None".to_string(),
            v => format!(
                "Some(NaturalGift {{ base_power: {}, type_: {} }})",
                u8_(&v["basePower"], &w),
                type_expr(v["type"].as_str().unwrap(), &w)
            ),
        };
        let fling = match &it["fling"] {
            Value::Null => "None".to_string(),
            v => format!(
                "Some(Fling {{ base_power: {}, status: {}, volatile_status: {}, effect: {} }})",
                u8_(&v["basePower"], &w),
                pool.opt(&v["status"]),
                pool.opt(&v["volatileStatus"]),
                boolean(&v["effect"], &w)
            ),
        };
        let item_user = match &it["itemUser"] {
            Value::Null => "&[]".to_string(),
            v => format!("&[{}]", strings(v).iter().map(|u| pool.add(u)).collect::<Vec<_>>().join(", ")),
        };
        let condition = match it["condition"].as_str() {
            None => "ConditionId::NONE".to_string(),
            Some(k) => g.condition_index(k, &w),
        };
        writeln!(
            s,
            "    ItemDef {{ name: {}, num: {}, is_berry: {}, is_gem: {}, is_pokeball: {}, is_choice: {}, \
             ignore_klutz: {}, natural_gift: {natural_gift}, fling: {fling}, item_user: {item_user}, \
             boosts: {}, condition: {condition}, handlers: {} }},",
            pool.add(it["name"].as_str().unwrap()),
            ranged(&it["num"], i16::MIN as i64, i16::MAX as i64, &w),
            boolean(&it["isBerry"], &w),
            boolean(&it["isGem"], &w),
            boolean(&it["isPokeball"], &w),
            boolean(&it["isChoice"], &w),
            boolean(&it["ignoreKlutz"], &w),
            boosts(&it["boosts"], &w),
            handlers(it, names, &w),
        )
        .unwrap();
    }
    writeln!(s, "];").unwrap();

    // Abilities.
    writeln!(s, "static {up}_ABILITIES: [AbilityDef; {}] = [", g.abilities.len()).unwrap();
    for a in &g.abilities {
        let key = check_key(a, &format!("{gn} ability"));
        let w = format!("{gn} ability {key}");
        let tenths = a["rating"].as_f64().unwrap() * 10.0;
        assert_eq!(tenths.fract(), 0.0, "{w}: rating");
        assert!((i8::MIN as f64..=i8::MAX as f64).contains(&tenths), "{w}: rating");
        let condition = match a["condition"].as_str() {
            None => "ConditionId::NONE".to_string(),
            Some(k) => g.condition_index(k, &w),
        };
        writeln!(
            s,
            "    AbilityDef {{ name: {}, num: {}, rating_tenths: {}, suppress_weather: {}, \
             flags: AbilityFlags({}), condition: {condition}, handlers: {} }},",
            pool.add(a["name"].as_str().unwrap()),
            ranged(&a["num"], i16::MIN as i64, i16::MAX as i64, &w),
            tenths as i8,
            boolean(&a["suppressWeather"], &w),
            Names::bits(&names.ability_flags, &a["flags"]),
            handlers(a, names, &w),
        )
        .unwrap();
    }
    writeln!(s, "];").unwrap();

    // Conditions.
    writeln!(s, "static {up}_CONDITIONS: [ConditionDef; {}] = [", g.conditions.len()).unwrap();
    for c in &g.conditions {
        let key = c["key"].as_str().unwrap();
        let w = format!("{gn} condition {key}");
        let effect_type = match c["effectType"].as_str().unwrap() {
            t @ ("Condition" | "Weather" | "Status") => format!("ConditionType::{t}"),
            o => panic!("{w}: effectType {o}"),
        };
        let source = match c["source"].as_str().unwrap() {
            "condition" => "ConditionSource::Condition",
            "move" => "ConditionSource::Move",
            "ability" => "ConditionSource::Ability",
            "item" => "ConditionSource::Item",
            o => panic!("{w}: source {o}"),
        };
        writeln!(
            s,
            "    ConditionDef {{ key: {}, name: {}, effect_type: {effect_type}, source: {source}, \
             duration: {}, counter_max: {}, no_copy: {}, affects_fainted: {}, handlers: {} }},",
            pool.add(key),
            pool.add(c["name"].as_str().unwrap()),
            opt_u8(&c["duration"], &w),
            opt_u8(&c["counterMax"], &w),
            boolean(&c["noCopy"], &w),
            boolean(&c["affectsFainted"], &w),
            handlers(c, names, &w),
        )
        .unwrap();
    }
    writeln!(s, "];").unwrap();

    // Typechart.
    let tc = &g.typechart;
    let order = strings(&tc["order"]);
    let mut chart = vec![vec![0u8; 19]; 19];
    let mut special = String::new();
    let index_of = |t: &str| -> usize {
        const ORDER: [&str; 18] = [
            "Normal", "Fire", "Water", "Electric", "Grass", "Ice", "Fighting", "Poison", "Ground",
            "Flying", "Psychic", "Bug", "Rock", "Ghost", "Dragon", "Dark", "Steel", "Fairy",
        ];
        ORDER.iter().position(|o| *o == t).unwrap_or_else(|| panic!("{gn}: type {t}"))
    };
    for row in tc["types"].as_array().unwrap() {
        let def = row["name"].as_str().unwrap();
        let w = format!("{gn} type {def}");
        for (atk, code) in row["damageTaken"].as_object().unwrap() {
            assert!(order.contains(atk), "{w}: attacker {atk}");
            chart[index_of(def)][index_of(atk)] = ranged(code, 0, 3, &w) as u8;
        }
        for (id, code) in row["special"].as_object().unwrap() {
            write!(
                special,
                "SpecialImmunity {{ type_: {}, id: {}, code: {} }}, ",
                type_expr(def, &w),
                pool.add(id),
                ranged(code, 0, 3, &w)
            )
            .unwrap();
        }
    }
    let rows: Vec<String> = chart
        .iter()
        .map(|r| format!("[{}]", r.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(", ")))
        .collect();

    writeln!(
        s,
        "/// The resolved `Dex.mod('{gn}')` of PS {ps}.\npub static {up}: Dex = Dex {{ gen: {gen}, \
         species: &{up}_SPECIES, moves: &{up}_MOVES, items: &{up}_ITEMS, abilities: &{up}_ABILITIES, \
         conditions: &{up}_CONDITIONS, cosmetic_formes: {up}_COSMETIC_FORMES, typechart: TypeChart {{ order: &[{order}], damage_taken: [{rows}], \
         special: &[{special}] }} }};",
        ps = g.ps,
        gen = g.gen,
        order = order.iter().map(|t| type_expr(t, gn)).collect::<Vec<_>>().join(", "),
        rows = rows.join(", "),
    )
    .unwrap();

    // Id constants.
    writeln!(s, "/// `{gn}` ids by PS key, e.g. `{gn}::moves::{}`.", g.moves[0]["key"].as_str().unwrap().to_uppercase()).unwrap();
    writeln!(s, "pub mod {gn} {{").unwrap();
    let consts = |s: &mut String, module: &str, ty: &str, list: &[Value]| {
        writeln!(s, "    pub mod {module} {{").unwrap();
        writeln!(s, "        use crate::ids::{ty};").unwrap();
        for (i, e) in list.iter().enumerate() {
            writeln!(s, "        pub const {}: {ty} = {ty}({i});", e["key"].as_str().unwrap().to_uppercase()).unwrap();
        }
        writeln!(s, "    }}").unwrap();
    };
    consts(&mut s, "moves", "MoveId", &g.moves);
    consts(&mut s, "species", "SpeciesId", &g.species);
    consts(&mut s, "items", "ItemId", &g.items);
    consts(&mut s, "abilities", "AbilityId", &g.abilities);
    consts(&mut s, "conditions", "ConditionId", &g.conditions);
    writeln!(s, "}}").unwrap();
    s
}

fn self_effect(g: &GenData, v: &Value, names: &Names, what: &str) -> String {
    if v.is_null() {
        return "None".into();
    }
    format!(
        "Some(&SelfEffect {{ chance: {}, status: {}, volatile_status: {}, side_condition: {}, \
         pseudo_weather: {}, boosts: {}, handlers: {} }})",
        opt_u8(&v["chance"], what),
        g.cond(&v["status"], what),
        g.cond(&v["volatileStatus"], what),
        g.cond(&v["sideCondition"], what),
        g.cond(&v["pseudoWeather"], what),
        boosts(&v["boosts"], what),
        handlers(v, names, what),
    )
}
