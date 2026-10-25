//! Unit tests for the power and damage callbacks (G3-M1), played through
//! real gen 3 custom games: what the sweep rarely reaches.

extern crate std;

use std::boxed::Box;
use std::string::String;

use ps_dex::gen3::conditions as c;
use ps_protocol::text::encode_log;
use ps_protocol::text::team::unpack;
use ps_protocol::{Choice, FormatSpec, SideId};

use crate::effect::EffectId;
use crate::mods::gen3::conditions::substitute;
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

fn turn(b: &mut Battle, p1: Choice, p2: Choice) {
    assert!(b.choose(0, p1));
    assert!(b.choose(1, p2));
}

const P1: MonRef = MonRef { side: 0, index: 0 };
const P2: MonRef = MonRef { side: 1, index: 0 };

fn sub_hp(b: &Battle, m: MonRef) -> f64 {
    substitute::hp(&b.mon(m).volatiles.get(EffectId::Condition(c::SUBSTITUTE)).unwrap().data)
}

#[test]
fn substitute_hp_round_trips_a_js_number() {
    let mut data = [0i16; 4];
    for hp in [0.0, 1.0, 100.0, 0.6999999999999993, 25.5, -0.25] {
        substitute::set_hp(&mut data, hp);
        assert_eq!(substitute::hp(&data).to_bits(), f64::to_bits(hp));
    }
}

#[test]
fn psywave_leaves_its_fraction_in_a_substitute() {
    // A level 51 Psywave deals `random(50, 151) * 51 / 100`, a fraction
    // unless the roll is 100; the doll's HP keeps it (PS's JS number).
    let mut b = battle(
        "Alakazam|||Stench|psywave,harden||||||51|",
        "Snorlax|||Stench|substitute,harden|||||||",
    );
    turn(&mut b, Choice::Move(2), Choice::Move(1));
    let start = sub_hp(&b, P2);
    assert_eq!(start, f64::from(b.mon(P2).maxhp / 4));
    take_log(&mut b);
    // Psywave is 80% accurate: until it hits.
    let mut log = String::new();
    for _ in 0..10 {
        turn(&mut b, Choice::Move(1), Choice::Move(2));
        log = take_log(&mut b);
        if !log.contains("[miss]") {
            break;
        }
    }
    assert!(log.contains("|-activate|p2a: Snorlax|Substitute|[damage]\n"), "{log}");
    let dealt = start - sub_hp(&b, P2);
    let roll = dealt * 100.0 / 51.0;
    assert!((50.0..=150.0).contains(&roll), "{dealt}");
    assert!((roll - f64::from(roll as i32)).abs() < 1e-9, "{dealt}");
    // The roll at this seed is not 100: the doll's HP is fractional.
    assert_ne!(sub_hp(&b, P2), f64::from(sub_hp(&b, P2) as i32), "{dealt}");
}

#[test]
fn eruption_prints_its_fractional_power_and_floors_it_to_one() {
    let mut b = battle("Typhlosion|||Stench|eruption|||||||", "Snorlax|||Stench|harden|||||||");
    b.mon_mut(P1).hp = 1;
    let maxhp = b.mon(P1).maxhp;
    let before = b.mon(P2).hp;
    turn(&mut b, Choice::Move(1), Choice::Move(1));
    let log = take_log(&mut b);
    let bp = 150.0 * 1.0 / f64::from(maxhp);
    assert!(bp < 1.0);
    assert!(log.contains(&std::format!("|debug|BP: {bp}\n")), "{log}");
    // `clampIntRange(basePower, 1)`: still a hit, for at least 1.
    assert!(b.mon(P2).hp < before, "{log}");
}

#[test]
fn hi_jump_kick_into_a_ghost_does_not_crash() {
    // `target.runImmunity('Fighting')` is false: no crash damage.
    let mut b = battle("Hitmonlee|||Stench|highjumpkick|||||||", "Gengar|||Levitate|harden|||||||");
    let hp = b.mon(P1).hp;
    turn(&mut b, Choice::Move(1), Choice::Move(1));
    let log = take_log(&mut b);
    assert!(log.contains("|-immune|p2a: Gengar\n"), "{log}");
    assert_eq!(b.mon(P1).hp, hp, "{log}");
}

#[test]
fn endeavor_fails_when_the_user_has_as_much_hp() {
    let mut b = battle("Snorlax|||Stench|endeavor|||||||", "Pikachu|||Stench|harden||||||5|");
    turn(&mut b, Choice::Move(1), Choice::Move(1));
    let log = take_log(&mut b);
    assert!(log.contains("|-fail|p1a: Snorlax\n"), "{log}");
}
