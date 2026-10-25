//! The item and ability primitives, the switch-in ability order, gen 3
//! Transform's ability copy, the boosts relays of base `boost`, and Hidden
//! Power, on gen 3 battles built through `>start` / `>player` with a test
//! registry whose handlers record their calls.

extern crate std;

use std::boxed::Box;
use std::string::String;
use std::vec::Vec;

use ps_dex::gen3::{abilities as a, conditions as c, items as i, moves as m};
use ps_protocol::text::encode_log;
use ps_protocol::text::team::unpack;
use ps_protocol::{FormatSpec, SideId};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler, Registry};
use crate::relay::Relay;
use crate::state::{Battle, Holder, MonRef, Staleness, StateRef};
use crate::text::Names;

std::thread_local! {
    static CALLS: core::cell::RefCell<Vec<String>> = const { core::cell::RefCell::new(Vec::new()) };
}

fn record(b: &mut Battle, what: &str) {
    let st = *b.effect_state();
    CALLS.with(|c| c.borrow_mut().push(std::format!("{what} order={} serial={}", st.effect_order, st.serial)));
}

fn calls() -> Vec<String> {
    CALLS.with(|c| core::mem::take(&mut *c.borrow_mut()))
}

fn names(v: &[String]) -> Vec<&str> {
    v.iter().map(|s| s.split(' ').next().unwrap()).collect()
}

macro_rules! recorder {
    ($name:ident, $what:literal) => {
        fn $name(b: &mut Battle, _: &EventArgs) -> Option<Relay> {
            record(b, $what);
            None
        }
    };
    ($name:ident, $what:literal, $ret:expr) => {
        fn $name(b: &mut Battle, _: &EventArgs) -> Option<Relay> {
            record(b, $what);
            Some($ret)
        }
    };
}

recorder!(leftovers_start, "Leftovers:Start");
recorder!(leftovers_end, "Leftovers:End");
recorder!(leftovers_use, "Leftovers:Use");
recorder!(sitrus_eat, "Sitrus:Eat");
recorder!(sitrus_end, "Sitrus:End");
recorder!(choiceband_start, "ChoiceBand:Start");
recorder!(stickyhold_takeitem, "StickyHold:TakeItem", Relay::FAIL);
recorder!(pressure_start, "Pressure:Start");
recorder!(pressure_end, "Pressure:End");
recorder!(intimidate_start, "Intimidate:Start");
recorder!(trace_start, "Trace:Start");
recorder!(drizzle_hazard, "Drizzle:AfterEntryHazard");

/// Oblivious refuses `TryEatItem` (a stand-in for a real refusal).
fn oblivious_try_eat(b: &mut Battle, _: &EventArgs) -> Option<Relay> {
    record(b, "Oblivious:TryEatItem");
    Some(Relay::FAIL)
}

/// Insomnia refuses `SetAbility` with `null` (PS returns that as is).
fn insomnia_set_ability(b: &mut Battle, _: &EventArgs) -> Option<Relay> {
    record(b, "Insomnia:SetAbility");
    Some(Relay::Null)
}

/// Mist-like `TryBoost`: `delete boost[i]` for every drop.
fn whitesmoke_try_boost(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    record(b, "WhiteSmoke:TryBoost");
    let Some(Relay::Boosts(mut t)) = a.relay else { panic!("TryBoost relay {:?}", a.relay) };
    for v in t.iter_mut() {
        if *v < 0 {
            *v = 0;
        }
    }
    Some(Relay::Boosts(t))
}

/// Halves the weight (`ModifyWeight`), like Light Metal.
fn thickfat_weight(_: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let Some(Relay::Int(w)) = a.relay else { panic!() };
    Some(Relay::Int(w / 2))
}

recorder!(lum_update, "Lum:Update");

/// Pursuit's volatile, Pursuit-like: `onFoeBeforeSwitchOut` marks the
/// switching foe once.
fn pursuit_before_switch_out(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    record(b, "Pursuit:FoeBeforeSwitchOut");
    let target = a.target.mon().unwrap();
    b.mon_mut(target).pursuit_activated = true;
    None
}

static PURSUIT: EffectDef =
    EffectDef::new(&[Handler::on(EventId::BeforeSwitchOut, pursuit_before_switch_out).prefix(crate::effect::Prefix::Foe)]);
static LUM: EffectDef = EffectDef::new(&[Handler::on(EventId::Update, lum_update)]);

static LEFTOVERS: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, leftovers_start),
    Handler::on(EventId::End, leftovers_end),
    Handler::on(EventId::Use, leftovers_use),
]);
static SITRUS: EffectDef =
    EffectDef::new(&[Handler::on(EventId::Eat, sitrus_eat), Handler::on(EventId::End, sitrus_end)]);
static CHOICEBAND: EffectDef = EffectDef::new(&[Handler::on(EventId::Start, choiceband_start)]);
static STICKYHOLD: EffectDef = EffectDef::new(&[Handler::on(EventId::TakeItem, stickyhold_takeitem)]);
static OBLIVIOUS: EffectDef = EffectDef::new(&[Handler::on(EventId::TryEatItem, oblivious_try_eat)]);
static INSOMNIA: EffectDef = EffectDef::new(&[Handler::on(EventId::SetAbility, insomnia_set_ability)]);
static PRESSURE: EffectDef =
    EffectDef::new(&[Handler::on(EventId::Start, pressure_start), Handler::on(EventId::End, pressure_end)]);
static INTIMIDATE: EffectDef = EffectDef::new(&[Handler::on(EventId::Start, intimidate_start)]);
static TRACE: EffectDef = EffectDef::new(&[Handler::on(EventId::Start, trace_start)]);
static DRIZZLE: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, drizzle_hazard),
    Handler::on(EventId::AfterEntryHazard, drizzle_hazard),
]);
static WHITESMOKE: EffectDef = EffectDef::new(&[Handler::on(EventId::TryBoost, whitesmoke_try_boost)]);
static THICKFAT: EffectDef = EffectDef::new(&[Handler::on(EventId::ModifyWeight, thickfat_weight)]);

/// The test registry, sorted by id (the dex's order within a kind), with gen 3's real move handlers (the
/// pipeline's Struggle and Hidden Power).
static REG: Registry = Registry {
    entries: &[
        (EffectId::Move(m::HIDDENPOWER), &crate::mods::gen3::moves::hiddenpower::DEF),
        (EffectId::Move(m::STRUGGLE), &crate::mods::gen3::moves::struggle::DEF),
        (EffectId::Condition(c::PURSUIT), &PURSUIT),
        (EffectId::Ability(a::DRIZZLE), &DRIZZLE),
        (EffectId::Ability(a::INTIMIDATE), &INTIMIDATE),
        (EffectId::Ability(a::PRESSURE), &PRESSURE),
        (EffectId::Ability(a::TRACE), &TRACE),
        (EffectId::Ability(a::INSOMNIA), &INSOMNIA),
        (EffectId::Ability(a::STICKYHOLD), &STICKYHOLD),
        (EffectId::Ability(a::THICKFAT), &THICKFAT),
        (EffectId::Ability(a::OBLIVIOUS), &OBLIVIOUS),
        (EffectId::Ability(a::WHITESMOKE), &WHITESMOKE),
        (EffectId::Item(i::SITRUSBERRY), &SITRUS),
        (EffectId::Item(i::CHOICEBAND), &CHOICEBAND),
        (EffectId::Item(i::LEFTOVERS), &LEFTOVERS),
        (EffectId::Item(i::LUMBERRY), &LUM),
    ],
};

#[test]
fn test_registry_is_sorted() {
    assert!(REG.is_sorted());
}

fn spill(_: &Battle, _: &[ps_protocol::Msg]) {}

/// A started gen 3 custom game of the two packed teams on [`REG`], its log
/// and the recorded calls cleared.
fn battle(p1: &str, p2: &str) -> Box<Battle> {
    let mut b = Box::new(Battle::from_start(FormatSpec::gen3_harness(), [1, 2, 3, 4], 1_700_000_000));
    b.registry = &REG;
    b.lax_handlers = true;
    b.spill = Some(spill);
    for (side, team) in [(SideId::P1, p1), (SideId::P2, p2)] {
        let team = unpack(team).unwrap();
        b.set_player(side, "p", "", "", Some(&team)).unwrap();
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
fn eat_item_ends_the_item_and_tracks_staleness() {
    let mut b = battle("Snorlax||SitrusBerry|Immunity|tackle|||||||", "Snorlax||Leftovers|Immunity|tackle|||||||");
    calls();
    let serial = b.mon(P1).item_state.serial;
    b.mon_mut(P1).pending_staleness = Staleness::Internal;
    assert!(b.eat_item(P1, false, None, EffectId::None));
    assert_eq!(take_log(&mut b), "|-enditem|p1a: Snorlax|Sitrus Berry|[eat]\n");
    // `Eat` sees the item's live state, before it is cleared.
    let got = calls();
    assert_eq!(names(&got), ["Sitrus:Eat"]);
    assert!(got[0].ends_with(&std::format!("serial={serial}")));
    let mon = b.mon(P1);
    assert!(mon.item.is_none());
    assert_eq!(mon.last_item, i::SITRUSBERRY);
    assert!(mon.used_item_this_turn && mon.ate_berry);
    // The state object is kept, emptied.
    assert_eq!((mon.item_state.serial, mon.item_state.effect_order), (serial, 0));
    // A restorative berry turns the pending staleness into staleness.
    assert_eq!((mon.staleness, mon.pending_staleness), (Staleness::Internal, Staleness::None));
    // Nothing left to eat.
    assert!(!b.eat_item(P1, false, None, EffectId::None));
    // Leftovers is no berry, but `eatItem` does not ask.
    assert!(b.eat_item(P2, true, None, EffectId::None));
    assert_eq!(b.mon(P2).staleness, Staleness::None);
}

#[test]
fn eat_item_refusals() {
    let mut b = battle("Snorlax||SitrusBerry|Oblivious|tackle|||||||", "Snorlax||SitrusBerry|Immunity|tackle|||||||");
    calls();
    // `TryEatItem` refuses unless forced.
    assert!(!b.eat_item(P1, false, None, EffectId::None));
    assert_eq!(names(&calls()), ["Oblivious:TryEatItem"]);
    assert_eq!(b.mon(P1).item, i::SITRUSBERRY);
    assert!(b.eat_item(P1, true, None, EffectId::None));
    assert_eq!(names(&calls()), ["Sitrus:Eat"]);
    // An item telling its holder to eat while it holds another one.
    assert!(!b.eat_item(P2, false, Some(P2), EffectId::Item(i::LEFTOVERS)));
    // Fainted or benched Pokemon eat nothing.
    b.mon_mut(P2).hp = 0;
    assert!(!b.eat_item(P2, false, None, EffectId::None));
    assert!(take_log(&mut b).lines().count() == 1);
}

#[test]
fn use_item_writes_a_plain_enditem() {
    let mut b = battle("Snorlax||Leftovers|Immunity|tackle|||||||", "Snorlax|||Immunity|tackle|||||||");
    calls();
    assert!(b.use_item(P1, None, EffectId::None));
    assert_eq!(take_log(&mut b), "|-enditem|p1a: Snorlax|Leftovers\n");
    assert_eq!(names(&calls()), ["Leftovers:Use"]);
    let mon = b.mon(P1);
    assert!(mon.item.is_none() && mon.used_item_this_turn && !mon.ate_berry);
    assert_eq!(mon.last_item, i::LEFTOVERS);
    assert!(!b.use_item(P2, None, EffectId::None));
}

#[test]
fn take_item_and_its_refusals() {
    let mut b = battle("Snorlax||Leftovers|Immunity|tackle|||||||", "Snorlax||Leftovers|StickyHold|tackle|||||||");
    calls();
    let serial = b.mon(P1).item_state.serial;
    b.mon_mut(P1).pending_staleness = Staleness::External;
    assert_eq!(b.take_item(P1, Some(P2)), Ok(i::LEFTOVERS));
    // `End` gets the old state object, already cleared (effectOrder 0).
    assert_eq!(calls(), [std::format!("Leftovers:End order=0 serial={serial}")]);
    assert!(b.mon(P1).item.is_none());
    assert_eq!(b.mon(P1).pending_staleness, Staleness::None);
    // No message: the caller writes one.
    assert_eq!(take_log(&mut b), "");
    // Nothing to take is PS's `undefined`.
    assert_eq!(b.take_item(P1, Some(P2)), Err(Relay::Null));
    // `TakeItem` refuses.
    assert_eq!(b.take_item(P2, Some(P1)), Err(Relay::FAIL));
    assert_eq!(names(&calls()), ["StickyHold:TakeItem"]);
    assert_eq!(b.mon(P2).item, i::LEFTOVERS);
    // Gen 4's knocked-off refusal is not gen 3's.
    b.mon_mut(P2).item_knocked_off = true;
    assert_eq!(b.take_item(P2, Some(P1)), Err(Relay::FAIL));
    assert_eq!(names(&calls()), ["StickyHold:TakeItem"]);
}

#[test]
fn set_item_ends_the_old_item_and_starts_the_new() {
    let mut b = battle("Snorlax||Leftovers|Immunity|tackle|||||||", "Snorlax|||Immunity|tackle|||||||");
    calls();
    let old = b.mon(P1).item_state;
    // As Trick from the foe: external staleness.
    b.frame.effect = EffectId::Move(m::TRICK);
    assert!(b.set_item(P1, i::CHOICEBAND, Some(P2), EffectId::Move(m::TRICK)));
    b.frame = crate::state::EffectFrame::TOP;
    let got = calls();
    assert_eq!(names(&got), ["Leftovers:End", "ChoiceBand:Start"]);
    // `End` sees the replaced state object, `Start` the new one.
    assert!(got[0].ends_with(&std::format!("order={} serial={}", old.effect_order, old.serial)));
    let new = b.mon(P1).item_state;
    assert_ne!(new.serial, old.serial);
    assert!(new.effect_order > old.effect_order);
    assert!(got[1].ends_with(&std::format!("serial={}", new.serial)));
    assert_eq!(b.mon(P1).pending_staleness, Staleness::External);
    // Not Trick: internal.
    assert!(b.clear_item(P1));
    assert_eq!(b.mon(P1).pending_staleness, Staleness::Internal);
    assert!(b.mon(P1).item.is_none());
    assert_eq!(b.mon(P1).item_state.effect_order, 0);
    // A fainted Pokemon gets nothing.
    b.mon_mut(P2).hp = 0;
    assert!(!b.set_item(P2, i::LEFTOVERS, None, EffectId::None));
    assert_eq!(take_log(&mut b), "");
}

#[test]
fn set_ability_writes_the_line_and_returns_the_old_ability() {
    let mut b = battle("Dusclops|||Pressure|tackle|||||||", "Snorlax|||Insomnia|tackle|||||||");
    calls();
    let old = b.mon(P1).ability_state;
    // As Role Play.
    let r = b.set_ability(P1, a::INTIMIDATE, Some(P2), EffectId::Move(m::ROLEPLAY), false, false);
    assert_eq!(r, Relay::Effect(EffectId::Ability(a::PRESSURE)));
    assert_eq!(take_log(&mut b), "|-ability|p1a: Dusclops|Intimidate|Pressure|[from] move: Role Play|[of] p2a: Snorlax\n");
    // The old ability's `End` with its state; gen 3 starts no ability here.
    let got = calls();
    assert_eq!(names(&got), ["Pressure:End"]);
    assert!(got[0].ends_with(&std::format!("serial={}", old.serial)));
    assert_eq!(b.mon(P1).ability, a::INTIMIDATE);
    assert_ne!(b.mon(P1).ability_state.serial, old.serial);
    // `SetAbility` refusing with `null` is returned as is.
    let r = b.set_ability(P2, a::PRESSURE, Some(P1), EffectId::Move(m::SKILLSWAP), false, false);
    assert_eq!(r, Relay::Null);
    assert_eq!(names(&calls()), ["Insomnia:SetAbility"]);
    assert_eq!(b.mon(P2).ability, a::INSOMNIA);
    // From a forme change (Transform's way): no event, no line.
    let r = b.set_ability(P2, a::PRESSURE, Some(P2), EffectId::None, true, true);
    assert_eq!(r, Relay::Effect(EffectId::Ability(a::INSOMNIA)));
    assert_eq!(take_log(&mut b), "");
    assert!(calls().is_empty());
    // Fainted.
    b.mon_mut(P2).hp = 0;
    assert_eq!(b.clear_ability(P2), Relay::FAIL);
}

#[test]
fn get_weight_runs_modify_weight() {
    let mut b = battle("Snorlax|||ThickFat|tackle|||||||", "Pikachu|||Static|tackle|||||||");
    assert_eq!(b.mon(P1).weighthg, 4600);
    assert_eq!(b.get_weight(P1), 2300);
    b.mon_mut(P1).weighthg = 1;
    assert_eq!(b.get_weight(P1), 1, "at least 1");
}

#[test]
fn gen3_transform_copies_weight_and_ability() {
    let mut b = battle("Ditto|||Limber|transform|||||||", "Dusclops|||Pressure|tackle|||||||");
    calls();
    assert!(b.transform_into(P1, P2));
    assert_eq!(take_log(&mut b), "|-transform|p1a: Ditto|p2a: Dusclops\n");
    assert_eq!(b.mon(P1).weighthg, b.mon(P2).weighthg);
    assert_eq!(b.mon(P1).ability, a::PRESSURE);
    // Limber has no handlers here; Pressure is not started (gen 3).
    assert!(calls().is_empty());
}

/// Gen 3 Knock Off's item loss on every switch-in.
#[test]
fn a_knocked_off_item_is_lost_on_switch_in() {
    let mut b = battle(
        "Snorlax|||Immunity|tackle|||||||]Pikachu||Leftovers|Static|tackle|||||||",
        "Snorlax|||Immunity|tackle|||||||",
    );
    let bench = MonRef::new(0, 1);
    b.mon_mut(bench).item_knocked_off = true;
    assert!(b.switch_in(bench, 0, EffectId::None, false));
    let log = take_log(&mut b);
    let lines: Vec<&str> = log.lines().collect();
    assert_eq!(lines[0], "|-hint|In Gens 3-4, Knock Off makes a Pokémon lose its held item every time it switches in.", "{log}");
    assert!(lines[1..].iter().any(|l| l.starts_with("|switch|p1a: Pikachu|")), "{log}");
    assert!(b.mon(bench).item.is_none());
    assert!(b.mon(bench).item_knocked_off);
}

/// `SwitchIn` answered by `onStart` is ordered by the `onSwitchIn` keys:
/// Pressure (3) before Intimidate (2) before Trace (1), whatever the
/// speeds.
#[test]
fn switch_in_abilities_follow_their_switch_in_priority() {
    // Trace's Pokemon is the faster.
    let mut b = battle("Alakazam|||Trace|tackle|||||||", "Gyarados|||Intimidate|tackle|||||||");
    let _ = &mut b;
    assert_eq!(names(&calls()), ["Intimidate:Start", "Trace:Start"]);
    let _ = battle("Alakazam|||Trace|tackle|||||||", "Dusclops|||Pressure|tackle|||||||");
    assert_eq!(names(&calls()), ["Pressure:Start", "Trace:Start"]);
    // Turn 0: Drizzle answers with `onAfterEntryHazard` (priority 3) and
    // goes before Intimidate.
    let _ = battle("Gyarados|||Intimidate|tackle|||||||", "Kyogre|||Drizzle|tackle|||||||");
    assert_eq!(names(&calls()), ["Drizzle:AfterEntryHazard", "Intimidate:Start"]);
}

/// After turn 0 a gen 3 weather ability runs from `switchIn`
/// (`AfterEntryHazard`), not from the `SwitchIn` event.
#[test]
fn weather_abilities_start_from_switch_in_after_turn_0() {
    let mut b = battle(
        "Snorlax|||Immunity|tackle|||||||]Kyogre|||Drizzle|tackle|||||||",
        "Snorlax|||Immunity|tackle|||||||",
    );
    calls();
    b.turn = 1;
    assert!(b.switch_in(MonRef::new(0, 1), 0, EffectId::None, false));
    assert_eq!(names(&calls()), ["Drizzle:AfterEntryHazard"]);
    // The queued `runSwitch` runs `SwitchIn`, which Drizzle no longer answers.
    let a = b.queue.shift().unwrap();
    b.run_action(a);
    assert!(calls().is_empty());
}

#[test]
fn boost_relays_and_lines() {
    let mut b = battle("Snorlax|||WhiteSmoke|tackle|||||||", "Snorlax|||Immunity|tackle|||||||");
    calls();
    // A drop removed by `TryBoost` writes nothing; the rise goes through.
    b.boost([1, -1, 0, 0, 0, 0, 0], P1, Some(P2), EffectId::Move(m::TACKLE));
    assert_eq!(names(&calls()), ["WhiteSmoke:TryBoost"]);
    assert_eq!(take_log(&mut b), "|-boost|p1a: Snorlax|atk|1\n");
    assert_eq!(b.mon(P1).boosts[..2], [1, 0]);
    // A stat at its cap is still a key: `+0` is written.
    b.mon_mut(P2).boosts[0] = 6;
    b.boost([1, 0, 0, 0, 0, 0, 0], P2, Some(P2), EffectId::Move(m::SWORDSDANCE));
    assert_eq!(take_log(&mut b), "|-boost|p2a: Snorlax|atk|0\n");
    // From an item and from an ability.
    b.mon_mut(P2).boosts[0] = 0;
    b.boost([0, 0, 0, 0, 1, 0, 0], P2, Some(P2), EffectId::Item(i::SALACBERRY));
    assert_eq!(take_log(&mut b), "|-boost|p2a: Snorlax|spe|1|[from] item: Salac Berry\n");
    b.boost([-1, 0, 0, 0, 0, 0, 0], P2, Some(P1), EffectId::Ability(a::INTIMIDATE));
    assert_eq!(take_log(&mut b), "|-ability|p2a: Snorlax|Intimidate|boost\n|-unboost|p2a: Snorlax|atk|1\n");
    // Belly Drum sets the stage.
    b.boost([12, 0, 0, 0, 0, 0, 0], P2, Some(P2), EffectId::Move(m::BELLYDRUM));
    assert_eq!(take_log(&mut b), "|-setboost|p2a: Snorlax|atk|6|[from] move: Belly Drum\n");
}

#[test]
fn hidden_power_type_power_and_category() {
    use crate::actions::ActiveMove;
    use ps_dex::{MoveCategory, Type};
    // IVs for Hidden Power Fire 70 (hp/atk/def/spa/spd/spe 31/30/31/30/31/30).
    let mut b = battle("Snorlax|||Immunity|hiddenpower||||,30,,30,,30|||", "Snorlax|||Immunity|tackle|||||||");
    assert_eq!((b.mon(P1).hp_type, b.mon(P1).hp_power), (Type::Fire, 70));
    let mv = b.new_active_move(ActiveMove::from_dex(b.dex, m::HIDDENPOWER));
    b.set_active_move(Some(mv), Some(P1), Some(P2));
    let effect = EffectId::Move(m::HIDDENPOWER);
    let relay = b.mv(mv).relay();
    b.single_event(EventId::ModifyMove, effect, StateRef::Scratch, 0, Holder::Mon(P1), Some(P2), effect, Some(relay));
    assert_eq!((b.mv(mv).type_, b.mv(mv).category), (Type::Fire, MoveCategory::Special));
    let bp = b.move_callback(mv, EventId::BasePowerCallback, P1, Some(P2));
    assert_eq!(bp, Some(Relay::Int(70)));
    assert_eq!(take_log(&mut b), "|debug|BP: 70\n");
    // A physical type.
    b.mon_mut(P1).hp_type = Type::Bug;
    b.single_event(EventId::ModifyMove, effect, StateRef::Scratch, 0, Holder::Mon(P1), Some(P2), effect, Some(relay));
    assert_eq!((b.mv(mv).type_, b.mv(mv).category), (Type::Bug, MoveCategory::Physical));
}

/// Gen 3's `Update` points around a move: one `eachEvent('Update')` per
/// strike of a multi-hit move, one after the move's damage, and one at the
/// end of the action (`runAction`, gen < 5, not after a switch).
#[test]
fn update_runs_per_strike_after_the_hit_and_after_the_action() {
    use crate::queue::{Action, ActionKind, ActionMove};
    let mut b = battle("Snorlax||LumBerry|Immunity|swift,doublekick|||||||", "Snorlax|||Immunity|swift|||||||");
    calls();
    b.run_move(ActionMove::Move(m::SWIFT), P1, 1, EffectId::None);
    let log = take_log(&mut b);
    assert_eq!(names(&calls()), ["Lum:Update"], "{log}");
    b.run_move(ActionMove::Move(m::DOUBLEKICK), P1, 1, EffectId::None);
    assert_eq!(names(&calls()), ["Lum:Update", "Lum:Update", "Lum:Update"]);
    // The action's own `Update`: the queue holds the foe's move after it.
    let mut mine = Action::new(ActionKind::Move).with_pokemon(P1);
    mine.move_ = ActionMove::Move(m::SWIFT);
    mine.target_loc = 1;
    let mut theirs = Action::new(ActionKind::Move).with_pokemon(P2);
    theirs.move_ = ActionMove::Move(m::SWIFT);
    theirs.target_loc = 1;
    b.queue.clear();
    b.queue.push(theirs);
    b.run_action(mine);
    assert_eq!(names(&calls()), ["Lum:Update", "Lum:Update"]);
}

/// Pursuit's switch hook: `BeforeSwitchOut` (the foe's `onFoe` handler)
/// runs once per switch-out, before the switch, and not for a drag.
#[test]
fn pursuit_hook_runs_once_per_switch_out() {
    let mut b = battle(
        "Snorlax|||Immunity|tackle|||||||]Pikachu|||Static|tackle|||||||]Raichu|||Static|tackle|||||||",
        "Snorlax|||Immunity|pursuit|||||||",
    );
    calls();
    assert_eq!(b.add_volatile(P2, c::PURSUIT, Some(P2), EffectId::Move(m::PURSUIT)), Relay::TRUE);
    b.mon_mut(P1).switch_flag = crate::state::SwitchFlag::Yes;
    b.run_pursuit_activation(P1);
    assert_eq!(names(&calls()), ["Pursuit:FoeBeforeSwitchOut"]);
    assert!(b.mon(P1).pursuit_activated && b.mon(P1).skip_pursuit);
    // The switch itself does not ask again; the flags go with the switch.
    assert!(b.switch_in(MonRef::new(0, 1), 0, EffectId::None, false));
    assert!(calls().is_empty());
    assert!(!b.mon(P1).pursuit_activated && !b.mon(P1).skip_pursuit);
    // A drag skips the hook.
    let pika = MonRef::new(0, 1);
    assert!(b.switch_in(MonRef::new(0, 2), 0, EffectId::None, true));
    assert!(calls().is_empty());
    assert!(!b.mon(pika).is_active);
}
