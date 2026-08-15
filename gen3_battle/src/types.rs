//! Gen 3 typing: seventeen types, the chart, and the physical/special split.

/// Type-effectiveness multiplier x10: 0 = immune, 5 = half, 10 = neutral,
/// 20 = double. Same encoding `gen1_battle` uses, so callers that hold both
/// engines do not need two mental models.
pub type TypeEffectiveness = u8;

// The chart itself is generated; see `crate::data`.
use crate::data::{TYPE_CHART, TYPE_COUNT};

/// The seventeen Gen 3 types. Discriminants are the chart's row/column order,
/// which `build.rs` emits against; `None` is the absent second type of a
/// single-typed mon and never indexes the chart.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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
    None,
}

impl Type {
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
            Type::None => "---",
        }
    }

    /// Three-letter abbreviation, matching the ones the display layer already
    /// uses for Gen 1 so one badge renderer serves both.
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
            Type::None => "---",
        }
    }
}

/// Whether a move is physical or special.
///
/// In Gen 3 this is still a property of the move's TYPE, not of the move: the
/// per-move split arrived in Gen 4. Getting this wrong is the classic Gen 3
/// bug, so it lives in one function rather than in the move data.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Category {
    Physical,
    Special,
    /// Deals no damage. Kept separate so callers never ask which stat a
    /// status move attacks with.
    Status,
}

/// The damaging category for a type, under Gen 1 to Gen 3 rules.
pub fn category_of(t: Type) -> Category {
    match t {
        Type::Normal
        | Type::Fighting
        | Type::Flying
        | Type::Poison
        | Type::Ground
        | Type::Rock
        | Type::Bug
        | Type::Ghost
        | Type::Steel => Category::Physical,
        Type::Fire
        | Type::Water
        | Type::Grass
        | Type::Electric
        | Type::Psychic
        | Type::Ice
        | Type::Dragon
        | Type::Dark => Category::Special,
        // Typeless DAMAGE is physical in this era — Struggle is the one
        // case, since zero-power moves resolve as Status before this.
        Type::None => Category::Physical,
    }
}

/// Effectiveness of `attacking` against one defending type, x10.
pub fn effectiveness(attacking: Type, defending: Type) -> TypeEffectiveness {
    let (a, d) = (attacking as usize, defending as usize);
    if a < TYPE_COUNT && d < TYPE_COUNT {
        TYPE_CHART[a][d]
    } else {
        10
    }
}

/// Effectiveness against a whole mon, x100 (100 = neutral, 400 = double
/// super effective). Applied per defending type in order, as the games do.
pub fn effectiveness_against(attacking: Type, defender: (Type, Type)) -> u32 {
    let mut mult = effectiveness(attacking, defender.0) as u32 * 10;
    if defender.1 != Type::None && defender.1 != defender.0 {
        mult = mult * effectiveness(attacking, defender.1) as u32 / 10;
    }
    mult
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_chart_has_a_row_and_column_per_type() {
        assert_eq!(TYPE_COUNT, 17, "Gen 3 has seventeen types, and no Fairy");
        assert_eq!(TYPE_CHART.len(), TYPE_COUNT);
        assert!(TYPE_CHART.iter().all(|row| row.len() == TYPE_COUNT));
    }

    #[test]
    fn steel_still_resists_ghost_and_dark() {
        // The Gen 6 chart makes both neutral. Reading the modern chart without
        // this patch is the easiest way to ship a subtly wrong Gen 3.
        assert_eq!(effectiveness(Type::Ghost, Type::Steel), 5);
        assert_eq!(effectiveness(Type::Dark, Type::Steel), 5);
    }

    #[test]
    fn the_familiar_matchups_hold() {
        assert_eq!(effectiveness(Type::Water, Type::Fire), 20);
        assert_eq!(effectiveness(Type::Fire, Type::Water), 5);
        assert_eq!(effectiveness(Type::Normal, Type::Ghost), 0);
        assert_eq!(effectiveness(Type::Ghost, Type::Normal), 0);
        assert_eq!(effectiveness(Type::Electric, Type::Ground), 0);
        // Dark and Steel are the two types Gen 1 did not have.
        assert_eq!(effectiveness(Type::Dark, Type::Psychic), 20);
        assert_eq!(effectiveness(Type::Fighting, Type::Steel), 20);
    }

    #[test]
    fn dual_types_multiply() {
        // Rock/Ground against Water: 2x and 2x.
        assert_eq!(effectiveness_against(Type::Water, (Type::Rock, Type::Ground)), 400);
        // Fire against Water/Flying: half, then neutral.
        assert_eq!(effectiveness_against(Type::Fire, (Type::Water, Type::Flying)), 50);
        // An immunity on either half wins outright.
        assert_eq!(effectiveness_against(Type::Ground, (Type::Flying, Type::Steel)), 0);
        // A single-typed mon only counts its one type.
        assert_eq!(effectiveness_against(Type::Water, (Type::Fire, Type::None)), 200);
    }

    #[test]
    fn the_split_follows_the_type_not_the_move() {
        assert_eq!(category_of(Type::Ghost), Category::Physical);
        assert_eq!(category_of(Type::Dark), Category::Special);
        assert_eq!(category_of(Type::Steel), Category::Physical);
        assert_eq!(category_of(Type::Ice), Category::Special);
    }
}
