//! Move dispatch: maps `MoveEffectKind` to side-effects, plus special-case
//! handlers for the famously weird Gen 1 moves (Counter, Bide, Wrap, etc.).
//!
//! Semantics are ported from Pokémon Showdown's Gen 1 mod (scripts.ts /
//! moves.ts / conditions.ts), the community's cartridge-verified reference.
//! Quirks are intentional; comments call out the ones that look like bugs.

extern crate alloc;

use alloc::format;
use alloc::string::String;

use crate::combat::{
    apply_status_drop, compute_damage, hit_roll, recalc_modified,
};
use crate::log::Event;
use crate::rng::Rng;
use crate::state::{Field, MoveSlot, Side, Status, Volatile};
use crate::tables::{
    move_by_id, MoveCategory, MoveEffectKind, MoveEntry, FLAG_HITS_INVULN, FLAG_IGNORE_IMMUNITY,
    MOVES,
};
use crate::types::Type;

/// Stat encoding in effect_param0 for boost-type effects.
/// Returns the STAGE index: 0=Atk 1=Def 2=Spc 3=Spe 4=Acc 5=Eva.
fn stat_idx_from_param(p: u8) -> usize {
    match p {
        1 => 0,
        2 => 1,
        3 | 4 => 2, // SpDef collapses to Spc in Gen 1
        5 => 3,
        6 => 4,
        7 => 5,
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

fn status_from_param(rng: &mut Rng, p: u8) -> Status {
    match p {
        1 => Status::Poison,
        2 => Status::Burn,
        3 => Status::Freeze,
        4 => Status::Paralysis,
        5 => Status::Sleep((rng.range(7) as u8) + 1), // 1..=7, last is the wake turn
        7 => Status::BadPoison,
        _ => Status::None,
    }
}

/// Moves that do NOT reset the battle's last-damage register when used
/// (cartridge list, via Showdown's SKIP_LASTDAMAGE). Everything else zeroes
/// it at move start — which is why stale Counter reads are possible at all.
fn skips_last_damage(id: &str) -> bool {
    matches!(
        id,
        "confuseray" | "conversion" | "counter" | "focusenergy" | "glare" | "haze"
            | "leechseed" | "lightscreen" | "mimic" | "mist" | "poisongas" | "poisonpowder"
            | "recover" | "reflect" | "rest" | "softboiled" | "splash" | "stunspore"
            | "substitute" | "supersonic" | "teleport" | "thunderwave" | "toxic" | "transform"
    )
}

/// Outcome of one move use.
#[derive(Clone, Copy, Debug, Default)]
pub struct MoveOutcome {
    pub hit: bool,
    pub crit: bool,
    pub damage_dealt: u16,
    pub fainted_target: bool,
    pub fainted_user: bool,
    /// The hit landed on a Substitute instead of the mon.
    pub hit_sub: bool,
    /// This move broke the target's Substitute.
    pub sub_broke: bool,
}

/// Where a chunk of damage landed.
enum HitRes {
    Mon,
    Sub { broke: bool },
}

// ─────────────────────────────────────────────────────────────────────────────
// Entry points
// ─────────────────────────────────────────────────────────────────────────────

/// Side `attacker_side_idx` uses the move in `move_slot_idx`.
pub fn execute_move(
    rng: &mut Rng,
    field: &mut Field,
    sides: &mut [Side; 2],
    attacker_side_idx: usize,
    move_slot_idx: usize,
    log: &mut Log,
) -> MoveOutcome {
    let mv_id = sides[attacker_side_idx].active().moves[move_slot_idx].move_id;
    if mv_id.is_empty() {
        return MoveOutcome::default();
    }
    let Some(mv) = move_by_id(mv_id) else {
        return MoveOutcome::default();
    };
    execute_move_entry(rng, field, sides, attacker_side_idx, mv, Some(move_slot_idx), false, log)
}

/// A locked-in continuation (TwoTurn release, Bide, Wrap, Thrash, Rage).
/// Locked uses never deduct PP — except a TwoTurn release, which is when
/// Gen 1 deducts for two-turn moves.
pub fn execute_locked_move(
    rng: &mut Rng,
    field: &mut Field,
    sides: &mut [Side; 2],
    attacker_side_idx: usize,
    move_id: &str,
    log: &mut Log,
) -> MoveOutcome {
    let Some(mv) = move_by_id(move_id) else {
        return MoveOutcome::default();
    };
    let slot = sides[attacker_side_idx].active().find_move_slot(move_id).map(|s| s as usize);
    execute_move_entry(rng, field, sides, attacker_side_idx, mv, slot, true, log)
}

/// Struggle — forced when every move slot is out of PP. Not slot-based.
pub fn execute_struggle(
    rng: &mut Rng,
    field: &mut Field,
    sides: &mut [Side; 2],
    attacker_side_idx: usize,
    log: &mut Log,
) -> MoveOutcome {
    let Some(mv) = move_by_id("struggle") else {
        return MoveOutcome::default();
    };
    execute_move_entry(rng, field, sides, attacker_side_idx, mv, None, false, log)
}

fn execute_move_entry(
    rng: &mut Rng,
    field: &mut Field,
    sides: &mut [Side; 2],
    attacker_side: usize,
    mv: &'static MoveEntry,
    pp_slot: Option<usize>,
    locked: bool,
    log: &mut Log,
) -> MoveOutcome {
    let outcome = run_move(rng, field, sides, attacker_side, mv, pp_slot, locked, log);

    // Disable and Self-Destruct/Explosion build Rage on the target even when
    // they miss or fail (Gen 1 oddity, Showdown scripts.ts).
    let defender_side = 1 - attacker_side;
    if matches!(mv.effect_kind, MoveEffectKind::Disable | MoveEffectKind::SelfDestruct) {
        rage_build(sides, defender_side, log);
    }
    outcome
}

fn run_move(
    rng: &mut Rng,
    field: &mut Field,
    sides: &mut [Side; 2],
    attacker_side: usize,
    mv: &'static MoveEntry,
    pp_slot: Option<usize>,
    locked: bool,
    log: &mut Log,
) -> MoveOutcome {
    use MoveEffectKind::*;
    let defender_side = 1 - attacker_side;
    let ek = mv.effect_kind;

    // PP: locked continuations don't deduct — except the TwoTurn release,
    // which is exactly when Gen 1 charges the PP for two-turn moves.
    let deduct = if locked {
        ek == TwoTurn && sides[attacker_side].active().volatile.has(Volatile::CHARGING)
    } else {
        ek != TwoTurn
    };
    if deduct {
        if let Some(slot) = pp_slot {
            let a = sides[attacker_side].active_mut();
            if a.moves[slot].pp > 0 {
                a.moves[slot].pp -= 1;
            }
        }
    }

    sides[attacker_side].active_mut().last_move_used = mv.id;
    sides[attacker_side].last_move_used = mv.id;

    {
        let s = &sides[attacker_side];
        log.push_board(format!("move|mon:{},{},0|name:{}", s.active().name, s.player_id, mv.name));
    }

    // A Bide turn resolves before the move-use pipeline on the cartridge:
    // it never resets the last-damage register (that's the accumulator bug),
    // never rolls accuracy, and its unleash pierces invulnerability.
    if ek == Bide && sides[attacker_side].active().volatile.has(Volatile::BIDING) {
        return apply_effect(rng, field, mv, sides, attacker_side, locked, false, log);
    }

    // Any move outside the cartridge skip-list zeroes the last-damage register.
    if !skips_last_damage(mv.id) {
        field.last_damage = 0;
    }

    // Partial-trapping moves negate Hyper Beam's recharge, even if they miss.
    if ek == Wrap && sides[defender_side].active().volatile.has(Volatile::MUST_RECHARGE) {
        sides[defender_side].active_mut().volatile.clear(Volatile::MUST_RECHARGE);
    }

    // Thrash/Petal Dance and Rage set their locks BEFORE the accuracy roll
    // (a first-turn miss still locks you in).
    if ek == ThrashLock {
        let a = sides[attacker_side].active_mut();
        if locked && a.volatile.multi_turn_move == mv.id {
            a.volatile.multi_turn_turns = a.volatile.multi_turn_turns.saturating_sub(1);
            if a.volatile.multi_turn_turns == 0 {
                // Lock ends this turn: the move still executes, then confusion
                // sets in — even if the user was already confused.
                a.volatile.multi_turn_move = "";
                a.volatile.locked_acc = 0;
                a.volatile.set(Volatile::CONFUSED);
                a.volatile.confused_turns = (rng.range(4) as u8) + 1;
                let s = &sides[attacker_side];
                log.push_board(format!("start|mon:{},{},0|what:confusion", s.active().name, s.player_id));
            }
        } else if !locked {
            a.volatile.multi_turn_move = mv.id;
            a.volatile.multi_turn_turns = 1 + (rng.coin() as u8); // 2..=3 total uses + final
            a.volatile.multi_turn_turns += 1;
            a.volatile.locked_acc = 255;
        }
    }
    if ek == Rage && !sides[attacker_side].active().volatile.has(Volatile::RAGE) {
        let a = sides[attacker_side].active_mut();
        a.volatile.set(Volatile::RAGE);
        a.volatile.multi_turn_move = mv.id; // locked in for the rest of the battle
        a.volatile.multi_turn_turns = 255;
        a.volatile.locked_acc = 255;
    }

    // Gen 1 bug: sleep moves against a recharging target skip the accuracy
    // roll and overwrite any existing status.
    let sleep_vs_recharge = ek == StatusOnly
        && mv.effect_param0 == 5
        && sides[defender_side].active().volatile.has(Volatile::MUST_RECHARGE);

    let targets_foe = !matches!(
        ek,
        BoostSelf | HealHalf | Rest | Substitute | LightScreen | Reflect | Mist | FocusEnergy
            | Haze | Metronome | MirrorMove | NoOp | Bide | ThrashLock
    );

    // Fly/Dig semi-invulnerability (Swift, Transform and Bide bypass it).
    if targets_foe
        && sides[defender_side].active().volatile.has(Volatile::INVULNERABLE)
        && (mv.flags & FLAG_HITS_INVULN) == 0
    {
        {
            let s = &sides[attacker_side];
            log.push_board(format!("miss|mon:{},{},0", s.active().name, s.player_id));
        }
        return miss_aftermath(rng, field, mv, sides, attacker_side, log);
    }

    // Dream Eater fails against an awake target before accuracy is rolled.
    if ek == DreamEater && !matches!(sides[defender_side].active().status, Status::Sleep(_)) {
        let s = &sides[defender_side];
        log.push_board(format!("immune|mon:{},{},0", s.active().name, s.player_id));
        return MoveOutcome::default();
    }

    // Type immunity, checked before accuracy. Only DAMAGING moves respect it:
    // Gen 1 status moves ignore the chart entirely (Thunder Wave paralyzes
    // Ground-types), and fixed-damage / trapping moves carry an explicit
    // ignore flag (Sonic Boom hits Gengar; Wrap traps Ghosts).
    if mv.category != MoveCategory::Status
        && (mv.flags & FLAG_IGNORE_IMMUNITY) == 0
        && ek != Counter
    {
        let d = sides[defender_side].active();
        let immune = crate::tables::type_effectiveness(mv.move_type, d.primary_type) == 0
            || (d.secondary_type != Type::None
                && crate::tables::type_effectiveness(mv.move_type, d.secondary_type) == 0);
        if immune {
            {
                let s = &sides[defender_side];
                log.push_board(format!("immune|mon:{},{},0", s.active().name, s.player_id));
            }
            return miss_aftermath(rng, field, mv, sides, attacker_side, log);
        }
    }

    // OHKO moves fail outright against a faster target (checked pre-accuracy).
    if ek == Ohko {
        let a_spe = sides[attacker_side].active().modified[4];
        let d_spe = sides[defender_side].active().modified[4];
        if d_spe > a_spe {
            let s = &sides[defender_side];
            log.push_board(format!("immune|mon:{},{},0", s.active().name, s.player_id));
            return MoveOutcome::default();
        }
    }

    // Accuracy.
    let skip_acc = mv.accuracy == 0
        || sleep_vs_recharge
        || (ek == Wrap && locked && sides[defender_side].active().volatile.has(Volatile::TRAPPED))
        || (ek == TwoTurn && !sides[attacker_side].active().volatile.has(Volatile::CHARGING));
    if !skip_acc {
        let (acc_stage, eva_stage) = {
            let a = sides[attacker_side].active();
            let d = sides[defender_side].active();
            (a.stages[4], d.stages[5])
        };
        // Thrash/Rage accuracy bug: stage multipliers compound onto LAST
        // turn's effective accuracy instead of the move's base accuracy.
        let locked_bug = {
            let a = sides[attacker_side].active();
            a.volatile.locked_acc > 0
                && (a.volatile.has(Volatile::RAGE) || ek == ThrashLock)
        };
        let base = if locked_bug {
            sides[attacker_side].active().volatile.locked_acc as u32
        } else {
            mv.accuracy as u32 * 255 / 100
        };
        let (hit, eff) = hit_roll(rng, base, acc_stage, eva_stage);
        if locked_bug {
            sides[attacker_side].active_mut().volatile.locked_acc = eff;
        }
        if !hit {
            {
                let s = &sides[attacker_side];
                log.push_board(format!("miss|mon:{},{},0", s.active().name, s.player_id));
            }
            field.last_damage = 0;
            return miss_aftermath(rng, field, mv, sides, attacker_side, log);
        }
    }

    apply_effect(rng, field, mv, sides, attacker_side, locked, sleep_vs_recharge, log)
}

/// Post-miss / post-immunity side effects: crash damage (1 HP, Gen 1),
/// Self-Destruct fainting anyway, and a missing trap move dropping its lock.
fn miss_aftermath(
    _rng: &mut Rng,
    field: &mut Field,
    mv: &MoveEntry,
    sides: &mut [Side; 2],
    attacker_side: usize,
    log: &mut Log,
) -> MoveOutcome {
    let mut outcome = MoveOutcome::default();
    match mv.effect_kind {
        MoveEffectKind::CrashOnMiss => {
            // Gen 1 quirk: crash damage is exactly 1 HP — and it obeys the
            // confusion/crash Substitute misdirection.
            self_hit_with_sub_redirect(field, sides, attacker_side, 1, log);
        }
        MoveEffectKind::SelfDestruct => {
            sides[attacker_side].active_mut().hp_cur = 0;
            outcome.fainted_user = true;
            let s = &sides[attacker_side];
            log.push_board(format!("faint|mon:{},{},0", s.active().name, s.player_id));
        }
        MoveEffectKind::Wrap => {
            let a = sides[attacker_side].active_mut();
            a.volatile.multi_turn_move = "";
            a.volatile.multi_turn_turns = 0;
            a.volatile.stored_damage = 0;
        }
        _ => {}
    }
    outcome
}

// ─────────────────────────────────────────────────────────────────────────────
// Effect application
// ─────────────────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn apply_effect(
    rng: &mut Rng,
    field: &mut Field,
    mv: &'static MoveEntry,
    sides: &mut [Side; 2],
    attacker_side: usize,
    locked: bool,
    sleep_vs_recharge: bool,
    log: &mut Log,
) -> MoveOutcome {
    let defender_side = 1 - attacker_side;
    use MoveEffectKind::*;
    let mut outcome = MoveOutcome { hit: true, ..Default::default() };

    match mv.effect_kind {
        Damage | CrashOnMiss => {
            outcome = damage_step(rng, field, sides, attacker_side, mv, false, None, log, outcome);
        }
        DamageMaybeStatus => {
            outcome = damage_step(rng, field, sides, attacker_side, mv, false, None, log, outcome);
            if outcome.hit && outcome.damage_dealt > 0 && !outcome.fainted_target {
                if mv.effect_param0 == 6 {
                    // Confusion secondary pierces a Substitute as long as the
                    // sub didn't break (Gen 1 quirk).
                    if !outcome.hit_sub || !outcome.sub_broke {
                        if roll_chance_byte(rng, mv.effect_param1) {
                            try_apply_confusion(rng, sides, defender_side, log);
                        }
                    }
                } else if !outcome.hit_sub {
                    // A par/brn/frz secondary never affects a target sharing
                    // the move's type (Body Slam can't paralyze Normals) —
                    // the RNG isn't even consulted in that case.
                    let type_blocked = matches!(mv.effect_param0, 2 | 3 | 4)
                        && sides[defender_side].active().is_type(mv.move_type);
                    if !type_blocked && roll_chance_byte(rng, mv.effect_param1) {
                        let st = status_from_param(rng, mv.effect_param0);
                        try_apply_status(sides, defender_side, st, false, log);
                    }
                }
                // Fire moves with a burn chance thaw a frozen target.
                if mv.effect_param0 == 2
                    && !outcome.hit_sub
                    && sides[defender_side].active().status == Status::Freeze
                {
                    sides[defender_side].active_mut().status = Status::None;
                    let s = &sides[defender_side];
                    log.push_board(format!("curestatus|mon:{},{},0|status:frz", s.active().name, s.player_id));
                }
            }
        }
        DamageMaybeFlinch => {
            outcome = damage_step(rng, field, sides, attacker_side, mv, false, None, log, outcome);
            if outcome.hit
                && outcome.damage_dealt > 0
                && !outcome.fainted_target
                && !outcome.hit_sub
                && roll_chance_byte(rng, mv.effect_param0)
            {
                let d = sides[defender_side].active_mut();
                d.volatile.set(Volatile::FLINCHED);
                // Flinching clears a pending recharge (Gen 1).
                d.volatile.clear(Volatile::MUST_RECHARGE);
            }
        }
        DamageMaybeBoostTarget => {
            outcome = damage_step(rng, field, sides, attacker_side, mv, false, None, log, outcome);
            if outcome.hit
                && outcome.damage_dealt > 0
                && !outcome.fainted_target
                && !outcome.hit_sub
                && roll_chance_byte(rng, 85)
            {
                let delta = mv.effect_param1 as i8;
                let _ = apply_stage_change(
                    sides, defender_side, attacker_side, mv.effect_param0, delta, true, log,
                );
            }
        }
        BoostSelf => {
            let delta = mv.effect_param1 as i8;
            if !apply_stage_change(sides, attacker_side, attacker_side, mv.effect_param0, delta, true, log) {
                fail_log(sides, attacker_side, log);
            }
        }
        BoostTarget => {
            let delta = mv.effect_param1 as i8;
            let blocked = (delta < 0 && sides[defender_side].active().volatile.has(Volatile::MIST))
                // A Substitute blocks stat-reducing moves.
                || sides[defender_side].active().volatile.has(Volatile::SUBSTITUTED);
            if blocked
                || !apply_stage_change(sides, defender_side, attacker_side, mv.effect_param0, delta, true, log)
            {
                fail_log(sides, attacker_side, log);
            }
        }
        StatusOnly => {
            apply_status_move(rng, field, mv, sides, attacker_side, sleep_vs_recharge, log);
        }
        MultiHit2to5 | MultiHitFixed | Twineedle => {
            let hits: u8 = match mv.effect_kind {
                MultiHit2to5 => *pick(rng, &[2, 2, 2, 3, 3, 3, 4, 5]),
                _ => mv.effect_param0.max(1),
            };
            // Gen 1: every hit of a multi-hit move deals the SAME damage as
            // the first (one damage calc, one crit roll).
            let mut per_hit: Option<u16> = None;
            let mut landed = 0u8;
            for _ in 0..hits {
                let before = outcome.damage_dealt;
                outcome = damage_step(
                    rng, field, sides, attacker_side, mv, false, per_hit, log, outcome,
                );
                if !outcome.hit {
                    break;
                }
                let dealt = outcome.damage_dealt - before;
                if per_hit.is_none() {
                    per_hit = Some(dealt);
                }
                landed += 1;
                if outcome.fainted_target || outcome.sub_broke {
                    break;
                }
            }
            let _ = landed;
            // Twineedle rolls its poison chance once, after the last hit.
            if mv.effect_kind == Twineedle
                && outcome.hit
                && outcome.damage_dealt > 0
                && !outcome.fainted_target
                && !outcome.hit_sub
                && roll_chance_byte(rng, mv.effect_param1)
            {
                try_apply_status(sides, defender_side, Status::Poison, false, log);
            }
        }
        DrainHp | DreamEater => {
            outcome = damage_step(rng, field, sides, attacker_side, mv, false, None, log, outcome);
            // Drain doesn't happen if the hit broke a Substitute.
            if outcome.hit && outcome.damage_dealt > 0 && !outcome.sub_broke {
                let heal = (outcome.damage_dealt / 2).max(1);
                if outcome.hit_sub {
                    // Gen 1 oddity: draining off a Substitute leaves the DRAIN
                    // amount in the last-damage register.
                    field.last_damage = heal;
                }
                heal_mon(sides, attacker_side, heal, log);
            }
        }
        Recoil1of4 | StruggleRecoil => {
            outcome = damage_step(rng, field, sides, attacker_side, mv, false, None, log, outcome);
            if outcome.hit && outcome.damage_dealt > 0 && !outcome.sub_broke {
                let div = if mv.effect_kind == StruggleRecoil { 2 } else { 4 };
                let recoil = (outcome.damage_dealt / div).max(1);
                direct_hp_loss(field, sides, attacker_side, recoil, true, log);
                outcome.fainted_user = sides[attacker_side].active().hp_cur == 0;
            }
        }
        Ohko => {
            // Speed check already passed; accuracy already rolled (30% base,
            // subject to stages and the 1/256). Damage is 65535 in Gen 1.
            let res = deal_damage(field, sides, defender_side, u16::MAX, log);
            note_hit(&mut outcome, res, u16::MAX);
            outcome.fainted_target = sides[defender_side].active().hp_cur == 0;
        }
        ForceSwitchTarget => {
            // Whirlwind/Roar/Teleport: always fail in Gen 1 link battles.
            fail_log(sides, attacker_side, log);
        }
        LevelDamage => {
            let dmg = sides[attacker_side].active().level as u16;
            let res = deal_damage(field, sides, defender_side, dmg, log);
            note_hit(&mut outcome, res, dmg);
            outcome.fainted_target = sides[defender_side].active().hp_cur == 0;
        }
        FlatDamage => {
            let dmg = mv.effect_param0 as u16;
            let res = deal_damage(field, sides, defender_side, dmg, log);
            note_hit(&mut outcome, res, dmg);
            outcome.fainted_target = sides[defender_side].active().hp_cur == 0;
        }
        Psywave => {
            // damage = random(1 .. 1.5×level - 1); the cartridge softlocks at
            // levels 0/1/171 — we just floor the range at 1 instead.
            let lvl = sides[attacker_side].active().level as u32;
            let cap = (lvl * 3 / 2).saturating_sub(1).max(1);
            let dmg = (rng.range(cap) + 1) as u16;
            let res = deal_damage(field, sides, defender_side, dmg, log);
            note_hit(&mut outcome, res, dmg);
            outcome.fainted_target = sides[defender_side].active().hp_cur == 0;
        }
        HalfHp => {
            let dmg = (sides[defender_side].active().hp_cur / 2).max(1);
            let res = deal_damage(field, sides, defender_side, dmg, log);
            note_hit(&mut outcome, res, dmg);
        }
        HealHalf => {
            let (cur, max) = {
                let a = sides[attacker_side].active();
                (a.hp_cur, a.hp_max)
            };
            if recovery_fails(cur, max) {
                fail_log(sides, attacker_side, log);
            } else {
                heal_mon(sides, attacker_side, max / 2, log);
            }
        }
        Rest => {
            let (cur, max) = {
                let a = sides[attacker_side].active();
                (a.hp_cur, a.hp_max)
            };
            if recovery_fails(cur, max) {
                fail_log(sides, attacker_side, log);
            } else {
                let a = sides[attacker_side].active_mut();
                a.hp_cur = max;
                // Overwrites ANY status; the toxic counter volatile survives
                // (Gen 1 Toxic/Rest glitch) and par/brn drops stay applied.
                a.status = Status::Sleep(2);
                a.rest_sleep = true; // exempt from Sleep Clause Mod
                let s = &sides[attacker_side];
                log.push_board(format!("status|mon:{},{},0|status:slp", s.active().name, s.player_id));
                let m = s.active();
                log.push_board(format!("heal|mon:{},{},0|health:{}/{}", m.name, s.player_id, m.hp_cur, m.hp_max));
            }
        }
        TwoTurn => {
            let charging = sides[attacker_side].active().volatile.has(Volatile::CHARGING);
            if charging {
                // Turn 2: deliver the damage, clear charging state.
                let a = sides[attacker_side].active_mut();
                a.volatile.clear(Volatile::CHARGING);
                a.volatile.clear(Volatile::INVULNERABLE);
                a.volatile.multi_turn_move = "";
                a.volatile.multi_turn_turns = 0;
                outcome = damage_step(rng, field, sides, attacker_side, mv, false, None, log, outcome);
            } else {
                let a = sides[attacker_side].active_mut();
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
            let biding = sides[attacker_side].active().volatile.has(Volatile::BIDING);
            if biding {
                // Each Bide turn adds the battle's last-damage register —
                // including stale values (the Gen 1 Bide accumulator bug).
                let a = sides[attacker_side].active_mut();
                a.volatile.stored_damage = a.volatile.stored_damage.saturating_add(field.last_damage);
                a.volatile.bide_turns = a.volatile.bide_turns.saturating_sub(1);
                if a.volatile.bide_turns > 0 {
                    let s = &sides[attacker_side];
                    log.push_board(format!("start|mon:{},{},0|what:bide", s.active().name, s.player_id));
                } else {
                    let stored = a.volatile.stored_damage;
                    a.volatile.clear(Volatile::BIDING);
                    a.volatile.stored_damage = 0;
                    a.volatile.multi_turn_move = "";
                    a.volatile.multi_turn_turns = 0;
                    {
                        let s = &sides[attacker_side];
                        log.push_board(format!("end|mon:{},{},0|what:bide", s.active().name, s.player_id));
                    }
                    if stored == 0 {
                        fail_log(sides, attacker_side, log);
                    } else {
                        // Typeless, uncounterable by type charts, and it hits
                        // through Fly/Dig invulnerability.
                        let dmg = stored.saturating_mul(2);
                        let res = deal_damage(field, sides, defender_side, dmg, log);
                        note_hit(&mut outcome, res, dmg);
                        outcome.fainted_target = sides[defender_side].active().hp_cur == 0;
                    }
                }
            } else {
                let a = sides[attacker_side].active_mut();
                a.volatile.set(Volatile::BIDING);
                a.volatile.stored_damage = 0;
                a.volatile.bide_turns = 2 + (rng.coin() as u8); // 2..=3
                a.volatile.multi_turn_move = mv.id;
                a.volatile.multi_turn_turns = a.volatile.bide_turns;
                let s = &sides[attacker_side];
                log.push_board(format!("start|mon:{},{},0|what:bide", s.active().name, s.player_id));
            }
        }
        HyperBeam => {
            outcome = damage_step(rng, field, sides, attacker_side, mv, false, None, log, outcome);
            // No recharge if the target was KO'd — or if the hit broke a
            // Substitute (both Gen 1 quirks).
            if outcome.hit && outcome.damage_dealt > 0 && !outcome.fainted_target && !outcome.sub_broke {
                sides[attacker_side].active_mut().volatile.set(Volatile::MUST_RECHARGE);
            }
        }
        Counter => {
            // Gen 1 Counter (Desync Clause flavor): succeeds iff the enemy's
            // last SELECTED move isn't Counter, their last USED move is
            // Normal/Fighting with base power > 0, and the battle-global
            // last-damage register is non-zero. Deals 2× that register —
            // which persists across turns and includes residual damage.
            let enemy_selected = sides[defender_side].last_selected_move;
            let enemy_used = sides[defender_side].last_move_used;
            let counterable = move_by_id(enemy_used)
                .map(|m| {
                    m.power > 0 && matches!(m.move_type, Type::Normal | Type::Fighting)
                })
                .unwrap_or(false);
            if enemy_selected != "counter" && counterable && field.last_damage > 0 {
                let dmg = field.last_damage.saturating_mul(2);
                let res = deal_damage(field, sides, defender_side, dmg, log);
                note_hit(&mut outcome, res, dmg);
                outcome.fainted_target = sides[defender_side].active().hp_cur == 0;
            } else {
                fail_log(sides, attacker_side, log);
            }
        }
        MirrorMove => {
            let last = sides[defender_side].active().last_move_used;
            if last.is_empty() || last == "mirrormove" {
                fail_log(sides, attacker_side, log);
            } else if let Some(mv2) = move_by_id(last) {
                outcome = execute_move_entry(rng, field, sides, attacker_side, mv2, None, false, log);
            }
        }
        Mimic => {
            let target_moves: heapless::Vec<&'static str, 4> = sides[defender_side]
                .active()
                .moves
                .iter()
                .filter(|s| !s.move_id.is_empty())
                .map(|s| s.move_id)
                .collect();
            if target_moves.is_empty() {
                fail_log(sides, attacker_side, log);
                return outcome;
            }
            // Gen 1 Mimic copies a RANDOM move of the target.
            let new_move = target_moves[(rng.range(target_moves.len() as u32)) as usize];
            let a = sides[attacker_side].active_mut();
            let mimic_slot = a.find_move_slot("mimic").unwrap_or(0) as usize;
            let copied_max = move_by_id(new_move).map(|m| m.pp).unwrap_or(5);
            // Current PP carries over from the Mimic slot (Gen 1).
            let cur_pp = a.moves[mimic_slot].pp;
            a.moves[mimic_slot] = MoveSlot {
                move_id: new_move,
                pp: cur_pp,
                max_pp: copied_max,
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
            // Snapshot the original once (re-transforming keeps the first
            // backup) — restored on switch-out / faint.
            if !sides[attacker_side].active().volatile.has(Volatile::TRANSFORMED) {
                let a = sides[attacker_side].active();
                sides[attacker_side].transform_backup = Some(crate::state::TransformBackup {
                    species_id: a.species_id,
                    primary_type: a.primary_type,
                    secondary_type: a.secondary_type,
                    stats: a.stats,
                    moves: a.moves,
                });
            }
            let a = sides[attacker_side].active_mut();
            a.species_id = target.species_id;
            a.primary_type = target.primary_type;
            a.secondary_type = target.secondary_type;
            // Stats: copy non-HP only (base_spe stays — the cartridge keeps
            // using the ORIGINAL species' base Speed for crits).
            for i in 1..5 {
                a.stats[i] = target.stats[i];
                a.modified[i] = target.modified[i];
            }
            a.stages = target.stages;
            for (i, m) in target.moves.iter().enumerate() {
                if m.move_id.is_empty() {
                    a.moves[i] = MoveSlot::default();
                } else {
                    a.moves[i] = MoveSlot { move_id: m.move_id, pp: 5, max_pp: 5 };
                }
            }
            a.volatile.set(Volatile::TRANSFORMED);
            let s = &sides[attacker_side];
            log.push_board(format!(
                "start|mon:{},{},0|what:transform|move:{}",
                s.active().name, s.player_id, target.name
            ));
            log.push_board(format!(
                "activemon|mon:{},{},0|name:{}|speed:{}",
                s.active().name, s.player_id, target.name, s.active().stats[4]
            ));
        }
        Substitute => {
            let (cur, max, has_sub) = {
                let a = sides[attacker_side].active();
                (a.hp_cur, a.hp_max, a.volatile.has(Volatile::SUBSTITUTED))
            };
            if has_sub {
                fail_log(sides, attacker_side, log);
            } else if (cur as u32) * 4 < max as u32 {
                // Fails strictly below 1/4 max HP…
                fail_log(sides, attacker_side, log);
            } else {
                // …but at exactly 1/4 you're allowed to make one and FAINT.
                let cost = if max <= 3 { 0 } else { max / 4 };
                let a = sides[attacker_side].active_mut();
                a.hp_cur = a.hp_cur.saturating_sub(cost);
                a.volatile.set(Volatile::SUBSTITUTED);
                a.volatile.substitute_hp = ((max / 4) + 1).min(255) as u8;
                // Making a Substitute frees you from a partial trap.
                a.volatile.clear(Volatile::TRAPPED);
                let s = &sides[attacker_side];
                log.push_board(format!("start|mon:{},{},0|what:substitute", s.active().name, s.player_id));
                let m = s.active();
                log.push_board(format!("damage|mon:{},{},0|health:{}/{}", m.name, s.player_id, m.hp_cur, m.hp_max));
                if m.hp_cur == 0 {
                    log.push_board(format!("faint|mon:{},{},0", m.name, s.player_id));
                    outcome.fainted_user = true;
                }
            }
        }
        Disable => {
            let candidates: heapless::Vec<u8, 4> = sides[defender_side]
                .active()
                .moves
                .iter()
                .enumerate()
                .filter(|(_, s)| !s.move_id.is_empty() && s.pp > 0)
                .map(|(i, _)| i as u8)
                .collect();
            if candidates.is_empty()
                || sides[defender_side].active().volatile.has(Volatile::DISABLED)
            {
                fail_log(sides, attacker_side, log);
            } else {
                let pick = candidates[(rng.range(candidates.len() as u32)) as usize];
                let turns = (rng.range(8) as u8) + 1; // 1..=8
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
            let continuing = locked
                && sides[attacker_side].active().volatile.multi_turn_move == mv.id
                && sides[defender_side].active().volatile.has(Volatile::TRAPPED);
            if continuing {
                // Auto-hit, repeating the FIRST hit's damage.
                let dmg = sides[attacker_side].active().volatile.stored_damage;
                if dmg > 0 {
                    let res = deal_damage(field, sides, defender_side, dmg, log);
                    note_hit(&mut outcome, res, dmg);
                    outcome.fainted_target = sides[defender_side].active().hp_cur == 0;
                } else {
                    outcome.hit = true;
                }
                let a = sides[attacker_side].active_mut();
                a.volatile.multi_turn_turns = a.volatile.multi_turn_turns.saturating_sub(1);
                if a.volatile.multi_turn_turns == 0 || outcome.fainted_target {
                    a.volatile.multi_turn_move = "";
                    a.volatile.multi_turn_turns = 0;
                    a.volatile.stored_damage = 0;
                }
                // The victim stays TRAPPED through this turn; battle EOT
                // cleanup frees it once the lock is gone.
            } else {
                // Fresh use (initial, or the victim switched mid-lock).
                // A Normal-type trap move "hits" Ghosts for 0 damage but
                // still traps them (ignore-immunity flag + this path).
                let roll = compute_damage(
                    rng,
                    sides[attacker_side].active(),
                    sides[defender_side].active(),
                    mv,
                    false,
                );
                if roll.dmg == 0 && !roll.immune {
                    // 0-damage glitch: the trap move outright misses.
                    let s = &sides[attacker_side];
                    log.push_board(format!("miss|mon:{},{},0", s.active().name, s.player_id));
                    return outcome;
                }
                let mut dealt = 0u16;
                if roll.dmg > 0 {
                    if roll.crit {
                        let s = &sides[defender_side];
                        log.push_board(format!("crit|mon:{},{},0", s.active().name, s.player_id));
                    }
                    if roll.effectiveness > 100 {
                        let s = &sides[defender_side];
                        log.push_board(format!("supereffective|mon:{},{},0", s.active().name, s.player_id));
                    } else if roll.effectiveness > 0 && roll.effectiveness < 100 {
                        let s = &sides[defender_side];
                        log.push_board(format!("resisted|mon:{},{},0", s.active().name, s.player_id));
                    }
                    let res = deal_damage(field, sides, defender_side, roll.dmg, log);
                    note_hit(&mut outcome, res, roll.dmg);
                    dealt = field.last_damage;
                    outcome.crit = roll.crit;
                    outcome.fainted_target = sides[defender_side].active().hp_cur == 0;
                }
                if !outcome.fainted_target {
                    let total = *pick(rng, &[2, 2, 2, 3, 3, 3, 4, 5]);
                    {
                        let a = sides[attacker_side].active_mut();
                        a.volatile.multi_turn_move = mv.id;
                        a.volatile.multi_turn_turns = total - 1; // remaining auto-uses
                        a.volatile.stored_damage = dealt;
                    }
                    sides[defender_side].active_mut().volatile.set(Volatile::TRAPPED);
                    let s = &sides[defender_side];
                    log.push_board(format!("start|mon:{},{},0|what:wrap", s.active().name, s.player_id));
                } else {
                    let a = sides[attacker_side].active_mut();
                    a.volatile.multi_turn_move = "";
                    a.volatile.multi_turn_turns = 0;
                    a.volatile.stored_damage = 0;
                }
            }
        }
        LeechSeed => {
            let d = sides[defender_side].active();
            if d.is_type(Type::Grass) {
                let s = &sides[defender_side];
                log.push_board(format!("immune|mon:{},{},0", s.active().name, s.player_id));
            } else if d.volatile.has(Volatile::LEECH_SEEDED) {
                fail_log(sides, attacker_side, log);
            } else {
                // Note: Leech Seed goes straight through a Substitute in Gen 1.
                sides[defender_side].active_mut().volatile.set(Volatile::LEECH_SEEDED);
                let s = &sides[defender_side];
                log.push_board(format!("start|mon:{},{},0|what:seeded", s.active().name, s.player_id));
            }
        }
        LightScreen => {
            if sides[attacker_side].active().volatile.has(Volatile::LIGHT_SCREEN) {
                fail_log(sides, attacker_side, log);
            } else {
                sides[attacker_side].active_mut().volatile.set(Volatile::LIGHT_SCREEN);
                let s = &sides[attacker_side];
                log.push_board(format!("start|mon:{},{},0|what:lightscreen", s.active().name, s.player_id));
            }
        }
        Reflect => {
            if sides[attacker_side].active().volatile.has(Volatile::REFLECT) {
                fail_log(sides, attacker_side, log);
            } else {
                sides[attacker_side].active_mut().volatile.set(Volatile::REFLECT);
                let s = &sides[attacker_side];
                log.push_board(format!("start|mon:{},{},0|what:reflect", s.active().name, s.player_id));
            }
        }
        Mist => {
            if sides[attacker_side].active().volatile.has(Volatile::MIST) {
                fail_log(sides, attacker_side, log);
            } else {
                sides[attacker_side].active_mut().volatile.set(Volatile::MIST);
                let s = &sides[attacker_side];
                log.push_board(format!("start|mon:{},{},0|what:mist", s.active().name, s.player_id));
            }
        }
        FocusEnergy => {
            if sides[attacker_side].active().volatile.has(Volatile::FOCUS_ENERGY) {
                fail_log(sides, attacker_side, log);
            } else {
                sides[attacker_side].active_mut().volatile.set(Volatile::FOCUS_ENERGY);
                let s = &sides[attacker_side];
                log.push_board(format!("start|mon:{},{},0|what:focusenergy", s.active().name, s.player_id));
            }
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
            apply_haze(field, sides, attacker_side, log);
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
            let cand: &'static MoveEntry = &MOVES[idx];
            outcome = execute_move_entry(rng, field, sides, attacker_side, cand, None, false, log);
        }
        SelfDestruct => {
            outcome = damage_step(rng, field, sides, attacker_side, mv, true, None, log, outcome);
            // The user survives if (and only if) the blast broke a Substitute.
            if !outcome.sub_broke {
                sides[attacker_side].active_mut().hp_cur = 0;
                outcome.fainted_user = true;
                let s = &sides[attacker_side];
                log.push_board(format!("faint|mon:{},{},0", s.active().name, s.player_id));
            }
        }
        Rage | ThrashLock => {
            // Locking was handled pre-accuracy; the move itself just hits.
            outcome = damage_step(rng, field, sides, attacker_side, mv, false, None, log, outcome);
        }
        NoOp => {
            fail_log(sides, attacker_side, log);
        }
    }
    outcome
}

fn apply_status_move(
    rng: &mut Rng,
    _field: &mut Field,
    mv: &MoveEntry,
    sides: &mut [Side; 2],
    attacker_side: usize,
    sleep_vs_recharge: bool,
    log: &mut Log,
) {
    let defender_side = 1 - attacker_side;
    match mv.effect_param0 {
        6 => {
            // Confuse Ray / Supersonic — blocked by a Substitute.
            if sides[defender_side].active().volatile.has(Volatile::SUBSTITUTED)
                || sides[defender_side].active().volatile.has(Volatile::CONFUSED)
            {
                fail_log(sides, attacker_side, log);
            } else {
                try_apply_confusion(rng, sides, defender_side, log);
            }
        }
        5 => {
            if sleep_vs_recharge {
                // Gen 1 bug: a sleep move on a recharging target always hits,
                // overwrites ANY status, and clears the recharge. The par/brn
                // stat drops are NOT reverted (sticky modified stats).
                // If Sleep Clause Mod blocks it, the recharge is NOT cleared.
                if sleep_clause_blocks(&sides[defender_side]) {
                    fail_log(sides, attacker_side, log);
                    return;
                }
                let d = sides[defender_side].active_mut();
                d.volatile.clear(Volatile::MUST_RECHARGE);
                d.status = Status::Sleep((rng.range(7) as u8) + 1);
                d.rest_sleep = false;
                let s = &sides[defender_side];
                log.push_board(format!("status|mon:{},{},0|status:slp", s.active().name, s.player_id));
            } else if !matches!(sides[defender_side].active().status, Status::None) {
                fail_log(sides, attacker_side, log);
            } else {
                let st = Status::Sleep((rng.range(7) as u8) + 1);
                if !try_apply_status(sides, defender_side, st, true, log) {
                    fail_log(sides, attacker_side, log);
                }
            }
        }
        1 | 7 => {
            // Poison / Toxic — blocked by a Substitute; Poison-types immune.
            let st = status_from_param(rng, mv.effect_param0);
            if sides[defender_side].active().volatile.has(Volatile::SUBSTITUTED) {
                fail_log(sides, attacker_side, log);
            } else if sides[defender_side].active().is_type(Type::Poison) {
                let s = &sides[defender_side];
                log.push_board(format!("immune|mon:{},{},0", s.active().name, s.player_id));
            } else if !try_apply_status(sides, defender_side, st, true, log) {
                fail_log(sides, attacker_side, log);
            }
        }
        _ => {
            // Paralysis (and any other primary status): goes through subs,
            // ignores type immunity (Thunder Wave hits Ground-types).
            let st = status_from_param(rng, mv.effect_param0);
            if !try_apply_status(sides, defender_side, st, true, log) {
                fail_log(sides, attacker_side, log);
            }
        }
    }
}

/// Gen 1 Haze: clears stat stages on BOTH actives (recomputing modified stats,
/// which erases par/brn drops), cures the FOE's major status, downgrades
/// Toxic to regular poison on both, and removes a specific volatile set —
/// Substitute, locks, and charge states survive.
fn apply_haze(field: &mut Field, sides: &mut [Side; 2], user_side: usize, log: &mut Log) {
    let foe_side = 1 - user_side;
    for i in 0..2 {
        let m = sides[i].active_mut();
        m.stages = [0; 6];
        for stat_i in 1..5 {
            recalc_modified(m, stat_i);
        }
        if m.status == Status::BadPoison {
            m.status = Status::Poison;
        }
        m.volatile.clear(
            Volatile::DISABLED
                | Volatile::CONFUSED
                | Volatile::MIST
                | Volatile::FOCUS_ENERGY
                | Volatile::LEECH_SEEDED
                | Volatile::LIGHT_SCREEN
                | Volatile::REFLECT
                | Volatile::TOX_COUNTER,
        );
        m.volatile.confused_turns = 0;
        m.volatile.disabled_slot = 0;
        m.volatile.disabled_turns = 0;
        m.volatile.toxic_counter = 0;
    }
    // Foe's status is cured outright; if that unfreezes/wakes a foe that
    // hasn't acted yet this turn, it loses that action (Gen 1 "cannotmove").
    let foe_status = sides[foe_side].active().status;
    if !matches!(foe_status, Status::None) {
        if matches!(foe_status, Status::Sleep(_) | Status::Freeze) && !field.foe_acted {
            sides[foe_side].active_mut().volatile.set(Volatile::SKIP_TURN);
        }
        sides[foe_side].active_mut().status = Status::None;
    }
    let s = &sides[user_side];
    log.push_board(format!("start|mon:{},{},0|what:haze", s.active().name, s.player_id));
}

// ─────────────────────────────────────────────────────────────────────────────
// Damage plumbing
// ─────────────────────────────────────────────────────────────────────────────

fn roll_chance_byte(rng: &mut Rng, threshold: u8) -> bool {
    rng.byte() < threshold
}

fn pick<'v, T>(rng: &mut Rng, options: &'v [T]) -> &'v T {
    &options[rng.range(options.len() as u32) as usize]
}

fn note_hit(outcome: &mut MoveOutcome, res: HitRes, dmg: u16) {
    outcome.hit = true;
    outcome.damage_dealt = outcome.damage_dealt.saturating_add(dmg);
    if let HitRes::Sub { broke } = res {
        outcome.hit_sub = true;
        outcome.sub_broke |= broke;
    }
}

/// One standard damage application: compute (or reuse `precomputed`), apply,
/// track Counter/Bide bookkeeping, feed Rage. Invulnerability and immunity
/// are checked by the caller before the accuracy roll.
#[allow(clippy::too_many_arguments)]
fn damage_step(
    rng: &mut Rng,
    field: &mut Field,
    sides: &mut [Side; 2],
    attacker_side: usize,
    mv: &MoveEntry,
    selfdestruct: bool,
    precomputed: Option<u16>,
    log: &mut Log,
    mut outcome: MoveOutcome,
) -> MoveOutcome {
    let defender_side = 1 - attacker_side;

    let dmg = match precomputed {
        Some(d) => d,
        None => {
            let roll = {
                let a = sides[attacker_side].active();
                let d = sides[defender_side].active();
                compute_damage(rng, a, d, mv, selfdestruct)
            };
            if roll.dmg == 0 {
                let s = &sides[defender_side];
                if roll.immune {
                    log.push_board(format!("immune|mon:{},{},0", s.active().name, s.player_id));
                } else {
                    // Gen 1 "0 damage glitch": a damage roll of 0 is a miss.
                    let s = &sides[attacker_side];
                    log.push_board(format!("miss|mon:{},{},0", s.active().name, s.player_id));
                }
                outcome.hit = false;
                return outcome;
            }
            if roll.crit {
                let s = &sides[defender_side];
                log.push_board(format!("crit|mon:{},{},0", s.active().name, s.player_id));
            }
            // Type-effectiveness dialogue (first hit only for multi-hit).
            if roll.effectiveness > 100 {
                let s = &sides[defender_side];
                log.push_board(format!("supereffective|mon:{},{},0", s.active().name, s.player_id));
            } else if roll.effectiveness > 0 && roll.effectiveness < 100 {
                let s = &sides[defender_side];
                log.push_board(format!("resisted|mon:{},{},0", s.active().name, s.player_id));
            }
            outcome.crit = outcome.crit || roll.crit;
            roll.dmg
        }
    };

    let res = deal_damage(field, sides, defender_side, dmg, log);
    note_hit(&mut outcome, res, dmg);
    outcome.fainted_target = sides[defender_side].active().hp_cur == 0;

    // A raging target's Attack climbs every time it's hit by a damaging,
    // non-exploding move (explosions build Rage via the miss-or-hit rule).
    if !selfdestruct && !outcome.fainted_target {
        rage_build(sides, defender_side, log);
    }
    outcome
}

/// Builds Rage (+1 Atk) on `side`'s active if it's locked in Rage.
fn rage_build(sides: &mut [Side; 2], side: usize, log: &mut Log) {
    if !sides[side].active().volatile.has(Volatile::RAGE) {
        return;
    }
    if sides[side].active().fainted() {
        return;
    }
    // No foe re-stack: this path mirrors Showdown's direct boost() call.
    let _ = apply_stage_change(sides, side, side, 1, 1, false, log);
}

/// Apply damage to a side's active, routing through a Substitute if one is up.
/// Updates the battle-global last-damage register: uncapped when a Sub takes
/// it, capped to remaining HP otherwise (matching the cartridge registers).
fn deal_damage(
    field: &mut Field,
    sides: &mut [Side; 2],
    target_side: usize,
    dmg: u16,
    log: &mut Log,
) -> HitRes {
    if sides[target_side].active().volatile.has(Volatile::SUBSTITUTED) {
        field.last_damage = dmg;
        let sub_hp = sides[target_side].active().volatile.substitute_hp as u16;
        if dmg >= sub_hp {
            let m = sides[target_side].active_mut();
            m.volatile.clear(Volatile::SUBSTITUTED);
            m.volatile.substitute_hp = 0;
            let s = &sides[target_side];
            log.push_board(format!("end|mon:{},{},0|what:substitute", s.active().name, s.player_id));
            HitRes::Sub { broke: true }
        } else {
            sides[target_side].active_mut().volatile.substitute_hp = (sub_hp - dmg) as u8;
            HitRes::Sub { broke: false }
        }
    } else {
        let actual = dmg.min(sides[target_side].active().hp_cur);
        field.last_damage = actual;
        sides[target_side].active_mut().hp_cur -= actual;
        let s = &sides[target_side];
        let m = s.active();
        log.push_board(format!("damage|mon:{},{},0|health:{}/{}", m.name, s.player_id, m.hp_cur, m.hp_max));
        if m.hp_cur == 0 {
            log.push_board(format!("faint|mon:{},{},0", m.name, s.player_id));
        }
        HitRes::Mon
    }
}

/// HP loss that bypasses Substitutes (recoil, residual poison/burn/seed).
fn direct_hp_loss(
    field: &mut Field,
    sides: &mut [Side; 2],
    side: usize,
    dmg: u16,
    set_last_damage: bool,
    log: &mut Log,
) {
    let actual = dmg.min(sides[side].active().hp_cur);
    if set_last_damage {
        field.last_damage = actual;
    }
    sides[side].active_mut().hp_cur -= actual;
    let s = &sides[side];
    let m = s.active();
    log.push_board(format!("damage|mon:{},{},0|health:{}/{}", m.name, s.player_id, m.hp_cur, m.hp_max));
    if m.hp_cur == 0 {
        log.push_board(format!("faint|mon:{},{},0", m.name, s.player_id));
    }
}

/// Confusion self-hits and Jump Kick crash damage: sets the last-damage
/// register (they're counterable in Gen 1!), and if the damaged mon has a
/// Substitute the damage is misdirected — to the FOE's Substitute if one
/// exists, otherwise nobody takes it (the Substitute+confusion glitch).
fn self_hit_with_sub_redirect(
    field: &mut Field,
    sides: &mut [Side; 2],
    side: usize,
    dmg: u16,
    log: &mut Log,
) {
    field.last_damage = dmg;
    if sides[side].active().volatile.has(Volatile::SUBSTITUTED) {
        let foe = 1 - side;
        if sides[foe].active().volatile.has(Volatile::SUBSTITUTED) {
            let sub_hp = sides[foe].active().volatile.substitute_hp as u16;
            if dmg >= sub_hp {
                let m = sides[foe].active_mut();
                m.volatile.clear(Volatile::SUBSTITUTED);
                m.volatile.substitute_hp = 0;
                let s = &sides[foe];
                log.push_board(format!("end|mon:{},{},0|what:substitute", s.active().name, s.player_id));
            } else {
                sides[foe].active_mut().volatile.substitute_hp = (sub_hp - dmg) as u8;
            }
        }
        // No sub on the foe: the damage vanishes entirely.
    } else {
        direct_hp_loss(field, sides, side, dmg, false, log);
    }
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

/// Gen 1 recovery bug: Recover/Softboiled/Rest fail when the HP deficit is
/// exactly 255 or 511 — unless current HP is divisible by 256.
fn recovery_fails(cur: u16, max: u16) -> bool {
    if cur == max {
        return true;
    }
    let deficit = max - cur;
    (deficit == 255 || deficit == 511) && cur % 256 != 0
}

fn fail_log(sides: &[Side; 2], side: usize, log: &mut Log) {
    let s = &sides[side];
    log.push_board(format!("fail|mon:{},{},0", s.active().name, s.player_id));
}

// ─────────────────────────────────────────────────────────────────────────────
// Status / stages
// ─────────────────────────────────────────────────────────────────────────────

/// Sleep Clause Mod (format rule, Gen 1 randbats): an enemy sleep move fails
/// while the target's side already has a mon sleeping from an enemy move.
/// Rest-sleep doesn't count.
fn sleep_clause_blocks(side: &Side) -> bool {
    side.team.iter().any(|m| {
        !m.empty() && m.hp_cur > 0 && matches!(m.status, Status::Sleep(_)) && !m.rest_sleep
    })
}

/// Freeze Clause Mod (format rule): a move can't freeze while the target's
/// side already has a frozen mon.
fn freeze_clause_blocks(side: &Side) -> bool {
    side.team.iter().any(|m| !m.empty() && m.hp_cur > 0 && m.status == Status::Freeze)
}

/// Try to inflict a major status. Applies the Gen 1 sticky stat drops on
/// success (burn halves Atk, paralysis quarters Spe — onto modified stats).
/// Returns false if the target already has a status, is immune, or a format
/// clause (Sleep/Freeze Clause Mod) blocks it. Only ever called for
/// ENEMY-inflicted status; Rest sets its own sleep directly.
fn try_apply_status(
    sides: &mut [Side; 2],
    side: usize,
    status: Status,
    _primary: bool,
    log: &mut Log,
) -> bool {
    if sides[side].active().hp_cur == 0 {
        return false;
    }
    if !matches!(sides[side].active().status, Status::None) {
        return false;
    }
    if matches!(status, Status::None) {
        return false;
    }
    // Poison-types can't be poisoned.
    if matches!(status, Status::Poison | Status::BadPoison)
        && sides[side].active().is_type(Type::Poison)
    {
        return false;
    }
    // Format rules (always on: this engine only plays Gen 1 randbats).
    if matches!(status, Status::Sleep(_)) && sleep_clause_blocks(&sides[side]) {
        return false;
    }
    if status == Status::Freeze && freeze_clause_blocks(&sides[side]) {
        return false;
    }
    {
        let m = sides[side].active_mut();
        m.status = status;
        if matches!(status, Status::Sleep(_)) {
            m.rest_sleep = false;
        }
        apply_status_drop(m);
        if status == Status::BadPoison && !m.volatile.has(Volatile::TOX_COUNTER) {
            // Toxic counter starts at 0 and survives Rest; if one is already
            // running (re-toxiced after Rest), it keeps escalating.
            m.volatile.set(Volatile::TOX_COUNTER);
            m.volatile.toxic_counter = 0;
        }
    }
    let status_str = match status {
        Status::Poison => "psn",
        Status::Burn => "brn",
        Status::Freeze => "frz",
        Status::Paralysis => "par",
        Status::Sleep(_) => "slp",
        Status::BadPoison => "tox",
        Status::None => "",
    };
    let s = &sides[side];
    let m = s.active();
    log.push_board(format!("status|mon:{},{},0|status:{}", m.name, s.player_id, status_str));
    true
}

/// Inflict the confusion volatile (Gen 1: 1-4 attacking turns after this one).
/// Silently does nothing if the target is already confused.
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

/// Gen 1 stat stage change. Recalculates the modified stat from scratch —
/// ERASING any paralysis/burn drop on it (the stat modification glitch) —
/// then, on success, re-stacks the status drop of the move USER's foe.
/// Handles the boost-at-999 / drop-at-1 edge quirk. Returns success.
pub fn apply_stage_change(
    sides: &mut [Side; 2],
    target_side: usize,
    user_side: usize,
    stat_param: u8,
    delta: i8,
    restack: bool,
    log: &mut Log,
) -> bool {
    let idx = stat_idx_from_param(stat_param);
    let changed = {
        let m = sides[target_side].active_mut();
        if idx >= 4 {
            // Accuracy/evasion: pure stage, no modified stat behind it.
            let new = (m.stages[idx] as i32 + delta as i32).clamp(-6, 6) as i8;
            if new == m.stages[idx] {
                false
            } else {
                m.stages[idx] = new;
                true
            }
        } else {
            let stat_i = idx + 1;
            if delta > 0 && m.modified[stat_i] == 999 {
                // Boosting a stat at 999: the stage rises then drops one, the
                // stat stays 999, and the move counts as failed (no re-stack).
                let s = (m.stages[idx] as i32 + delta as i32).clamp(-6, 6) - 1;
                m.stages[idx] = s.clamp(-6, 6) as i8;
                false
            } else if delta < 0 && m.modified[stat_i] == 1 {
                let s = (m.stages[idx] as i32 + delta as i32).clamp(-6, 6) + 1;
                m.stages[idx] = s.clamp(-6, 6) as i8;
                false
            } else {
                let new = (m.stages[idx] as i32 + delta as i32).clamp(-6, 6) as i8;
                if new == m.stages[idx] {
                    false
                } else {
                    m.stages[idx] = new;
                    recalc_modified(m, stat_i);
                    true
                }
            }
        }
    };
    if !changed {
        return false;
    }
    {
        let s = &sides[target_side];
        log.push_board(format!(
            "boost|mon:{},{},0|stat:{}|delta:{}",
            s.active().name, s.player_id, stat_name_from_param(stat_param), delta
        ));
    }
    if restack {
        // Gen 1: whenever a stat-affecting move lands, the non-acting side's
        // par/brn drop is applied AGAIN (compounding /4 speed, /2 attack).
        let foe = 1 - user_side;
        apply_status_drop(sides[foe].active_mut());
    }
    true
}

// ─────────────────────────────────────────────────────────────────────────────
// Switching
// ─────────────────────────────────────────────────────────────────────────────

/// Gen 1 switch-out semantics for the mon LEAVING the field: volatile state
/// and stat stages reset, modified stats restored, Toxic downgrades to regular
/// poison, and Transform reverts to the pre-transform snapshot.
pub fn reset_on_switch_out(s: &mut Side) {
    if s.active().volatile.has(Volatile::TRANSFORMED) {
        if let Some(b) = s.transform_backup.take() {
            let m = s.active_mut();
            m.species_id = b.species_id;
            m.primary_type = b.primary_type;
            m.secondary_type = b.secondary_type;
            m.stats = b.stats;
            m.moves = b.moves;
        }
    }
    s.transform_backup = None;
    let m = s.active_mut();
    m.volatile = Volatile::default();
    m.stages = [0; 6];
    m.modified = m.stats;
    if m.status == Status::BadPoison {
        m.status = Status::Poison;
    }
}

/// Gen 1 switch-in semantics for the mon ENTERING the field: the paralysis/
/// burn stat drop is applied (again) onto fresh modified stats, and a
/// poisoned/burned mon takes an immediate 1/16 residual (flat — the toxic
/// counter died with the switch).
pub fn after_switch_in(field: &mut Field, sides: &mut [Side; 2], side: usize, log: &mut Log) {
    apply_status_drop(sides[side].active_mut());
    if matches!(sides[side].active().status, Status::Poison | Status::Burn) {
        let max = sides[side].active().hp_max;
        direct_hp_loss(field, sides, side, (max / 16).max(1), true, log);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Residual damage (after each side's own action — NOT end of turn)
// ─────────────────────────────────────────────────────────────────────────────

/// Gen 1 applies residual damage right after the afflicted mon's own action
/// (even one spent asleep/trapped/paralyzed). Order: brn/psn/tox, then Leech
/// Seed. All of it updates the last-damage register — Counter can counter
/// poison ticks.
pub fn after_action_residuals(field: &mut Field, sides: &mut [Side; 2], side: usize, log: &mut Log) {
    if sides[side].active().hp_cur == 0 || sides[side].active().empty() {
        return;
    }
    let max = sides[side].active().hp_max;
    let base = (max / 16).max(1);

    match sides[side].active().status {
        Status::Poison | Status::Burn => {
            // Multiplied by a live toxic counter (kept by Rest), NOT incremented.
            let mult = if sides[side].active().volatile.has(Volatile::TOX_COUNTER) {
                sides[side].active().volatile.toxic_counter.max(1) as u16
            } else {
                1
            };
            direct_hp_loss(field, sides, side, base.saturating_mul(mult), true, log);
        }
        Status::BadPoison => {
            let m = sides[side].active_mut();
            m.volatile.set(Volatile::TOX_COUNTER);
            m.volatile.toxic_counter = m.volatile.toxic_counter.saturating_add(1);
            let mult = m.volatile.toxic_counter as u16;
            direct_hp_loss(field, sides, side, base.saturating_mul(mult), true, log);
        }
        _ => {}
    }

    if sides[side].active().hp_cur > 0 && sides[side].active().volatile.has(Volatile::LEECH_SEEDED) {
        // Leech Seed shares — and INCREMENTS — the toxic counter (Gen 1).
        let mult = if sides[side].active().volatile.has(Volatile::TOX_COUNTER) {
            let m = sides[side].active_mut();
            m.volatile.toxic_counter = m.volatile.toxic_counter.saturating_add(1);
            m.volatile.toxic_counter as u16
        } else {
            1
        };
        let drain = base.saturating_mul(mult);
        direct_hp_loss(field, sides, side, drain, true, log);
        // The seeder heals the full drain amount, not capped by the victim's
        // remaining HP (Gen 1 quirk).
        let healer = 1 - side;
        if !sides[healer].active().fainted() && !sides[healer].active().empty() {
            heal_mon(sides, healer, drain, log);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Pre-move status checks (cartridge order): returns true if the mon can act.
// ─────────────────────────────────────────────────────────────────────────────

/// `chosen_slot` is the move slot about to be used (for the Disable check).
pub fn pre_move_check(
    rng: &mut Rng,
    field: &mut Field,
    sides: &mut [Side; 2],
    side: usize,
    chosen_slot: Option<u8>,
    log: &mut Log,
) -> bool {
    // Haze cured this mon's sleep/freeze before it acted: turn forfeited.
    if sides[side].active().volatile.has(Volatile::SKIP_TURN) {
        sides[side].active_mut().volatile.clear(Volatile::SKIP_TURN);
        let s = &sides[side];
        log.push_board(format!("cant|mon:{},{},0|from:cannotmove", s.active().name, s.player_id));
        return false;
    }

    // 1. Freeze — permanent, checked before recharge (which is why a frozen
    //    Hyper Beam user stays stuck recharging: the flag is never consumed).
    if sides[side].active().status == Status::Freeze {
        sides[side].active_mut().last_move_used = "";
        let s = &sides[side];
        log.push_board(format!("cant|mon:{},{},0|from:frz", s.active().name, s.player_id));
        return false;
    }

    // 2. Sleep — decrement; hitting 0 wakes but the wake turn is still lost.
    if let Status::Sleep(t) = sides[side].active().status {
        let t = t.saturating_sub(1);
        if t == 0 {
            sides[side].active_mut().status = Status::None;
            let s = &sides[side];
            log.push_board(format!("curestatus|mon:{},{},0|status:slp", s.active().name, s.player_id));
        } else {
            sides[side].active_mut().status = Status::Sleep(t);
            let s = &sides[side];
            log.push_board(format!("cant|mon:{},{},0|from:slp", s.active().name, s.player_id));
        }
        sides[side].active_mut().last_move_used = "";
        return false; // Gen 1: lose the wake turn.
    }

    // 3. Partial trap (Wrap/Bind/Fire Spin/Clamp from the opponent).
    if sides[side].active().volatile.has(Volatile::TRAPPED) {
        let s = &sides[side];
        log.push_board(format!("cant|mon:{},{},0|from:trapped", s.active().name, s.player_id));
        return false;
    }

    // 4. Flinch.
    if sides[side].active().volatile.has(Volatile::FLINCHED) {
        sides[side].active_mut().volatile.clear(Volatile::FLINCHED);
        let s = &sides[side];
        log.push_board(format!("cant|mon:{},{},0|from:flinch", s.active().name, s.player_id));
        return false;
    }

    // 5. Hyper Beam recharge.
    if sides[side].active().volatile.has(Volatile::MUST_RECHARGE) {
        sides[side].active_mut().volatile.clear(Volatile::MUST_RECHARGE);
        let s = &sides[side];
        log.push_board(format!("cant|mon:{},{},0|from:recharge", s.active().name, s.player_id));
        return false;
    }

    // 6. Disable countdown + blocked-move check.
    if sides[side].active().volatile.has(Volatile::DISABLED) {
        let expired = {
            let m = sides[side].active_mut();
            m.volatile.disabled_turns = m.volatile.disabled_turns.saturating_sub(1);
            m.volatile.disabled_turns == 0
        };
        if expired {
            let m = sides[side].active_mut();
            m.volatile.clear(Volatile::DISABLED);
            m.volatile.disabled_slot = 0;
            let s = &sides[side];
            log.push_board(format!("end|mon:{},{},0|what:disable", s.active().name, s.player_id));
        } else if chosen_slot == Some(sides[side].active().volatile.disabled_slot) {
            {
                let s = &sides[side];
                log.push_board(format!("cant|mon:{},{},0|from:disabled", s.active().name, s.player_id));
            }
            // Being Disabled out of a two-turn move drops the charge lock
            // (but not the invulnerability — same family as paralysis).
            let m = sides[side].active_mut();
            m.volatile.clear(Volatile::CHARGING);
            if !m.volatile.has(Volatile::RAGE) {
                m.volatile.multi_turn_move = "";
                m.volatile.multi_turn_turns = 0;
            }
            return false;
        }
    }

    // 7. Confusion (checked BEFORE paralysis in Gen 1).
    if sides[side].active().volatile.has(Volatile::CONFUSED) {
        if sides[side].active().volatile.confused_turns == 0 {
            sides[side].active_mut().volatile.clear(Volatile::CONFUSED);
            let s = &sides[side];
            log.push_board(format!("end|mon:{},{},0|what:confusion", s.active().name, s.player_id));
        } else {
            sides[side].active_mut().volatile.confused_turns -= 1;
            if rng.byte() < 128 {
                {
                    let s = &sides[side];
                    log.push_board(format!("cant|mon:{},{},0|from:confusion", s.active().name, s.player_id));
                }
                // 40-power typeless self-hit off the MODIFIED stats: no crit,
                // no random factor, counterable, and misdirected by subs.
                let dmg = {
                    let m = sides[side].active();
                    let lvl = m.level as u32;
                    let atk = m.modified[1] as u32;
                    let def = (m.modified[2] as u32).max(1);
                    (((2 * lvl / 5 + 2) * 40 * atk) / def / 50 + 2) as u16
                };
                self_hit_with_sub_redirect(field, sides, side, dmg, log);
                // A confusion self-hit cancels Bide, charge state AND the
                // invulnerability (unlike full paralysis), trap locks, and
                // Thrash — Rage survives.
                let m = sides[side].active_mut();
                m.volatile.clear(Volatile::BIDING | Volatile::CHARGING | Volatile::INVULNERABLE);
                m.volatile.stored_damage = 0;
                m.volatile.bide_turns = 0;
                if !m.volatile.has(Volatile::RAGE) {
                    m.volatile.multi_turn_move = "";
                    m.volatile.multi_turn_turns = 0;
                    m.volatile.locked_acc = 0;
                }
                return false;
            }
        }
    }

    // 8. Full paralysis: 63/256. Cancels Bide, charge and trap locks, and
    //    Thrash — but the Fly/Dig INVULNERABILITY stays stuck until the mon
    //    switches or completes the move (the Gen 1 invulnerability glitch).
    if sides[side].active().status == Status::Paralysis && rng.byte() < 63 {
        {
            let s = &sides[side];
            log.push_board(format!("cant|mon:{},{},0|from:par", s.active().name, s.player_id));
        }
        let m = sides[side].active_mut();
        m.volatile.clear(Volatile::BIDING | Volatile::CHARGING);
        m.volatile.stored_damage = 0;
        m.volatile.bide_turns = 0;
        if !m.volatile.has(Volatile::RAGE) {
            m.volatile.multi_turn_move = "";
            m.volatile.multi_turn_turns = 0;
            m.volatile.locked_acc = 0;
        }
        return false;
    }

    true
}

/// Returns Some(move_id) if the mon must repeat a specific move this turn
/// (TwoTurn release, Bide, Wrap continuation, Thrash, Rage).
pub fn locked_move_id(side: &Side) -> Option<&'static str> {
    let m = side.active();
    if !m.volatile.multi_turn_move.is_empty() {
        return Some(m.volatile.multi_turn_move);
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
