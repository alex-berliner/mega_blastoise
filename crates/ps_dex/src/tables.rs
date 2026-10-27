//! The shape of one generation's dex. `build.rs` fills these structs from
//! the JSON dumps of `tools/ps-diff/dump-dex.js` (PS's resolved
//! `Dex.mod(genN)`), one `static` [`Dex`] per generation.
//!
//! Field names follow PS's data fields (`basePower` is `base_power`, ...).
//! Where PS has "nothing" the table says so explicitly: an `Option`, an id's
//! `NONE`, an empty slice or an empty [`Str`]. Every field is checked back
//! against the JSON by `tests/roundtrip.rs`.

use crate::ids::{AbilityId, ConditionId, ItemId, MoveId, SpeciesId};
use crate::text::{name_is_id, Str};
use crate::types::{
    ConditionSource, ConditionType, Gender, MoveCategory, MoveTarget, SelfDestruct, SelfSwitch,
    Type, TYPE_COUNT,
};

include!(concat!(env!("OUT_DIR"), "/names.rs"));

/// What a handler key holds. PS's `runEvent` calls a function and returns
/// anything else as is, so `onTry: false` (Teleport) and
/// `onLockMove: 'recharge'` (mustrecharge) are handlers too.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandlerValue {
    /// A function: its code lives in `ps_sim`, keyed by effect and name.
    Function,
    /// The constant `false`.
    False,
    /// A constant id string, an index into [`HANDLER_CONSTANTS`].
    Id(u8),
}

/// One handler of an effect: the key PS dispatches on (`onResidual`,
/// `basePowerCallback`, ...) and the ordering data beside it in PS's data
/// files (`onResidualOrder`, `onResidualPriority`, `onResidualSubOrder`).
///
/// PS `resolvePriority` (sim/battle.ts) reads them as `order || false`,
/// `priority || 0` and `subOrder || 0` (then a per-effect-type default), so
/// each is 0 where PS has nothing, and an `order` of 0 is PS's `false`
/// (no PS data file has a literal 0 order; `build.rs` checks).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Handler {
    /// Index into [`HANDLER_NAMES`].
    pub name: u8,
    pub value: HandlerValue,
    pub priority: i8,
    /// 0 is PS's `false`; see [`Handler::order`].
    pub order: i8,
    pub sub_order: i8,
}

impl Handler {
    /// `order`, `None` where PS's `resolvePriority` gets `false`.
    pub fn order(&self) -> Option<i8> {
        if self.order == 0 {
            None
        } else {
            Some(self.order)
        }
    }

    /// The key PS dispatches on, e.g. `onBeforeMove`.
    pub fn name(&self) -> &'static str {
        HANDLER_NAMES[self.name as usize]
    }

    /// The constant id of an `Id` handler (`'recharge'`), if it is one.
    pub fn constant_id(&self) -> Option<&'static str> {
        match self.value {
            HandlerValue::Id(i) => Some(HANDLER_CONSTANTS[i as usize]),
            _ => None,
        }
    }
}

/// Finds a handler by name in a handler list.
pub fn find_handler(handlers: &'static [Handler], name: &str) -> Option<&'static Handler> {
    handlers.iter().find(|h| h.name() == name)
}

/// A boosts table (`boosts: {atk: 1}`): one stage count per stat, in PS's
/// `BoostID` order. A stat PS does not list is 0 (no dump lists a 0).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Boosts {
    pub atk: i8,
    pub def: i8,
    pub spa: i8,
    pub spd: i8,
    pub spe: i8,
    pub accuracy: i8,
    pub evasion: i8,
}

impl Boosts {
    /// PS's boost ids, in field order.
    pub const STATS: [&'static str; 7] = ["atk", "def", "spa", "spd", "spe", "accuracy", "evasion"];

    /// The stages, in [`Boosts::STATS`] order.
    pub fn as_array(&self) -> [i8; 7] {
        [self.atk, self.def, self.spa, self.spd, self.spe, self.accuracy, self.evasion]
    }
}

/// PS's `[numerator, denominator]` fields: `drain`, `recoil`, `heal`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fraction {
    pub num: u8,
    pub den: u8,
}

/// PS's `move.accuracy`: `true` (never misses) or a percentage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Accuracy {
    Always,
    Percent(u8),
}

/// PS's `move.damage`: fixed damage, either `'level'` or a number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FixedDamage {
    Level,
    Hp(u16),
}

/// A set of PS move flags (`move.flags`), one bit per name in
/// [`MOVE_FLAG_NAMES`]. The constants (`MoveFlags::CONTACT`, ...) are
/// generated with the names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct MoveFlags(pub u64);

impl MoveFlags {
    pub fn contains(self, other: MoveFlags) -> bool {
        self.0 & other.0 == other.0 && other.0 != 0
    }

    /// True when PS's `move.flags[name]` is set.
    pub fn has(self, name: &str) -> bool {
        MOVE_FLAG_NAMES
            .iter()
            .position(|n| *n == name)
            .is_some_and(|i| self.0 & (1 << i) != 0)
    }
}

/// The boolean move fields PS's sim reads (`ohko`, `willCrit`,
/// `ignoreImmunity`, ...), one bit per name in [`MOVE_BOOL_NAMES`]: the ones
/// some gen 1 or gen 3 move sets, out of the dumper's fixed list. A name
/// that is not there is `false` on every move.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct MoveBools(pub u16);

impl MoveBools {
    pub fn contains(self, other: MoveBools) -> bool {
        self.0 & other.0 == other.0 && other.0 != 0
    }

    /// True when PS's `move[name]` is `true`.
    pub fn has(self, name: &str) -> bool {
        MOVE_BOOL_NAMES
            .iter()
            .position(|n| *n == name)
            .is_some_and(|i| self.0 & (1 << i) != 0)
    }
}

/// PS ability flags (`ability.flags`), one bit per name in
/// [`ABILITY_FLAG_NAMES`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct AbilityFlags(pub u32);

impl AbilityFlags {
    pub fn has(self, name: &str) -> bool {
        ABILITY_FLAG_NAMES
            .iter()
            .position(|n| *n == name)
            .is_some_and(|i| self.0 & (1 << i) != 0)
    }
}

/// A move's `self` effect (also a secondary's `self`): PS's `HitEffect`
/// subset that gens 1 and 3 use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelfEffect {
    pub chance: Option<u8>,
    pub status: ConditionId,
    pub volatile_status: ConditionId,
    pub side_condition: ConditionId,
    pub pseudo_weather: ConditionId,
    pub boosts: Option<&'static Boosts>,
    pub handlers: &'static [Handler],
}

/// One entry of a move's `secondaries`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Secondary {
    pub chance: Option<u8>,
    pub status: ConditionId,
    pub volatile_status: ConditionId,
    pub boosts: Option<&'static Boosts>,
    pub self_effect: Option<&'static SelfEffect>,
    pub handlers: &'static [Handler],
}

/// A move, as `Dex.mod(gen).moves.get(key)` resolves it.
///
/// The fields most moves leave unset sit behind [`MoveDef::extra`], read
/// through the accessors (`drain()`, `side_condition()`, ...), to keep the
/// table small.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoveDef {
    /// `move.name`. The table key (what `moves.get` looks up) is its toID.
    pub name: Str,
    pub handlers: &'static [Handler],
    pub extra: Option<&'static MoveExtra>,
    pub num: i16,
    /// PS's `placeholderFor` (the Hidden Power hack): `hiddenpowerbug`'s
    /// `move.id` is `hiddenpower`, this entry. `NONE` for every other move,
    /// whose id is its own key.
    pub placeholder_for: MoveId,
    pub base_power: u16,
    /// `status`, as the condition it names.
    pub status: ConditionId,
    /// `volatileStatus`, as the condition it names.
    pub volatile_status: ConditionId,
    /// The move's own `condition`, as the conditions table holds it (under
    /// the move's key), or `NONE`.
    pub condition: ConditionId,
    pub flags: MoveFlags,
    pub bools: MoveBools,
    pub type_: Type,
    pub base_move_type: Type,
    pub category: MoveCategory,
    pub target: MoveTarget,
    pub priority: i8,
    /// `accuracy`, 0 for PS's `true`; see [`MoveDef::accuracy`].
    pub accuracy: u8,
    pub pp: u8,
    pub crit_ratio: u8,
}

/// The move fields most moves leave unset. Condition fields are the
/// condition each names (PS resolves them with `dex.conditions.get`, so
/// `slotCondition: 'Wish'` is `wish`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoveExtra {
    /// PS's `secondaries` (empty where PS has `undefined`).
    pub secondaries: &'static [Secondary],
    pub self_effect: Option<&'static SelfEffect>,
    pub boosts: Option<&'static Boosts>,
    pub side_condition: ConditionId,
    pub slot_condition: ConditionId,
    pub weather: ConditionId,
    pub pseudo_weather: ConditionId,
    pub damage: Option<FixedDamage>,
    /// `multihit`: `[min, max]` (a fixed count is `[n, n]`).
    pub multihit: Option<[u8; 2]>,
    pub drain: Option<Fraction>,
    pub recoil: Option<Fraction>,
    pub heal: Option<Fraction>,
    pub selfdestruct: Option<SelfDestruct>,
    pub self_switch: Option<SelfSwitch>,
}

impl MoveExtra {
    /// Every field unset: what a move without an `extra` has.
    pub const NONE: MoveExtra = MoveExtra {
        secondaries: &[],
        self_effect: None,
        boosts: None,
        side_condition: ConditionId::NONE,
        slot_condition: ConditionId::NONE,
        weather: ConditionId::NONE,
        pseudo_weather: ConditionId::NONE,
        damage: None,
        multihit: None,
        drain: None,
        recoil: None,
        heal: None,
        selfdestruct: None,
        self_switch: None,
    };
}

impl MoveDef {
    /// `accuracy`: `true` or a percentage.
    pub fn accuracy(&self) -> Accuracy {
        if self.accuracy == 0 {
            Accuracy::Always
        } else {
            Accuracy::Percent(self.accuracy)
        }
    }

    /// The rarely-set fields, [`MoveExtra::NONE`] if the move has none.
    pub fn extra(&self) -> &'static MoveExtra {
        match self.extra {
            Some(e) => e,
            None => &MoveExtra::NONE,
        }
    }

    pub fn secondaries(&self) -> &'static [Secondary] {
        self.extra().secondaries
    }
    pub fn self_effect(&self) -> Option<&'static SelfEffect> {
        self.extra().self_effect
    }
    pub fn boosts(&self) -> Option<&'static Boosts> {
        self.extra().boosts
    }
    pub fn side_condition(&self) -> ConditionId {
        self.extra().side_condition
    }
    pub fn slot_condition(&self) -> ConditionId {
        self.extra().slot_condition
    }
    pub fn weather(&self) -> ConditionId {
        self.extra().weather
    }
    pub fn pseudo_weather(&self) -> ConditionId {
        self.extra().pseudo_weather
    }
    pub fn damage(&self) -> Option<FixedDamage> {
        self.extra().damage
    }
    pub fn multihit(&self) -> Option<[u8; 2]> {
        self.extra().multihit
    }
    pub fn drain(&self) -> Option<Fraction> {
        self.extra().drain
    }
    pub fn recoil(&self) -> Option<Fraction> {
        self.extra().recoil
    }
    pub fn heal(&self) -> Option<Fraction> {
        self.extra().heal
    }
    pub fn selfdestruct(&self) -> Option<SelfDestruct> {
        self.extra().selfdestruct
    }
    pub fn self_switch(&self) -> Option<SelfSwitch> {
        self.extra().self_switch
    }
}

/// A species, as `Dex.mod(gen).species.get(key)` resolves it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpeciesDef {
    pub name: Str,
    /// The fields few species set (forme data, `maxHP`), `None` where all
    /// are unset (a base forme's `baseSpecies` is its own name).
    pub forme: Option<&'static Forme>,
    pub num: i16,
    pub weighthg: u16,
    /// `types`; a single-typed species has `Type::None` second.
    pub types: [Type; 2],
    /// `baseStats` in PS's order: hp, atk, def, spa, spd, spe.
    pub base_stats: [u8; 6],
    /// `abilities['0']` and `abilities['1']`: indices into
    /// [`ABILITY_NAMES`], `NO_ABILITY` for an empty slot; see
    /// [`SpeciesDef::ability`]. Gen 1 species keep the base data's
    /// abilities (`Overgrow`), which the gen 1 dex marks nonstandard; that
    /// is PS's data, not a dump choice.
    pub abilities: [u8; 2],
    /// `gender`: `Either` where PS has `''`.
    pub gender: Gender,
    /// `genderRatio` `{M, F}` in eighths (every PS ratio is k/8).
    pub gender_ratio: [u8; 2],
    pub nfe: bool,
}

/// An empty ability slot in [`SpeciesDef::abilities`].
pub const NO_ABILITY: u8 = u8::MAX;

/// The fields few species set: forme data, and `maxHP` (Shedinja's 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Forme {
    pub max_hp: Option<u16>,
    /// `baseSpecies`, a species name (`Castform` for `Castform-Sunny`).
    pub base_species: Str,
    /// `forme`, empty for a base forme.
    pub forme: Str,
    /// `battleOnly`, a species name, empty if none.
    pub battle_only: Str,
    /// `requiredItem`, an item name, empty if none.
    pub required_item: Str,
}

impl SpeciesDef {
    /// `abilities[slot]` for slot 0 or 1, as a name.
    pub fn ability(&self, slot: usize) -> Option<&'static str> {
        match self.abilities[slot] {
            NO_ABILITY => None,
            i => Some(ABILITY_NAMES[i as usize]),
        }
    }

    /// `baseSpecies`.
    pub fn base_species(&self) -> Str {
        match self.forme {
            Some(f) if !f.base_species.is_empty() => f.base_species,
            _ => self.name,
        }
    }

    /// `maxHP`.
    pub fn max_hp(&self) -> Option<u16> {
        self.forme.and_then(|f| f.max_hp)
    }
}

/// An item's `naturalGift`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NaturalGift {
    pub base_power: u8,
    pub type_: Type,
}

/// An item's `fling`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fling {
    pub base_power: u8,
    /// A condition id, empty if none.
    pub status: Str,
    /// A condition id, empty if none.
    pub volatile_status: Str,
    /// True where PS has a `fling.effect` function (Mental Herb, White
    /// Herb). Fling is a gen 4 move, so no gen 1 or 3 battle reaches it.
    pub effect: bool,
}

/// An item, as `Dex.mod(gen).items.get(key)` resolves it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemDef {
    pub name: Str,
    pub num: i16,
    pub is_berry: bool,
    pub is_gem: bool,
    pub is_pokeball: bool,
    pub is_choice: bool,
    pub ignore_klutz: bool,
    pub natural_gift: Option<NaturalGift>,
    pub fling: Option<Fling>,
    /// `itemUser`, species names; empty where PS has none.
    pub item_user: &'static [Str],
    pub boosts: Option<&'static Boosts>,
    pub condition: ConditionId,
    pub handlers: &'static [Handler],
    /// `onSwitchInPriority` / `Order` / `SubOrder`, see [`SwitchInOrder`].
    pub switch_in_order: SwitchInOrder,
}

/// An ability's or item's `onSwitchInPriority`, `onSwitchInOrder` and
/// `onSwitchInSubOrder`, whether or not it has an `onSwitchIn` handler:
/// PS's `getCallback` lets one without it answer `SwitchIn` with its
/// `onStart` (gen 3 on turn 0: `onAfterEntryHazard`), and `resolvePriority`
/// reads these keys for that answer (gen 3 Intimidate 2, Trace 1). 0 is
/// PS's missing key (`order` 0 is `false`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct SwitchInOrder {
    pub priority: i8,
    pub order: i8,
    pub sub_order: i8,
}

impl SwitchInOrder {
    pub const NONE: SwitchInOrder = SwitchInOrder { priority: 0, order: 0, sub_order: 0 };
}

/// An ability, as `Dex.mod(gen).abilities.get(key)` resolves it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AbilityDef {
    pub name: Str,
    pub num: i16,
    /// `rating` in tenths (PS's `3.5` is 35).
    pub rating_tenths: i8,
    pub suppress_weather: bool,
    pub flags: AbilityFlags,
    pub condition: ConditionId,
    pub handlers: &'static [Handler],
    /// `onSwitchInPriority` / `Order` / `SubOrder`, see [`SwitchInOrder`].
    pub switch_in_order: SwitchInOrder,
}

/// A condition, as `Dex.mod(gen).conditions.get(key)` resolves it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConditionDef {
    /// The id PS looks it up by (`brn`, `partiallytrapped`, a move's key).
    pub key: Str,
    /// `condition.name`: the key for a Conditions-table entry without one,
    /// the move's (ability's, item's) name for a nested `condition`.
    pub name: Str,
    pub effect_type: ConditionType,
    pub source: ConditionSource,
    pub duration: Option<u8>,
    pub counter_max: Option<u8>,
    pub no_copy: bool,
    pub affects_fainted: bool,
    pub handlers: &'static [Handler],
    /// Residual ordering keys with no residual handler, see
    /// [`ResidualOrder`].
    pub residual_order: Option<ResidualOrder>,
}

/// Which residual a condition's [`ResidualOrder`] keys are for:
/// `onResidual*` (a Pokemon's), `onSideResidual*`, `onFieldResidual*`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResidualKind {
    Residual,
    SideResidual,
    FieldResidual,
}

/// A condition's `on{,Side,Field}ResidualOrder` / `Priority` /
/// `SubOrder` keys where it has no residual handler beside them (gen 3
/// Taunt 10/15, Yawn 10/19, Wish 7, Reflect 1...). PS's residual gathers a
/// condition with a `duration` whether or not it answers, and
/// `resolvePriority` reads these keys for it, so they order its end. The
/// keys use [`SwitchInOrder`]'s encoding (0 is a missing key).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResidualOrder {
    pub kind: ResidualKind,
    pub keys: SwitchInOrder,
}

/// A non-type immunity of a defending type (`Poison` is immune to `psn`,
/// `Rock` to `sandstorm`): `damageTaken[id]`, which PS's `getImmunity`
/// reads for status and weather ids.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpecialImmunity {
    pub type_: Type,
    pub id: Str,
    pub code: u8,
}

/// The type chart, PS's `damageTaken` codes: 0 normal, 1 weak (2x),
/// 2 resists (0.5x), 3 immune.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TypeChart {
    /// The gen's types in PS's iteration order (`dex.types.all()`).
    pub order: &'static [Type],
    /// `damage_taken[defender][attacker]`, indexed by `Type as usize`. A
    /// type the gen lacks, and `Type::None` (`???`), are all 0.
    pub damage_taken: [[u8; TYPE_COUNT]; TYPE_COUNT],
    pub special: &'static [SpecialImmunity],
}

impl TypeChart {
    /// `damageTaken` code of `defender` against an `attacker` move type.
    pub fn code(&self, attacker: Type, defender: Type) -> u8 {
        self.damage_taken[defender as usize][attacker as usize]
    }

    /// The code of a non-type id (`psn`, `sandstorm`), 0 if PS lists none.
    pub fn special_code(&self, id: &str, defender: Type) -> u8 {
        self.special
            .iter()
            .find(|s| s.type_ == defender && s.id.as_str() == id)
            .map_or(0, |s| s.code)
    }
}

/// One generation's resolved dex.
#[derive(Debug)]
pub struct Dex {
    pub gen: u8,
    pub species: &'static [SpeciesDef],
    pub moves: &'static [MoveDef],
    pub items: &'static [ItemDef],
    pub abilities: &'static [AbilityDef],
    pub conditions: &'static [ConditionDef],
    /// PS `species.cosmeticFormes` of the species that have them.
    pub cosmetic_formes: &'static [CosmeticFormes],
    pub typechart: TypeChart,
}

/// A species' cosmetic formes (PS `species.cosmeticFormes`, in order):
/// formes that differ only in name, which the details line prints
/// (`Unown-T`). PS builds one as its own species object when a set names
/// it; the species table lists only the base.
#[derive(Clone, Copy, Debug)]
pub struct CosmeticFormes {
    pub base: SpeciesId,
    pub names: &'static [Str],
}

fn position<T>(table: &[T], id: &str, name: impl Fn(&T) -> Str) -> Option<u16> {
    table.iter().position(|e| name_is_id(name(e).as_str(), id)).map(|i| i as u16)
}

impl Dex {
    /// PS's `moves.get(id)`: the entry keyed by `id` (already an id, so
    /// `hiddenpowerbug`, not `Hidden Power Bug`). Linear; not for a hot path.
    pub fn move_id(&self, id: &str) -> Option<MoveId> {
        position(self.moves, id, |m| m.name).map(MoveId)
    }

    pub fn species_id(&self, id: &str) -> Option<SpeciesId> {
        position(self.species, id, |s| s.name).map(SpeciesId)
    }

    pub fn item_id(&self, id: &str) -> Option<ItemId> {
        position(self.items, id, |i| i.name).map(ItemId)
    }

    pub fn ability_id(&self, id: &str) -> Option<AbilityId> {
        position(self.abilities, id, |a| a.name).map(AbilityId)
    }

    pub fn condition_id(&self, id: &str) -> Option<ConditionId> {
        self.conditions
            .iter()
            .position(|c| c.key.as_str() == id)
            .map(|i| ConditionId(i as u16))
    }

    pub fn move_def(&self, id: MoveId) -> &'static MoveDef {
        &self.moves[id.index()]
    }

    pub fn species_def(&self, id: SpeciesId) -> &'static SpeciesDef {
        &self.species[id.index()]
    }

    pub fn item_def(&self, id: ItemId) -> &'static ItemDef {
        &self.items[id.index()]
    }

    pub fn ability_def(&self, id: AbilityId) -> &'static AbilityDef {
        &self.abilities[id.index()]
    }

    pub fn condition_def(&self, id: ConditionId) -> &'static ConditionDef {
        &self.conditions[id.index()]
    }

    /// The cosmetic formes of `species` (empty for most).
    pub fn cosmetic_formes(&self, species: SpeciesId) -> &'static [Str] {
        self.cosmetic_formes.iter().find(|c| c.base == species).map_or(&[], |c| c.names)
    }

    /// PS's `species.get(id)` for a cosmetic forme (`unownt`): the base
    /// species and the forme's 1-based index in its `cosmeticFormes`.
    /// Linear; not for a hot path.
    pub fn cosmetic_forme_id(&self, id: &str) -> Option<(SpeciesId, u8)> {
        self.cosmetic_formes.iter().find_map(|c| {
            c.names.iter().position(|n| name_is_id(n.as_str(), id)).map(|i| (c.base, i as u8 + 1))
        })
    }

    /// The name of cosmetic forme `forme` (1-based) of `species`.
    pub fn cosmetic_forme_name(&self, species: SpeciesId, forme: u8) -> Option<Str> {
        self.cosmetic_formes(species).get(usize::from(forme).checked_sub(1)?).copied()
    }

    /// PS's `move.id`: the placeholder's id for a Hidden Power variant,
    /// otherwise the move's own.
    pub fn move_ps_id(&self, id: MoveId) -> MoveId {
        let def = self.move_def(id);
        if def.placeholder_for == MoveId::NONE {
            id
        } else {
            def.placeholder_for
        }
    }
}
