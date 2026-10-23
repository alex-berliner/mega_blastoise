//! The vocabulary the dex tables are written in: types, genders, statuses,
//! move categories and move targets.
//!
//! These are the enums PS's data files spell out in words ("Physical",
//! "foeSide", "N"), so the tables can hold them as one byte each.

/// A Pokemon type. `None` is the empty second slot of a single-typed mon.
///
/// The order is load-bearing. The type chart of every generation is emitted
/// in this order, so a new variant belongs at the END of the real types and
/// before `None`, never in the middle.
///
/// A generation that lacks a type simply never produces it. The chart of a
/// generation that does not have a type reads as neutral for it, which is
/// exactly right: Gen 3 has no Fairy, and asking about one gets 1x back
/// rather than a panic or a wrong row.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Type {
    Normal = 0,
    Fire,
    Water,
    Electric,
    Grass,
    Ice,
    Fighting,
    Poison,
    Ground,
    Flying,
    Psychic,
    Bug,
    Rock,
    Ghost,
    Dragon,
    Dark,
    Steel,
    Fairy,
    None,
}

/// The number of [`Type`] variants, `None` included. Every chart is
/// `TYPE_COUNT` square.
pub const TYPE_COUNT: usize = 19;

impl Type {
    /// The type's printed name, as the dex spells it.
    pub fn name(self) -> &'static str {
        match self {
            Type::Normal => "Normal",
            Type::Fire => "Fire",
            Type::Water => "Water",
            Type::Electric => "Electric",
            Type::Grass => "Grass",
            Type::Ice => "Ice",
            Type::Fighting => "Fighting",
            Type::Poison => "Poison",
            Type::Ground => "Ground",
            Type::Flying => "Flying",
            Type::Psychic => "Psychic",
            Type::Bug => "Bug",
            Type::Rock => "Rock",
            Type::Ghost => "Ghost",
            Type::Dragon => "Dragon",
            Type::Dark => "Dark",
            Type::Steel => "Steel",
            Type::Fairy => "Fairy",
            Type::None => "",
        }
    }

    /// Three-letter label for the badges and chips both renderers draw.
    pub fn abbr(self) -> &'static str {
        match self {
            Type::Normal => "NRM",
            Type::Fire => "FIR",
            Type::Water => "WAT",
            Type::Electric => "ELC",
            Type::Grass => "GRS",
            Type::Ice => "ICE",
            Type::Fighting => "FGT",
            Type::Poison => "PSN",
            Type::Ground => "GND",
            Type::Flying => "FLY",
            Type::Psychic => "PSY",
            Type::Bug => "BUG",
            Type::Rock => "RCK",
            Type::Ghost => "GHO",
            Type::Dragon => "DRG",
            Type::Dark => "DRK",
            Type::Steel => "STL",
            Type::Fairy => "FAI",
            Type::None => "",
        }
    }

    /// The type's index in the chart, which is its discriminant.
    pub fn index(self) -> u8 {
        self as u8
    }

    /// The name PS's data and protocol use: [`Type::name`], except that
    /// [`Type::None`] is PS's `???`.
    pub fn ps_name(self) -> &'static str {
        match self {
            Type::None => "???",
            other => other.name(),
        }
    }

    /// Every variant in discriminant order, `None` last.
    pub const ALL: [Type; TYPE_COUNT] = [
        Type::Normal,
        Type::Fire,
        Type::Water,
        Type::Electric,
        Type::Grass,
        Type::Ice,
        Type::Fighting,
        Type::Poison,
        Type::Ground,
        Type::Flying,
        Type::Psychic,
        Type::Bug,
        Type::Rock,
        Type::Ghost,
        Type::Dragon,
        Type::Dark,
        Type::Steel,
        Type::Fairy,
        Type::None,
    ];

    /// The type PS's data files spell this way. PS's typeless `???` (the
    /// type of gen 1 Bide and gen 3 Curse) is [`Type::None`]: like `???`, it
    /// has no row or column in any chart, so it is neutral against
    /// everything and immune to nothing (`Dex.getEffectiveness` /
    /// `getImmunity` find no `damageTaken` entry for it).
    pub fn from_name(name: &str) -> Option<Type> {
        Some(match name {
            "???" => Type::None,
            "Normal" => Type::Normal,
            "Fire" => Type::Fire,
            "Water" => Type::Water,
            "Electric" => Type::Electric,
            "Grass" => Type::Grass,
            "Ice" => Type::Ice,
            "Fighting" => Type::Fighting,
            "Poison" => Type::Poison,
            "Ground" => Type::Ground,
            "Flying" => Type::Flying,
            "Psychic" => Type::Psychic,
            "Bug" => Type::Bug,
            "Rock" => Type::Rock,
            "Ghost" => Type::Ghost,
            "Dragon" => Type::Dragon,
            "Dark" => Type::Dark,
            "Steel" => Type::Steel,
            "Fairy" => Type::Fairy,
            _ => return None,
        })
    }
}

/// A species' gender, as PS spells it: `M`, `F`, `N` (genderless) or the
/// empty string (either).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gender {
    /// Either: PS samples one per Pokemon.
    Either = 0,
    Male,
    Female,
    /// Genderless (`N`).
    Neutral,
}

impl Gender {
    /// The letter PS prints for this gender.
    pub fn letter(self) -> &'static str {
        match self {
            Gender::Either => "",
            Gender::Male => "M",
            Gender::Female => "F",
            Gender::Neutral => "N",
        }
    }
}

/// A status condition. PS's ids: `brn`, `par`, `slp`, `frz`, `psn`, `tox`.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Burn = 0,
    Paralysis,
    Sleep,
    Freeze,
    Poison,
    Toxic,
}

impl Status {
    /// The status' PS id.
    pub fn id(self) -> &'static str {
        match self {
            Status::Burn => "brn",
            Status::Paralysis => "par",
            Status::Sleep => "slp",
            Status::Freeze => "frz",
            Status::Poison => "psn",
            Status::Toxic => "tox",
        }
    }

    /// The status PS's data files spell this way.
    pub fn from_id(id: &str) -> Option<Status> {
        Some(match id {
            "brn" => Status::Burn,
            "par" => Status::Paralysis,
            "slp" => Status::Sleep,
            "frz" => Status::Freeze,
            "psn" => Status::Poison,
            "tox" => Status::Toxic,
            _ => return None,
        })
    }
}

/// What a move does to its target.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveCategory {
    Physical = 0,
    Special,
    Status,
}

impl MoveCategory {
    /// PS's spelling of this category.
    pub fn name(self) -> &'static str {
        match self {
            MoveCategory::Physical => "Physical",
            MoveCategory::Special => "Special",
            MoveCategory::Status => "Status",
        }
    }

    pub fn from_name(name: &str) -> Option<MoveCategory> {
        Some(match name {
            "Physical" => MoveCategory::Physical,
            "Special" => MoveCategory::Special,
            "Status" => MoveCategory::Status,
            _ => return None,
        })
    }
}

/// Who a move aims at. PS's `target` field.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveTarget {
    Normal = 0,
    Self_,
    AllAdjacentFoes,
    All,
    FoeSide,
    AllySide,
    AllyTeam,
    Allies,
    Any,
    RandomNormal,
    Scripted,
    AdjacentFoe,
    AdjacentAlly,
    AdjacentAllyOrSelf,
    AllAdjacent,
}

impl MoveTarget {
    /// PS's spelling of this target.
    pub fn id(self) -> &'static str {
        match self {
            MoveTarget::Normal => "normal",
            MoveTarget::Self_ => "self",
            MoveTarget::AllAdjacentFoes => "allAdjacentFoes",
            MoveTarget::All => "all",
            MoveTarget::FoeSide => "foeSide",
            MoveTarget::AllySide => "allySide",
            MoveTarget::AllyTeam => "allyTeam",
            MoveTarget::Allies => "allies",
            MoveTarget::Any => "any",
            MoveTarget::RandomNormal => "randomNormal",
            MoveTarget::Scripted => "scripted",
            MoveTarget::AdjacentFoe => "adjacentFoe",
            MoveTarget::AdjacentAlly => "adjacentAlly",
            MoveTarget::AdjacentAllyOrSelf => "adjacentAllyOrSelf",
            MoveTarget::AllAdjacent => "allAdjacent",
        }
    }

    /// PS's spelling of this target, read back.
    pub fn from_id(id: &str) -> Option<MoveTarget> {
        Some(match id {
            "normal" => MoveTarget::Normal,
            "self" => MoveTarget::Self_,
            "allAdjacentFoes" => MoveTarget::AllAdjacentFoes,
            "all" => MoveTarget::All,
            "foeSide" => MoveTarget::FoeSide,
            "allySide" => MoveTarget::AllySide,
            "allyTeam" => MoveTarget::AllyTeam,
            "allies" => MoveTarget::Allies,
            "any" => MoveTarget::Any,
            "randomNormal" => MoveTarget::RandomNormal,
            "scripted" => MoveTarget::Scripted,
            "adjacentFoe" => MoveTarget::AdjacentFoe,
            "adjacentAlly" => MoveTarget::AdjacentAlly,
            "adjacentAllyOrSelf" => MoveTarget::AdjacentAllyOrSelf,
            "allAdjacent" => MoveTarget::AllAdjacent,
            _ => return None,
        })
    }
}

/// PS's `move.selfdestruct`: the user faints `always`, or only `ifHit`.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelfDestruct {
    Always = 0,
    IfHit,
}

impl SelfDestruct {
    pub fn id(self) -> &'static str {
        match self {
            SelfDestruct::Always => "always",
            SelfDestruct::IfHit => "ifHit",
        }
    }
}

/// PS's `move.selfSwitch`: `true`, or `'copyvolatile'` (Baton Pass).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelfSwitch {
    True = 0,
    CopyVolatile,
}

/// PS's `condition.effectType` after `new Condition()`: `Weather`, `Status`
/// or (anything else) `Condition`.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConditionType {
    Condition = 0,
    Weather,
    Status,
}

impl ConditionType {
    pub fn id(self) -> &'static str {
        match self {
            ConditionType::Condition => "Condition",
            ConditionType::Weather => "Weather",
            ConditionType::Status => "Status",
        }
    }
}

/// Where `dex.conditions.getByID` finds a condition: the Conditions table,
/// or the `condition` field of a move, an ability or an item (looked up in
/// that order, the first one wins).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConditionSource {
    Condition = 0,
    Move,
    Ability,
    Item,
}

impl ConditionSource {
    pub fn id(self) -> &'static str {
        match self {
            ConditionSource::Condition => "condition",
            ConditionSource::Move => "move",
            ConditionSource::Ability => "ability",
            ConditionSource::Item => "item",
        }
    }
}
