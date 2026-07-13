//! Damage formula, accuracy, crit chance, stat-stage math.
//!
//! Ported from Pokémon Showdown's Gen 1 mod (`data/mods/gen1/scripts.ts`,
//! the community's cartridge-verified reference implementation), quirks and
//! all: crits double the level term and read unmodified stats, stats ≥ 256
//! divide both sides by 4 with an 8-bit rollover, 100%-accuracy moves miss
//! 1/256 of the time, Focus Energy QUARTERS the crit rate, and a damage roll
//! of 0 turns into a miss.

use crate::rng::Rng;
use crate::state::{Mon, Volatile};
use crate::tables::{type_effectiveness, MoveCategory, MoveEntry, FLAG_HIGH_CRIT};
use crate::types::Type;

/// Stat-stage multiplier as (numerator, denominator) per Gen 1 table.
pub fn stage_mult(stage: i8) -> (u32, u32) {
    match stage.clamp(-6, 6) {
        -6 => (25, 100),
        -5 => (28, 100),
        -4 => (33, 100),
        -3 => (40, 100),
        -2 => (50, 100),
        -1 => (66, 100),
        0 => (1, 1),
        1 => (150, 100),
        2 => (200, 100),
        3 => (250, 100),
        4 => (300, 100),
        5 => (350, 100),
        6 => (400, 100),
        _ => (1, 1),
    }
}

/// Recalculate one modified stat from the unmodified stat and current stage.
/// This is the Gen 1 "recalc" — it does NOT reapply paralysis/burn drops,
/// which is exactly the stat modification glitch (Agility cures the par drop).
pub fn recalc_modified(mon: &mut Mon, stat_idx: usize) {
    debug_assert!((1..=4).contains(&stat_idx));
    let (n, d) = stage_mult(mon.stages[stat_idx - 1]);
    mon.modified[stat_idx] = ((mon.stats[stat_idx] as u32 * n / d).clamp(1, 999)) as u16;
}

/// Apply a fractional modifier onto the CURRENT modified stat (min 1). Used
/// for the paralysis /4 and burn /2 drops, including their re-stacking.
pub fn modify_stat(mon: &mut Mon, stat_idx: usize, num: u32, den: u32) {
    let v = (mon.modified[stat_idx] as u32 * num / den).max(1);
    mon.modified[stat_idx] = v.min(999) as u16;
}

/// Apply the sticky status stat drop for the mon's current major status
/// (on infliction, on switch-in, and on the re-stack glitch).
pub fn apply_status_drop(mon: &mut Mon) {
    match mon.status {
        crate::state::Status::Paralysis => modify_stat(mon, 4, 1, 4),
        crate::state::Status::Burn => modify_stat(mon, 1, 1, 2),
        _ => {}
    }
}

/// Critical-hit roll, straight from the cartridge routine:
/// chance = floor(base_speed / 2); Focus Energy DIVIDES by 2 where it should
/// multiply (the famous bug), otherwise ×2 capped at 255; high-crit moves ×4
/// capped, normal moves ÷2. Roll a byte under the threshold.
pub fn crit_roll(rng: &mut Rng, attacker: &Mon, mv: &MoveEntry) -> bool {
    let mut c = (attacker.base_spe as u32) / 2;
    if attacker.volatile.has(Volatile::FOCUS_ENERGY) {
        c /= 2;
    } else {
        c = (c * 2).clamp(1, 255);
    }
    if (mv.flags & FLAG_HIGH_CRIT) != 0 {
        c = (c * 4).clamp(1, 255);
    } else {
        c /= 2;
    }
    if c == 0 {
        return false;
    }
    (rng.byte() as u32) < c
}

/// Accuracy check on the 0..=255 scale.
///
/// `base_acc_255` is the move accuracy already scaled to /256 (or a stored
/// effective accuracy for the Thrash/Rage compounding bug). Returns the roll
/// outcome plus the effective accuracy so lock volatiles can store it back.
/// A 255 effective accuracy still misses 1/256 of the time.
pub fn hit_roll(rng: &mut Rng, base_acc_255: u32, acc_stage: i8, eva_stage: i8) -> (bool, u8) {
    let mut acc = base_acc_255;
    let (an, ad) = stage_mult(acc_stage);
    acc = acc * an / ad;
    let (en, ed) = stage_mult(-eva_stage);
    acc = acc * en / ed;
    let acc = acc.clamp(1, 255);
    ((rng.byte() as u32) < acc, acc as u8)
}

/// Result of the damage formula.
#[derive(Clone, Copy, Debug, Default)]
pub struct DamageRoll {
    pub dmg: u16,
    pub crit: bool,
    /// Type chart zeroed the damage (target immune). Callers normally
    /// pre-check immunity before rolling accuracy; this covers the rest.
    pub immune: bool,
}

/// The Gen 1 damage formula, step for step.
///
/// A non-immune result of 0 means the move MISSES (Gen 1 "0 damage glitch":
/// possible when a 2-or-3 base result meets a 4× resist).
pub fn compute_damage(
    rng: &mut Rng,
    attacker: &Mon,
    defender: &Mon,
    mv: &MoveEntry,
    selfdestruct: bool,
) -> DamageRoll {
    if mv.power == 0 {
        return DamageRoll::default();
    }

    let crit = crit_roll(rng, attacker, mv);

    let (atk_idx, def_idx) = match mv.category {
        MoveCategory::Physical => (1usize, 2usize),
        MoveCategory::Special => (3, 3),
        MoveCategory::Status => return DamageRoll::default(),
    };

    let mut level = attacker.level as u32;
    let (mut atk, mut def);
    if crit {
        // Crits ignore stat stages, par/brn drops, AND screens — and double
        // the level term instead of the final damage.
        level *= 2;
        atk = attacker.stats[atk_idx] as u32;
        def = defender.stats[def_idx] as u32;
    } else {
        atk = attacker.modified[atk_idx] as u32;
        def = defender.modified[def_idx] as u32;
        // Screens double the (Sp)Def BEFORE the ≥256 rollover check, which is
        // why Reflect can make you take MORE damage in Gen 1.
        let screened = match mv.category {
            MoveCategory::Physical => defender.volatile.has(Volatile::REFLECT),
            _ => defender.volatile.has(Volatile::LIGHT_SCREEN),
        };
        if screened {
            def *= 2;
        }
    }

    // When either stat is ≥ 256, the cartridge divides both by 4 and keeps
    // only the low byte — the rollover behind the division-by-zero freeze.
    // We clamp the zero case to 1 instead of hanging, like Showdown does.
    if atk >= 256 || def >= 256 {
        atk = ((atk / 4) % 256).max(1);
        def = (def / 4) % 256;
        if def == 0 {
            def = 1;
        }
    }

    // Self-Destruct / Explosion halve defense at this point.
    if selfdestruct {
        def = (def / 2).max(1);
    }

    // Core formula, floor-by-floor like the game.
    let mut dmg = level * 2 / 5 + 2;
    dmg = dmg * mv.power as u32 * atk / def / 50;
    dmg = dmg.min(997) + 2;

    // STAB.
    if mv.move_type != Type::None
        && (mv.move_type == attacker.primary_type || mv.move_type == attacker.secondary_type)
    {
        dmg += dmg / 2;
    }

    // Type effectiveness, applied per defender type in order.
    let e1 = type_effectiveness(mv.move_type, defender.primary_type) as u32;
    dmg = dmg * e1 / 10;
    if defender.secondary_type != Type::None {
        let e2 = type_effectiveness(mv.move_type, defender.secondary_type) as u32;
        dmg = dmg * e2 / 10;
    }
    let immune = {
        let e2 = if defender.secondary_type == Type::None {
            10
        } else {
            type_effectiveness(mv.move_type, defender.secondary_type) as u32
        };
        e1 == 0 || e2 == 0
    };
    if dmg == 0 {
        return DamageRoll { dmg: 0, crit, immune };
    }

    // Random factor 217..=255 /255, only when damage > 1.
    if dmg > 1 {
        let r = 217 + rng.range(39);
        dmg = dmg * r / 255;
    }

    DamageRoll { dmg: dmg.min(u16::MAX as u32) as u16, crit, immune: false }
}
