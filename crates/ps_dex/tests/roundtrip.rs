//! Every gen 1 and gen 3 entry of the generated tables, field by field,
//! against the JSON dumps it was built from (`tools/ps-diff/dump-dex.js`),
//! and the counts against design section 6 of
//! `docs/living/showdown-split-plan.md`.

use std::fs;
use std::path::PathBuf;

use ps_dex::*;
use serde_json::Value;

fn load(gen: &str, file: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data").join(gen).join(file);
    serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap()
}

fn list(gen: &str, file: &str) -> Vec<Value> {
    load(gen, file).as_array().unwrap().clone()
}

fn dex(gen: &str) -> &'static Dex {
    match gen {
        "gen1" => &GEN1,
        "gen3" => &GEN3,
        _ => unreachable!(),
    }
}

fn s(v: &Value) -> &str {
    v.as_str().unwrap_or("")
}

fn opt_u64(v: &Value) -> Option<u64> {
    v.as_u64()
}

fn strs(v: &Value) -> Vec<String> {
    v.as_array().map_or(vec![], |a| a.iter().map(|x| x.as_str().unwrap().to_string()).collect())
}

fn ty(name: &str) -> Type {
    Type::from_name(name).unwrap_or_else(|| panic!("type {name}"))
}

fn cond_key(d: &Dex, id: ConditionId) -> Option<&'static str> {
    if id.is_none() {
        None
    } else {
        Some(d.condition_def(id).key.as_str())
    }
}

fn check_cond(d: &Dex, id: ConditionId, v: &Value, what: &str) {
    assert_eq!(cond_key(d, id), v.as_str(), "{what}");
}

fn check_boosts(b: Option<&Boosts>, v: &Value, what: &str) {
    match b {
        None => assert!(v.is_null(), "{what}: boosts missing"),
        Some(b) => {
            let obj = v.as_object().unwrap_or_else(|| panic!("{what}: boosts not in JSON"));
            for (stat, n) in Boosts::STATS.iter().zip(b.as_array()) {
                let want = obj.get(*stat).map_or(0, |x| x.as_i64().unwrap());
                assert_eq!(n as i64, want, "{what}: boost {stat}");
            }
            for k in obj.keys() {
                assert!(Boosts::STATS.contains(&k.as_str()), "{what}: boost {k}");
            }
        }
    }
}

fn check_fraction(f: Option<Fraction>, v: &Value, what: &str) {
    match f {
        None => assert!(v.is_null(), "{what}"),
        Some(f) => assert_eq!(vec![f.num as u64, f.den as u64], vec![v[0].as_u64().unwrap(), v[1].as_u64().unwrap()], "{what}"),
    }
}

fn check_handlers(hs: &[Handler], v: &Value, what: &str) {
    let want = v["handlers"].as_array().unwrap();
    assert_eq!(hs.len(), want.len(), "{what}: handler count");
    for (h, w) in hs.iter().zip(want) {
        let name = w["name"].as_str().unwrap();
        let wh = format!("{what}.{name}");
        assert_eq!(h.name(), name, "{wh}");
        match &w["value"] {
            Value::String(f) if f == "function" => assert_eq!(h.value, HandlerValue::Function, "{wh}"),
            Value::Bool(false) => assert_eq!(h.value, HandlerValue::False, "{wh}"),
            Value::String(id) => assert_eq!(h.constant_id(), Some(id.as_str()), "{wh}"),
            other => panic!("{wh}: {other}"),
        }
        assert_eq!(h.priority as i64, w["priority"].as_i64().unwrap_or(0), "{wh} priority");
        assert_eq!(h.order().map(|o| o as i64), w["order"].as_i64(), "{wh} order");
        assert_eq!(h.sub_order as i64, w["subOrder"].as_i64().unwrap_or(0), "{wh} subOrder");
    }
}

fn check_self(d: &Dex, e: Option<&SelfEffect>, v: &Value, what: &str) {
    let Some(e) = e else {
        assert!(v.is_null(), "{what}: self missing");
        return;
    };
    assert!(!v.is_null(), "{what}: self not in JSON");
    assert_eq!(e.chance.map(u64::from), opt_u64(&v["chance"]), "{what} chance");
    check_cond(d, e.status, &v["status"], &format!("{what} status"));
    check_cond(d, e.volatile_status, &v["volatileStatus"], &format!("{what} volatileStatus"));
    check_cond(d, e.side_condition, &v["sideCondition"], &format!("{what} sideCondition"));
    check_cond(d, e.pseudo_weather, &v["pseudoWeather"], &format!("{what} pseudoWeather"));
    check_boosts(e.boosts, &v["boosts"], what);
    check_handlers(e.handlers, v, what);
}

fn check_moves(gen: &str) -> usize {
    let d = dex(gen);
    let json = list(gen, "moves.json");
    assert_eq!(d.moves.len(), json.len(), "{gen} move count");
    for (i, (m, j)) in d.moves.iter().zip(&json).enumerate() {
        let key = s(&j["key"]);
        let w = format!("{gen} move {key}");
        assert_eq!(m.name.as_str(), s(&j["name"]), "{w} name");
        assert_eq!(d.move_id(key), Some(MoveId(i as u16)), "{w} lookup");
        let ps_id = d.move_ps_id(MoveId(i as u16));
        assert!(text::name_is_id(d.move_def(ps_id).name.as_str(), s(&j["id"])), "{w} id");
        assert_eq!(m.num as i64, j["num"].as_i64().unwrap(), "{w} num");
        assert_eq!(m.type_, ty(s(&j["type"])), "{w} type");
        assert_eq!(m.type_.ps_name(), s(&j["type"]), "{w} type name");
        assert_eq!(m.base_move_type, ty(s(&j["baseMoveType"])), "{w} baseMoveType");
        assert_eq!(m.category.name(), s(&j["category"]), "{w} category");
        assert_eq!(m.target.id(), s(&j["target"]), "{w} target");
        assert_eq!(m.priority as i64, j["priority"].as_i64().unwrap(), "{w} priority");
        assert_eq!(m.base_power as i64, j["basePower"].as_i64().unwrap(), "{w} basePower");
        match m.accuracy() {
            Accuracy::Always => assert_eq!(j["accuracy"], Value::Bool(true), "{w} accuracy"),
            Accuracy::Percent(p) => assert_eq!(p as u64, j["accuracy"].as_u64().unwrap(), "{w} accuracy"),
        }
        assert_eq!(m.pp as u64, j["pp"].as_u64().unwrap(), "{w} pp");
        assert_eq!(m.crit_ratio as u64, j["critRatio"].as_u64().unwrap(), "{w} critRatio");
        match m.damage() {
            None => assert!(j["damage"].is_null(), "{w} damage"),
            Some(FixedDamage::Level) => assert_eq!(j["damage"], "level", "{w} damage"),
            Some(FixedDamage::Hp(n)) => assert_eq!(n as u64, j["damage"].as_u64().unwrap(), "{w} damage"),
        }
        match m.multihit() {
            None => assert!(j["multihit"].is_null(), "{w} multihit"),
            Some([a, b]) => {
                assert_eq!((a as u64, b as u64), (j["multihit"][0].as_u64().unwrap(), j["multihit"][1].as_u64().unwrap()), "{w} multihit")
            }
        }
        check_fraction(m.drain(), &j["drain"], &format!("{w} drain"));
        check_fraction(m.recoil(), &j["recoil"], &format!("{w} recoil"));
        check_fraction(m.heal(), &j["heal"], &format!("{w} heal"));
        assert_eq!(m.selfdestruct().map(|x| x.id()), j["selfdestruct"].as_str(), "{w} selfdestruct");
        match m.self_switch() {
            None => assert!(j["selfSwitch"].is_null(), "{w} selfSwitch"),
            Some(SelfSwitch::True) => assert_eq!(j["selfSwitch"], Value::Bool(true), "{w} selfSwitch"),
            Some(SelfSwitch::CopyVolatile) => assert_eq!(j["selfSwitch"], "copyvolatile", "{w} selfSwitch"),
        }
        check_cond(d, m.status, &j["status"], &format!("{w} status"));
        check_cond(d, m.volatile_status, &j["volatileStatus"], &format!("{w} volatileStatus"));
        check_cond(d, m.side_condition(), &j["sideCondition"], &format!("{w} sideCondition"));
        check_cond(d, m.slot_condition(), &j["slotCondition"], &format!("{w} slotCondition"));
        check_cond(d, m.weather(), &j["weather"], &format!("{w} weather"));
        check_cond(d, m.pseudo_weather(), &j["pseudoWeather"], &format!("{w} pseudoWeather"));
        check_cond(d, m.condition, &j["condition"], &format!("{w} condition"));
        if !m.condition.is_none() {
            assert_eq!(d.condition_def(m.condition).source, ConditionSource::Move, "{w} condition source");
        }
        check_boosts(m.boosts(), &j["boosts"], &w);
        if let Some(e) = m.extra {
            assert_ne!(*e, MoveExtra::NONE, "{w}: an all-unset extra should be None");
        }
        let flags = strs(&j["flags"]);
        for (bit, name) in MOVE_FLAG_NAMES.iter().enumerate() {
            assert_eq!(m.flags.0 >> bit & 1 == 1, flags.iter().any(|f| f == name), "{w} flag {name}");
            assert_eq!(m.flags.has(name), flags.iter().any(|f| f == name), "{w} flag {name}");
        }
        let bools = strs(&j["bools"]);
        for (bit, name) in MOVE_BOOL_NAMES.iter().enumerate() {
            assert_eq!(m.bools.0 >> bit & 1 == 1, bools.iter().any(|f| f == name), "{w} {name}");
        }
        assert!(flags.iter().all(|f| m.flags.has(f)) && bools.iter().all(|b| m.bools.has(b)), "{w} unknown flag");
        let secs = j["secondaries"].as_array().cloned().unwrap_or_default();
        assert_eq!(m.secondaries().len(), secs.len(), "{w} secondaries");
        for (k, (sec, js)) in m.secondaries().iter().zip(&secs).enumerate() {
            let ws = format!("{w}.secondaries[{k}]");
            assert_eq!(sec.chance.map(u64::from), opt_u64(&js["chance"]), "{ws} chance");
            check_cond(d, sec.status, &js["status"], &format!("{ws} status"));
            check_cond(d, sec.volatile_status, &js["volatileStatus"], &format!("{ws} volatileStatus"));
            check_boosts(sec.boosts, &js["boosts"], &ws);
            check_self(d, sec.self_effect, &js["self"], &format!("{ws}.self"));
            check_handlers(sec.handlers, js, &ws);
        }
        check_self(d, m.self_effect(), &j["self"], &format!("{w}.self"));
        check_handlers(m.handlers, j, &w);
    }
    json.len()
}

fn check_species(gen: &str) -> usize {
    let d = dex(gen);
    let json = list(gen, "species.json");
    assert_eq!(d.species.len(), json.len(), "{gen} species count");
    for (i, (sp, j)) in d.species.iter().zip(&json).enumerate() {
        let key = s(&j["key"]);
        let w = format!("{gen} species {key}");
        assert_eq!(sp.name.as_str(), s(&j["name"]), "{w}");
        assert_eq!(d.species_id(key), Some(SpeciesId(i as u16)), "{w} lookup");
        assert_eq!(sp.num as i64, j["num"].as_i64().unwrap(), "{w} num");
        assert_eq!(sp.base_species().as_str(), s(&j["baseSpecies"]), "{w} baseSpecies");
        assert_eq!(sp.forme.map_or("", |f| f.forme.as_str()), s(&j["forme"]), "{w} forme");
        assert_eq!(sp.forme.map_or("", |f| f.battle_only.as_str()), s(&j["battleOnly"]), "{w} battleOnly");
        assert_eq!(sp.forme.map_or("", |f| f.required_item.as_str()), s(&j["requiredItem"]), "{w} requiredItem");
        let types: Vec<Type> = strs(&j["types"]).iter().map(|t| ty(t)).collect();
        assert_eq!(sp.types[0], types[0], "{w} type 1");
        assert_eq!(sp.types[1], types.get(1).copied().unwrap_or(Type::None), "{w} type 2");
        let stats: Vec<u64> = j["baseStats"].as_array().unwrap().iter().map(|x| x.as_u64().unwrap()).collect();
        assert_eq!(sp.base_stats.iter().map(|&x| x as u64).collect::<Vec<_>>(), stats, "{w} baseStats");
        assert_eq!(sp.ability(0), j["abilities"][0].as_str(), "{w} ability 0");
        assert_eq!(sp.ability(1), j["abilities"][1].as_str(), "{w} ability 1");
        assert_eq!(sp.gender.letter(), s(&j["gender"]), "{w} gender");
        let ratio: Vec<f64> = j["genderRatio"].as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect();
        assert_eq!([sp.gender_ratio[0] as f64 / 8.0, sp.gender_ratio[1] as f64 / 8.0], [ratio[0], ratio[1]], "{w} genderRatio");
        assert_eq!(sp.weighthg as u64, j["weighthg"].as_u64().unwrap(), "{w} weighthg");
        assert_eq!(sp.nfe, j["nfe"].as_bool().unwrap(), "{w} nfe");
        assert_eq!(sp.max_hp().map(u64::from), opt_u64(&j["maxHP"]), "{w} maxHP");
        let cosmetic: Vec<&str> = d.cosmetic_formes(SpeciesId(i as u16)).iter().map(|n| n.as_str()).collect();
        let want = strs(&j["cosmeticFormes"]);
        assert_eq!(cosmetic, want, "{w} cosmeticFormes");
        for (k, name) in want.iter().enumerate() {
            let id: String = name.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_lowercase();
            assert_eq!(d.cosmetic_forme_id(&id), Some((SpeciesId(i as u16), k as u8 + 1)), "{w} {name}");
            assert_eq!(d.cosmetic_forme_name(SpeciesId(i as u16), k as u8 + 1).map(|n| n.as_str()), Some(name.as_str()), "{w}");
        }
    }
    json.len()
}

fn check_items(gen: &str) -> usize {
    let d = dex(gen);
    let json = list(gen, "items.json");
    assert_eq!(d.items.len(), json.len(), "{gen} item count");
    for (i, (it, j)) in d.items.iter().zip(&json).enumerate() {
        let key = s(&j["key"]);
        let w = format!("{gen} item {key}");
        assert_eq!(it.name.as_str(), s(&j["name"]), "{w}");
        assert_eq!(d.item_id(key), Some(ItemId(i as u16)), "{w} lookup");
        assert_eq!(it.num as i64, j["num"].as_i64().unwrap(), "{w} num");
        assert_eq!(it.is_berry, j["isBerry"].as_bool().unwrap(), "{w}");
        assert_eq!(it.is_gem, j["isGem"].as_bool().unwrap(), "{w}");
        assert_eq!(it.is_pokeball, j["isPokeball"].as_bool().unwrap(), "{w}");
        assert_eq!(it.is_choice, j["isChoice"].as_bool().unwrap(), "{w}");
        assert_eq!(it.ignore_klutz, j["ignoreKlutz"].as_bool().unwrap(), "{w}");
        match it.natural_gift {
            None => assert!(j["naturalGift"].is_null(), "{w} naturalGift"),
            Some(n) => {
                assert_eq!(n.base_power as u64, j["naturalGift"]["basePower"].as_u64().unwrap(), "{w}");
                assert_eq!(n.type_, ty(s(&j["naturalGift"]["type"])), "{w}");
            }
        }
        match it.fling {
            None => assert!(j["fling"].is_null(), "{w} fling"),
            Some(f) => {
                assert_eq!(f.base_power as u64, j["fling"]["basePower"].as_u64().unwrap(), "{w}");
                assert_eq!(f.status.as_str(), s(&j["fling"]["status"]), "{w}");
                assert_eq!(f.volatile_status.as_str(), s(&j["fling"]["volatileStatus"]), "{w}");
                assert_eq!(f.effect, j["fling"]["effect"].as_bool().unwrap(), "{w}");
            }
        }
        let users: Vec<&str> = it.item_user.iter().map(|u| u.as_str()).collect();
        assert_eq!(users, strs(&j["itemUser"]), "{w} itemUser");
        check_boosts(it.boosts, &j["boosts"], &w);
        check_cond(d, it.condition, &j["condition"], &format!("{w} condition"));
        check_handlers(it.handlers, j, &w);
    }
    json.len()
}

fn check_abilities(gen: &str) -> usize {
    let d = dex(gen);
    let json = list(gen, "abilities.json");
    assert_eq!(d.abilities.len(), json.len(), "{gen} ability count");
    for (i, (a, j)) in d.abilities.iter().zip(&json).enumerate() {
        let key = s(&j["key"]);
        let w = format!("{gen} ability {key}");
        assert_eq!(a.name.as_str(), s(&j["name"]), "{w}");
        assert_eq!(d.ability_id(key), Some(AbilityId(i as u16)), "{w} lookup");
        assert_eq!(a.num as i64, j["num"].as_i64().unwrap(), "{w} num");
        assert_eq!(a.rating_tenths as f64 / 10.0, j["rating"].as_f64().unwrap(), "{w} rating");
        assert_eq!(a.suppress_weather, j["suppressWeather"].as_bool().unwrap(), "{w}");
        let flags = strs(&j["flags"]);
        for (bit, name) in ABILITY_FLAG_NAMES.iter().enumerate() {
            assert_eq!(a.flags.0 >> bit & 1 == 1, flags.iter().any(|f| f == name), "{w} flag {name}");
        }
        check_cond(d, a.condition, &j["condition"], &format!("{w} condition"));
        check_handlers(a.handlers, j, &w);
    }
    json.len()
}

fn check_conditions(gen: &str) -> usize {
    let d = dex(gen);
    let json = list(gen, "conditions.json");
    assert_eq!(d.conditions.len(), json.len(), "{gen} condition count");
    for (i, (c, j)) in d.conditions.iter().zip(&json).enumerate() {
        let key = s(&j["key"]);
        let w = format!("{gen} condition {key}");
        assert_eq!(c.key.as_str(), key, "{w}");
        assert_eq!(d.condition_id(key), Some(ConditionId(i as u16)), "{w} lookup");
        assert_eq!(c.name.as_str(), s(&j["name"]), "{w} name");
        assert_eq!(c.effect_type.id(), s(&j["effectType"]), "{w} effectType");
        assert_eq!(c.source.id(), s(&j["source"]), "{w} source");
        assert_eq!(c.duration.map(u64::from), opt_u64(&j["duration"]), "{w} duration");
        assert_eq!(c.counter_max.map(u64::from), opt_u64(&j["counterMax"]), "{w} counterMax");
        assert_eq!(c.no_copy, j["noCopy"].as_bool().unwrap(), "{w} noCopy");
        assert_eq!(c.affects_fainted, j["affectsFainted"].as_bool().unwrap(), "{w} affectsFainted");
        check_handlers(c.handlers, j, &w);
    }
    json.len()
}

fn check_typechart(gen: &str) {
    let d = dex(gen);
    let j = load(gen, "typechart.json");
    let order: Vec<Type> = strs(&j["order"]).iter().map(|t| ty(t)).collect();
    assert_eq!(d.typechart.order, &order[..], "{gen} type order");
    let mut specials = 0;
    for row in j["types"].as_array().unwrap() {
        let def = ty(s(&row["name"]));
        for t in Type::ALL {
            let want = row["damageTaken"].get(t.ps_name()).map_or(0, |c| c.as_u64().unwrap());
            if !order.contains(&t) {
                assert_eq!(want, 0);
            }
            assert_eq!(d.typechart.code(t, def) as u64, want, "{gen} {} vs {}", t.name(), def.name());
        }
        for (id, code) in row["special"].as_object().unwrap() {
            assert_eq!(d.typechart.special_code(id, def) as u64, code.as_u64().unwrap(), "{gen} {id} vs {}", def.name());
            specials += 1;
        }
    }
    assert_eq!(d.typechart.special.len(), specials, "{gen} special immunities");
    // Rows of types the gen lacks, and `???`, are all neutral.
    for def in Type::ALL {
        if !order.contains(&def) {
            assert!(Type::ALL.iter().all(|&a| d.typechart.code(a, def) == 0), "{gen} {}", def.name());
        }
    }
}

#[test]
fn gen1_round_trips() {
    assert_eq!(GEN1.gen, 1);
    assert_eq!(check_moves("gen1"), 165);
    assert_eq!(check_species("gen1"), 151);
    assert_eq!(check_items("gen1"), 9);
    assert_eq!(check_abilities("gen1"), 1);
    assert_eq!(check_conditions("gen1"), 55);
    check_typechart("gen1");
}

#[test]
fn gen3_round_trips() {
    assert_eq!(GEN3.gen, 3);
    assert_eq!(check_moves("gen3"), 370);
    assert_eq!(check_species("gen3"), 392);
    // Unown's 27 letters are gen 3's only cosmetic formes.
    assert_eq!(GEN3.cosmetic_formes.len(), 1);
    assert_eq!(GEN3.cosmetic_forme_id("unownquestion"), Some((GEN3.species_id("unown").unwrap(), 27)));
    assert_eq!(check_items("gen3"), 106);
    // 76 standard abilities, then `noability` (dump-dex.js appends it).
    assert_eq!(check_abilities("gen3"), 77);
    assert_eq!(check_conditions("gen3"), 90);
    check_typechart("gen3");
}

fn function_count(hs: &[Handler]) -> usize {
    hs.iter().filter(|h| h.value == HandlerValue::Function).count()
}

/// Functions of an effect plus its nested `condition` (and, for a move, its
/// `self` and `secondaries`), the way design section 6 counts them.
fn move_functions(d: &Dex, m: &MoveDef) -> usize {
    let mut n = function_count(m.handlers);
    if let Some(e) = m.self_effect() {
        n += function_count(e.handlers);
    }
    for sec in m.secondaries() {
        n += function_count(sec.handlers);
        if let Some(e) = sec.self_effect {
            n += function_count(e.handlers);
        }
    }
    n + nested_condition(d, m.condition, ConditionSource::Move)
}

fn nested_condition(d: &Dex, id: ConditionId, source: ConditionSource) -> usize {
    if id.is_none() {
        return 0;
    }
    let c = d.condition_def(id);
    // A move's condition that the Conditions table shadows (same key) is not
    // the move's; the dumper keeps the first one PS's getByID finds.
    if c.source == source {
        function_count(c.handlers)
    } else {
        0
    }
}

/// Design section 6: "Gen 1: 165 moves, 40 with handlers (66 functions);
/// 151 species; no items or abilities. Gen 3: 370 moves, 129 with handlers
/// (296 functions); 106 items, 60 with handlers (99 functions); 76
/// abilities, 68 with handlers (96 functions); 392 species."
#[test]
fn counts_match_design_section_6() {
    let tally = |d: &Dex| {
        let moves: Vec<usize> = d.moves.iter().map(|m| move_functions(d, m)).collect();
        let items: Vec<usize> = d
            .items
            .iter()
            .map(|i| function_count(i.handlers) + nested_condition(d, i.condition, ConditionSource::Item) + i.fling.map_or(0, |f| f.effect as usize))
            .collect();
        let abilities: Vec<usize> = d
            .abilities
            .iter()
            .map(|a| function_count(a.handlers) + nested_condition(d, a.condition, ConditionSource::Ability))
            .collect();
        let with = |v: &[usize]| v.iter().filter(|&&n| n > 0).count();
        let sum = |v: &[usize]| v.iter().sum::<usize>();
        (
            (d.moves.len(), with(&moves), sum(&moves)),
            (d.items.len(), with(&items), sum(&items)),
            (d.abilities.len(), with(&abilities), sum(&abilities)),
            d.species.len(),
        )
    };
    // Gen 1's "no items or abilities": PS's gen 1 dex has 9 standard items
    // (Poke Balls and evolution stones) and the `noability` ability, none
    // with a handler, so nothing to port.
    assert_eq!(tally(&GEN1), ((165, 40, 66), (9, 0, 0), (1, 0, 0), 151));
    // Abilities: 96 functions at 0.11.11; the re-synced pin (c0845f33d)
    // adds `onAfterEntryHazard` to Air Lock, Cloud Nine, Drizzle, Drought,
    // Forecast and Sand Stream (R-1 section 4.3, G3-1). The 77th ability is
    // `noability` (no handlers), appended for custom games.
    assert_eq!(tally(&GEN3), ((370, 129, 296), (106, 60, 99), (77, 68, 102), 392));
}

#[test]
fn ids_and_constants() {
    assert_eq!(GEN1.move_id("bide"), Some(gen1::moves::BIDE));
    assert_eq!(GEN3.ability_id("intimidate"), Some(gen3::abilities::INTIMIDATE));
    assert_eq!(GEN3.condition_id("brn"), Some(gen3::conditions::BRN));
    assert_eq!(GEN1.species_id("mew"), Some(gen1::species::MEW));
    assert_eq!(GEN3.item_id("leftovers"), Some(gen3::items::LEFTOVERS));
    // The Hidden Power hack: hiddenpowerbug's move.id is hiddenpower.
    assert_eq!(GEN3.move_ps_id(gen3::moves::HIDDENPOWERBUG), gen3::moves::HIDDENPOWER);
    assert_eq!(GEN3.move_def(gen3::moves::HIDDENPOWERBUG).type_, Type::Bug);
    // Constant handlers.
    let teleport = GEN1.move_def(gen1::moves::TELEPORT);
    assert_eq!(find_handler(teleport.handlers, "onTry").unwrap().value, HandlerValue::False);
    let recharge = GEN1.condition_def(gen1::conditions::MUSTRECHARGE);
    assert_eq!(find_handler(recharge.handlers, "onLockMove").unwrap().constant_id(), Some("recharge"));
    // PS's `???`.
    assert_eq!(GEN1.move_def(gen1::moves::BIDE).type_, Type::None);
    assert_eq!(GEN3.move_def(gen3::moves::CURSE).type_, Type::None);
    assert!(GEN1.move_def(gen1::moves::BODYSLAM).flags.contains(MoveFlags::CONTACT));
    assert!(GEN3.move_def(gen3::moves::SHEERCOLD).bools.contains(MoveBools::OHKO));
    assert_eq!(GEN3.typechart.special_code("sandstorm", Type::Steel), 3);
    assert_eq!(GEN1.typechart.code(Type::Ghost, Type::Psychic), 3, "gen 1 Ghost does not affect Psychic");
}

#[test]
fn type_names_unchanged() {
    // The old `battle_types::Type` contract: same variants, names, abbrs.
    assert_eq!(Type::ALL.len(), TYPE_COUNT);
    for (i, t) in Type::ALL.iter().enumerate() {
        assert_eq!(*t as usize, i);
    }
    assert_eq!(Type::Fire.name(), "Fire");
    assert_eq!(Type::Fire.abbr(), "FIR");
    assert_eq!(Type::None.name(), "");
    assert_eq!(Type::None.abbr(), "");
    assert_eq!(Type::None.ps_name(), "???");
}
