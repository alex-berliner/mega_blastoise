//! Move dispatch: maps `MoveEffectKind` to side-effects, plus special-case
//! handlers for the famously weird Gen 1 moves (Counter, Bide, etc.).
//!
//! Status / volatile encoding aligns with `state.rs` and the param scheme
//! defined in `build.rs::classify_move`.

extern crate alloc;

use alloc::format;
use alloc::string::String;

use crate::combat::{compute_damage, hit_roll, stage_mult};
use crate::log::Event;
use crate::rng::Rng;
use crate::state::{Mon, MoveSlot, Side, Status, Volatile};
use crate::tables::{move_by_id, MoveCategory, MoveEffectKind, MoveEntry, MOVES};

/// Stat encoding in effect_param0 for boost-type effects.
fn stat_idx_from_param(p: u8) -> usize {
    // Atk=1, Def=2, SpAtk(Spc)=3, SpDef=4(unused gen1), Spe=5
    match p {
        1 => 0, // -> stages[0] = Atk
        2 => 1, // -> stages[1] = Def
        3 => 2, // -> stages[2] = Spc
        4 => 2, // SpDef collapses to Spc in Gen 1
        5 => 3, // -> stages[3] = Spe
        _ => 0,
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

/// Use `attacker_idx` (0=p1, 1=p2) attacks the opposite side with `move_slot`.
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

    log.push(Event::MoveUsed {
        side: attacker_side_idx as u8,
        move_id: mv.id,
    });

    // Accuracy check (status moves still respect accuracy).
    let hit = {
        let (a, d) = active_pair(sides, attacker_side_idx);
        hit_roll(rng, mv, a, d)
    };
    if !hit {
        log.push(Event::Miss { side: attacker_side_idx as u8 });
        handle_miss_side_effects(rng, mv, sides, attacker_side_idx, log);
        return MoveOutcome::default();
    }

    // Apply effect based on kind.
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
        let a = sides[attacker_side].active_mut();
        let crash = 1u16; // Gen 1 quirk: 1 HP, not 1/8.
        a.hp_cur = a.hp_cur.saturating_sub(crash);
        log.push(Event::Damage {
            side: attacker_side as u8,
            dealt: crash,
        });
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
                let new_status = status_from_param(mv.effect_param0);
                try_apply_status(sides, defender_side, new_status, log);
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
                // param0 = stat_id, param1 = delta as u8 (i8 reinterpret), encoded ~30% chance.
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
            let delta = mv.effect_param1 as i8;
            apply_stage_change(sides, defender_side, mv.effect_param0, delta, log);
        }
        StatusOnly => {
            let mut s = status_from_param(mv.effect_param0);
            // Sleep: random 1..=7
            if matches!(s, Status::Sleep(_)) {
                let turns = (rng.range(7) as u8) + 1;
                s = Status::Sleep(turns);
            }
            try_apply_status(sides, defender_side, s, log);
        }
        MultiHit2to5 => {
            // 3/8 for 2, 3/8 for 3, 1/8 for 4, 1/8 for 5.
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
                log.push(Event::Miss { side: attacker_side as u8 });
                return MoveOutcome::default();
            }
            // 30% chance.
            if rng.byte() < (30u32 * 255 / 100) as u8 {
                let dmg = sides[defender_side].active().hp_cur;
                deal_damage(sides, defender_side, dmg, log);
                outcome.damage_dealt = dmg;
                outcome.fainted_target = sides[defender_side].active().hp_cur == 0;
            } else {
                log.push(Event::Miss { side: attacker_side as u8 });
            }
        }
        ForceSwitchTarget => {
            // Gen 1: always fails in trainer battle.
            log.push(Event::Failed);
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
            // Skipping multi-turn implementation for first cut: deal damage in one turn.
            outcome = damage_step(rng, sides, attacker_side, mv, log, outcome);
        }
        Bide => {
            // Skipping: behave as plain damage for first cut.
            outcome = damage_step(rng, sides, attacker_side, mv, log, outcome);
        }
        HyperBeam => {
            outcome = damage_step(rng, sides, attacker_side, mv, log, outcome);
            if outcome.damage_dealt > 0 && !outcome.fainted_target {
                sides[attacker_side].active_mut().volatile.set(Volatile::MUST_RECHARGE);
            }
        }
        Counter => {
            // 2× the last Normal/Fighting damage taken by attacker this turn.
            let cs = sides[attacker_side].active().counter_source_dmg;
            if cs > 0 {
                let dmg = cs.saturating_mul(2);
                deal_damage(sides, defender_side, dmg, log);
                outcome.damage_dealt = dmg;
                outcome.fainted_target = sides[defender_side].active().hp_cur == 0;
            } else {
                log.push(Event::Failed);
            }
        }
        MirrorMove => {
            let last = sides[defender_side].last_move_used;
            if last.is_empty() || last == "mirrormove" {
                log.push(Event::Failed);
            } else if let Some(mv2) = move_by_id(last) {
                // Recursively dispatch; no PP deduction (skip the path above).
                let mut faux = MoveOutcome { hit: true, ..Default::default() };
                let _ = mv2;
                faux = apply_effect(rng, mv2, sides, attacker_side, log);
                outcome = faux;
            }
        }
        Mimic | Transform | Substitute | Disable | Wrap | LeechSeed | LightScreen
        | Reflect | Mist | FocusEnergy | Conversion | Haze | Metronome | PayDay
        | SelfDestruct | NoOp => {
            // Implement the meaningful subset; rest are stubs that log.
            match mv.effect_kind {
                LightScreen => {
                    sides[attacker_side].light_screen_turns = 5;
                    sides[attacker_side].active_mut().volatile.set(Volatile::LIGHT_SCREEN);
                }
                Reflect => {
                    sides[attacker_side].reflect_turns = 5;
                    sides[attacker_side].active_mut().volatile.set(Volatile::REFLECT);
                }
                Mist => {
                    sides[attacker_side].active_mut().volatile.set(Volatile::MIST);
                }
                FocusEnergy => {
                    sides[attacker_side].active_mut().volatile.set(Volatile::FOCUS_ENERGY);
                }
                LeechSeed => {
                    if !matches!(
                        sides[defender_side].active().primary_type,
                        crate::types::Type::Grass
                    ) && !matches!(
                        sides[defender_side].active().secondary_type,
                        crate::types::Type::Grass
                    ) {
                        sides[defender_side].active_mut().volatile.set(Volatile::LEECH_SEEDED);
                    }
                }
                Substitute => {
                    let max = sides[attacker_side].active().hp_max;
                    let cost = (max / 4).max(1);
                    if sides[attacker_side].active().hp_cur > cost {
                        sides[attacker_side].active_mut().hp_cur -= cost;
                        sides[attacker_side].active_mut().volatile.set(Volatile::SUBSTITUTED);
                        sides[attacker_side].active_mut().volatile.substitute_hp =
                            (cost + 1).min(255) as u8;
                    }
                }
                Haze => {
                    for s in sides.iter_mut() {
                        for st in &mut s.active_mut().stages {
                            *st = 0;
                        }
                        s.active_mut().volatile.flags = 0;
                    }
                }
                SelfDestruct => {
                    // Halves defender Def in Gen 1; we just deal damage with a 2× scale.
                    outcome = damage_step(rng, sides, attacker_side, mv, log, outcome);
                    sides[attacker_side].active_mut().hp_cur = 0;
                    outcome.fainted_user = true;
                }
                Metronome => {
                    // Pick a random move from the table, recurse.
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
                    outcome = apply_effect(rng, cand, sides, attacker_side, log);
                }
                Mimic | Transform | Disable | Wrap | Conversion | PayDay | NoOp => {
                    log.push(Event::NoEffect);
                }
                _ => {}
            }
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
        log.push(Event::Immune);
        return outcome;
    }
    if crit {
        log.push(Event::Crit { side: attacker_side as u8 });
    }
    // Record source damage for Counter.
    if matches!(mv.move_type, crate::types::Type::Normal | crate::types::Type::Fighting) {
        sides[defender_side].active_mut().counter_source_dmg = dmg;
    }
    deal_damage(sides, defender_side, dmg, log);
    outcome.damage_dealt = outcome.damage_dealt.saturating_add(dmg);
    outcome.crit = outcome.crit || crit;
    outcome.fainted_target = sides[defender_side].active().hp_cur == 0;
    outcome
}

fn deal_damage(sides: &mut [Side; 2], target_side: usize, dmg: u16, log: &mut Log) {
    let m = sides[target_side].active_mut();

    // Substitute absorbs damage.
    if m.volatile.has(Volatile::SUBSTITUTED) {
        let sub_hp = m.volatile.substitute_hp as u16;
        if dmg >= sub_hp {
            m.volatile.clear(Volatile::SUBSTITUTED);
            m.volatile.substitute_hp = 0;
        } else {
            m.volatile.substitute_hp = (sub_hp - dmg) as u8;
        }
        return;
    }

    let actual = dmg.min(m.hp_cur);
    m.hp_cur -= actual;
    log.push(Event::Damage {
        side: target_side as u8,
        dealt: actual,
    });
    if m.hp_cur == 0 {
        log.push(Event::Faint { side: target_side as u8 });
    }
}

fn damage_self(sides: &mut [Side; 2], side: usize, dmg: u16, log: &mut Log) {
    deal_damage(sides, side, dmg, log);
}

fn heal_mon(sides: &mut [Side; 2], side: usize, amt: u16, log: &mut Log) {
    let m = sides[side].active_mut();
    let new_hp = (m.hp_cur as u32 + amt as u32).min(m.hp_max as u32) as u16;
    let healed = new_hp - m.hp_cur;
    m.hp_cur = new_hp;
    if healed > 0 {
        log.push(Event::Heal { side: side as u8, amount: healed });
    }
}

fn try_apply_status(sides: &mut [Side; 2], side: usize, status: Status, log: &mut Log) {
    let m = sides[side].active_mut();
    if !matches!(m.status, Status::None) {
        return;
    }
    if !matches!(status, Status::None) {
        m.status = status;
        log.push(Event::StatusInflicted { side: side as u8 });
    }
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
        log.push(Event::StatChanged {
            side: side as u8,
            stat: stat_param,
            delta,
        });
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
        let m = sides[i].active_mut();
        // Burn / Poison
        let max = m.hp_max;
        match m.status {
            Status::Poison => {
                let d = (max / 16).max(1);
                m.hp_cur = m.hp_cur.saturating_sub(d);
                log.push(Event::Damage { side: i as u8, dealt: d });
            }
            Status::Burn => {
                let d = (max / 16).max(1);
                m.hp_cur = m.hp_cur.saturating_sub(d);
                log.push(Event::Damage { side: i as u8, dealt: d });
            }
            Status::BadPoison(c) => {
                let d = ((max as u32 * c as u32) / 16).max(1) as u16;
                m.hp_cur = m.hp_cur.saturating_sub(d);
                log.push(Event::Damage { side: i as u8, dealt: d });
                m.status = Status::BadPoison(c.saturating_add(1).min(15));
            }
            _ => {}
        }
        if m.volatile.has(Volatile::LEECH_SEEDED) && m.hp_cur > 0 {
            let d = (max / 16).max(1);
            m.hp_cur = m.hp_cur.saturating_sub(d);
            log.push(Event::Damage { side: i as u8, dealt: d });
            let healer = 1 - i;
            heal_mon(sides, healer, d, log);
        }
        if sides[i].active().hp_cur == 0 {
            log.push(Event::Faint { side: i as u8 });
        }
    }
    // Decrement screens.
    for s in sides.iter_mut() {
        if s.reflect_turns > 0 {
            s.reflect_turns -= 1;
        }
        if s.light_screen_turns > 0 {
            s.light_screen_turns -= 1;
        }
    }
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
    let m = sides[side].active_mut();
    if m.volatile.has(Volatile::FLINCHED) {
        m.volatile.clear(Volatile::FLINCHED);
        return false;
    }
    if m.volatile.has(Volatile::MUST_RECHARGE) {
        m.volatile.clear(Volatile::MUST_RECHARGE);
        log.push(Event::Recharge { side: side as u8 });
        return false;
    }
    match m.status {
        Status::Freeze => return false,
        Status::Sleep(t) => {
            if t == 0 {
                m.status = Status::None;
                log.push(Event::Wake { side: side as u8 });
                return false; // Gen 1: lose the wake turn.
            } else {
                m.status = Status::Sleep(t - 1);
                return false;
            }
        }
        Status::Paralysis => {
            if rng.byte() < (255u32 / 4) as u8 {
                log.push(Event::FullyParalyzed { side: side as u8 });
                return false;
            }
        }
        _ => {}
    }
    if m.volatile.has(Volatile::CONFUSED) {
        if m.volatile.confused_turns == 0 {
            m.volatile.clear(Volatile::CONFUSED);
        } else {
            m.volatile.confused_turns -= 1;
            if rng.byte() < 128 {
                // hit self for ~40 power typeless physical against own def
                let lvl = m.level as u32;
                let atk = m.stats[1] as u32;
                let def = m.stats[2] as u32;
                let dmg = ((((2 * lvl / 5 + 2) * 40 * atk) / def.max(1)) / 50 + 2) as u16;
                let actual = dmg.min(m.hp_cur);
                m.hp_cur -= actual;
                log.push(Event::ConfusionSelfHit { side: side as u8, dealt: actual });
                return false;
            }
        }
    }
    true
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
        // Format to a compact string at the boundary (heap allocation, but
        // drained per turn — bounded).
        self.pending.push(format!("{}", ev));
    }
}
