//! Move dispatch: maps `MoveEffectKind` to side-effects, plus special-case
//! handlers for the famously weird Gen 1 moves (Counter, Bide, etc.).
//!
//! Status / volatile encoding aligns with `state.rs` and the param scheme
//! defined in `build.rs::classify_move`.

extern crate alloc;

use alloc::format;
use alloc::string::String;

use crate::combat::{compute_damage, hit_roll};
use crate::log::Event;
use crate::rng::Rng;
use crate::state::{Mon, MoveSlot, Side, Status, Volatile};
use crate::tables::{move_by_id, MoveCategory, MoveEffectKind, MoveEntry, MOVES};

/// Stat encoding in effect_param0 for boost-type effects.
fn stat_idx_from_param(p: u8) -> usize {
    // Atk=1, Def=2, SpAtk(Spc)=3, SpDef=4(unused gen1), Spe=5, Acc=6, Eva=7
    match p {
        1 => 0, // -> stages[0] = Atk
        2 => 1, // -> stages[1] = Def
        3 => 2, // -> stages[2] = Spc
        4 => 2, // SpDef collapses to Spc in Gen 1
        5 => 3, // -> stages[3] = Spe
        6 => 4, // -> stages[4] = Accuracy
        7 => 5, // -> stages[5] = Evasion
        _ => 0,
    }
}

/// Display name for a stat param (for the `boost|` board line).
fn stat_name_from_param(p: u8) -> &'static str {
    match p {
        1 => "Attack",
        2 => "Defense",
        3 | 4 => "Special",
        5 => "Speed",
        6 => "accuracy",
        7 => "evasiveness",
        _ => "stat",
    }
}

fn status_from_param(p: u8) -> Status {
    match p {
        1 => Status::Poison,
        2 => Status::Burn,
        3 => Status::Freeze,
        4 => Status::Paralysis,
        5 => Status::Sleep(2), // randomized by caller if desired
        7 => Status::BadPoison(1),
        _ => Status::None,
    }
}

/// Outcome of one move use.
#[derive(Clone, Copy, Debug, Default)]
pub struct MoveOutcome {
    pub hit: bool,
    pub crit: bool,
    pub damage_dealt: u16,
    pub fainted_target: bool,
    pub fainted_user: bool,
}

/// Use `attacker_side_idx` (0=p1, 1=p2) attacks with `move_slot_idx`.
///
/// All `Side` and `Mon` mutation happens inside this function.
pub fn execute_move(
    rng: &mut Rng,
    sides: &mut [Side; 2],
    attacker_side_idx: usize,
    move_slot_idx: usize,
    log: &mut Log,
) -> MoveOutcome {
    let defender_side_idx = 1 - attacker_side_idx;
    let mv_id = sides[attacker_side_idx].active().moves[move_slot_idx].move_id;
    if mv_id.is_empty() {
        return MoveOutcome::default();
    }
    let Some(mv) = move_by_id(mv_id) else {
        return MoveOutcome::default();
    };

    // Disable check.
    {
        let a = sides[attacker_side_idx].active();
        if a.volatile.has(Volatile::DISABLED)
            && a.volatile.disabled_slot as usize == move_slot_idx
        {
            let s = &sides[attacker_side_idx];
            log.push_board(format!("cant|mon:{},{},0|from:disabled", s.active().name, s.player_id));
            return MoveOutcome::default();
        }
    }

    // Deduct PP up-front (Gen 1 behavior).
    {
        let a = sides[attacker_side_idx].active_mut();
        if a.moves[move_slot_idx].pp > 0 {
            a.moves[move_slot_idx].pp -= 1;
        }
        a.last_move_used = mv.id;
    }
    sides[attacker_side_idx].last_move_used = mv.id;
    sides[attacker_side_idx].last_move_was_normal_or_fighting =
        matches!(mv.move_type, crate::types::Type::Normal | crate::types::Type::Fighting);

    {
        let s = &sides[attacker_side_idx];
        log.push_board(format!("move|mon:{},{},0|name:{}", s.active().name, s.player_id, mv.name));
    }

    // Accuracy check (status moves still respect accuracy).
    let hit = {
        let (a, d) = active_pair(sides, attacker_side_idx);
        hit_roll(rng, mv, a, d)
    };
    if !hit {
        {
            let s = &sides[attacker_side_idx];
            log.push_board(format!("miss|mon:{},{},0", s.active().name, s.player_id));
        }
        handle_miss_side_effects(rng, mv, sides, attacker_side_idx, log);
        return MoveOutcome::default();
    }
    let _ = defender_side_idx;

    apply_effect(rng, mv, sides, attacker_side_idx, log)
}

/// Execute a move without consuming PP and without re-announcing it (used by
/// Mirror Move / Metronome / TwoTurn release / Bide release / Wrap continuation).
fn execute_move_no_pp(
    rng: &mut Rng,
    mv: &MoveEntry,
    sides: &mut [Side; 2],
    attacker_side_idx: usize,
    log: &mut Log,
) -> MoveOutcome {
    sides[attacker_side_idx].active_mut().last_move_used = mv.id;
    sides[attacker_side_idx].last_move_used = mv.id;
    sides[attacker_side_idx].last_move_was_normal_or_fighting =
        matches!(mv.move_type, crate::types::Type::Normal | crate::types::Type::Fighting);
    {
        let s = &sides[attacker_side_idx];
        log.push_board(format!("move|mon:{},{},0|name:{}", s.active().name, s.player_id, mv.name));
    }

    let hit = {
        let (a, d) = active_pair(sides, attacker_side_idx);
        hit_roll(rng, mv, a, d)
    };
    if !hit {
        {
            let s = &sides[attacker_side_idx];
            log.push_board(format!("miss|mon:{},{},0", s.active().name, s.player_id));
        }
        handle_miss_side_effects(rng, mv, sides, attacker_side_idx, log);
        return MoveOutcome::default();
    }
    apply_effect(rng, mv, sides, attacker_side_idx, log)
}

fn handle_miss_side_effects(
    _rng: &mut Rng,
    mv: &MoveEntry,
    sides: &mut [Side; 2],
    attacker_side: usize,
    log: &mut Log,
) {
    if mv.effect_kind == MoveEffectKind::CrashOnMiss {
        let crash = 1u16; // Gen 1 quirk: 1 HP, not 1/8.
        let cur = sides[attacker_side].active().hp_cur;
        sides[attacker_side].active_mut().hp_cur = cur.saturating_sub(crash);
        let s = &sides[attacker_side];
        let m = s.active();
        log.push_board(format!("damage|mon:{},{},0|health:{}/{}", m.name, s.player_id, m.hp_cur, m.hp_max));
    }
}

/// Apply the move's primary effect. Returns outcome.
fn apply_effect(
    rng: &mut Rng,
    mv: &MoveEntry,
    sides: &mut [Side; 2],
    attacker_side: usize,
    log: &mut Log,
) -> MoveOutcome {
    let defender_side = 1 - attacker_side;
    use MoveEffectKind::*;
    let mut outcome = MoveOutcome { hit: true, ..Default::default() };

    match mv.effect_kind {
        Damage => {
            outcome = damage_step(rng, sides, attacker_side, mv, log, outcome);
        }
        DamageMaybeStatus => {
            outcome = damage_step(rng, sides, attacker_side, mv, log, outcome);
            if outcome.damage_dealt > 0 && roll_chance_byte(rng, mv.effect_param1) {
                if mv.effect_param0 == 6 {
                    // Confusion is a volatile, not a major status.
                    try_apply_confusion(rng, sides, defender_side, log);
                } else {
                    let new_status = status_from_param(mv.effect_param0);
                    try_apply_status(sides, defender_side, new_status, log);
                }
            }
        }
        DamageMaybeFlinch => {
            outcome = damage_step(rng, sides, attacker_side, mv, log, outcome);
            if outcome.damage_dealt > 0 && roll_chance_byte(rng, mv.effect_param0) {
                sides[defender_side].active_mut().volatile.set(Volatile::FLINCHED);
            }
        }
        DamageMaybeBoostTarget => {
            outcome = damage_step(rng, sides, attacker_side, mv, log, outcome);
            if outcome.damage_dealt > 0 {
                if roll_chance_byte(rng, 76) {
                    let delta = mv.effect_param1 as i8;
                    apply_stage_change(sides, defender_side, mv.effect_param0, delta, log);
                }
            }
        }
        DamageMaybeBoostSelf => {
            outcome = damage_step(rng, sides, attacker_side, mv, log, outcome);
            if outcome.damage_dealt > 0 && roll_chance_byte(rng, 76) {
                let delta = mv.effect_param1 as i8;
                apply_stage_change(sides, attacker_side, mv.effect_param0, delta, log);
            }
        }
        BoostSelf => {
            let delta = mv.effect_param1 as i8;
            apply_stage_change(sides, attacker_side, mv.effect_param0, delta, log);
        }
        BoostTarget => {
            // Mist on the target side blocks stat reductions.
            if (mv.effect_param1 as i8) < 0
                && sides[defender_side].active().volatile.has(Volatile::MIST)
            {
                let s = &sides[attacker_side];
                log.push_board(format!("fail|mon:{},{},0", s.active().name, s.player_id));
            } else {
                let delta = mv.effect_param1 as i8;
                apply_stage_change(sides, defender_side, mv.effect_param0, delta, log);
            }
        }
        StatusOnly => {
            if mv.effect_param0 == 6 {
                // Confuse Ray / Supersonic — confusion is a volatile.
                if sides[defender_side].active().volatile.has(Volatile::CONFUSED) {
                    let s = &sides[attacker_side];
                    log.push_board(format!("fail|mon:{},{},0", s.active().name, s.player_id));
                } else {
                    try_apply_confusion(rng, sides, defender_side, log);
                }
            } else {
                let mut s = status_from_param(mv.effect_param0);
                if matches!(s, Status::Sleep(_)) {
                    let turns = (rng.range(7) as u8) + 1;
                    s = Status::Sleep(turns);
                }
                try_apply_status(sides, defender_side, s, log);
            }
        }
        MultiHit2to5 => {
            let r = rng.range(8) as u8;
            let hits = match r {
                0 | 1 | 2 => 2,
                3 | 4 | 5 => 3,
                6 => 4,
                _ => 5,
            };
            for _ in 0..hits {
                outcome = damage_step(rng, sides, attacker_side, mv, log, outcome);
                if outcome.fainted_target {
                    break;
                }
            }
        }
        MultiHitFixed => {
            let hits = mv.effect_param0.max(1);
            for _ in 0..hits {
                outcome = damage_step(rng, sides, attacker_side, mv, log, outcome);
                if outcome.fainted_target {
                    break;
                }
            }
        }
        DrainHp => {
            outcome = damage_step(rng, sides, attacker_side, mv, log, outcome);
            if outcome.damage_dealt > 0 {
                let num = mv.effect_param0.max(1) as u32;
                let den = mv.effect_param1.max(1) as u32;
                let heal = (outcome.damage_dealt as u32 * num / den) as u16;
                heal_mon(sides, attacker_side, heal, log);
            }
        }
        Recoil1of4 => {
            outcome = damage_step(rng, sides, attacker_side, mv, log, outcome);
            if outcome.damage_dealt > 0 {
                let recoil = (outcome.damage_dealt / 4).max(1);
                damage_self(sides, attacker_side, recoil, log);
            }
        }
        CrashOnMiss => {
            outcome = damage_step(rng, sides, attacker_side, mv, log, outcome);
        }
        Ohko => {
            let a_spe = sides[attacker_side].active().stats[4];
            let d_spe = sides[defender_side].active().stats[4];
            if a_spe < d_spe {
                let s = &sides[attacker_side];
                log.push_board(format!("miss|mon:{},{},0", s.active().name, s.player_id));
                return MoveOutcome::default();
            }
            if rng.byte() < (30u32 * 255 / 100) as u8 {
                let dmg = sides[defender_side].active().hp_cur;
                deal_damage(sides, defender_side, dmg, log);
                outcome.damage_dealt = dmg;
                outcome.fainted_target = sides[defender_side].active().hp_cur == 0;
            } else {
                let s = &sides[attacker_side];
                log.push_board(format!("miss|mon:{},{},0", s.active().name, s.player_id));
            }
        }
        ForceSwitchTarget => {
            let s = &sides[attacker_side];
            log.push_board(format!("fail|mon:{},{},0", s.active().name, s.player_id));
        }
        LevelDamage => {
            let dmg = sides[attacker_side].active().level as u16;
            deal_damage(sides, defender_side, dmg, log);
            outcome.damage_dealt = dmg;
            outcome.fainted_target = sides[defender_side].active().hp_cur == 0;
        }
        FlatDamage => {
            let dmg = mv.effect_param0 as u16;
            deal_damage(sides, defender_side, dmg, log);
            outcome.damage_dealt = dmg;
            outcome.fainted_target = sides[defender_side].active().hp_cur == 0;
        }
        Psywave => {
            let lvl = sides[attacker_side].active().level as u32;
            let max = (lvl * 3 / 2).max(1) as u32;
            let dmg = rng.range(max) as u16 + 1;
            deal_damage(sides, defender_side, dmg, log);
            outcome.damage_dealt = dmg;
            outcome.fainted_target = sides[defender_side].active().hp_cur == 0;
        }
        HalfHp => {
            let dmg = (sides[defender_side].active().hp_cur / 2).max(1);
            deal_damage(sides, defender_side, dmg, log);
            outcome.damage_dealt = dmg;
        }
        HealHalf => {
            let max = sides[attacker_side].active().hp_max;
            heal_mon(sides, attacker_side, max / 2, log);
        }
        Rest => {
            let max = sides[attacker_side].active().hp_max;
            let a = sides[attacker_side].active_mut();
            a.hp_cur = max;
            a.status = Status::Sleep(2);
        }
        TwoTurn => {
            let a = sides[attacker_side].active_mut();
            if a.volatile.has(Volatile::CHARGING) {
                // Turn 2: deliver the damage, clear charging state.
                a.volatile.clear(Volatile::CHARGING);
                a.volatile.clear(Volatile::INVULNERABLE);
                a.volatile.multi_turn_move = "";
                a.volatile.multi_turn_turns = 0;
                outcome = damage_step(rng, sides, attacker_side, mv, log, outcome);
            } else {
                // Turn 1: announce charge, set lock.
                a.volatile.set(Volatile::CHARGING);
                a.volatile.multi_turn_move = mv.id;
                a.volatile.multi_turn_turns = 1;
                if mv.effect_param0 == 1 {
                    a.volatile.set(Volatile::INVULNERABLE);
                }
                let s = &sides[attacker_side];
                log.push_board(format!(
                    "start|mon:{},{},0|what:charging|move:{}",
                    s.active().name, s.player_id, mv.name
                ));
            }
        }
        Bide => {
            let a = sides[attacker_side].active_mut();
            if a.volatile.has(Volatile::BIDING) {
                if a.volatile.bide_turns > 0 {
                    a.volatile.bide_turns -= 1;
                    let s = &sides[attacker_side];
                    log.push_board(format!("start|mon:{},{},0|what:bide", s.active().name, s.player_id));
                } else {
                    // Unleash 2× stored damage as untyped damage.
                    let stored = a.volatile.bide_damage;
                    a.volatile.clear(Volatile::BIDING);
                    a.volatile.bide_damage = 0;
                    a.volatile.multi_turn_move = "";
                    let dmg = stored.saturating_mul(2).max(1);
                    deal_damage(sides, defender_side, dmg, log);
                    outcome.damage_dealt = dmg;
                    outcome.fainted_target = sides[defender_side].active().hp_cur == 0;
                    let s = &sides[attacker_side];
                    log.push_board(format!("end|mon:{},{},0|what:bide", s.active().name, s.player_id));
                }
            } else {
                a.volatile.set(Volatile::BIDING);
                a.volatile.bide_damage = 0;
                // 2 or 3 turns to store (Gen 1 rolls one of these).
                a.volatile.bide_turns = 1 + (rng.byte() & 1);
                a.volatile.multi_turn_move = mv.id;
                let s = &sides[attacker_side];
                log.push_board(format!("start|mon:{},{},0|what:bide", s.active().name, s.player_id));
            }
        }
        HyperBeam => {
            outcome = damage_step(rng, sides, attacker_side, mv, log, outcome);
            if outcome.damage_dealt > 0 && !outcome.fainted_target {
                sides[attacker_side].active_mut().volatile.set(Volatile::MUST_RECHARGE);
            }
        }
        Counter => {
            let cs = sides[attacker_side].active().counter_source_dmg;
            if cs > 0 {
                let dmg = cs.saturating_mul(2);
                deal_damage(sides, defender_side, dmg, log);
                outcome.damage_dealt = dmg;
                outcome.fainted_target = sides[defender_side].active().hp_cur == 0;
            } else {
                let s = &sides[attacker_side];
                log.push_board(format!("fail|mon:{},{},0", s.active().name, s.player_id));
            }
        }
        MirrorMove => {
            let last = sides[defender_side].last_move_used;
            if last.is_empty() || last == "mirrormove" {
                let s = &sides[attacker_side];
                log.push_board(format!("fail|mon:{},{},0", s.active().name, s.player_id));
            } else if let Some(mv2) = move_by_id(last) {
                outcome = execute_move_no_pp(rng, mv2, sides, attacker_side, log);
            }
        }
        Mimic => {
            let target_last = sides[defender_side].last_move_used;
            let target_moves: alloc::vec::Vec<&'static str> = sides[defender_side]
                .active()
                .moves
                .iter()
                .filter(|s| !s.move_id.is_empty())
                .map(|s| s.move_id)
                .collect();
            // Prefer last_move_used; else random from target's moves.
            let learn: Option<&'static str> = if !target_last.is_empty() {
                Some(target_last)
            } else if !target_moves.is_empty() {
                Some(target_moves[(rng.range(target_moves.len() as u32)) as usize])
            } else {
                None
            };
            let Some(new_move) = learn else {
                let s = &sides[attacker_side];
                log.push_board(format!("fail|mon:{},{},0", s.active().name, s.player_id));
                return outcome;
            };
            // Find the user's Mimic slot — fallback to first slot.
            let a = sides[attacker_side].active_mut();
            let mimic_slot = a.find_move_slot("mimic").unwrap_or(0) as usize;
            let new_max = move_by_id(new_move).map(|m| m.pp).unwrap_or(5).min(5);
            a.moves[mimic_slot] = MoveSlot {
                move_id: new_move,
                pp: new_max,
                max_pp: new_max,
            };
            let learned = move_by_id(new_move).map(|m| m.name).unwrap_or(new_move);
            let s = &sides[attacker_side];
            log.push_board(format!(
                "start|mon:{},{},0|what:mimic|move:{}",
                s.active().name, s.player_id, learned
            ));
        }
        Transform => {
            let target = sides[defender_side].active().clone();
            let a = sides[attacker_side].active_mut();
            a.species_id = target.species_id;
            a.primary_type = target.primary_type;
            a.secondary_type = target.secondary_type;
            // Stats: copy non-HP only.
            a.stats[1] = target.stats[1];
            a.stats[2] = target.stats[2];
            a.stats[3] = target.stats[3];
            a.stats[4] = target.stats[4];
            a.stages = target.stages;
            // Copy moves with PP=5.
            for (i, m) in target.moves.iter().enumerate() {
                if m.move_id.is_empty() {
                    a.moves[i] = MoveSlot::default();
                } else {
                    a.moves[i] = MoveSlot {
                        move_id: m.move_id,
                        pp: 5,
                        max_pp: 5,
                    };
                }
            }
            a.volatile.set(Volatile::TRANSFORMED);
            let s = &sides[attacker_side];
            log.push_board(format!("start|mon:{},{},0|what:transform", s.active().name, s.player_id));
        }
        Substitute => {
            let max = sides[attacker_side].active().hp_max;
            let cost = (max / 4).max(1);
            if sides[attacker_side].active().hp_cur > cost {
                sides[attacker_side].active_mut().hp_cur -= cost;
                sides[attacker_side].active_mut().volatile.set(Volatile::SUBSTITUTED);
                sides[attacker_side].active_mut().volatile.substitute_hp =
                    (cost + 1).min(255) as u8;
                let s = &sides[attacker_side];
                log.push_board(format!("start|mon:{},{},0|what:substitute", s.active().name, s.player_id));
            } else {
                let s = &sides[attacker_side];
                log.push_board(format!("fail|mon:{},{},0", s.active().name, s.player_id));
            }
        }
        Disable => {
            // Pick a target move slot with PP > 0.
            let candidates: alloc::vec::Vec<u8> = sides[defender_side]
                .active()
                .moves
                .iter()
                .enumerate()
                .filter(|(_, s)| !s.move_id.is_empty() && s.pp > 0)
                .map(|(i, _)| i as u8)
                .collect();
            if candidates.is_empty() {
                let s = &sides[attacker_side];
                log.push_board(format!("fail|mon:{},{},0", s.active().name, s.player_id));
            } else {
                let pick = candidates[(rng.range(candidates.len() as u32)) as usize];
                let turns = ((rng.byte() & 0b111) as u8).saturating_add(1); // 1..=8
                let d = sides[defender_side].active_mut();
                d.volatile.set(Volatile::DISABLED);
                d.volatile.disabled_slot = pick;
                d.volatile.disabled_turns = turns;
                let disabled = move_by_id(d.moves[pick as usize].move_id).map(|m| m.name).unwrap_or("move");
                let s = &sides[defender_side];
                log.push_board(format!(
                    "start|mon:{},{},0|what:disable|move:{}",
                    s.active().name, s.player_id, disabled
                ));
            }
        }
        Wrap => {
            // Hit once, then trap target for 1..=4 additional turns.
            outcome = damage_step(rng, sides, attacker_side, mv, log, outcome);
            if outcome.damage_dealt > 0 && !outcome.fainted_target {
                let extra = (rng.range(4) as u8) + 1; // 1..=4
                // Attacker locks into the move; defender is trapped.
                {
                    let a = sides[attacker_side].active_mut();
                    a.volatile.multi_turn_move = mv.id;
                    a.volatile.multi_turn_turns = extra;
                }
                {
                    let d = sides[defender_side].active_mut();
                    d.volatile.set(Volatile::TRAPPED);
                    d.volatile.trapping_turns = extra;
                }
                let s = &sides[defender_side];
                log.push_board(format!("start|mon:{},{},0|what:wrap", s.active().name, s.player_id));
            }
        }
        LeechSeed => {
            let dt1 = sides[defender_side].active().primary_type;
            let dt2 = sides[defender_side].active().secondary_type;
            if matches!(dt1, crate::types::Type::Grass)
                || matches!(dt2, crate::types::Type::Grass)
            {
                let s = &sides[attacker_side];
                log.push_board(format!("fail|mon:{},{},0", s.active().name, s.player_id));
            } else if sides[defender_side].active().volatile.has(Volatile::LEECH_SEEDED) {
                let s = &sides[attacker_side];
                log.push_board(format!("fail|mon:{},{},0", s.active().name, s.player_id));
            } else {
                sides[defender_side].active_mut().volatile.set(Volatile::LEECH_SEEDED);
                let s = &sides[defender_side];
                log.push_board(format!("start|mon:{},{},0|what:seeded", s.active().name, s.player_id));
            }
        }
        LightScreen => {
            sides[attacker_side].light_screen_turns = 5;
            sides[attacker_side].active_mut().volatile.set(Volatile::LIGHT_SCREEN);
            let s = &sides[attacker_side];
            log.push_board(format!("start|mon:{},{},0|what:lightscreen", s.active().name, s.player_id));
        }
        Reflect => {
            sides[attacker_side].reflect_turns = 5;
            sides[attacker_side].active_mut().volatile.set(Volatile::REFLECT);
            let s = &sides[attacker_side];
            log.push_board(format!("start|mon:{},{},0|what:reflect", s.active().name, s.player_id));
        }
        Mist => {
            sides[attacker_side].active_mut().volatile.set(Volatile::MIST);
            let s = &sides[attacker_side];
            log.push_board(format!("start|mon:{},{},0|what:mist", s.active().name, s.player_id));
        }
        FocusEnergy => {
            sides[attacker_side].active_mut().volatile.set(Volatile::FOCUS_ENERGY);
            let s = &sides[attacker_side];
            log.push_board(format!("start|mon:{},{},0|what:focusenergy", s.active().name, s.player_id));
        }
        Conversion => {
            let t1 = sides[defender_side].active().primary_type;
            let t2 = sides[defender_side].active().secondary_type;
            let a = sides[attacker_side].active_mut();
            a.primary_type = t1;
            a.secondary_type = t2;
            let s = &sides[attacker_side];
            log.push_board(format!("start|mon:{},{},0|what:conversion", s.active().name, s.player_id));
        }
        Haze => {
            for s in sides.iter_mut() {
                for st in &mut s.active_mut().stages {
                    *st = 0;
                }
                // Clear non-major status on opponent in Gen 1.
                let mv = s.active_mut();
                mv.volatile.flags = 0;
            }
            sides[defender_side].active_mut().status = Status::None;
            let s = &sides[attacker_side];
            log.push_board(format!("start|mon:{},{},0|what:haze", s.active().name, s.player_id));
        }
        Metronome => {
            let mut idx = rng.range(MOVES.len() as u32) as usize;
            let mut safety = 32;
            while safety > 0 {
                let cand = &MOVES[idx];
                if cand.id != "metronome" && cand.id != "struggle" {
                    break;
                }
                idx = rng.range(MOVES.len() as u32) as usize;
                safety -= 1;
            }
            let cand: &MoveEntry = &MOVES[idx];
            outcome = execute_move_no_pp(rng, cand, sides, attacker_side, log);
        }
        PayDay => {
            outcome = damage_step(rng, sides, attacker_side, mv, log, outcome);
        }
        SelfDestruct => {
            outcome = damage_step(rng, sides, attacker_side, mv, log, outcome);
            sides[attacker_side].active_mut().hp_cur = 0;
            outcome.fainted_user = true;
            {
                let s = &sides[attacker_side];
                log.push_board(format!("faint|mon:{},{},0", s.active().name, s.player_id));
            }
        }
        NoOp => {
            let s = &sides[attacker_side];
            log.push_board(format!("fail|mon:{},{},0", s.active().name, s.player_id));
        }
    }
    outcome
}

fn roll_chance_byte(rng: &mut Rng, threshold: u8) -> bool {
    rng.byte() < threshold
}

fn active_pair<'s>(sides: &'s [Side; 2], attacker_idx: usize) -> (&'s Mon, &'s Mon) {
    let a = sides[attacker_idx].active();
    let d = sides[1 - attacker_idx].active();
    (a, d)
}

fn damage_step(
    rng: &mut Rng,
    sides: &mut [Side; 2],
    attacker_side: usize,
    mv: &MoveEntry,
    log: &mut Log,
    mut outcome: MoveOutcome,
) -> MoveOutcome {
    let defender_side = 1 - attacker_side;
    // Invulnerable target (Fly/Dig)?
    if sides[defender_side].active().volatile.has(Volatile::INVULNERABLE) {
        let s = &sides[attacker_side];
        log.push_board(format!("miss|mon:{},{},0", s.active().name, s.player_id));
        return outcome;
    }
    let (dmg, crit) = {
        let (atk_side_ref, def_side_ref) = if attacker_side == 0 {
            let (l, r) = sides.split_at(1);
            (&l[0], &r[0])
        } else {
            let (l, r) = sides.split_at(1);
            (&r[0], &l[0])
        };
        compute_damage(
            rng,
            atk_side_ref.active(),
            def_side_ref.active(),
            mv,
            atk_side_ref,
            def_side_ref,
        )
    };
    if dmg == 0 {
        let s = &sides[defender_side];
        log.push_board(format!("immune|mon:{},{},0", s.active().name, s.player_id));
        return outcome;
    }
    if crit {
        let s = &sides[defender_side];
        log.push_board(format!("crit|mon:{},{},0", s.active().name, s.player_id));
    }
    // Record source damage for Counter, and Bide-stored damage on defender.
    if matches!(mv.move_type, crate::types::Type::Normal | crate::types::Type::Fighting) {
        sides[defender_side].active_mut().counter_source_dmg = dmg;
    }
    deal_damage(sides, defender_side, dmg, log);
    if sides[defender_side].active().volatile.has(Volatile::BIDING) {
        let actual = dmg;
        sides[defender_side]
            .active_mut()
            .volatile
            .bide_damage = sides[defender_side]
            .active()
            .volatile
            .bide_damage
            .saturating_add(actual);
    }
    outcome.damage_dealt = outcome.damage_dealt.saturating_add(dmg);
    outcome.crit = outcome.crit || crit;
    outcome.fainted_target = sides[defender_side].active().hp_cur == 0;
    outcome
}

fn deal_damage(sides: &mut [Side; 2], target_side: usize, dmg: u16, log: &mut Log) {
    // Substitute absorbs damage.
    if sides[target_side].active().volatile.has(Volatile::SUBSTITUTED) {
        let sub_hp = sides[target_side].active().volatile.substitute_hp as u16;
        if dmg >= sub_hp {
            sides[target_side].active_mut().volatile.clear(Volatile::SUBSTITUTED);
            sides[target_side].active_mut().volatile.substitute_hp = 0;
            let s = &sides[target_side];
            log.push_board(format!("end|mon:{},{},0|what:substitute", s.active().name, s.player_id));
        } else {
            sides[target_side].active_mut().volatile.substitute_hp = (sub_hp - dmg) as u8;
        }
        return;
    }

    let actual = dmg.min(sides[target_side].active().hp_cur);
    sides[target_side].active_mut().hp_cur -= actual;
    let s = &sides[target_side];
    let m = s.active();
    log.push_board(format!("damage|mon:{},{},0|health:{}/{}", m.name, s.player_id, m.hp_cur, m.hp_max));
    if m.hp_cur == 0 {
        log.push_board(format!("faint|mon:{},{},0", m.name, s.player_id));
    }
}

fn damage_self(sides: &mut [Side; 2], side: usize, dmg: u16, log: &mut Log) {
    deal_damage(sides, side, dmg, log);
}

fn heal_mon(sides: &mut [Side; 2], side: usize, amt: u16, log: &mut Log) {
    let (new_hp, healed) = {
        let m = sides[side].active();
        let new = (m.hp_cur as u32 + amt as u32).min(m.hp_max as u32) as u16;
        (new, new - m.hp_cur)
    };
    sides[side].active_mut().hp_cur = new_hp;
    if healed > 0 {
        let s = &sides[side];
        let m = s.active();
        log.push_board(format!("heal|mon:{},{},0|health:{}/{}", m.name, s.player_id, m.hp_cur, m.hp_max));
    }
}

fn try_apply_status(sides: &mut [Side; 2], side: usize, status: Status, log: &mut Log) {
    if !matches!(sides[side].active().status, Status::None) {
        return;
    }
    if !matches!(status, Status::None) {
        sides[side].active_mut().status = status;
        let status_str = match status {
            Status::Poison => "psn",
            Status::Burn => "brn",
            Status::Freeze => "frz",
            Status::Paralysis => "par",
            Status::Sleep(_) => "slp",
            Status::BadPoison(_) => "tox",
            Status::None => "",
        };
        let s = &sides[side];
        let m = s.active();
        log.push_board(format!("status|mon:{},{},0|status:{}", m.name, s.player_id, status_str));
    }
}

/// Inflict the confusion volatile (Gen 1: 1-4 attacking turns). Silently does
/// nothing if the target is already confused.
fn try_apply_confusion(rng: &mut Rng, sides: &mut [Side; 2], side: usize, log: &mut Log) {
    let d = sides[side].active_mut();
    if d.volatile.has(Volatile::CONFUSED) || d.hp_cur == 0 {
        return;
    }
    d.volatile.set(Volatile::CONFUSED);
    d.volatile.confused_turns = (rng.range(4) as u8) + 1;
    let s = &sides[side];
    log.push_board(format!("start|mon:{},{},0|what:confusion", s.active().name, s.player_id));
}

fn apply_stage_change(
    sides: &mut [Side; 2],
    side: usize,
    stat_param: u8,
    delta: i8,
    log: &mut Log,
) {
    let idx = stat_idx_from_param(stat_param);
    let m = sides[side].active_mut();
    let new = (m.stages[idx] as i32 + delta as i32).clamp(-6, 6) as i8;
    if new != m.stages[idx] {
        m.stages[idx] = new;
        let s = &sides[side];
        log.push_board(format!(
            "boost|mon:{},{},0|stat:{}|delta:{}",
            s.active().name, s.player_id, stat_name_from_param(stat_param), delta
        ));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// End-of-turn damage (status/leech-seed)
// ─────────────────────────────────────────────────────────────────────────────

pub fn end_of_turn(sides: &mut [Side; 2], log: &mut Log) {
    for i in 0..2 {
        if sides[i].active().fainted() {
            continue;
        }
        let max = sides[i].active().hp_max;
        match sides[i].active().status {
            Status::Poison => {
                let d = (max / 16).max(1);
                let new_hp = sides[i].active().hp_cur.saturating_sub(d);
                sides[i].active_mut().hp_cur = new_hp;
                let s = &sides[i];
                log.push_board(format!("damage|mon:{},{},0|health:{}/{}", s.active().name, s.player_id, new_hp, max));
            }
            Status::Burn => {
                let d = (max / 16).max(1);
                let new_hp = sides[i].active().hp_cur.saturating_sub(d);
                sides[i].active_mut().hp_cur = new_hp;
                let s = &sides[i];
                log.push_board(format!("damage|mon:{},{},0|health:{}/{}", s.active().name, s.player_id, new_hp, max));
            }
            Status::BadPoison(c) => {
                let d = ((max as u32 * c as u32) / 16).max(1) as u16;
                let new_hp = sides[i].active().hp_cur.saturating_sub(d);
                sides[i].active_mut().hp_cur = new_hp;
                sides[i].active_mut().status = Status::BadPoison(c.saturating_add(1).min(15));
                let s = &sides[i];
                log.push_board(format!("damage|mon:{},{},0|health:{}/{}", s.active().name, s.player_id, new_hp, max));
            }
            _ => {}
        }
        if sides[i].active().volatile.has(Volatile::LEECH_SEEDED) && sides[i].active().hp_cur > 0 {
            let d = (max / 16).max(1);
            let new_hp = sides[i].active().hp_cur.saturating_sub(d);
            sides[i].active_mut().hp_cur = new_hp;
            {
                let s = &sides[i];
                log.push_board(format!("damage|mon:{},{},0|health:{}/{}", s.active().name, s.player_id, new_hp, max));
            }
            let healer = 1 - i;
            heal_mon(sides, healer, d, log);
        }
        if sides[i].active().hp_cur == 0 {
            let s = &sides[i];
            log.push_board(format!("faint|mon:{},{},0", s.active().name, s.player_id));
        }
    }
    // Decrement screens.
    for s in sides.iter_mut() {
        if s.reflect_turns > 0 {
            s.reflect_turns -= 1;
            if s.reflect_turns == 0 {
                s.active_mut().volatile.clear(Volatile::REFLECT);
            }
        }
        if s.light_screen_turns > 0 {
            s.light_screen_turns -= 1;
            if s.light_screen_turns == 0 {
                s.active_mut().volatile.clear(Volatile::LIGHT_SCREEN);
            }
        }
    }
    // Decrement Disable counters on both sides.
    for i in 0..2 {
        let m = sides[i].active_mut();
        if m.volatile.has(Volatile::DISABLED) {
            if m.volatile.disabled_turns > 0 {
                m.volatile.disabled_turns -= 1;
                if m.volatile.disabled_turns == 0 {
                    m.volatile.clear(Volatile::DISABLED);
                    m.volatile.disabled_slot = 0;
                    let s = &sides[i];
                    log.push_board(format!("end|mon:{},{},0|what:disable", s.active().name, s.player_id));
                }
            }
        }
    }
    // Wrap-trap countdown happens at turn use (in the trapping user's choice).
}

// ─────────────────────────────────────────────────────────────────────────────
// Pre-move status checks: returns true if the mon can act.
// ─────────────────────────────────────────────────────────────────────────────

pub fn pre_move_check(
    rng: &mut Rng,
    sides: &mut [Side; 2],
    side: usize,
    log: &mut Log,
) -> bool {
    if sides[side].active().volatile.has(Volatile::FLINCHED) {
        sides[side].active_mut().volatile.clear(Volatile::FLINCHED);
        return false;
    }
    if sides[side].active().volatile.has(Volatile::MUST_RECHARGE) {
        sides[side].active_mut().volatile.clear(Volatile::MUST_RECHARGE);
        let s = &sides[side];
        log.push_board(format!("cant|mon:{},{},0|from:recharge", s.active().name, s.player_id));
        return false;
    }
    if sides[side].active().volatile.has(Volatile::TRAPPED) {
        // Can't move while trapped (Wrap/Bind/Fire Spin/Clamp from opponent).
        {
            let m = sides[side].active_mut();
            if m.volatile.trapping_turns > 0 {
                m.volatile.trapping_turns -= 1;
                if m.volatile.trapping_turns == 0 {
                    m.volatile.clear(Volatile::TRAPPED);
                }
            }
        }
        let s = &sides[side];
        log.push_board(format!("cant|mon:{},{},0|from:trapped", s.active().name, s.player_id));
        return false;
    }
    let current_status = sides[side].active().status;
    match current_status {
        Status::Freeze => {
            let s = &sides[side];
            log.push_board(format!("cant|mon:{},{},0|from:frz", s.active().name, s.player_id));
            return false;
        }
        Status::Sleep(t) => {
            if t == 0 {
                sides[side].active_mut().status = Status::None;
                let s = &sides[side];
                log.push_board(format!("curestatus|mon:{},{},0|status:slp", s.active().name, s.player_id));
                return false; // Gen 1: lose the wake turn.
            } else {
                sides[side].active_mut().status = Status::Sleep(t - 1);
                return false;
            }
        }
        Status::Paralysis => {
            if rng.byte() < (255u32 / 4) as u8 {
                let s = &sides[side];
                log.push_board(format!("cant|mon:{},{},0|from:par", s.active().name, s.player_id));
                return false;
            }
        }
        _ => {}
    }
    if sides[side].active().volatile.has(Volatile::CONFUSED) {
        if sides[side].active().volatile.confused_turns == 0 {
            sides[side].active_mut().volatile.clear(Volatile::CONFUSED);
        } else {
            sides[side].active_mut().volatile.confused_turns -= 1;
            if rng.byte() < 128 {
                {
                    let s = &sides[side];
                    log.push_board(format!("cant|mon:{},{},0|from:confusion", s.active().name, s.player_id));
                }
                let (lvl, atk, def, hp_cur) = {
                    let m = sides[side].active();
                    (m.level as u32, m.stats[1] as u32, m.stats[2] as u32, m.hp_cur)
                };
                let dmg = ((((2 * lvl / 5 + 2) * 40 * atk) / def.max(1)) / 50 + 2) as u16;
                let actual = dmg.min(hp_cur);
                sides[side].active_mut().hp_cur -= actual;
                let s = &sides[side];
                let m = s.active();
                log.push_board(format!("damage|mon:{},{},0|health:{}/{}", m.name, s.player_id, m.hp_cur, m.hp_max));
                return false;
            }
        }
    }
    true
}

/// Returns Some(move_slot) if the mon must use a specific move this turn
/// (TwoTurn charging→release, Bide unleash, Wrap continuation).
/// Used by Battle::do_battle_turn to override the player's choice.
pub fn locked_move_slot(side: &Side) -> Option<u8> {
    let m = side.active();
    if !m.volatile.multi_turn_move.is_empty() {
        return m.find_move_slot(m.volatile.multi_turn_move);
    }
    None
}

// ─────────────────────────────────────────────────────────────────────────────
// Log
// ─────────────────────────────────────────────────────────────────────────────

pub struct Log {
    /// Pending event-as-string queue. Drained per turn.
    pub pending: alloc::vec::Vec<String>,
}

impl Log {
    pub fn new() -> Self {
        Self { pending: alloc::vec::Vec::new() }
    }
    pub fn push(&mut self, ev: Event) {
        self.pending.push(format!("{}", ev));
    }
    pub fn push_board(&mut self, s: String) {
        self.pending.push(s);
    }
}
