//! Weather as a field effect: `setWeather` (its state, duration, the
//! `FieldStart` refusal and the gen 3 rules for setting the same weather
//! again), `effectiveWeather` under Air Lock, and `clearWeather`, on gen 3
//! battles with a test registry.

extern crate std;

use std::boxed::Box;
use std::string::String;
use std::vec::Vec;

use ps_dex::gen3::{abilities as a, conditions as c, moves as m};
use ps_dex::ConditionId;
use ps_protocol::text::team::unpack;
use ps_protocol::{FormatSpec, SideId};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler, Registry};
use crate::relay::Relay;
use crate::state::{Battle, Holder, MonRef};

std::thread_local! {
    static CALLS: core::cell::RefCell<Vec<String>> = const { core::cell::RefCell::new(Vec::new()) };
    /// What the sandstorm's `FieldStart` answers.
    static REFUSE: core::cell::Cell<bool> = const { core::cell::Cell::new(false) };
}

fn record(what: &str) {
    CALLS.with(|c| c.borrow_mut().push(String::from(what)));
}

fn calls() -> Vec<String> {
    CALLS.with(|c| core::mem::take(&mut *c.borrow_mut()))
}

fn rain_duration(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    // `durationCallback(source, effect)`: the source is the event target.
    assert!(a.target.mon().is_some());
    let _ = b;
    record("Rain:durationCallback");
    Some(Relay::Int(5))
}
fn rain_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let st = *b.effect_state();
    assert_eq!(st.duration, 5);
    assert!(st.source.is_some() && st.source == a.source);
    record("Rain:FieldStart");
    None
}
fn rain_end(_: &mut Battle, _: &EventArgs) -> Option<Relay> {
    record("Rain:FieldEnd");
    None
}
fn noop(_: &mut Battle, _: &EventArgs) -> Option<Relay> {
    None
}
fn sand_start(_: &mut Battle, _: &EventArgs) -> Option<Relay> {
    record("Sand:FieldStart");
    REFUSE.with(|r| r.get()).then_some(Relay::FAIL)
}
fn forecast_weather_change(_: &mut Battle, _: &EventArgs) -> Option<Relay> {
    record("Forecast:WeatherChange");
    None
}

static RAIN: EffectDef = EffectDef::new(&[
    Handler::on(EventId::DurationCallback, rain_duration),
    Handler::on(EventId::WeatherModifyDamage, noop),
    Handler::on(EventId::FieldStart, rain_start),
    Handler::on(EventId::FieldResidual, noop),
    Handler::on(EventId::FieldEnd, rain_end),
]);
fn sand_duration(_: &mut Battle, _: &EventArgs) -> Option<Relay> {
    Some(Relay::Int(5))
}

static SAND: EffectDef = EffectDef::new(&[
    Handler::on(EventId::DurationCallback, sand_duration),
    Handler::on(EventId::FieldStart, sand_start),
    Handler::on(EventId::Immunity, noop),
    Handler::on(EventId::FieldResidual, noop),
    Handler::on(EventId::Weather, noop),
    Handler::on(EventId::FieldEnd, noop),
]);
static FORECAST: EffectDef = EffectDef::new(&[Handler::on(EventId::WeatherChange, forecast_weather_change)]);

static REG: Registry = Registry {
    entries: &[
        (EffectId::Condition(c::SANDSTORM), &SAND),
        (EffectId::Condition(c::RAINDANCE), &RAIN),
        (EffectId::Ability(a::FORECAST), &FORECAST),
    ],
};

#[test]
fn test_registry_is_sorted() {
    assert!(REG.is_sorted());
}

fn spill(_: &Battle, _: &[ps_protocol::Msg]) {}

fn battle(p1: &str, p2: &str) -> Box<Battle> {
    let mut b = Box::new(Battle::from_start(FormatSpec::gen3_harness(), [1, 2, 3, 4], 1_700_000_000));
    b.registry = &REG;
    b.lax_handlers = true;
    b.spill = Some(spill);
    for (side, team) in [(SideId::P1, p1), (SideId::P2, p2)] {
        b.set_player(side, "p", "", "", Some(&unpack(team).unwrap())).unwrap();
    }
    b.log.clear();
    calls();
    b
}

const P1: MonRef = MonRef { side: 0, index: 0 };
const P2: MonRef = MonRef { side: 1, index: 0 };

#[test]
fn set_weather_from_a_move() {
    let mut b = battle("Castform|||Forecast|raindance|||||||", "Snorlax|||Immunity|tackle|||||||");
    let r = b.set_weather(c::RAINDANCE, Some(P2), EffectId::Move(m::RAINDANCE));
    assert_eq!(r, Relay::TRUE);
    assert_eq!(b.field.weather, c::RAINDANCE);
    let st = b.field.weather_state;
    assert_eq!((st.duration, st.source, st.effect_order), (5, Some(P2), 0));
    assert_eq!(st.source_slot, Some(b.slot_of(P2)));
    // `durationCallback`, `FieldStart`, then `WeatherChange` on each active.
    assert_eq!(calls(), ["Rain:durationCallback", "Rain:FieldStart", "Forecast:WeatherChange"]);
    // Gen 3: a move cannot set the weather that is up.
    assert_eq!(b.set_weather(c::RAINDANCE, Some(P1), EffectId::Move(m::RAINDANCE)), Relay::FAIL);
    // An ability can, while the weather has turns left (Drizzle over Rain
    // Dance), but not over a weather without a duration.
    assert_eq!(b.set_weather(c::RAINDANCE, Some(P1), EffectId::Ability(a::DRIZZLE)), Relay::TRUE);
    b.field.weather_state.duration = 0;
    assert_eq!(b.set_weather(c::RAINDANCE, Some(P1), EffectId::Ability(a::DRIZZLE)), Relay::FAIL);
    calls();
    // `clearWeather`: `FieldEnd`, an emptied state, `WeatherChange`.
    let serial = b.field.weather_state.serial;
    assert!(b.clear_weather());
    assert_eq!(calls(), ["Rain:FieldEnd", "Forecast:WeatherChange"]);
    assert_eq!(b.field.weather, ConditionId::NONE);
    assert_eq!((b.field.weather_state.serial, b.field.weather_state.duration), (serial, 0));
    assert!(!b.clear_weather());
}

#[test]
fn a_refused_field_start_restores_the_weather() {
    let mut b = battle("Snorlax|||Immunity|tackle|||||||", "Snorlax|||Immunity|tackle|||||||");
    assert_eq!(b.set_weather(c::RAINDANCE, Some(P1), EffectId::Move(m::RAINDANCE)), Relay::TRUE);
    let before = b.field.weather_state;
    calls();
    REFUSE.with(|r| r.set(true));
    let r = b.set_weather(c::SANDSTORM, Some(P2), EffectId::Move(m::SANDSTORM));
    REFUSE.with(|r| r.set(false));
    assert_eq!(r, Relay::FAIL);
    assert_eq!(calls(), ["Sand:FieldStart"]);
    assert_eq!(b.field.weather, c::RAINDANCE);
    assert_eq!(b.field.weather_state, before);
}

#[test]
fn air_lock_hides_the_weather() {
    let mut b = battle("Rayquaza|||AirLock|tackle|||||||", "Snorlax|||Immunity|tackle|||||||");
    assert_eq!(b.set_weather(c::RAINDANCE, Some(P2), EffectId::Move(m::RAINDANCE)), Relay::TRUE);
    assert_eq!(b.field.weather, c::RAINDANCE);
    assert_eq!(b.effective_weather(), ConditionId::NONE);
    assert!(!b.is_weather(&[c::RAINDANCE]));
    assert_eq!(b.pokemon_effective_weather(P2), ConditionId::NONE);
    // The rain's handler is skipped with PS's debug line.
    b.log.clear();
    let r = b.run_event(EventId::WeatherModifyDamage, Holder::Mon(P1), Some(P2), EffectId::Move(m::SURF), Some(Relay::Int(100)));
    assert_eq!(r, Relay::Int(100));
    let tables = crate::text::dex_tables(b.dex);
    let log = ps_protocol::text::encode_log(b.log.as_slice(), &tables, &crate::text::Names(&b));
    assert_eq!(log, "|debug|WeatherModifyDamage handler suppressed by Air Lock");
    // Without Air Lock on the field.
    b.mon_mut(P1).ability = a::LEVITATE;
    assert!(b.is_weather(&[c::SUNNYDAY, c::RAINDANCE]));
    assert_eq!(b.pokemon_effective_weather(P2), c::RAINDANCE);
}
