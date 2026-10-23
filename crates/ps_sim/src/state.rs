//! Battle state: [`Battle`], [`Side`], [`Pokemon`], [`Field`], and the
//! effect-state tables they hold (design 5.1).
//!
//! Everything is fixed capacity and lives inline in [`Battle`]; nothing here
//! allocates. Identity is by index: a [`MonRef`] names a Pokemon by side and
//! team slot (the slot never changes; PS's reordering of `side.pokemon` on
//! switches is a separate permutation, added with switching in P1-6), and a
//! [`StateRef`] names where an [`EffectState`] lives.
//!
//! This file also ports the PS methods that create and destroy effect
//! states, because that is where `effectOrder` is assigned and where the
//! insertion order of the tables comes from: `initEffectState`,
//! `addVolatile`, `removeVolatile`, `setStatus`, `clearStatus`,
//! `getStatus`, `runStatusImmunity`, `getTypes`, `ignoringAbility`,
//! `ignoringItem`, `suppressingAbility`, `addSideCondition`,
//! `removeSideCondition`, `addSlotCondition`, `removeSlotCondition`,
//! `addPseudoWeather`, `removePseudoWeather`, `clearWeather`,
//! `suppressingWeather`.
//!
//! PS: sim/battle.ts (fields, `initEffectState`, `clearEffectState`),
//! sim/pokemon.ts, sim/side.ts, sim/field.ts.

use ps_dex::{AbilityId, ConditionId, Dex, ItemId, MoveId, SpeciesId, Type};
use ps_protocol::{Arg, ArgValue, Kw, Msg, MoveNum, PokemonIdent, SideId, Status};

use crate::effect::{EffectId, EffectType, EventArgs, EventId, Prefix, Registry, Rule};
use crate::modify::Trunc;
use crate::prng::{Prng, Seed};
use crate::relay::{Relay, TypeList};

/// Pokemon per side.
pub const TEAM_CAP: usize = 6;
/// Volatiles one Pokemon can hold at once.
pub const VOLATILE_CAP: usize = 12;
/// Side conditions on one side (gen 3 has five: Spikes, Reflect, Light
/// Screen, Mist, Safeguard).
pub const SIDE_CONDITION_CAP: usize = 6;
/// Slot conditions on one active slot (Future Sight, Wish).
pub const SLOT_CONDITION_CAP: usize = 2;
/// Pseudo-weathers on the field, the format's rules with handlers included.
pub const PSEUDO_WEATHER_CAP: usize = 4;
/// PS throws "Stack overflow" at `eventDepth >= 8`.
pub const MAX_EVENT_DEPTH: usize = 8;
/// Lines the log holds between flushes (design 5.6). P1-6 sets the
/// flushing; until then a full log panics, as PS's line limit throws.
pub const LOG_CAP: usize = 128;
/// Removed effect states kept so a handler gathered before the removal
/// still sees its state object, as PS's detached objects do.
const GRAVEYARD_CAP: usize = 4;

/// A Pokemon: side index (0 for p1) and team slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MonRef {
    pub side: u8,
    pub index: u8,
}

impl MonRef {
    pub const fn new(side: u8, index: u8) -> MonRef {
        MonRef { side, index }
    }
}

/// What holds an effect, or what an event targets: PS's
/// `Pokemon | Side | Field | Battle` (`effectHolder`, `target`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Holder {
    /// No target (PS's `undefined` / `null`).
    #[default]
    None,
    Mon(MonRef),
    Side(u8),
    Field,
    Battle,
}

impl Holder {
    pub fn mon(self) -> Option<MonRef> {
        match self {
            Holder::Mon(m) => Some(m),
            _ => None,
        }
    }
}

/// An active position, PS's `getSlot()` (`p1a`): side and position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlotRef {
    pub side: u8,
    pub position: u8,
}

/// PS's `EffectState`: the data an effect keeps where it is attached.
///
/// PS stores arbitrary keys on it; the ones gens 1 and 3 use map onto these
/// fields by name, and each condition's file documents its own mapping
/// (`this.effectState.time` is `data[0]`, ...). Fields PS leaves undefined
/// are 0 / `None` here; every PS read of them is a truthiness check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EffectState {
    /// `effectOrder`: the creation order, see [`Battle::init_effect_state`].
    pub effect_order: u32,
    /// Identity of this state object (PS compares states with `!==`);
    /// assigned from a battle-wide counter, 0 for scratch states.
    pub serial: u16,
    /// `sourceEffect`.
    pub source_effect: EffectId,
    /// `move` (Encore's, Disable's, Bide's move...), `MoveId::NONE` if unset.
    pub move_: MoveId,
    /// Rarely used extra fields; see the owning condition's doc comment.
    pub data: [i16; 4],
    /// `counter` (Protect's stall counter, Toxic's stage...).
    pub counter: u16,
    /// `duration`, 0 when PS has none.
    pub duration: i8,
    /// `layers` (Spikes).
    pub layers: u8,
    /// `hitCount`.
    pub hit_count: u8,
    /// `trueDuration`.
    pub true_duration: u8,
    /// `source`.
    pub source: Option<MonRef>,
    /// `sourceSlot`.
    pub source_slot: Option<SlotRef>,
    /// `target`: what the state is attached to, as PS last set it.
    pub target: Holder,
    /// `ending` (abilities that are being removed).
    pub ending: bool,
}

impl EffectState {
    /// A blank state: PS's `initEffectState({})`.
    pub const EMPTY: EffectState = EffectState {
        effect_order: 0,
        serial: 0,
        source_effect: EffectId::None,
        move_: MoveId::NONE,
        data: [0; 4],
        counter: 0,
        duration: 0,
        layers: 0,
        hit_count: 0,
        true_duration: 0,
        source: None,
        source_slot: None,
        target: Holder::None,
        ending: false,
    };
}

/// Up to `N` effect states keyed by effect, in INSERTION order, like a JS
/// object's string keys: a removed key's later re-insertion goes last.
/// PS iterates `pokemon.volatiles` and the side and field tables this way,
/// which decides who acts first among exact ties.
#[derive(Clone, Copy, Debug)]
pub struct EffectTable<const N: usize> {
    len: u8,
    ids: [EffectId; N],
    states: [EffectState; N],
}

impl<const N: usize> Default for EffectTable<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> EffectTable<N> {
    pub const fn new() -> Self {
        EffectTable { len: 0, ids: [EffectId::None; N], states: [EffectState::EMPTY; N] }
    }

    pub fn len(&self) -> usize {
        self.len as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn position(&self, id: EffectId) -> Option<usize> {
        self.ids[..self.len()].iter().position(|&k| k == id)
    }

    pub fn contains(&self, id: EffectId) -> bool {
        self.position(id).is_some()
    }

    pub fn get(&self, id: EffectId) -> Option<&EffectState> {
        self.position(id).map(|i| &self.states[i])
    }

    pub fn get_mut(&mut self, id: EffectId) -> Option<&mut EffectState> {
        self.position(id).map(move |i| &mut self.states[i])
    }

    /// `table[id] = state`: replaces in place if present, else appends.
    /// Panics past capacity (a capacity bug, never a silent drop).
    pub fn insert(&mut self, id: EffectId, state: EffectState) {
        if let Some(i) = self.position(id) {
            self.states[i] = state;
            return;
        }
        let n = self.len();
        assert!(n < N, "EffectTable full ({N}) adding {id:?}");
        self.ids[n] = id;
        self.states[n] = state;
        self.len += 1;
    }

    /// `delete table[id]`, keeping the others in order.
    pub fn remove(&mut self, id: EffectId) -> Option<EffectState> {
        let i = self.position(id)?;
        let n = self.len();
        let state = self.states[i];
        self.ids.copy_within(i + 1..n, i);
        self.states.copy_within(i + 1..n, i);
        self.len -= 1;
        Some(state)
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// The `i`th key in insertion order.
    pub fn id_at(&self, i: usize) -> EffectId {
        self.ids[..self.len()][i]
    }

    /// The keys in insertion order.
    pub fn ids(&self) -> &[EffectId] {
        &self.ids[..self.len()]
    }

    /// `(key, state)` in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (EffectId, &EffectState)> {
        self.ids[..self.len()].iter().copied().zip(self.states.iter())
    }
}

/// One move slot (`pokemon.moveSlots[i]`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoveSlot {
    pub id: MoveId,
    pub pp: u8,
    pub maxpp: u8,
    /// `disabled`: 0 `false`, 1 `true`, 2 `'hidden'`.
    pub disabled: u8,
    pub used: bool,
    pub virtual_: bool,
}

impl MoveSlot {
    pub const EMPTY: MoveSlot =
        MoveSlot { id: MoveId::NONE, pp: 0, maxpp: 0, disabled: 0, used: false, virtual_: false };
}

/// One `attackedBy` entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Attacker {
    pub source: MonRef,
    pub damage: u16,
    pub this_turn: bool,
    pub move_: MoveId,
    pub slot: SlotRef,
}

/// A Pokemon: the PS fields mechanics read. Later tasks add what their
/// ports read (the constructor's stat calc, gen 1's registers, ...).
#[derive(Clone, Copy, Debug)]
pub struct Pokemon {
    /// False for an empty team slot.
    pub exists: bool,
    pub species: SpeciesId,
    pub base_species: SpeciesId,
    pub level: u16,
    pub gender: ps_dex::Gender,
    /// `position`: the active slot index (0 in singles).
    pub position: u8,
    pub is_active: bool,
    pub fainted: bool,
    pub faint_queued: bool,
    pub transformed: bool,
    pub hp: u16,
    pub maxhp: u16,
    pub base_maxhp: u16,
    /// `storedStats` (atk, def, spa, spd, spe).
    pub stored_stats: [u16; 5],
    pub base_stored_stats: [u16; 5],
    /// atk, def, spa, spd, spe, accuracy, evasion.
    pub boosts: [i8; 7],
    /// `speed`, refreshed by `updateSpeed`.
    pub speed: i32,
    /// `types`.
    pub types: TypeList,
    /// `status`, `ConditionId::NONE` for PS's `''`.
    pub status: ConditionId,
    pub status_state: EffectState,
    pub volatiles: EffectTable<VOLATILE_CAP>,
    pub ability: AbilityId,
    pub base_ability: AbilityId,
    pub ability_state: EffectState,
    pub item: ItemId,
    pub last_item: ItemId,
    pub item_state: EffectState,
    pub move_slots: [MoveSlot; 4],
    pub base_move_slots: [MoveSlot; 4],
    pub last_move: MoveId,
    pub last_move_used: MoveId,
    pub last_damage: u16,
    pub attacked_by: [Option<Attacker>; 4],
    pub active_turns: u16,
}

impl Pokemon {
    pub const EMPTY: Pokemon = Pokemon {
        exists: false,
        species: SpeciesId::NONE,
        base_species: SpeciesId::NONE,
        level: 0,
        gender: ps_dex::Gender::Neutral,
        position: 0,
        is_active: false,
        fainted: false,
        faint_queued: false,
        transformed: false,
        hp: 0,
        maxhp: 0,
        base_maxhp: 0,
        stored_stats: [0; 5],
        base_stored_stats: [0; 5],
        boosts: [0; 7],
        speed: 0,
        types: TypeList([Type::None, Type::None]),
        status: ConditionId::NONE,
        status_state: EffectState::EMPTY,
        volatiles: EffectTable::new(),
        ability: AbilityId::NONE,
        base_ability: AbilityId::NONE,
        ability_state: EffectState::EMPTY,
        item: ItemId::NONE,
        last_item: ItemId::NONE,
        item_state: EffectState::EMPTY,
        move_slots: [MoveSlot::EMPTY; 4],
        base_move_slots: [MoveSlot::EMPTY; 4],
        last_move: MoveId::NONE,
        last_move_used: MoveId::NONE,
        last_damage: 0,
        attacked_by: [None; 4],
        active_turns: 0,
    };

    /// `getStatus()` as an effect.
    pub fn status_effect(&self) -> EffectId {
        if self.status.is_none() {
            EffectId::None
        } else {
            EffectId::Condition(self.status)
        }
    }

    pub fn ability_effect(&self) -> EffectId {
        if self.ability.is_none() {
            EffectId::None
        } else {
            EffectId::Ability(self.ability)
        }
    }

    pub fn item_effect(&self) -> EffectId {
        if self.item.is_none() {
            EffectId::None
        } else {
            EffectId::Item(self.item)
        }
    }
}

/// A side (`p1` / `p2`).
#[derive(Clone, Copy, Debug)]
pub struct Side {
    /// `n`: 0 for p1.
    pub n: u8,
    pub mons: [Pokemon; TEAM_CAP],
    /// `active[i]`: the team slot in active position `i`.
    pub active: [Option<u8>; 1],
    pub side_conditions: EffectTable<SIDE_CONDITION_CAP>,
    /// `slotConditions[position]`.
    pub slot_conditions: [EffectTable<SLOT_CONDITION_CAP>; 1],
}

impl Side {
    pub const fn new(n: u8) -> Side {
        Side {
            n,
            mons: [Pokemon::EMPTY; TEAM_CAP],
            active: [None],
            side_conditions: EffectTable::new(),
            slot_conditions: [EffectTable::new()],
        }
    }

    pub fn id(&self) -> SideId {
        if self.n == 0 {
            SideId::P1
        } else {
            SideId::P2
        }
    }
}

/// The field: weather and pseudo-weathers. Gens 1 and 3 have no terrain,
/// so there is no terrain slot (PS's `getTerrain()` is always empty there).
#[derive(Clone, Copy, Debug)]
pub struct Field {
    /// `weather`, `ConditionId::NONE` for `''`.
    pub weather: ConditionId,
    pub weather_state: EffectState,
    pub pseudo_weather: EffectTable<PSEUDO_WEATHER_CAP>,
}

impl Field {
    pub const fn new() -> Field {
        Field {
            weather: ConditionId::NONE,
            weather_state: EffectState::EMPTY,
            pseudo_weather: EffectTable::new(),
        }
    }

    pub fn weather_effect(&self) -> EffectId {
        if self.weather.is_none() {
            EffectId::None
        } else {
            EffectId::Condition(self.weather)
        }
    }
}

impl Default for Field {
    fn default() -> Self {
        Field::new()
    }
}

/// Where an [`EffectState`] lives (PS passes the object itself).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateRef {
    /// A fresh `initEffectState({})` for one handler call, kept per event
    /// depth in [`Battle`].
    Scratch,
    Status(MonRef),
    Volatile(MonRef, EffectId),
    Ability(MonRef),
    Item(MonRef),
    /// `speciesState`. No gen 1 or gen 3 species has handlers, so it is a
    /// scratch state here (no storage per Pokemon).
    Species(MonRef),
    SideCondition(u8, EffectId),
    SlotCondition(u8, u8, EffectId),
    PseudoWeather(EffectId),
    Weather,
    FormatData,
}

/// The current move, the fields of PS's `ActiveMove` the event core reads.
/// The move pipelines (P1-8, P1-9) extend it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActiveMove {
    pub id: MoveId,
    /// `ignoreAbility` (Mold Breaker and friends).
    pub ignore_ability: bool,
}

/// PS's `this.event`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EventFrame {
    /// `id`, `None` for PS's top-level `{id: ''}`.
    pub id: Option<EventId>,
    pub target: Holder,
    pub source: Option<MonRef>,
    pub effect: EffectId,
    /// `modifier` in 4096ths; `None` in `singleEvent` frames, which PS
    /// builds without one.
    pub modifier: Option<i32>,
}

impl EventFrame {
    pub const TOP: EventFrame = EventFrame {
        id: None,
        target: Holder::None,
        source: None,
        effect: EffectId::None,
        modifier: None,
    };
}

/// `this.effect` and `this.effectState` while a handler runs. The holder
/// (`this.effectState.target`) is the state's own [`EffectState::target`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EffectFrame {
    pub effect: EffectId,
    pub state: StateRef,
    /// The state's identity when the handler was gathered.
    pub serial: u16,
}

impl EffectFrame {
    pub const TOP: EffectFrame = EffectFrame { effect: EffectId::None, state: StateRef::Scratch, serial: 0 };
}

/// The battle log between flushes.
#[derive(Clone, Copy, Debug)]
pub struct Log {
    len: u16,
    msgs: [Msg; LOG_CAP],
}

impl Log {
    pub const fn new() -> Log {
        Log { len: 0, msgs: [Msg::Spacer; LOG_CAP] }
    }

    pub fn push(&mut self, m: Msg) {
        let n = self.len as usize;
        assert!(n < LOG_CAP, "battle log full ({LOG_CAP} lines)");
        self.msgs[n] = m;
        self.len += 1;
    }

    pub fn as_slice(&self) -> &[Msg] {
        &self.msgs[..self.len as usize]
    }

    pub fn as_mut_slice(&mut self) -> &mut [Msg] {
        &mut self.msgs[..self.len as usize]
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }
}

impl Default for Log {
    fn default() -> Self {
        Log::new()
    }
}

/// Dex entries PS's core names by id (`volatiles['gastroacid']`,
/// `hasItem('Ability Shield')`), resolved once per battle; `NONE` where
/// the gen lacks them.
#[derive(Clone, Copy, Debug)]
pub struct KnownIds {
    pub gastroacid: ConditionId,
    pub embargo: ConditionId,
    pub magicroom: ConditionId,
    pub commanding: ConditionId,
    pub nightmare_condition: ConditionId,
    pub nightmare_move: MoveId,
    pub neutralizinggas: AbilityId,
    pub klutz: AbilityId,
    pub corrosion: AbilityId,
    pub abilityshield: ItemId,
}

impl KnownIds {
    pub fn resolve(dex: &Dex) -> KnownIds {
        let c = |k: &str| dex.condition_id(k).unwrap_or(ConditionId::NONE);
        let a = |k: &str| dex.ability_id(k).unwrap_or(AbilityId::NONE);
        KnownIds {
            gastroacid: c("gastroacid"),
            embargo: c("embargo"),
            magicroom: c("magicroom"),
            commanding: c("commanding"),
            nightmare_condition: c("nightmare"),
            nightmare_move: dex.move_id("nightmare").unwrap_or(MoveId::NONE),
            neutralizinggas: a("neutralizinggas"),
            klutz: a("klutz"),
            corrosion: a("corrosion"),
            abilityshield: dex.item_id("abilityshield").unwrap_or(ItemId::NONE),
        }
    }
}

/// The battle. Design 5.1; the queue, requests and turn loop arrive with
/// P1-6.
pub struct Battle {
    pub dex: &'static Dex,
    pub gen: u8,
    /// The gen's effect definitions.
    pub registry: &'static Registry,
    /// The format's `trunc`.
    pub trunc: Trunc,
    pub prng: Prng,
    pub field: Field,
    pub sides: [Side; 2],
    pub turn: u16,
    pub ended: bool,
    /// The format, as an effect (`this.format`).
    pub format: EffectId,
    /// `formatData`.
    pub format_data: EffectState,
    /// `this.effectOrder`: the next effect order to hand out.
    pub effect_order: u32,
    /// `this.effect` / `this.effectState`.
    pub frame: EffectFrame,
    /// `this.event`.
    pub event: EventFrame,
    /// `this.eventDepth`.
    pub event_depth: u8,
    /// `activeMove`, `activePokemon`, `activeTarget`.
    pub active_move: Option<ActiveMove>,
    pub active_pokemon: Option<MonRef>,
    pub active_target: Option<MonRef>,
    /// `speedOrder`: field position values sorted by speed.
    pub speed_order: [u8; 2],
    pub log: Log,
    /// `lastMoveLine`: the index in [`Log`] of the last `|move|`.
    pub last_move_line: Option<u16>,
    pub known: KnownIds,
    /// One `initEffectState({})` per event depth.
    pub(crate) scratch: [EffectState; MAX_EVENT_DEPTH + 1],
    /// Recently removed states, by serial.
    pub(crate) graveyard: [EffectState; GRAVEYARD_CAP],
    pub(crate) graveyard_next: u8,
    pub(crate) next_serial: u16,
}

impl Battle {
    /// An empty battle over `dex` with the effects of `registry`. P1-6
    /// builds the full constructor (format, rules, players) on top.
    pub fn new(dex: &'static Dex, registry: &'static Registry, seed: Seed, trunc: Trunc) -> Battle {
        let mut b = Battle {
            dex,
            gen: dex.gen,
            registry,
            trunc,
            prng: Prng::new(seed),
            field: Field::new(),
            sides: [Side::new(0), Side::new(1)],
            turn: 0,
            ended: false,
            format: EffectId::Rule(Rule::Format),
            format_data: EffectState::EMPTY,
            effect_order: 0,
            frame: EffectFrame::TOP,
            event: EventFrame::TOP,
            event_depth: 0,
            active_move: None,
            active_pokemon: None,
            active_target: None,
            speed_order: [0, 1],
            log: Log::new(),
            last_move_line: None,
            known: KnownIds::resolve(dex),
            scratch: [EffectState::EMPTY; MAX_EVENT_DEPTH + 1],
            graveyard: [EffectState::EMPTY; GRAVEYARD_CAP],
            graveyard_next: 0,
            next_serial: 1,
        };
        // PS: `this.formatData = this.initEffectState({ id: format.id })`
        // (no target, so effectOrder 0).
        b.format_data = b.init_effect_state(true, Holder::None);
        b
    }

    // ---- access -----------------------------------------------------------

    pub fn mon(&self, m: MonRef) -> &Pokemon {
        &self.sides[m.side as usize].mons[m.index as usize]
    }

    pub fn mon_mut(&mut self, m: MonRef) -> &mut Pokemon {
        &mut self.sides[m.side as usize].mons[m.index as usize]
    }

    /// The Pokemon in `side`'s active position `pos`.
    pub fn active_at(&self, side: u8, pos: u8) -> Option<MonRef> {
        self.sides[side as usize].active[pos as usize].map(|i| MonRef::new(side, i))
    }

    /// PS `getAllActive(includeFainted)`: the active Pokemon, p1 first,
    /// fainted ones only when asked.
    pub fn all_active(&self, include_fainted: bool) -> ([MonRef; 2], usize) {
        let mut out = [MonRef::new(0, 0); 2];
        let mut n = 0;
        for s in 0..2u8 {
            for pos in 0..self.sides[s as usize].active.len() as u8 {
                if let Some(m) = self.active_at(s, pos) {
                    if include_fainted || !self.mon(m).fainted {
                        out[n] = m;
                        n += 1;
                    }
                }
            }
        }
        (out, n)
    }

    /// PS `side.allies()`: the side's active Pokemon with HP.
    pub fn allies(&self, side: u8) -> ([MonRef; 1], usize) {
        let mut out = [MonRef::new(side, 0); 1];
        let mut n = 0;
        for pos in 0..self.sides[side as usize].active.len() as u8 {
            if let Some(m) = self.active_at(side, pos) {
                if self.mon(m).hp != 0 {
                    out[n] = m;
                    n += 1;
                }
            }
        }
        (out, n)
    }

    /// PS `getSlot()`.
    pub fn slot_of(&self, m: MonRef) -> SlotRef {
        SlotRef { side: m.side, position: self.mon(m).position }
    }

    /// PS `getFieldPositionValue()`: `side.n + sides.length * position`.
    pub fn field_position_value(&self, m: MonRef) -> u8 {
        m.side + 2 * self.mon(m).position
    }

    /// The protocol identity of a Pokemon (`p1a: NAME` when active).
    pub fn ident(&self, m: MonRef) -> PokemonIdent {
        let side = if m.side == 0 { SideId::P1 } else { SideId::P2 };
        if self.mon(m).is_active {
            PokemonIdent::active(side, m.index)
        } else {
            PokemonIdent::inactive(side, m.index)
        }
    }

    pub fn effect_type(&self, id: EffectId) -> EffectType {
        EffectType::of(self.dex, id)
    }

    /// PS `this.add(...)`.
    pub fn add(&mut self, m: Msg) {
        self.log.push(m);
    }

    /// PS `attrLastMove('[still]')`: no animation, so the move's target is
    /// hidden; an `|-anim|` line is dropped instead.
    // PS: sim/battle.ts attrLastMove
    pub fn attr_last_move_still(&mut self) {
        let Some(i) = self.last_move_line else { return };
        let i = i as usize;
        match &mut self.log.as_mut_slice()[i] {
            Msg::Anim { .. } => {
                let n = self.log.len as usize;
                self.log.msgs.copy_within(i + 1..n, i);
                self.log.len -= 1;
                self.last_move_line = None;
            }
            Msg::Move { target, kw, .. } => {
                *target = None;
                kw.flags.0 |= ps_protocol::KwFlags::STILL.0;
            }
            _ => {}
        }
    }

    // ---- effect states ----------------------------------------------------

    fn take_serial(&mut self) -> u16 {
        let s = self.next_serial;
        self.next_serial = self.next_serial.wrapping_add(1).max(1);
        s
    }

    /// PS `initEffectState(obj)`: a new state for an effect with an id
    /// (`has_id`) attached to `target`. It gets the next `effectOrder` only
    /// when it has an id and a target, and the target is not an inactive
    /// Pokemon; otherwise its order is 0.
    // PS: sim/battle.ts initEffectState
    pub fn init_effect_state(&mut self, has_id: bool, target: Holder) -> EffectState {
        let mut st = EffectState::EMPTY;
        st.serial = self.take_serial();
        st.target = target;
        let counts = match target {
            Holder::None => false,
            Holder::Mon(m) => self.mon(m).is_active,
            _ => true,
        };
        if has_id && counts {
            st.effect_order = self.effect_order;
            self.effect_order += 1;
        }
        st
    }

    fn bury(&mut self, st: EffectState) {
        let i = self.graveyard_next as usize;
        self.graveyard[i] = st;
        self.graveyard_next = ((i + 1) % GRAVEYARD_CAP) as u8;
    }

    fn live_state(&self, r: StateRef) -> Option<&EffectState> {
        match r {
            StateRef::Scratch | StateRef::Species(_) => None,
            StateRef::Status(m) => Some(&self.mon(m).status_state),
            StateRef::Volatile(m, id) => self.mon(m).volatiles.get(id),
            StateRef::Ability(m) => Some(&self.mon(m).ability_state),
            StateRef::Item(m) => Some(&self.mon(m).item_state),
            StateRef::SideCondition(s, id) => self.sides[s as usize].side_conditions.get(id),
            StateRef::SlotCondition(s, p, id) => {
                self.sides[s as usize].slot_conditions[p as usize].get(id)
            }
            StateRef::PseudoWeather(id) => self.field.pseudo_weather.get(id),
            StateRef::Weather => Some(&self.field.weather_state),
            StateRef::FormatData => Some(&self.format_data),
        }
    }

    fn live_state_mut(&mut self, r: StateRef) -> Option<&mut EffectState> {
        match r {
            StateRef::Scratch | StateRef::Species(_) => None,
            StateRef::Status(m) => Some(&mut self.mon_mut(m).status_state),
            StateRef::Volatile(m, id) => self.mon_mut(m).volatiles.get_mut(id),
            StateRef::Ability(m) => Some(&mut self.mon_mut(m).ability_state),
            StateRef::Item(m) => Some(&mut self.mon_mut(m).item_state),
            StateRef::SideCondition(s, id) => self.sides[s as usize].side_conditions.get_mut(id),
            StateRef::SlotCondition(s, p, id) => {
                self.sides[s as usize].slot_conditions[p as usize].get_mut(id)
            }
            StateRef::PseudoWeather(id) => self.field.pseudo_weather.get_mut(id),
            StateRef::Weather => Some(&mut self.field.weather_state),
            StateRef::FormatData => Some(&mut self.format_data),
        }
    }

    /// The state at `r` if it is still the object with identity `serial`.
    pub fn state_at(&self, r: StateRef, serial: u16) -> Option<&EffectState> {
        self.live_state(r).filter(|s| s.serial == serial)
    }

    /// The serial of the state currently at `r` (0 for scratch).
    pub fn serial_at(&self, r: StateRef) -> u16 {
        self.live_state(r).map_or(0, |s| s.serial)
    }

    /// The state object `(r, serial)` names: where it lives, or, if it was
    /// removed or replaced since, its detached copy (PS keeps the object
    /// alive for whoever still holds it).
    pub fn resolve_state_mut(&mut self, r: StateRef, serial: u16) -> &mut EffectState {
        let depth = self.event_depth as usize;
        if serial == 0 || matches!(r, StateRef::Scratch | StateRef::Species(_)) {
            return &mut self.scratch[depth];
        }
        if self.live_state(r).is_some_and(|s| s.serial == serial) {
            return self.live_state_mut(r).unwrap();
        }
        if let Some(i) = self.graveyard.iter().position(|s| s.serial == serial) {
            return &mut self.graveyard[i];
        }
        // Gone for good: hand out a scratch copy rather than another
        // effect's state.
        &mut self.scratch[depth]
    }

    /// `this.effectState` of the running handler.
    pub fn effect_state(&mut self) -> &mut EffectState {
        let f = self.frame;
        self.resolve_state_mut(f.state, f.serial)
    }

    // ---- pokemon helpers --------------------------------------------------

    /// PS `getStatus()`.
    pub fn get_status(&self, m: MonRef) -> EffectId {
        self.mon(m).status_effect()
    }

    /// PS `getTypes()` (no Terastallization or added types in gens 1-3):
    /// the `Type` event over `this.types`.
    // PS: sim/pokemon.ts getTypes
    pub fn get_types(&mut self, m: MonRef) -> TypeList {
        let types = self.mon(m).types;
        match self.run_event(EventId::Type, Holder::Mon(m), None, EffectId::None, Some(Relay::Types(types))) {
            Relay::Types(t) => t,
            // An empty array becomes `['???']` before gen 5; nothing in
            // gens 1-3 returns anything but a types array.
            other => panic!("Type event returned {other:?}"),
        }
    }

    /// PS `hasItem(id)`.
    pub fn has_item(&mut self, m: MonRef, item: ItemId) -> bool {
        if item.is_none() || self.mon(m).item != item {
            return false;
        }
        !self.ignoring_item(m)
    }

    /// PS `hasAbility(id)`.
    pub fn has_ability(&mut self, m: MonRef, ability: AbilityId) -> bool {
        if ability.is_none() || self.mon(m).ability != ability {
            return false;
        }
        !self.ignoring_ability(m)
    }

    fn ability_flag(&self, a: AbilityId, flag: &str) -> bool {
        !a.is_none() && self.dex.ability_def(a).flags.has(flag)
    }

    fn has_volatile(&self, m: MonRef, c: ConditionId) -> bool {
        !c.is_none() && self.mon(m).volatiles.contains(EffectId::Condition(c))
    }

    /// PS `ignoringAbility()`.
    // PS: sim/pokemon.ts ignoringAbility
    pub fn ignoring_ability(&mut self, m: MonRef) -> bool {
        let mon = self.mon(m);
        if self.gen >= 5 && !mon.is_active {
            return true;
        }
        let ability = mon.ability;
        if self.ability_flag(ability, "notransform") && mon.transformed {
            return true;
        }
        if self.ability_flag(ability, "cantsuppress") {
            return false;
        }
        if self.has_volatile(m, self.known.gastroacid) {
            return true;
        }
        let shield = self.known.abilityshield;
        if (!shield.is_none() && self.has_item(m, shield))
            || (!ability.is_none() && ability == self.known.neutralizinggas)
        {
            return false;
        }
        let gas = self.known.neutralizinggas;
        if gas.is_none() {
            return false;
        }
        let (actives, n) = self.all_active(false);
        for &p in &actives[..n] {
            let pm = self.mon(p);
            if pm.ability == gas
                && !self.has_volatile(p, self.known.gastroacid)
                && !pm.transformed
                && !pm.ability_state.ending
                && !self.has_volatile(m, self.known.commanding)
            {
                return true;
            }
        }
        false
    }

    /// PS `ignoringItem()` (`isFling` false; no item in gens 1-3 is a
    /// Primal Orb).
    // PS: sim/pokemon.ts ignoringItem
    pub fn ignoring_item(&mut self, m: MonRef) -> bool {
        if self.gen >= 5 && !self.mon(m).is_active {
            return true;
        }
        if self.has_volatile(m, self.known.embargo)
            || (!self.known.magicroom.is_none()
                && self.field.pseudo_weather.contains(EffectId::Condition(self.known.magicroom)))
        {
            return true;
        }
        let item = self.mon(m).item;
        let ignore_klutz = !item.is_none() && self.dex.item_def(item).ignore_klutz;
        let klutz = self.known.klutz;
        !ignore_klutz && !klutz.is_none() && self.has_ability(m, klutz)
    }

    /// PS `suppressingAbility(target)`: a Mold Breaker-style move is being
    /// used against `target`.
    // PS: sim/battle.ts suppressingAbility
    pub fn suppressing_ability(&mut self, target: Option<MonRef>) -> bool {
        let Some(t) = target else { return false };
        let Some(am) = self.active_move else { return false };
        let Some(ap) = self.active_pokemon else { return false };
        let shield = self.known.abilityshield;
        self.mon(ap).is_active
            && (ap != t || self.gen < 8)
            && am.ignore_ability
            && !(!shield.is_none() && self.has_item(t, shield))
    }

    /// PS `field.suppressingWeather()`.
    // PS: sim/field.ts suppressingWeather
    pub fn suppressing_weather(&mut self) -> bool {
        for s in 0..2u8 {
            for pos in 0..self.sides[s as usize].active.len() as u8 {
                let Some(p) = self.active_at(s, pos) else { continue };
                let mon = self.mon(p);
                if mon.fainted {
                    continue;
                }
                let ability = mon.ability;
                let ending = mon.ability_state.ending;
                if !self.ignoring_ability(p)
                    && !ability.is_none()
                    && self.dex.ability_def(ability).suppress_weather
                    && !ending
                {
                    return true;
                }
            }
        }
        false
    }

    /// PS `runStatusImmunity(type, message)`; `id` is a condition's key
    /// (`psn`, `confusion`) or empty.
    // PS: sim/pokemon.ts runStatusImmunity
    pub fn run_status_immunity(&mut self, m: MonRef, id: ConditionId, message: bool) -> bool {
        if self.mon(m).fainted {
            return false;
        }
        if id.is_none() {
            return true;
        }
        let key = self.dex.condition_def(id).key.as_str();
        // PS `dex.getImmunity(type, pokemon)`: no type of the Pokemon has
        // `damageTaken[type] === 3`.
        let types = self.get_types(m);
        if types.as_slice().iter().any(|&t| self.dex.typechart.special_code(key, t) == 3) {
            if message {
                let mon = self.ident(m);
                self.add(Msg::Immune { mon, kw: Kw::default() });
            }
            return false;
        }
        let immunity = self.run_event(
            EventId::Immunity,
            Holder::Mon(m),
            None,
            EffectId::None,
            Some(Relay::Effect(EffectId::Condition(id))),
        );
        if !immunity.truthy() {
            if message && immunity != Relay::Null {
                let mon = self.ident(m);
                self.add(Msg::Immune { mon, kw: Kw::default() });
            }
            return false;
        }
        true
    }

    /// `(sourceEffect as Move)?.status`: the status a move inflicts.
    fn move_status(&self, effect: EffectId) -> ConditionId {
        match effect {
            EffectId::Move(mv) => self.dex.move_def(mv).status,
            _ => ConditionId::NONE,
        }
    }

    /// A condition's `durationCallback`, called directly as PS does
    /// (`status.durationCallback.call(this.battle, target, source, effect)`).
    fn duration_callback(
        &mut self,
        status: ConditionId,
        target: Holder,
        source: Option<MonRef>,
        effect: EffectId,
    ) -> Option<i8> {
        let h = self.registry.handler(EffectId::Condition(status), Prefix::None, EventId::DurationCallback)?;
        let args = EventArgs { id: EventId::DurationCallback, relay: None, target, source, effect };
        let r = match h.callback {
            crate::effect::Callback::Fn(f) => f(self, &args),
            crate::effect::Callback::Const(v) => Some(v),
        };
        match r {
            Some(Relay::Int(n)) => Some(n as i8),
            other => panic!("durationCallback returned {other:?}"),
        }
    }

    /// Defaults PS applies to `source` / `sourceEffect` from the running
    /// event (`if (this.battle.event)`, always true): the event's source
    /// and `this.effect`.
    fn default_source(&self, source: Option<MonRef>, effect: EffectId) -> (Option<MonRef>, EffectId) {
        let source = source.or(self.event.source);
        let effect = if effect.is_none() { self.frame.effect } else { effect };
        (source, effect)
    }

    /// PS `addVolatile(status, source, sourceEffect)` (no `linkedStatus`:
    /// nothing in gens 1-3 links volatiles). Returns PS's return value.
    // PS: sim/pokemon.ts addVolatile
    pub fn add_volatile(
        &mut self,
        m: MonRef,
        status: ConditionId,
        source: Option<MonRef>,
        source_effect: EffectId,
    ) -> Relay {
        let def = self.dex.condition_def(status);
        if self.mon(m).hp == 0 && !def.affects_fainted {
            return Relay::FAIL;
        }
        let (source, source_effect) = self.default_source(source, source_effect);
        let source = source.unwrap_or(m);
        let key = EffectId::Condition(status);
        if let Some(st) = self.mon(m).volatiles.get(key) {
            let serial = st.serial;
            if self.registry.handler(key, Prefix::None, EventId::Restart).is_none() {
                return Relay::FAIL;
            }
            return self.single_event(
                EventId::Restart,
                key,
                StateRef::Volatile(m, key),
                serial,
                Holder::Mon(m),
                Some(source),
                source_effect,
                None,
            );
        }
        if !self.run_status_immunity(m, status, false) {
            if !self.move_status(source_effect).is_none() {
                let mon = self.ident(m);
                self.add(Msg::Immune { mon, kw: Kw::default() });
            }
            return Relay::FAIL;
        }
        let result = self.run_event(
            EventId::TryAddVolatile,
            Holder::Mon(m),
            Some(source),
            source_effect,
            Some(Relay::Effect(key)),
        );
        if !result.truthy() {
            return result;
        }
        let mut st = self.init_effect_state(true, Holder::Mon(m));
        st.source = Some(source);
        st.source_slot = Some(self.slot_of(source));
        st.source_effect = source_effect;
        if let Some(d) = def.duration {
            st.duration = d as i8;
        }
        self.mon_mut(m).volatiles.insert(key, st);
        if let Some(d) = self.duration_callback(status, Holder::Mon(m), Some(source), source_effect) {
            if let Some(s) = self.mon_mut(m).volatiles.get_mut(key) {
                s.duration = d;
            }
        }
        let serial = st.serial;
        let result = self.single_event(
            EventId::Start,
            key,
            StateRef::Volatile(m, key),
            serial,
            Holder::Mon(m),
            Some(source),
            source_effect,
            None,
        );
        if !result.truthy() {
            if let Some(gone) = self.mon_mut(m).volatiles.remove(key) {
                self.bury(gone);
            }
            return result;
        }
        Relay::TRUE
    }

    /// PS `removeVolatile(status)`.
    // PS: sim/pokemon.ts removeVolatile
    pub fn remove_volatile(&mut self, m: MonRef, status: ConditionId) -> bool {
        if self.mon(m).hp == 0 {
            return false;
        }
        let key = EffectId::Condition(status);
        let Some(st) = self.mon(m).volatiles.get(key) else { return false };
        let serial = st.serial;
        self.single_event(EventId::End, key, StateRef::Volatile(m, key), serial, Holder::Mon(m), None, EffectId::None, None);
        // `delete this.volatiles[id]` removes the key whatever End did; if
        // End re-added it, that newer state is what goes.
        if let Some(gone) = self.mon_mut(m).volatiles.remove(key) {
            self.bury(gone);
        }
        true
    }

    /// PS `setStatus(status, source, sourceEffect, ignoreImmunities)`;
    /// `ConditionId::NONE` is `''`. Returns PS's return value.
    // PS: sim/pokemon.ts setStatus
    pub fn set_status(
        &mut self,
        m: MonRef,
        status: ConditionId,
        source: Option<MonRef>,
        source_effect: EffectId,
        ignore_immunities: bool,
    ) -> Relay {
        if self.mon(m).hp == 0 {
            return Relay::FAIL;
        }
        let (source, source_effect) = self.default_source(source, source_effect);
        let source = source.unwrap_or(m);

        let cur = self.mon(m).status;
        if cur == status {
            let effect_status = self.move_status(source_effect);
            if !effect_status.is_none() && effect_status == cur {
                let mon = self.ident(m);
                let what = self.status_arg(cur);
                self.add(Msg::Fail { mon, what, what2: None, kw: Kw::default() });
            } else if !effect_status.is_none() {
                let mon = self.ident(source);
                self.add(Msg::Fail { mon, what: None, what2: None, kw: Kw::default() });
                self.attr_last_move_still();
            }
            return Relay::FAIL;
        }

        if !ignore_immunities && !status.is_none() {
            let key = self.dex.condition_def(status).key.as_str();
            let corrosion = self.known.corrosion;
            let corroded = !corrosion.is_none()
                && self.has_ability(source, corrosion)
                && (key == "tox" || key == "psn");
            if !corroded {
                let immunity_id = if key == "tox" {
                    self.dex.condition_id("psn").unwrap_or(status)
                } else {
                    status
                };
                if !self.run_status_immunity(m, immunity_id, false) {
                    if !self.move_status(source_effect).is_none() {
                        let mon = self.ident(m);
                        self.add(Msg::Immune { mon, kw: Kw::default() });
                    }
                    return Relay::FAIL;
                }
            }
        }
        let prev_status = cur;
        let prev_state = self.mon(m).status_state;
        let key = if status.is_none() { EffectId::None } else { EffectId::Condition(status) };
        if !status.is_none() {
            let result =
                self.run_event(EventId::SetStatus, Holder::Mon(m), Some(source), source_effect, Some(Relay::Effect(key)));
            if !result.truthy() {
                return result;
            }
        }

        let mut st = self.init_effect_state(!status.is_none(), Holder::Mon(m));
        st.source = Some(source);
        if !status.is_none() {
            if let Some(d) = self.dex.condition_def(status).duration {
                st.duration = d as i8;
            }
        }
        {
            let mon = self.mon_mut(m);
            mon.status = status;
            mon.status_state = st;
        }
        if prev_state.serial != 0 {
            self.bury(prev_state);
        }
        if !status.is_none() {
            if let Some(d) = self.duration_callback(status, Holder::Mon(m), Some(source), source_effect) {
                self.mon_mut(m).status_state.duration = d;
            }
        }

        if !status.is_none() {
            let serial = self.mon(m).status_state.serial;
            let started = self.single_event(
                EventId::Start,
                key,
                StateRef::Status(m),
                serial,
                Holder::Mon(m),
                Some(source),
                source_effect,
                None,
            );
            if !started.truthy() {
                let mon = self.mon_mut(m);
                mon.status = prev_status;
                mon.status_state = prev_state;
                return Relay::FAIL;
            }
            let after = self.run_event(
                EventId::AfterSetStatus,
                Holder::Mon(m),
                Some(source),
                source_effect,
                Some(Relay::Effect(key)),
            );
            if !after.truthy() {
                return Relay::FAIL;
            }
        }
        Relay::TRUE
    }

    /// The `-fail` / `-status` argument for a status condition.
    fn status_arg(&self, status: ConditionId) -> Option<Arg> {
        let s = Status::parse(self.dex.condition_def(status).key.as_str())?;
        Some(Arg::new(ArgValue::Status(s)))
    }

    /// PS `clearStatus()`: no cure message.
    // PS: sim/pokemon.ts clearStatus
    pub fn clear_status(&mut self, m: MonRef) -> bool {
        let mon = self.mon(m);
        if mon.hp == 0 || mon.status.is_none() {
            return false;
        }
        let is_slp = self.dex.condition_def(mon.status).key.as_str() == "slp";
        let nightmare = self.known.nightmare_condition;
        if is_slp && !nightmare.is_none() && self.remove_volatile(m, nightmare) {
            let mon = self.ident(m);
            let effect = Arg::new(ArgValue::Move(MoveNum(self.known.nightmare_move.0)));
            let kw = Kw { flags: ps_protocol::KwFlags::SILENT, ..Kw::default() };
            self.add(Msg::EndEffect { mon, effect, kw });
        }
        self.set_status(m, ConditionId::NONE, None, EffectId::None, false);
        true
    }

    // ---- side and slot conditions -------------------------------------

    /// PS `side.addSideCondition(status, source, sourceEffect)`.
    // PS: sim/side.ts addSideCondition
    pub fn add_side_condition(
        &mut self,
        side: u8,
        status: ConditionId,
        source: Option<MonRef>,
        source_effect: EffectId,
    ) -> Relay {
        // `source ||= event.target`; a Side stands for its first active.
        let source = source
            .or(match self.event.target {
                Holder::Mon(m) => Some(m),
                Holder::Side(s) => self.active_at(s, 0),
                _ => None,
            })
            .expect("setting sidecond without a source");
        let key = EffectId::Condition(status);
        if let Some(st) = self.sides[side as usize].side_conditions.get(key) {
            let serial = st.serial;
            if self.registry.handler(key, Prefix::None, EventId::SideRestart).is_none() {
                return Relay::FAIL;
            }
            return self.single_event(
                EventId::SideRestart,
                key,
                StateRef::SideCondition(side, key),
                serial,
                Holder::Side(side),
                Some(source),
                source_effect,
                None,
            );
        }
        let mut st = self.init_effect_state(true, Holder::Side(side));
        st.source = Some(source);
        st.source_slot = Some(self.slot_of(source));
        if let Some(d) = self.dex.condition_def(status).duration {
            st.duration = d as i8;
        }
        self.sides[side as usize].side_conditions.insert(key, st);
        let first = self.active_at(side, 0).map_or(Holder::None, Holder::Mon);
        if let Some(d) = self.duration_callback(status, first, Some(source), source_effect) {
            if let Some(s) = self.sides[side as usize].side_conditions.get_mut(key) {
                s.duration = d;
            }
        }
        let started = self.single_event(
            EventId::SideStart,
            key,
            StateRef::SideCondition(side, key),
            st.serial,
            Holder::Side(side),
            Some(source),
            source_effect,
            None,
        );
        if !started.truthy() {
            if let Some(gone) = self.sides[side as usize].side_conditions.remove(key) {
                self.bury(gone);
            }
            return Relay::FAIL;
        }
        self.run_event(EventId::SideConditionStart, Holder::Side(side), Some(source), key, None);
        Relay::TRUE
    }

    /// PS `side.removeSideCondition(status)`.
    // PS: sim/side.ts removeSideCondition
    pub fn remove_side_condition(&mut self, side: u8, status: ConditionId) -> bool {
        let key = EffectId::Condition(status);
        let Some(st) = self.sides[side as usize].side_conditions.get(key) else { return false };
        let serial = st.serial;
        self.single_event(
            EventId::SideEnd,
            key,
            StateRef::SideCondition(side, key),
            serial,
            Holder::Side(side),
            None,
            EffectId::None,
            None,
        );
        if let Some(gone) = self.sides[side as usize].side_conditions.remove(key) {
            self.bury(gone);
        }
        true
    }

    /// PS `side.addSlotCondition(target, status, source, sourceEffect)`.
    // PS: sim/side.ts addSlotCondition
    pub fn add_slot_condition(
        &mut self,
        side: u8,
        position: u8,
        status: ConditionId,
        source: Option<MonRef>,
        source_effect: EffectId,
    ) -> Relay {
        let source = source.or(self.event.target.mon()).expect("setting sidecond without a source");
        let key = EffectId::Condition(status);
        let target_mon = self.active_at(side, position).map_or(Holder::None, Holder::Mon);
        let r = StateRef::SlotCondition(side, position, key);
        if let Some(st) = self.sides[side as usize].slot_conditions[position as usize].get(key) {
            let serial = st.serial;
            if self.registry.handler(key, Prefix::None, EventId::Restart).is_none() {
                return Relay::FAIL;
            }
            return self.single_event(EventId::Restart, key, r, serial, Holder::Side(side), Some(source), source_effect, None);
        }
        let mut st = self.init_effect_state(true, Holder::Side(side));
        st.source = Some(source);
        st.source_slot = Some(self.slot_of(source));
        if let Some(d) = self.dex.condition_def(status).duration {
            st.duration = d as i8;
        }
        self.sides[side as usize].slot_conditions[position as usize].insert(key, st);
        let first = self.active_at(side, 0).map_or(Holder::None, Holder::Mon);
        if let Some(d) = self.duration_callback(status, first, Some(source), source_effect) {
            if let Some(s) = self.sides[side as usize].slot_conditions[position as usize].get_mut(key) {
                s.duration = d;
            }
        }
        let started = self.single_event(EventId::Start, key, r, st.serial, target_mon, Some(source), source_effect, None);
        if !started.truthy() {
            if let Some(gone) = self.sides[side as usize].slot_conditions[position as usize].remove(key) {
                self.bury(gone);
            }
            return Relay::FAIL;
        }
        Relay::TRUE
    }

    /// PS `side.removeSlotCondition(target, status)`.
    // PS: sim/side.ts removeSlotCondition
    pub fn remove_slot_condition(&mut self, side: u8, position: u8, status: ConditionId) -> bool {
        let key = EffectId::Condition(status);
        let Some(st) = self.sides[side as usize].slot_conditions[position as usize].get(key) else {
            return false;
        };
        let serial = st.serial;
        let target_mon = self.active_at(side, position).map_or(Holder::None, Holder::Mon);
        self.single_event(
            EventId::End,
            key,
            StateRef::SlotCondition(side, position, key),
            serial,
            target_mon,
            None,
            EffectId::None,
            None,
        );
        if let Some(gone) = self.sides[side as usize].slot_conditions[position as usize].remove(key) {
            self.bury(gone);
        }
        true
    }

    // ---- field --------------------------------------------------------

    /// PS `field.addPseudoWeather(status, source, sourceEffect)`. `status`
    /// is a condition or a format rule (the `Battle` constructor adds every
    /// rule with handlers this way).
    // PS: sim/field.ts addPseudoWeather
    pub fn add_pseudo_weather(&mut self, status: EffectId, source: Option<MonRef>, source_effect: EffectId) -> Relay {
        let source = source.or(self.event.target.mon());
        if let Some(st) = self.field.pseudo_weather.get(status) {
            let serial = st.serial;
            if self.registry.handler(status, Prefix::None, EventId::FieldRestart).is_none() {
                return Relay::FAIL;
            }
            return self.single_event(
                EventId::FieldRestart,
                status,
                StateRef::PseudoWeather(status),
                serial,
                Holder::Field,
                source,
                source_effect,
                None,
            );
        }
        // `initEffectState({id, source, sourceSlot, duration})`: no target,
        // so effectOrder 0.
        let mut st = self.init_effect_state(true, Holder::None);
        st.source = source;
        st.source_slot = source.map(|s| self.slot_of(s));
        if let EffectId::Condition(c) = status {
            if let Some(d) = self.dex.condition_def(c).duration {
                st.duration = d as i8;
            }
        }
        self.field.pseudo_weather.insert(status, st);
        if let EffectId::Condition(c) = status {
            if self.registry.handler(status, Prefix::None, EventId::DurationCallback).is_some() {
                let src = source.expect("setting fieldcond without a source");
                if let Some(d) = self.duration_callback(c, Holder::Mon(src), Some(src), source_effect) {
                    if let Some(s) = self.field.pseudo_weather.get_mut(status) {
                        s.duration = d;
                    }
                }
            }
        }
        let started = self.single_event(
            EventId::FieldStart,
            status,
            StateRef::PseudoWeather(status),
            st.serial,
            Holder::Field,
            source,
            source_effect,
            None,
        );
        if !started.truthy() {
            if let Some(gone) = self.field.pseudo_weather.remove(status) {
                self.bury(gone);
            }
            return Relay::FAIL;
        }
        let target = source.map_or(Holder::None, Holder::Mon);
        self.run_event(EventId::PseudoWeatherChange, target, source, status, None);
        Relay::TRUE
    }

    /// PS `field.removePseudoWeather(status)`.
    // PS: sim/field.ts removePseudoWeather
    pub fn remove_pseudo_weather(&mut self, status: EffectId) -> bool {
        let Some(st) = self.field.pseudo_weather.get(status) else { return false };
        let serial = st.serial;
        self.single_event(
            EventId::FieldEnd,
            status,
            StateRef::PseudoWeather(status),
            serial,
            Holder::Field,
            None,
            EffectId::None,
            None,
        );
        if let Some(gone) = self.field.pseudo_weather.remove(status) {
            self.bury(gone);
        }
        true
    }

    /// PS `field.clearWeather()`.
    // PS: sim/field.ts clearWeather
    pub fn clear_weather(&mut self) -> bool {
        if self.field.weather.is_none() {
            return false;
        }
        let prev = self.field.weather_effect();
        let serial = self.field.weather_state.serial;
        self.single_event(EventId::FieldEnd, prev, StateRef::Weather, serial, Holder::Field, None, EffectId::None, None);
        self.field.weather = ConditionId::NONE;
        // `clearEffectState`: the same object, emptied (id '' and
        // effectOrder 0; `target` kept).
        let target = self.field.weather_state.target;
        self.field.weather_state = EffectState { serial, target, ..EffectState::EMPTY };
        self.each_event(EventId::WeatherChange, EffectId::None, None);
        true
    }

    /// PS `faintMessages()`. The event core calls it after each residual
    /// handler; the queue task (P1-6) ports the body.
    pub fn faint_messages(&mut self) {}
}

// Design section 6: at most 16 KB of RAM per battle, on every target.
const _: () = assert!(core::mem::size_of::<Battle>() <= 16384, "Battle over 16 KB");

#[cfg(test)]
mod size_tests {
    use super::*;

    #[test]
    fn sizes() {
        extern crate std;
        std::println!(
            "Battle {} Side {} Pokemon {} EffectState {} EffectTable<12> {} Log {} Field {}",
            core::mem::size_of::<Battle>(),
            core::mem::size_of::<Side>(),
            core::mem::size_of::<Pokemon>(),
            core::mem::size_of::<EffectState>(),
            core::mem::size_of::<EffectTable<VOLATILE_CAP>>(),
            core::mem::size_of::<Log>(),
            core::mem::size_of::<Field>(),
        );
        assert!(core::mem::size_of::<Battle>() <= 16384);
    }
}
