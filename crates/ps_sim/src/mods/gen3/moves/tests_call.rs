//! Unit tests for the call and copy moves: the slot objects Mimic and
//! Sketch build (no `target`, Sketch's base slot and PP), Transform's
//! unknown type, Conversion 2's read of the move object's type, Skill
//! Swap's line and staleness, Sleep Talk's `nopp`, and the moves Magic
//! Coat, Snatch and Mirror Move use with their `[from]`. Battles are gen 3
//! custom games on the real gen 3 registry, started through `>start` /
//! `>player`.

extern crate std;

use std::boxed::Box;
use std::string::String;

use ps_dex::gen3::{abilities as a, conditions as c, moves as m};
use ps_dex::Type;
use ps_protocol::text::encode_log;
use ps_protocol::text::team::unpack;
use ps_protocol::{Choice, Command, FormatSpec, SideId};

use crate::state::{Battle, MonRef};
use crate::text::Names;

fn spill(_: &Battle, _: &[ps_protocol::Msg]) {}

/// A started gen 3 custom game of the two packed teams.
fn battle(p1: &str, p2: &str) -> Box<Battle> {
    let mut b = Box::new(Battle::from_start(FormatSpec::gen3_harness(), [1, 2, 3, 4], 1_700_000_000));
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
    let s = encode_log(b.log.as_slice(), &tables, &Names(b));
    b.log.clear();
    s
}

fn moves(b: &mut Battle, p1: u8, p2: u8) {
    b.step(&Command::Choose { side: SideId::P1, choice: Choice::Move(p1) }).unwrap();
    b.step(&Command::Choose { side: SideId::P2, choice: Choice::Move(p2) }).unwrap();
}

const P1: MonRef = MonRef { side: 0, index: 0 };
const P2: MonRef = MonRef { side: 1, index: 0 };

#[test]
fn sketch_replaces_its_slot_for_good_without_a_target() {
    // Jolteon moves first, so Sketch (1 PP) finds its Growl.
    let mut b = battle("Smeargle|||OwnTempo|sketch,tackle|||||||", "Jolteon|||VoltAbsorb|growl|||||||");
    moves(&mut b, 1, 1);
    let log = take_log(&mut b);
    assert!(log.contains("|-activate|p1a: Smeargle|move: Mimic|Growl"), "{log}");
    let (slot, base) = (b.mon(P1).move_slots[0], b.mon(P1).base_move_slots[0]);
    assert_eq!(slot, base);
    // Growl's 40 PP; Sketch's own PP Ups (none: `noPPBoosts`) set maxpp.
    assert_eq!((slot.id, slot.pp, slot.maxpp, slot.virtual_, slot.no_target), (m::GROWL, 40, 40, false, true));
    let req = b.get_moves(P1, None, false);
    assert_eq!(req[0].target, None);
    assert!(req[1].target.is_some());
}

#[test]
fn mimic_copies_into_a_virtual_slot_with_five_pp() {
    let mut b = battle("Mr. Mime|||Soundproof|mimic|||||||", "Snorlax|||Stench|bodyslam||||||5|");
    moves(&mut b, 1, 1);
    take_log(&mut b);
    moves(&mut b, 1, 1);
    let log = take_log(&mut b);
    assert!(log.contains("|-activate|p1a: Mr. Mime|move: Mimic|Body Slam"), "{log}");
    let slot = b.mon(P1).move_slots[0];
    // `calculatePP(move, ppUps[0])`: Mimic's 3 PP Ups on Body Slam's 15.
    assert_eq!((slot.id, slot.pp, slot.maxpp, slot.virtual_, slot.no_target), (m::BODYSLAM, 5, 24, true, true));
    assert_eq!(b.mon(P1).base_move_slots[0].id, m::MIMIC);
}

#[test]
fn transform_leaves_the_copied_type_unknown() {
    let mut b = battle("Ditto|||Limber|transform|||||||", "Snorlax|||Stench|bodyslam||||||5|");
    assert!(b.mon(P1).known_type);
    moves(&mut b, 1, 1);
    assert!(take_log(&mut b).contains("|-transform|p1a: Ditto|p2a: Snorlax"));
    assert!(b.mon(P1).transformed);
    assert!(!b.mon(P1).known_type);
    assert_eq!(b.mon(P1).ability, a::STENCH);
    // `clearVolatile` (a switch-out) sets the species, and the type is
    // known again.
    b.clear_volatile(P1, true);
    assert!(b.mon(P1).known_type);
}

#[test]
fn conversion_2_reads_the_last_move_objects_type() {
    // Hidden Power is Normal in the dex; its gen 3 `onModifyMove` makes it
    // Fire here, and only Fire's resisters are drawn.
    let mut b = battle("Porygon2|||Trace|conversion2|||||||", "Jolteon|||VoltAbsorb|hiddenpowerfire|||||||");
    moves(&mut b, 1, 1);
    assert_eq!(b.mon(P2).last_move_used, m::HIDDENPOWER);
    assert_eq!(b.mon(P2).last_move_used_type, Type::Fire);
    let t = b.mon(P1).types.as_slice()[0];
    assert!(matches!(t, Type::Fire | Type::Water | Type::Rock | Type::Dragon), "{t:?}");
    assert!(take_log(&mut b).contains("|-start|p1a: Porygon2|typechange|"));
}

#[test]
fn skill_swap_trades_without_naming_and_stales_the_target() {
    let mut b = battle("Alakazam|||Synchronize|skillswap|||||||", "Snorlax|||Stench|bodyslam||||||5|");
    moves(&mut b, 1, 1);
    let log = take_log(&mut b);
    assert!(log.contains("|-activate|p1a: Alakazam|Skill Swap|||[of] p2a: Snorlax\n"), "{log}");
    assert_eq!((b.mon(P1).ability, b.mon(P2).ability), (a::STENCH, a::SYNCHRONIZE));
    assert!(!b.mon(P1).volatile_staleness && b.mon(P2).volatile_staleness);
    // Gen 5 and earlier: the same ability fails.
    let mut b = battle("Alakazam|||Synchronize|skillswap|||||||", "Snorlax|||Synchronize|bodyslam|||||||");
    moves(&mut b, 1, 1);
    assert!(take_log(&mut b).contains("|-fail|p1a: Alakazam"));
}

#[test]
fn sleep_talk_draws_an_empty_slot_and_cannot_use_it() {
    let mut b = battle("Snorlax|||Stench|sleeptalk,bodyslam|||||||", "Breloom|||Limber|spore|||||||");
    b.mon_mut(P1).move_slots[1].pp = 0;
    moves(&mut b, 1, 1);
    let log = take_log(&mut b);
    assert!(log.contains("|move|p1a: Snorlax|Sleep Talk|p1a: Snorlax\n|cant|p1a: Snorlax|nopp|bodyslam\n"), "{log}");
}

#[test]
fn magic_coat_bounces_a_reflectable_move() {
    let mut b = battle("Xatu|||Synchronize|magiccoat|||||||", "Jolteon|||VoltAbsorb|thunderwave|||||||");
    moves(&mut b, 1, 1);
    let log = take_log(&mut b);
    assert!(log.contains("|-singleturn|p1a: Xatu|move: Magic Coat\n"), "{log}");
    assert!(log.contains("|move|p1a: Xatu|Thunder Wave|p2a: Jolteon|[from] Magic Coat\n"), "{log}");
    assert_eq!(b.mon(P2).status, c::PAR);
    assert!(!b.mon(P1).volatiles.contains(crate::effect::EffectId::Condition(c::MAGICCOAT)));
}

#[test]
fn snatch_steals_a_snatchable_move() {
    let mut b = battle("Absol|||Pressure|snatch|||||||", "Scizor|||Swarm|swordsdance|||||||");
    moves(&mut b, 1, 1);
    let log = take_log(&mut b);
    assert!(log.contains("|-singleturn|p1a: Absol|Snatch\n"), "{log}");
    assert!(log.contains("|-activate|p1a: Absol|move: Snatch|[of] p2a: Scizor\n|move|p1a: Absol|Swords Dance|p1a: Absol|[from] Snatch\n"), "{log}");
    assert_eq!((b.mon(P1).boosts[0], b.mon(P2).boosts[0]), (2, 0));
}

#[test]
fn mirror_move_uses_the_move_that_last_hit_the_user() {
    let mut b = battle("Pidgeot|||KeenEye|mirrormove|||||||", "Snorlax|||Stench|bodyslam||||||5|");
    moves(&mut b, 1, 1);
    // Pidgeot moves first: nothing has hit it yet.
    let log = take_log(&mut b);
    assert!(log.contains("|move|p1a: Pidgeot|Mirror Move||[still]\n|-fail|p1a: Pidgeot"), "{log}");
    moves(&mut b, 1, 1);
    let log = take_log(&mut b);
    assert!(log.contains("|move|p1a: Pidgeot|Body Slam|p2a: Snorlax|[from] Mirror Move\n"), "{log}");
}
