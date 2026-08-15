//! Gen 3 damage.
//!
//! The order the modifiers apply in is part of the mechanics, not an
//! implementation detail: each step floors, so moving one changes results by a
//! point or two. Gen 3 applies them as
//!
//!   base -> burn and screens -> critical hit -> random roll -> STAB -> type
//!
//! and this module does the same, in integer arithmetic, flooring where the
//! games floor.
//!
//! The random roll is an input rather than something drawn here, so a caller
//! can replay a turn, and so tests can pin an exact number.

use crate::stats::apply_stage;
use crate::types::{category_of, effectiveness_against, Category, Type};

/// The side using the move.
#[derive(Clone, Copy, Debug)]
pub struct Attacker {
    pub level: u8,
    pub atk: u16,
    pub sp_atk: u16,
    pub atk_stage: i8,
    pub sp_atk_stage: i8,
    pub types: (Type, Type),
    /// Burn halves physical damage in Gen 3.
    pub burned: bool,
}

/// The side taking it.
#[derive(Clone, Copy, Debug)]
pub struct Defender {
    pub def: u16,
    pub sp_def: u16,
    pub def_stage: i8,
    pub sp_def_stage: i8,
    pub types: (Type, Type),
    pub reflect: bool,
    pub light_screen: bool,
}

/// The move being used. Category is derived from the type, per Gen 3, unless
/// the move deals no damage at all.
#[derive(Clone, Copy, Debug)]
pub struct MoveUse {
    pub move_type: Type,
    pub power: u16,
}

impl MoveUse {
    pub fn category(&self) -> Category {
        if self.power == 0 {
            Category::Status
        } else {
            category_of(self.move_type)
        }
    }
}

/// The two rolls a turn makes, supplied by the caller.
#[derive(Clone, Copy, Debug)]
pub struct Roll {
    pub crit: bool,
    /// 85..=100. Anything outside is clamped.
    pub random: u8,
}

impl Roll {
    /// The highest roll, no crit — the number a damage calculator quotes as
    /// "max".
    pub const MAX: Roll = Roll { crit: false, random: 100 };
    /// The lowest.
    pub const MIN: Roll = Roll { crit: false, random: 85 };
}

/// Chance denominator of a critical hit at `stage`: 1 in N.
pub fn crit_denominator(stage: u8) -> u32 {
    match stage {
        0 => 16,
        1 => 8,
        2 => 4,
        3 => 3,
        _ => 2,
    }
}

/// Damage in HP. Returns 0 for a status move or a type immunity.
pub fn damage(a: &Attacker, d: &Defender, m: &MoveUse, roll: Roll) -> u32 {
    let category = m.category();
    if category == Category::Status || m.power == 0 {
        return 0;
    }
    let eff = effectiveness_against(m.move_type, d.types);
    if eff == 0 {
        return 0;
    }

    // A critical hit ignores stat stages that favour the defender: the
    // attacker's drops and the defender's boosts are both skipped.
    let (atk_stage, def_stage, sp_atk_stage, sp_def_stage) = if roll.crit {
        (
            a.atk_stage.max(0),
            d.def_stage.min(0),
            a.sp_atk_stage.max(0),
            d.sp_def_stage.min(0),
        )
    } else {
        (a.atk_stage, d.def_stage, a.sp_atk_stage, d.sp_def_stage)
    };

    let (attack, defence, screened) = match category {
        Category::Physical => (
            apply_stage(a.atk, atk_stage),
            apply_stage(d.def, def_stage),
            d.reflect,
        ),
        _ => (
            apply_stage(a.sp_atk, sp_atk_stage),
            apply_stage(d.sp_def, sp_def_stage),
            d.light_screen,
        ),
    };
    let defence = defence.max(1) as u32;

    // Base damage.
    let level_term = (2 * a.level as u32) / 5 + 2;
    let mut dmg = ((level_term * m.power as u32 * attack as u32 / defence) / 50) + 2;

    // Burn, then screens. A critical hit goes through a screen in Gen 3.
    if a.burned && category == Category::Physical {
        dmg /= 2;
    }
    if screened && !roll.crit {
        dmg /= 2;
    }

    if roll.crit {
        dmg *= 2;
    }

    // The roll, then STAB, then type. Each floors.
    let r = roll.random.clamp(85, 100) as u32;
    dmg = dmg * r / 100;
    if m.move_type == a.types.0 || m.move_type == a.types.1 {
        dmg = dmg * 3 / 2;
    }
    dmg = dmg * eff / 100;

    // A hit that connects always takes at least one HP.
    dmg.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attacker() -> Attacker {
        Attacker {
            level: 100,
            atk: 299,
            sp_atk: 299,
            atk_stage: 0,
            sp_atk_stage: 0,
            types: (Type::Normal, Type::None),
            burned: false,
        }
    }

    fn defender() -> Defender {
        Defender {
            def: 200,
            sp_def: 200,
            def_stage: 0,
            sp_def_stage: 0,
            types: (Type::Normal, Type::None),
            reflect: false,
            light_screen: false,
        }
    }

    const TACKLE: MoveUse = MoveUse { move_type: Type::Normal, power: 100 };

    /// Worked by hand: level term 42; 42*100*299/200 = 6279; /50 = 125; +2 = 127.
    #[test]
    fn base_damage_matches_the_hand_calculation() {
        let mut a = attacker();
        a.types = (Type::Fighting, Type::None); // no STAB on a Normal move
        assert_eq!(damage(&a, &defender(), &TACKLE, Roll::MAX), 127);
    }

    #[test]
    fn stab_and_effectiveness_stack_on_top() {
        // Same 127, with STAB: floor(127 * 3/2) = 190.
        assert_eq!(damage(&attacker(), &defender(), &TACKLE, Roll::MAX), 190);

        // Against a Rock defender a Normal move is halved: floor(190 * 50/100).
        let mut d = defender();
        d.types = (Type::Rock, Type::None);
        assert_eq!(damage(&attacker(), &d, &TACKLE, Roll::MAX), 95);
    }

    #[test]
    fn the_low_roll_is_eighty_five_percent() {
        // 127 * 85/100 = 107, then STAB: floor(107 * 3/2) = 160.
        assert_eq!(damage(&attacker(), &defender(), &TACKLE, Roll::MIN), 160);
        let out_of_range = Roll { crit: false, random: 3 };
        assert_eq!(
            damage(&attacker(), &defender(), &TACKLE, out_of_range),
            160,
            "a roll below the range clamps rather than dealing nothing",
        );
    }

    #[test]
    fn a_crit_doubles_and_ignores_the_stages_that_would_soften_it() {
        // 381, not 380: the crit doubles the BASE (127 -> 254) and STAB
        // floors afterwards, so it is not the same as doubling the 190 the
        // uncritical hit lands for. Order matters because every step floors.
        let crit = Roll { crit: true, random: 100 };
        assert_eq!(damage(&attacker(), &defender(), &TACKLE, crit), 381);

        // The defender is at +2 Def and the attacker at -2 Atk. Without a crit
        // that matters a great deal; with one, neither stage counts.
        let mut a = attacker();
        let mut d = defender();
        a.atk_stage = -2;
        d.def_stage = 2;
        assert!(damage(&a, &d, &TACKLE, Roll::MAX) < 190);
        assert_eq!(damage(&a, &d, &TACKLE, crit), 381);
    }

    #[test]
    fn the_split_decides_which_stats_are_read() {
        // A Dark move is special in Gen 3, so a physical wall does not help.
        let mut d = defender();
        d.def = 1000;
        d.sp_def = 100;
        let bite = MoveUse { move_type: Type::Dark, power: 100 };
        let physical = MoveUse { move_type: Type::Rock, power: 100 };
        assert!(damage(&attacker(), &d, &bite, Roll::MAX) > damage(&attacker(), &d, &physical, Roll::MAX));
    }

    #[test]
    fn burn_halves_physical_only() {
        let mut a = attacker();
        a.burned = true;
        let special = MoveUse { move_type: Type::Water, power: 100 };
        let burned_physical = damage(&a, &defender(), &TACKLE, Roll::MAX);
        let healthy_physical = damage(&attacker(), &defender(), &TACKLE, Roll::MAX);
        assert!(burned_physical < healthy_physical);
        assert_eq!(
            damage(&a, &defender(), &special, Roll::MAX),
            damage(&attacker(), &defender(), &special, Roll::MAX),
            "a burn does not touch special damage",
        );
    }

    #[test]
    fn screens_halve_but_a_crit_goes_through_them() {
        let mut d = defender();
        d.reflect = true;
        assert!(damage(&attacker(), &d, &TACKLE, Roll::MAX) < 190);
        let crit = Roll { crit: true, random: 100 };
        assert_eq!(damage(&attacker(), &d, &TACKLE, crit), 381);
    }

    #[test]
    fn immunity_and_status_deal_nothing() {
        let mut d = defender();
        d.types = (Type::Ghost, Type::None);
        assert_eq!(damage(&attacker(), &d, &TACKLE, Roll::MAX), 0);
        let status = MoveUse { move_type: Type::Normal, power: 0 };
        assert_eq!(damage(&attacker(), &defender(), &status, Roll::MAX), 0);
    }

    #[test]
    fn crit_rates_are_the_gen_three_ladder() {
        assert_eq!(crit_denominator(0), 16);
        assert_eq!(crit_denominator(1), 8);
        assert_eq!(crit_denominator(2), 4);
        assert_eq!(crit_denominator(3), 3);
        assert_eq!(crit_denominator(4), 2);
        assert_eq!(crit_denominator(9), 2, "the ladder tops out");
    }
}
