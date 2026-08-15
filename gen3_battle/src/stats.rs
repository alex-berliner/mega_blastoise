//! Gen 3 stat calculation: five stats plus HP, IVs, EVs and natures.
//!
//! This is the first place Gen 3 stops being Gen 1. Special is two stats, not
//! one; every mon carries per-stat IVs (0..=31) and EVs (0..=255, 510 total);
//! and a nature raises one stat by 10% while lowering another. The formulas
//! floor at every step, so they are written in integer arithmetic throughout
//! rather than with floats rounded at the end.

/// The five stats a nature can move, in the canonical Gen 3 order. HP is not
/// among them: no nature touches it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stat {
    Atk,
    Def,
    Spe,
    SpAtk,
    SpDef,
}

/// The 25 natures, ordered so that `index = 5 * raised + lowered` over
/// [`Stat`]'s order. The five on that diagonal raise and lower the same stat
/// and so do nothing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Nature {
    Hardy = 0,
    Lonely,
    Brave,
    Adamant,
    Naughty,
    Bold,
    Docile,
    Relaxed,
    Impish,
    Lax,
    Timid,
    Hasty,
    Serious,
    Jolly,
    Naive,
    Modest,
    Mild,
    Quiet,
    Bashful,
    Rash,
    Calm,
    Gentle,
    Sassy,
    Careful,
    Quirky,
}

const STAT_ORDER: [Stat; 5] = [Stat::Atk, Stat::Def, Stat::Spe, Stat::SpAtk, Stat::SpDef];

impl Nature {
    /// From a 0..25 index, which is how the games store it (personality
    /// value mod 25). Anything out of range is treated as neutral.
    pub fn from_index(i: u8) -> Nature {
        if i >= 25 {
            return Nature::Hardy;
        }
        // SAFETY-free equivalent of a transmute: the discriminants are 0..=24
        // and contiguous, so a match on the index is exhaustive by table.
        NATURES[i as usize]
    }

    /// The stat this nature raises by 10%, and the one it lowers by 10%.
    /// Equal means neutral.
    pub fn effect(self) -> (Stat, Stat) {
        let i = self as usize;
        (STAT_ORDER[i / 5], STAT_ORDER[i % 5])
    }

    pub fn is_neutral(self) -> bool {
        let (up, down) = self.effect();
        up == down
    }

    /// Multiplier for `stat`, as a numerator over 10.
    pub fn modifier(self, stat: Stat) -> u32 {
        let (up, down) = self.effect();
        if up == down {
            10
        } else if stat == up {
            11
        } else if stat == down {
            9
        } else {
            10
        }
    }

    pub fn name(self) -> &'static str {
        NATURE_NAMES[self as usize]
    }
}

static NATURES: [Nature; 25] = [
    Nature::Hardy,
    Nature::Lonely,
    Nature::Brave,
    Nature::Adamant,
    Nature::Naughty,
    Nature::Bold,
    Nature::Docile,
    Nature::Relaxed,
    Nature::Impish,
    Nature::Lax,
    Nature::Timid,
    Nature::Hasty,
    Nature::Serious,
    Nature::Jolly,
    Nature::Naive,
    Nature::Modest,
    Nature::Mild,
    Nature::Quiet,
    Nature::Bashful,
    Nature::Rash,
    Nature::Calm,
    Nature::Gentle,
    Nature::Sassy,
    Nature::Careful,
    Nature::Quirky,
];

static NATURE_NAMES: [&str; 25] = [
    "Hardy", "Lonely", "Brave", "Adamant", "Naughty", "Bold", "Docile", "Relaxed", "Impish",
    "Lax", "Timid", "Hasty", "Serious", "Jolly", "Naive", "Modest", "Mild", "Quiet", "Bashful",
    "Rash", "Calm", "Gentle", "Sassy", "Careful", "Quirky",
];

/// One mon's investment in a single stat.
#[derive(Clone, Copy, Debug, Default)]
pub struct Invest {
    /// 0..=31.
    pub iv: u8,
    /// 0..=255, and at most 510 across all six stats.
    pub ev: u8,
}

/// HP at `level`. HP takes no nature and uses its own tail.
///
/// A base HP of 1 IS 1 at every level and investment: that is Shedinja's
/// rule, and the formula would otherwise hand it a liveable number. Found by
/// the Showdown parity suite.
pub fn hp_stat(base: u16, inv: Invest, level: u8) -> u16 {
    if base == 1 {
        return 1;
    }
    let common = 2 * base as u32 + inv.iv as u32 + (inv.ev as u32 / 4);
    ((common * level as u32) / 100 + level as u32 + 10) as u16
}

/// Any stat other than HP, including the nature's 10%.
pub fn other_stat(base: u16, inv: Invest, level: u8, nature: Nature, stat: Stat) -> u16 {
    let common = 2 * base as u32 + inv.iv as u32 + (inv.ev as u32 / 4);
    let pre = (common * level as u32) / 100 + 5;
    // The nature multiplies last and floors, which is why 1.1 cannot be
    // folded into the line above.
    ((pre * nature.modifier(stat)) / 10) as u16
}

/// Apply a stat stage (-6..=6) the Gen 3 way: a ratio, not a percentage.
pub fn apply_stage(stat: u16, stage: i8) -> u16 {
    let s = stage.clamp(-6, 6);
    let (num, den): (u32, u32) = if s >= 0 { (2 + s as u32, 2) } else { (2, 2 + (-s) as u32) };
    ((stat as u32 * num) / den) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAXED: Invest = Invest { iv: 31, ev: 252 };
    const BLANK: Invest = Invest { iv: 0, ev: 0 };

    #[test]
    fn natures_pair_up_the_way_the_games_index_them() {
        assert!(Nature::Hardy.is_neutral());
        assert!(Nature::Serious.is_neutral());
        assert!(Nature::Quirky.is_neutral());
        assert_eq!(Nature::Adamant.effect(), (Stat::Atk, Stat::SpAtk));
        assert_eq!(Nature::Modest.effect(), (Stat::SpAtk, Stat::Atk));
        assert_eq!(Nature::Jolly.effect(), (Stat::Spe, Stat::SpAtk));
        assert_eq!(Nature::Timid.effect(), (Stat::Spe, Stat::Atk));
        assert_eq!(Nature::from_index(3), Nature::Adamant);
        assert_eq!(Nature::from_index(99), Nature::Hardy, "out of range is neutral");
    }

    #[test]
    fn natures_only_move_their_own_two_stats() {
        assert_eq!(Nature::Adamant.modifier(Stat::Atk), 11);
        assert_eq!(Nature::Adamant.modifier(Stat::SpAtk), 9);
        assert_eq!(Nature::Adamant.modifier(Stat::Spe), 10);
        assert_eq!(Nature::Hardy.modifier(Stat::Atk), 10, "a neutral nature moves nothing");
    }

    /// Hand-checked against the standard Gen 3 formulas.
    #[test]
    fn stats_match_the_worked_examples() {
        // Blissey, base 255 HP, level 100, maxed: 2*255+31+63 = 604;
        // 604*100/100 = 604; +100+10 = 714.
        assert_eq!(hp_stat(255, MAXED, 100), 714);
        // A level 50 mon with base 100 HP and nothing invested:
        // 200*50/100 = 100; +50+10 = 160.
        assert_eq!(hp_stat(100, BLANK, 50), 160);
        // Shedinja: base 1 means 1, not what the formula says.
        assert_eq!(hp_stat(1, MAXED, 100), 1);

        // Base 100 attack, level 100, maxed, neutral: 2*100+31+63 = 294;
        // 294*100/100 = 294; +5 = 299.
        assert_eq!(other_stat(100, MAXED, 100, Nature::Hardy, Stat::Atk), 299);
        // The same mon, Adamant: floor(299 * 11/10) = 328.
        assert_eq!(other_stat(100, MAXED, 100, Nature::Adamant, Stat::Atk), 328);
        // And its Sp.Atk is lowered: floor(299 * 9/10) = 269.
        assert_eq!(other_stat(100, MAXED, 100, Nature::Adamant, Stat::SpAtk), 269);
    }

    #[test]
    fn ev_investment_only_counts_in_fours() {
        // 4 EVs is one point at level 100; 3 is none.
        let three = other_stat(100, Invest { iv: 31, ev: 3 }, 100, Nature::Hardy, Stat::Atk);
        let four = other_stat(100, Invest { iv: 31, ev: 4 }, 100, Nature::Hardy, Stat::Atk);
        assert_eq!(four, three + 1);
    }

    #[test]
    fn stages_are_ratios() {
        assert_eq!(apply_stage(200, 0), 200);
        assert_eq!(apply_stage(200, 1), 300);
        assert_eq!(apply_stage(200, 2), 400);
        assert_eq!(apply_stage(200, 6), 800);
        assert_eq!(apply_stage(200, -1), 133);
        assert_eq!(apply_stage(200, -6), 50);
        assert_eq!(apply_stage(200, 9), apply_stage(200, 6), "stages clamp at six");
    }
}
