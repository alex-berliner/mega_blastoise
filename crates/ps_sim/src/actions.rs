//! The move pipeline seam (design 5.4): PS's `BattleActions` per gen.
//!
//! PS overrides the move pipeline per gen in `data/mods/genN/scripts.ts`,
//! each mod inheriting from the next gen up. [`GenScripts`] is the set of
//! functions a gen overrides; `mods::gen1::scripts::Gen1Scripts` folds in
//! what gen 1 inherits, and [`Battle::run_move`] and friends pick the gen's
//! implementation by a plain `match` on the gen (no trait objects).
//!
//! This file also holds what every pipeline shares and PS keeps outside
//! `BattleActions`: the [`ActiveMove`] object and its arena, `addMove` /
//! `attrLastMove`, `spreadDamage` / `damage` / `heal`, and the `Pokemon`
//! methods the pipelines call (`runImmunity`, `isGrounded`,
//! `runEffectiveness`, `hasType`, `gotAttacked`, `moveUsed`, `heal`).
//! Gen 4's `applyRecoilDamage` (which gens 1 and 3 inherit) is here too.
//!
//! # The `ActiveMove` object
//!
//! PS's pipeline passes one mutable `ActiveMove` object around:
//! `getActiveMove` returns an `ActiveMove` it is given unchanged, so in the
//! `runAction` path `runMove`, `useMove`, `useMoveInner` and every hit see
//! the same object, and `battle.activeMove` points at it. A move that calls
//! another (`useMove(id)` from Metronome) makes a new one. Here the objects
//! live in [`Battle::moves`], a small arena reset at each top-level
//! `runMove`, and [`MoveRef`] is the reference: [`Battle::active_move`] is
//! one, and handlers reach the move in flight through
//! [`Battle::active_move_mut`]. `pokemon.lastMove`, `side.lastMove` and the
//! other references PS keeps past the action store the move's id.
//!
//! PS: sim/battle.ts `setActiveMove`, `addMove`, `attrLastMove`,
//! `spreadDamage`, `damage`, `heal`; sim/pokemon.ts `runImmunity`,
//! `isGrounded`, `runEffectiveness`, `hasType`, `gotAttacked`, `moveUsed`,
//! `heal`, `getMoveHitData`; sim/dex.ts `getActiveMove`, `getImmunity`,
//! `getEffectiveness`; data/mods/gen4/scripts.ts `applyRecoilDamage`.

use ps_dex::{FixedDamage, MoveCategory, MoveId, Type};
use ps_protocol::{Arg, ArgValue, Kw, KwFlags, Msg, Word};

use crate::effect::{EffectId, EventId, Pseudo};
use crate::mods::gen1::scripts::Gen1Scripts;
use crate::queue::ActionMove;
use crate::relay::Relay;
use crate::state::{Attacker, Battle, Holder, MonRef};

/// How many `ActiveMove` objects one action can make: the action's own,
/// plus the moves it calls (Metronome, Mirror Move) and theirs.
pub const MOVE_CAP: usize = 4;

/// A reference to an [`ActiveMove`] in [`Battle::moves`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoveRef(pub u8);

/// The ids PS gives a move object that names no move (`getActiveMove` of
/// an unknown id is an empty move with that id).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FakeMove {
    /// A dex move.
    None,
    /// Gen 1's `nomove` (`lastSelectedMove` before any selection).
    NoMove,
    /// Gen 1's `cannotmove` (partially trapped).
    CannotMove,
    /// `recharge` (a `LockMove` answer; gen 1's `commitChoices` turns it
    /// back into the move, so only later gens run it).
    Recharge,
}

/// PS's `ActiveMove`: the dex move, copied per use, with the fields the
/// pipelines write. Fields not here are read from the dex by `id`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActiveMove {
    /// `move.id` (`MoveId::NONE` for a [`FakeMove`]).
    pub id: MoveId,
    pub fake: FakeMove,
    pub base_power: u16,
    /// `type`; `Type::None` is `'???'`.
    pub type_: Type,
    pub category: MoveCategory,
    /// `accuracy` times 256 (`None` is `true`): every dex accuracy is a
    /// whole percentage, and gen 1's `nomove` is `100 * 81 / 256`.
    pub accuracy_x256: Option<u16>,
    pub ohko: bool,
    /// `ignoreImmunity` (the dex sets it on every move: `true` for status
    /// moves unless the data says otherwise).
    pub ignore_immunity: bool,
    pub ignore_offensive: bool,
    pub ignore_defensive: bool,
    /// `ignoreAbility` (Mold Breaker and friends).
    pub ignore_ability: bool,
    /// `hit`: the strike being made, 0 before the first.
    pub hit: u8,
    /// `lastHit`.
    pub last_hit: bool,
    /// `damage`: the dex's fixed damage, or gen 1's multi-hit damage of the
    /// first strike; `None` is `null` / `undefined`.
    pub damage: Option<FixedDamage>,
    /// `sourceEffect`.
    pub source_effect: EffectId,
    /// `getMoveHitData(target)` of the one target singles has: `crit`,
    /// `typeMod`.
    pub crit: bool,
    pub type_mod: i8,
}

impl ActiveMove {
    /// An unused arena slot.
    pub const EMPTY: ActiveMove = ActiveMove {
        id: MoveId::NONE,
        fake: FakeMove::None,
        base_power: 0,
        type_: Type::None,
        category: MoveCategory::Status,
        accuracy_x256: None,
        ohko: false,
        ignore_immunity: false,
        ignore_offensive: false,
        ignore_defensive: false,
        ignore_ability: false,
        hit: 0,
        last_hit: false,
        damage: None,
        source_effect: EffectId::None,
        crit: false,
        type_mod: 0,
    };

    /// PS `dex.getActiveMove(id)`: a fresh copy of the dex move, `hit` 0.
    // PS: sim/dex.ts getActiveMove
    pub fn from_dex(dex: &ps_dex::Dex, id: MoveId) -> ActiveMove {
        let def = dex.move_def(id);
        ActiveMove {
            id,
            fake: FakeMove::None,
            base_power: def.base_power,
            type_: def.type_,
            category: def.category,
            accuracy_x256: match def.accuracy() {
                ps_dex::Accuracy::Always => None,
                ps_dex::Accuracy::Percent(p) => Some(u16::from(p) * 256),
            },
            ohko: def.bools.has("ohko"),
            ignore_immunity: def.bools.has("ignoreImmunity"),
            ignore_offensive: def.bools.has("ignoreOffensive"),
            ignore_defensive: def.bools.has("ignoreDefensive"),
            ignore_ability: def.bools.has("ignoreAbility"),
            damage: def.damage(),
            ..ActiveMove::EMPTY
        }
    }

    /// The move as an effect (`EffectId::None` for a fake move).
    pub fn effect(&self) -> EffectId {
        if self.id.is_none() {
            EffectId::None
        } else {
            EffectId::Move(self.id)
        }
    }

    /// The move's relay value, as PS passes the move object to
    /// `ModifyMove` and friends.
    pub fn relay(&self) -> Relay {
        Relay::Move(self.id)
    }
}

/// What PS's pipeline functions return where a damage number, `false`,
/// `null` and `undefined` all mean different things (`getDamage`'s doc in
/// gen 1's `moveHit` spells them out).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveResult {
    Undefined,
    Null,
    False,
    True,
    Int(i32),
}

impl MoveResult {
    /// JS truthiness.
    pub fn truthy(self) -> bool {
        match self {
            MoveResult::Int(n) => n != 0,
            MoveResult::True => true,
            _ => false,
        }
    }

    /// `x || x === 0`: a number.
    pub fn is_number(self) -> bool {
        matches!(self, MoveResult::Int(_))
    }

    /// The number, or 0 for anything else (`x || 0`).
    pub fn or_zero(self) -> i32 {
        match self {
            MoveResult::Int(n) => n,
            _ => 0,
        }
    }

    /// A relay value as a pipeline result.
    pub fn from_relay(r: Relay) -> MoveResult {
        match r {
            Relay::Int(n) => MoveResult::Int(n),
            Relay::Bool(true) => MoveResult::True,
            Relay::Bool(false) => MoveResult::False,
            Relay::Null => MoveResult::Null,
            Relay::NotFail => MoveResult::Undefined,
            other => panic!("pipeline result {other:?}"),
        }
    }
}

/// The `moveData` of `moveHit`: the move itself, one of its `secondaries`,
/// or a `self` effect.
#[derive(Clone, Copy, Debug)]
pub enum MoveData {
    Move,
    Secondary(&'static ps_dex::Secondary),
    SelfEffect(&'static ps_dex::SelfEffect),
}

/// The functions a gen's `scripts.ts` overrides (gen 1's list; later gens
/// add theirs), as associated functions over the battle. Each gen's
/// implementation folds in what its mod inherits.
pub trait GenScripts {
    /// `actions.runMove(move, pokemon, targetLoc, {sourceEffect})`.
    fn run_move(b: &mut Battle, mv: ActionMove, pokemon: MonRef, target_loc: i8, source_effect: EffectId);
    /// `actions.useMove(move, pokemon, {target, sourceEffect})`; `target`
    /// `None` is `undefined`.
    fn use_move(b: &mut Battle, mv: MoveRef, pokemon: MonRef, target: Option<Option<MonRef>>, source_effect: EffectId) -> bool;
    /// `actions.useMoveInner(move, pokemon, {target, sourceEffect})`.
    fn use_move_inner(
        b: &mut Battle,
        mv: MoveRef,
        pokemon: MonRef,
        target: Option<Option<MonRef>>,
        source_effect: EffectId,
    ) -> bool;
    /// `actions.tryMoveHit(target, pokemon, move)`.
    fn try_move_hit(b: &mut Battle, target: MonRef, pokemon: MonRef, mv: MoveRef) -> MoveResult;
    /// `actions.moveHit(target, pokemon, move, moveData, isSecondary, isSelf)`.
    fn move_hit(
        b: &mut Battle,
        target: Option<MonRef>,
        pokemon: MonRef,
        mv: MoveRef,
        data: MoveData,
        is_secondary: bool,
        is_self: bool,
    ) -> MoveResult;
    /// `actions.getDamage(source, target, move, suppressMessages)`.
    fn get_damage(b: &mut Battle, source: MonRef, target: MonRef, mv: MoveRef, data: MoveData, suppress: bool) -> MoveResult;
    /// `pokemon.deductPP(move, amount)`: the PP taken.
    fn deduct_pp(b: &mut Battle, pokemon: MonRef, amount: i32) -> i32;
    /// `battle.boost(boost, target, source, effect)`: `boosts` in PS's
    /// `BoostID` order (atk, def, spa, spd, spe, accuracy, evasion), 0 for a
    /// stat the table leaves out. PS returns `0`, `null` or `true`.
    fn boost(b: &mut Battle, boosts: [i8; 7], target: MonRef, source: Option<MonRef>, effect: EffectId) -> MoveResult;
    /// `pokemon.clearBoosts()`.
    fn clear_boosts(b: &mut Battle, pokemon: MonRef);
}

impl Battle {
    // ---- dispatch -------------------------------------------------------

    /// PS `actions.runMove(move, pokemon, targetLoc, options)`, the gen's
    /// pipeline.
    // PS: data/mods/gen1/scripts.ts runMove
    pub fn run_move(&mut self, mv: ActionMove, pokemon: MonRef, target_loc: i8, source_effect: EffectId) {
        #[cfg(any(test, feature = "test-override"))]
        if let Some(hook) = self.move_hook {
            hook(self, mv, pokemon, target_loc);
            return;
        }
        // Nothing holds an `ActiveMove` across actions but by id.
        self.move_count = 0;
        match self.gen {
            1 => Gen1Scripts::run_move(self, mv, pokemon, target_loc, source_effect),
            g => todo!("unported: the gen {g} move pipeline (runMove)"),
        }
    }

    /// PS `actions.useMove(move, pokemon, options)`.
    pub fn use_move(&mut self, mv: MoveRef, pokemon: MonRef, target: Option<Option<MonRef>>, source_effect: EffectId) -> bool {
        match self.gen {
            1 => Gen1Scripts::use_move(self, mv, pokemon, target, source_effect),
            g => todo!("unported: the gen {g} move pipeline (useMove)"),
        }
    }

    /// PS `battle.boost(boost, target, source, effect)`.
    pub fn boost(&mut self, boosts: [i8; 7], target: MonRef, source: Option<MonRef>, effect: EffectId) -> MoveResult {
        match self.gen {
            1 => Gen1Scripts::boost(self, boosts, target, source, effect),
            g => todo!("unported: the gen {g} boost"),
        }
    }

    /// PS `pokemon.clearBoosts()`.
    pub fn clear_boosts(&mut self, pokemon: MonRef) {
        match self.gen {
            1 => Gen1Scripts::clear_boosts(self, pokemon),
            g => todo!("unported: the gen {g} clearBoosts"),
        }
    }

    // ---- the ActiveMove arena -------------------------------------------

    /// PS `dex.getActiveMove(id)` for an action's move: a new object.
    // PS: sim/dex.ts getActiveMove
    pub fn get_active_move(&mut self, mv: ActionMove) -> MoveRef {
        let am = match mv {
            ActionMove::Move(id) => ActiveMove::from_dex(self.dex, id),
            ActionMove::NoMove => ActiveMove { fake: FakeMove::NoMove, ..ActiveMove::EMPTY },
            ActionMove::CannotMove => ActiveMove { fake: FakeMove::CannotMove, ..ActiveMove::EMPTY },
            ActionMove::Recharge => ActiveMove { fake: FakeMove::Recharge, ..ActiveMove::EMPTY },
            other => panic!("getActiveMove({other:?})"),
        };
        self.new_active_move(am)
    }

    /// Puts a new move object in the arena.
    pub fn new_active_move(&mut self, am: ActiveMove) -> MoveRef {
        let i = self.move_count as usize;
        assert!(i < MOVE_CAP, "more than {MOVE_CAP} ActiveMove objects in one action");
        self.moves[i] = am;
        self.move_count += 1;
        MoveRef(i as u8)
    }

    /// The move object `r`.
    pub fn mv(&self, r: MoveRef) -> &ActiveMove {
        &self.moves[r.0 as usize]
    }

    pub fn mv_mut(&mut self, r: MoveRef) -> &mut ActiveMove {
        &mut self.moves[r.0 as usize]
    }

    /// `battle.activeMove`.
    pub fn active_move(&self) -> Option<&ActiveMove> {
        self.active_move.map(|r| self.mv(r))
    }

    /// `battle.activeMove`, to change it (a handler's `move.x = ...`).
    pub fn active_move_mut(&mut self) -> Option<&mut ActiveMove> {
        self.active_move.map(|r| &mut self.moves[r.0 as usize])
    }

    /// PS `setActiveMove(move, pokemon, target)`.
    // PS: sim/battle.ts setActiveMove
    pub fn set_active_move(&mut self, mv: Option<MoveRef>, pokemon: Option<MonRef>, target: Option<MonRef>) {
        self.active_move = mv;
        self.active_pokemon = pokemon;
        self.active_target = target.or(pokemon);
    }

    // ---- the log ----------------------------------------------------------

    /// PS `addMove('move', ...)`: the line `attrLastMove` edits.
    // PS: sim/battle.ts addMove
    pub fn add_move(&mut self, m: Msg) {
        // The old `|move|` line is out of `attrLastMove`'s reach now, so a
        // full log may spill it.
        self.last_move_line = None;
        self.add(m);
        self.last_move_line = Some(self.log.as_slice().len() as u16 - 1);
    }

    /// PS `attrLastMove(flag)` for the flags other than `[still]`
    /// ([`Battle::attr_last_move_still`]): `[miss]`, `[notarget]`.
    // PS: sim/battle.ts attrLastMove
    pub fn attr_last_move(&mut self, flag: KwFlags) {
        let Some(i) = self.last_move_line else { return };
        if let Msg::Move { kw, .. } = &mut self.log.as_mut_slice()[i as usize] {
            kw.flags.insert(flag);
        }
    }

    // ---- damage and healing -------------------------------------------

    /// PS `battle.damage(damage, target, source, effect)` (one target, no
    /// `instafaint`): `spreadDamage` of one.
    // PS: sim/battle.ts damage, spreadDamage
    pub fn damage(&mut self, damage: i32, target: Option<MonRef>, source: Option<MonRef>, effect: EffectId) -> MoveResult {
        let target = target.or(self.event.target.mon());
        let source = source.or(self.event.source);
        let effect = if effect.is_none() { self.frame.effect } else { effect };
        let Some(target) = target else { return MoveResult::Int(0) };
        if self.mon(target).hp == 0 {
            return MoveResult::Int(0);
        }
        if !self.mon(target).is_active {
            return MoveResult::False;
        }
        let mut target_damage = damage;
        if target_damage != 0 {
            target_damage = target_damage.max(1);
        }
        // Struggle recoil is not affected by effects (no `strugglerecoil`
        // reaches `spreadDamage` in gens 1-3).
        if let (crate::effect::EffectType::Weather, EffectId::Condition(c)) = (self.effect_type(effect), effect) {
            if !self.run_status_immunity(target, c, false) {
                self.debug(ps_protocol::Text::new(ps_protocol::TextId::WeatherImmunity));
                return MoveResult::Int(0);
            }
        }
        let r = self.run_event_full(EventId::Damage, Holder::Mon(target), source, effect, Some(Relay::Int(target_damage)), true, false);
        match r {
            Relay::Int(n) => target_damage = n,
            other => {
                self.debug(ps_protocol::Text::new(ps_protocol::TextId::DamageEventFailed));
                return MoveResult::from_relay(other);
            }
        }
        if target_damage != 0 {
            target_damage = target_damage.max(1);
        }
        if self.gen <= 1 {
            let skip = matches!(effect, EffectId::Pseudo(Pseudo::Recoil | Pseudo::Drain))
                || self.effect_key(effect) == "leechseed"
                || self.effect_type(effect) == crate::effect::EffectType::Status;
            if !skip {
                self.last_damage = target_damage as u16;
            }
        }
        target_damage = self.damage_mon(target, target_damage, source, effect);
        if target_damage != 0 {
            self.mon_mut(target).hurt_this_turn = Some(self.mon(target).hp);
        }
        let is_move = matches!(effect, EffectId::Move(_));
        if let (Some(s), true) = (source, is_move) {
            self.mon_mut(s).last_damage = target_damage as u16;
        }
        let mon = self.ident(target);
        let health = self.health(target);
        let kw = match effect {
            EffectId::Move(_) | EffectId::None => Kw::NONE,
            EffectId::Pseudo(Pseudo::Recoil) => {
                let from = Arg::new(ArgValue::Word(Word::Recoil));
                match source {
                    Some(s) if s != target => Kw::from_of(from, self.ident(s)),
                    _ => Kw::from(from),
                }
            }
            other => panic!("unported: the -damage line of {other:?}"),
        };
        self.add(Msg::Damage { mon, health, kw });

        if target_damage != 0 && is_move {
            let drain = match effect {
                EffectId::Move(m) => self.dex.move_def(m).drain(),
                _ => None,
            };
            if let (true, Some(d), Some(s)) = (self.gen <= 4, drain, source) {
                let amount = (target_damage * i32::from(d.num) / i32::from(d.den)).max(1);
                if self.gen <= 1 {
                    self.last_damage = amount as u16;
                }
                self.heal(amount, Some(s), Some(target), EffectId::Pseudo(Pseudo::Drain));
            }
        }
        MoveResult::Int(target_damage)
    }

    /// PS `battle.heal(damage, target, source, effect)`: the HP healed, or
    /// PS's falsy result as 0 (the callers here only test truthiness).
    // PS: sim/battle.ts heal
    pub fn heal(&mut self, damage: i32, target: Option<MonRef>, source: Option<MonRef>, effect: EffectId) -> i32 {
        let target = target.or(self.event.target.mon());
        let source = source.or(self.event.source);
        let effect = if effect.is_none() { self.frame.effect } else { effect };
        let mut damage = damage;
        if damage != 0 && damage <= 1 {
            damage = 1;
        }
        damage = self.trunc.int(i64::from(damage), 0) as i32;
        let holder = target.map_or(Holder::None, Holder::Mon);
        match self.run_event(EventId::TryHeal, holder, source, effect, Some(Relay::Int(damage))) {
            Relay::Int(n) => damage = n,
            _ => return 0,
        }
        if damage == 0 {
            return 0;
        }
        let Some(target) = target else { return 0 };
        let mon = self.mon(target);
        if mon.hp == 0 || !mon.is_active || mon.hp >= mon.maxhp {
            return 0;
        }
        let final_damage = self.heal_mon(target, damage);
        let mon = self.ident(target);
        let health = self.health(target);
        let kw = match effect {
            EffectId::Pseudo(Pseudo::Drain) => {
                let from = Arg::new(ArgValue::Word(Word::Drain));
                match source {
                    Some(s) => Kw::from_of(from, self.ident(s)),
                    None => Kw::from(from),
                }
            }
            EffectId::Move(_) => Kw::NONE,
            other => panic!("unported: the -heal line of {other:?}"),
        };
        self.add(Msg::Heal { mon, health, kw });
        self.run_event(EventId::Heal, Holder::Mon(target), source, effect, Some(Relay::Int(final_damage)));
        final_damage
    }

    /// PS `pokemon.heal(d)`: the HP actually healed (0 for PS's `false`).
    // PS: sim/pokemon.ts heal
    pub fn heal_mon(&mut self, m: MonRef, d: i32) -> i32 {
        let mon = self.mon(m);
        if mon.hp == 0 {
            return 0;
        }
        let mut d = self.trunc.int(i64::from(d), 0) as i32;
        if d <= 0 || mon.hp >= mon.maxhp {
            return 0;
        }
        let mut hp = i32::from(mon.hp) + d;
        let maxhp = i32::from(mon.maxhp);
        if hp > maxhp {
            d -= hp - maxhp;
            hp = maxhp;
        }
        self.mon_mut(m).hp = hp as u16;
        d
    }

    /// Gen 4's `actions.applyRecoilDamage(damageDealt, move, pokemon)`
    /// (gens 1 and 3 inherit it). No gen 1-3 move has `struggleRecoil`,
    /// `mindBlownRecoil` or `chloroblastRecoil`.
    // PS: data/mods/gen4/scripts.ts applyRecoilDamage
    pub fn apply_recoil_damage(&mut self, damage_dealt: i32, mv: MoveRef, pokemon: MonRef) -> Option<i32> {
        let id = self.mv(mv).id;
        let recoil = if id.is_none() { None } else { self.dex.move_def(id).recoil() }?;
        let recoil_damage = (damage_dealt * i32::from(recoil.num) / i32::from(recoil.den)).max(1);
        self.damage(recoil_damage, Some(pokemon), Some(pokemon), EffectId::Pseudo(Pseudo::Recoil));
        Some(recoil_damage)
    }

    /// Panics `unported: <id>` when the pipeline reaches an effect whose
    /// handlers are not all ported ([`crate::mods::is_ported`]).
    pub fn require_ported(&self, id: EffectId) {
        if crate::mods::is_ported(self.dex, self.registry, id) {
            return;
        }
        match id {
            EffectId::Move(m) => panic!("unported: {}", crate::mods::PsId(self.dex.move_def(m).name.as_str())),
            EffectId::Condition(c) => panic!("unported: {}", self.dex.condition_def(c).key.as_str()),
            other => panic!("unported: {other:?}"),
        }
    }

    /// `effect.id` for the effects the pipelines compare by id.
    pub fn effect_key(&self, effect: EffectId) -> &'static str {
        match effect {
            EffectId::Condition(c) => self.dex.condition_def(c).key.as_str(),
            EffectId::Pseudo(p) => p.id(),
            _ => "",
        }
    }

    // ---- Pokemon methods ----------------------------------------------

    /// PS `pokemon.hasType(type)`.
    // PS: sim/pokemon.ts hasType
    pub fn has_type(&mut self, m: MonRef, t: Type) -> bool {
        self.get_types(m).as_slice().contains(&t)
    }

    /// PS `dex.getEffectiveness(type, targetType)` for one target type:
    /// 1 super effective, -1 resisted, 0 otherwise (immunity included).
    // PS: sim/dex.ts getEffectiveness
    pub fn type_effectiveness(&self, source: Type, target: Type) -> i32 {
        if source == Type::None || target == Type::None {
            return 0;
        }
        match self.dex.typechart.code(source, target) {
            1 => 1,
            2 => -1,
            _ => 0,
        }
    }

    /// PS `dex.getImmunity(type, pokemon)`: false when a type of the
    /// Pokemon takes no damage from `t`.
    // PS: sim/dex.ts getImmunity
    pub fn get_immunity(&mut self, t: Type, m: MonRef) -> bool {
        let types = self.get_types(m);
        !types.as_slice().iter().any(|&d| t != Type::None && d != Type::None && self.dex.typechart.code(t, d) == 3)
    }

    /// PS `pokemon.runImmunity(move, message)` for a move (a type string
    /// source is [`Battle::run_type_immunity`]).
    // PS: sim/pokemon.ts runImmunity
    pub fn run_immunity(&mut self, m: MonRef, mv: Option<MoveRef>, message: bool) -> bool {
        let Some(mv) = mv else { return true };
        if self.mv(mv).ignore_immunity {
            return true;
        }
        let t = self.mv(mv).type_;
        self.run_type_immunity(m, t, message)
    }

    /// PS `pokemon.runImmunity(type, message)`.
    // PS: sim/pokemon.ts runImmunity
    pub fn run_type_immunity(&mut self, m: MonRef, t: Type, message: bool) -> bool {
        if t == Type::None {
            return true;
        }
        // `runEvent('NegateImmunity', this, type)`: the type string is no
        // Pokemon, so no `onSource` handlers.
        let negate_immunity = !self.run_event(EventId::NegateImmunity, Holder::Mon(m), None, EffectId::None, None).truthy();
        let not_immune = if t == Type::Ground {
            self.is_grounded(m, negate_immunity)
        } else {
            Some(negate_immunity || self.get_immunity(t, m))
        };
        if not_immune == Some(true) {
            return true;
        }
        if !message {
            return false;
        }
        let mon = self.ident(m);
        let kw = match not_immune {
            None => {
                let levitate = self.dex.ability_id("levitate").expect("levitate in the dex");
                Kw::from(Arg::new(ArgValue::AbilityEffect(ps_protocol::AbilityNum(levitate.0))))
            }
            _ => Kw::NONE,
        };
        self.add(Msg::Immune { mon, kw });
        false
    }

    /// PS `pokemon.isGrounded(negateImmunity)`: `None` is PS's `null`
    /// (Levitate). No Gravity, Ingrain (gen 4), Smack Down, Iron Ball,
    /// Roost, Magnet Rise, Telekinesis or Air Balloon in gens 1-3.
    // PS: sim/pokemon.ts isGrounded
    pub fn is_grounded(&mut self, m: MonRef, negate_immunity: bool) -> Option<bool> {
        if !negate_immunity && self.has_type(m, Type::Flying) {
            return Some(false);
        }
        if let Some(levitate) = self.dex.ability_id("levitate") {
            if self.has_ability(m, levitate) && !self.suppressing_ability(Some(m)) {
                return None;
            }
        }
        Some(true)
    }

    /// PS `pokemon.runEffectiveness(move)`.
    // PS: sim/pokemon.ts runEffectiveness
    pub fn run_effectiveness(&mut self, m: MonRef, mv: MoveRef) -> i32 {
        let mut total = 0;
        let types = self.get_types(m);
        let am = *self.mv(mv);
        for &t in types.as_slice() {
            let type_mod = self.type_effectiveness(am.type_, t);
            // `singleEvent('Effectiveness', move, ...)` and
            // `runEvent('Effectiveness', this, type, move, typeMod)`: the
            // type string is no Pokemon.
            let r = self.single_event(
                EventId::Effectiveness,
                am.effect(),
                crate::state::StateRef::Scratch,
                0,
                Holder::Mon(m),
                None,
                am.effect(),
                Some(Relay::Int(type_mod)),
            );
            let type_mod = r.as_int().expect("Effectiveness relay");
            let r = self.run_event(EventId::Effectiveness, Holder::Mon(m), None, am.effect(), Some(Relay::Int(type_mod)));
            total += r.as_int().expect("Effectiveness relay");
        }
        total
    }

    /// PS `pokemon.gotAttacked(move, damage, source)`. `attackedBy` keeps
    /// its last four entries (PS reads only the last ones).
    // PS: sim/pokemon.ts gotAttacked
    pub fn got_attacked(&mut self, m: MonRef, mv: MoveId, damage: MoveResult, source: MonRef) {
        let entry = Attacker {
            source,
            damage: damage.or_zero().max(0) as u16,
            this_turn: true,
            move_: mv,
            slot: self.slot_of(source),
        };
        let list = &mut self.mon_mut(m).attacked_by;
        match list.iter().position(|a| a.is_none()) {
            Some(i) => list[i] = Some(entry),
            None => {
                list.copy_within(1.., 0);
                let last = list.len() - 1;
                list[last] = Some(entry);
            }
        }
    }

    /// PS `pokemon.moveUsed(move, targetLoc)` (gen 1 and 3: no gen 2
    /// exceptions).
    // PS: sim/pokemon.ts moveUsed
    pub fn move_used(&mut self, m: MonRef, mv: MoveRef, target_loc: i8) {
        let id = self.mv(mv).id;
        let mon = self.mon_mut(m);
        mon.last_move = id;
        mon.last_move_target_loc = target_loc;
        mon.move_this_turn = id;
    }
}
