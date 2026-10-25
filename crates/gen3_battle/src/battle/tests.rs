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

/// A Hidden Power goes through the chart as whatever its holder's IVs typed
/// it, but gen 3's `frz.onDamagingHit` asks `dex.moves.get(move.id).type`,
/// which is the TABLE type and so Normal. An IV-typed Fire thaws nobody,
/// which is the whole of the difference between the two readings.
#[test]
fn a_hidden_power_fire_does_not_thaw_the_target_it_hits() {
    let spread = [30, 30, 31, 30, 30, 31].map(|iv| Invest { iv, ev: 0 });
    let attacker =
        Mon::with_spread("weezing", 50, Nature::Hardy, &spread, &["hiddenpower"]).unwrap();
    assert_eq!(attacker.hidden_power, Type::Fire, "these IVs type it Fire");
    let mut target = mon("azumarill", 50, &["bide"]);
    target.status = Some(Status::Freeze);
    let mut b = Battle::new(
        Side::new(alloc::vec![attacker]),
        Side::new(alloc::vec![target]),
        11,
    );
    let events = b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::Damage { side: 2, .. })),
        "the Hidden Power landed"
    );
    assert_eq!(
        b.sides[1].mon().status,
        Some(Status::Freeze),
        "and left the frozen target alone"
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

/// The sim answers `moveSlots = baseMoveSlots.slice()` when a mon leaves
/// the field, and neither Mimic nor Transform writes `baseMoveSlots`: Mimic
/// copies into the slot holding it and Transform replaces the list outright.
/// A mon that Mimicked and then Transformed therefore has to put the
/// Mimic's slot back into the list as it stood BEFORE the Transform, which
/// is the list the Transform's own copy is holding.
#[test]
fn a_mimic_that_transformed_after_it_comes_back_with_its_own_moves() {
    let move_ids = |b: &Battle| -> Vec<&'static str> {
        b.sides[0].party[0].moves.iter().map(|m| m.entry.id).collect()
    };
    let mut b = Battle::new(
        Side::new(alloc::vec![
            mon("ditto", 50, &["splash", "tackle", "mimic", "transform"]),
            mon("chansey", 50, &["softboiled"]),
        ]),
        // One move on the other side, so the Transform's copy is one move
        // long and a Mimic index of two points off its end.
        Side::new(alloc::vec![mon("snorlax", 50, &["growl"])]),
        5,
    );
    let base = move_ids(&b);
    // Turn one is the foe's Growl; only then is there a move to copy.
    b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
    b.step_with([Choice::Move(2), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
    assert_eq!(b.sides[0].mon().moves[2].entry.id, "growl");
    b.step_with([Choice::Move(3), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
    assert_eq!(b.sides[0].mon().moves.len(), 1, "the copy is the foe's list");
    b.step_with([Choice::Switch(1), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
    assert_eq!(move_ids(&b), base, "the mon left with the set it came out with");
}

/// The other order, which play cannot reach (a transformed mon cannot Mimic)
/// and the restore still has to hold: the Transform's copy is taken off
/// first, so a Mimic index is read against the list it was found in rather
/// than against the copy standing in its place.
#[test]
fn a_transform_copy_is_unwound_before_the_mimic_slot_is_put_back() {
    let move_ids = |b: &Battle| -> Vec<&'static str> {
        b.sides[0].party[0].moves.iter().map(|m| m.entry.id).collect()
    };
    let mut b = Battle::new(
        Side::new(alloc::vec![
            mon("ditto", 50, &["splash", "tackle", "mimic", "transform"]),
            mon("chansey", 50, &["softboiled"]),
        ]),
        Side::new(alloc::vec![mon("snorlax", 50, &["growl"])]),
        5,
    );
    let base = move_ids(&b);
    let growl = MoveSlot {
        entry: crate::data::move_by_id("growl").unwrap(),
        pp: 5,
        typed_as: None,
    };
    let mon = b.sides[0].mon_mut();
    mon.transform_backup = Some(mon.moves.clone());
    mon.moves = alloc::vec![growl];
    mon.mimic_backup = Some((2, mon.transform_backup.as_ref().unwrap()[2]));
    b.step_with([Choice::Switch(1), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
    assert_eq!(move_ids(&b), base, "the mon left with the set it came out with");
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

/// A White Herb answers an Intimidate out of `runMove`'s `AfterMove` event,
/// so an action that never reached the move leaves the drop standing. The
/// foe's turn lost to its own freeze was the case: the herb came back early,
/// the Intimidate it was holding never touched a Vital Throw, and the first
/// hit of the battle came out of full Attack instead of -1.
///
/// Deoxys is the faster of the two, so it switched in FIRST and its own item
/// started before Intimidate landed: the drop is there when turn one opens,
/// and it is still there when Deoxys swings.
#[test]
fn a_turn_lost_to_its_own_status_does_not_answer_a_white_herb() {
    let hit_with = |herb: bool, cowed: bool| {
        let mut arcanine = mon("arcanine", 63, &["mistball"]);
        if cowed {
            arcanine.ability = "intimidate";
        }
        arcanine.status = Some(Status::Freeze);
        let mut deoxys = mon("deoxys", 63, &["vitalthrow"]);
        deoxys.item = if herb { "whiteherb" } else { "" };
        let mut b = Battle::new(
            Side::new(alloc::vec![arcanine]),
            Side::new(alloc::vec![deoxys]),
            1,
        );
        b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([PLAIN, SeatScript { random: 94, ..PLAIN }]),
        );
        b.sides[0].mon().max_hp - b.sides[0].mon().hp
    };
    // 41, which is what the reference sim deals: Deoxys's Attack reads 142
    // off the -1 Intimidate left it (Gen 3's -1 is two thirds, not a half)
    // against Arcanine's 125 Defense, and Vital Throw is neutral on Fire.
    assert_eq!(hit_with(true, true), 41);
    assert_eq!(
        hit_with(false, true),
        41,
        "nothing was there to restore the drop, and it did not need one"
    );
    assert_eq!(hit_with(false, false), 62, "an uncowed Deoxys swings at full Attack");
    assert_eq!(
        hit_with(true, false),
        62,
        "and an herb with nothing to restore costs its holder nothing"
    );
}

#[test]
fn a_chosen_replacement_takes_the_turns_residual() {
    // The sim asks for the replacement at the action boundary and runs the
    // residual phase after it, so a poisoned pick takes this turn's tick
    // whether the engine chose it or the player did.
    let build = |picks: bool| {
        let mut sick = mon("blaziken", 100, &["ember"]);
        sick.status = Some(Status::Poison);
        let mut b = Battle::new(
            Side::new(alloc::vec![mon("magikarp", 5, &["splash"]), sick]),
            Side::new(alloc::vec![mon("blaziken", 100, &["ember"])]),
            7,
        );
        b.player_picks_replacement = picks;
        b
    };
    let mut auto = build(false);
    auto.step([Choice::Move(0), Choice::Move(0)]);
    let mut chosen = build(true);
    let mut events = chosen.step([Choice::Move(0), Choice::Move(0)]);
    assert!(chosen.pending_replacements()[0].is_some());
    assert!(!events.iter().any(|e| matches!(e, Event::Residual { .. })));
    chosen.send_out([Some(1), None], &mut events);
    let full = auto.sides[0].mon().max_hp;
    assert!(auto.sides[0].mon().hp < full, "the automatic pick missed its tick");
    assert_eq!(chosen.sides[0].mon().hp, auto.sides[0].mon().hp);
    assert!(events.iter().any(|e| matches!(e, Event::Residual { side: 1, .. })));
}

/// Charge's volatile is dropped by its own `onAfterMove`, and an intercepting
/// Pursuit is fired from the switch-out hook through `useMove`, below
/// `runMove`, which is the only thing that runs that event. So the charge
/// rides straight through an interception, while any move that actually goes
/// off spends it.
#[test]
fn a_pursuit_interception_leaves_the_charge_alone() {
    let still_charged = |second: Choice| {
        let mut b = Battle::new(
            Side::new(alloc::vec![mon("raikou", 50, &["charge", "pursuit", "surf"])]),
            Side::new(alloc::vec![
                mon("gyarados", 50, &["splash"]),
                mon("chansey", 50, &["softboiled"]),
            ]),
            3,
        );
        b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
        assert!(b.sides[0].mon().charged_elec, "the Charge went up");
        b.step_with([second, Choice::Switch(1)], &scripted([PLAIN, PLAIN]));
        b.sides[0].mon().charged_elec
    };
    assert!(
        still_charged(Choice::Move(1)),
        "the interception reaches no AfterMove, so nothing spends the charge"
    );
    assert!(
        !still_charged(Choice::Move(2)),
        "and a move that does go off spends it, Electric or not"
    );
}

/// Rage's volatile hangs its `onHit` off the sim's `Hit` event, not off
/// damage, so anything that reaches the end of the hit loop aimed at a rager
/// counts for it: a Present that handed its gift back lands exactly as hard
/// on the count as one that took HP off, because `runMoveEffects` heals and
/// then runs the same event. Reading it off damage left a rager untouched by
/// a Present, and the two sides of that battle drifted a point of damage at
/// a time apart.
///
/// Tauros is faster, so its Rage is already volatile when the gift arrives.
/// The rival's Present is the whole of the test: a mon at full health finds
/// nothing to give, the gift fails before the event, and nothing climbs.
#[test]
fn a_raging_target_reads_its_attack_off_the_hit_event() {
    let gained = |healed_first: bool| {
        let mut b = battle(mon("tauros", 50, &["rage"]), mon("blissey", 50, &["present"]));
        if healed_first {
            b.sides[0].mon_mut().hp = b.sides[0].mon().max_hp - 10;
        }
        // Present draws its gift or one of three damage tiers off a roll of
        // its own; the script's secondary knob is the pinned floor, which is
        // the gift.
        b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([PLAIN, SeatScript { secondary: true, ..PLAIN }]),
        );
        b.sides[0].mon().stages[Stat::Atk as usize]
    };
    assert_eq!(
        gained(true),
        1,
        "a Present whose gift lands is a hit, and a rager reads its Attack off that"
    );
    assert_eq!(
        gained(false),
        0,
        "a Present refused at full health never reaches the event, so nothing climbs"
    );
}

/// The sim floors a base power that arrives under one BEFORE its `BasePower`
/// event and floors nothing after it: `if (!basePower) return 0`, a move
/// that connects and takes nothing. Eruption on a sliver of health is the
/// one power that gets there, and Thick Fat is what halves it; a padding
/// back to one invented a point of damage the reference never deals.
#[test]
fn a_base_power_the_chain_floors_to_zero_takes_nothing() {
    let dealt = |thick_fat: bool, hp: u16| {
        let mut seel = mon("seel", 50, &["splash"]);
        if thick_fat {
            seel.ability = "thickfat";
        }
        let mut b = battle(mon("typhlosion", 50, &["eruption"]), seel);
        b.sides[0].mon_mut().hp = hp;
        b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
        b.sides[1].mon().max_hp - b.sides[1].mon().hp
    };
    // Typhlosion at 1 of its HP is one base power of Eruption, and Thick Fat
    // halves a Fire move: 150 * 1 / 130 floors to one, and one halved is
    // nothing at all.
    assert_eq!(dealt(true, 1), 0, "the chain floored the power to zero");
    assert_eq!(
        dealt(true, 2),
        0,
        "and at two HP the power is still just one before the halving"
    );
    assert!(
        dealt(false, 2) > 0,
        "without Thick Fat to halve it, the same sliver of health still burns"
    );
}

/// A pinch berry heals its holder, and if that holder's nature lowers the
/// stat whose flavour it carries, the heal comes with confusion. The sim
/// hangs it off the berry's own onEat: `addVolatile("confusion")` on the
/// eater, which a Safeguard does not reach (it spares only foe-inflicted
/// effects), an Own Tempo refuses, and a liked flavour does nothing.
#[test]
fn a_pinch_berry_confuses_its_disliked_flavour() {
    // Lax lowers Sp.Def, so the bitterness Aguav carries is the flavour it
    // dislikes; Lonely lowers Atk, so Figy's spice is fine with it.
    let holder = |berry: &'static str, nature: Nature| {
        let mut m = Mon::new("snorlax", 50, nature, Invest { iv: 31, ev: 0 }, &["splash"])
            .unwrap_or_else(|| panic!("snorlax is not in the dex"));
        m.item = berry;
        m
    };
    let scripted_eaten = |berry: &'static str, nature: Nature| {
        let mut b = battle(
            holder(berry, nature),
            mon("treecko", 50, &["splash"]),
        );
        // At half, exactly where this era's berries ripen.
        b.sides[0].mon_mut().hp = b.sides[0].mon().max_hp / 2;
        let at = b.sides[0].mon().hp;
        let events = b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([PLAIN, PLAIN]),
        );
        assert_eq!(b.sides[0].mon().item, "", "the {berry} was eaten");
        assert_eq!(b.sides[0].mon().hp, at + (b.sides[0].mon().max_hp / 8).max(1));
        (b, events)
    };

    // Aguav on Lax muddles as it heals: the sim's onStart pins random(2, 6)
    // to its floor, so the clock is exactly 2.
    let (b, events) = scripted_eaten("aguavberry", Nature::Lax);
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::ConfusionStarted { side: 1 })));
    assert_eq!(b.sides[0].mon().confusion_n, 2);

    // Figy on Lax only heals: Spicy offends Atk, and Lax lowers nothing.
    let (b, events) = scripted_eaten("figyberry", Nature::Lax);
    assert!(!events
        .iter()
        .any(|e| matches!(e, Event::ConfusionStarted { side: 1 })));
    assert_eq!(b.sides[0].mon().confusion_n, 0);

    // A neutral nature lowers NO stat, even though `effect()` reports its
    // raised stat in both slots: Mago's dryness never muddles a Serious
    // holder (the seed-23 shape), nor Hardy a Figy, Docile an Iapapa,
    // Bashful a Wiki, Quirky an Aguav.
    for (berry, nature) in [
        ("magoberry", Nature::Serious),
        ("figyberry", Nature::Hardy),
        ("iapapaberry", Nature::Docile),
        ("wikiberry", Nature::Bashful),
        ("aguavberry", Nature::Quirky),
    ] {
        let (b, events) = scripted_eaten(berry, nature);
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, Event::ConfusionStarted { side: 1 })),
            "{} confused a {} holder",
            berry,
            nature.name(),
        );
        assert_eq!(b.sides[0].mon().confusion_n, 0);
    }
}

#[test]
fn a_chosen_baton_pass_target_takes_the_passed_stages() {
    // The sim asks the passer's side who comes in once the move is over, and
    // the slower foe's move waits for the answer. Whoever is picked inherits
    // the stages; the mon passed over keeps none of them.
    let build = |picks: bool| {
        let mut passer = mon("jolteon", 100, &["batonpass"]);
        passer.stages[Stat::Atk as usize] = 2;
        passer.eva_stage = 1;
        let mut b = Battle::new(
            Side::new(alloc::vec![
                passer,
                mon("magikarp", 50, &["splash"]),
                mon("blaziken", 50, &["ember"]),
            ]),
            Side::new(alloc::vec![mon("slowbro", 50, &["splash"])]),
            7,
        );
        b.player_picks_replacement = picks;
        b
    };
    let mut auto = build(false);
    auto.step([Choice::Move(0), Choice::Move(0)]);
    assert_eq!(auto.sides[0].active, 1, "the engine's own pick is the lowest slot");

    let mut chosen = build(true);
    let mut events = chosen.step([Choice::Move(0), Choice::Move(0)]);
    assert_eq!(chosen.pending_replacements()[0], Some(alloc::vec![1, 2]));
    assert_eq!(chosen.pending_pass(), [true, false]);
    assert!(
        !events.iter().any(|e| matches!(e, Event::Used { side: 2, .. })),
        "the foe's move waits for the answer"
    );
    chosen.send_out([Some(2), None], &mut events);
    assert!(chosen.pending_replacements()[0].is_none());
    assert_eq!(chosen.sides[0].active, 2);
    assert_eq!(chosen.sides[0].mon().stages[Stat::Atk as usize], 2);
    assert_eq!(chosen.sides[0].mon().eva_stage, 1);
    assert_eq!(chosen.sides[0].party[1].stages, [0; 5]);
    assert!(events.iter().any(|e| matches!(e, Event::Used { side: 2, .. })));
}

/// A Focus Band clamps a lethal blow to a point, and a clamp is still a
/// CONNECTED hit: the sim's `selfDrops` skips only the targets a move failed
/// on, and zero damage is not a failure. So the Hyper Beam still costs the
/// attacker its next action, which is what seeds 46 and 18 turned on.
#[test]
fn a_band_clamped_hit_still_costs_the_recharge() {
    let mut b = battle(
        mon("blaziken", 50, &["hyperbeam"]),
        mon("snorlax", 50, &["splash"]),
    );
    let mon = b.sides[1].mon_mut();
    mon.item = "focusband".into();
    mon.hp = 1;
    b.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([PLAIN, SeatScript { band: true, ..PLAIN }]),
    );
    assert_eq!(b.sides[1].mon().hp, 1, "the band held the mon alive");
    assert!(
        b.sides[0].mon().must_recharge,
        "a Hyper Beam the band ate still costs the next turn"
    );
}

/// Endure is stripped before a delayed hit resolves (`futuremove`'s `onEnd`),
/// but the band is not: the release goes through `trySpreadMoveHit`, so it
/// runs the same Damage event an ordinary move does.
#[test]
fn a_future_sight_lands_through_the_band() {
    let mut b = battle(
        mon("mew", 50, &["futuresight"]),
        mon("snorlax", 50, &["splash"]),
    );
    let mon = b.sides[1].mon_mut();
    mon.item = "focusband".into();
    mon.hp = 1;
    // The release lands on the third residual counting from the launch.
    for _ in 0..3 {
        b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([PLAIN, SeatScript { band: true, ..PLAIN }]),
        );
    }
    assert_eq!(b.sides[1].mon().hp, 1, "the delayed blow asked the band");
    assert!(!b.sides[1].mon().fainted());
}

/// Gen 3's `tryMoveHit` answers false on a miss and never touches the
/// `lockedmove` volatile, and the `onAfterMove` that would re-arm its clock
/// is void this era. The lock runs out at the residual instead, which is
/// where the fatigue confusion comes from.
#[test]
fn a_locked_move_that_misses_keeps_its_lock() {
    let mut b = battle(
        mon("blaziken", 50, &["outrage"]),
        mon("snorlax", 50, &["splash"]),
    );
    b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
    let miss = SeatScript {
        hit: false,
        ..PLAIN
    };
    let events = b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([miss, PLAIN]));
    let pos = |want: fn(&Event) -> bool| events.iter().position(want);
    let foe_moved = pos(|e| matches!(e, Event::Used { side: 2, .. }));
    let fatigued = pos(|e| matches!(e, Event::ConfusionStarted { side: 1 }));
    assert!(
        b.sides[0].mon().rampage.is_none(),
        "the lock ran out at the residual"
    );
    assert!(
        fatigued.is_some() && foe_moved.is_some() && foe_moved < fatigued,
        "the lock outlived the miss: the foe's whole turn came first, \
         the fatigue confusion only afterwards"
    );
}

/// A rolling lock counts its strikes in the sim's base-power callback, which
/// runs while the damage is COMPUTED, before the Damage event an Endure or a
/// Focus Band clamps it. So a blow held at one HP counts all the same, and
/// the `iceball` volatile comes back with `duration: 2`, whose `onLockMove`
/// hands the request its `trapped` flag: the attacker cannot leave the field.
#[test]
fn a_rolling_lock_survives_a_blow_clamped_to_nothing() {
    // Snorlax is the slower of the two, so the Endure is already up when
    // its Ice Ball lands; the band asks its tenth out of the target's script.
    let clamped = |endure: bool| {
        let mut foe = mon(
            "blaziken",
            50,
            if endure { &["endure"] } else { &["splash"] },
        );
        foe.item = if endure { "" } else { "focusband" }.into();
        let mut b = Battle::new(
            Side::new(alloc::vec![
                mon("snorlax", 50, &["iceball"]),
                mon("mudkip", 50, &["splash"])
            ]),
            Side::new(alloc::vec![foe]),
            42,
        );
        b.sides[1].mon_mut().hp = 1;
        b.step_with(
            [Choice::Move(0), Choice::Move(0)],
            &scripted([PLAIN, SeatScript { band: true, ..PLAIN }]),
        );
        assert_eq!(b.sides[1].mon().hp, 1, "the blow was clamped to nothing");
        b
    };
    for endure in [false, true] {
        let b = clamped(endure);
        assert_eq!(
            b.sides[0].mon().rolling,
            Some(1),
            "the strike counted anyway"
        );
        assert!(
            !b.can_switch(0),
            "and the lock holds the field against a switch"
        );
    }
}

/// `transformInto` answers both halves of the Hidden Power with the target's
/// before gen 5, so a copy swings the power of the thing it is wearing
/// rather than the one it was born with, and gives it back on the way out.
#[test]
fn a_transformed_mon_answers_with_the_hidden_power_it_wears() {
    let spread = |iv: u8| [Invest { iv, ev: 0 }; 6];
    let mut b = Battle::new(
        Side::new(alloc::vec![
            Mon::with_spread("ditto", 50, Nature::Hardy, &spread(28), &["transform"])
                .unwrap(),
            mon("magikarp", 50, &["splash"]),
        ]),
        Side::new(alloc::vec![
            Mon::with_spread("blaziken", 50, Nature::Hardy, &spread(31), &["ember"]).unwrap(),
        ]),
        9,
    );
    let foe = b.sides[1].mon().clone();
    let own = b.sides[0].mon().clone();
    b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
    let copy = b.sides[0].mon();
    assert_eq!(copy.hidden_power, foe.hidden_power);
    assert_eq!(copy.hidden_power_bp, foe.hidden_power_bp);
    assert_ne!(
        own.hidden_power_bp, foe.hidden_power_bp,
        "these IVs differ, so the copy has to be the foe's"
    );
    b.step_with([Choice::Switch(1), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
    let back = &b.sides[0].party[0];
    assert_eq!(back.hidden_power, own.hidden_power, "the copy came off");
    assert_eq!(back.hidden_power_bp, own.hidden_power_bp);
}

/// The sim builds its turn out of `getMoves`, which greys out the last move
/// of a mon under Torment and hands a mon with nothing usable left a
/// Struggle. `beforeTurnMove` and the Pursuit volatile hang off the action as
/// it was queued, so a pursuiter that was going to Struggle never arms one
/// and there is nothing to intercept: the switch goes through untouched and
/// the Struggle lands on the mon that came in.
///
/// The interception read the chosen SLOT instead of the action, so it fired
/// anyway, at double power, on the mon on its way out.
#[test]
fn a_pursuiter_going_to_struggle_never_intercepts_the_switch() {
    let mut b = Battle::new(
        Side::new(alloc::vec![mon("houndoom", 50, &["pursuit"])]),
        Side::new(alloc::vec![
            mon("voltorb", 50, &["torment"]),
            mon("chansey", 50, &["softboiled"]),
        ]),
        5,
    );
    b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
    assert!(b.sides[0].mon().tormented, "the Torment landed");
    let leaving = b.sides[1].party[0].hp;
    let arriving = b.sides[1].party[1].hp;
    let events = b.step_with(
        [Choice::Move(0), Choice::Switch(1)],
        &scripted([PLAIN, PLAIN]),
    );
    let switched = events
        .iter()
        .position(|e| matches!(e, Event::Switched { side: 2, party_index: 1 }));
    let struck = events
        .iter()
        .position(|e| matches!(e, Event::Damage { side: 2, .. }));
    assert!(
        struck.is_none() || switched < struck,
        "nothing struck the field before the switch: {events:?}",
    );
    assert_eq!(
        b.sides[1].party[0].hp,
        leaving,
        "the mon on its way out took no hit",
    );
    assert_eq!(b.sides[1].active, 1);
    assert!(
        b.sides[1].mon().hp < arriving,
        "the Struggle went to the mon that came in"
    );
}

/// Rage's volatile rides on `self.volatileStatus`, which the sim adds from
/// `selfDrops` once the hit is done. That walk skips only a `false` target,
/// and a Substitute NULLS its target rather than removing it from the list,
/// so a Rage the doll swallowed still arms the volatile: the next hit the
/// rager takes raises its Attack, exactly as one taken straight on the rager
/// would have.
///
/// Reading the arming off the damage left the rager cold for a whole turn,
/// and every hit it took after that came out a point low.
#[test]
fn a_rage_the_substitute_swallowed_still_arms_the_rager() {
    let mut b = battle(
        mon("tauros", 50, &["tackle", "rage"]),
        mon("blissey", 50, &["substitute", "tackle"]),
    );
    assert!(b.sides[0].mon().spe > b.sides[1].mon().spe);
    b.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([PLAIN, PLAIN]),
    );
    assert!(b.sides[1].mon().sub_hp > 0, "the doll is up");
    let events = b.step_with(
        [Choice::Move(1), Choice::Move(1)],
        &scripted([PLAIN, PLAIN]),
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::SubDamage { side: 2, .. })),
        "the Rage went into the doll: {events:?}",
    );
    assert!(b.sides[1].mon().sub_hp > 0, "and the doll is still standing");
    assert_eq!(
        b.sides[0].mon().stages[Stat::Atk as usize],
        1,
        "the hit that followed raised the rager's Attack",
    );
}


/// A bind outlives the mon that threw it, and the sim deletes it in its own
/// residual: `onResidual` finds the source off the field and clears the
/// volatile without a chip. A Rapid Spin in that same turn takes the clock
/// to zero of its own accord, and the volatile IT deletes is gone just the
/// same, so the stale flag has to be spent at that residual too. Left lying
/// about the source, it ate the first chip of the NEXT bind.
///
/// That is the wide-fuzz seed-2170 shape: Murkrow's Whirlpool binds a
/// Castform that spins itself free, leaves, comes back and binds again, and
/// the second Whirlpool's victim only started bleeding once the flag was.
#[test]
fn a_bind_left_by_a_departed_user_does_not_eat_the_next_chips() {
    let mut b = Battle::new(
        Side::new(alloc::vec![mon("heracross", 50, &["tackle", "rapidspin"])]),
        Side::new(alloc::vec![
            mon("murkrow", 50, &["whirlpool"]),
            mon("beldum", 50, &["splash"]),
        ]),
        7,
    );
    // The Whirlpool binds, and the bind's own residual chips.
    let first = b.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([PLAIN, PLAIN]),
    );
    assert!(
        first
            .iter()
            .any(|e| matches!(e, Event::TrapDamage { side: 1, .. })),
        "the bind should have bitten: {first:?}",
    );
    // The trapper leaves while the bind still has clock on it, and the Rapid
    // Spin that follows empties it. There is nothing left to chip.
    let second = b.step_with(
        [Choice::Move(1), Choice::Switch(1)],
        &scripted([PLAIN, PLAIN]),
    );
    assert_eq!(b.sides[0].mon().trapped_n, 0, "the spin flung it off");
    assert!(
        !second
            .iter()
            .any(|e| matches!(e, Event::TrapDamage { side: 1, .. })),
        "a bind with no clock left cannot bite: {second:?}",
    );
    // Murkrow comes back and binds again, and this time nothing clears the
    // clock mid-turn: the first chip is the one a stale flag would swallow.
    b.step_with([Choice::Move(0), Choice::Switch(0)], &scripted([PLAIN, PLAIN]));
    let third = b.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([PLAIN, PLAIN]),
    );
    assert!(
        third
            .iter()
            .any(|e| matches!(e, Event::TrapDamage { side: 1, .. })),
        "the fresh bind should have bitten: {third:?}",
    );
}

/// `eachEvent('Update')` runs inside `tryMoveHit`, right after the blow, so
/// a Leppa Berry is eaten the moment a slot runs dry. Waiting for the top of
/// the next turn costs the restore whenever that blow ended the battle, which
/// is the wide-fuzz seed-2311 shape: an Extreme Speed's last point of PP went
/// out with Exploud.
#[test]
fn a_leppa_answers_the_blow_that_empties_the_slot() {
    let mut holder = mon("pinsir", 50, &["extremespeed"]);
    holder.item = "leppaberry";
    holder.moves[0].pp = 1;
    let mut b = battle(holder, mon("snorlax", 50, &["tackle"]));
    // The blow ends the battle, so there is no next turn to wait for.
    b.sides[1].mon_mut().hp = 1;
    let events = b.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([PLAIN, PLAIN]),
    );
    assert!(b.over(), "the battle ended on that blow: {events:?}");
    assert_eq!(b.sides[0].mon().item, "", "the berry was eaten");
    assert_eq!(
        b.sides[0].mon().moves[0].pp,
        5,
        "ten points back, capped at the move's own five",
    );
}

/// A mon under Torment has its last move greyed out at the REQUEST, so
/// `getMoves` comes back empty and `chooseMove` queues it a Struggle. An
/// Encore that lands over it in the same turn has nothing to redirect: the
/// sim runs `OverrideAction` on the queued move and skips it for Struggle
/// (`baseMove.id !== "struggle"`), so the Struggle is what goes off and what
/// becomes `lastMove`. That last move is what the NEXT turn's Torment reads,
/// so the encored move is free again a turn later.
///
/// Redirecting the Struggle made the victim swing the encored move instead,
/// which also left it with that move as `lastMove` and Torment greying it out
/// for the turn after, one fight out of step with the reference.
#[test]
fn an_encore_over_a_strruggle_leaves_the_struggle_alone() {
    // The same two turns with and without the Encore over them: a Struggle
    // either way on the second, and Pound again on the third.
    let turns = |encore: bool| {
        let mut b = battle(
            mon("blaziken", 50, &["torment", if encore { "encore" } else { "splash" }]),
            mon("treecko", 50, &["pound"]),
        );
        b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
        assert!(b.sides[1].mon().tormented, "the Torment landed");
        let events = b.step_with([Choice::Move(1), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
        let hit = events
            .iter()
            .find_map(|e| match e {
                Event::Damage { side: 1, amount, .. } => Some(*amount),
                _ => None,
            })
            .expect("the victim swung at something");
        let after_encore = b.sides[1].mon().last_used_id;
        b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
        (hit, after_encore, b.sides[1].mon().last_used_id)
    };
    let (struggle, used, again) = turns(false);
    let (same, used_under_encore, again_under_encore) = turns(true);
    assert_eq!(same, struggle, "the Encore changed what the victim swung");
    assert_eq!(used_under_encore, Some("struggle"), "{used:?} without it");
    assert_eq!(again_under_encore, Some("pound"), "{again:?} without it");
}

/// Beat Up rallies everyone on the user's side who is neither fainted nor
/// statused, in `side.pokemon` order. That list is not the party's: every
/// switch swaps the incoming mon into the front slot and pushes the outgoing
/// one into the slot it came from, so two switches leave the bench in an
/// order the party cannot name. Each strike swings the NEXT ally's base
/// Attack, so the order is the damage.
///
/// Walking the party instead struck the bench off the party order, which put
/// every strike after the first on the wrong ally's Attack.
#[test]
fn beat_up_strikes_in_the_sims_own_party_order() {
    let mut b = Battle::new(
        Side::new(alloc::vec![
            mon("snorlax", 50, &["splash"]),
            mon("gardevoir", 50, &["beatup"]),
            mon("gloom", 50, &["splash"]),
            mon("hoothoot", 50, &["splash"]),
        ]),
        Side::new(alloc::vec![mon("krabby", 50, &["splash"])]),
        42,
    );
    // Gloom out, then Gardevoir: the sim's list then reads Gardevoir, Gloom,
    // Snorlax, while the party order would answer Gardevoir, Snorlax, Gloom.
    // Hoothoot never comes in and is not fainted, so it rallies too; burn it
    // so three strikes are left.
    b.step_with([Choice::Switch(2), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
    b.step_with([Choice::Switch(1), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
    b.sides[0].party[3].status = Some(Status::Burn);
    assert_eq!(b.sides[0].active, 1);
    let events = b.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([SeatScript { random: 100, ..PLAIN }, PLAIN]),
    );
    let strikes: Vec<u16> = events
        .iter()
        .filter_map(|e| match e {
            Event::Damage { side: 2, amount, .. } => Some(*amount as u16),
            _ => None,
        })
        .collect();
    // Base Attack 65 (Gardevoir, the user), then 65 (Gloom), then 110
    // (Snorlax), typeless and special against the target's own base Defence.
    assert_eq!(strikes, alloc::vec![5, 5, 7], "{events:?}");
}

/// Grudge's `onFaint` runs once per faint. The engine settles a faint both
/// at the tail of the hit and again at the end of the action, and a Leppa
/// Berry eaten in between used to see its refill drained a second time.
#[test]
fn a_grudge_drains_once_and_a_leppa_refill_survives() {
    let mut attacker = mon("snorlax", 50, &["megapunch"]);
    attacker.item = "leppaberry";
    let mut grudger = mon("jolteon", 50, &["grudge"]);
    grudger.hp = 1;
    let mut b = Battle::new(
        Side::new(alloc::vec![attacker]),
        Side::new(alloc::vec![grudger, mon("chansey", 50, &["splash"])]),
        3,
    );
    b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
    assert!(b.sides[1].party[0].fainted(), "the Mega Punch knocked the grudger out");
    assert_eq!(b.sides[0].mon().item, "", "the Leppa Berry was eaten");
    assert_eq!(b.sides[0].mon().moves[0].pp, 10, "drained once, then refilled by ten");
}

/// A one-hit KO comes off the hand like any other strike, so the sim's
/// `eachEvent('Update')` runs for it as well (`data/mods/gen3/scripts.js`
/// `tryMoveHit`, the `eachEvent` that follows the `-ohko` line). The blow
/// that empties the last point of Horn Drill is the one that answers a
/// Leppa Berry, which is the wide-fuzz seed-5860 shape: that KO also ended
/// the battle, so there was no next turn to wait for.
#[test]
fn a_one_hit_ko_takes_the_update_with_it() {
    let mut holder = mon("electabuzz", 50, &["horndrill"]);
    holder.item = "leppaberry";
    holder.moves[0].pp = 1;
    let mut b = battle(holder, mon("slowpoke", 50, &["tackle"]));
    // The drill ends the battle, so the restore has nowhere later to wait.
    b.sides[1].mon_mut().hp = 1;
    let events = b.step_with(
        [Choice::Move(0), Choice::Move(0)],
        &scripted([PLAIN, PLAIN]),
    );
    assert!(b.over(), "the drill took the last point: {events:?}");
    assert_eq!(b.sides[0].mon().item, "", "the berry was eaten");
    assert_eq!(
        b.sides[0].mon().moves[0].pp,
        5,
        "ten points back, capped at the move's own five",
    );
}

/// Beat Up answers with the bare base stats. Its volatile hands the sim's
/// ModifySpA and ModifySpD events a number off the species line and pins
/// `event.modifier` back to 1 as it does, which throws away every stat
/// chain that had spoken by then (`data/mods/gen3/moves.js` `beatup.condition`
/// at `onModifySpA` and `onFoeModifySpD`). Two Clamperls, identical in
/// every stat, land the same damage however their items are dressed.
#[test]
fn beat_up_answers_with_the_base_stats_and_nothing_else() {
    let beat = |user_item: &'static str, foe_item: &'static str| {
        let mut user = mon("clamperl", 50, &["beatup"]);
        user.item = user_item;
        let mut foe = mon("clamperl", 50, &["splash"]);
        foe.item = foe_item;
        let mut b = battle(user, foe);
        let events = b.step_with([Choice::Move(0), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
        events
            .iter()
            .filter_map(|e| match e {
                Event::Damage { side: 2, amount, .. } => Some(*amount),
                _ => None,
            })
            .sum::<u16>()
    };

    // Level 50 Clamperl into a level 50 Clamperl, 10 BP, one ally on the
    // field: base Attack 64 into base Defence 85 gives
    // floor(floor(22 * 10 * 64 / 85) / 50) + 2 = 5. Typeless, so no STAB and
    // no effectiveness step either way.
    let bare = beat("", "");
    assert_eq!(bare, 5, "the bare base stats: {bare}");
    // A Deep Sea Scale would double the 85 the hit reads, and a Deep Sea
    // Tooth the 64 it swings with: floor(floor(22 * 10 * 64 / 170) / 50) + 2
    // is 3, and with the tooth it is 8. The sim deals 5 for all three.
    assert_eq!(beat("", "deepseascale"), bare, "a Deep Sea Scale is not read");
    assert_eq!(beat("deepseatooth", ""), bare, "a Deep Sea Tooth is not read");
}

/// The end-of-turn pool is ONE list, and a mon's own volatiles go into it in
/// the order they were added: `findPokemonEventHandlers` walks
/// `pokemon.volatiles` by insertion order (`sim/battle.js`), so a mon that
/// took its Leech Seed before its Ingrain pools them the other way round. The
/// sim's `speedSort` is a selection sort, so a Speed tie does not leave the
/// pair where the gather put it.
#[test]
fn the_volatiles_pool_in_the_order_they_landed() {
    // Two Slowpoke on the same investment tie on Speed, which is what lets
    // the tie reach the displacement at all.
    let mut b = battle(
        mon("slowpoke", 50, &["ingrain", "leechseed"]),
        mon("slowpoke", 50, &["ingrain", "leechseed"]),
    );
    let hp = b.sides[0].mon().max_hp;
    // Turn 1: p1 roots itself, p2 seeds it. p2 carries nothing yet.
    b.step_with([Choice::Move(0), Choice::Move(1)], &scripted([PLAIN, PLAIN]));
    assert_eq!(
        b.sides[0].mon().hp,
        hp - hp / 8,
        "the seed bit came off: {:?}",
        b.sides[0].mon().hp
    );
    // Turn 2: p1 seeds p2, p2 roots itself, so the two sides carry the pair
    // in opposite orders.
    b.step_with([Choice::Move(1), Choice::Move(0)], &scripted([PLAIN, PLAIN]));
    // Pooled as the sim pools them, p2 pays its seed and its root heals it
    // straight back, while p1's seed still shows. Pooled by bucket instead
    // (both roots first) the damage lands the other way round, and the two
    // sides would each end an eighth light: the order is the whole
    // difference.
    assert_eq!(
        b.sides[0].mon().hp,
        hp - hp / 8,
        "p1 pools its ingrain before its seed",
    );
    assert_eq!(
        b.sides[1].mon().hp,
        hp,
        "p2's root healed back what its seed took",
    );
}
