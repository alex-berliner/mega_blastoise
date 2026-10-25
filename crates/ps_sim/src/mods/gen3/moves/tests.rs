//! Unit tests for this batch's moves, played through real gen 3 custom
//! games (`>start` / `>player`, then choices): Bide's store and strike,
//! the future moves' delayed hit, Beat Up's allies and Pursuit on a
//! switching foe.

extern crate std;

use std::boxed::Box;
use std::string::String;

use ps_dex::gen3::conditions as c;
use ps_protocol::text::encode_log;
use ps_protocol::text::team::unpack;
use ps_protocol::{Choice, FormatSpec, SideId};

use crate::effect::EffectId;
use crate::state::{Battle, MonRef};
use crate::text::Names;

fn spill(_: &Battle, _: &[ps_protocol::Msg]) {}

/// A started gen 3 custom game of the two packed teams, its log cleared.
fn battle(p1: &str, p2: &str) -> Box<Battle> {
    battle_seeded(p1, p2, [1, 2, 3, 4])
}

fn battle_seeded(p1: &str, p2: &str, seed: [u16; 4]) -> Box<Battle> {
    let mut b = Box::new(Battle::from_start(FormatSpec::gen3_harness(), seed, 1_700_000_000));
    b.spill = Some(spill);
    for (side, team) in [(SideId::P1, p1), (SideId::P2, p2)] {
        b.set_player(side, "p", "", "", Some(&unpack(team).unwrap())).unwrap();
    }
    b.log.clear();
    b
}

/// The log as PS text, one line per entry, and cleared.
fn take_log(b: &mut Battle) -> String {
    let tables = crate::text::dex_tables(b.dex);
    let mut s = encode_log(b.log.as_slice(), &tables, &Names(b));
    if !s.is_empty() {
        s.push('\n');
    }
    b.log.clear();
    s
}

/// One turn: both sides' choices.
fn turn(b: &mut Battle, p1: Choice, p2: Choice) {
    assert!(b.choose(0, p1));
    assert!(b.choose(1, p2));
}

const P1: MonRef = MonRef { side: 0, index: 0 };
const P2: MonRef = MonRef { side: 1, index: 0 };

#[test]
fn bide_strikes_back_for_twice_the_stored_damage() {
    // Jolteon outspeeds Snorlax, so its third-turn strike comes before the
    // third Tackle.
    let mut b = battle("Jolteon|||Stench|bide|||||||", "Snorlax|||Stench|tackle|||||||");
    let max = b.mon(P1).maxhp;
    turn(&mut b, Choice::Move(1), Choice::Move(1));
    let log = take_log(&mut b);
    assert!(log.contains("|-start|p1a: Jolteon|move: Bide\n"), "{log}");
    turn(&mut b, Choice::Default, Choice::Move(1));
    let log = take_log(&mut b);
    assert!(log.contains("|-activate|p1a: Jolteon|move: Bide\n"), "{log}");
    let key = EffectId::Condition(c::BIDE);
    let total = super::super::conditions::bide::total_damage(b.mon(P1).volatiles.get(key).unwrap());
    assert_eq!(total, i32::from(max - b.mon(P1).hp));
    assert!(total > 0);
    let before = b.mon(P2).hp;
    turn(&mut b, Choice::Default, Choice::Move(1));
    let log = take_log(&mut b);
    assert!(log.contains("|-end|p1a: Jolteon|move: Bide\n"), "{log}");
    assert!(log.contains("|-end|p1a: Jolteon|move: Bide|[silent]\n"), "{log}");
    assert_eq!(i32::from(before - b.mon(P2).hp), (2 * total).min(i32::from(before)));
    assert!(!b.mon(P1).volatiles.contains(key));
}

#[test]
fn bide_unleashed_into_a_ghost_is_immune() {
    // The plain `moveData` has no `ignoreImmunity`: gen 3's `tryMoveHit`
    // makes it `false`, so Normal fails against Ghost.
    let mut b = battle("Jolteon|||Stench|bide|||||||", "Gengar|||Levitate|nightshade|||||||");
    turn(&mut b, Choice::Move(1), Choice::Move(1));
    turn(&mut b, Choice::Default, Choice::Move(1));
    take_log(&mut b);
    let before = b.mon(P2).hp;
    turn(&mut b, Choice::Default, Choice::Move(1));
    let log = take_log(&mut b);
    assert!(log.contains("|-end|p1a: Jolteon|move: Bide\n|-immune|p2a: Gengar\n"), "{log}");
    assert_eq!(b.mon(P2).hp, before);
}

/// Future Sight on turn 1, Harden on turns 2 and 3: the log of turn 3,
/// the rolled damage and the target's HP before and after turn 3.
fn future_sight_game(seed: [u16; 4]) -> (String, i16, u16, u16) {
    let mut b = battle_seeded("Alakazam|||Stench|futuresight,harden|||||||", "Snorlax|||Stench|harden|||||||", seed);
    turn(&mut b, Choice::Move(1), Choice::Move(1));
    let log = take_log(&mut b);
    assert!(log.contains("|-start|p1a: Alakazam|Future Sight\n"), "{log}");
    let key = EffectId::Condition(c::FUTUREMOVE);
    let st = *b.sides[1].slot_conditions[0].get(key).unwrap();
    let damage = st.data[super::super::conditions::futuremove::MOVE_DAMAGE];
    assert!(damage > 0);
    // `duration: 3`, one residual later.
    assert_eq!((st.duration, st.source), (2, Some(P1)));
    // A second one on the slot fails.
    turn(&mut b, Choice::Move(1), Choice::Move(1));
    let log = take_log(&mut b);
    assert!(!log.contains("-start"), "{log}");
    let before = b.mon(P2).hp;
    turn(&mut b, Choice::Move(2), Choice::Move(1));
    let log = take_log(&mut b);
    assert!(log.contains("|-end|p2a: Snorlax|move: Future Sight\n"), "{log}");
    assert!(!b.sides[1].slot_conditions[0].contains(key));
    (log, damage, before, b.mon(P2).hp)
}

#[test]
fn future_sight_rolls_its_damage_now_and_lands_two_turns_later() {
    let (log, damage, before, after) = future_sight_game([5, 6, 7, 8]);
    assert!(!log.contains("-miss"), "{log}");
    assert_eq!(after, before - damage as u16, "{log}");
}

#[test]
fn a_future_move_miss_marks_the_last_move_line() {
    // The hit has no `|move|` line of its own: `attrLastMove('[miss]')`
    // lands on the turn's last one, Snorlax's Harden.
    let (log, _, before, after) = future_sight_game([1, 2, 3, 4]);
    assert!(log.contains("|move|p2a: Snorlax|Harden|p2a: Snorlax|[miss]\n"), "{log}");
    assert!(log.contains("|-end|p2a: Snorlax|move: Future Sight\n|-miss|p1a: Alakazam|p2a: Snorlax\n"), "{log}");
    assert_eq!(after, before);
}

#[test]
fn beat_up_strikes_once_per_healthy_ally_naming_each() {
    let mut b = battle(
        "Sneasel|||Stench|beatup|||||||]Bob|Snorlax||Stench|tackle|||||||]Jolteon|||Stench|tackle|||||||",
        "Snorlax|||Stench|harden|||||||",
    );
    turn(&mut b, Choice::Move(1), Choice::Move(1));
    let log = take_log(&mut b);
    let expected = "|-activate|p1a: Sneasel|move: Beat Up|[of] Sneasel\n";
    assert!(log.contains(expected), "{log}");
    assert!(log.contains("|-activate|p1a: Sneasel|move: Beat Up|[of] Bob\n"), "{log}");
    assert!(log.contains("|-activate|p1a: Sneasel|move: Beat Up|[of] Jolteon\n"), "{log}");
    assert!(log.contains("|-hitcount|p2a: Snorlax|3\n"), "{log}");
    // `duration: 1`: gone by the end of the turn.
    assert!(!b.mon(P1).volatiles.contains(EffectId::Condition(c::BEATUP)));
}

#[test]
fn pursuit_hits_a_switching_foe_first() {
    let mut b = battle(
        "Umbreon|||Stench|pursuit|||||||",
        "Snorlax|||Stench|harden|||||||]Jolteon|||Stench|harden|||||||",
    );
    let before = b.mon(P2).hp;
    turn(&mut b, Choice::Move(1), Choice::Switch(2));
    let log = take_log(&mut b);
    let activate = log.find("|-activate|p2a: Snorlax|move: Pursuit\n").expect(&log);
    let hit = log.find("|move|p1a: Umbreon|Pursuit|p2a: Snorlax").expect(&log);
    let switch = log.find("|switch|p2a: Jolteon").expect(&log);
    assert!(activate < hit && hit < switch, "{log}");
    assert!(b.mon(P2).hp < before);
    // The queued Pursuit was used up by the switch-out hit.
    assert_eq!(log.matches("|Pursuit|").count(), 1, "{log}");
}

#[test]
fn attr_last_move_marks_anim_lines_and_repeats_a_miss() {
    use ps_dex::gen3::moves as m;
    use ps_protocol::{Kw, KwFlags, MoveNum, Msg};
    let mut b = battle("Sunflora|||Stench|solarbeam|||||||", "Snorlax|||Stench|tackle|||||||");
    let (user, target) = (b.ident(P1), b.ident(P2));
    // Solar Beam in the sun: `addMove('-anim', ...)`, then a miss.
    b.add_move(Msg::Anim { mon: user, mv: MoveNum(m::SOLARBEAM.0), target: Some(target), kw: Kw::NONE });
    b.attr_last_move(KwFlags::MISS);
    assert_eq!(take_log(&mut b), "|-anim|p1a: Sunflora|Solar Beam|p2a: Snorlax|[miss]\n");
    // A `|move|` line that missed, then a future move's miss.
    b.add_move(Msg::Move { user: target, mv: MoveNum(m::TACKLE.0), target: Some(user), kw: Kw::NONE });
    b.attr_last_move(KwFlags::MISS);
    b.attr_last_move(KwFlags::MISS);
    assert_eq!(take_log(&mut b), "|move|p2a: Snorlax|Tackle|p1a: Sunflora|[miss]|[miss]\n");
    // A `[notarget]` line, then a future move's miss: PS appends the text.
    b.add_move(Msg::Move { user, mv: MoveNum(m::TACKLE.0), target: None, kw: Kw::NONE });
    b.attr_last_move(KwFlags::NOTARGET);
    b.attr_last_move(KwFlags::MISS);
    assert!(take_log(&mut b).ends_with("|[notarget]|[miss]\n"));
}
