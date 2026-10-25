//! Unit tests for the volatile-control batch's core pieces the protocol
//! diff reaches only rarely: the `trapped` / `trapper` link (switching
//! either side out, a fainted source, Baton Pass's copy) and the
//! residual order of a condition that ends by `duration` with ordering
//! keys and no residual handler (Taunt, Yawn).

extern crate std;

use std::boxed::Box;
use std::string::String;

use ps_dex::gen3::{conditions as c, moves as m};
use ps_protocol::text::encode_log;
use ps_protocol::text::team::unpack;
use ps_protocol::{FormatSpec, SideId};

use crate::effect::{EffectId, EventId};
use crate::relay::Relay;
use crate::state::{Battle, MonRef};
use crate::text::Names;

fn spill(_: &Battle, _: &[ps_protocol::Msg]) {}

fn battle(p1: &str, p2: &str) -> Box<Battle> {
    let mut b = Box::new(Battle::from_start(FormatSpec::gen3_harness(), [1, 2, 3, 4], 1_700_000_000));
    b.spill = Some(spill);
    for (side, team) in [(SideId::P1, p1), (SideId::P2, p2)] {
        b.set_player(side, "p", "", "", Some(&unpack(team).unwrap())).unwrap();
    }
    b.log.clear();
    b
}

fn take_log(b: &mut Battle) -> String {
    let tables = crate::text::dex_tables(b.dex);
    let s = encode_log(b.log.as_slice(), &tables, &Names(b));
    b.log.clear();
    s
}

const P1: MonRef = MonRef { side: 0, index: 0 };
const P2: MonRef = MonRef { side: 1, index: 0 };
const P2_BENCH: MonRef = MonRef { side: 1, index: 1 };

const UMBREON: &str = "Umbreon|||Levitate|meanlook|||||||";
const SNORLAXES: &str = "Snorlax|||Stench|tackle|||||||]Snorlax|||Stench|tackle|||||||";

fn has(b: &Battle, mon: MonRef, cond: ps_dex::ConditionId) -> bool {
    b.mon(mon).volatiles.contains(EffectId::Condition(cond))
}

fn trap(b: &mut Battle) -> Relay {
    b.add_volatile_linked(P2, c::TRAPPED, P1, EffectId::Move(m::MEANLOOK), c::TRAPPER)
}

#[test]
fn mean_look_links_trapped_and_trapper() {
    let mut b = battle(UMBREON, SNORLAXES);
    assert_eq!(trap(&mut b), Relay::TRUE);
    assert!(has(&b, P2, c::TRAPPED) && has(&b, P1, c::TRAPPER));
    assert_eq!(b.linked_status(P2, EffectId::Condition(c::TRAPPED)), Some((c::TRAPPER, P1)));
    assert_eq!(b.linked_status(P1, EffectId::Condition(c::TRAPPER)), Some((c::TRAPPED, P2)));
    // A second Mean Look: `trapped` has no `onRestart`, so `false`.
    assert_eq!(trap(&mut b), Relay::FAIL);
    // The trapper switching out frees the target (`clearVolatile` unlinks).
    b.clear_volatile(P1, true);
    assert!(!has(&b, P2, c::TRAPPED));

    assert_eq!(trap(&mut b), Relay::TRUE);
    // The target switching out drops the trapper's half.
    b.clear_volatile(P2, true);
    assert!(!has(&b, P1, c::TRAPPER));
}

#[test]
fn a_fainted_source_links_nothing() {
    let mut b = battle(UMBREON, SNORLAXES);
    b.mon_mut(P1).hp = 0;
    assert_eq!(trap(&mut b), Relay::FAIL);
    assert!(!has(&b, P2, c::TRAPPED));
}

#[test]
fn removing_one_half_removes_the_other() {
    let mut b = battle(UMBREON, SNORLAXES);
    assert_eq!(trap(&mut b), Relay::TRUE);
    assert!(b.remove_volatile(P1, c::TRAPPER));
    assert!(!has(&b, P2, c::TRAPPED));
}

#[test]
fn baton_pass_moves_the_link() {
    let mut b = battle(UMBREON, SNORLAXES);
    assert_eq!(trap(&mut b), Relay::TRUE);
    // `copyVolatileFrom`: `trapped` is copied (no `noCopy`), the trapper's
    // list points at the new holder and the old one unlinks nothing.
    b.copy_volatile_from(P2_BENCH, P2);
    assert!(!has(&b, P2, c::TRAPPED) && has(&b, P2_BENCH, c::TRAPPED));
    assert!(has(&b, P1, c::TRAPPER));
    assert_eq!(b.linked_status(P1, EffectId::Condition(c::TRAPPER)), Some((c::TRAPPED, P2_BENCH)));
    b.clear_volatile(P2_BENCH, true);
    assert!(!has(&b, P1, c::TRAPPER));
}

#[test]
fn taunt_and_yawn_end_by_their_residual_keys() {
    let keys = |cond| ps_dex::GEN3.condition_def(cond).residual_order.map(|r| (r.keys.order, r.keys.sub_order));
    assert_eq!(keys(c::TAUNT), Some((10, 15)));
    assert_eq!(keys(c::YAWN), Some((10, 19)));
    let mut b = battle("Snorlax|||Stench|tackle|||||||", "Snorlax|||Stench|tackle|||||||");
    // Yawn first in insertion order; Taunt's sub-order 15 still ends first.
    assert_eq!(b.add_volatile(P1, c::YAWN, Some(P2), EffectId::Move(m::YAWN)), Relay::TRUE);
    assert_eq!(b.add_volatile(P1, c::TAUNT, Some(P2), EffectId::Move(m::TAUNT)), Relay::TRUE);
    for cond in [c::YAWN, c::TAUNT] {
        b.mon_mut(P1).volatiles.get_mut(EffectId::Condition(cond)).unwrap().duration = 1;
    }
    take_log(&mut b);
    b.field_event(EventId::Residual, None);
    let log = take_log(&mut b);
    let taunt = log.find("|-end|p1a: Snorlax|move: Taunt|[silent]").expect("taunt ends");
    let yawn = log.find("|-end|p1a: Snorlax|move: Yawn|[silent]").expect("yawn ends");
    assert!(taunt < yawn, "{log}");
    assert!(log.contains("|-status|p1a: Snorlax|slp"), "{log}");
}
