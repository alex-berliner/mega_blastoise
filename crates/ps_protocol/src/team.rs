//! Teams: [`PackedSet`] is PS's packed format field by field (what
//! `>player` carries), [`PokemonSet`] is the typed set the sim builds a
//! Pokemon from.
//!
//! PS: `sim/teams.ts` `pack` / `unpack` / `packName`, `sim/pokemon.ts`
//! constructor (the defaults applied to a set).
//!
//! Turning a `PackedSet` into a `PokemonSet` needs the dex (PS `unpackName`
//! looks every name up, and an ability field of `0`, `1`, `H` or `S` is a
//! slot of the species' abilities), so it lives with the sim, not here.

use crate::ident::{AbilityNum, Gender, ItemNum, MoveNum, NatureNum, SpeciesNum, TypeNum};
use crate::request::Pokeball;
use crate::{BoundedStr, BoundedVec, Name, MAX_MOVES, MAX_TEAM};

/// A packed team: sets joined by `]`.
pub type PackedTeam = BoundedVec<PackedSet, MAX_TEAM>;

/// A typed team, in `>player` order (the order [`crate::PokemonIdent::slot`]
/// indexes).
pub type Team = BoundedVec<PokemonSet, MAX_TEAM>;

/// Six stats in PS's `StatsTable` key order: hp, atk, def, spa, spd, spe.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct StatTable {
    pub hp: u16,
    pub atk: u16,
    pub def: u16,
    pub spa: u16,
    pub spd: u16,
    pub spe: u16,
}

impl StatTable {
    pub const fn splat(v: u16) -> Self {
        Self { hp: v, atk: v, def: v, spa: v, spd: v, spe: v }
    }

    pub const fn to_array(self) -> [u16; 6] {
        [self.hp, self.atk, self.def, self.spa, self.spd, self.spe]
    }

    pub const fn from_array(a: [u16; 6]) -> Self {
        Self { hp: a[0], atk: a[1], def: a[2], spa: a[3], spd: a[4], spe: a[5] }
    }
}

/// A short packed field (nature, type, ball).
pub type ShortName = BoundedStr<16>;

/// One set in PS's packed format, kept as text where PS keeps text so that
/// a packed string from PS's `Teams.pack` re-packs byte for byte.
///
/// Fields, `|`-separated in this order (PS `pack`):
///
/// | # | Field | Packed as |
/// |---|---|---|
/// | 1 | `name` | the nickname, or the species when unnamed |
/// | 2 | `species` | `packName(species)`, empty when `packName(name)` equals it |
/// | 3 | `item` | `packName(item)` (`SoulDew`) |
/// | 4 | `ability` | `packName(ability)`, or a slot `0` / `1` / `H` / `S` |
/// | 5 | `moves` | `packName` of each, joined by `,` (case as given: `hiddenpowerfire`, `Thunderbolt`) |
/// | 6 | `nature` | the nature name, or empty |
/// | 7 | `evs` | empty, or six values joined by `,` with 0 written as empty |
/// | 8 | `gender` | `M`, `F`, `N` or empty |
/// | 9 | `ivs` | empty, or six values joined by `,` with 31 written as empty |
/// | 10 | `shiny` | `S` or empty |
/// | 11 | `level` | empty for 100 |
/// | 12 | `happiness` | empty for 255, then [`PackedMisc`] when present |
///
/// PS writes `evs` empty when every value would be (all zero) and `ivs` empty
/// when every value is 31; a decoder reads an empty field as `None`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PackedSet {
    pub name: Name,
    pub species: Name,
    pub item: Name,
    pub ability: Name,
    pub moves: BoundedVec<Name, MAX_MOVES>,
    pub nature: ShortName,
    pub evs: Option<StatTable>,
    pub gender: Option<Gender>,
    pub ivs: Option<StatTable>,
    pub shiny: bool,
    /// 100 packs as empty.
    pub level: u16,
    /// 255 packs as empty.
    pub happiness: u8,
    pub misc: Option<PackedMisc>,
}

/// The tail of the happiness field: `,HPTYPE,BALL,G,DYNAMAXLEVEL,TERATYPE`,
/// written when any of them is set (PS `pack`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PackedMisc {
    pub hp_type: ShortName,
    /// `packName(pokeball)`.
    pub pokeball: ShortName,
    pub gigantamax: bool,
    /// 10 packs as empty.
    pub dynamax_level: u8,
    pub tera_type: ShortName,
}

/// A set as the sim reads it: PS's `PokemonSet` after `unpack` and the
/// `Pokemon` constructor's defaults.
///
/// Defaults to apply when building one (PS `Pokemon` constructor): level
/// `set.level || 100` clamped to 1..9999; happiness 255 unless given; EVs 0
/// and IVs 31 where missing, then clamped to 0..255 and 0..31 (and IVs
/// `&= 30` in gens 1 and 2); `hp_type` from the Hidden Power move's type or
/// else from the IVs. The gender is NOT defaulted here: `None` means PS rolls
/// it with `battle.sample(['M', 'F'])` (a PRNG call) unless the species has a
/// fixed gender.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PokemonSet {
    /// The nickname (PS `set.name`, the species name when unnamed).
    pub name: Name,
    pub species: SpeciesNum,
    pub item: Option<ItemNum>,
    pub ability: Option<AbilityNum>,
    pub moves: BoundedVec<MoveNum, MAX_MOVES>,
    pub nature: Option<NatureNum>,
    pub evs: StatTable,
    pub ivs: StatTable,
    pub gender: Option<Gender>,
    pub shiny: bool,
    pub level: u16,
    pub happiness: u8,
    pub hp_type: Option<TypeNum>,
    pub pokeball: Pokeball,
}

impl Default for PokemonSet {
    fn default() -> Self {
        Self {
            name: Name::new(),
            species: SpeciesNum(0),
            item: None,
            ability: None,
            moves: BoundedVec::new(),
            nature: None,
            evs: StatTable::splat(0),
            ivs: StatTable::splat(31),
            gender: None,
            shiny: false,
            level: 100,
            happiness: 255,
            hp_type: None,
            pokeball: Pokeball::Default,
        }
    }
}

impl PackedSet {
    /// An unnamed level-100 set of `species` (packed name) with `moves`.
    pub fn new(species: &str, moves: &[&str]) -> Result<Self, crate::CapacityError> {
        let mut set = PackedSet {
            name: Name::from_str(species)?,
            level: 100,
            happiness: 255,
            ..Default::default()
        };
        for m in moves {
            set.moves.push(Name::from_str(m)?)?;
        }
        Ok(set)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_ps() {
        let s = PokemonSet::default();
        assert_eq!(s.level, 100);
        assert_eq!(s.happiness, 255);
        assert_eq!(s.ivs, StatTable::splat(31));
        assert_eq!(s.evs, StatTable::splat(0));
        assert_eq!(s.gender, None);
    }

    #[test]
    fn packed_set_constructor() {
        let p = PackedSet::new("Moltres", &["fireblast", "agility", "doubleedge", "hyperbeam"]).unwrap();
        assert_eq!(&*p.name, "Moltres");
        assert_eq!(p.moves.len(), 4);
        assert_eq!(&*p.moves[3], "hyperbeam");
        assert!(PackedSet::new("Moltres", &["a", "b", "c", "d", "e"]).is_err());
    }

    #[test]
    fn stat_table_order() {
        let t = StatTable::from_array([1, 2, 3, 4, 5, 6]);
        assert_eq!(t.atk, 2);
        assert_eq!(t.spe, 6);
        assert_eq!(t.to_array(), [1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn team_capacity_is_six() {
        let mut team = PackedTeam::new();
        for _ in 0..6 {
            team.push(PackedSet::default()).unwrap();
        }
        assert!(team.push(PackedSet::default()).is_err());
    }
}
