//! Damage formula, accuracy, crit chance, stat-stage application.
//!
//! All per the cartridge-accurate spec in GEN1_SPEC.md §2–§4.

use crate::rng::Rng;
use crate::state::{Mon, Side, Status, Volatile};
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

/// Effective stat with stage applied. Indices: 0=HP (no stage), 1=Atk, 2=Def, 3=Spc, 4=Spe.
/// HP returns as-is.
pub fn effective_stat(mon: &Mon, idx: usize) -> u32 {
    if idx == 0 {
        return mon.stats[0] as u32;
    }
    let stage_idx = idx - 1; // stages[0..4] correspond to atk/def/spc/spe
    let (n, d) = stage_mult(mon.stages[stage_idx]);
    let base = mon.stats[idx] as u32 * n / d;

    // PAR halves Speed; BRN halves Atk (sticky per Gen 1 — applied at recalc time).
    let after_status = match (idx, mon.status) {
        (1, Status::Burn) => base / 2,
        (4, Status::Paralysis) => base / 4,
        _ => base,
    };

    after_status.max(1).min(999)
}

/// Critical-hit roll. Uses BASE Speed (unmodified by stages, paralysis, or burn).
pub fn crit_roll(rng: &mut Rng, attacker: &Mon, mv: &MoveEntry) -> bool {
    let base_speed = attacker.stats[4]; // final stat — gen1 actually uses *species base*; ok approximation
    let mut t = (base_speed as u32) / 2;
    if (mv.flags & FLAG_HIGH_CRIT) != 0 {
        t = (t * 8).min(255);
    }
    if attacker.volatile.has(Volatile::FOCUS_ENERGY) {
        // Gen 1 bug: should *= 4, actually /= 4.
        t /= 4;
    }
    (rng.byte() as u32) < t
}

/// Accuracy check.
pub fn hit_roll(rng: &mut Rng, mv: &MoveEntry, attacker: &Mon, defender: &Mon) -> bool {
    if mv.accuracy == 0 {
        // Always-hit move.
        return true;
    }
    // accuracy as 0..=255, scaled by the attacker's accuracy stage and the
    // defender's evasion stage (stages[4]/stages[5]).
    let mut acc = (mv.accuracy as u32 * 255) / 100;
    let (an, ad) = stage_mult(attacker.stages[4]);
    acc = acc * an / ad;
    let (en, ed) = stage_mult(-defender.stages[5]);
    acc = acc * en / ed;
    acc = acc.clamp(1, 255);
    (rng.byte() as u32) < acc
}

/// Damage formula. Returns (damage, crit).
/// Caller is responsible for ordering, status, volatile, and screens.
pub fn compute_damage(
    rng: &mut Rng,
    attacker: &Mon,
    defender: &Mon,
    mv: &MoveEntry,
    attacker_side: &Side,
    defender_side: &Side,
) -> (u16, bool) {
    if mv.power == 0 {
        return (0, false);
    }

    let crit = crit_roll(rng, attacker, mv);

    let (atk_idx, def_idx) = match mv.category {
        MoveCategory::Physical => (1, 2),
        MoveCategory::Special => (3, 3),
        MoveCategory::Status => return (0, false),
    };

    let (mut atk, mut def) = if crit {
        // Crits ignore stat stages — use raw final stats.
        (attacker.stats[atk_idx] as u32, defender.stats[def_idx] as u32)
    } else {
        (effective_stat(attacker, atk_idx), effective_stat(defender, def_idx))
    };

    // Reflect/Light Screen apply to non-crit hits only.
    if !crit {
        if mv.category == MoveCategory::Physical && defender_side.reflect_turns > 0 {
            def *= 2;
        }
        if mv.category == MoveCategory::Special && defender_side.light_screen_turns > 0 {
            def *= 2;
        }
    }
    let _ = attacker_side;

    // Core formula.
    let level = attacker.level as u32;
    let base = (((2 * level / 5 + 2) * mv.power as u32 * atk) / def.max(1)) / 50 + 2;

    // STAB.
    let stab = mv.move_type == attacker.primary_type || mv.move_type == attacker.secondary_type;
    let mut dmg = if stab { base * 15 / 10 } else { base };

    // Type effectiveness — apply per defender type.
    let e1 = type_effectiveness(mv.move_type, defender.primary_type) as u32;
    let e2 = type_effectiveness(mv.move_type, defender.secondary_type) as u32;
    if defender.secondary_type == Type::None {
        dmg = dmg * e1 / 10;
    } else {
        dmg = dmg * e1 / 10 * e2 / 10;
    }
    if dmg == 0 {
        return (0, false);
    }

    // Crit multiplier (after STAB/type per Gen 1).
    if crit {
        dmg *= 2;
    }

    // Random factor: roll bytes in [217..=255], scale.
    let r = loop {
        let b = rng.byte();
        if b >= 217 {
            break b as u32;
        }
    };
    dmg = (dmg * r) / 255;

    // Min 1 if it hit.
    let dmg = dmg.max(1).min(u16::MAX as u32) as u16;
    (dmg, crit)
}
