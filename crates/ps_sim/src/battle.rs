//! The server's control flow: from `>start` to `|win|` / `|tie|`.
//!
//! The PS constructor and `setPlayer` build the battle and its teams;
//! `start` writes the header and queues the `start` action; `turnLoop` runs
//! the queue one `runAction` at a time, `endTurn` closes the turn and asks
//! for the next choices. Faints are queued by `faint`, announced by
//! `faintMessages` and replaced through `checkFainted` and a `switch`
//! request. The battle ends in `win` (a tie is `win` with no side) from
//! `checkWin`, Gen 1's no-progress check, the turn limit, or `>forcewin` /
//! `>forcetie`.
//!
//! The move pipeline is not here: `runMove` panics `unported` until P1-8
//! and P1-9 bring the gen scripts. Unit tests install a stub through
//! [`Battle::move_hook`] (`cfg(test)` / the `test-override` feature).
//!
//! PS: sim/battle.ts (constructor, `setPlayer`, `getTeam`, `start`,
//! `turnLoop`, `runAction`, `endTurn`, `maybeTriggerEndlessBattleClause`,
//! `checkFainted`, `faintMessages`, `checkWin`, `win`, `tie`, `forceWin`,
//! `choose`, `commitChoices`, `undoChoice`, `allChoicesDone`,
//! `getRandomTarget`, `getTarget`, `validTargetLoc`, `directDamage`,
//! `faint`); sim/battle-actions.ts `switchIn`, `dragIn`;
//! data/mods/gen4/scripts.ts `runSwitch` (gens 1 and 3 inherit it);
//! sim/pokemon.ts (constructor, `clearVolatile`, `setSpecies`,
//! `copyVolatileFrom`, `faint`, `damage`, `getLocOf`, `getAtLoc`);
//! sim/teams.ts `unpack` (name resolution); data/rulesets.ts.

use ps_dex::{AbilityId, Dex, ItemId, MoveId, SpeciesId, Type};
use ps_protocol::{
    Arg, ArgValue, Command, FormatSpec, GameType, Kw, Msg, MoveNum, Name, PackedSet, PackedTeam, Pokeball, RuleId,
    SideId, Text, TextArg, TextId, Word,
};

use crate::effect::{Callback, EffectId, EventArgs, EventId, Prefix, Rule};
use crate::modify::Trunc;
use crate::prng::Seed;
use crate::queue::{Action, ActionKind, ActionMove};
use crate::relay::{Relay, TypeList};
use crate::request::RequestState;
use crate::state::{Battle, EffectState, FaintData, Holder, MonRef, MoveSlot, StateRef, SwitchFlag, Trapped};
use crate::stats::Stat;

/// The test-only move pipeline: `runMove(move, pokemon, targetLoc)`.
pub type MoveHook = fn(&mut Battle, ActionMove, MonRef, i8);

/// Why `>player` failed (PS throws).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TeamError {
    /// `Unidentified species`.
    UnknownSpecies,
    /// A move, item, ability or nature the gen's dex lacks.
    UnknownName,
    /// `Set NAME has no moves`.
    NoMoves,
    /// `Player pN already has a team!`
    AlreadyHasTeam,
    /// More than six Pokemon, or none.
    TeamSize,
}

/// What `switchIn` returns: PS's `false`, `true` or `'pursuitfaint'`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwitchResult {
    Failed,
    Done,
    PursuitFaint,
}

fn side_id(side: u8) -> SideId {
    if side == 0 {
        SideId::P1
    } else {
        SideId::P2
    }
}

fn rule_of(r: RuleId) -> Rule {
    match r {
        RuleId::HpPercentageMod => Rule::HpPercentageMod,
        RuleId::CancelMod => Rule::CancelMod,
        RuleId::DesyncClauseMod => Rule::DesyncClauseMod,
        RuleId::SleepClauseMod => Rule::SleepClauseMod,
        RuleId::FreezeClauseMod => Rule::FreezeClauseMod,
    }
}

/// The format's rule table keys in PS's order, as far as they bear on the
/// battle: the format's own `ruleset` (`gen1customgame` adds Desync Clause
/// Mod to the two both have), then the custom rules. `maxteamsize`,
/// `maxmovecount`, `maxlevel` and `defaultlevel` only validate teams.
fn rule_table(spec: &FormatSpec) -> ([RuleId; 8], usize) {
    let mut out = [RuleId::HpPercentageMod; 8];
    let mut n = 0;
    let mut push = |r: RuleId| {
        if !out[..n].contains(&r) {
            out[n] = r;
            n += 1;
        }
    };
    push(RuleId::HpPercentageMod);
    push(RuleId::CancelMod);
    if spec.base.gen() == 1 {
        push(RuleId::DesyncClauseMod);
    }
    for &r in spec.custom_rules.iter() {
        push(r);
    }
    (out, n)
}

/// PS `toID` into a fixed buffer.
fn to_id(text: &str) -> ps_protocol::BoundedStr<40> {
    ps_protocol::to_id::<40>(text).unwrap_or_default()
}

/// A set after `Teams.unpack`'s name resolution: dex ids, and PS's
/// `undefined` as `None`.
#[derive(Clone, Copy, Debug)]
struct UnpackedSet {
    /// The nickname as packed (empty when unnamed).
    name: Name,
    /// The species text (`set.species`): the dex name when it resolves.
    species_text: Name,
    species: SpeciesId,
    /// The cosmetic forme ([`ps_protocol::Details::forme`]), 0 for none.
    forme: u8,
    item: ItemId,
    ability: AbilityId,
    moves: [MoveId; 4],
    move_count: usize,
    nature: Option<u8>,
    evs: Option<[u16; 6]>,
    ivs: Option<[u16; 6]>,
    gender: Option<ps_protocol::Gender>,
    shiny: bool,
    level: u16,
    happiness: u8,
    hp_type: Option<Type>,
    pokeball: Pokeball,
}

impl Battle {
    // ---- construction -------------------------------------------------

    /// PS `new Battle({formatid, seed})`, as `>start` builds it. `clock`
    /// is the wall clock for `|t:|`, in seconds. Gen 1 turns on the
    /// `Cartridge Trunc` rule (design section 3): PS's bit-width `trunc`
    /// in place of the custom game's `Math.trunc`.
    // PS: sim/battle.ts constructor
    pub fn from_start(spec: FormatSpec, seed: Seed, clock: u32) -> Battle {
        let dex: &'static Dex = match spec.base.gen() {
            1 => &ps_dex::GEN1,
            _ => &ps_dex::GEN3,
        };
        let registry = crate::mods::registry_for(dex);
        let cartridge = spec.base.gen() == 1;
        let trunc = if cartridge { Trunc::Cartridge } else { Trunc::Math };
        let mut b = Battle::new(dex, registry, seed, trunc);
        b.clock = clock;
        b.format_spec = spec;
        b.add(Msg::Timestamp(clock));
        let (rules, n) = rule_table(&spec);
        for &r in &rules[..n] {
            b.rules |= 1 << rule_of(r) as u8;
        }
        if cartridge {
            b.rules |= 1 << Rule::CartridgeTrunc as u8;
        }
        // Both custom games are `debug` formats.
        b.debug_mode = true;
        b.report_exact_hp = true;
        b.add(Msg::GameType(GameType::Singles));
        // Rules with event handlers live on the field as pseudo-weathers.
        for &r in &rules[..n] {
            let id = EffectId::Rule(rule_of(r));
            if b.registry.get(id).is_some() {
                b.add_pseudo_weather(id, None, EffectId::None);
            }
        }
        b
    }

    /// `ruleTable.has(rule)`.
    pub fn has_rule(&self, r: Rule) -> bool {
        self.rules & (1 << r as u8) != 0
    }

    /// One command, as `BattleStream` routes it (`>start` is
    /// [`Battle::from_start`]). Call [`Battle::send_updates`] after.
    // PS: sim/battle-stream.ts _writeLine
    pub fn step(&mut self, cmd: &Command) -> Result<(), TeamError> {
        match cmd {
            Command::Start { .. } => panic!("the battle has started: >start makes a new Battle"),
            Command::Player { side, name, avatar, rating, team } => {
                let team = if team.is_empty() { None } else { Some(team) };
                self.set_player(*side, name, avatar, rating, team)?;
            }
            Command::Choose { side, choice } => {
                let s = side.index() as u8;
                if *choice == ps_protocol::Choice::Undo {
                    self.undo_choice(s);
                } else {
                    self.choose(s, *choice);
                }
            }
            Command::ForceWin(side) => {
                self.win(Some(side.index() as u8));
            }
            Command::ForceTie => {
                self.win(None);
            }
        }
        Ok(())
    }

    /// `Teams.unpack`'s name lookups for one set, against the gen's dex.
    // PS: sim/teams.ts unpack, unpackName
    fn unpack_set(&self, p: &PackedSet) -> Result<UnpackedSet, TeamError> {
        let dex = self.dex;
        let species_text = if p.species.is_empty() { p.name } else { p.species };
        // A cosmetic forme (`Unown-T`) is its own species object in PS; here
        // it is the base species and the forme's index.
        let id = to_id(&species_text);
        let (species, forme) = match dex.species_id(&id) {
            Some(s) => (s, 0),
            None => dex.cosmetic_forme_id(&id).ok_or(TeamError::UnknownSpecies)?,
        };
        let species_name = match dex.cosmetic_forme_name(species, forme) {
            Some(name) => name.as_str(),
            None => dex.species_def(species).name.as_str(),
        };
        let species_text = if p.species.is_empty() { p.name } else { Name::from_str(species_name).unwrap_or(p.species) };
        let item = if p.item.is_empty() {
            ItemId::NONE
        } else {
            dex.item_id(&to_id(&p.item)).ok_or(TeamError::UnknownName)?
        };
        let ability = match p.ability.as_str() {
            "" => AbilityId::NONE,
            "0" | "1" | "H" | "S" => {
                let slot = if p.ability.as_str() == "1" { 1 } else { 0 };
                match (p.ability.as_str(), dex.species_def(species).ability(slot)) {
                    ("0" | "1", Some(name)) => dex.ability_id(&to_id(name)).ok_or(TeamError::UnknownName)?,
                    _ => return Err(TeamError::UnknownName),
                }
            }
            name => dex.ability_id(&to_id(name)).ok_or(TeamError::UnknownName)?,
        };
        let mut moves = [MoveId::NONE; 4];
        let mut move_count = 0;
        for m in p.moves.iter() {
            if m.is_empty() {
                continue;
            }
            moves[move_count] = dex.move_id(&to_id(m)).ok_or(TeamError::UnknownName)?;
            move_count += 1;
        }
        let nature = if p.nature.is_empty() {
            None
        } else {
            Some(crate::stats::nature_from_id(&to_id(&p.nature)).ok_or(TeamError::UnknownName)?)
        };
        let (hp_type, pokeball) = match &p.misc {
            Some(misc) => {
                let hp_type = if misc.hp_type.is_empty() {
                    None
                } else {
                    Some(Type::from_name(&misc.hp_type).ok_or(TeamError::UnknownName)?)
                };
                let ball = if misc.pokeball.is_empty() {
                    Pokeball::Default
                } else {
                    let i = dex.item_id(&to_id(&misc.pokeball)).ok_or(TeamError::UnknownName)?;
                    Pokeball::Item(ps_protocol::ItemNum(i.0))
                };
                (hp_type, ball)
            }
            None => (None, Pokeball::Default),
        };
        Ok(UnpackedSet {
            name: p.name,
            species_text,
            species,
            forme,
            item,
            ability,
            moves,
            move_count,
            nature,
            evs: p.evs.map(|t| t.to_array()),
            ivs: p.ivs.map(|t| t.to_array()),
            gender: p.gender,
            shiny: p.shiny,
            level: p.level,
            happiness: p.happiness,
            hp_type,
            pokeball,
        })
    }

    /// PS `new Pokemon(set, side)` into team slot `m`.
    // PS: sim/pokemon.ts constructor
    fn new_pokemon(&mut self, m: MonRef, set: &UnpackedSet) -> Result<(), TeamError> {
        let dex = self.dex;
        let species_def = dex.species_def(set.species);
        let mut mon = crate::state::Pokemon::EMPTY;
        mon.exists = true;
        mon.base_species = set.species;
        mon.species = set.species;
        mon.forme = set.forme;
        // `if (set.name === set.species || !set.name) set.name =
        // baseSpecies.baseSpecies`, then the first 20 characters.
        let base_name = species_def.base_species().as_str();
        let raw = if set.name.is_empty() || set.name == set.species_text { base_name } else { set.name.as_str() };
        let end = raw.char_indices().nth(20).map_or(raw.len(), |(i, _)| i);
        mon.name = Name::from_str(&raw[..end]).unwrap_or_default();
        mon.level = if set.level == 0 { 100 } else { set.level.clamp(1, 9999) };
        mon.gender = match set.gender {
            Some(ps_protocol::Gender::M) => ps_dex::Gender::Male,
            Some(ps_protocol::Gender::F) => ps_dex::Gender::Female,
            Some(ps_protocol::Gender::N) => ps_dex::Gender::Neutral,
            None => match species_def.gender {
                ps_dex::Gender::Either => *self.prng.sample(&[ps_dex::Gender::Male, ps_dex::Gender::Female]),
                g => g,
            },
        };
        mon.happiness = set.happiness;
        mon.pokeball = set.pokeball;

        let mut hp_type = set.hp_type;
        let mut n = 0;
        for &id in &set.moves[..set.move_count] {
            let mut id = id;
            let def = dex.move_def(id);
            if dex.move_ps_id(id) != id && def.type_ != Type::Normal {
                if hp_type.is_none() {
                    hp_type = Some(def.type_);
                }
                id = dex.move_ps_id(id);
            }
            let def = dex.move_def(id);
            let pp_ups = if def.bools.has("noPPBoosts") || def.name.as_str() == "Trump Card" { 0 } else { 3 };
            let pp = crate::stats::calculate_pp(self.gen, def, pp_ups);
            mon.base_move_slots[n] = MoveSlot { id, pp, maxpp: pp, disabled: 0, used: false, virtual_: false };
            n += 1;
        }
        if n == 0 {
            return Err(TeamError::NoMoves);
        }
        mon.position = m.index;
        mon.status = ps_dex::ConditionId::NONE;
        mon.shiny = set.shiny;

        let evs = set.evs.unwrap_or([0; 6]);
        let ivs = set.ivs.unwrap_or([31; 6]);
        for i in 0..6 {
            mon.evs[i] = evs[i].min(255) as u8;
            let mut iv = ivs[i].min(31) as u8;
            if self.gen <= 2 {
                iv &= 30;
            }
            mon.ivs[i] = iv;
        }
        mon.nature = set.nature;
        let (hp_ty, hp_power) = crate::stats::hidden_power(self.gen, mon.ivs);
        mon.hp_type = hp_type.unwrap_or(hp_ty);
        mon.hp_power = hp_power;
        mon.base_hp_type = mon.hp_type;
        mon.base_hp_power = mon.hp_power;

        mon.base_ability = set.ability;
        mon.ability = set.ability;
        mon.item = set.item;
        mon.types = TypeList(species_def.types);
        *self.mon_mut(m) = mon;
        // `initEffectState({})` for the status, `{id, target: this}` for
        // the ability and item: an inactive target takes no effectOrder.
        let st = self.init_effect_state(false, Holder::None);
        self.mon_mut(m).status_state = st;
        let st = self.init_effect_state(!set.ability.is_none(), Holder::Mon(m));
        self.mon_mut(m).ability_state = st;
        let st = self.init_effect_state(!set.item.is_none(), Holder::Mon(m));
        self.mon_mut(m).item_state = st;

        self.mon_mut(m).maxhp = 0;
        self.mon_mut(m).base_maxhp = 0;
        self.mon_mut(m).hp = 0;
        self.clear_volatile(m, true);
        let maxhp = self.mon(m).maxhp;
        self.mon_mut(m).hp = maxhp;
        Ok(())
    }

    /// PS `setPlayer(slot, options)`. `team` is `options.team` (`None`
    /// when absent: editing a player that exists).
    // PS: sim/battle.ts setPlayer, getTeam; sim/side.ts constructor
    pub fn set_player(
        &mut self,
        side: SideId,
        name: &str,
        avatar: &str,
        rating: &str,
        team: Option<&PackedTeam>,
    ) -> Result<(), TeamError> {
        let s = side.index() as u8;
        let mut did_something = true;
        if !self.sides[s as usize].exists {
            let team = team.ok_or(TeamError::TeamSize)?;
            if team.is_empty() || team.len() > crate::state::TEAM_CAP {
                return Err(TeamError::TeamSize);
            }
            let mut sets = [None; crate::state::TEAM_CAP];
            for (i, p) in team.iter().enumerate() {
                sets[i] = Some(self.unpack_set(p)?);
            }
            let side_name = if name.is_empty() {
                let mut n = Name::new();
                let _ = core::fmt::Write::write_fmt(&mut n, format_args!("Player {}", s + 1));
                n
            } else {
                Name::from_str(name).unwrap_or_default()
            };
            {
                let sd = &mut self.sides[s as usize];
                sd.exists = true;
                sd.name = side_name;
                sd.avatar = Name::from_str(avatar).unwrap_or_default();
                sd.team_size = team.len() as u8;
            }
            for (i, set) in sets.iter().enumerate() {
                if let Some(set) = set {
                    self.new_pokemon(MonRef::new(s, i as u8), set)?;
                }
            }
            self.sides[s as usize].pokemon_left = team.len() as u8;
        } else {
            did_something = false;
            let sd = &mut self.sides[s as usize];
            if !name.is_empty() && sd.name.as_str() != name {
                sd.name = Name::from_str(name).unwrap_or_default();
                did_something = true;
            }
            if !avatar.is_empty() && sd.avatar.as_str() != avatar {
                sd.avatar = Name::from_str(avatar).unwrap_or_default();
                did_something = true;
            }
            if team.is_some() {
                return Err(TeamError::AlreadyHasTeam);
            }
        }
        if !did_something {
            return Ok(());
        }
        self.sides[s as usize].rating = Name::from_str(rating).unwrap_or_default();
        self.add(Msg::Player(side));
        if self.sides.iter().all(|sd| sd.exists) && !self.started {
            self.start();
        }
        Ok(())
    }

    /// PS `start()`: the header, the rules' `onBegin`, the custom-rules
    /// box, then the `start` action and the first `turnLoop`.
    // PS: sim/battle.ts start
    pub fn start(&mut self) {
        assert!(!self.started, "Battle already started");
        self.started = true;
        self.add(Msg::Gen(self.gen));
        self.add(Msg::Tier(self.format_spec.base));
        // No `rated`; the custom games have no `onBegin` of their own.
        let (rules, n) = rule_table(&self.format_spec);
        for &r in &rules[..n] {
            self.rule_on_begin(r);
        }
        if self.debug_mode {
            self.check_ev_balance();
        }
        let custom = self.format_spec.custom_rules;
        match custom.len() {
            0 => {}
            1 => self.add(Msg::Raw(Text::with1(TextId::CustomRules1, TextArg::rule(custom[0])))),
            2 => self.add(Msg::Raw(Text::with2(TextId::CustomRules2, TextArg::rule(custom[0]), TextArg::rule(custom[1])))),
            k => panic!("{k} custom rules: no infobox template"),
        }
        // `runPickTeam`: no team preview in our formats.
        self.add_choice(Action::new(ActionKind::Start));
        self.mid_turn = true;
        if self.request_state == RequestState::None {
            self.turn_loop();
        }
    }

    /// A rule's `onBegin`.
    // PS: data/rulesets.ts (onBegin of each rule)
    fn rule_on_begin(&mut self, r: RuleId) {
        match r {
            RuleId::HpPercentageMod => {
                self.add(Msg::Rule(r));
                self.report_percentages = true;
            }
            RuleId::CancelMod => self.support_cancel = true,
            RuleId::DesyncClauseMod | RuleId::SleepClauseMod | RuleId::FreezeClauseMod => self.add(Msg::Rule(r)),
        }
    }

    /// PS `checkEVBalance()`.
    // PS: sim/battle.ts checkEVBalance
    fn check_ev_balance(&mut self) {
        let mut limited: Option<bool> = None;
        for s in 0..2 {
            let side = &self.sides[s];
            let side_limited = !side.pokemon().iter().any(|&slot| {
                side.mons[slot as usize].evs.iter().map(|&e| u32::from(e)).sum::<u32>() > 510
            });
            match limited {
                None => limited = Some(side_limited),
                Some(l) if l != side_limited => self.add(Msg::BigError(Text::new(TextId::EvLimitWarning))),
                Some(_) => {}
            }
        }
    }

    // ---- Pokemon state --------------------------------------------------

    /// PS `pokemon.clearVolatile(includeSwitchFlags)`.
    // PS: sim/pokemon.ts clearVolatile
    pub fn clear_volatile(&mut self, m: MonRef, include_switch_flags: bool) {
        let mon = self.mon_mut(m);
        mon.boosts = [0; 7];
        mon.move_slots = mon.base_move_slots;
        mon.transformed = false;
        mon.ability = mon.base_ability;
        mon.hp_type = mon.base_hp_type;
        mon.hp_power = mon.base_hp_power;
        // No linked volatiles in gens 1-3; Eternatus is not in them.
        mon.volatiles.clear();
        if include_switch_flags {
            mon.switch_flag = SwitchFlag::No;
            mon.force_switch_flag = false;
        }
        mon.last_move = MoveId::NONE;
        mon.last_move_used = MoveId::NONE;
        mon.move_this_turn = MoveId::NONE;
        mon.move_last_turn_result = None;
        mon.move_this_turn_result = None;
        mon.last_damage = 0;
        mon.attacked_by = [None; 4];
        mon.hurt_this_turn = None;
        mon.newly_switched = true;
        mon.being_called_back = false;
        // `delete abilityState.started` / `itemState.started`: no gen 1-3
        // ability or item keeps `started`.
        let species = mon.base_species;
        self.set_species(m, species, false);
    }

    /// PS `pokemon.setSpecies(species, source, isTransform)`.
    // PS: sim/pokemon.ts setSpecies
    pub fn set_species(&mut self, m: MonRef, species: SpeciesId, is_transform: bool) {
        // `runEvent('ModifySpecies', ...)`: no gen 1-3 effect answers it
        // (the species relay arrives with the first one).
        let r = self.run_event(EventId::ModifySpecies, Holder::Mon(m), None, self.frame.effect, Some(Relay::TRUE));
        debug_assert_eq!(r, Relay::TRUE, "ModifySpecies handler: species relay not ported");
        let def = self.dex.species_def(species);
        let mut stats = self.spread_modify(def.base_stats, self.mon(m));
        if let Some(hp) = def.max_hp() {
            stats[0] = hp;
        }
        let gen = self.gen;
        let mon = self.mon_mut(m);
        mon.species = species;
        // `setType(species.types, true)`.
        mon.types = TypeList(def.types);
        if mon.maxhp == 0 {
            mon.base_maxhp = stats[0];
            mon.maxhp = stats[0];
            mon.hp = stats[0];
        }
        let five = [stats[1], stats[2], stats[3], stats[4], stats[5]];
        if !is_transform {
            mon.base_stored_stats = five;
        }
        mon.stored_stats = five;
        if gen == 1 {
            mon.modified_stats = five;
        }
        if gen <= 1 {
            let st = self.mon(m).status;
            let key = if st.is_none() { "" } else { self.dex.condition_def(st).key.as_str() };
            if key == "par" {
                self.modify_stat_gen1(m, Stat::Spe, 1, 4);
            }
            if key == "brn" {
                self.modify_stat_gen1(m, Stat::Atk, 1, 2);
            }
        }
        let spe = self.mon(m).stored_stats[4];
        self.mon_mut(m).speed = i32::from(spe);
    }

    /// Gen 1's `pokemon.modifyStat(statName, modifier)`, the modifier as a
    /// ratio.
    // PS: data/mods/gen1/scripts.ts modifyStat
    pub fn modify_stat_gen1(&mut self, m: MonRef, stat: Stat, num: u32, den: u32) {
        let s = stat as usize;
        let v = u32::from(self.mon(m).modified_stats[s]) * num / den;
        self.mon_mut(m).modified_stats[s] = v.max(1) as u16;
    }

    /// PS `pokemon.copyVolatileFrom(pokemon, switchCause)` (Baton Pass's
    /// `copyvolatile`; Shed Tail is gen 9).
    // PS: sim/pokemon.ts copyVolatileFrom
    pub fn copy_volatile_from(&mut self, to: MonRef, from: MonRef) {
        self.clear_volatile(to, true);
        let boosts = self.mon(from).boosts;
        self.mon_mut(to).boosts = boosts;
        let vols = self.mon(from).volatiles;
        for (id, st) in vols.iter() {
            if let Some(c) = id.condition() {
                if self.dex.condition_def(c).no_copy {
                    continue;
                }
            }
            // `initEffectState({...state, target: this})`: the incoming
            // Pokemon is not active yet, so effectOrder 0.
            let fresh = self.init_effect_state(true, Holder::Mon(to));
            let copy = EffectState { serial: fresh.serial, effect_order: fresh.effect_order, target: Holder::Mon(to), ..*st };
            self.mon_mut(to).volatiles.insert(id, copy);
        }
        self.clear_volatile(from, true);
        // `singleEvent('Copy', ...)` per volatile: no gen 1-3 condition
        // has `onCopy`.
    }

    /// PS `pokemon.faint(source, effect)`: queues the faint; returns the
    /// HP it took.
    // PS: sim/pokemon.ts faint
    pub fn faint(&mut self, m: MonRef, source: Option<MonRef>, effect: EffectId) -> u16 {
        let mon = self.mon(m);
        if mon.fainted || mon.faint_queued {
            return 0;
        }
        let d = mon.hp;
        let mon = self.mon_mut(m);
        mon.hp = 0;
        mon.switch_flag = SwitchFlag::No;
        mon.faint_queued = true;
        let slot = self.faint_queue.iter_mut().find(|f| f.is_none()).expect("faint queue full");
        *slot = Some(FaintData { target: m, source, effect });
        d
    }

    /// PS `pokemon.damage(d, source, effect)`.
    // PS: sim/pokemon.ts damage
    pub fn damage_mon(&mut self, m: MonRef, d: i32, source: Option<MonRef>, effect: EffectId) -> i32 {
        if self.mon(m).hp == 0 || d <= 0 {
            return 0;
        }
        let mut d = self.trunc.int(i64::from(d), 0) as i32;
        let hp = i32::from(self.mon(m).hp) - d;
        if hp <= 0 {
            d += hp;
            // PS's `hp` is negative until `faint` zeroes it.
            self.mon_mut(m).hp = 0;
            self.faint(m, source, effect);
        } else {
            self.mon_mut(m).hp = hp as u16;
        }
        d
    }

    /// PS `directDamage(damage, target, source, effect)`.
    // PS: sim/battle.ts directDamage
    pub fn direct_damage(
        &mut self,
        damage: i32,
        target: Option<MonRef>,
        source: Option<MonRef>,
        effect: EffectId,
    ) -> i32 {
        let target = target.or(self.event.target.mon());
        let source = source.or(self.event.source);
        let effect = if effect.is_none() { self.frame.effect } else { effect };
        let Some(target) = target else { return 0 };
        if self.mon(target).hp == 0 || damage == 0 {
            return 0;
        }
        let damage = damage.max(1);
        // `effect.id`.
        let key = match effect {
            EffectId::Condition(c) => to_id(self.dex.condition_def(c).key.as_str()),
            EffectId::Move(mv) => to_id(self.dex.move_def(mv).name.as_str()),
            _ => to_id(""),
        };
        let key = key.as_str();
        if self.gen <= 1 && matches!(key, "confusion" | "jumpkick" | "highjumpkick") {
            self.last_damage = damage as u16;
            if self.has_volatile_key(target, "substitute") {
                panic!("unported: gen 1 directDamage against a Substitute (P1-8)");
            }
        }
        let damage = self.damage_mon(target, damage, source, effect);
        let mon = self.ident(target);
        let health = self.health(target);
        let kw = match key {
            "strugglerecoil" => Kw::from(Arg::new(ArgValue::Word(Word::StruggleRecoil))),
            "confusion" => Kw::from(Arg::new(ArgValue::Word(Word::Confusion))),
            _ => Kw::NONE,
        };
        self.add(Msg::Damage { mon, health, kw });
        if self.mon(target).fainted {
            self.faint(target, None, EffectId::None);
        }
        damage
    }

    // ---- targets --------------------------------------------------------

    /// The dex `target` of an action's move (`None`: no move, PS's
    /// `undefined`).
    fn move_target(&self, mv: ActionMove) -> Option<ps_dex::MoveTarget> {
        mv.id().map(|m| self.dex.move_def(m).target)
    }

    /// PS `pokemon.getLocOf(target)`: 1-based, negative on the same side.
    // PS: sim/pokemon.ts getLocOf
    pub fn get_loc_of(&self, source: MonRef, target: MonRef) -> i8 {
        let position = self.mon(target).position as i8 + 1;
        if source.side == target.side {
            -position
        } else {
            position
        }
    }

    /// PS `pokemon.getAtLoc(targetLoc)`.
    // PS: sim/pokemon.ts getAtLoc
    pub fn get_at_loc(&self, source: MonRef, target_loc: i8) -> Option<MonRef> {
        let side = if target_loc < 0 { source.side } else { 1 - source.side };
        let loc = target_loc.unsigned_abs() as usize;
        if loc == 0 || loc > self.sides[side as usize].active.len() {
            return None;
        }
        self.active_at(side, loc as u8 - 1)
    }

    /// PS `validTargetLoc(targetLoc, source, targetType)`.
    // PS: sim/battle.ts validTargetLoc
    pub fn valid_target_loc(&self, target_loc: i8, source: MonRef, target_type: Option<ps_dex::MoveTarget>) -> bool {
        use ps_dex::MoveTarget as T;
        if target_loc == 0 {
            return true;
        }
        let num_slots: i8 = 1;
        let source_loc = self.get_loc_of(source, source);
        if target_loc.abs() > num_slots {
            return false;
        }
        let is_self = source_loc == target_loc;
        let is_foe = target_loc > 0;
        let across = -(num_slots + 1 - target_loc);
        let is_adjacent = if target_loc > 0 {
            (across - source_loc).abs() <= 1
        } else {
            (target_loc - source_loc).abs() == 1
        };
        match target_type {
            Some(T::RandomNormal | T::Scripted | T::Normal) => is_adjacent,
            Some(T::AdjacentAlly) => is_adjacent && !is_foe,
            Some(T::AdjacentAllyOrSelf) => (is_adjacent && !is_foe) || is_self,
            Some(T::AdjacentFoe) => is_adjacent && is_foe,
            Some(T::Any) => !is_self,
            _ => false,
        }
    }

    /// PS `getRandomTarget(pokemon, move)` in singles.
    // PS: sim/battle.ts getRandomTarget
    pub fn get_random_target(&mut self, pokemon: MonRef, mv: ActionMove) -> Option<MonRef> {
        use ps_dex::MoveTarget as T;
        match self.move_target(mv) {
            Some(T::Self_ | T::All | T::AllySide | T::AllyTeam | T::AdjacentAllyOrSelf) => Some(pokemon),
            Some(T::AdjacentAlly) => None,
            _ => self.active_at(1 - pokemon.side, 0),
        }
    }

    /// PS `getTarget(pokemon, move, targetLoc, originalTarget)`.
    // PS: sim/battle.ts getTarget
    pub fn get_target(
        &mut self,
        pokemon: MonRef,
        mv: ActionMove,
        target_loc: i8,
        original_target: Option<MonRef>,
    ) -> Option<MonRef> {
        use ps_dex::MoveTarget as T;
        let def = mv.id().map(|m| self.dex.move_def(m));
        // No Stalwart or Propeller Tail in gens 1-3.
        let tracks_target = def.is_some_and(|d| d.bools.has("tracksTarget"));
        if tracks_target {
            if let Some(o) = original_target {
                if self.mon(o).is_active {
                    return Some(o);
                }
            }
        }
        if def.is_some_and(|d| d.bools.has("smartTarget")) {
            let cur = self.get_at_loc(pokemon, target_loc);
            return match cur {
                Some(c) if !self.mon(c).fainted => Some(c),
                _ => self.get_random_target(pokemon, mv),
            };
        }
        let self_loc = self.get_loc_of(pokemon, pokemon);
        let target_type = self.move_target(mv);
        if matches!(target_type, Some(T::AdjacentAlly | T::Any | T::Normal))
            && target_loc == self_loc
            && !self.has_volatile_key(pokemon, "twoturnmove")
            && !self.has_volatile_key(pokemon, "iceball")
            && !self.has_volatile_key(pokemon, "rollout")
        {
            return if def.is_some_and(|d| d.flags.has("futuremove")) { Some(pokemon) } else { None };
        }
        if target_type != Some(T::RandomNormal) && self.valid_target_loc(target_loc, pokemon, target_type) {
            let target = self.get_at_loc(pokemon, target_loc);
            if let Some(t) = target {
                if self.mon(t).fainted && t.side == pokemon.side {
                    if target_type == Some(T::AdjacentAllyOrSelf) && self.gen != 5 {
                        return Some(pokemon);
                    }
                    return Some(t);
                }
                if !self.mon(t).fainted {
                    return Some(t);
                }
            }
        }
        self.get_random_target(pokemon, mv)
    }

    // ---- the turn loop --------------------------------------------------

    /// PS `turnLoop()`.
    // PS: sim/battle.ts turnLoop
    pub fn turn_loop(&mut self) {
        self.add(Msg::Spacer);
        self.add(Msg::Timestamp(self.clock));
        if self.request_state != RequestState::None {
            self.request_state = RequestState::None;
        }
        if !self.mid_turn {
            self.insert_choice(Action::new(ActionKind::BeforeTurn), false);
            self.add_choice(Action::new(ActionKind::Residual));
            self.mid_turn = true;
        }
        while let Some(action) = self.queue.shift() {
            self.run_action(action);
            if self.request_state != RequestState::None || self.ended {
                return;
            }
        }
        self.end_turn();
        self.mid_turn = false;
        self.queue.clear();
    }

    /// PS `actions.runMove(move, pokemon, targetLoc, options)`: the gen's
    /// move pipeline. Not ported yet (P1-8, P1-9).
    // PS: sim/battle-actions.ts runMove
    pub fn run_move(&mut self, mv: ActionMove, pokemon: MonRef, target_loc: i8) {
        #[cfg(any(test, feature = "test-override"))]
        if let Some(hook) = self.move_hook {
            hook(self, mv, pokemon, target_loc);
            return;
        }
        let _ = (mv, pokemon, target_loc);
        todo!("unported: the gen {} move pipeline (runMove)", self.gen)
    }

    /// PS `runAction(action)`. Returns whether the turn stopped for a
    /// request or the battle ended (PS's `true`).
    // PS: sim/battle.ts runAction
    pub fn run_action(&mut self, action: Action) -> bool {
        let original_hp = action.pokemon.map(|m| self.mon(m).hp);
        match action.choice {
            ActionKind::Start => {
                for s in 0..2u8 {
                    let sd = &mut self.sides[s as usize];
                    if sd.pokemon_left != 0 {
                        sd.pokemon_left = sd.team_size;
                    }
                    let size = sd.team_size;
                    self.add(Msg::TeamSize { side: side_id(s), size });
                }
                self.add(Msg::Start);
                for s in 0..2u8 {
                    for i in 0..self.sides[s as usize].team_size as usize {
                        let m = self.pokemon_at(s, i);
                        // `dex.conditions.getByID(pokemon.species.id)`.
                        let key = self.dex.species_def(self.mon(m).species).name.as_str();
                        if let Some(c) = self.dex.condition_id(&to_id(key)) {
                            self.single_event(
                                EventId::BattleStart,
                                EffectId::Condition(c),
                                StateRef::Species(m),
                                0,
                                Holder::Mon(m),
                                None,
                                EffectId::None,
                                None,
                            );
                        }
                    }
                }
                // No `onBattleStart` in the formats or their rules.
                for s in 0..2u8 {
                    if self.sides[s as usize].pokemon_left == 0 {
                        // Forfeited before starting.
                        let m = self.pokemon_at(s, 0);
                        self.sides[s as usize].active[0] = Some(m.index);
                        self.mon_mut(m).fainted = true;
                        self.mon_mut(m).hp = 0;
                    } else {
                        let m = self.pokemon_at(s, 0);
                        self.switch_in(m, 0, EffectId::None, false);
                    }
                }
                self.mid_turn = true;
            }
            ActionKind::Move => {
                let pokemon = action.pokemon.expect("move action without a pokemon");
                if !self.mon(pokemon).is_active || self.mon(pokemon).fainted {
                    return false;
                }
                self.run_move(action.move_, pokemon, action.target_loc);
            }
            ActionKind::BeforeTurnMove => {
                let pokemon = action.pokemon.expect("beforeTurnMove without a pokemon");
                if !self.mon(pokemon).is_active || self.mon(pokemon).fainted {
                    return false;
                }
                let mv = action.move_.id().expect("beforeTurnMove of a dex move");
                self.debug(Text::with1(TextId::BeforeTurnCallback, TextArg::move_id(MoveNum(mv.0))));
                let Some(target) = self.get_target(pokemon, action.move_, action.target_loc, None) else {
                    return false;
                };
                let h = self
                    .registry
                    .handler(EffectId::Move(mv), Prefix::None, EventId::BeforeTurnCallback)
                    .unwrap_or_else(|| panic!("unported: beforeTurnCallback of {}", self.dex.move_def(mv).name.as_str()));
                let args = EventArgs {
                    id: EventId::BeforeTurnCallback,
                    relay: None,
                    target: Holder::Mon(pokemon),
                    source: Some(target),
                    effect: EffectId::Move(mv),
                };
                if let Callback::Fn(f) = h.callback {
                    f(self, &args);
                }
            }
            ActionKind::Event => {
                let pokemon = action.pokemon.expect("event action without a pokemon");
                let ev = action.event.expect("event action without an event");
                self.run_event(ev, Holder::Mon(pokemon), None, EffectId::None, None);
            }
            ActionKind::Pass => return false,
            ActionKind::InstaSwitch | ActionKind::Switch => {
                let pokemon = action.pokemon.expect("switch action without a pokemon");
                // A `switch` of a statused Pokemon runs
                // `singleEvent('CheckShow', naturalcure, ...)`: no gen 1-3
                // ability has `onCheckShow`.
                let target = action.target.expect("switch action without a target");
                let position = self.mon(pokemon).position;
                if self.switch_in(target, position, action.source_effect, false) == SwitchResult::PursuitFaint {
                    // Gens 2-4: the switch still happens.
                    self.hint(TextId::HintPursuitSwitch, false, None);
                    let mut a = action;
                    a.priority = -101;
                    self.queue.unshift(a);
                }
            }
            ActionKind::RunSwitch => {
                let pokemon = action.pokemon.expect("runSwitch without a pokemon");
                self.run_switch(pokemon);
            }
            ActionKind::BeforeTurn => {
                self.each_event(EventId::BeforeTurn, EffectId::None, None);
            }
            ActionKind::Residual => {
                self.add(Msg::Spacer);
                self.clear_active_move(true);
                self.update_speed();
                self.field_event(EventId::Residual, None);
                if !self.ended {
                    self.add(Msg::Upkeep);
                }
            }
        }

        // Phazing (Roar, Whirlwind).
        for s in 0..2u8 {
            for pos in 0..self.sides[s as usize].active.len() as u8 {
                let Some(m) = self.active_at(s, pos) else { continue };
                if self.mon(m).force_switch_flag {
                    if self.mon(m).hp != 0 {
                        let position = self.mon(m).position;
                        self.drag_in(s, position);
                    }
                    self.mon_mut(m).force_switch_flag = false;
                }
            }
        }

        self.clear_active_move(false);

        self.faint_messages(false, false, true);
        if self.ended {
            return true;
        }

        // Gen <= 3 brings in replacements after every move, not only at the
        // end of the turn.
        let next = self.queue.peek().map(|a| a.choice);
        if next.is_none() || (self.gen <= 3 && matches!(next, Some(ActionKind::Move | ActionKind::Residual))) {
            self.check_fainted();
        } else if next == Some(ActionKind::InstaSwitch) {
            return false;
        }

        if action.choice == ActionKind::RunSwitch {
            let m = action.pokemon.expect("runSwitch without a pokemon");
            let mon = self.mon(m);
            let half = u32::from(mon.maxhp);
            if mon.hp != 0 && 2 * u32::from(mon.hp) <= half && original_hp.is_some_and(|h| 2 * u32::from(h) > half) {
                self.run_event(EventId::EmergencyExit, Holder::Mon(m), None, EffectId::None, None);
            }
        }

        let mut switches = [false; 2];
        for s in 0..2u8 {
            switches[s as usize] = self.active_at(s, 0).is_some_and(|m| self.mon(m).switch_flag.is_set());
        }
        for s in 0..2u8 {
            if switches[s as usize] && self.can_switch(s) == 0 {
                // No Revival Blessing in gens 1-3.
                if let Some(m) = self.active_at(s, 0) {
                    self.mon_mut(m).switch_flag = SwitchFlag::No;
                }
                switches[s as usize] = false;
            } else if switches[s as usize] {
                if let Some(m) = self.active_at(s, 0) {
                    let mon = self.mon(m);
                    if mon.hp != 0 && mon.switch_flag.is_set() && !mon.skip_before_switch_out_event_flag {
                        self.run_event(EventId::BeforeSwitchOut, Holder::Mon(m), None, EffectId::None, None);
                        self.mon_mut(m).skip_before_switch_out_event_flag = true;
                        self.faint_messages(false, false, true);
                        if self.ended {
                            return true;
                        }
                        if self.mon(m).fainted {
                            switches[s as usize] = self.mon(m).switch_flag.is_set();
                        }
                    }
                }
            }
        }
        if switches.iter().any(|&s| s) {
            self.make_request(Some(RequestState::Switch));
            return true;
        }

        if self.gen < 5 {
            self.each_event(EventId::Update, EffectId::None, None);
        }
        false
    }

    /// PS `clearActiveMove(failed)`.
    // PS: sim/battle.ts clearActiveMove
    pub fn clear_active_move(&mut self, _failed: bool) {
        if self.active_move.is_some() {
            // `lastMove` is the pipelines' (P1-8, P1-9).
            self.active_move = None;
            self.active_pokemon = None;
            self.active_target = None;
        }
    }

    /// PS `endTurn()`.
    // PS: sim/battle.ts endTurn
    pub fn end_turn(&mut self) {
        self.turn += 1;
        self.last_successful_move_this_turn = MoveId::NONE;
        // No Dynamax in gens 1-3.

        if self.gen == 1 {
            self.gen1_partial_trapping_cleanup();
        }

        let mut trapped_by_side = [true; 2];
        for s in 0..2u8 {
            let mut side_trapped = true;
            for pos in 0..self.sides[s as usize].active.len() as u8 {
                let Some(m) = self.active_at(s, pos) else { continue };
                let turn = self.turn;
                let mon = self.mon_mut(m);
                mon.move_this_turn = MoveId::NONE;
                mon.newly_switched = false;
                mon.move_last_turn_result = mon.move_this_turn_result;
                mon.move_this_turn_result = None;
                if turn != 1 {
                    mon.used_item_this_turn = false;
                    mon.stats_raised_this_turn = false;
                    mon.stats_lowered_this_turn = false;
                    mon.hurt_this_turn = None;
                }
                mon.maybe_disabled = false;
                mon.maybe_locked = false;
                for slot in mon.move_slots.iter_mut() {
                    slot.disabled = 0;
                }
                self.run_event(EventId::DisableMove, Holder::Mon(m), None, EffectId::None, None);
                for i in 0..4 {
                    let id = self.mon(m).move_slots[i].id;
                    if id.is_none() {
                        continue;
                    }
                    self.single_event(
                        EventId::DisableMove,
                        EffectId::Move(id),
                        StateRef::Scratch,
                        0,
                        Holder::Mon(m),
                        None,
                        EffectId::None,
                        None,
                    );
                    assert!(
                        !self.dex.move_def(id).flags.has("cantusetwice"),
                        "unported: cantusetwice (gen 9)"
                    );
                }
                // `attackedBy`: entries of Pokemon still active stay, for
                // this turn no more; the rest go.
                let actives: [bool; 12] = core::array::from_fn(|k| self.mon(MonRef::new((k / 6) as u8, (k % 6) as u8)).is_active);
                let mon = self.mon_mut(m);
                let mut kept = [None; 4];
                let mut n = 0;
                for a in mon.attacked_by.iter().flatten() {
                    if actives[a.source.side as usize * 6 + a.source.index as usize] {
                        kept[n] = Some(crate::state::Attacker { this_turn: false, ..*a });
                        n += 1;
                    }
                }
                mon.attacked_by = kept;

                let mon = self.mon_mut(m);
                mon.trapped = Trapped::No;
                mon.maybe_trapped = false;
                self.run_event(EventId::TrapPokemon, Holder::Mon(m), None, EffectId::None, None);
                // `!knownType || getImmunity('trapped', pokemon)`; the type
                // is always known in gens 1-3.
                if self.trap_immunity_passes(m) {
                    self.run_event(EventId::MaybeTrapPokemon, Holder::Mon(m), None, EffectId::None, None);
                }
                // Gens 3+ ask each foe's other abilities (`FoeMaybeTrapPokemon`)
                // unless the format is hackmons-like: neither custom game
                // has `obtainableabilities`, so every slot is skipped.

                if self.mon(m).fainted {
                    continue;
                }
                side_trapped = side_trapped && self.mon(m).trapped.truthy();
                // Staleness only matters under Endless Battle Clause.
                self.mon_mut(m).active_turns += 1;
            }
            trapped_by_side[s as usize] = side_trapped;
            let sd = &mut self.sides[s as usize];
            sd.fainted_last_turn = sd.fainted_this_turn;
            sd.fainted_this_turn = None;
        }

        if self.maybe_trigger_endless_battle_clause(trapped_by_side) {
            return;
        }

        self.add(Msg::Turn(self.turn));
        if self.gen == 2 {
            self.quick_claw_roll = self.prng.random_chance(60, 256);
        }
        if self.gen == 3 {
            self.quick_claw_roll = self.prng.random_chance(1, 5);
        }
        self.make_request(Some(RequestState::Move));
    }

    /// `dex.getImmunity('trapped', pokemon)`: true unless a type of the
    /// Pokemon is immune to being trapped.
    // PS: sim/dex.ts getImmunity
    fn trap_immunity_passes(&mut self, m: MonRef) -> bool {
        let types = self.get_types(m);
        !types.as_slice().iter().any(|&t| self.dex.typechart.special_code("trapped", t) == 3)
    }

    /// Gen 1's end-of-turn partial trapping cleanup in `endTurn`. The
    /// condition ports (G1) store `partialtrappinglock.locked` and
    /// `fakepartiallytrapped.counterpart` in `data[0]` (see
    /// [`EffectState::data_mon`]).
    fn gen1_partial_trapping_cleanup(&mut self) {
        let (actives, n) = self.all_active(false);
        let lock = self.dex.condition_id("partialtrappinglock");
        let trapped = self.dex.condition_id("partiallytrapped");
        let fake = self.dex.condition_id("fakepartiallytrapped");
        let has = |b: &Battle, m: MonRef, c: Option<ps_dex::ConditionId>| {
            c.is_some_and(|c| b.mon(m).volatiles.contains(EffectId::Condition(c)))
        };
        for &m in &actives[..n] {
            if let Some(c) = lock {
                if let Some(st) = self.mon(m).volatiles.get(EffectId::Condition(c)) {
                    let target = st.data_mon(0).expect("partialtrappinglock.locked");
                    if self.mon(target).hp == 0 || !has(self, target, trapped) {
                        self.mon_mut(m).volatiles.remove(EffectId::Condition(c));
                    }
                }
            }
            if let Some(c) = trapped {
                if let Some(st) = self.mon(m).volatiles.get(EffectId::Condition(c)) {
                    let source = st.source.expect("partiallytrapped.source");
                    if self.mon(source).hp == 0 || !has(self, source, lock) {
                        self.mon_mut(m).volatiles.remove(EffectId::Condition(c));
                    }
                }
            }
            if let Some(c) = fake {
                if let Some(st) = self.mon(m).volatiles.get(EffectId::Condition(c)) {
                    let other = st.data_mon(0).expect("fakepartiallytrapped.counterpart");
                    if self.mon(other).hp == 0 || !has(self, other, fake) {
                        self.mon_mut(m).volatiles.remove(EffectId::Condition(c));
                    }
                }
            }
        }
    }

    /// PS `maybeTriggerEndlessBattleClause(trappedBySide, stalenessBySide)`.
    /// Neither custom game has Endless Battle Clause, so past the turn
    /// limit only Gen 1's no-progress check remains.
    // PS: sim/battle.ts maybeTriggerEndlessBattleClause
    pub fn maybe_trigger_endless_battle_clause(&mut self, _trapped_by_side: [bool; 2]) -> bool {
        if self.gen <= 1 {
            let mut no_progress = true;
            for s in 0..2u8 {
                let foe = 1 - s;
                let mut foe_all_ghosts = true;
                for i in 0..self.sides[foe as usize].team_size as usize {
                    let p = self.pokemon_at(foe, i);
                    if !(self.mon(p).fainted || self.get_types(p).as_slice().contains(&Type::Ghost)) {
                        foe_all_ghosts = false;
                        break;
                    }
                }
                let foe_all_transform = (0..self.sides[foe as usize].team_size as usize).all(|i| {
                    let p = self.pokemon_at(foe, i);
                    self.mon(p).fainted || self.only_transform(p)
                });
                let side_ok = (0..self.sides[s as usize].team_size as usize).all(|i| {
                    let p = self.pokemon_at(s, i);
                    let mon = self.mon(p);
                    let st = mon.status;
                    let frz = !st.is_none() && self.dex.condition_def(st).key.as_str() == "frz";
                    mon.fainted
                        || frz
                        || (self.only_transform(p) && foe_all_transform)
                        || (mon.move_slots.iter().filter(|m| !m.id.is_none()).all(|m| m.pp == 0) && foe_all_ghosts)
                });
                if !side_ok {
                    no_progress = false;
                    break;
                }
            }
            if no_progress {
                self.add(Msg::Message(Text::new(TextId::EndlessBattleNoProgress)));
                self.tie();
                return true;
            }
        }
        if self.turn <= 100 {
            return false;
        }
        if self.turn > 1000 {
            self.add(Msg::Notice(Text::new(TextId::TurnLimit)));
            self.tie();
            return true;
        }
        let t = self.turn;
        if (t >= 500 && t % 100 == 0) || (t >= 900 && t % 10 == 0) || t >= 990 {
            let left = 1000 - i32::from(t);
            let text = if left == 1 {
                Text::new(TextId::AutoTieIn1)
            } else {
                Text::with1(TextId::AutoTieIn, TextArg::int(left))
            };
            self.add(Msg::BigError(text));
        }
        // No `endlessbattleclause` in the custom games.
        false
    }

    /// `pokemon.moves.every(moveid => moveid === 'transform')`.
    fn only_transform(&self, m: MonRef) -> bool {
        self.mon(m)
            .move_slots
            .iter()
            .filter(|s| !s.id.is_none())
            .all(|s| self.dex.move_def(s.id).name.as_str() == "Transform")
    }

    /// PS `checkFainted()`: fainted actives get `fnt` and a switch flag.
    /// The `fnt` status is no condition; it reads as no status here.
    // PS: sim/battle.ts checkFainted
    pub fn check_fainted(&mut self) {
        for s in 0..2u8 {
            let Some(m) = self.active_at(s, 0) else { continue };
            if self.mon(m).fainted {
                let mon = self.mon_mut(m);
                mon.status = ps_dex::ConditionId::NONE;
                mon.switch_flag = SwitchFlag::Yes;
            }
        }
    }

    /// PS `faintMessages(lastFirst, forceCheck, checkWin)`. Returns PS's
    /// `true` when the battle ended.
    // PS: sim/battle.ts faintMessages
    pub fn faint_messages(&mut self, last_first: bool, force_check: bool, check_win: bool) -> bool {
        if self.ended {
            return false;
        }
        let length = self.faint_queue.iter().flatten().count();
        if length == 0 {
            if force_check && self.check_win(None) {
                return true;
            }
            return false;
        }
        if last_first {
            let last = self.faint_queue[length - 1];
            self.faint_queue.copy_within(0..length - 1, 1);
            self.faint_queue[0] = last;
        }
        let mut check_win = check_win;
        let mut faint_data = None;
        while let Some(fd) = self.faint_queue[0] {
            let left = self.faint_queue.iter().flatten().count();
            self.faint_queue.copy_within(1.., 0);
            self.faint_queue[crate::state::FAINT_QUEUE_CAP - 1] = None;
            faint_data = Some(fd);
            let m = fd.target;
            if self.mon(m).fainted {
                continue;
            }
            if !self.run_event(EventId::BeforeFaint, Holder::Mon(m), fd.source, fd.effect, None).truthy() {
                continue;
            }
            let ident = self.ident(m);
            self.add(Msg::Faint(ident));
            let sd = &mut self.sides[m.side as usize];
            if sd.pokemon_left != 0 {
                sd.pokemon_left -= 1;
            }
            if sd.total_fainted < 100 {
                sd.total_fainted += 1;
            }
            self.run_event(EventId::Faint, Holder::Mon(m), fd.source, fd.effect, None);
            let ability = self.mon(m).ability_effect();
            let serial = self.mon(m).ability_state.serial;
            self.single_event(EventId::End, ability, StateRef::Ability(m), serial, Holder::Mon(m), None, EffectId::None, None);
            let item = self.mon(m).item_effect();
            let serial = self.mon(m).item_state.serial;
            self.single_event(EventId::End, item, StateRef::Item(m), serial, Holder::Mon(m), None, EffectId::None, None);
            // No forme regression in gens 1-3.
            self.clear_volatile(m, false);
            let mon = self.mon_mut(m);
            mon.fainted = true;
            mon.is_active = false;
            mon.is_started = false;
            self.sides[m.side as usize].fainted_this_turn = Some(m.index);
            if self.faint_queue.iter().flatten().count() >= left {
                check_win = true;
            }
        }

        if self.gen <= 1 {
            // Fainting skips the rest of the turn.
            self.queue.clear();
        } else if self.gen <= 3 {
            // Fainting in singles skips to the residuals: gen 3 cancels
            // every action of the Pokemon still standing.
            let (actives, n) = self.all_active(false);
            for &m in &actives[..n] {
                if self.gen <= 2 {
                    self.queue.cancel_move(m);
                } else {
                    self.queue.cancel_action(m);
                }
            }
        }

        if check_win && self.check_win(faint_data) {
            return true;
        }
        if let Some(fd) = faint_data {
            self.run_event(EventId::AfterFaint, Holder::Mon(fd.target), fd.source, fd.effect, Some(Relay::Int(length as i32)));
        }
        false
    }

    /// PS `checkWin(faintData)`: both sides out is a tie in gens 1-4.
    // PS: sim/battle.ts checkWin
    pub fn check_win(&mut self, faint_data: Option<FaintData>) -> bool {
        if self.sides.iter().all(|s| s.pokemon_left == 0) {
            let side = if self.gen > 4 { faint_data.map(|f| f.target.side) } else { None };
            self.win(side);
            return true;
        }
        for s in 0..2u8 {
            if self.sides[1 - s as usize].pokemon_left == 0 {
                self.win(Some(s));
                return true;
            }
        }
        false
    }

    /// PS `win(side)`: `None` is a tie.
    // PS: sim/battle.ts win
    pub fn win(&mut self, side: Option<u8>) -> bool {
        if self.ended {
            return false;
        }
        self.winner = Some(side.map(side_id));
        self.add(Msg::Spacer);
        match side {
            Some(s) => self.add(Msg::Win(side_id(s))),
            None => self.add(Msg::Tie),
        }
        self.ended = true;
        self.request_state = RequestState::None;
        for s in 0..2 {
            self.sides[s].active_request = None;
        }
        true
    }

    /// PS `tie()`.
    // PS: sim/battle.ts tie
    pub fn tie(&mut self) -> bool {
        self.win(None)
    }

    // ---- choices ----------------------------------------------------------

    /// PS `choose(sideid, input)`.
    // PS: sim/battle.ts choose
    pub fn choose(&mut self, side: u8, choice: ps_protocol::Choice) -> bool {
        if !self.side_choose(side, choice) {
            // PS's "Unknown error for choice" needs a failure without an
            // error message, which a singles choice cannot produce.
            assert!(self.sides[side as usize].choice.error, "Unknown error for choice {choice:?}");
            return false;
        }
        // One action fills a singles choice, so "Incomplete choice" cannot
        // happen either.
        assert!(self.is_choice_done(side), "Incomplete choice {choice:?}");
        if self.all_choices_done() {
            self.commit_choices();
        }
        true
    }

    /// PS `allChoicesDone()`.
    // PS: sim/battle.ts allChoicesDone
    pub fn all_choices_done(&mut self) -> bool {
        let mut total = 0;
        for s in 0..2u8 {
            if self.is_choice_done(s) {
                if !self.support_cancel {
                    self.sides[s as usize].choice.cant_undo = true;
                }
                total += 1;
            }
        }
        total >= 2
    }

    /// PS `commitChoices()`: the new actions are sorted and go before what
    /// was left of the turn.
    // PS: sim/battle.ts commitChoices
    pub fn commit_choices(&mut self) {
        self.update_speed();
        let old_queue = self.queue;
        self.queue.clear();
        assert!(self.all_choices_done(), "Not all choices done");
        for s in 0..2u8 {
            self.side_commit_choices(s);
        }
        self.clear_request();
        self.sort_queue();
        for a in old_queue.as_slice() {
            self.queue.push(*a);
        }
        self.request_state = RequestState::None;
        for s in 0..2 {
            self.sides[s].active_request = None;
        }
        self.turn_loop();
    }

    /// PS `undoChoice(sideid)`.
    // PS: sim/battle.ts undoChoice
    pub fn undo_choice(&mut self, side: u8) {
        let rs = self.side_request_state(side);
        if rs == RequestState::None {
            return;
        }
        if self.sides[side as usize].choice.cant_undo {
            self.emit_choice_error(side, Text::new(TextId::CantUndoLeak), None);
            return;
        }
        let mut updated = false;
        if rs == RequestState::Move {
            let choice = self.sides[side as usize].choice;
            for c in choice.actions() {
                if c.action.choice != ActionKind::Move {
                    continue;
                }
                let Some(pokemon) = c.action.pokemon else { continue };
                if self.update_request_for_pokemon(pokemon, |b, m| b.update_disabled_request(m)) {
                    updated = true;
                }
            }
        }
        self.clear_choice(side);
        if updated {
            self.emit_request(side, true);
        }
    }

    // ---- switching ------------------------------------------------------

    /// PS `actions.switchIn(pokemon, pos, sourceEffect, isDrag)`.
    // PS: sim/battle-actions.ts switchIn
    pub fn switch_in(&mut self, pokemon: MonRef, pos: u8, source_effect: EffectId, is_drag: bool) -> SwitchResult {
        if self.mon(pokemon).is_active {
            self.hint(TextId::HintSwitchFailed, false, None);
            return SwitchResult::Failed;
        }
        let s = pokemon.side;
        assert!((pos as usize) < self.sides[s as usize].active.len(), "Invalid switch position {pos}");
        let old_active = self.active_at(s, pos);
        let unfainted = old_active.filter(|&o| self.mon(o).hp != 0);
        if let Some(old) = unfainted {
            self.mon_mut(old).being_called_back = true;
            let copy_volatile = match source_effect {
                EffectId::Move(mv) => matches!(self.dex.move_def(mv).self_switch(), Some(ps_dex::SelfSwitch::CopyVolatile)),
                _ => false,
            };
            if !self.mon(old).skip_before_switch_out_event_flag && !is_drag {
                self.run_event(EventId::BeforeSwitchOut, Holder::Mon(old), None, EffectId::None, None);
            }
            self.mon_mut(old).skip_before_switch_out_event_flag = false;
            if !self.run_event(EventId::SwitchOut, Holder::Mon(old), None, EffectId::None, None).truthy() {
                return SwitchResult::Failed;
            }
            if self.mon(old).hp == 0 {
                return SwitchResult::PursuitFaint;
            }
            let ability = self.mon(old).ability_effect();
            let serial = self.mon(old).ability_state.serial;
            self.single_event(EventId::End, ability, StateRef::Ability(old), serial, Holder::Mon(old), None, EffectId::None, None);
            let item = self.mon(old).item_effect();
            let serial = self.mon(old).item_state.serial;
            self.single_event(EventId::End, item, StateRef::Item(old), serial, Holder::Mon(old), None, EffectId::None, None);
            self.queue.cancel_action(old);
            if copy_volatile {
                self.copy_volatile_from(pokemon, old);
            }
            self.clear_volatile(old, true);
        }
        if let Some(old) = old_active {
            let in_position = self.mon(pokemon).position;
            let mon = self.mon_mut(old);
            mon.is_active = false;
            mon.is_started = false;
            mon.used_item_this_turn = false;
            mon.stats_raised_this_turn = false;
            mon.stats_lowered_this_turn = false;
            mon.position = in_position;
            if mon.fainted {
                mon.status = ps_dex::ConditionId::NONE;
            }
            let last_item = mon.last_item;
            mon.last_item = ItemId::NONE;
            // Gens <= 4 hand `lastItem` to the incoming Pokemon.
            self.mon_mut(pokemon).last_item = last_item;
            self.mon_mut(pokemon).position = pos;
            let sd = &mut self.sides[s as usize];
            sd.order[pos as usize] = pokemon.index;
            sd.order[in_position as usize] = old.index;
        }
        {
            let mon = self.mon_mut(pokemon);
            mon.is_active = true;
            mon.active_turns = 0;
            mon.active_move_actions = 0;
            for slot in mon.move_slots.iter_mut() {
                slot.used = false;
            }
        }
        self.sides[s as usize].active[pos as usize] = Some(pokemon.index);
        let has_ability = !self.mon(pokemon).ability.is_none();
        let st = self.init_effect_state(has_ability, Holder::Mon(pokemon));
        self.mon_mut(pokemon).ability_state = st;
        let has_item = !self.mon(pokemon).item.is_none();
        let st = self.init_effect_state(has_item, Holder::Mon(pokemon));
        self.mon_mut(pokemon).item_state = st;
        self.run_event(EventId::BeforeSwitchIn, Holder::Mon(pokemon), None, EffectId::None, None);
        let mon = self.ident(pokemon);
        let details = self.details(pokemon);
        let health = self.health(pokemon);
        let from = match source_effect {
            EffectId::Move(mv) => Some(Arg::new(ArgValue::Move(MoveNum(mv.0)))),
            EffectId::None => None,
            other => panic!("switch [from] {other:?}: not a move"),
        };
        self.add(if is_drag {
            Msg::Drag { mon, details, health, from }
        } else {
            Msg::Switch { mon, details, health, from }
        });
        let mon = self.mon_mut(pokemon);
        mon.previously_switched_in = mon.previously_switched_in.saturating_add(1);
        // Gen <= 4: runSwitch is queued, drags included.
        self.insert_choice(Action::new(ActionKind::RunSwitch).with_pokemon(pokemon), false);
        SwitchResult::Done
    }

    /// PS `actions.dragIn(side, pos)`.
    // PS: sim/battle-actions.ts dragIn
    pub fn drag_in(&mut self, side: u8, pos: u8) -> bool {
        let Some(pokemon) = self.get_random_switchable(side) else { return false };
        if self.mon(pokemon).is_active {
            return false;
        }
        let old = self.active_at(side, pos).expect("nothing to drag out");
        if self.mon(old).hp == 0 {
            return false;
        }
        if !self.run_event(EventId::DragOut, Holder::Mon(old), None, EffectId::None, None).truthy() {
            return false;
        }
        self.switch_in(pokemon, pos, EffectId::None, true) == SwitchResult::Done
    }

    /// Gen 4's `actions.runSwitch(pokemon)`, which gens 1 and 3 inherit.
    // PS: data/mods/gen4/scripts.ts runSwitch
    pub fn run_switch(&mut self, pokemon: MonRef) -> bool {
        self.run_event(EventId::EntryHazard, Holder::Mon(pokemon), None, EffectId::None, None);
        self.run_event(EventId::SwitchIn, Holder::Mon(pokemon), None, EffectId::None, None);
        if self.gen <= 2 {
            let (actives, n) = self.all_active(false);
            for &p in &actives[..n] {
                self.mon_mut(p).last_move = MoveId::NONE;
            }
            if self.gen == 1 {
                self.sides[pokemon.side as usize].last_selected_move_slot = 0;
                let foe = 1 - pokemon.side;
                if let Some(poke) = self.active_at(foe, 0).filter(|&p| self.mon(p).hp != 0) {
                    let slot = self.sides[foe as usize].last_selected_move_slot as usize;
                    let lock = self.has_volatile_key(poke, "partialtrappinglock");
                    let id = self.mon(poke).move_slots[slot].id;
                    if lock && !id.is_none() && self.dex.move_def(id).name.as_str() == "Metronome" {
                        self.sides[foe as usize].last_selected_move = crate::choice::SelectedMove::Move(id);
                        self.sides[foe as usize].last_enemy_selected_move = crate::choice::SelectedMove::Move(id);
                        if self.will_move(poke).is_some() {
                            // PS passes `poke` (not `pokemon`), so
                            // `changeAction` fills in the Pokemon.
                            let mut a = Action::new(ActionKind::Move);
                            a.move_ = ActionMove::Move(id);
                            self.change_action(poke, a);
                        }
                    }
                }
            }
            // `draggedIn` is only set in gen 2.
            if self.sides[pokemon.side as usize].fainted_this_turn.is_none() {
                self.run_event(EventId::AfterSwitchInSelf, Holder::Mon(pokemon), None, EffectId::None, None);
            }
        }
        if self.mon(pokemon).hp == 0 {
            return false;
        }
        self.mon_mut(pokemon).is_started = true;
        if !self.mon(pokemon).fainted {
            let ability = self.mon(pokemon).ability_effect();
            let serial = self.mon(pokemon).ability_state.serial;
            self.single_event(EventId::Start, ability, StateRef::Ability(pokemon), serial, Holder::Mon(pokemon), None, EffectId::None, None);
            let item = self.mon(pokemon).item_effect();
            let serial = self.mon(pokemon).item_state.serial;
            self.single_event(EventId::Start, item, StateRef::Item(pokemon), serial, Holder::Mon(pokemon), None, EffectId::None, None);
        }
        true
    }
}

impl EffectState {
    /// A Pokemon stored in `data[i]` (side * 8 + slot + 1; 0 is none), for
    /// the few conditions that keep one besides `source`.
    pub fn data_mon(&self, i: usize) -> Option<MonRef> {
        let v = self.data[i];
        (v > 0).then(|| MonRef::new(((v - 1) / 8) as u8, ((v - 1) % 8) as u8))
    }

    /// Stores `m` in `data[i]` (see [`EffectState::data_mon`]).
    pub fn set_data_mon(&mut self, i: usize, m: Option<MonRef>) {
        self.data[i] = m.map_or(0, |m| i16::from(m.side) * 8 + i16::from(m.index) + 1);
    }
}

#[cfg(test)]
#[path = "battle_tests.rs"]
mod tests;
