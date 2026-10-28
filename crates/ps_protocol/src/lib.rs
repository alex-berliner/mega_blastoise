//! Pokemon Showdown sim protocol types.
//!
//! The typed form of the subset of the PS sim protocol named in
//! `docs/living/showdown-split-plan.md` section 3: what the server emits
//! ([`Msg`], [`Request`]) and what it takes ([`Command`]), plus the identifiers
//! they are built from ([`PokemonIdent`], [`Details`], [`Health`]) and teams
//! ([`PackedSet`] as PS packs it, [`PokemonSet`] as the sim reads it).
//!
//! Nothing here is text. Names are dex numbers ([`MoveNum`], [`SpeciesNum`],
//! ...) and per-battle handles (a side, a team slot); turning them into PS's
//! bytes needs a dex ([`DexNames`]) and the battle's player and nickname tables
//! ([`BattleNames`]). The text codec that does so lives behind the `text`
//! feature (task P1-4) and follows the encoding notes on each type here.
//!
//! Capacity is fixed throughout: [`Msg`] holds no heap and no pointers and is at
//! most 24 bytes (`tests` asserts it), requests and teams use [`BoundedVec`] and
//! [`BoundedStr`].
//!
//! Source of truth: PS 0.11.11, `sim/SIM-PROTOCOL.md`, `sim/battle.ts` (`add`,
//! `addSplit`, `addMove`, `attrLastMove`, `getRequests`), `sim/side.ts`
//! (`getRequestData`, `choose*`, `emitRequest`), `sim/pokemon.ts` (`details`,
//! `getHealth`, `getMoveRequestData`, `getMoves`, `getSwitchRequestData`) and
//! `sim/teams.ts` (`pack`, `unpack`). The line-kind to variant table is on
//! [`msg`].

#![no_std]

extern crate alloc;

pub mod command;
pub mod ident;
pub mod msg;
pub mod request;
pub mod team;

#[cfg(feature = "text")]
pub mod text;

pub use command::{Choice, Command, CustomRule, FormatId, FormatSpec, PrngSeed, RuleId};
pub use ident::{
    AbilityNum, Details, Gender, Health, Hp, HpStatus, ItemNum, MoveNum, NatureNum, PokemonIdent,
    SideId, SpeciesNum, Stat, Status, TypeNum,
};
pub use msg::{
    Arg, ArgValue, GameType, Kw, KwFlags, KwPart, Msg, Text, TextArg, TextArgKind, TextArgValue, TextId,
    Weather,
    Word,
};
pub use request::{
    ActiveRequest, MoveRequest, MoveTarget, Pokeball, Request, RequestMove, RequestMoveName,
    SideMoveId, SidePokemon, SideRequest, SwitchRequest, TeamPreviewRequest, WaitRequest,
};
pub use team::{PackedMisc, PackedSet, PackedTeam, PokemonSet, ShortName, StatTable, Team};

/// Most Pokemon on one side (a PS team may hold 24; this device plays 6).
pub const MAX_TEAM: usize = 6;
/// Most moves on one Pokemon.
pub const MAX_MOVES: usize = 4;
/// Active slots per side. Singles only (design section 2: no doubles).
pub const MAX_ACTIVE: usize = 1;
/// Byte capacity of a nickname, a player name or a packed name field. PS caps
/// nicknames at 18 characters and usernames at 18; 24 bytes leaves room for
/// a few multi-byte characters.
pub const NAME_CAP: usize = 24;

/// A name of at most [`NAME_CAP`] bytes.
pub type Name = BoundedStr<NAME_CAP>;

/// A value did not fit its fixed capacity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CapacityError;

/// A UTF-8 string of at most `N` bytes stored inline (no heap). Equality
/// and hashing look at the string only.
#[derive(Clone, Copy)]
pub struct BoundedStr<const N: usize> {
    len: u8,
    buf: [u8; N],
}

impl<const N: usize> BoundedStr<N> {
    /// The empty string.
    pub const fn new() -> Self {
        assert!(N <= u8::MAX as usize, "BoundedStr capacity must fit a u8 length");
        Self { len: 0, buf: [0; N] }
    }

    /// Copies `s`, or fails if it is longer than `N` bytes.
    pub fn from_str(s: &str) -> Result<Self, CapacityError> {
        let mut out = Self::new();
        out.push_str(s)?;
        Ok(out)
    }

    /// Appends `s`; on overflow nothing is appended.
    pub fn push_str(&mut self, s: &str) -> Result<(), CapacityError> {
        let len = self.len as usize;
        let end = len + s.len();
        if end > N {
            return Err(CapacityError);
        }
        self.buf[len..end].copy_from_slice(s.as_bytes());
        self.len = end as u8;
        Ok(())
    }

    /// Appends one character.
    pub fn push(&mut self, c: char) -> Result<(), CapacityError> {
        let mut tmp = [0u8; 4];
        self.push_str(c.encode_utf8(&mut tmp))
    }

    pub fn as_str(&self) -> &str {
        // Only whole `&str`s are ever copied in, so the prefix is UTF-8.
        core::str::from_utf8(&self.buf[..self.len as usize]).unwrap_or("")
    }

    pub fn len(&self) -> usize {
        self.len as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }
}

impl<const N: usize> PartialEq for BoundedStr<N> {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl<const N: usize> Eq for BoundedStr<N> {}

impl<const N: usize> core::hash::Hash for BoundedStr<N> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state)
    }
}

impl<const N: usize> Default for BoundedStr<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> core::ops::Deref for BoundedStr<N> {
    type Target = str;
    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl<const N: usize> core::fmt::Debug for BoundedStr<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(self.as_str(), f)
    }
}

impl<const N: usize> core::fmt::Display for BoundedStr<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<const N: usize> core::fmt::Write for BoundedStr<N> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.push_str(s).map_err(|_| core::fmt::Error)
    }
}

#[cfg(feature = "defmt")]
impl<const N: usize> defmt::Format for BoundedStr<N> {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "{=str}", self.as_str())
    }
}

/// Up to `N` values of `T` stored inline (no heap), in insertion order.
/// Equality and hashing look at the live elements only.
#[derive(Clone, Copy)]
pub struct BoundedVec<T: Copy + Default, const N: usize> {
    len: u8,
    items: [T; N],
}

impl<T: Copy + Default, const N: usize> BoundedVec<T, N> {
    pub fn new() -> Self {
        assert!(N <= u8::MAX as usize, "BoundedVec capacity must fit a u8 length");
        Self { len: 0, items: [T::default(); N] }
    }

    /// Copies `items`, or fails if there are more than `N`.
    pub fn from_slice(items: &[T]) -> Result<Self, CapacityError> {
        let mut out = Self::new();
        for &item in items {
            out.push(item)?;
        }
        Ok(out)
    }

    /// Appends `item`, or fails (unchanged) when full.
    pub fn push(&mut self, item: T) -> Result<(), CapacityError> {
        let len = self.len as usize;
        if len == N {
            return Err(CapacityError);
        }
        self.items[len] = item;
        self.len += 1;
        Ok(())
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        Some(self.items[self.len as usize])
    }

    pub fn as_slice(&self) -> &[T] {
        &self.items[..self.len as usize]
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.items[..self.len as usize]
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub const fn capacity(&self) -> usize {
        N
    }
}

impl<T: Copy + Default + PartialEq, const N: usize> PartialEq for BoundedVec<T, N> {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<T: Copy + Default + Eq, const N: usize> Eq for BoundedVec<T, N> {}

impl<T: Copy + Default + core::hash::Hash, const N: usize> core::hash::Hash for BoundedVec<T, N> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state)
    }
}

impl<T: Copy + Default, const N: usize> Default for BoundedVec<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Copy + Default, const N: usize> core::ops::Deref for BoundedVec<T, N> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T: Copy + Default, const N: usize> core::ops::DerefMut for BoundedVec<T, N> {
    fn deref_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T: Copy + Default + core::fmt::Debug, const N: usize> core::fmt::Debug for BoundedVec<T, N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self.as_slice()).finish()
    }
}

#[cfg(feature = "defmt")]
impl<T: Copy + Default + defmt::Format, const N: usize> defmt::Format for BoundedVec<T, N> {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "{=[?]}", self.as_slice())
    }
}

impl<'a, T: Copy + Default, const N: usize> IntoIterator for &'a BoundedVec<T, N> {
    type Item = &'a T;
    type IntoIter = core::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

/// PS `toID`: lowercase ASCII letters and digits only, everything else
/// dropped (`"Mr. Mime"` -> `"mrmime"`). Writes into a bounded buffer.
pub fn to_id<const N: usize>(text: &str) -> Result<BoundedStr<N>, CapacityError> {
    let mut out = BoundedStr::<N>::new();
    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase())?;
        }
    }
    Ok(out)
}

/// Dex names the text codec needs, implemented by `ps_dex` for one gen. The
/// `*_id` methods give PS's lowercase id (`"thunderbolt"`), the others the
/// display name (`"Thunderbolt"`). The `from_id` lookups take an id (callers
/// run PS's [`to_id`] first), which is how PS itself resolves names.
pub trait DexNames {
    fn move_name(&self, m: MoveNum) -> &str;
    fn move_id(&self, m: MoveNum) -> &str;
    fn move_from_id(&self, id: &str) -> Option<MoveNum>;
    fn species_name(&self, s: SpeciesNum) -> &str;
    fn species_from_id(&self, id: &str) -> Option<SpeciesNum>;
    /// The name of cosmetic forme `forme` of `s` ([`Details::forme`], 1-based
    /// into PS `species.cosmeticFormes`): `Unown-T`.
    fn cosmetic_forme_name(&self, s: SpeciesNum, forme: u8) -> &str;
    /// The species and [`Details::forme`] of a cosmetic forme's id (`unownt`).
    fn cosmetic_forme_from_id(&self, id: &str) -> Option<(SpeciesNum, u8)>;
    fn item_name(&self, i: ItemNum) -> &str;
    fn item_id(&self, i: ItemNum) -> &str;
    fn item_from_id(&self, id: &str) -> Option<ItemNum>;
    fn ability_name(&self, a: AbilityNum) -> &str;
    fn ability_id(&self, a: AbilityNum) -> &str;
    fn ability_from_id(&self, id: &str) -> Option<AbilityNum>;
    fn type_name(&self, t: TypeNum) -> &str;
    fn type_from_id(&self, id: &str) -> Option<TypeNum>;
    fn nature_name(&self, n: NatureNum) -> &str;
    fn nature_from_id(&self, id: &str) -> Option<NatureNum>;
}

/// Per-battle names the text codec needs: what `>player` and the teams set.
/// The server knows them from its commands; a client decoding a real PS
/// stream learns them from `|player|` lines and its first request.
pub trait BattleNames {
    /// The player's name (`|player|p1|NAME|...`, `|win|NAME`, `p1: NAME`).
    fn player_name(&self, side: SideId) -> &str;
    /// The player's avatar field of `|player|` (empty unless `>player` set one).
    fn player_avatar(&self, side: SideId) -> &str;
    /// The player's rating field of `|player|` (empty unless `>player` set one).
    fn player_rating(&self, side: SideId) -> &str;
    /// The nickname of the Pokemon in team slot `slot` (0-based, team order
    /// as `>player` gave it), the `NAME` of `p1a: NAME`.
    fn nickname(&self, side: SideId, slot: u8) -> &str;
    /// Custom rule `i` of the format as written after `@@@` (the custom
    /// rules infobox prints them as given).
    fn custom_rule(&self, i: u8) -> &str;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_str_basics() {
        let mut s = BoundedStr::<4>::from_str("ab").unwrap();
        assert_eq!(s.as_str(), "ab");
        assert!(s.push_str("cde").is_err());
        assert_eq!(s.as_str(), "ab", "a failed push leaves the string as it was");
        s.push('c').unwrap();
        assert_eq!(&*s, "abc");
        assert!(BoundedStr::<2>::from_str("abc").is_err());
        // Equality ignores stale bytes past the end.
        let mut a = BoundedStr::<4>::from_str("abcd").unwrap();
        a.clear();
        a.push_str("x").unwrap();
        assert_eq!(a, BoundedStr::<4>::from_str("x").unwrap());
    }

    #[test]
    fn bounded_vec_basics() {
        let mut v = BoundedVec::<u8, 2>::new();
        v.push(1).unwrap();
        v.push(2).unwrap();
        assert!(v.push(3).is_err());
        assert_eq!(v.as_slice(), &[1, 2]);
        assert_eq!(v.pop(), Some(2));
        // Equality ignores the popped element still in the buffer.
        assert_eq!(v, BoundedVec::<u8, 2>::from_slice(&[1]).unwrap());
        assert!(BoundedVec::<u8, 1>::from_slice(&[1, 2]).is_err());
    }

    #[test]
    fn to_id_matches_ps() {
        assert_eq!(to_id::<24>("Mr. Mime").unwrap().as_str(), "mrmime");
        assert_eq!(to_id::<24>("Farfetch’d").unwrap().as_str(), "farfetchd");
        assert_eq!(to_id::<24>("Hidden Power Fire 70").unwrap().as_str(), "hiddenpowerfire70");
        assert_eq!(to_id::<24>("Soft-Boiled").unwrap().as_str(), "softboiled");
    }

    #[test]
    fn side_ids() {
        assert_eq!(SideId::parse("p2"), Some(SideId::P2));
        assert_eq!(SideId::P1.foe(), SideId::P2);
        assert_eq!(SideId::from_index(1), Some(SideId::P2));
        assert_eq!(SideId::P2.as_str(), "p2");
    }

    #[test]
    fn health_forms() {
        // PS getHealth with HP Percentage Mod.
        assert_eq!(Hp::percent(1, 300), Hp::new(1, 100));
        assert_eq!(Hp::percent(299, 300), Hp::new(99, 100), "never 100 while not full");
        assert_eq!(Hp::percent(300, 300), Hp::new(100, 100));
        assert_eq!(Hp::percent(150, 282), Hp::new(54, 100));
        // Without it: 48 pixels, at least 1.
        assert_eq!(Hp::pixels(1, 714), Hp::new(1, 48));
        assert_eq!(Hp::pixels(357, 714), Hp::new(24, 48));
        let h = Health::split_percent(213, 282, Some(Status::Par));
        assert!(h.split);
        assert_eq!(h.secret(), HpStatus { hp: Hp::new(213, 282), status: Some(Status::Par) });
        assert_eq!(h.shared(), HpStatus { hp: Hp::new(76, 100), status: Some(Status::Par) });
        assert_eq!(Health::split_percent(0, 282, Some(Status::Par)), Health::FAINTED_SPLIT);
        assert_eq!(HpStatus::exact(0, 100, None), HpStatus::FAINTED);
        let p = Health::plain(HpStatus::exact(40, 100, None));
        assert!(!p.split);
        assert_eq!(p.secret, p.shared);
    }

    #[test]
    fn stats_parse_back() {
        for s in Stat::ALL {
            assert_eq!(Stat::parse_abbr(s.abbr()), Some(s));
            assert_eq!(Stat::parse_name(s.name()), Some(s));
        }
    }
}
