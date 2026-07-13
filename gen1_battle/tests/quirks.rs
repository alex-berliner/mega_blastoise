//! Regression tests for the Gen 1 battle glitches ported from Pokémon
//! Showdown's gen1 mod (Bulbapedia: "List of battle glitches in Generation I").

use gen1_battle::testing::*;
use gen1_battle::{type_effectiveness, Type};

fn fresh_mon(species: &'static str, level: u8, moves: &[&'static str]) -> Mon {
    Mon::from_species(species, level, moves).expect("species lookup")
}

fn make_sides(p1: Mon, p2: Mon) -> [Side; 2] {
    let mut sides = <[Side; 2]>::default();
    let _ = sides[0].player_id.push_str("p1");
    let _ = sides[1].player_id.push_str("p2");
    sides[0].team[0] = p1;
    sides[1].team[0] = p2;
    sides
}

// ─────────────────────────────────────────────────────────────────────────────
// Type chart
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn gen1_type_chart_quirks() {
    // The famous one: Ghost does NOTHING to Psychic in Gen 1.
    assert_eq!(type_effectiveness(Type::Ghost, Type::Psychic), 0);
    // Bug and Poison are super-effective against each other.
    assert_eq!(type_effectiveness(Type::Bug, Type::Poison), 20);
    assert_eq!(type_effectiveness(Type::Poison, Type::Bug), 20);
    // Ice is neutral against Fire (resisted from Gen 2 on).
    assert_eq!(type_effectiveness(Type::Ice, Type::Fire), 10);
    // Sanity: unchanged cells.
    assert_eq!(type_effectiveness(Type::Water, Type::Fire), 20);
    assert_eq!(type_effectiveness(Type::Normal, Type::Ghost), 0);
    assert_eq!(type_effectiveness(Type::Electric, Type::Ground), 0);
}

// ─────────────────────────────────────────────────────────────────────────────
// 1/256 miss
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn hundred_percent_accuracy_can_miss() {
    // Effective accuracy 255/256: hit_roll must miss for SOME seed.
    let mut missed = false;
    for seed in 1..3000u64 {
        let mut rng = Rng::new(seed);
        let (hit, eff) = hit_roll(&mut rng, 255, 0, 0);
        assert_eq!(eff, 255);
        if !hit {
            missed = true;
            break;
        }
    }
    assert!(missed, "the 1/256 miss must exist");
}

// ─────────────────────────────────────────────────────────────────────────────
// Crits: base-Speed rate, Focus Energy bug
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn focus_energy_lowers_crit_rate() {
    let plain = fresh_mon("tauros", 100, &["bodyslam"]);
    let mut pumped = plain.clone();
    pumped.volatile.set(Volatile::FOCUS_ENERGY);
    let mv = gen1_battle::move_by_id("bodyslam").unwrap();

    let count = |mon: &Mon| -> u32 {
        let mut crits = 0;
        for seed in 1..2000u64 {
            let mut rng = Rng::new(seed);
            if crit_roll(&mut rng, mon, mv) {
                crits += 1;
            }
        }
        crits
    };
    let base_crits = count(&plain);
    let fe_crits = count(&pumped);
    // Tauros base 110 Speed: ~21% normally, ~5% with the Focus Energy bug.
    assert!(base_crits > fe_crits * 2, "Focus Energy must LOWER crit rate ({base_crits} vs {fe_crits})");
}

#[test]
fn crit_rate_uses_species_base_speed_not_final_stat() {
    // A transformed/boosted final Speed must not change crit rate: crank the
    // final stat and verify the rate stays put.
    let mut slow = fresh_mon("snorlax", 100, &["bodyslam"]); // base 30 Speed
    slow.stats[4] = 999;
    slow.modified[4] = 999;
    let mv = gen1_battle::move_by_id("bodyslam").unwrap();
    let mut crits = 0;
    for seed in 1..2000u64 {
        let mut rng = Rng::new(seed);
        if crit_roll(&mut rng, &slow, mv) {
            crits += 1;
        }
    }
    // base 30 → threshold 15/256 ≈ 5.9%. Allow generous slack, but far
    // below the ~50%+ a 999 final Speed would produce.
    assert!(crits < 300, "crit rate must come from base Speed ({crits}/2000)");
}

// ─────────────────────────────────────────────────────────────────────────────
// Stat modification errors (sticky modified stats)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn paralysis_drop_erased_by_own_boost() {
    let mut a = fresh_mon("zapdos", 100, &["agility"]);
    let spe = a.modified[4];
    a.status = Status::Paralysis;
    apply_status_drop(&mut a);
    assert_eq!(a.modified[4], spe / 4, "paralysis quarters Speed");
    let d = fresh_mon("snorlax", 100, &["rest"]);
    let mut sides = make_sides(a, d);
    let mut rng = Rng::new(1);
    let mut field = Field::default();
    let mut log = Log::new();
    let _ = execute_move(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
    // Agility recalculates Speed from stages — the paralysis drop is GONE.
    assert_eq!(sides[0].active().modified[4], (spe as u32 * 2).min(999) as u16);
}

#[test]
fn foe_boost_restacks_paralysis_drop() {
    let mut par = fresh_mon("zapdos", 100, &["thunderbolt"]);
    par.status = Status::Paralysis;
    apply_status_drop(&mut par);
    let once = par.modified[4];
    let sd = fresh_mon("snorlax", 100, &["swordsdance"]);
    let mut sides = make_sides(sd, par);
    let mut rng = Rng::new(1);
    let mut field = Field::default();
    let mut log = Log::new();
    // Snorlax uses Swords Dance: the paralyzed foe's Speed is quartered AGAIN.
    let _ = execute_move(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
    assert_eq!(sides[1].active().modified[4], (once / 4).max(1));
}

#[test]
fn burn_drop_erased_by_own_boost() {
    let mut a = fresh_mon("tauros", 100, &["swordsdance"]);
    let atk = a.modified[1];
    a.status = Status::Burn;
    apply_status_drop(&mut a);
    assert_eq!(a.modified[1], atk / 2);
    let d = fresh_mon("snorlax", 100, &["rest"]);
    let mut sides = make_sides(a, d);
    let mut rng = Rng::new(1);
    let mut field = Field::default();
    let mut log = Log::new();
    let _ = execute_move(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
    assert_eq!(sides[0].active().modified[1], (atk as u32 * 2).min(999) as u16);
}

// ─────────────────────────────────────────────────────────────────────────────
// Toxic counter (residualdmg)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn leech_seed_shares_and_increments_toxic_counter() {
    let mut victim = fresh_mon("snorlax", 100, &["rest"]);
    victim.status = Status::BadPoison;
    victim.volatile.set(Volatile::LEECH_SEEDED);
    let seeder = fresh_mon("venusaur", 100, &["leechseed"]);
    let mut sides = make_sides(seeder, victim);
    let max = sides[1].active().hp_max;
    let base = (max / 16).max(1);
    let mut field = Field::default();
    let mut log = Log::new();
    after_action_residuals(&mut field, &mut sides, 1, &mut log);
    // Turn 1: tox tick ×1, then seed tick ×2 (counter incremented by both).
    let lost = max - sides[1].active().hp_cur;
    assert_eq!(lost, base * 1 + base * 2);
    after_action_residuals(&mut field, &mut sides, 1, &mut log);
    // Turn 2: tox ×3, seed ×4.
    let lost2 = max - sides[1].active().hp_cur - lost;
    assert_eq!(lost2, base * 3 + base * 4);
}

#[test]
fn rest_preserves_toxic_counter() {
    let mut a = fresh_mon("snorlax", 100, &["rest"]);
    a.status = Status::BadPoison;
    let d = fresh_mon("tauros", 100, &["bodyslam"]);
    let mut sides = make_sides(a, d);
    let max = sides[0].active().hp_max;
    let base = (max / 16).max(1);
    let mut field = Field::default();
    let mut rng = Rng::new(1);
    let mut log = Log::new();
    // Two toxic ticks: counter = 2.
    after_action_residuals(&mut field, &mut sides, 0, &mut log);
    after_action_residuals(&mut field, &mut sides, 0, &mut log);
    assert_eq!(sides[0].active().volatile.toxic_counter, 2);
    // Rest: cures the poison, keeps the counter.
    let _ = execute_move(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
    assert!(matches!(sides[0].active().status, Status::Sleep(_)));
    assert!(sides[0].active().volatile.has(Volatile::TOX_COUNTER));
    assert_eq!(sides[0].active().volatile.toxic_counter, 2);
    // Re-poisoned later: residuals use the PRESERVED counter (×2, flat).
    sides[0].active_mut().status = Status::Poison;
    let pre = sides[0].active().hp_cur;
    after_action_residuals(&mut field, &mut sides, 0, &mut log);
    assert_eq!(pre - sides[0].active().hp_cur, base * 2);
}

#[test]
fn toxic_counter_dies_on_switch() {
    let mut a = fresh_mon("snorlax", 100, &["rest"]);
    a.status = Status::BadPoison;
    a.volatile.set(Volatile::TOX_COUNTER);
    a.volatile.toxic_counter = 5;
    let d = fresh_mon("tauros", 100, &["bodyslam"]);
    let mut sides = make_sides(a, d);
    reset_on_switch_out(&mut sides[0]);
    let m = sides[0].active();
    assert_eq!(m.status, Status::Poison, "tox downgrades to psn on switch");
    assert!(!m.volatile.has(Volatile::TOX_COUNTER));
    assert_eq!(m.volatile.toxic_counter, 0);
}

// ─────────────────────────────────────────────────────────────────────────────
// Recovery move failure (255/511 deficit)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn recover_fails_at_255_deficit() {
    let mut a = fresh_mon("chansey", 100, &["softboiled"]);
    // Chansey's max HP at L100 is 703: set deficit to exactly 255.
    let max = a.hp_max;
    assert!(max > 256);
    a.hp_cur = max - 255;
    assert_ne!(a.hp_cur % 256, 0);
    let d = fresh_mon("tauros", 100, &["bodyslam"]);
    let mut sides = make_sides(a, d);
    let mut rng = Rng::new(1);
    let mut field = Field::default();
    let mut log = Log::new();
    let pre = sides[0].active().hp_cur;
    let _ = execute_move(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
    assert_eq!(sides[0].active().hp_cur, pre, "recovery must fail at deficit 255");

    // One HP away from the cursed value it works fine.
    sides[0].active_mut().hp_cur = max - 254;
    let pre = sides[0].active().hp_cur;
    let _ = execute_move(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
    assert!(sides[0].active().hp_cur > pre);
}

// ─────────────────────────────────────────────────────────────────────────────
// Freeze / Hyper Beam interactions
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn fire_move_with_burn_chance_thaws() {
    let a = fresh_mon("charizard", 100, &["flamethrower"]);
    let mut d = fresh_mon("snorlax", 100, &["rest"]);
    d.status = Status::Freeze;
    let mut sides = make_sides(a, d);
    let mut field = Field::default();
    let mut log = Log::new();
    for seed in 1..10u64 {
        let mut rng = Rng::new(seed);
        let out = execute_move(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
        if out.damage_dealt > 0 {
            assert!(matches!(sides[1].active().status, Status::None), "fire hit must thaw");
            return;
        }
    }
    panic!("flamethrower never hit");
}

#[test]
fn frozen_mon_keeps_recharge_flag() {
    // Hyper Beam + Freeze: the recharge is never consumed while frozen.
    let mut a = fresh_mon("snorlax", 100, &["hyperbeam"]);
    a.status = Status::Freeze;
    a.volatile.set(Volatile::MUST_RECHARGE);
    let d = fresh_mon("tauros", 100, &["bodyslam"]);
    let mut sides = make_sides(a, d);
    let mut rng = Rng::new(1);
    let mut field = Field::default();
    let mut log = Log::new();
    for _ in 0..5 {
        assert!(!pre_move_check(&mut rng, &mut field, &mut sides, 0, Some(0), &mut log));
        assert!(sides[0].active().volatile.has(Volatile::MUST_RECHARGE));
    }
}

#[test]
fn sleep_move_on_recharging_target_always_hits_and_overwrites() {
    let a = fresh_mon("gengar", 100, &["hypnosis"]);
    let mut d = fresh_mon("snorlax", 100, &["hyperbeam"]);
    d.status = Status::Paralysis; // would normally block a second status
    d.volatile.set(Volatile::MUST_RECHARGE);
    let mut sides = make_sides(a, d);
    let mut rng = Rng::new(1);
    let mut field = Field::default();
    let mut log = Log::new();
    let _ = execute_move(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
    assert!(matches!(sides[1].active().status, Status::Sleep(_)), "sleep must overwrite");
    assert!(!sides[1].active().volatile.has(Volatile::MUST_RECHARGE));
}

#[test]
fn hyper_beam_no_recharge_on_sub_break() {
    let a = fresh_mon("tauros", 100, &["hyperbeam"]);
    let mut d = fresh_mon("snorlax", 100, &["substitute"]);
    d.volatile.set(Volatile::SUBSTITUTED);
    d.volatile.substitute_hp = 5; // guaranteed to break
    let mut sides = make_sides(a, d);
    let mut field = Field::default();
    let mut log = Log::new();
    for seed in 1..20u64 {
        let mut rng = Rng::new(seed);
        sides[1].active_mut().volatile.set(Volatile::SUBSTITUTED);
        sides[1].active_mut().volatile.substitute_hp = 5;
        sides[0].active_mut().volatile.clear(Volatile::MUST_RECHARGE);
        let out = execute_move(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
        if out.sub_broke {
            assert!(
                !sides[0].active().volatile.has(Volatile::MUST_RECHARGE),
                "breaking a sub must not require a recharge"
            );
            return;
        }
    }
    panic!("hyper beam never broke the sub");
}

// ─────────────────────────────────────────────────────────────────────────────
// Secondary-status type immunity (Body Slam vs Normal)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn body_slam_never_paralyzes_normal_types() {
    let a = fresh_mon("tauros", 100, &["bodyslam"]);
    let d = fresh_mon("snorlax", 100, &["rest"]); // Normal-type
    for seed in 1..300u64 {
        let mut sides = make_sides(a.clone(), d.clone());
        let mut rng = Rng::new(seed);
        let mut field = Field::default();
        let mut log = Log::new();
        let _ = execute_move(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
        assert!(
            !matches!(sides[1].active().status, Status::Paralysis),
            "Body Slam must never paralyze a Normal-type (seed {seed})"
        );
    }
}

#[test]
fn thunderbolt_can_paralyze_non_electric() {
    let a = fresh_mon("zapdos", 100, &["thunderbolt"]);
    let d = fresh_mon("starmie", 100, &["recover"]);
    for seed in 1..300u64 {
        let mut sides = make_sides(a.clone(), d.clone());
        let mut rng = Rng::new(seed);
        let mut field = Field::default();
        let mut log = Log::new();
        let _ = execute_move(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
        if matches!(sides[1].active().status, Status::Paralysis) {
            return;
        }
    }
    panic!("thunderbolt never paralyzed in 300 seeds");
}

// ─────────────────────────────────────────────────────────────────────────────
// Invulnerability glitch + Swift
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn full_paralysis_leaves_invulnerability_stuck() {
    let mut a = fresh_mon("dugtrio", 100, &["dig"]);
    a.status = Status::Paralysis;
    a.volatile.set(Volatile::CHARGING);
    a.volatile.set(Volatile::INVULNERABLE);
    a.volatile.multi_turn_move = "dig";
    a.volatile.multi_turn_turns = 1;
    let d = fresh_mon("snorlax", 100, &["rest"]);
    let mut sides = make_sides(a, d);
    let mut field = Field::default();
    let mut log = Log::new();
    for seed in 1..500u64 {
        let mut rng = Rng::new(seed);
        // Restore the charge state each attempt.
        {
            let m = sides[0].active_mut();
            m.volatile.set(Volatile::CHARGING | Volatile::INVULNERABLE);
            m.volatile.multi_turn_move = "dig";
            m.volatile.multi_turn_turns = 1;
        }
        if !pre_move_check(&mut rng, &mut field, &mut sides, 0, Some(0), &mut log) {
            let m = sides[0].active();
            assert!(m.volatile.has(Volatile::INVULNERABLE), "invulnerability must stick");
            assert!(!m.volatile.has(Volatile::CHARGING), "charge lock must be cancelled");
            assert!(m.volatile.multi_turn_move.is_empty());
            return;
        }
    }
    panic!("never fully paralyzed in 500 seeds");
}

#[test]
fn swift_hits_through_invulnerability() {
    let a = fresh_mon("dragonite", 100, &["swift"]);
    let mut d = fresh_mon("pidgeot", 100, &["fly"]);
    d.volatile.set(Volatile::INVULNERABLE);
    let mut sides = make_sides(a, d);
    let mut rng = Rng::new(1);
    let mut field = Field::default();
    let mut log = Log::new();
    let out = execute_move(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
    assert!(out.damage_dealt > 0, "Swift must hit a mon in Fly/Dig");
}

// ─────────────────────────────────────────────────────────────────────────────
// 0-damage glitch
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn zero_damage_becomes_a_miss() {
    // A pitifully weak double-resisted hit rounds to 0 → miss, not min-1.
    let mut a = fresh_mon("caterpie", 2, &["tackle"]);
    a.stats[1] = 1;
    a.modified[1] = 1;
    // Aerodactyl (Rock/Flying) double-resists... Normal is neutral; use a
    // fighting move vs Ghost? immune, not resist. Use Onix (Rock/Ground):
    // Normal ½ × neutral. We need ¼: Vine Whip (Grass) vs Charizard
    // (Fire/Flying) = ¼×¼? ½×½ = ¼. Level 2, 1 Atk → 0 damage.
    let mut a = fresh_mon("bulbasaur", 2, &["vinewhip"]);
    a.stats[3] = 1;
    a.modified[3] = 1;
    a.base_spe = 0; // never crit (crits would use raw stats)
    let d = fresh_mon("charizard", 100, &["flamethrower"]);
    let mut sides = make_sides(a, d);
    let mut field = Field::default();
    let mut log = Log::new();
    for seed in 1..50u64 {
        let mut rng = Rng::new(seed);
        let out = execute_move(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
        assert_eq!(out.damage_dealt, 0, "quad-resisted 2-damage base must round to 0 (seed {seed})");
        assert!(!out.hit || out.damage_dealt == 0);
    }
    assert_eq!(sides[1].active().hp_cur, sides[1].active().hp_max);
}

// ─────────────────────────────────────────────────────────────────────────────
// Explosion / Substitute
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn explosion_user_survives_breaking_a_sub() {
    let a = fresh_mon("gengar", 100, &["explosion"]);
    let mut d = fresh_mon("snorlax", 100, &["substitute"]);
    d.volatile.set(Volatile::SUBSTITUTED);
    d.volatile.substitute_hp = 3;
    let mut sides = make_sides(a, d);
    let mut field = Field::default();
    let mut log = Log::new();
    for seed in 1..20u64 {
        let mut rng = Rng::new(seed);
        sides[1].active_mut().volatile.set(Volatile::SUBSTITUTED);
        sides[1].active_mut().volatile.substitute_hp = 3;
        let out = execute_move(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
        if out.sub_broke {
            assert!(sides[0].active().hp_cur > 0, "user survives when the blast breaks a sub");
            return;
        }
        if out.fainted_user {
            // Missed (1/256) — user faints; try another seed for the break.
            sides[0].active_mut().hp_cur = sides[0].active().hp_max;
        }
    }
    panic!("explosion never broke the sub");
}

#[test]
fn explosion_user_faints_on_miss_or_immune() {
    let a = fresh_mon("electrode", 100, &["explosion"]);
    let d = fresh_mon("gengar", 100, &["nightshade"]); // Ghost: immune to Normal
    let mut sides = make_sides(a, d);
    let mut rng = Rng::new(1);
    let mut field = Field::default();
    let mut log = Log::new();
    let _ = execute_move(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
    assert_eq!(sides[0].active().hp_cur, 0, "exploding into an immune target still faints");
}

// ─────────────────────────────────────────────────────────────────────────────
// Substitute edge cases
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn substitute_at_exactly_quarter_hp_faints_user() {
    // The lethal exact-quarter case needs max HP divisible by 4 (otherwise
    // hp < maxhp/4 fails first — same as the cartridge float compare).
    let mut a = fresh_mon("snorlax", 100, &["substitute"]);
    a.hp_max = 400;
    a.stats[0] = 400;
    a.hp_cur = 100; // exactly the cost
    let d = fresh_mon("tauros", 100, &["bodyslam"]);
    let mut sides = make_sides(a, d);
    let mut rng = Rng::new(1);
    let mut field = Field::default();
    let mut log = Log::new();
    let out = execute_move(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
    assert!(out.fainted_user, "sub at exactly 1/4 HP is allowed and lethal");
    assert!(sides[0].active().volatile.has(Volatile::SUBSTITUTED));

    // Strictly below 1/4 it fails instead.
    let mut a2 = fresh_mon("snorlax", 100, &["substitute"]);
    a2.hp_max = 400;
    a2.stats[0] = 400;
    a2.hp_cur = 99;
    let mut sides2 = make_sides(a2, fresh_mon("tauros", 100, &["bodyslam"]));
    let out2 = execute_move(&mut rng, &mut field, &mut sides2, 0, 0, &mut log);
    assert!(!out2.fainted_user);
    assert!(!sides2[0].active().volatile.has(Volatile::SUBSTITUTED));
}

#[test]
fn confusion_self_hit_redirects_to_foe_sub() {
    let mut a = fresh_mon("snorlax", 100, &["bodyslam"]);
    a.volatile.set(Volatile::CONFUSED | Volatile::SUBSTITUTED);
    a.volatile.substitute_hp = 50;
    a.volatile.confused_turns = 4;
    let mut d = fresh_mon("tauros", 100, &["rest"]);
    d.volatile.set(Volatile::SUBSTITUTED);
    d.volatile.substitute_hp = 200;
    let mut sides = make_sides(a, d);
    let mut field = Field::default();
    let mut log = Log::new();
    for seed in 1..200u64 {
        let mut rng = Rng::new(seed);
        sides[0].active_mut().volatile.set(Volatile::CONFUSED);
        sides[0].active_mut().volatile.confused_turns = 4;
        let own_hp = sides[0].active().hp_cur;
        let foe_sub = sides[1].active().volatile.substitute_hp;
        if !pre_move_check(&mut rng, &mut field, &mut sides, 0, Some(0), &mut log) {
            // Self-hit happened: own HP untouched, FOE's sub took it.
            assert_eq!(sides[0].active().hp_cur, own_hp);
            assert!(sides[1].active().volatile.substitute_hp < foe_sub);
            return;
        }
    }
    panic!("never hit itself in confusion across 200 seeds");
}

// ─────────────────────────────────────────────────────────────────────────────
// Thrash / Rage
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn thrash_locks_then_confuses() {
    let a = fresh_mon("tauros", 100, &["thrash"]);
    let d = fresh_mon("snorlax", 100, &["rest"]);
    let mut sides = make_sides(a, d);
    let mut rng = Rng::new(5);
    let mut field = Field::default();
    let mut log = Log::new();
    let _ = execute_move(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
    assert_eq!(locked_move_id(&sides[0]), Some("thrash"));
    // Keep using it until the lock ends; then the user must be confused.
    for _ in 0..5 {
        if locked_move_id(&sides[0]).is_none() {
            break;
        }
        let _ = execute_locked_move(&mut rng, &mut field, &mut sides, 0, "thrash", &mut log);
    }
    assert_eq!(locked_move_id(&sides[0]), None, "thrash must end");
    assert!(sides[0].active().volatile.has(Volatile::CONFUSED), "thrash ends in confusion");
}

#[test]
fn rage_builds_attack_when_hit() {
    let mut rager = fresh_mon("tauros", 100, &["rage"]);
    rager.volatile.set(Volatile::RAGE);
    rager.volatile.multi_turn_move = "rage";
    rager.volatile.multi_turn_turns = 255;
    let attacker = fresh_mon("snorlax", 100, &["bodyslam"]);
    let mut sides = make_sides(attacker, rager);
    let mut field = Field::default();
    let mut log = Log::new();
    for seed in 1..10u64 {
        let mut rng = Rng::new(seed);
        let out = execute_move(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
        if out.damage_dealt > 0 {
            assert_eq!(sides[1].active().stages[0], 1, "rage must build +1 Atk when hit");
            return;
        }
    }
    panic!("body slam never hit");
}

// ─────────────────────────────────────────────────────────────────────────────
// Bide
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn bide_returns_double_accumulated_last_damage() {
    let a = fresh_mon("snorlax", 100, &["bide"]);
    let d = fresh_mon("tauros", 100, &["bodyslam"]);
    let mut sides = make_sides(a, d);
    let mut rng = Rng::new(1);
    let mut field = Field::default();
    let mut log = Log::new();
    let _ = execute_move(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
    assert!(sides[0].active().volatile.has(Volatile::BIDING));
    let turns = sides[0].active().volatile.bide_turns;
    // Simulate taking 30 damage once; the register then goes stale and gets
    // re-added every remaining Bide turn (cartridge accumulator bug).
    field.last_damage = 30;
    let pre_foe = sides[1].active().hp_cur;
    let mut unleashed = 0;
    for _ in 0..turns {
        let out = execute_locked_move(&mut rng, &mut field, &mut sides, 0, "bide", &mut log);
        if out.damage_dealt > 0 {
            unleashed = out.damage_dealt;
        }
    }
    assert_eq!(unleashed, 30 * turns as u16 * 2, "stale last-damage re-added each turn, ×2");
    assert!(sides[1].active().hp_cur < pre_foe);
}

// ─────────────────────────────────────────────────────────────────────────────
// Haze foe-status cure lost turn
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn haze_cure_costs_the_foe_its_turn() {
    let a = fresh_mon("golbat", 100, &["haze"]);
    let mut d = fresh_mon("snorlax", 100, &["rest"]);
    d.status = Status::Freeze;
    let mut sides = make_sides(a, d);
    let mut rng = Rng::new(1);
    let mut field = Field::default();
    field.foe_acted = false; // foe hasn't moved yet this turn
    let mut log = Log::new();
    let _ = execute_move(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
    assert!(matches!(sides[1].active().status, Status::None));
    // The thawed foe loses its action this turn.
    assert!(!pre_move_check(&mut rng, &mut field, &mut sides, 1, Some(0), &mut log));
    // …but only this turn.
    assert!(pre_move_check(&mut rng, &mut field, &mut sides, 1, Some(0), &mut log));
}

// ─────────────────────────────────────────────────────────────────────────────
// Dream Eater
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn dream_eater_fails_on_awake_target() {
    let a = fresh_mon("hypno", 100, &["dreameater"]);
    let d = fresh_mon("snorlax", 100, &["rest"]);
    let mut sides = make_sides(a, d);
    let mut rng = Rng::new(1);
    let mut field = Field::default();
    let mut log = Log::new();
    let out = execute_move(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
    assert_eq!(out.damage_dealt, 0);

    sides[1].active_mut().status = Status::Sleep(3);
    let out = execute_move(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
    assert!(out.damage_dealt > 0, "dream eater works on a sleeper");
}

// ─────────────────────────────────────────────────────────────────────────────
// Format rules: Sleep Clause Mod / Freeze Clause Mod
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn sleep_clause_blocks_second_enemy_sleep() {
    let a = fresh_mon("gengar", 100, &["hypnosis"]);
    let d = fresh_mon("snorlax", 100, &["rest"]);
    let mut benched = fresh_mon("chansey", 100, &["softboiled"]);
    benched.status = Status::Sleep(4); // enemy-inflicted (rest_sleep = false)
    let mut sides = make_sides(a, d);
    sides[1].team[1] = benched;
    let mut field = Field::default();
    let mut log = Log::new();
    for seed in 1..200u64 {
        let mut rng = Rng::new(seed);
        let _ = execute_move(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
        assert!(
            matches!(sides[1].active().status, Status::None),
            "Sleep Clause must block a second enemy sleep (seed {seed})"
        );
    }
}

#[test]
fn rest_sleep_does_not_trigger_sleep_clause() {
    let a = fresh_mon("gengar", 100, &["hypnosis"]);
    let d = fresh_mon("snorlax", 100, &["rest"]);
    let mut benched = fresh_mon("chansey", 100, &["softboiled"]);
    benched.status = Status::Sleep(2);
    benched.rest_sleep = true; // slept via its own Rest
    let mut sides = make_sides(a, d);
    sides[1].team[1] = benched;
    let mut field = Field::default();
    let mut log = Log::new();
    for seed in 1..100u64 {
        let mut rng = Rng::new(seed);
        let _ = execute_move(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
        if matches!(sides[1].active().status, Status::Sleep(_)) {
            assert!(!sides[1].active().rest_sleep, "enemy sleep must clear the Rest flag");
            return; // hypnosis (60%) landed despite the resting teammate
        }
    }
    panic!("hypnosis never landed in 100 seeds");
}

#[test]
fn sleep_clause_blocks_recharge_sleep_and_keeps_recharge() {
    let a = fresh_mon("gengar", 100, &["hypnosis"]);
    let mut d = fresh_mon("snorlax", 100, &["hyperbeam"]);
    d.volatile.set(Volatile::MUST_RECHARGE);
    let mut benched = fresh_mon("chansey", 100, &["softboiled"]);
    benched.status = Status::Sleep(4);
    let mut sides = make_sides(a, d);
    sides[1].team[1] = benched;
    let mut rng = Rng::new(1);
    let mut field = Field::default();
    let mut log = Log::new();
    let _ = execute_move(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
    assert!(matches!(sides[1].active().status, Status::None));
    assert!(
        sides[1].active().volatile.has(Volatile::MUST_RECHARGE),
        "blocked recharge-sleep must NOT clear the recharge"
    );
}

#[test]
fn freeze_clause_blocks_second_freeze() {
    let a = fresh_mon("lapras", 100, &["icebeam"]);
    let d = fresh_mon("snorlax", 100, &["rest"]);
    let mut benched = fresh_mon("chansey", 100, &["softboiled"]);
    benched.status = Status::Freeze;
    let mut sides = make_sides(a, d);
    sides[1].team[1] = benched;
    let mut field = Field::default();
    let mut log = Log::new();
    for seed in 1..500u64 {
        let mut rng = Rng::new(seed);
        let _ = execute_move(&mut rng, &mut field, &mut sides, 0, 0, &mut log);
        assert!(
            sides[1].active().status != Status::Freeze,
            "Freeze Clause must block a second freeze (seed {seed})"
        );
        // Reset chip damage so the loop can't KO.
        let max = sides[1].active().hp_max;
        sides[1].active_mut().hp_cur = max;
    }
}
