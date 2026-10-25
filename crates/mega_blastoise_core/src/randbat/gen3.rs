//! Gen 3 random-battle teams, drawn the way Showdown's `gen3randombattle`
//! draws them: species, role, moves, ability, held item, EVs and IVs.
//!
//! PS: data/random-battles/gen3/teams.ts `RandomGen3Teams`, with the helpers
//! it inherits from data/random-battles/gen9/teams.ts `RandomTeams`
//! (`queryMoves`, `addMove`, `incompatibleMoves`, `getMoveType`,
//! `getForme`, `getPokemonPool`, `fastPop`, `sampleNoReplace`) and gen 4's
//! `PRIORITY_POKEMON`. The data is PS's `sets.json` (built into
//! `GEN3_RANDOM_SETS` by `build.rs`) and `ps_dex::GEN3`. The PRNG is PS's
//! (`ps_sim::prng::Prng`), called in PS's order, so a seed draws the team PS's
//! generator draws for it (the `same_teams_as_ps` test).
//!
//! Only the [`MoveCounter`] keys some gen 3 rule reads are counted
//! (`queryMoves` also counts `technician`, `sheerforce` and others that
//! only later gens look at).

extern crate alloc;

use alloc::vec::Vec;

use ps_dex::{find_handler, Dex, MoveCategory, SpeciesDef, Type};
use ps_protocol::{Gender, Name, PackedSet, PackedTeam, StatTable};
use ps_sim::prng::Prng;

/// One `sets.json` set.
pub struct SetData {
    pub role: &'static str,
    pub movepool: &'static [&'static str],
    pub abilities: &'static [&'static str],
    pub preferred_types: &'static [&'static str],
}

/// One `sets.json` species: its level and its sets.
pub struct SpeciesSets {
    pub id: &'static str,
    pub level: u8,
    pub sets: &'static [SetData],
}

include!(concat!(env!("OUT_DIR"), "/gen3_random_sets.rs"));

const DEX: &Dex = &ps_dex::GEN3;

/// `ruleTable.maxMoveCount` for `gen3randombattle`.
const MAX_MOVE_COUNT: usize = 4;

// Moves that restore HP:
const RECOVERY_MOVES: &[&str] =
    &["milkdrink", "moonlight", "morningsun", "recover", "slackoff", "softboiled", "synthesis"];
// Conglomerate for ease of access
const SETUP: &[&str] = &[
    "acidarmor", "agility", "bellydrum", "bulkup", "calmmind", "curse", "dragondance", "growth",
    "howl", "irondefense", "meditate", "raindance", "sunnyday", "swordsdance", "tailglow",
];
// Moves that shouldn't be the only STAB moves:
const NO_STAB: &[&str] = &[
    "eruption", "explosion", "fakeout", "focuspunch", "futuresight", "icywind", "knockoff",
    "machpunch", "pursuit", "quickattack", "rapidspin", "selfdestruct", "skyattack", "waterspout",
];
// Moves that should be paired together when possible
const MOVE_PAIRS: &[[&str; 2]] = &[
    ["sleeptalk", "rest"],
    ["protect", "wish"],
    ["leechseed", "substitute"],
    ["focuspunch", "substitute"],
    ["batonpass", "spiderweb"],
];
/// Pokemon who should never be in the lead slot
const NO_LEAD_POKEMON: &[&str] = &["dugtrio", "wobbuffet"];
/// PS: data/random-battles/gen4/teams.ts `PRIORITY_POKEMON` (gen 3's
/// constructor runs gen 4's, which sets it).
const PRIORITY_POKEMON: &[&str] =
    &["cacturne", "dusknoir", "honchkrow", "mamoswine", "scizor", "shedinja", "shiftry"];

/// PS's `RandomTeamsTypes.TeamDetails`, the keys gen 3 sets.
#[derive(Default)]
struct TeamDetails {
    rain: bool,
    sun: bool,
    sand: bool,
    status_cure: bool,
    spikes: u8,
    rapid_spin: bool,
}

/// PS's `MoveCounter`: a multiset of tags, plus the damaging moves in
/// insertion order.
#[derive(Default)]
struct MoveCounter {
    counts: Vec<(&'static str, u32)>,
    damaging_moves: Vec<&'static str>,
}

impl MoveCounter {
    fn get(&self, key: &str) -> u32 {
        self.counts.iter().find(|(k, _)| *k == key).map_or(0, |(_, n)| *n)
    }

    fn add(&mut self, key: &'static str) {
        match self.counts.iter_mut().find(|(k, _)| *k == key) {
            Some((_, n)) => *n += 1,
            None => self.counts.push((key, 1)),
        }
    }

    fn set(&mut self, key: &'static str, n: u32) {
        match self.counts.iter_mut().find(|(k, _)| *k == key) {
            Some((_, v)) => *v = n,
            None => self.counts.push((key, n)),
        }
    }

    fn add_damaging(&mut self, id: &'static str) {
        if !self.damaging_moves.contains(&id) {
            self.damaging_moves.push(id);
        }
    }
}

/// One drawn set (PS's `RandomTeamsTypes.RandomSet`, the fields a team
/// packs).
#[derive(Clone, Debug)]
pub struct RandomSet {
    pub species_id: &'static str,
    /// `getForme`'s pick: the species, or a cosmetic forme (`Unown-B`).
    pub species: &'static str,
    /// The species entry's own name (`Deoxys-Attack`, `Unown` for an
    /// `Unown-B`): the nickname [`packed`] gives the mon.
    pub nickname: &'static str,
    pub gender: Gender,
    pub genderless_or_fixed: bool,
    pub shiny: bool,
    pub level: u8,
    pub moves: Vec<&'static str>,
    pub ability: &'static str,
    pub item: &'static str,
    pub evs: [u16; 6],
    pub ivs: [u16; 6],
}

fn species(id: &str) -> &'static SpeciesDef {
    DEX.species_def(DEX.species_id(id).unwrap_or_else(|| panic!("no gen 3 species {id}")))
}

fn mv(id: &str) -> &'static ps_dex::MoveDef {
    DEX.move_def(DEX.move_id(id).unwrap_or_else(|| panic!("no gen 3 move {id}")))
}

fn types_of(s: &SpeciesDef) -> impl Iterator<Item = Type> + '_ {
    s.types.iter().copied().filter(|t| *t != Type::None)
}

fn type_named(name: &str) -> Type {
    Type::from_name(name).unwrap_or(Type::None)
}

/// `move.basePower || move.basePowerCallback`.
fn has_base_power(m: &ps_dex::MoveDef) -> bool {
    m.base_power > 0 || find_handler(m.handlers, "basePowerCallback").is_some()
}

fn index_of(list: &[&'static str], id: &str) -> Option<usize> {
    list.iter().position(|m| *m == id)
}

/// PS `fastPop`: the last element takes the removed one's place. PS throws
/// on an index of -1 (`indexOf` missed); every caller here checks first.
fn fast_pop(list: &mut Vec<&'static str>, index: usize) -> &'static str {
    list.swap_remove(index)
}

fn pop_id(list: &mut Vec<&'static str>, id: &str) {
    if let Some(i) = index_of(list, id) {
        fast_pop(list, i);
    }
}

/// PS `dex.getEffectiveness(type, species)`: the sum over the species'
/// types of +1 (weak), -1 (resists), 0 (neutral or immune).
fn effectiveness(attacker: Type, s: &SpeciesDef) -> i32 {
    types_of(s)
        .map(|t| match DEX.typechart.code(attacker, t) {
            1 => 1,
            2 => -1,
            _ => 0,
        })
        .sum()
}

/// PS `typeData.HPivs` for gen 3's Hidden Power types, in `[hp, atk, def,
/// spa, spd, spe]` order (`None` where the type leaves the IV alone).
fn hp_ivs(t: &str) -> [Option<u16>; 6] {
    const T: Option<u16> = Some(30);
    const N: Option<u16> = None;
    match t {
        "bug" => [N, T, T, N, T, N],
        "dark" => [N; 6],
        "dragon" => [N, T, N, N, N, N],
        "electric" => [N, N, N, T, N, N],
        "fighting" => [N, N, T, T, T, T],
        "fire" => [N, T, N, T, N, T],
        "flying" => [T, T, T, T, T, N],
        "ghost" => [N, N, T, N, T, N],
        "grass" => [N, T, N, T, N, N],
        "ground" => [N, N, N, T, T, N],
        "ice" => [N, T, T, N, N, N],
        "poison" => [N, N, T, T, T, N],
        "psychic" => [N, T, N, N, N, T],
        "rock" => [N, N, T, N, T, T],
        "steel" => [N, N, N, N, T, N],
        "water" => [N, T, T, T, N, N],
        _ => [N; 6],
    }
}

/// PS's `RandomGen3Teams`: one generator, which a battle keeps for both
/// players (`battle.teamGenerator`), so `battleHasWobbuffet` holds across
/// the two teams.
pub struct Gen3Teams {
    prng: Prng,
    max_team_size: usize,
    battle_has_ditto: bool,
    battle_has_wobbuffet: bool,
}

impl Gen3Teams {
    pub fn new(seed: [u16; 4], max_team_size: usize) -> Self {
        Gen3Teams {
            prng: Prng::new(seed),
            max_team_size,
            battle_has_ditto: false,
            battle_has_wobbuffet: false,
        }
    }

    /// PS `setSeed`.
    pub fn set_seed(&mut self, seed: [u16; 4]) {
        self.prng = Prng::new(seed);
    }

    fn sample<T: Copy>(&mut self, items: &[T]) -> T {
        *self.prng.sample(items)
    }

    /// PS `sampleNoReplace`.
    fn sample_no_replace(&mut self, list: &mut Vec<&'static str>) -> Option<&'static str> {
        if list.is_empty() {
            return None;
        }
        let i = self.prng.random_n(list.len() as u32) as usize;
        Some(fast_pop(list, i))
    }

    /// PS `getMoveType` (the branches a gen 3 move can reach).
    fn move_type(&self, id: &str, abilities: &[&str]) -> Type {
        let m = mv(id);
        if m.name.as_str() == "Weather Ball" {
            if abilities.iter().any(|a| ["Chlorophyll", "Drought"].contains(a)) {
                return Type::Fire;
            }
            if abilities.contains(&"Drizzle") {
                return Type::Water;
            }
        }
        m.type_
    }

    /// PS `queryMoves`.
    fn query_moves(&self, moves: &[&'static str], s: &SpeciesDef, sid: &str, abilities: &[&str]) -> MoveCounter {
        let mut counter = MoveCounter::default();
        if moves.is_empty() {
            return counter;
        }
        let mut categories = [0u32; 3];
        for &id in moves {
            let m = mv(id);
            let move_type = self.move_type(id, abilities);
            if m.damage().is_some() || find_handler(m.handlers, "damageCallback").is_some() {
                // Moves that do a set amount of damage:
                counter.add("damage");
                counter.add_damaging(id);
            } else {
                // Are Physical/Special/Status moves:
                categories[match m.category {
                    MoveCategory::Physical => 0,
                    MoveCategory::Special => 1,
                    MoveCategory::Status => 2,
                }] += 1;
            }
            if m.recoil().is_some() || m.bools.has("hasCrashDamage") {
                counter.add("recoil");
            }
            // Moves which have a base power:
            if has_base_power(m) {
                if !NO_STAB.contains(&id) || (PRIORITY_POKEMON.contains(&sid) && m.priority > 0) {
                    counter.add(move_type.name());
                    if types_of(s).any(|t| t == move_type) {
                        counter.add("stab");
                    }
                    counter.add_damaging(id);
                }
                if m.priority > 0 {
                    counter.add("priority");
                }
            }
            // Moves with low accuracy:
            if m.accuracy != 0 && m.accuracy < 90 {
                counter.add("inaccurate");
            }
        }
        counter.set("Physical", categories[0]);
        counter.set("Special", categories[1]);
        counter.set("Status", categories[2]);
        counter
    }

    /// PS `incompatibleMoves`.
    fn incompatible_moves(moves: &[&'static str], pool: &mut Vec<&'static str>, a: &[&str], b: &[&str]) {
        if moves.len() + pool.len() <= MAX_MOVE_COUNT {
            return;
        }
        for &id1 in moves {
            if b.contains(&id1) {
                for &id2 in a {
                    if id1 != id2 && pool.contains(&id2) {
                        pop_id(pool, id2);
                        if moves.len() + pool.len() <= MAX_MOVE_COUNT {
                            return;
                        }
                    }
                }
            }
            if a.contains(&id1) {
                for &id2 in b {
                    if id1 != id2 && pool.contains(&id2) {
                        pop_id(pool, id2);
                        if moves.len() + pool.len() <= MAX_MOVE_COUNT {
                            return;
                        }
                    }
                }
            }
        }
    }

    /// PS `RandomGen3Teams.cullMovePool`.
    fn cull_move_pool(moves: &[&'static str], pool: &mut Vec<&'static str>, td: &TeamDetails, role: &str) {
        // Pokemon cannot have multiple Hidden Powers in any circumstance
        if moves.iter().any(|m| m.starts_with("hiddenpower")) {
            while let Some(i) = pool.iter().position(|m| m.starts_with("hiddenpower")) {
                fast_pop(pool, i);
            }
        }

        if moves.len() + pool.len() <= MAX_MOVE_COUNT {
            return;
        }
        // If we have two unfilled moves and only one unpaired move, cull the unpaired move.
        if moves.len() == MAX_MOVE_COUNT - 2 {
            let mut unpaired = pool.clone();
            for [a, b] in MOVE_PAIRS {
                if pool.contains(a) && pool.contains(b) {
                    pop_id(&mut unpaired, a);
                    pop_id(&mut unpaired, b);
                }
            }
            if unpaired.len() == 1 {
                pop_id(pool, unpaired[0]);
            }
        }

        // These moves are paired, and shouldn't appear if there is not room for them both.
        if moves.len() == MAX_MOVE_COUNT - 1 {
            for [a, b] in MOVE_PAIRS {
                if pool.contains(a) && pool.contains(b) {
                    pop_id(pool, a);
                    pop_id(pool, b);
                }
            }
        }

        // Team-based move culls
        if td.rapid_spin {
            pop_id(pool, "rapidspin");
            if moves.len() + pool.len() <= MAX_MOVE_COUNT {
                return;
            }
        }
        if td.spikes >= 2 {
            pop_id(pool, "spikes");
            if moves.len() + pool.len() <= MAX_MOVE_COUNT {
                return;
            }
        }
        if td.status_cure {
            pop_id(pool, "aromatherapy");
            pop_id(pool, "healbell");
            if moves.len() + pool.len() <= MAX_MOVE_COUNT {
                return;
            }
        }

        // Develop additional move lists
        let bad_with_setup: &[&str] = &["knockoff", "rapidspin", "toxic"];

        // General incompatibilities
        let incompatible_pairs: [(&[&str], &[&str]); 9] = [
            // These moves don't mesh well with other aspects of the set
            (GEN3_STATUS_MOVES, &["trick"]),
            (SETUP, bad_with_setup),
            (&["rest"], &["protect", "substitute"]),
            (&["selfdestruct", "explosion"], &["destinybond", "painsplit", "rest"]),
            // These attacks are redundant with each other
            (&["surf"], &["hydropump"]),
            (&["bodyslam", "return"], &["bodyslam", "doubleedge"]),
            (&["fireblast"], &["flamethrower"]),
            // Assorted hardcodes go here:
            // Granbull
            (&["bulkup"], &["overheat"]),
            // Heracross
            (&["endure"], &["substitute"]),
        ];
        for (a, b) in incompatible_pairs {
            Self::incompatible_moves(moves, pool, a, b);
        }

        let status_inflicting: &[&str] = &["stunspore", "thunderwave", "toxic", "willowisp", "yawn"];
        if role != "Staller" {
            Self::incompatible_moves(moves, pool, status_inflicting, status_inflicting);
        }
    }

    /// PS `addMove`.
    #[allow(clippy::too_many_arguments)]
    fn add_move(
        &self,
        id: &'static str,
        moves: &mut Vec<&'static str>,
        pool: &mut Vec<&'static str>,
        s: &SpeciesDef,
        sid: &str,
        abilities: &[&str],
        td: &TeamDetails,
        role: &str,
    ) -> MoveCounter {
        if !moves.contains(&id) {
            moves.push(id);
        }
        pop_id(pool, id);
        let counter = self.query_moves(moves, s, sid, abilities);
        Self::cull_move_pool(moves, pool, td, role);
        counter
    }

    /// PS `RandomGen3Teams.moveEnforcementCheckers[type]`, false for a type
    /// with no checker.
    fn enforcement_checker(t: Type, counter: &MoveCounter, s: &SpeciesDef, sid: &str) -> bool {
        let none = |name: &str| counter.get(name) == 0;
        match t {
            Type::Bug => none("Bug") && ["armaldo", "heracross", "parasect"].contains(&sid),
            Type::Dark => none("Dark"),
            Type::Electric => none("Electric"),
            Type::Fighting => none("Fighting"),
            Type::Fire => none("Fire"),
            Type::Flying => none("Flying") && sid != "crobat",
            Type::Ghost => none("Ghost"),
            Type::Ground => none("Ground"),
            Type::Ice => none("Ice"),
            Type::Normal => none("Normal"),
            Type::Poison => none("Poison"),
            Type::Psychic => none("Psychic") && s.base_stats[3] >= 100,
            Type::Rock => none("Rock"),
            Type::Steel => none("Steel") && sid != "forretress",
            Type::Water => none("Water"),
            _ => false,
        }
    }

    /// The pool's moves that count as attacks of a type `keep` accepts
    /// (`!noStab.includes(moveid) && (move.basePower ||
    /// move.basePowerCallback)`), in pool order.
    fn stab_moves(&self, pool: &[&'static str], abilities: &[&str], keep: impl Fn(Type) -> bool) -> Vec<&'static str> {
        pool.iter()
            .copied()
            .filter(|id| !NO_STAB.contains(id) && has_base_power(mv(id)) && keep(self.move_type(id, abilities)))
            .collect()
    }

    /// PS `RandomGen3Teams.randomMoveset`.
    #[allow(clippy::too_many_arguments)]
    fn random_moveset(
        &mut self,
        s: &SpeciesDef,
        sid: &str,
        abilities: &[&str],
        td: &TeamDetails,
        pool: &mut Vec<&'static str>,
        preferred_types: &[&str],
        role: &str,
    ) -> Vec<&'static str> {
        let mut moves: Vec<&'static str> = Vec::new();
        let mut counter = self.query_moves(&moves, s, sid, abilities);
        Self::cull_move_pool(&moves, pool, td, role);

        // If there are only four moves, add all moves and return early
        if pool.len() <= MAX_MOVE_COUNT || sid == "unown" {
            while !pool.is_empty() {
                let id = self.sample(pool);
                self.add_move(id, &mut moves, pool, s, sid, abilities, td, role);
            }
            return moves;
        }

        // Enforce Seismic Toss and Spore
        for id in ["seismictoss", "spikes", "spore"] {
            if pool.contains(&id) {
                counter = self.add_move(id, &mut moves, pool, s, sid, abilities, td, role);
            }
        }

        // Enforce Substitute on non-Setup sets with Baton Pass
        if !role.contains("Setup") && pool.contains(&"batonpass") && pool.contains(&"substitute") {
            counter = self.add_move("substitute", &mut moves, pool, s, sid, abilities, td, role);
        }

        // Enforce moves of all Preferred Types
        for &name in preferred_types {
            if counter.get(name) == 0 {
                let t = type_named(name);
                let stab = self.stab_moves(pool, abilities, |mt| mt == t);
                if !stab.is_empty() {
                    let id = self.sample(&stab);
                    counter = self.add_move(id, &mut moves, pool, s, sid, abilities, td, role);
                }
            }
        }

        // Enforce STAB
        for t in types_of(s) {
            // Check if a STAB move of that type should be required
            let mut stab = self.stab_moves(pool, abilities, |mt| mt == t);
            while Self::enforcement_checker(t, &counter, s, sid) {
                let Some(id) = self.sample_no_replace(&mut stab) else { break };
                counter = self.add_move(id, &mut moves, pool, s, sid, abilities, td, role);
            }
        }

        // If no STAB move was added, add a STAB move
        if counter.get("stab") == 0 {
            let stab = self.stab_moves(pool, abilities, |mt| types_of(s).any(|t| t == mt));
            if !stab.is_empty() {
                let id = self.sample(&stab);
                counter = self.add_move(id, &mut moves, pool, s, sid, abilities, td, role);
            }
        }

        // Enforce recovery
        if ["Bulky Support", "Bulky Attacker", "Bulky Setup", "Staller"].contains(&role) {
            let recovery: Vec<&'static str> = pool.iter().copied().filter(|m| RECOVERY_MOVES.contains(m)).collect();
            if !recovery.is_empty() {
                let id = self.sample(&recovery);
                counter = self.add_move(id, &mut moves, pool, s, sid, abilities, td, role);
            }
        }

        // Enforce Staller moves
        if role == "Staller" {
            for id in ["protect", "toxic", "wish"] {
                if pool.contains(&id) {
                    counter = self.add_move(id, &mut moves, pool, s, sid, abilities, td, role);
                }
            }
        }

        // Enforce setup
        if role.contains("Setup") || role == "Berry Sweeper" {
            let setup: Vec<&'static str> = pool.iter().copied().filter(|m| SETUP.contains(m)).collect();
            if !setup.is_empty() {
                let id = self.sample(&setup);
                counter = self.add_move(id, &mut moves, pool, s, sid, abilities, td, role);
            }
        }

        // Enforce Berry Sweeper moves
        if role == "Berry Sweeper" {
            // Enforce Flail/Reversal
            for id in ["flail", "reversal"] {
                if pool.contains(&id) {
                    counter = self.add_move(id, &mut moves, pool, s, sid, abilities, td, role);
                }
            }
            // Enforce one of Endure and Substitute, but not both
            let hp_control: Vec<&'static str> =
                pool.iter().copied().filter(|m| ["endure", "substitute"].contains(m)).collect();
            if !hp_control.is_empty() {
                let id = self.sample(&hp_control);
                counter = self.add_move(id, &mut moves, pool, s, sid, abilities, td, role);
            }
        }

        // Enforce a move not on the noSTAB list
        if counter.damaging_moves.is_empty() {
            // Choose an attacking move
            let attacking: Vec<&'static str> = pool
                .iter()
                .copied()
                .filter(|id| !NO_STAB.contains(id) && mv(id).category != MoveCategory::Status)
                .collect();
            if !attacking.is_empty() {
                let id = self.sample(&attacking);
                counter = self.add_move(id, &mut moves, pool, s, sid, abilities, td, role);
            }
        }

        // Enforce coverage move
        if ["Fast Attacker", "Setup Sweeper", "Bulky Attacker", "Wallbreaker", "Berry Sweeper"].contains(&role)
            && counter.damaging_moves.len() == 1
        {
            // Find the type of the current attacking move
            let current = mv(counter.damaging_moves[0]).type_;
            // Choose an attacking move that is of different type to the current single attack
            let coverage = self.stab_moves(pool, abilities, |mt| mt != current);
            if !coverage.is_empty() {
                let id = self.sample(&coverage);
                counter = self.add_move(id, &mut moves, pool, s, sid, abilities, td, role);
            }
        }

        // Choose remaining moves randomly from movepool and add them to moves list:
        while moves.len() < MAX_MOVE_COUNT && !pool.is_empty() {
            let id = self.sample(pool);
            self.add_move(id, &mut moves, pool, s, sid, abilities, td, role);
            for [a, b] in MOVE_PAIRS {
                if id == *a && pool.contains(b) {
                    self.add_move(b, &mut moves, pool, s, sid, abilities, td, role);
                }
                if id == *b && pool.contains(a) {
                    self.add_move(a, &mut moves, pool, s, sid, abilities, td, role);
                }
            }
        }
        let _ = counter;
        moves
    }

    /// PS `RandomGen3Teams.shouldCullAbility`.
    fn should_cull_ability(ability: &str, counter: &MoveCounter, td: &TeamDetails) -> bool {
        match ability {
            "Chlorophyll" => !td.sun,
            "Rock Head" => counter.get("recoil") == 0,
            "Swift Swim" => !td.rain,
            _ => false,
        }
    }

    /// PS `RandomGen3Teams.getAbility`.
    fn get_ability(&mut self, abilities: &[&'static str], counter: &MoveCounter, td: &TeamDetails, sid: &str) -> &'static str {
        if abilities.len() <= 1 {
            return abilities.first().copied().unwrap_or("");
        }

        // Hard-code abilities here
        if sid == "yanma" {
            return if counter.get("inaccurate") > 0 { "Compound Eyes" } else { "Speed Boost" };
        }

        // Obtain a list of abilities that are allowed (not culled)
        let allowed: Vec<&'static str> =
            abilities.iter().copied().filter(|a| !Self::should_cull_ability(a, counter, td)).collect();

        // Pick a random allowed ability
        if !allowed.is_empty() {
            return self.sample(&allowed);
        }

        // If all abilities are rejected, prioritize weather abilities over non-weather abilities
        let weather: Vec<&'static str> =
            abilities.iter().copied().filter(|a| ["Chlorophyll", "Swift Swim"].contains(a)).collect();
        if !weather.is_empty() {
            return self.sample(&weather);
        }

        // Pick a random ability
        self.sample(abilities)
    }

    /// PS `RandomGen3Teams.getItem`.
    fn get_item(&mut self, ability: &str, moves: &[&str], counter: &MoveCounter, s: &SpeciesDef, sid: &str, role: &str) -> &'static str {
        let has = |m: &str| moves.contains(&m);
        let physical = counter.get("Physical");
        let special = counter.get("Special");
        // First, the high-priority items
        if sid == "latias" || sid == "latios" {
            return "Soul Dew";
        }
        if sid == "linoone" && role == "Setup Sweeper" {
            return "Silk Scarf";
        }
        if sid == "marowak" {
            return "Thick Club";
        }
        if sid == "pikachu" {
            return "Light Ball";
        }
        if sid == "unown" {
            return if physical > 0 { "Choice Band" } else { "Twisted Spoon" };
        }
        if sid == "deoxys" || sid == "deoxysattack" {
            return "White Herb";
        }

        if has("trick") {
            return "Choice Band";
        }

        // Medium priority items
        if physical >= 4 {
            return "Choice Band";
        }
        if physical >= 3 && (has("batonpass") || (role == "Wallbreaker" && special > 0)) {
            return "Choice Band";
        }

        if sid == "shedinja" {
            return "Lum Berry";
        }

        if has("dragondance") && ability != "Natural Cure" && !has("healbell") && !has("substitute") {
            return "Lum Berry";
        }
        if has("bellydrum") {
            return if has("substitute") { "Salac Berry" } else { "Lum Berry" };
        }

        if has("raindance") && special >= 3 {
            return "Petaya Berry";
        }

        if role == "Berry Sweeper" {
            if has("endure") {
                return "Salac Berry";
            }
            if has("flail") || has("reversal") {
                return if s.base_stats[5] >= 90 { "Liechi Berry" } else { "Salac Berry" };
            }
            if has("substitute") && physical >= 3 {
                return "Liechi Berry";
            }
            if has("substitute") && special >= 3 {
                return "Petaya Berry";
            }
        }

        if sid == "farfetchd" {
            return "Stick";
        }

        let spe = s.base_stats[5];
        let salac_reqs = (60..=100).contains(&spe) && counter.get("priority") == 0;

        if has("swordsdance") && has("substitute") && counter.get("Status") == 2 {
            if salac_reqs {
                return "Salac Berry";
            }
            if spe > 100 && physical >= 2 {
                return "Liechi Berry";
            }
        }

        if has("swordsdance") && counter.get("Status") == 1 {
            if salac_reqs {
                return "Salac Berry";
            }
            if spe > 100 {
                return if physical >= 3 && self.prng.random_chance(1, 2) { "Liechi Berry" } else { "Lum Berry" };
            }
        }

        // Default to Leftovers
        "Leftovers"
    }

    /// PS `getForme` (the branches a gen 3 species can reach).
    fn get_forme(&mut self, sid: SpeciesIdOf) -> &'static str {
        let s = DEX.species_def(sid.0);
        if let Some(f) = s.forme {
            if !f.battle_only.is_empty() {
                return f.battle_only.as_str();
            }
        }
        let cosmetic = DEX.cosmetic_formes(sid.0);
        if !cosmetic.is_empty() {
            let i = self.prng.random_n(cosmetic.len() as u32 + 1) as usize;
            return if i == 0 { s.name.as_str() } else { cosmetic[i - 1].as_str() };
        }
        s.name.as_str()
    }

    /// PS `RandomGen3Teams.randomSet`.
    fn random_set(&mut self, entry: &'static SpeciesSets, td: &TeamDetails) -> RandomSet {
        let sid = entry.id;
        let id = SpeciesIdOf(DEX.species_id(sid).unwrap_or_else(|| panic!("no gen 3 species {sid}")));
        let s = DEX.species_def(id.0);
        let forme = self.get_forme(id);
        let set = &entry.sets[self.prng.random_n(entry.sets.len() as u32) as usize];
        let role = set.role;
        let mut pool: Vec<&'static str> = set.movepool.to_vec();

        let mut evs = [85u16; 6];
        let mut ivs = [31u16; 6];
        let abilities = set.abilities;

        // Get moves
        let moves = self.random_moveset(s, sid, abilities, td, &mut pool, set.preferred_types, role);
        let counter = self.query_moves(&moves, s, sid, abilities);

        // Get ability
        let ability = self.get_ability(abilities, &counter, td, sid);

        // Get items
        let item = self.get_item(ability, &moves, &counter, s, sid, role);

        let level = entry.level;

        // We use a special variable to track Hidden Power
        // so that we can check for all Hidden Powers at once
        let hp_move = moves.iter().rev().find(|m| m.starts_with("hiddenpower"));
        let has_hidden_power = hp_move.is_some();
        if let Some(m) = hp_move {
            for (iv, set_to) in ivs.iter_mut().zip(hp_ivs(&m["hiddenpower".len()..])) {
                if let Some(v) = set_to {
                    *iv = v;
                }
            }
        }

        let base_hp = u32::from(s.base_stats[0]);
        let hp_of = |ivs: &[u16; 6], evs: &[u16; 6]| -> u32 {
            (2 * base_hp + u32::from(ivs[0]) + u32::from(evs[0]) / 4 + 100) * u32::from(level) / 100 + 10
        };
        let has = |m: &str| moves.contains(&m);
        let pinch_berry = ["Salac Berry", "Petaya Berry", "Liechi Berry"].contains(&item);

        // Prepare optimal HP
        while evs[0] > 1 {
            let hp = hp_of(&ivs, &evs);
            if has("substitute") && (has("flail") || has("reversal")) {
                // Flail/Reversal users should be able to use four Substitutes
                if hp % 4 > 0 {
                    break;
                }
            } else if has("substitute") && pinch_berry {
                // Other pinch berry holders should have berries activate after three Substitutes
                if hp % 4 == 0 {
                    break;
                }
            } else if has("bellydrum") {
                // Belly Drum users should be able to use Belly Drum twice
                if hp % 2 > 0 {
                    break;
                }
            } else {
                break;
            }
            evs[0] -= 4;
        }

        // Minimize confusion damage
        if counter.get("Physical") == 0 && !has("transform") {
            evs[1] = 0;
            ivs[1] = if has_hidden_power { (if ivs[1] == 0 { 31 } else { ivs[1] }) - 28 } else { 0 };
        }

        // Prepare optimal HP
        let mut hp = hp_of(&ivs, &evs);
        if has("substitute") && (has("endeavor") || has("flail") || has("reversal")) {
            // Endeavor/Flail/Reversal users should be able to use four Substitutes
            if hp % 4 == 0 {
                evs[0] -= 4;
            }
        } else if has("substitute") && pinch_berry {
            // Other pinch berry holders should have berries activate after three Substitutes
            while hp % 4 > 0 {
                evs[0] -= 4;
                hp = hp_of(&ivs, &evs);
            }
        }

        // shuffle moves to add more randomness to camomons
        let mut shuffled = moves.clone();
        let n = shuffled.len();
        self.prng.shuffle(&mut shuffled, 0, n);

        let shiny = self.prng.random_chance(1, 1024);
        let (gender, genderless_or_fixed) = match s.gender {
            ps_dex::Gender::Either => (Gender::N, false),
            ps_dex::Gender::Male => (Gender::M, true),
            ps_dex::Gender::Female => (Gender::F, true),
            ps_dex::Gender::Neutral => (Gender::N, true),
        };
        RandomSet {
            species_id: sid,
            species: forme,
            nickname: s.name.as_str(),
            gender,
            genderless_or_fixed,
            shiny,
            level,
            moves: shuffled,
            ability,
            item,
            evs,
            ivs,
        }
    }

    /// PS `getPokemonCompatibility` (gen 3's).
    fn compatible(sid: &str, team: &[RandomSet]) -> bool {
        const REVERSAL_USERS: &[&str] =
            &["raticate", "primeape", "hitmonlee", "furret", "yanma", "heracross", "blaziken", "medicham"];
        const FLAIL_USERS: &[&str] = &["dodrio", "farfetchd"];
        let flail_or_reversal = |m: &str| FLAIL_USERS.contains(&m) || REVERSAL_USERS.contains(&m);
        let pairs: [(&dyn Fn(&str) -> bool, &dyn Fn(&str) -> bool); 2] = [
            // Prevent Shedinja + Tyranitar
            (&|m| m == "shedinja", &|m| m == "tyranitar"),
            // Prevent Reversal/Flail users + Tyranitar
            (&flail_or_reversal, &|m| m == "tyranitar"),
        ];
        for (a, b) in pairs {
            if b(sid) && team.iter().any(|m| a(m.species_id)) {
                return false;
            }
            if a(sid) && team.iter().any(|m| b(m.species_id)) {
                return false;
            }
        }
        true
    }

    /// PS `RandomGen3Teams.randomTeam`, for a format with no team preview
    /// and `maxTeamSize` of the device's battle size.
    pub fn random_team(&mut self) -> Vec<RandomSet> {
        let mut pokemon: Vec<RandomSet> = Vec::new();

        // For Monotype
        let _type = self.sample(DEX.typechart.order);

        let mut base_formes: Vec<&'static str> = Vec::new();
        let mut type_count = [0u32; ps_dex::TYPE_COUNT];
        let mut type_weaknesses = [0u32; ps_dex::TYPE_COUNT];
        let mut type_double_weaknesses = [0u32; ps_dex::TYPE_COUNT];
        let mut td = TeamDetails::default();
        let mut num_max_level = 0u32;

        // PS `getPokemonPool`: the sets' species grouped by base species, in
        // `Object.keys(randomSets)` order; a base species goes in the pool
        // once per three formes (at most three times).
        let mut groups: Vec<(&'static str, Vec<&'static SpeciesSets>)> = Vec::new();
        for entry in GEN3_RANDOM_SETS {
            let base = species(entry.id).base_species().as_str();
            match groups.iter_mut().find(|(b, _)| *b == base) {
                Some((_, v)) => v.push(entry),
                None => groups.push((base, alloc::vec![entry])),
            }
        }
        let mut base_pool: Vec<&'static str> = Vec::new();
        for (base, v) in &groups {
            for _ in 0..v.len().div_ceil(3).min(3) {
                base_pool.push(base);
            }
        }

        let mut leads_remaining = 1;
        while !base_pool.is_empty() && pokemon.len() < self.max_team_size {
            let Some(base) = self.sample_no_replace(&mut base_pool) else { break };
            let formes = &groups.iter().find(|(b, _)| *b == base).expect("a pooled base species").1;
            let entry = formes[self.prng.random_n(formes.len() as u32) as usize];
            let s = species(entry.id);

            // Limit to one of each species (Species Clause)
            if base_formes.contains(&s.base_species().as_str()) {
                continue;
            }

            // Limit to one Wobbuffet per battle (not just per team)
            if s.name.as_str() == "Wobbuffet" && self.battle_has_wobbuffet {
                continue;
            }

            // Dynamically scale limits for different team sizes. The default and minimum value is 1.
            // (`Math.round(maxTeamSize / 6) || 1`; JS rounds halves up.)
            let limit_factor = ((2 * self.max_team_size as u32 + 6) / 12).max(1);

            // Limit two of any type
            if types_of(s).any(|t| type_count[t as usize] >= 2 * limit_factor) {
                continue;
            }

            // Limit three weak to any type, and one double weak to any type
            let mut skip = false;
            for &t in DEX.typechart.order {
                let e = effectiveness(t, s);
                // it's weak to the type
                if e > 0 && type_weaknesses[t as usize] >= 3 * limit_factor {
                    skip = true;
                    break;
                }
                if e > 1 && type_double_weaknesses[t as usize] >= limit_factor {
                    skip = true;
                    break;
                }
            }
            if skip {
                continue;
            }

            // Limit one level 100 Pokemon
            if entry.level == 100 && num_max_level >= limit_factor {
                continue;
            }

            // Check compatibility with team
            if !Self::compatible(entry.id, &pokemon) {
                continue;
            }

            // Some Pokemon shouldn't be in the lead slot
            let set = if leads_remaining > 0 {
                if NO_LEAD_POKEMON.contains(&entry.id) {
                    if pokemon.len() + leads_remaining == self.max_team_size {
                        continue;
                    }
                    let set = self.random_set(entry, &td);
                    pokemon.push(set.clone());
                    set
                } else {
                    let set = self.random_set(entry, &td);
                    pokemon.insert(0, set.clone());
                    leads_remaining -= 1;
                    set
                }
            } else {
                let set = self.random_set(entry, &td);
                pokemon.push(set.clone());
                set
            };

            // Don't bother tracking details for the last Pokemon
            if pokemon.len() == self.max_team_size {
                break;
            }

            // Now that our Pokemon has passed all checks, we can increment our counters
            base_formes.push(s.base_species().as_str());

            // Increment type counters
            for t in types_of(s) {
                type_count[t as usize] += 1;
            }

            // Increment weakness counter
            for &t in DEX.typechart.order {
                let e = effectiveness(t, s);
                if e > 0 {
                    type_weaknesses[t as usize] += 1;
                }
                if e > 1 {
                    type_double_weaknesses[t as usize] += 1;
                }
            }

            // Increment level 100 counter
            if set.level == 100 {
                num_max_level += 1;
            }

            // Update team details
            let has = |m: &str| set.moves.contains(&m);
            if set.ability == "Drizzle" || has("raindance") {
                td.rain = true;
            }
            if set.ability == "Drought" || has("sunnyday") {
                td.sun = true;
            }
            if set.ability == "Sand Stream" {
                td.sand = true;
            }
            if has("aromatherapy") || has("healbell") {
                td.status_cure = true;
            }
            if has("spikes") {
                td.spikes = 1;
            }
            if has("rapidspin") {
                td.rapid_spin = true;
            }

            // In Gen 3, Shadow Tag users can prevent each other from switching out, possibly causing and endless battle or at least causing a long stall war
            // To prevent this, we prevent more than one Wobbuffet in a single battle.
            if entry.id == "wobbuffet" {
                self.battle_has_wobbuffet = true;
            }
            if entry.id == "ditto" {
                self.battle_has_ditto = true;
            }
        }
        let _ = (td.sand, self.battle_has_ditto);
        pokemon
    }
}

#[derive(Clone, Copy)]
struct SpeciesIdOf(ps_dex::SpeciesId);

/// A drawn set in the protocol's packed form. The nickname is the species
/// entry's own name (`Deoxys-Attack`), where PS's generator names a mon
/// after its base species (`Deoxys`): the device labels and draws a mon by
/// its nickname, and a forme with its own art keeps it (the label still
/// reads `Deoxys`, `sprites_color::species_display_name`). A cosmetic forme
/// (`Unown-B`) has no art of its own and keeps the species' name.
pub fn packed(set: &RandomSet) -> PackedSet {
    let name = |s: &str| Name::from_str(s).unwrap_or_default();
    let mut p = PackedSet {
        name: name(set.nickname),
        species: name(set.species),
        item: name(set.item),
        ability: name(set.ability),
        level: u16::from(set.level),
        happiness: 255,
        shiny: set.shiny,
        gender: set.genderless_or_fixed.then_some(set.gender),
        evs: Some(StatTable { hp: set.evs[0], atk: set.evs[1], def: set.evs[2], spa: set.evs[3], spd: set.evs[4], spe: set.evs[5] }),
        ivs: Some(StatTable { hp: set.ivs[0], atk: set.ivs[1], def: set.ivs[2], spa: set.ivs[3], spd: set.ivs[4], spe: set.ivs[5] }),
        ..Default::default()
    };
    for m in &set.moves {
        let _ = p.moves.push(name(mv(m).name.as_str()));
    }
    p
}

/// Two Gen 3 random-battle teams of `count` mons from one device seed, the
/// way a PS battle draws both players' teams: one generator, seeded per
/// player (`battle.getTeam`), with p2's seed salted as the Gen 1 draw salts
/// it. Returns `[p1, p2]`.
pub fn draw_two_teams(seed: u64, count: usize) -> [PackedTeam; 2] {
    let words = |s: u64| [(s >> 48) as u16, (s >> 32) as u16, (s >> 16) as u16, s as u16];
    let count = count.clamp(1, ps_protocol::MAX_TEAM);
    let mut gen = Gen3Teams::new(words(seed), count);
    let mut out = [PackedTeam::new(), PackedTeam::new()];
    for (i, team) in out.iter_mut().enumerate() {
        if i == 1 {
            gen.set_seed(words(seed.wrapping_add(super::TEAM_SEED_SALT)));
        }
        for set in gen.random_team() {
            let _ = team.push(packed(&set));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use alloc::string::{String, ToString};

    /// A drawn pair of teams as the reference script prints it.
    fn as_json(seed: u64, count: usize) -> serde_json::Value {
        let words = |s: u64| [(s >> 48) as u16, (s >> 32) as u16, (s >> 16) as u16, s as u16];
        let mut gen = Gen3Teams::new(words(seed), count);
        let mut teams = Vec::new();
        for i in 0..2 {
            if i == 1 {
                gen.set_seed(words(seed.wrapping_add(crate::randbat::TEAM_SEED_SALT)));
            }
            let team: Vec<serde_json::Value> = gen
                .random_team()
                .iter()
                .map(|s| {
                    serde_json::json!({
                        "species": s.species, "item": s.item, "ability": s.ability,
                        "moves": s.moves, "level": s.level, "shiny": s.shiny,
                        "gender": match (s.genderless_or_fixed, s.gender) { (false, _) => "", (_, Gender::M) => "M", (_, Gender::F) => "F", (_, Gender::N) => "N" },
                        "evs": s.evs, "ivs": s.ivs,
                    })
                })
                .collect();
            teams.push(serde_json::Value::Array(team));
        }
        serde_json::json!({ "seed": seed.to_string(), "count": count, "teams": teams })
    }

    /// The device seed the reference script derives from its index.
    fn seed_of(i: u64) -> u64 {
        i.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// PS's `Teams.getGenerator('gen3randombattle', seed)` for index 1 (3
    /// mons a side), from `.scratch/gen3teams.js` (see `same_teams_as_ps`).
    #[test]
    fn a_seed_draws_ps_teams() {
        let v = as_json(seed_of(1), 3);
        let names = |t: usize| -> Vec<String> {
            v["teams"][t].as_array().unwrap().iter().map(|s| s["species"].as_str().unwrap().to_string()).collect()
        };
        assert_eq!(names(0), ["Girafarig", "Ledian", "Nidoking"]);
        let ledian = &v["teams"][0][1];
        assert_eq!(ledian["item"], "Leftovers");
        assert_eq!(ledian["ability"], "Early Bird");
        assert_eq!(ledian["moves"], serde_json::json!(["psybeam", "rest", "toxic", "hiddenpowerfire"]));
        // Hidden Power Fire's IVs, and no Attack (no physical move).
        assert_eq!(ledian["ivs"], serde_json::json!([31, 2, 31, 30, 31, 30]));
        assert_eq!(ledian["evs"], serde_json::json!([85, 0, 85, 85, 85, 85]));
        assert_eq!(v["teams"][0][2]["item"], "Choice Band");
        assert_eq!(v["teams"][0][2]["gender"], "M");
    }

    /// Every line of a reference file written by `node .scratch/gen3teams.js
    /// FIRST N` (PS's own generator, same seeds), when
    /// `GEN3_TEAMS_REFERENCE` names one.
    #[test]
    fn same_teams_as_ps() {
        let Ok(path) = std::env::var("GEN3_TEAMS_REFERENCE") else { return };
        let text = std::fs::read_to_string(&path).unwrap();
        let mut n = 0;
        for line in text.lines() {
            let want: serde_json::Value = serde_json::from_str(line).unwrap();
            let seed: u64 = want["seed"].as_str().unwrap().parse().unwrap();
            let count = want["count"].as_u64().unwrap() as usize;
            assert_eq!(as_json(seed, count), want, "seed {seed}, {count} a side");
            n += 1;
        }
        std::println!("{n} team pairs identical to PS's generator");
    }

    /// Every item and ability a drawn team can carry is one `ps_sim` ports.
    #[test]
    fn drawn_items_and_abilities_are_ported() {
        let ported = ps_sim::mods::ported(3);
        let is_ported = |kind: &str, name: &str| {
            let id = ps_dex::text::IdBuf::from_name(name);
            ported.iter().any(|e| e.split_once(':') == Some((kind, id.as_str())))
        };
        for seed in 0..300u64 {
            for team in draw_two_teams(seed_of(seed), 6) {
                assert_eq!(team.len(), 6, "seed {seed}");
                for set in team.iter() {
                    assert!(is_ported("item", set.item.as_str()), "{}", set.item.as_str());
                    assert!(is_ported("ability", set.ability.as_str()), "{}", set.ability.as_str());
                    assert!(!set.moves.is_empty());
                }
            }
        }
    }
}
