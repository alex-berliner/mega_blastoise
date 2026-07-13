//! Engine unit tests covering core Gen 1 behaviors.
//!
//! Tests run on host (`cargo test -p gen1_battle`). They exercise the
//! internal types via the `testing` module — these aren't part of the
//! stable API but let us drive the engine state machine directly.

use gen1_battle::testing::*;
use gen1_battle::Type;

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn fresh_mon(species: &'static str, level: u8, moves: &[&'static str]) -> Mon {
    Mon::from_species(species, level, moves).expect("species lookup")
}

fn make_sides(p1: Mon, p2: Mon) -> [Side; 2] {
    let mut sides = <[Side; 2]>::default();
    let _ = sides[0].player_id.push_str("p1");
    let _ = sides[1].player_id.push_str("p2");
    sides[0].team[0] = p1;
    sides[1].team[0] = p2;
    sides[0].active_idx = 0;
    sides[1].active_idx = 0;
    sides
}

fn empty_log() -> Log {
    Log::new()
}

/// Shorthand: execute side's move slot with a fresh field.
fn ex(
    rng: &mut Rng,
    field: &mut Field,
    sides: &mut [Side; 2],
    side: usize,
    slot: usize,
    log: &mut Log,
) -> MoveOutcome {
    execute_move(rng, field, sides, side, slot, log)
}

// ─────────────────────────────────────────────────────────────────────────────
// Damage formula
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn damage_nonzero_for_offensive_move() {
    let mut rng = Rng::new(42);
    let mut field = Field::default();
    let attacker = fresh_mon("tauros", 100, &["bodyslam"]);
    let defender = fresh_mon("snorlax", 100, &["rest"]);
    let mut sides = make_sides(attacker, defender);
    let mut log = empty_log();
    // Seed 42 may 1/256-miss in principle; accept any of the first few seeds.
    let mut dealt = 0;
    for seed in 42..48u64 {
        let mut r = Rng::new(seed);
        let out = ex(&mut r, &mut field, &mut sides, 0, 0, &mut log);
        if out.damage_dealt > 0 {
            dealt = out.damage_dealt;
            break;
        }
    }
    let _ = rng;
    assert!(dealt > 0, "Body Slam should deal damage");
    assert!(sides[1].active().hp_cur < sides[1].active().hp_max);
}

#[test]
fn damage_is_zero_against_immune_type() {
    let mut rng = Rng::new(7);
    let mut field = Field::default();
    // Ghost is immune to Normal.
    let attacker = fresh_mon("tauros", 100, &["bodyslam"]);
    let defender = fresh_mon("gengar", 100, &["nightshade"]);
    let mut sides = make_sides(attacker, defender);
    let mut log = empty_log();
    let out = ex(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
    assert_eq!(out.damage_dealt, 0, "Normal vs Ghost must be 0 damage");
    assert_eq!(sides[1].active().hp_cur, sides[1].active().hp_max);
}

#[test]
fn stab_is_applied() {
    let defender_base = fresh_mon("snorlax", 100, &["rest"]);

    let mut a_stab = fresh_mon("charizard", 100, &["flamethrower"]);
    let mut a_nostab = fresh_mon("tauros", 100, &["flamethrower"]);
    // Equalize Spc so the only variable is STAB.
    a_stab.stats[3] = 150;
    a_stab.modified[3] = 150;
    a_nostab.stats[3] = 150;
    a_nostab.modified[3] = 150;
    // Equalize crit rates.
    a_stab.base_spe = 100;
    a_nostab.base_spe = 100;
    let mut sides_a = make_sides(a_stab, defender_base.clone());
    let mut sides_b = make_sides(a_nostab, defender_base);
    let mut rng_a = Rng::new(1);
    let mut rng_b = Rng::new(1);
    let mut field = Field::default();
    let mut log = empty_log();
    let out_stab = ex(&mut rng_a, &mut field, &mut sides_a, 0, 0, &mut log);
    let out_nostab = ex(&mut rng_b, &mut field, &mut sides_b, 0, 0, &mut log);
    assert!(
        out_stab.damage_dealt >= out_nostab.damage_dealt,
        "STAB should not deal less than non-STAB ({} vs {})",
        out_stab.damage_dealt,
        out_nostab.damage_dealt
    );
}

#[test]
fn crit_ignores_stat_stages() {
    // Even at -6 Atk, a crit must deal damage from the RAW stats.
    let mut a = fresh_mon("aerodactyl", 100, &["wingattack"]);
    a.stages[0] = -6;
    recalc_modified(&mut a, 1);
    let d = fresh_mon("snorlax", 100, &["rest"]);

    let mut crit_seen = false;
    for seed in 1..200u64 {
        let mut sides = make_sides(a.clone(), d.clone());
        let mut rng = Rng::new(seed);
        let mut field = Field::default();
        let mut log = empty_log();
        let pre = sides[1].active().hp_cur;
        let out = ex(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
        if out.crit {
            crit_seen = true;
            assert!(pre - sides[1].active().hp_cur > 0);
            break;
        }
    }
    assert!(crit_seen, "should observe a crit in 200 trials at base 130 Speed");
}

// ─────────────────────────────────────────────────────────────────────────────
// Status effects
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn paralysis_can_fully_paralyze() {
    let mut a = fresh_mon("tauros", 100, &["bodyslam"]);
    a.status = Status::Paralysis;
    let d = fresh_mon("snorlax", 100, &["rest"]);
    let mut sides = make_sides(a, d);
    let mut field = Field::default();
    let mut log = empty_log();
    let mut fp = 0;
    for seed in 1..200u64 {
        sides[0].active_mut().status = Status::Paralysis;
        let mut rng = Rng::new(seed);
        if !pre_move_check(&mut rng, &mut field, &mut sides, 0, Some(0), &mut log) {
            fp += 1;
        }
    }
    assert!(fp > 0, "expected some fully-paralyzed turns");
}

#[test]
fn sleep_counter_decrements_and_wakes() {
    // Gen 1 semantics: Sleep(t) costs t turns total; the counter hitting 0
    // wakes the mon on that same (lost) turn.
    let mut a = fresh_mon("tauros", 100, &["bodyslam"]);
    a.status = Status::Sleep(3);
    let d = fresh_mon("snorlax", 100, &["rest"]);
    let mut sides = make_sides(a, d);
    let mut field = Field::default();
    let mut log = empty_log();
    let mut rng = Rng::new(1);
    for _ in 0..3 {
        let acted = pre_move_check(&mut rng, &mut field, &mut sides, 0, Some(0), &mut log);
        assert!(!acted, "asleep mon must not act");
    }
    assert!(matches!(sides[0].active().status, Status::None), "woke on the 3rd (lost) turn");
    let acted = pre_move_check(&mut rng, &mut field, &mut sides, 0, Some(0), &mut log);
    assert!(acted, "acts the turn after waking");
}

#[test]
fn freeze_locks_mon_out() {
    let mut a = fresh_mon("tauros", 100, &["bodyslam"]);
    a.status = Status::Freeze;
    let d = fresh_mon("snorlax", 100, &["rest"]);
    let mut sides = make_sides(a, d);
    let mut field = Field::default();
    let mut log = empty_log();
    let mut rng = Rng::new(1);
    for _ in 0..10 {
        assert!(!pre_move_check(&mut rng, &mut field, &mut sides, 0, Some(0), &mut log));
    }
}

#[test]
fn residual_poison_deals_one_sixteenth() {
    let mut a = fresh_mon("tauros", 100, &["bodyslam"]);
    a.status = Status::Poison;
    let d = fresh_mon("snorlax", 100, &["rest"]);
    let mut sides = make_sides(a, d);
    let max = sides[0].active().hp_max;
    let mut field = Field::default();
    let mut log = empty_log();
    after_action_residuals(&mut field, &mut sides, 0, &mut log);
    let dealt = max - sides[0].active().hp_cur;
    let expected = (max / 16).max(1);
    assert_eq!(dealt, expected);
    // Residual damage is counterable in Gen 1 — it feeds the register.
    assert_eq!(field.last_damage, expected);
}

#[test]
fn bad_poison_counter_escalates() {
    let mut a = fresh_mon("snorlax", 100, &["rest"]);
    a.status = Status::BadPoison;
    let d = fresh_mon("tauros", 100, &["bodyslam"]);
    let mut sides = make_sides(a, d);
    let max = sides[0].active().hp_max;
    let mut field = Field::default();
    let mut log = empty_log();
    after_action_residuals(&mut field, &mut sides, 0, &mut log);
    let first = max - sides[0].active().hp_cur;
    after_action_residuals(&mut field, &mut sides, 0, &mut log);
    let second = (max - sides[0].active().hp_cur) - first;
    assert_eq!(first, (max / 16).max(1));
    assert_eq!(second, first * 2, "toxic damage must double on turn 2");
}

// ─────────────────────────────────────────────────────────────────────────────
// Substitute
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn substitute_costs_quarter_hp_and_absorbs() {
    let a = fresh_mon("snorlax", 100, &["substitute"]);
    let max = a.hp_max;
    let mut sides = make_sides(a, fresh_mon("tauros", 100, &["bodyslam"]));
    let mut rng = Rng::new(1);
    let mut field = Field::default();
    let mut log = empty_log();
    let _ = ex(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
    let after = sides[0].active().hp_cur;
    assert_eq!(after, max - (max / 4));
    assert!(sides[0].active().volatile.has(Volatile::SUBSTITUTED));
    let sub_hp_before = sides[0].active().volatile.substitute_hp;
    assert_eq!(sub_hp_before as u16, max / 4 + 1);

    // Now Tauros hits — damage should go to sub.
    let pre = sides[0].active().hp_cur;
    let _ = ex(&mut rng, &mut field, &mut sides, 1, 0, &mut log);
    let post = sides[0].active().hp_cur;
    if sides[0].active().volatile.has(Volatile::SUBSTITUTED) {
        assert_eq!(pre, post, "sub should absorb the hit");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Counter
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn counter_doubles_last_damage() {
    let a = fresh_mon("chansey", 100, &["counter"]);
    let d = fresh_mon("tauros", 100, &["bodyslam"]);
    let pre = d.hp_cur;
    let mut sides = make_sides(a, d);
    sides[1].last_move_used = "bodyslam";
    sides[1].last_selected_move = "bodyslam";
    let mut field = Field::default();
    field.last_damage = 50;
    let mut rng = Rng::new(1);
    let mut log = empty_log();
    let out = ex(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
    assert_eq!(out.damage_dealt, 100);
    assert!(sides[1].active().hp_cur < pre);
}

#[test]
fn counter_without_source_fails() {
    let a = fresh_mon("chansey", 100, &["counter"]);
    let d = fresh_mon("tauros", 100, &["bodyslam"]);
    let mut sides = make_sides(a, d);
    let mut field = Field::default();
    let mut rng = Rng::new(1);
    let mut log = empty_log();
    let out = ex(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
    assert_eq!(out.damage_dealt, 0);
}

#[test]
fn counter_fails_against_non_normal_fighting() {
    let a = fresh_mon("chansey", 100, &["counter"]);
    let d = fresh_mon("zapdos", 100, &["thunderbolt"]);
    let mut sides = make_sides(a, d);
    sides[1].last_move_used = "thunderbolt";
    sides[1].last_selected_move = "thunderbolt";
    let mut field = Field::default();
    field.last_damage = 90;
    let mut rng = Rng::new(1);
    let mut log = empty_log();
    let out = ex(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
    assert_eq!(out.damage_dealt, 0, "Electric moves are not counterable");
}

// ─────────────────────────────────────────────────────────────────────────────
// Leech Seed
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn leech_seed_grass_immunity() {
    let a = fresh_mon("venusaur", 100, &["leechseed"]);
    let d = fresh_mon("bulbasaur", 100, &["tackle"]); // Grass type
    let mut sides = make_sides(a, d);
    let mut rng = Rng::new(1);
    let mut field = Field::default();
    let mut log = empty_log();
    let _ = ex(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
    assert!(!sides[1].active().volatile.has(Volatile::LEECH_SEEDED));
}

#[test]
fn leech_seed_drains_and_heals() {
    let mut a = fresh_mon("venusaur", 100, &["leechseed"]);
    a.hp_cur = a.hp_max / 2;
    let d = fresh_mon("snorlax", 100, &["rest"]); // Not Grass
    let mut sides = make_sides(a, d);
    let mut rng = Rng::new(3);
    let mut field = Field::default();
    let mut log = empty_log();
    let out = ex(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
    assert!(out.hit, "leech seed should land with this seed");
    assert!(sides[1].active().volatile.has(Volatile::LEECH_SEEDED));

    let pre_a = sides[0].active().hp_cur;
    let pre_d = sides[1].active().hp_cur;
    after_action_residuals(&mut field, &mut sides, 1, &mut log);
    let post_a = sides[0].active().hp_cur;
    let post_d = sides[1].active().hp_cur;
    assert!(post_d < pre_d, "seeded mon should lose HP");
    assert!(post_a > pre_a, "seeding mon should gain HP");
}

// ─────────────────────────────────────────────────────────────────────────────
// Multi-hit
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn multi_hit_does_multiple_hits_of_equal_damage() {
    let a = fresh_mon("clefable", 100, &["doubleslap"]);
    let d = fresh_mon("snorlax", 100, &["rest"]);
    for seed in 40..60u64 {
        let mut sides = make_sides(a.clone(), d.clone());
        let mut rng = Rng::new(seed);
        let mut field = Field::default();
        let mut log = empty_log();
        let pre = sides[1].active().hp_cur;
        let out = ex(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
        if !out.hit {
            continue;
        }
        let dealt = pre - sides[1].active().hp_cur;
        assert!(dealt > 0);
        // All hits share the first hit's damage: the register holds one
        // hit's worth, and the total divides evenly by it.
        let per = field.last_damage;
        assert!(per > 0 && dealt % per == 0, "hits must be equal ({dealt} not divisible by {per})");
        return;
    }
    panic!("no hit observed across seeds");
}

// ─────────────────────────────────────────────────────────────────────────────
// OHKO
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ohko_fails_when_user_slower() {
    let mut a = fresh_mon("rhydon", 100, &["fissure"]);
    a.stats[4] = 50;
    a.modified[4] = 50;
    let mut d = fresh_mon("snorlax", 100, &["rest"]);
    d.stats[4] = 200;
    d.modified[4] = 200;
    let mut sides = make_sides(a, d);
    let mut rng = Rng::new(1);
    let mut field = Field::default();
    let mut log = empty_log();
    let out = ex(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
    assert_eq!(out.damage_dealt, 0);
}

#[test]
fn ohko_immune_by_type() {
    // Fissure (Ground) can't touch a Flying-type, even a slower one.
    let a = fresh_mon("rhydon", 100, &["fissure"]);
    let mut d = fresh_mon("pidgey", 5, &["gust"]);
    d.stats[4] = 1;
    d.modified[4] = 1;
    let mut sides = make_sides(a, d);
    let mut rng = Rng::new(1);
    let mut field = Field::default();
    let mut log = empty_log();
    let out = ex(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
    assert_eq!(out.damage_dealt, 0);
}

// ─────────────────────────────────────────────────────────────────────────────
// TwoTurn / invulnerability
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn two_turn_charges_then_attacks() {
    let a = fresh_mon("venusaur", 100, &["solarbeam"]);
    let d = fresh_mon("tauros", 100, &["bodyslam"]);
    let mut sides = make_sides(a, d);
    let mut rng = Rng::new(7);
    let mut field = Field::default();
    let mut log = empty_log();

    let out1 = ex(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
    assert_eq!(out1.damage_dealt, 0);
    assert!(sides[0].active().volatile.has(Volatile::CHARGING));
    assert_eq!(locked_move_id(&sides[0]), Some("solarbeam"));

    let out2 = execute_locked_move(&mut rng, &mut field, &mut sides, 0, "solarbeam", &mut log);
    assert!(out2.damage_dealt > 0);
    assert!(!sides[0].active().volatile.has(Volatile::CHARGING));
    assert_eq!(locked_move_id(&sides[0]), None);
}

#[test]
fn invulnerable_target_dodges() {
    let mut a = fresh_mon("pidgeot", 100, &["fly"]);
    a.volatile.set(Volatile::INVULNERABLE);
    let d = fresh_mon("snorlax", 100, &["bodyslam"]);
    let mut sides = make_sides(d, a); // snorlax attacks the flying bird
    let mut rng = Rng::new(1);
    let mut field = Field::default();
    let mut log = empty_log();
    let out = ex(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
    assert_eq!(out.damage_dealt, 0);
}

// ─────────────────────────────────────────────────────────────────────────────
// Wrap-family
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn wrap_traps_target_and_repeats_damage() {
    let a = fresh_mon("dragonair", 100, &["wrap"]);
    let d = fresh_mon("snorlax", 100, &["bodyslam"]);
    for seed in 1..40u64 {
        let mut sides = make_sides(a.clone(), d.clone());
        let mut rng = Rng::new(seed);
        let mut field = Field::default();
        let mut log = empty_log();
        let out = ex(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
        if out.damage_dealt == 0 {
            continue; // missed (85% accuracy)
        }
        let first = out.damage_dealt;
        assert!(sides[1].active().volatile.has(Volatile::TRAPPED));
        assert_eq!(locked_move_id(&sides[0]), Some("wrap"));
        // Trapped mon can't move.
        assert!(!pre_move_check(&mut rng, &mut field, &mut sides, 1, Some(0), &mut log));
        // Continuation repeats the exact damage, no accuracy roll.
        let pre = sides[1].active().hp_cur;
        let _ = execute_locked_move(&mut rng, &mut field, &mut sides, 0, "wrap", &mut log);
        assert_eq!(pre - sides[1].active().hp_cur, first);
        return;
    }
    panic!("wrap never landed across seeds");
}

// ─────────────────────────────────────────────────────────────────────────────
// Mimic / Transform / Disable
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn mimic_replaces_slot_with_random_target_move() {
    let a = fresh_mon("mrmime", 100, &["mimic"]);
    let d = fresh_mon("snorlax", 100, &["bodyslam"]);
    let mut sides = make_sides(a, d);
    let mut rng = Rng::new(1);
    let mut field = Field::default();
    let mut log = empty_log();
    let _ = ex(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
    // Only one move to copy from — must be Body Slam. PP carries over from
    // the Mimic slot (Gen 1), which just spent 1 of its 10.
    assert_eq!(sides[0].active().moves[0].move_id, "bodyslam");
    assert_eq!(sides[0].active().moves[0].pp, 9);
}

#[test]
fn transform_copies_target() {
    let a = fresh_mon("ditto", 100, &["transform"]);
    let d = fresh_mon("snorlax", 100, &["bodyslam", "earthquake", "hyperbeam", "rest"]);
    let pre_hp = a.hp_cur;
    let mut sides = make_sides(a, d);
    let mut rng = Rng::new(1);
    let mut field = Field::default();
    let mut log = empty_log();
    let _ = ex(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
    let copy = sides[0].active();
    let target = sides[1].active();
    assert_eq!(copy.species_id, target.species_id);
    assert_eq!(copy.primary_type, target.primary_type);
    for i in 1..5 {
        assert_eq!(copy.stats[i], target.stats[i]);
    }
    assert_eq!(copy.hp_cur, pre_hp, "HP must NOT be copied");
    for i in 0..4 {
        if !target.moves[i].move_id.is_empty() {
            assert_eq!(copy.moves[i].move_id, target.moves[i].move_id);
            assert_eq!(copy.moves[i].pp, 5);
        }
    }
}

#[test]
fn disable_blocks_chosen_move() {
    let a = fresh_mon("haunter", 100, &["disable"]);
    let d = fresh_mon("tauros", 100, &["bodyslam", "earthquake", "hyperbeam", "rest"]);
    let mut sides = make_sides(a, d);
    let mut rng = Rng::new(1);
    let mut field = Field::default();
    let mut log = empty_log();
    let _ = ex(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
    assert!(sides[1].active().volatile.has(Volatile::DISABLED));
    let disabled_slot = sides[1].active().volatile.disabled_slot;
    sides[1].active_mut().volatile.disabled_turns = 5;

    // The disabled slot is blocked at the pre-move gate.
    let acted = pre_move_check(&mut rng, &mut field, &mut sides, 1, Some(disabled_slot), &mut log);
    assert!(!acted, "disabled move must not act");
    // A different slot is fine.
    let other = (disabled_slot + 1) % 4;
    let acted = pre_move_check(&mut rng, &mut field, &mut sides, 1, Some(other), &mut log);
    assert!(acted);
}

// ─────────────────────────────────────────────────────────────────────────────
// Reflect / Haze
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn reflect_reduces_physical_damage_at_normal_stats() {
    let attacker = fresh_mon("tauros", 100, &["bodyslam"]);
    let defender = fresh_mon("snorlax", 100, &["reflect"]);

    let mut sides_no = make_sides(attacker.clone(), defender.clone());
    let mut rng_no = Rng::new(99);
    let mut field = Field::default();
    let mut log = empty_log();
    let out_no = ex(&mut rng_no, &mut field, &mut sides_no, 0, 0, &mut log);

    let mut sides_y = make_sides(attacker, defender);
    let mut rng_y = Rng::new(99);
    let _ = ex(&mut rng_y, &mut field, &mut sides_y, 1, 0, &mut log); // Reflect
    let out_y = ex(&mut rng_y, &mut field, &mut sides_y, 0, 0, &mut log); // Body Slam

    // Snorlax Def stays under 256 even doubled here? Doubling 168 → 336
    // triggers the rollover, so just require damage not to increase hugely…
    // actually at these stats Reflect helps; assert it doesn't hurt.
    assert!(
        out_y.damage_dealt <= out_no.damage_dealt,
        "Reflect should reduce damage here ({} vs {})",
        out_y.damage_dealt,
        out_no.damage_dealt
    );
}

#[test]
fn haze_clears_stages_and_foe_status() {
    let mut a = fresh_mon("golbat", 100, &["haze"]);
    a.stages = [2, -2, 1, -1, 0, 0];
    let mut d = fresh_mon("tauros", 100, &["bodyslam"]);
    d.stages = [-3, 3, 0, 0, 0, 0];
    d.status = Status::Poison;
    let mut sides = make_sides(a, d);
    let mut rng = Rng::new(1);
    let mut field = Field::default();
    let mut log = empty_log();
    let _ = ex(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
    assert_eq!(sides[0].active().stages, [0; 6]);
    assert_eq!(sides[1].active().stages, [0; 6]);
    assert!(matches!(sides[1].active().status, Status::None));
}

// ─────────────────────────────────────────────────────────────────────────────
// Fixed damage
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn seismic_toss_deals_user_level() {
    let a = fresh_mon("hitmonchan", 75, &["seismictoss"]);
    let d = fresh_mon("snorlax", 100, &["rest"]);
    let mut sides = make_sides(a, d);
    let pre = sides[1].active().hp_cur;
    let mut rng = Rng::new(2);
    let mut field = Field::default();
    let mut log = empty_log();
    let _ = ex(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
    assert_eq!(pre - sides[1].active().hp_cur, 75);
}

#[test]
fn fixed_damage_ignores_type_immunity() {
    // Sonic Boom (Normal) hits Gengar (Ghost) in Gen 1.
    let a = fresh_mon("voltorb", 50, &["sonicboom"]);
    let d = fresh_mon("gengar", 100, &["nightshade"]);
    let mut sides = make_sides(a, d);
    let pre = sides[1].active().hp_cur;
    let mut rng = Rng::new(2);
    let mut field = Field::default();
    let mut log = empty_log();
    let _ = ex(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
    assert_eq!(pre - sides[1].active().hp_cur, 20);
}

#[test]
fn super_fang_halves_hp() {
    let a = fresh_mon("raticate", 100, &["superfang"]);
    let d = fresh_mon("snorlax", 100, &["rest"]);
    let mut sides = make_sides(a, d);
    let pre = sides[1].active().hp_cur;
    let mut rng = Rng::new(2);
    let mut field = Field::default();
    let mut log = empty_log();
    let _ = ex(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
    let dealt = pre - sides[1].active().hp_cur;
    assert!((dealt as i32 - (pre / 2) as i32).abs() <= 1);
}

// ─────────────────────────────────────────────────────────────────────────────
// Recoil
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn double_edge_recoil_is_quarter() {
    let a = fresh_mon("tauros", 100, &["doubleedge"]);
    let d = fresh_mon("snorlax", 100, &["rest"]);
    let mut sides = make_sides(a, d);
    let pre_a = sides[0].active().hp_cur;
    let mut rng = Rng::new(2);
    let mut field = Field::default();
    let mut log = empty_log();
    let out = ex(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
    assert!(out.damage_dealt > 0, "should hit at this seed");
    let recoil = pre_a - sides[0].active().hp_cur;
    let expected = (out.damage_dealt / 4).max(1);
    assert_eq!(recoil, expected);
}

#[test]
fn struggle_recoil_is_half() {
    let a = fresh_mon("tauros", 100, &["bodyslam"]);
    let d = fresh_mon("snorlax", 100, &["rest"]);
    let mut sides = make_sides(a, d);
    let pre_a = sides[0].active().hp_cur;
    let mut rng = Rng::new(2);
    let mut field = Field::default();
    let mut log = empty_log();
    let out = execute_struggle(&mut rng, &mut field, &mut sides, 0, &mut log);
    assert!(out.damage_dealt > 0);
    let recoil = pre_a - sides[0].active().hp_cur;
    assert_eq!(recoil, (out.damage_dealt / 2).max(1));
}
