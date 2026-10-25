//! Unit tests for the gen 3 conditions the protocol diff cannot reach yet
//! (their only sources are other batches' moves and items: `trapped`,
//! `choicelock`, `stall`, `twoturnmove`, `futuremove`) and for the lines
//! an ability source gives a status or a weather. Battles are gen 3
//! custom games on the real gen 3 registry, started through `>start` /
//! `>player`.

extern crate std;

use std::boxed::Box;
use std::string::String;

use ps_dex::gen3::{abilities as a, conditions as c, items as i, moves as m};
use ps_protocol::text::encode_log;
use ps_protocol::text::team::unpack;
use ps_protocol::{FormatSpec, SideId};

use crate::effect::{EffectId, EventId};
use crate::queue::ActionMove;
use crate::relay::Relay;
use crate::state::{Battle, Holder, MonRef, Trapped};
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

const P1: MonRef = MonRef { side: 0, index: 0 };
const P2: MonRef = MonRef { side: 1, index: 0 };

#[test]
fn trapped_activates_and_traps() {
    let mut b = battle("Umbreon|||Levitate|meanlook|||||||", "Snorlax|||Stench|tackle|||||||");
    let r = b.add_volatile(P2, c::TRAPPED, Some(P1), EffectId::Move(m::MEANLOOK));
    assert_eq!(r, Relay::TRUE);
    assert_eq!(take_log(&mut b), "|-activate|p2a: Snorlax|trapped\n");
    b.mon_mut(P2).trapped = Trapped::No;
    b.run_event(EventId::TrapPokemon, Holder::Mon(P2), None, EffectId::None, None);
    assert_eq!(b.mon(P2).trapped, Trapped::Yes);
    // `tryTrap(true)` on a trapped Pokemon keeps the visible trap.
    assert!(b.try_trap(P2, true));
    assert_eq!(b.mon(P2).trapped, Trapped::Yes);
}

#[test]
fn choicelock_needs_a_last_move_and_disables_the_others() {
    let mut b = battle("Snorlax||ChoiceBand|Stench|tackle,bodyslam,earthquake|||||||", "Snorlax|||Stench|tackle|||||||");
    // Choice Band's own handlers belong to another batch.
    b.lax_handlers = true;
    // `if (!pokemon.lastMove) return false`.
    assert_eq!(b.add_volatile(P1, c::CHOICELOCK, None, EffectId::Item(i::CHOICEBAND)), Relay::FAIL);
    assert!(!b.mon(P1).volatiles.contains(EffectId::Condition(c::CHOICELOCK)));
    b.mon_mut(P1).last_move = m::BODYSLAM;
    assert_eq!(b.add_volatile(P1, c::CHOICELOCK, None, EffectId::Item(i::CHOICEBAND)), Relay::TRUE);
    b.run_event(EventId::DisableMove, Holder::Mon(P1), None, EffectId::None, None);
    let disabled: std::vec::Vec<u8> = b.mon(P1).move_slots[..3].iter().map(|s| s.disabled).collect();
    assert_eq!(disabled, [1, 0, 1]);
    // Without a Choice item the lock goes.
    b.mon_mut(P1).item = ps_dex::ItemId::NONE;
    b.run_event(EventId::DisableMove, Holder::Mon(P1), None, EffectId::None, None);
    assert!(!b.mon(P1).volatiles.contains(EffectId::Condition(c::CHOICELOCK)));
}

#[test]
fn stall_halves_the_chance_up_to_counter_max() {
    let mut b = battle("Snorlax|||Stench|protect|||||||", "Snorlax|||Stench|tackle|||||||");
    assert_eq!(b.add_volatile(P1, c::STALL, None, EffectId::Move(m::PROTECT)), Relay::TRUE);
    let st = |b: &Battle| *b.mon(P1).volatiles.get(EffectId::Condition(c::STALL)).unwrap();
    assert_eq!((st(&b).counter, st(&b).duration), (2, 2));
    let calls = b.prng.calls;
    let r = b.run_event(EventId::StallMove, Holder::Mon(P1), None, EffectId::None, None);
    assert!(matches!(r, Relay::Bool(_)));
    assert_eq!(b.prng.calls, calls + 1);
    assert_eq!(take_log(&mut b), "|debug|Success chance: 50%\n");
    for want in [4, 8, 8] {
        b.mon_mut(P1).volatiles.get_mut(EffectId::Condition(c::STALL)).unwrap().duration = 1;
        b.add_volatile(P1, c::STALL, None, EffectId::Move(m::PROTECT));
        assert_eq!((st(&b).counter, st(&b).duration), (want, 2));
    }
    b.run_event(EventId::StallMove, Holder::Mon(P1), None, EffectId::None, None);
    assert_eq!(take_log(&mut b), "|debug|Success chance: 13%\n");
}

#[test]
fn twoturnmove_adds_the_moves_volatile_and_locks() {
    let mut b = battle("Snorlax|||Stench|solarbeam,dig|||||||", "Snorlax|||Stench|tackle|||||||");
    b.mon_mut(P1).last_move_target_loc = 1;
    assert_eq!(b.add_volatile(P1, c::TWOTURNMOVE, Some(P2), EffectId::Move(m::SOLARBEAM)), Relay::TRUE);
    // Solar Beam has no condition: a handler-less volatile keyed by the move.
    let key = b.move_volatile_key(m::SOLARBEAM);
    assert_eq!(key, EffectId::Move(m::SOLARBEAM));
    let own = *b.mon(P1).volatiles.get(key).expect("the move's volatile");
    assert_eq!(own.data[super::twoturnmove::TARGET_LOC], 1);
    assert_eq!(b.get_locked_move(P1), Some(ActionMove::Move(m::SOLARBEAM)));
    // The move's volatile answers no event (Solar Beam's own handlers are
    // the move's, not the volatile's).
    b.run_event(EventId::BasePower, Holder::Mon(P1), Some(P2), EffectId::None, Some(Relay::Int(120)));
    // `onEnd` takes the move's volatile with it.
    assert!(b.remove_volatile(P1, c::TWOTURNMOVE));
    assert!(!b.mon(P1).volatiles.contains(key));
    assert_eq!(b.get_locked_move(P1), None);

    // Dig's volatile is its own condition; `MoveAborted` drops the charge.
    assert_eq!(b.move_volatile_key(m::DIG), EffectId::Condition(c::DIG));
    b.lax_handlers = true; // Dig's condition belongs to another batch.
    b.add_volatile(P1, c::TWOTURNMOVE, Some(P2), EffectId::Move(m::DIG));
    assert!(b.mon(P1).volatiles.contains(EffectId::Condition(c::DIG)));
    b.run_event(EventId::MoveAborted, Holder::Mon(P1), Some(P2), EffectId::Move(m::DIG), None);
    assert!(!b.mon(P1).volatiles.contains(EffectId::Condition(c::TWOTURNMOVE)));
    assert!(!b.mon(P1).volatiles.contains(EffectId::Condition(c::DIG)));
}

#[test]
fn futuremove_lands_on_its_turn_and_skips_fainted_targets() {
    let mut b = battle("Snorlax|||Stench|tackle|||||||", "Snorlax|||Stench|tackle|||||||");
    b.turn = 4;
    assert_eq!(b.add_slot_condition(1, 0, c::FUTUREMOVE, Some(P1), EffectId::Move(m::FUTURESIGHT)), Relay::TRUE);
    // What Future Sight's `onTry` fills in.
    let key = EffectId::Condition(c::FUTUREMOVE);
    {
        let st = b.sides[1].slot_conditions[0].get_mut(key).unwrap();
        st.duration = 3;
        st.move_ = m::FUTURESIGHT;
        st.source = Some(P1);
    }
    let st = *b.sides[1].slot_conditions[0].get(key).unwrap();
    assert_eq!(st.data[super::futuremove::ENDING_TURN], 5);
    assert_eq!((st.data[super::futuremove::TARGET_SIDE], st.data[super::futuremove::TARGET_POSITION]), (1, 0));
    // `onRestart` refuses a second one.
    assert_eq!(b.add_slot_condition(1, 0, c::FUTUREMOVE, Some(P1), EffectId::Move(m::FUTURESIGHT)), Relay::FAIL);
    // Too early: the residual leaves it alone.
    b.turn = 5;
    b.field_event(EventId::Residual, None);
    assert!(b.sides[1].slot_conditions[0].contains(key));
    take_log(&mut b);
    // A fainted target is not hit.
    b.mon_mut(P2).fainted = true;
    b.turn = 6;
    b.field_event(EventId::Residual, None);
    assert!(!b.sides[1].slot_conditions[0].contains(key));
    assert_eq!(take_log(&mut b), "|-hint|Future Sight did not hit because the target is fainted.\n");
}

#[test]
fn ability_sources_on_status_and_weather_lines() {
    let mut b = battle("Pikachu|||Levitate|tackle|||||||", "Snorlax|||Stench|tackle|||||||");
    assert_eq!(b.set_status(P2, c::PAR, Some(P1), EffectId::Ability(a::STATIC), false), Relay::TRUE);
    assert_eq!(take_log(&mut b), "|-status|p2a: Snorlax|par|[from] ability: Static|[of] p1a: Pikachu\n");
    // An ability's weather is endless in gen 3.
    assert_eq!(b.set_weather(c::RAINDANCE, Some(P1), EffectId::Ability(a::DRIZZLE)), Relay::TRUE);
    assert_eq!(take_log(&mut b), "|-weather|RainDance|[from] ability: Drizzle|[of] p1a: Pikachu\n");
    assert_eq!(b.field.weather_state.duration, 0);
    // A move's lasts five turns.
    assert_eq!(b.set_weather(c::SANDSTORM, Some(P1), EffectId::Move(m::SANDSTORM)), Relay::TRUE);
    assert_eq!(b.field.weather_state.duration, 5);
    assert_eq!(take_log(&mut b), "|-weather|Sandstorm\n");
}
