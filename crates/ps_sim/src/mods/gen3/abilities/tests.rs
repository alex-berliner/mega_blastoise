//! Unit tests for the gen 3 abilities: the lines and state the protocol
//! diff reaches only by chance (a one-sided split, `[of]` on `cant`, the
//! forme change), the random-battle ability-slot trap check, and the
//! handlers no singles battle runs. Battles are gen 3 custom games on the
//! real gen 3 registry, started through `>start` / `>player`.

extern crate std;

use std::boxed::Box;
use std::string::String;

use ps_dex::gen3::{abilities as a, conditions as c, moves as m};
use ps_protocol::text::encode_log;
use ps_protocol::text::team::unpack;
use ps_protocol::{Choice, Command, FormatId, FormatSpec, SideId};

use crate::effect::{EffectId, EventId};
use crate::relay::Relay;
use crate::state::{Battle, Holder, MonRef};
use crate::text::Names;

fn spill(_: &Battle, _: &[ps_protocol::Msg]) {}

/// A started gen 3 custom game of the two packed teams; the log of the
/// start is kept.
fn battle(p1: &str, p2: &str) -> Box<Battle> {
    let mut b = Box::new(Battle::from_start(FormatSpec::gen3_harness(), [1, 2, 3, 4], 1_700_000_000));
    b.spill = Some(spill);
    for (side, team) in [(SideId::P1, p1), (SideId::P2, p2)] {
        b.set_player(side, "p", "", "", Some(&unpack(team).unwrap())).unwrap();
    }
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

/// The start's lines between the leads' `|switch|` pairs and `|turn|1`.
fn opening(b: &mut Battle) -> String {
    let log = take_log(b);
    let from = log.find("|start\n").expect("started");
    let to = log.find("|turn|1").expect("turn 1");
    let lines: std::vec::Vec<&str> = log[from + "|start\n".len()..to].split_inclusive('\n').collect();
    let mut out = String::new();
    for (i, l) in lines.iter().enumerate() {
        let next_is_switch = lines.get(i + 1).is_some_and(|n| n.starts_with("|switch|"));
        if l.starts_with("|switch|") || (l.starts_with("|split|") && next_is_switch) {
            continue;
        }
        out.push_str(l);
    }
    out
}

fn moves(b: &mut Battle, p1: u8, p2: u8) {
    b.step(&Command::Choose { side: SideId::P1, choice: Choice::Move(p1) }).unwrap();
    b.step(&Command::Choose { side: SideId::P2, choice: Choice::Move(p2) }).unwrap();
}

const P1: MonRef = MonRef { side: 0, index: 0 };
const P2: MonRef = MonRef { side: 1, index: 0 };

#[test]
fn pressure_announces_itself_to_its_side_only() {
    let mut b = battle("Zapdos|||Pressure|thunderbolt|||||||", "Snorlax|||Stench|tackle|||||||");
    let open = opening(&mut b);
    assert!(
        open.ends_with("|split|p1\n|-ability|p1a: Zapdos|Pressure|[silent]\n\n"),
        "{open}"
    );
}

#[test]
fn pressure_costs_a_foe_one_more_pp() {
    let mut b = battle("Zapdos|||Pressure|thunderbolt|||||||", "Snorlax|||Stench|tackle|||||||");
    moves(&mut b, 1, 1);
    // Tackle's 56 PP (35 with PP Ups) lose 2; Zapdos's own move loses 1.
    let tackle = b.mon(P2).move_slots[0];
    assert_eq!(i32::from(tackle.maxpp) - i32::from(tackle.pp), 2);
    let tbolt = b.mon(P1).move_slots[0];
    assert_eq!(i32::from(tbolt.maxpp) - i32::from(tbolt.pp), 1);
}

#[test]
fn intimidate_lowers_attack_and_trace_copies_with_a_prng_call() {
    let mut b = battle("Gyarados|||Intimidate|tackle|||||||", "Gardevoir|||Trace|psychic|||||||");
    let open = opening(&mut b);
    // Intimidate (2) before Trace (1); the traced Intimidate does not start
    // in gen 3.
    assert_eq!(
        open,
        "|-ability|p1a: Gyarados|Intimidate|boost\n\
         |-unboost|p2a: Gardevoir|atk|1\n\
         |-ability|p2a: Gardevoir|Intimidate|Trace|[from] ability: Trace|[of] p1a: Gyarados\n"
    );
    assert_eq!(b.mon(P2).ability, a::INTIMIDATE);
    assert_eq!(b.mon(P1).boosts[0], 0);
}

#[test]
fn intimidate_does_not_activate_behind_a_substitute() {
    let mut b = battle("Snorlax|||Stench|tackle|||||||", "Snorlax|||Stench|tackle|||||||");
    take_log(&mut b);
    // Substitute is another batch's: the volatile is enough here.
    let st = b.init_effect_state(true, Holder::Mon(P2));
    b.mon_mut(P2).volatiles.insert(EffectId::Condition(c::SUBSTITUTE), st);
    b.mon_mut(P1).ability = a::INTIMIDATE;
    let state = b.mon(P1).ability_state;
    b.single_event(EventId::Start, EffectId::Ability(a::INTIMIDATE), crate::state::StateRef::Ability(P1), state.serial, Holder::Mon(P1), None, EffectId::None, None);
    assert_eq!(
        take_log(&mut b),
        "|split|p1\n|-hint|In Gen 3, Intimidate does not activate if every target has a Substitute.\n\n"
    );
    assert_eq!(b.mon(P2).boosts[0], 0);
}

#[test]
fn damp_stops_explosion_with_its_user_named() {
    let mut b = battle("Golduck|||Damp|surf|||||||", "Electrode|||Stench|explosion|||||||");
    take_log(&mut b);
    moves(&mut b, 1, 1);
    let log = take_log(&mut b);
    // `[still]` hides the target, as `attrLastMove('[still]')` does.
    assert!(
        log.contains("|move|p2a: Electrode|Explosion||[still]\n|cant|p1a: Golduck|ability: Damp|Explosion|[of] p2a: Electrode\n"),
        "{log}"
    );
    assert!(b.mon(P2).hp > 0);
}

#[test]
fn forecast_follows_the_weather() {
    let mut b = battle("Castform|||Forecast|tackle|||||||", "Kyogre|||Drizzle|surf|||||||");
    let open = opening(&mut b);
    assert_eq!(
        open,
        "|-weather|RainDance|[from] ability: Drizzle|[of] p2a: Kyogre\n\
         |-formechange|p1a: Castform|Castform-Rainy|[msg]|[from] ability: Forecast\n"
    );
    assert_eq!(b.mon(P1).types.0[0], ps_dex::Type::Water);
    // Back to Castform when the weather goes.
    b.clear_weather();
    let log = take_log(&mut b);
    assert!(log.ends_with("|-formechange|p1a: Castform|Castform|[msg]|[from] ability: Forecast\n"), "{log}");
    assert_eq!(b.mon(P1).types.0[0], ps_dex::Type::Normal);
}

#[test]
fn air_lock_ending_tells_forecast() {
    let mut b = battle("Castform|||Forecast|tackle|||||||", "Kyogre|||Drizzle|surf|||||||");
    take_log(&mut b);
    // Rayquaza's Air Lock arrives: the rain no longer counts...
    b.mon_mut(P2).ability = a::AIRLOCK;
    b.each_event(EventId::WeatherChange, EffectId::Ability(a::AIRLOCK), None);
    assert!(take_log(&mut b).ends_with("|Castform|[msg]|[from] ability: Forecast\n"));
    // ...and when it ends (`ending`), it does again.
    let state = b.mon(P2).ability_state;
    b.single_event(EventId::End, EffectId::Ability(a::AIRLOCK), crate::state::StateRef::Ability(P2), state.serial, Holder::Mon(P2), None, EffectId::None, None);
    assert!(b.mon(P2).ability_state.ending);
    assert!(take_log(&mut b).ends_with("|Castform-Rainy|[msg]|[from] ability: Forecast\n"));
}

#[test]
fn truant_loafs_every_other_turn() {
    let mut b = battle("Slaking|||Truant|tackle|||||||", "Snorlax|||Stench|tackle|||||||");
    // Turn 0: no loafing turn yet.
    assert!(!b.mon(P1).truant_turn);
    moves(&mut b, 1, 1);
    // The residual flipped it: the next turn is the loafing one.
    assert!(b.mon(P1).truant_turn);
    assert!(b.mon(P1).volatiles.contains(EffectId::Condition(c::TRUANT)));
    take_log(&mut b);
    moves(&mut b, 1, 1);
    assert!(take_log(&mut b).contains("|cant|p1a: Slaking|ability: Truant\n"));
    assert!(!b.mon(P1).truant_turn);
}

#[test]
fn own_tempo_names_confusion_on_its_immune_line() {
    let mut b = battle("Slowbro|||OwnTempo|surf|||||||", "Gengar|||Levitate|confuseray|||||||");
    take_log(&mut b);
    moves(&mut b, 1, 1);
    let log = take_log(&mut b);
    assert!(log.contains("|-immune|p1a: Slowbro|confusion|[from] ability: Own Tempo\n"), "{log}");
    assert!(log.contains("|debug|add volatile [confusion] interrupted\n"), "{log}");
}

#[test]
fn water_absorb_heals_with_the_attacker_named() {
    let mut b = battle("Vaporeon|||WaterAbsorb|tackle|||||||", "Starmie|||Illuminate|surf|||||||");
    take_log(&mut b);
    // Full HP: `-immune`.
    moves(&mut b, 1, 1);
    assert!(take_log(&mut b).contains("|-immune|p1a: Vaporeon|[from] ability: Water Absorb\n"));
    b.mon_mut(P1).hp -= 100;
    moves(&mut b, 1, 1);
    let log = take_log(&mut b);
    assert!(log.contains("|[from] ability: Water Absorb|[of] p2a: Starmie\n"), "{log}");
}

#[test]
fn the_heal_of_a_tiny_quarter_is_one() {
    let mut b = battle("Snorlax|||Stench|tackle|||||||", "Snorlax|||Stench|tackle|||||||");
    b.mon_mut(P1).base_maxhp = 3;
    assert_eq!(super::quarter_base_maxhp(&b, P1), 1);
    b.mon_mut(P1).base_maxhp = 403;
    assert_eq!(super::quarter_base_maxhp(&b, P1), 100);
}

#[test]
fn magnet_pull_traps_steel_types_only() {
    let mut b = battle("Magneton|||MagnetPull|thunderbolt|||||||", "Skarmory|||EarlyBird|drillpeck|||||||");
    // `endTurn` (turn 0 included) asks `TrapPokemon`.
    assert!(b.mon(P2).trapped.truthy());
    let mut b = battle("Magneton|||MagnetPull|thunderbolt|||||||", "Snorlax|||Stench|tackle,bodyslam|||||||]Pikachu|||Stench|tackle|||||||");
    assert!(!b.mon(P2).trapped.truthy());
    assert!(!b.mon(P2).maybe_trapped);
}

#[test]
fn random_battles_ask_every_ability_slot_of_the_foe_species() {
    // Dugtrio holding Stench: its species' slots (Sand Veil, Arena Trap)
    // still answer `FoeMaybeTrapPokemon` in a random-team format.
    let mut b = battle("Snorlax|||Stench|tackle|||||||", "Dugtrio|||Stench|earthquake|||||||");
    b.mon_mut(P1).maybe_trapped = false;
    b.foe_maybe_trap_abilities(P1);
    assert!(!b.mon(P1).maybe_trapped, "custom games skip the slots");
    b.format_spec.base = FormatId::Gen3RandomBattle;
    b.foe_maybe_trap_abilities(P1);
    assert!(b.mon(P1).maybe_trapped);
    // A Flying foe of Arena Trap is not grounded.
    let mut b = battle("Pidgeot|||Stench|gust|||||||", "Dugtrio|||Stench|earthquake|||||||");
    b.format_spec.base = FormatId::Gen3RandomBattle;
    b.foe_maybe_trap_abilities(P1);
    assert!(!b.mon(P1).maybe_trapped);
}

#[test]
fn lightning_rod_never_redirects_in_singles() {
    let mut b = battle("Rhydon|||LightningRod|earthquake|||||||", "Raichu|||Stench|thunderbolt|||||||");
    take_log(&mut b);
    // `getMoveTargets` asks `RedirectTarget` only with more than one active
    // Pokemon a side: a singles Thunderbolt hits Rhydon (Ground: immune)
    // without it.
    moves(&mut b, 1, 1);
    let log = take_log(&mut b);
    assert!(log.contains("|-immune|p1a: Rhydon\n"), "{log}");
    // The handler itself: not Electric, nothing.
    let r = b.run_event(EventId::RedirectTarget, Holder::Mon(P2), Some(P2), EffectId::Move(m::EARTHQUAKE), Some(Relay::TRUE));
    assert_eq!(r, Relay::TRUE);
}
