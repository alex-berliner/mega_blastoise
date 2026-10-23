//! Identifiers: sides, Pokemon, details strings, HP and status, dex numbers.
//!
//! PS: `sim/SIM-PROTOCOL.md` "Identifying Pokemon", `sim/pokemon.ts`
//! (`toString`, `fullname`, `getUpdatedDetails`, `getHealth`).

/// Dex numbers. Each wraps the same `u16` (or `u8`) index `ps_dex` assigns
/// (PS's iteration order of the resolved `Dex.mod(genN)` table), so converting
/// to and from the dex's own id newtypes is a plain field copy. They live here
/// so the protocol does not depend on the dex.
macro_rules! dex_num {
    ($(#[$doc:meta])* $name:ident($inner:ty)) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub struct $name(pub $inner);
    };
}

dex_num!(
    /// A move, by its `ps_dex` number.
    MoveNum(u16)
);
dex_num!(
    /// A species (forme included), by its `ps_dex` number.
    SpeciesNum(u16)
);
dex_num!(
    /// An item, by its `ps_dex` number.
    ItemNum(u16)
);
dex_num!(
    /// An ability, by its `ps_dex` number (`noability` is an ability in PS).
    AbilityNum(u16)
);
dex_num!(
    /// A type, by its `ps_dex` number.
    TypeNum(u8)
);
dex_num!(
    /// A nature, by its `ps_dex` number.
    NatureNum(u8)
);

/// A player slot. Renders `p1` / `p2`; as a side reference in minor actions
/// (`-sidestart|p1: NAME|...`) it renders `p1: ` plus the player's name.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SideId {
    #[default]
    P1,
    P2,
}

impl SideId {
    /// `"p1"` or `"p2"`.
    pub const fn as_str(self) -> &'static str {
        match self {
            SideId::P1 => "p1",
            SideId::P2 => "p2",
        }
    }

    /// 0 for p1, 1 for p2 (PS `side.n`).
    pub const fn index(self) -> usize {
        self as usize
    }

    pub const fn from_index(n: usize) -> Option<SideId> {
        match n {
            0 => Some(SideId::P1),
            1 => Some(SideId::P2),
            _ => None,
        }
    }

    pub fn parse(s: &str) -> Option<SideId> {
        match s {
            "p1" => Some(SideId::P1),
            "p2" => Some(SideId::P2),
            _ => None,
        }
    }

    pub const fn foe(self) -> SideId {
        match self {
            SideId::P1 => SideId::P2,
            SideId::P2 => SideId::P1,
        }
    }
}

/// A Pokemon ID, PS `POSITION: NAME`.
///
/// Encoding: `p1a: NAME` when `active` (PS `pokemon.toString()` while
/// `isActive`; the position letter is always `a`, singles only), else
/// `p1: NAME` (PS `fullname`, used for inactive Pokemon and in requests).
/// `NAME` is the nickname of team slot `slot` from [`crate::BattleNames`].
///
/// `slot` is the 0-based index in the team as `>player` gave it, which is
/// stable; PS's `side.pokemon` reorders on every switch, so the request's
/// list order is not a stable address. Two Pokemon on one side with the same
/// nickname render alike (PS allows it outside Species Clause); a decoder
/// resolves a name to the first matching slot, as PS's `switch NAME` does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct PokemonIdent {
    pub side: SideId,
    pub slot: u8,
    pub active: bool,
}

impl PokemonIdent {
    /// `p1a: NAME`, a Pokemon in an active position.
    pub const fn active(side: SideId, slot: u8) -> Self {
        Self { side, slot, active: true }
    }

    /// `p1: NAME`, a Pokemon off the field (or any Pokemon in a request).
    pub const fn inactive(side: SideId, slot: u8) -> Self {
        Self { side, slot, active: false }
    }
}

/// Gender as PS stores it (`pokemon.gender`): `M`, `F`, or genderless (`N`,
/// which PS stores as the empty string in battle and renders as nothing).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Gender {
    M,
    F,
    #[default]
    N,
}

impl Gender {
    /// The letter in a details string, or `None` for genderless.
    pub const fn details_letter(self) -> Option<&'static str> {
        match self {
            Gender::M => Some("M"),
            Gender::F => Some("F"),
            Gender::N => None,
        }
    }
}

/// A details string, PS `pokemon.details` (`getUpdatedDetails`).
///
/// Encoding: `SPECIES`, then `, L{level}` unless level is 100, then `, M` or
/// `, F` unless genderless, then `, shiny` if shiny. `SPECIES` is
/// `species.name`: the species' display name (`Castform-Sunny`, a forme with
/// its own dex entry), or with `forme` set the cosmetic forme's name
/// ([`crate::DexNames::cosmetic_forme_name`], `Unown-T`). Gen 9's
/// `, tera:TYPE` is out of scope.
///
/// Packed (alignment 1, 7 bytes) so that [`crate::Msg::Switch`] and
/// [`crate::Msg::Drag`], which carry a `Details`, a [`Health`], a Pokemon and
/// an [`crate::Arg`], stay inside the 24-byte `Msg`. Read the fields by value
/// (`d.level`); a reference to `species` or `level` would be unaligned.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C, packed)]
pub struct Details {
    pub species: SpeciesNum,
    /// PS levels are 1 to 100 in our formats; custom games allow up to 9999.
    pub level: u16,
    pub gender: Gender,
    pub shiny: bool,
    /// The cosmetic forme the details print, PS `species.cosmeticFormes`:
    /// 0 for none (the species' own name), else 1 plus the forme's index in
    /// that list (Unown-B is 1, Unown-Question is 27). PS builds a cosmetic
    /// forme as its own species object when a set names one (`Unown-T`), but
    /// the dex lists only the base species, so the forme rides here.
    pub forme: u8,
}

#[cfg(feature = "defmt")]
impl defmt::Format for Details {
    fn format(&self, f: defmt::Formatter) {
        let (species, level, gender, shiny, forme) =
            (self.species, self.level, self.gender, self.shiny, self.forme);
        defmt::write!(
            f,
            "Details {{ species: {}, level: {=u16}, gender: {}, shiny: {=bool}, forme: {=u8} }}",
            species,
            level,
            gender,
            shiny,
            forme
        )
    }
}

impl Details {
    /// A details string with no cosmetic forme.
    pub const fn new(species: SpeciesNum, level: u16, gender: Gender, shiny: bool) -> Self {
        Self { species, level, gender, shiny, forme: 0 }
    }
}

/// A major status condition, PS `pokemon.status`. `Fnt` only appears in HP
/// strings (`0 fnt`), never as `pokemon.status`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Status {
    Brn,
    Par,
    Slp,
    Frz,
    Psn,
    Tox,
    Fnt,
}

impl Status {
    pub const fn as_str(self) -> &'static str {
        match self {
            Status::Brn => "brn",
            Status::Par => "par",
            Status::Slp => "slp",
            Status::Frz => "frz",
            Status::Psn => "psn",
            Status::Tox => "tox",
            Status::Fnt => "fnt",
        }
    }

    pub fn parse(s: &str) -> Option<Status> {
        Some(match s {
            "brn" => Status::Brn,
            "par" => Status::Par,
            "slp" => Status::Slp,
            "frz" => Status::Frz,
            "psn" => Status::Psn,
            "tox" => Status::Tox,
            "fnt" => Status::Fnt,
            _ => return None,
        })
    }
}

/// A boostable stat: the `STAT` of `-boost` / `-unboost` / `-setboost`, or a
/// request stats key (`Atk` to `Spe`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Stat {
    Atk,
    Def,
    Spa,
    Spd,
    Spe,
    Accuracy,
    Evasion,
}

impl Stat {
    pub const ALL: [Stat; 7] =
        [Stat::Atk, Stat::Def, Stat::Spa, Stat::Spd, Stat::Spe, Stat::Accuracy, Stat::Evasion];

    /// PS's `BoostID`, as `-boost` writes it.
    pub const fn abbr(self) -> &'static str {
        match self {
            Stat::Atk => "atk",
            Stat::Def => "def",
            Stat::Spa => "spa",
            Stat::Spd => "spd",
            Stat::Spe => "spe",
            Stat::Accuracy => "accuracy",
            Stat::Evasion => "evasion",
        }
    }

    /// The literal PS handlers write as a stat's name (`-fail|...|unboost|Attack`
    /// from Hyper Cutter, `...|unboost|accuracy` from Keen Eye): `Dex.stats.names`
    /// for the five stats, the bare id for accuracy and evasion.
    pub const fn name(self) -> &'static str {
        match self {
            Stat::Atk => "Attack",
            Stat::Def => "Defense",
            Stat::Spa => "Special Attack",
            Stat::Spd => "Special Defense",
            Stat::Spe => "Speed",
            Stat::Accuracy => "accuracy",
            Stat::Evasion => "evasion",
        }
    }

    pub fn parse_abbr(s: &str) -> Option<Stat> {
        Stat::ALL.into_iter().find(|st| st.abbr() == s)
    }

    pub fn parse_name(s: &str) -> Option<Stat> {
        Stat::ALL.into_iter().find(|st| st.name() == s)
    }
}

/// One HP fraction as PS writes it: `CUR/MAX`, or just `CUR` when `max` is 0
/// (PS writes a fainted Pokemon's HP as `0 fnt`, with no maximum).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Hp {
    pub cur: u16,
    pub max: u16,
}

impl Hp {
    pub const fn new(cur: u16, max: u16) -> Self {
        Self { cur, max }
    }

    /// The `0` of `0 fnt`.
    pub const FAINTED: Hp = Hp { cur: 0, max: 0 };

    /// PS `getHealth` under HP Percentage Mod (`reportPercentages`):
    /// `ceil(100 * hp / maxhp)`, shown as 99 rather than 100 while not full.
    pub fn percent(hp: u16, maxhp: u16) -> Hp {
        let (hp32, max32) = (hp as u32, maxhp as u32);
        let mut pct = (100 * hp32).div_ceil(max32);
        if pct == 100 && hp32 < max32 {
            pct = 99;
        }
        Hp { cur: pct as u16, max: 100 }
    }

    /// PS `getHealth` without HP Percentage Mod: `floor(48 * hp / maxhp) || 1`
    /// pixels out of 48 (the gen 5+ colour suffixes do not apply to gens 1-3).
    pub fn pixels(hp: u16, maxhp: u16) -> Hp {
        let px = (48 * hp as u32) / maxhp as u32;
        Hp { cur: if px == 0 { 1 } else { px as u16 }, max: 48 }
    }
}

/// `HP STATUS`, one form: the request's `condition`, or a line as one viewer
/// sees it. Encoding: [`Hp`], then a space and the status if any.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct HpStatus {
    pub hp: Hp,
    pub status: Option<Status>,
}

impl HpStatus {
    /// PS `getHealth().secret`: `0 fnt` at 0 HP, else `hp/maxhp[ status]`.
    pub fn exact(hp: u16, maxhp: u16, status: Option<Status>) -> Self {
        if hp == 0 {
            return Self::FAINTED;
        }
        Self { hp: Hp::new(hp, maxhp), status }
    }

    pub const FAINTED: HpStatus = HpStatus { hp: Hp::FAINTED, status: Some(Status::Fnt) };
}

/// The `HP STATUS` of an HP-bearing line (`switch`, `drag`, `detailschange`,
/// `-damage`, `-heal`, `-sethp`), carrying both of PS's forms.
///
/// PS builds these lines with `getHealth` as a function part, so `add()`
/// always routes them through `addSplit`: the omniscient stream gets
/// `|split|pN`, the line with the secret (exact) HP, then the line with the
/// shared HP, where `pN` is the side of the Pokemon the line is about. Both
/// forms share `status`.
///
/// `split == false` is a line as one viewer received it (a player or
/// spectator stream from a real PS server): one line, `secret` holds what was
/// shown and `shared` equals it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Health {
    pub secret: Hp,
    pub shared: Hp,
    pub status: Option<Status>,
    pub split: bool,
}

impl Health {
    /// The split pair PS emits, from the Pokemon's exact HP and the shared
    /// form the format reports ([`Hp::percent`] or [`Hp::pixels`]). At 0 HP
    /// both forms are `0 fnt`, as `getHealth` returns.
    pub fn split(hp: u16, maxhp: u16, shared: Hp, status: Option<Status>) -> Self {
        if hp == 0 {
            return Self::FAINTED_SPLIT;
        }
        Self { secret: Hp::new(hp, maxhp), shared, status, split: true }
    }

    /// `split` with HP Percentage Mod's shared form (both our formats have it).
    pub fn split_percent(hp: u16, maxhp: u16, status: Option<Status>) -> Self {
        Self::split(hp, maxhp, Hp::percent(hp, maxhp), status)
    }

    /// One unsplit form, as a single viewer sees it.
    pub fn plain(hs: HpStatus) -> Self {
        Self { secret: hs.hp, shared: hs.hp, status: hs.status, split: false }
    }

    pub const FAINTED_SPLIT: Health =
        Health { secret: Hp::FAINTED, shared: Hp::FAINTED, status: Some(Status::Fnt), split: true };

    /// The exact form (`hp/maxhp status`).
    pub fn secret(&self) -> HpStatus {
        HpStatus { hp: self.secret, status: self.status }
    }

    /// The public form (`pct/100 status`).
    pub fn shared(&self) -> HpStatus {
        HpStatus { hp: self.shared, status: self.status }
    }
}
