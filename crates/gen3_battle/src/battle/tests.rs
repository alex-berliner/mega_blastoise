//! The turn loop's own unit tests, engine-level and script-driven.

use super::*;

fn mon(id: &str, level: u8, moves: &[&str]) -> Mon {
    Mon::new(id, level, Nature::Hardy, Invest { iv: 31, ev: 0 }, moves)
        .unwrap_or_else(|| panic!("{id} is not in the dex"))
}

fn battle(a: Mon, b: Mon) -> Battle {
    Battle::new(Side::new(alloc::vec![a]), Side::new(alloc::vec![b]), 42)
}

#[test]
fn a_turn_damages_and_spends_pp() {
    let mut b = battle(
        mon("blaziken", 50, &["ember"]),
        mon("treecko", 50, &["pound"]),
    );
    let before = b.sides[1].mon().hp;
    let pp_before = b.sides[0].mon().moves[0].pp;
    let events = b.step([Choice::Move(0), Choice::Move(0)]);

    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Used { side: 1, .. })));
    assert!(
        b.sides[1].mon().hp < before,
        "treecko should have taken a hit"
    );
    assert_eq!(b.sides[0].mon().moves[0].pp, pp_before - 1);
}

#[test]
fn the_faster_mon_moves_first() {
    // Base Speed 80 against 70, so Blaziken moves first. Asserted against
    // the stats rather than a memory of who is fast.
    let mut b = battle(
        mon("blaziken", 50, &["ember"]),
        mon("treecko", 50, &["pound"]),
    );
    let faster = if b.sides[0].mon().spe > b.sides[1].mon().spe {
        1
    } else {
        2
    };
    assert_ne!(
        b.sides[0].mon().spe,
        b.sides[1].mon().spe,
        "the tie-break is a different test"
    );
    let events = b.step([Choice::Move(0), Choice::Move(0)]);
    let first_used = events
        .iter()
        .find_map(|e| match e {
            Event::Used { side, .. } => Some(*side),
            _ => None,
        })
        .expect("someone moved");
    assert_eq!(first_used, faster);
}

#[test]
fn type_effectiveness_reaches_the_events() {
    // Ember into Treecko is super effective: Fire on Grass.
    let mut b = battle(
        mon("blaziken", 50, &["ember"]),
        mon("treecko", 50, &["pound"]),
    );
    let events = b.step([Choice::Move(0), Choice::Move(0)]);
    let eff = events
        .iter()
        .find_map(|e| match e {
            Event::Damage {
                side: 2,
                effectiveness,
                ..
            } => Some(*effectiveness),
            _ => None,
        })
        .expect("treecko took damage");
    assert_eq!(eff, 200);
}

#[test]
fn a_battle_ends_when_a_side_is_out() {
    // A level 100 attacker against a level 5 defender: one hit, one win.
    let mut b = battle(
        mon("blaziken", 100, &["ember"]),
        mon("treecko", 5, &["pound"]),
    );
    let mut saw_win = None;
    for _ in 0..8 {
        for e in b.step([Choice::Move(0), Choice::Move(0)]) {
            if let Event::Win { side } = e {
                saw_win = Some(side);
            }
        }
        if b.over() {
            break;
        }
    }
    assert_eq!(saw_win, Some(1), "blaziken should win");
    assert!(b.over());
}

#[test]
fn a_fainted_mon_is_replaced_from_the_party() {
    let side1 = Side::new(alloc::vec![mon("blaziken", 100, &["ember"])]);
    let side2 = Side::new(alloc::vec![
        mon("treecko", 5, &["pound"]),
        mon("mudkip", 50, &["pound"])
    ]);
    let mut b = Battle::new(side1, side2, 7);
    let events = b.step([Choice::Move(0), Choice::Move(0)]);
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Fainted { side: 2 })));
    assert_eq!(b.sides[1].active, 1, "the next mon steps in");
    assert!(!b.over(), "the side still has a mon");
}

#[test]
fn switching_happens_before_anyone_attacks() {
    let side1 = Side::new(alloc::vec![
        mon("blaziken", 50, &["ember"]),
        mon("mudkip", 50, &["pound"])
    ]);
    let side2 = Side::new(alloc::vec![mon("treecko", 50, &["pound"])]);
    let mut b = Battle::new(side1, side2, 3);
    let events = b.step([Choice::Switch(1), Choice::Move(0)]);
    let switched = events
        .iter()
        .position(|e| matches!(e, Event::Switched { side: 1, .. }));
    let used = events.iter().position(|e| matches!(e, Event::Used { .. }));
    assert!(switched.is_some() && used.is_some());
    assert!(switched < used, "the switch resolves first");
    assert_eq!(b.sides[0].active, 1);
}

#[test]
fn a_move_with_no_pp_struggles_instead() {
    let mut b = battle(
        mon("blaziken", 50, &["ember"]),
        mon("treecko", 50, &["pound"]),
    );
    b.sides[0].party[0].moves[0].pp = 0;
    let hp = b.sides[1].mon().hp;
    let before = b.sides[0].mon().hp;
    let events = b.step([Choice::Move(0), Choice::Move(0)]);
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Used { side: 1, .. })));
    assert!(b.sides[1].mon().hp < hp, "Struggle landed");
    assert!(b.sides[0].mon().hp < before, "and recoiled");
    assert_eq!(b.sides[0].mon().moves[0].pp, 0, "no PP moved");
}

fn scripted(script: [SeatScript; 2]) -> TurnScript {
    TurnScript {
        seats: [Some(script[0]), Some(script[1])],
        claw: false,
    }
}

const PLAIN: SeatScript = SeatScript {
    hit: true,
    crit: false,
    random: 100,
    secondary: false,
    immobile: false,
    hits: 0,
    selfhit: false,
    stall: false,
    band: false,
};

#[test]
fn a_flinched_mon_loses_its_action_for_exactly_one_turn() {
    // Blaziken outspeeds and Headbutt's flinch procs: Snorlax never moves.
    // (Snorlax rather than Treecko so two Headbutts cannot KO it.)
    let mut b = battle(
        mon("blaziken", 50, &["headbutt"]),
        mon("snorlax", 50, &["pound"]),
    );
    assert!(b.sides[0].mon().spe > b.sides[1].mon().spe);
    let events = b.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([
            SeatScript {
                secondary: true,
                ..PLAIN
            },
            PLAIN,
        ]),
    );
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Flinched { side: 2 })));
    assert!(!events
        .iter()
        .any(|e| matches!(e, Event::Used { side: 2, .. })));
    // The flinch does not leak into the next turn.
    let events = b.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([PLAIN, PLAIN]),
    );
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Used { side: 2, .. })));
}

#[test]
fn sleep_lasts_its_clock_and_the_mon_acts_the_turn_it_wakes() {
    let mut b = battle(
        mon("blaziken", 50, &["sing"]),
        mon("snorlax", 50, &["pound"]),
    );
    // Turn 1: Sing lands (clock 2), and slower Snorlax's own action
    // already ticks it to 1 — a Cant the very turn it fell asleep.
    let events = b.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([PLAIN, PLAIN]),
    );
    assert!(events.iter().any(|e| matches!(
        e,
        Event::Statused {
            side: 2,
            status: Status::Sleep
        }
    )));
    assert!(events.iter().any(|e| matches!(
        e,
        Event::Cant {
            side: 2,
            status: Status::Sleep
        }
    )));
    assert_eq!(b.sides[1].mon().sleep_n, 1);
    // Turn 2: 1 -> 0, it wakes and moves that same turn. The turn's
    // earlier Sing could not re-land — Snorlax still carried slp when it
    // resolved — so the wake leaves it clean.
    let events = b.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([PLAIN, PLAIN]),
    );
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Used { side: 2, .. })));
    assert_eq!(b.sides[1].mon().status, None);
}

#[test]
fn thunder_wave_respects_ground_immunity() {
    let mut b = battle(
        mon("pikachu", 50, &["thunderwave"]),
        mon("golem", 50, &["splash"]),
    );
    let events = b.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([PLAIN, PLAIN]),
    );
    assert!(!events.iter().any(|e| matches!(e, Event::Statused { .. })));
    assert_eq!(
        b.sides[1].mon().status,
        None,
        "a Ground type shrugs off Thunder Wave"
    );
}

#[test]
fn confusion_ticks_selfhits_and_lifts() {
    // Gengar confuses Snorlax; the scripted coin says "hit yourself".
    let mut b = battle(
        mon("gengar", 50, &["confuseray"]),
        mon("snorlax", 50, &["pound"]),
    );
    let hp = b.sides[1].mon().hp;
    let events = b.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([
            PLAIN,
            SeatScript {
                selfhit: true,
                ..PLAIN
            },
        ]),
    );
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::ConfusionStarted { side: 2 })));
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::ConfusedHit { side: 2, .. })));
    assert!(!events
        .iter()
        .any(|e| matches!(e, Event::Used { side: 2, .. })));
    assert!(b.sides[1].mon().hp < hp, "the self-hit landed");
    // Next turn the clock hits zero: confusion lifts and Snorlax acts,
    // and the re-Confuse Ray fails against the still-confused target
    // (it resolved before the clock ticked).
    let events = b.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([
            PLAIN,
            SeatScript {
                selfhit: true,
                ..PLAIN
            },
        ]),
    );
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::ConfusionEnded { side: 2 })));
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Used { side: 2, .. })));
}

#[test]
fn full_paralysis_spends_the_turn_but_no_pp() {
    let mut b = battle(
        mon("blaziken", 50, &["ember"]),
        mon("treecko", 50, &["pound"]),
    );
    b.sides[0].party[0].status = Some(Status::Paralysis);
    let pp = b.sides[0].mon().moves[0].pp;
    let events = b.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([
            SeatScript {
                immobile: true,
                ..PLAIN
            },
            PLAIN,
        ]),
    );
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::FullyParalyzed { side: 1 })));
    assert!(!events
        .iter()
        .any(|e| matches!(e, Event::Used { side: 1, .. })));
    assert_eq!(
        b.sides[0].mon().moves[0].pp,
        pp,
        "full paralysis spends no PP"
    );
}

#[test]
fn toxic_ticks_grow_and_reset_on_switching_out() {
    let side1 = Side::new(alloc::vec![
        mon("snorlax", 50, &["pound"]),
        mon("mudkip", 50, &["pound"])
    ]);
    let side2 = Side::new(alloc::vec![mon("treecko", 50, &["pound"])]);
    let mut b = Battle::new(side1, side2, 3);
    b.sides[0].party[0].status = Some(Status::Toxic);
    let max = b.sides[0].mon().max_hp;
    let hp0 = b.sides[0].mon().hp;
    let miss = SeatScript {
        hit: false,
        ..PLAIN
    };
    b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([miss, miss]));
    let tick1 = hp0 - b.sides[0].mon().hp;
    assert_eq!(tick1, (max / 16).max(1), "first tick is one sixteenth");
    let hp1 = b.sides[0].mon().hp;
    b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([miss, miss]));
    assert_eq!(hp1 - b.sides[0].mon().hp, tick1 * 2, "second tick doubles");
    // Switching out resets the clock; the turn Snorlax comes back in,
    // its tick is a sixteenth again rather than a third multiple.
    b.step_with(
        [Choice::Switch(1), Choice::Move(0)],
        &scripted([miss, miss]),
    );
    let hp = b.sides[0].party[0].hp;
    b.step_with(
        [Choice::Switch(0), Choice::Move(0)],
        &scripted([miss, miss]),
    );
    assert_eq!(hp - b.sides[0].party[0].hp, tick1, "the counter restarted");
}

#[test]
fn drain_heals_half_the_damage_and_recoil_floors_a_third() {
    let mut b = battle(
        mon("blaziken", 50, &["doubleedge"]),
        mon("snorlax", 50, &["gigadrain"]),
    );
    b.sides[1].party[0].hp -= 40; // room to heal into
    let (hp1, hp2) = (b.sides[0].mon().hp, b.sides[1].mon().hp);
    let events = b.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([PLAIN, PLAIN]),
    );
    let dealt = |events: &[Event], to: u8| {
        events
            .iter()
            .find_map(|e| match e {
                Event::Damage { side, amount, .. } if *side == to => Some(*amount),
                _ => None,
            })
            .unwrap()
    };
    let (to_snorlax, to_blaziken) = (dealt(&events, 2), dealt(&events, 1));
    // Blaziken: hit by Giga Drain, and its own Double-Edge recoil.
    assert_eq!(
        b.sides[0].mon().hp,
        hp1 - to_blaziken - (to_snorlax / 3).max(1)
    );
    // Snorlax: hit by Double-Edge, healed half of its own Giga Drain.
    assert_eq!(
        b.sides[1].mon().hp,
        hp2 - to_snorlax + (to_blaziken / 2).max(1)
    );
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Recoil { side: 1, .. })));
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Drained { side: 2, .. })));
}

#[test]
fn a_multi_hit_move_strikes_the_scripted_count() {
    // Fury Attack at 4 scripted strikes: four damage events, one PP.
    let mut b = battle(
        mon("blaziken", 50, &["furyattack"]),
        mon("snorlax", 50, &["pound"]),
    );
    let miss = SeatScript {
        hit: false,
        ..PLAIN
    };
    let events = b.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([SeatScript { hits: 4, ..PLAIN }, miss]),
    );
    let strikes = events
        .iter()
        .filter(|e| matches!(e, Event::Damage { side: 2, .. }))
        .count();
    assert_eq!(strikes, 4);
    assert_eq!(
        b.sides[0].mon().moves[0].pp,
        b.sides[0].mon().moves[0].entry.pp - 1
    );

    // Double Kick is a fixed two: the script's count does not move it.
    let mut b = battle(
        mon("blaziken", 50, &["doublekick"]),
        mon("snorlax", 50, &["pound"]),
    );
    let events = b.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([SeatScript { hits: 5, ..PLAIN }, miss]),
    );
    let strikes = events
        .iter()
        .filter(|e| matches!(e, Event::Damage { side: 2, .. }))
        .count();
    assert_eq!(strikes, 2);
}

#[test]
fn status_moves_inflict_boost_and_heal() {
    // Thunder Wave: paralysis lands through the status-move path.
    let mut b = battle(
        mon("blaziken", 50, &["thunderwave"]),
        mon("snorlax", 50, &["pound"]),
    );
    let events = b.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([PLAIN, PLAIN]),
    );
    assert!(events.iter().any(|e| matches!(
        e,
        Event::Statused {
            side: 2,
            status: Status::Paralysis
        }
    )));
    assert_eq!(b.sides[1].mon().status, Some(Status::Paralysis));

    // Swords Dance doubles the next physical hit: +2 Attack stages.
    let mut b = battle(
        mon("blaziken", 50, &["swordsdance", "doublekick"]),
        mon("snorlax", 50, &["splash"]),
    );
    b.step_with(
        [Choice::Move(0), Choice::Move(1)],
        &scripted([PLAIN, PLAIN]),
    );
    assert_eq!(b.sides[0].mon().stages[Stat::Atk as usize], 2);
    let hp_before = b.sides[1].mon().hp;
    b.step_with(
        [Choice::Move(1), Choice::Move(1)],
        &scripted([PLAIN, PLAIN]),
    );
    let boosted = hp_before - b.sides[1].mon().hp;
    let mut plain = battle(
        mon("blaziken", 50, &["doublekick"]),
        mon("snorlax", 50, &["splash"]),
    );
    let hp_before = plain.sides[1].mon().hp;
    plain.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([PLAIN, PLAIN]),
    );
    let unboosted = hp_before - plain.sides[1].mon().hp;
    // Not exactly double: the flat +2 and the floors sit outside the
    // stage multiply. Meaningfully bigger is the claim.
    assert!(
        boosted > unboosted * 3 / 2,
        "+2 Atk hits harder: {boosted} vs {unboosted}"
    );

    // Recover heals half of max, capped at full.
    let mut b = battle(
        mon("blaziken", 50, &["recover"]),
        mon("snorlax", 50, &["splash"]),
    );
    let max = b.sides[0].mon().max_hp;
    b.sides[0].party[0].hp = 1;
    let events = b.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([PLAIN, PLAIN]),
    );
    assert_eq!(b.sides[0].mon().hp, 1 + max / 2);
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Healed { side: 1, .. })));

    // A scripted miss keeps Growl off the target's stages.
    let mut b = battle(
        mon("blaziken", 50, &["growl"]),
        mon("snorlax", 50, &["splash"]),
    );
    let miss = SeatScript {
        hit: false,
        ..PLAIN
    };
    b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([miss, PLAIN]));
    assert_eq!(b.sides[1].mon().stages[Stat::Atk as usize], 0);
    b.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([PLAIN, PLAIN]),
    );
    assert_eq!(b.sides[1].mon().stages[Stat::Atk as usize], -1);
}

#[test]
fn charge_moves_take_two_turns_and_recharge_costs_one() {
    // Solar Beam: turn 1 charges (one PP, no damage), turn 2 releases.
    let mut b = battle(
        mon("venusaur", 50, &["solarbeam"]),
        mon("snorlax", 50, &["splash"]),
    );
    let hp = b.sides[1].mon().hp;
    let events = b.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([PLAIN, PLAIN]),
    );
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Charging { side: 1 })));
    assert!(!events
        .iter()
        .any(|e| matches!(e, Event::Damage { side: 2, .. })));
    assert_eq!(
        b.sides[0].mon().moves[0].pp,
        b.sides[0].mon().moves[0].entry.pp - 1
    );
    let events = b.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([PLAIN, PLAIN]),
    );
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Damage { side: 2, .. })));
    assert!(b.sides[1].mon().hp < hp);
    assert_eq!(
        b.sides[0].mon().moves[0].pp,
        b.sides[0].mon().moves[0].entry.pp - 1,
        "the release costs no second PP"
    );

    // Hyper Beam: the landed hit costs the next action. (Snorlax's own
    // bulk keeps the target alive to see the recharge.)
    let mut b = battle(
        mon("snorlax", 50, &["hyperbeam"]),
        mon("snorlax", 50, &["splash"]),
    );
    b.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([PLAIN, PLAIN]),
    );
    let events = b.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([PLAIN, PLAIN]),
    );
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Recharging { side: 1 })));
    assert!(!events
        .iter()
        .any(|e| matches!(e, Event::Used { side: 1, .. })));
    // And the turn after, it attacks again.
    let events = b.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([PLAIN, PLAIN]),
    );
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Used { side: 1, .. })));
}

#[test]
fn semi_invulnerability_dodges_and_earthquake_pierces_dig_doubled() {
    // Mid-Dig, Tackle whiffs without even rolling accuracy.
    let mut b = battle(
        mon("sandslash", 50, &["dig"]),
        mon("snorlax", 50, &["tackle"]),
    );
    let events = b.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([PLAIN, PLAIN]),
    );
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Charging { side: 1 })));
    assert!(!events
        .iter()
        .any(|e| matches!(e, Event::Damage { side: 1, .. })));
    assert_eq!(b.sides[0].mon().hp, b.sides[0].mon().max_hp);

    // Mid-Dig, Earthquake connects — at double power.
    let mut plain = battle(
        mon("snorlax", 50, &["earthquake"]),
        mon("sandslash", 50, &["splash"]),
    );
    let hp = plain.sides[1].mon().hp;
    plain.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([PLAIN, PLAIN]),
    );
    let normal_hit = hp - plain.sides[1].mon().hp;

    let mut b = battle(
        mon("sandslash", 50, &["dig"]),
        mon("snorlax", 50, &["earthquake"]),
    );
    // Snorlax is slower: Sandslash digs, then Earthquake lands doubled.
    assert!(b.sides[0].mon().spe > b.sides[1].mon().spe);
    let hp = b.sides[0].mon().hp;
    b.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([PLAIN, PLAIN]),
    );
    let pierced = hp - b.sides[0].mon().hp;
    assert!(
        pierced > normal_hit * 3 / 2,
        "doubled: {pierced} vs {normal_hit}"
    );
}

#[test]
fn screens_halve_safeguard_shields_and_mist_holds_stages() {
    // Reflect roughly halves a physical hit.
    let mut plain = battle(
        mon("snorlax", 50, &["tackle"]),
        mon("chansey", 50, &["splash"]),
    );
    let hp = plain.sides[1].mon().hp;
    plain.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([PLAIN, PLAIN]),
    );
    let open_hit = hp - plain.sides[1].mon().hp;

    let mut b = battle(
        mon("snorlax", 50, &["tackle"]),
        mon("chansey", 50, &["reflect"]),
    );
    assert!(
        b.sides[1].mon().spe > b.sides[0].mon().spe,
        "chansey screens first"
    );
    b.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([PLAIN, PLAIN]),
    );
    let hp = b.sides[1].mon().hp;
    b.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([PLAIN, PLAIN]),
    );
    let screened = hp - b.sides[1].mon().hp;
    assert!(
        screened < open_hit * 2 / 3,
        "reflected: {screened} vs {open_hit}"
    );

    // Safeguard blocks Thunder Wave for the whole team. (Snorlax is
    // slower than Chansey, so the shield is up before the wave.)
    let mut b = battle(
        mon("snorlax", 50, &["thunderwave"]),
        mon("chansey", 50, &["safeguard"]),
    );
    b.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([PLAIN, PLAIN]),
    );
    b.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([PLAIN, PLAIN]),
    );
    assert_eq!(b.sides[1].mon().status, None);

    // Mist holds Growl off.
    let mut b = battle(
        mon("snorlax", 50, &["growl"]),
        mon("chansey", 50, &["mist"]),
    );
    b.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([PLAIN, PLAIN]),
    );
    b.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([PLAIN, PLAIN]),
    );
    assert_eq!(b.sides[1].mon().stages[Stat::Atk as usize], 0);
}

#[test]
fn recoil_can_knock_the_user_out() {
    let side1 = Side::new(alloc::vec![
        mon("blaziken", 100, &["doubleedge"]),
        mon("mudkip", 50, &["pound"])
    ]);
    let side2 = Side::new(alloc::vec![mon("snorlax", 100, &["pound"])]);
    let mut b = Battle::new(side1, side2, 3);
    b.sides[0].party[0].hp = 1;
    let miss = SeatScript {
        hit: false,
        ..PLAIN
    };
    let events = b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, miss]));
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::Fainted { side: 1 })));
    assert_eq!(b.sides[0].active, 1, "the bench replaced the recoil faint");
}

#[test]
fn a_fire_hit_thaws_the_target_but_its_burn_chance_is_blocked() {
    let mut b = battle(
        mon("blaziken", 50, &["ember"]),
        mon("treecko", 50, &["pound"]),
    );
    b.sides[1].party[0].status = Some(Status::Freeze);
    let events = b.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([
            SeatScript {
                secondary: true,
                ..PLAIN
            },
            PLAIN,
        ]),
    );
    assert_eq!(b.sides[1].mon().status, None, "the freeze thawed");
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Event::Statused { side: 2, .. })),
        "the burn chance was blocked by the freeze it cured"
    );
}

#[test]
fn the_same_seed_replays_the_same_battle() {
    let run = || {
        let mut b = battle(
            mon("blaziken", 50, &["ember"]),
            mon("treecko", 50, &["pound"]),
        );
        let mut all = Vec::new();
        for _ in 0..4 {
            all.extend(b.step([Choice::Move(0), Choice::Move(0)]));
        }
        all
    };
    assert_eq!(run(), run(), "a seeded battle has to be reproducible");
}

/// Two dozen of this era's damaging moves carry 0 in the power column and
/// work it out in a callback. Reading that column as "not a damaging move"
/// classified every one of them as Status, so they spent their PP, produced
/// no event, and did nothing at all. The category is DATA now; this pins the
/// ones with no other arm to fall back on.
#[test]
fn the_callback_power_moves_actually_hit() {
    for id in [
        "return",
        "frustration",
        "hiddenpower",
        "flail",
        "reversal",
        "lowkick",
        "magnitude",
    ] {
        let mut b = battle(mon("blaziken", 100, &[id]), mon("snorlax", 100, &["splash"]));
        assert!(
            b.sides[0].mon().moves[0].entry.damaging,
            "{id} is not marked damaging",
        );
        let before = b.sides[1].mon().hp;
        let events = b.step([Choice::Move(0), Choice::Move(0)]);
        assert!(
            b.sides[1].mon().hp < before,
            "{id} did nothing: {events:?}",
        );
    }
}
