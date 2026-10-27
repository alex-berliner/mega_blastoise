//! Unit tests for the item moves, Counter and Mirror Coat and the rest of
//! their batch (Spite, Fake Out, Dream Eater, Baton Pass), played through
//! real gen 3 custom games, and for Endless Battle Clause's berry-cycling
//! verdict, which Trick and Recycle make reachable.

extern crate std;

use std::boxed::Box;
use std::string::String;

use ps_dex::gen3::items;
use ps_protocol::text::encode_log;
use ps_protocol::text::team::unpack;
use ps_protocol::{Choice, FormatId, FormatSpec, SideId};

use crate::state::{Battle, MonRef, Staleness};
use crate::text::Names;

fn spill(_: &Battle, _: &[ps_protocol::Msg]) {}

fn battle_in(format: FormatSpec, p1: &str, p2: &str) -> Box<Battle> {
    let mut b = Box::new(Battle::from_start(format, [1, 2, 3, 4], 1_700_000_000));
    b.spill = Some(spill);
    for (side, name, team) in [(SideId::P1, "Alice", p1), (SideId::P2, "Bob", p2)] {
        b.set_player(side, name, "", "", Some(&unpack(team).unwrap())).unwrap();
    }
    b.log.clear();
    b
}

/// A started gen 3 custom game of the two packed teams, its log cleared.
fn battle(p1: &str, p2: &str) -> Box<Battle> {
    battle_in(FormatSpec::gen3_harness(), p1, p2)
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

fn turn(b: &mut Battle, p1: Choice, p2: Choice) {
    assert!(b.choose(0, p1));
    assert!(b.choose(1, p2));
}

const P1: MonRef = MonRef { side: 0, index: 0 };
const P2: MonRef = MonRef { side: 1, index: 0 };

#[test]
fn trick_swaps_the_items() {
    let mut b = battle("Alakazam||firestone|Synchronize|trick|||||||", "Snorlax||sitrusberry|Immunity|harden|||||||");
    turn(&mut b, Choice::Move(1), Choice::Move(1));
    let log = take_log(&mut b);
    assert!(log.contains("|-activate|p1a: Alakazam|move: Trick|[of] p2a: Snorlax\n"), "{log}");
    assert!(log.contains("|-item|p2a: Snorlax|Fire Stone|[from] move: Trick\n"), "{log}");
    assert!(log.contains("|-item|p1a: Alakazam|Sitrus Berry|[from] move: Trick\n"), "{log}");
    assert_eq!((b.mon(P1).item, b.mon(P2).item), (items::SITRUSBERRY, items::FIRESTONE));
}

#[test]
fn trick_with_one_item_ends_the_other_silently() {
    let mut b = battle("Alakazam|||Synchronize|trick|||||||", "Snorlax||firestone|Immunity|harden|||||||");
    turn(&mut b, Choice::Move(1), Choice::Move(1));
    let log = take_log(&mut b);
    assert!(log.contains("|-enditem|p2a: Snorlax|Fire Stone|[silent]|[from] move: Trick\n"), "{log}");
    assert!(log.contains("|-item|p1a: Alakazam|Fire Stone|[from] move: Trick\n"), "{log}");
    assert!(b.mon(P2).item.is_none());
}

#[test]
fn trick_fails_on_sticky_hold_and_with_no_items() {
    let mut b = battle("Alakazam||firestone|Synchronize|trick|||||||", "Muk||waterstone|StickyHold|harden|||||||");
    turn(&mut b, Choice::Move(1), Choice::Move(1));
    let log = take_log(&mut b);
    assert!(log.contains("|-immune|p2a: Muk\n"), "{log}");
    assert_eq!(b.mon(P2).item, items::WATERSTONE);
    let mut b = battle("Alakazam|||Synchronize|trick|||||||", "Snorlax|||Immunity|harden|||||||");
    turn(&mut b, Choice::Move(1), Choice::Move(1));
    let log = take_log(&mut b);
    assert!(log.contains("|-fail|p1a: Alakazam\n"), "{log}");
}

#[test]
fn knock_off_knocks_the_item_off() {
    let mut b = battle("Sneasel|||InnerFocus|knockoff|||||||", "Snorlax||firestone|Immunity|harden|||||||");
    turn(&mut b, Choice::Move(1), Choice::Move(1));
    let log = take_log(&mut b);
    assert!(log.contains("|-enditem|p2a: Snorlax|Fire Stone|[from] move: Knock Off|[of] p1a: Sneasel\n"), "{log}");
    assert!(b.mon(P2).item.is_none() && b.mon(P2).item_knocked_off);
    // Sticky Hold keeps it, even from Knock Off.
    let mut b = battle("Sneasel|||InnerFocus|knockoff|||||||", "Muk||firestone|StickyHold|harden|||||||");
    turn(&mut b, Choice::Move(1), Choice::Move(1));
    let log = take_log(&mut b);
    assert!(log.contains("|-activate|p2a: Muk|ability: Sticky Hold\n"), "{log}");
    assert_eq!(b.mon(P2).item, items::FIRESTONE);
}

#[test]
fn thief_and_covet_take_the_item_into_an_empty_hand() {
    let mut b = battle("Sneasel|||InnerFocus|thief|||||||", "Snorlax||firestone|Immunity|harden|||||||");
    turn(&mut b, Choice::Move(1), Choice::Move(1));
    let log = take_log(&mut b);
    assert!(log.contains("|-enditem|p2a: Snorlax|Fire Stone|[silent]|[from] move: Thief|[of] p1a: Sneasel\n"), "{log}");
    assert!(log.contains("|-item|p1a: Sneasel|Fire Stone|[from] move: Thief|[of] p2a: Snorlax\n"), "{log}");
    assert_eq!((b.mon(P1).item, b.mon(P2).item), (items::FIRESTONE, ps_dex::ItemId::NONE));
    let mut b = battle("Skitty|||CuteCharm|covet|||||||", "Snorlax||firestone|Immunity|harden|||||||");
    turn(&mut b, Choice::Move(1), Choice::Move(1));
    let log = take_log(&mut b);
    assert!(!log.contains("|-enditem|"), "{log}");
    assert!(log.contains("|-item|p1a: Skitty|Fire Stone|[from] move: Covet|[of] p2a: Snorlax\n"), "{log}");
    // A user holding an item takes nothing.
    let mut b = battle("Sneasel||waterstone|InnerFocus|thief|||||||", "Snorlax||firestone|Immunity|harden|||||||");
    turn(&mut b, Choice::Move(1), Choice::Move(1));
    assert!(!take_log(&mut b).contains("|-item|"));
    assert_eq!(b.mon(P2).item, items::FIRESTONE);
}

#[test]
fn recycle_restores_the_last_item_once() {
    let mut b = battle("Snorlax|||Immunity|recycle|||||||", "Snorlax|||Immunity|harden|||||||");
    b.mon_mut(P1).last_item = items::SITRUSBERRY;
    turn(&mut b, Choice::Move(1), Choice::Move(1));
    let log = take_log(&mut b);
    assert!(log.contains("|-item|p1a: Snorlax|Sitrus Berry|[from] move: Recycle\n"), "{log}");
    assert_eq!((b.mon(P1).item, b.mon(P1).last_item), (items::SITRUSBERRY, ps_dex::ItemId::NONE));
    assert_eq!(b.mon(P1).pending_staleness, Staleness::Internal);
    turn(&mut b, Choice::Move(1), Choice::Move(1));
    let log = take_log(&mut b);
    assert!(log.contains("|-fail|p1a: Snorlax\n"), "{log}");
}

#[test]
fn baton_pass_fails_without_a_bench_and_passes_boosts() {
    let mut b = battle("Jolteon|||VoltAbsorb|batonpass|||||||", "Snorlax|||Immunity|harden|||||||");
    turn(&mut b, Choice::Move(1), Choice::Move(1));
    let log = take_log(&mut b);
    assert!(log.contains("|move|p1a: Jolteon|Baton Pass||[still]\n|-fail|p1a: Jolteon\n"), "{log}");
    let mut b =
        battle("Jolteon|||VoltAbsorb|batonpass,agility|||||||]Snorlax|||Immunity|harden|||||||", "Snorlax|||Immunity|harden|||||||");
    turn(&mut b, Choice::Move(2), Choice::Move(1));
    turn(&mut b, Choice::Move(1), Choice::Move(1));
    assert!(b.mon(P1).skip_pursuit);
    assert!(b.choose(0, Choice::Switch(2)));
    let log = take_log(&mut b);
    assert!(log.contains("|switch|p1a: Snorlax|"), "{log}");
    let snorlax = b.active_at(0, 0).unwrap();
    assert_eq!(b.mon(snorlax).boosts[4], 2);
}

#[test]
fn counter_returns_twice_the_physical_damage() {
    let mut b = battle("Wobbuffet|||ShadowTag|counter|||||||", "Snorlax|||Immunity|tackle|||||||");
    let (max, foe) = (b.mon(P1).maxhp, b.mon(P2).hp);
    turn(&mut b, Choice::Move(1), Choice::Move(1));
    let log = take_log(&mut b);
    let taken = max - b.mon(P1).hp;
    assert!(taken > 0, "{log}");
    assert_eq!(foe - b.mon(P2).hp, (2 * taken).min(foe), "{log}");
    // A special move is not countered: `onTry` fails it silently.
    let mut b = battle("Wobbuffet|||ShadowTag|counter|||||||", "Starmie|||Illuminate|surf|||||||");
    let foe = b.mon(P2).hp;
    turn(&mut b, Choice::Move(1), Choice::Move(1));
    let log = take_log(&mut b);
    assert!(log.contains("|move|p1a: Wobbuffet|Counter|p2a: Starmie\n"), "{log}");
    assert_eq!(b.mon(P2).hp, foe, "{log}");
}

#[test]
fn mirror_coat_returns_twice_the_special_damage() {
    let mut b = battle("Wobbuffet|||ShadowTag|mirrorcoat|||||||", "Starmie|||Illuminate|surf|||||||");
    let (max, foe) = (b.mon(P1).maxhp, b.mon(P2).hp);
    turn(&mut b, Choice::Move(1), Choice::Move(1));
    let log = take_log(&mut b);
    let taken = max - b.mon(P1).hp;
    assert!(taken > 0, "{log}");
    assert_eq!(foe - b.mon(P2).hp, (2 * taken).min(foe), "{log}");
}

#[test]
fn spite_cuts_the_last_move_pp() {
    let mut b = battle("Gengar|||Levitate|spite|||||||", "Snorlax|||Immunity|tackle|||||||");
    // Gengar is faster: the first Spite finds no last move.
    turn(&mut b, Choice::Move(1), Choice::Move(1));
    let log = take_log(&mut b);
    assert!(log.contains("|move|p1a: Gengar|Spite||[still]\n|-fail|p1a: Gengar\n"), "{log}");
    let pp = b.mon(P2).move_slots[0].pp;
    turn(&mut b, Choice::Move(1), Choice::Move(1));
    let log = take_log(&mut b);
    let line = log.lines().find(|l| l.starts_with("|-activate|p2a: Snorlax|move: Spite|tackle|")).expect(&log);
    let roll: i8 = line.rsplit('|').next().unwrap().parse().unwrap();
    assert!((2..=5).contains(&roll));
    assert_eq!(b.mon(P2).move_slots[0].pp, pp - roll - 1);
}

#[test]
fn fake_out_works_on_the_first_turn_only() {
    let mut b = battle("Persian|||Limber|fakeout|||||||", "Snorlax|||Immunity|harden|||||||");
    turn(&mut b, Choice::Move(1), Choice::Move(1));
    let log = take_log(&mut b);
    assert!(log.contains("|cant|p2a: Snorlax|flinch\n"), "{log}");
    turn(&mut b, Choice::Move(1), Choice::Move(1));
    let log = take_log(&mut b);
    // Gen 3's `tryMoveHit` fails a `false` from `onTry` silently.
    assert!(log.contains("|move|p1a: Persian|Fake Out|p2a: Snorlax\n|-hint|Fake Out only works on your first turn out.\n|move|"), "{log}");
}

#[test]
fn dream_eater_needs_a_sleeping_target() {
    let mut b = battle("Hypno|||Insomnia|dreameater|||||||", "Snorlax|||Immunity|harden|||||||");
    turn(&mut b, Choice::Move(1), Choice::Move(1));
    let log = take_log(&mut b);
    assert!(log.contains("|-immune|p2a: Snorlax\n"), "{log}");
}

#[test]
fn endless_battle_clause_judges_berry_cycling() {
    let format = || FormatSpec::new(FormatId::Gen3RandomBattle);
    // Alice's set can cycle a restorative berry: she loses.
    let mut b = battle_in(format(), "Snorlax||sitrusberry|Immunity|recycle|||||||", "Snorlax|||Immunity|harden|||||||");
    assert!(b.mon(P1).set_restorative_berry && b.mon(P1).set_cycles_berries);
    b.turn = 101;
    // Not every side is stale yet.
    assert!(!b.maybe_trigger_endless_battle_clause([false; 2], [Staleness::External, Staleness::None]));
    b.mon_mut(P1).staleness = Staleness::External;
    b.mon_mut(P2).staleness = Staleness::Internal;
    assert!(b.maybe_trigger_endless_battle_clause([false; 2], [Staleness::External, Staleness::Internal]));
    let log = take_log(&mut b);
    assert!(
        log.contains("|-message|Alice's team started with the rudimentary means to perform restorative berry-cycling and thus loses.\n"),
        "{log}"
    );
    assert!(log.ends_with("|win|Bob\n"), "{log}");
    // Nobody can cycle: a tie.
    let mut b = battle_in(format(), "Snorlax||sitrusberry|Immunity|harden|||||||", "Snorlax|||Immunity|harden|||||||");
    b.turn = 101;
    b.mon_mut(P1).staleness = Staleness::External;
    b.mon_mut(P2).staleness = Staleness::External;
    // A trapped side cannot switch to a non-stale Pokemon either way.
    assert!(b.maybe_trigger_endless_battle_clause([true, false], [Staleness::External, Staleness::External]));
    assert!(take_log(&mut b).ends_with("|tie\n"));
    // Internal staleness alone never activates it.
    let mut b = battle_in(format(), "Snorlax||sitrusberry|Immunity|harden|||||||", "Snorlax|||Immunity|harden|||||||");
    b.turn = 101;
    assert!(!b.maybe_trigger_endless_battle_clause([false; 2], [Staleness::Internal, Staleness::Internal]));
}
