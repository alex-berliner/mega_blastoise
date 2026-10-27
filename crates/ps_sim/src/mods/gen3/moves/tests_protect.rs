//! Unit tests for the protection, side condition and healing batch,
//! played through real gen 3 custom games: Protect's block, Spikes'
//! layers and Rapid Spin's `self` effect, Wish's delayed heal, Pain Split,
//! Safeguard, and Rest.

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
fn protect_stops_the_foes_move_for_the_turn() {
    let mut b = battle("Jolteon|||Stench|protect,splash|||||||", "Snorlax|||Stench|tackle|||||||");
    let max = b.mon(P1).maxhp;
    turn(&mut b, Choice::Move(1), Choice::Move(1));
    let log = take_log(&mut b);
    assert!(log.contains("|-singleturn|p1a: Jolteon|Protect\n"), "{log}");
    assert!(log.contains("|-activate|p1a: Jolteon|Protect\n"), "{log}");
    assert_eq!(b.mon(P1).hp, max);
    // The volatile lasts the turn; `stall` stays for the next one.
    assert!(!b.mon(P1).volatiles.contains(EffectId::Condition(c::PROTECT)));
    assert!(b.mon(P1).volatiles.contains(EffectId::Condition(c::STALL)));
}

#[test]
fn spikes_stack_to_three_and_rapid_spin_clears_them() {
    let mut b = battle(
        "Skarmory|||Stench|spikes,splash|||||||",
        "Starmie|||Stench|splash,rapidspin|||||||]Snorlax|||Stench|splash|||||||",
    );
    for layers in 1..=3u8 {
        turn(&mut b, Choice::Move(1), Choice::Move(1));
        let log = take_log(&mut b);
        assert!(log.contains("|-sidestart|p2: p|Spikes\n"), "{log}");
        let st = b.sides[1].side_conditions.get(EffectId::Condition(c::SPIKES)).unwrap();
        assert_eq!(st.layers, layers);
    }
    // A fourth layer fails (`onSideRestart` answers false); Snorlax
    // switching in loses a quarter of its HP.
    turn(&mut b, Choice::Move(1), Choice::Switch(2));
    let log = take_log(&mut b);
    assert!(log.contains("|-fail|p1a: Skarmory\n"), "{log}");
    let snorlax = b.active_at(1, 0).unwrap();
    let max = i32::from(b.mon(snorlax).maxhp);
    assert_eq!(i32::from(b.mon(snorlax).hp), max - 6 * max / 24);
    assert!(log.contains("[from] Spikes\n"), "{log}");
    // Back to Starmie, whose Rapid Spin's `self.onHit` clears the side.
    turn(&mut b, Choice::Move(1), Choice::Switch(2));
    take_log(&mut b);
    turn(&mut b, Choice::Move(2), Choice::Move(2));
    let log = take_log(&mut b);
    assert!(log.contains("|-sideend|p2: p|Spikes|[from] move: Rapid Spin|[of] p2a: Starmie\n"), "{log}");
    assert!(!b.sides[1].side_conditions.contains(EffectId::Condition(c::SPIKES)));
}

#[test]
fn wish_heals_the_slot_at_the_end_of_the_next_turn() {
    let mut b = battle("Jirachi|||Stench|wish,splash|||||||", "Snorlax|||Stench|splash|||||||");
    b.mon_mut(P1).hp = 1;
    turn(&mut b, Choice::Move(1), Choice::Move(1));
    let log = take_log(&mut b);
    assert!(!log.contains("|-heal|"), "{log}");
    turn(&mut b, Choice::Move(2), Choice::Move(1));
    let log = take_log(&mut b);
    assert!(log.contains("|[from] move: Wish|[wisher] Jirachi\n"), "{log}");
    assert_eq!(b.mon(P1).hp, 1 + b.mon(P1).base_maxhp / 2);
}

#[test]
fn pain_split_averages_the_hp() {
    let mut b = battle("Misdreavus|||Stench|painsplit|||||||", "Snorlax|||Stench|splash|||||||");
    b.mon_mut(P1).hp = 10;
    let average = (10 + i32::from(b.mon(P2).hp)) / 2;
    turn(&mut b, Choice::Move(1), Choice::Move(1));
    let log = take_log(&mut b);
    assert!(log.contains("|[from] move: Pain Split|[silent]\n"), "{log}");
    assert!(log.contains("|-sethp|p1a: Misdreavus|"), "{log}");
    assert_eq!(i32::from(b.mon(P1).hp), average);
    assert_eq!(i32::from(b.mon(P2).hp), average);
}

#[test]
fn safeguard_stops_the_foes_status_move() {
    let mut b = battle("Jolteon|||Stench|safeguard,splash|||||||", "Raichu|||Stench|toxic|||||||");
    turn(&mut b, Choice::Move(1), Choice::Move(1));
    let log = take_log(&mut b);
    assert!(log.contains("|-sidestart|p1: p|Safeguard\n"), "{log}");
    assert!(log.contains("|-activate|p1a: Jolteon|move: Safeguard\n"), "{log}");
    assert!(b.mon(P1).status.is_none());
}

#[test]
fn rest_sleeps_for_two_turns_and_heals() {
    let mut b = battle("Snorlax|||Stench|rest,splash|||||||", "Snorlax|||Stench|splash|||||||");
    b.mon_mut(P1).hp = 1;
    turn(&mut b, Choice::Move(1), Choice::Move(1));
    let log = take_log(&mut b);
    assert!(log.contains("|-status|p1a: Snorlax|slp|[from] move: Rest\n"), "{log}");
    assert!(log.contains("|[silent]\n"), "{log}");
    assert_eq!(b.mon(P1).hp, b.mon(P1).maxhp);
    assert_eq!(b.mon(P1).status_state.data[crate::mods::gen3::conditions::slp::TIME], 3);
    // At full HP it fails: `-fail|MON|heal`.
    let mut b = battle("Snorlax|||Stench|rest|||||||", "Snorlax|||Stench|splash|||||||");
    turn(&mut b, Choice::Move(1), Choice::Move(1));
    let log = take_log(&mut b);
    assert!(log.contains("|-fail|p1a: Snorlax|heal\n"), "{log}");
}
