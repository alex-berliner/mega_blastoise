//! Unit tests for the gen 3 berries and herbs: the lines and state the
//! protocol diff checks in bulk, pinned here per handler, plus the paths
//! the diff cannot reach yet (Mental Herb needs Attract, Lansat Berry
//! needs Focus Energy). Battles are gen 3 custom games on the real gen 3
//! registry, started through `>start` / `>player`.

extern crate std;

use std::boxed::Box;
use std::format;
use std::string::String;

use ps_dex::gen3::{conditions as c, items as i};
use ps_protocol::text::encode_log;
use ps_protocol::text::team::unpack;
use ps_protocol::{FormatSpec, SideId};

use crate::effect::{EffectId, EventId};
use crate::state::{Battle, Holder, MonRef};
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

fn hp(b: &Battle, m: MonRef) -> String {
    format!("{}/{}", b.mon(m).hp, b.mon(m).maxhp)
}

/// `battle.heal`'s default case: a berry's heal is `[from] item: X`.
#[test]
fn sitrus_heals_30_from_the_item() {
    let mut b = battle("Snorlax||Sitrus Berry|Stench|tackle|||||||", "Snorlax|||Stench|tackle|||||||");
    b.mon_mut(P1).hp = 100;
    b.run_event(EventId::Residual, Holder::Mon(P1), None, EffectId::None, None);
    assert_eq!(b.mon(P1).hp, 130);
    assert!(b.mon(P1).item.is_none());
    // An HP line is split: the owner's copy, then the spectators' (the same
    // numbers here, without HP Percentage Mod).
    let heal = format!("|-heal|p1a: Snorlax|{}|[from] item: Sitrus Berry\n", hp(&b, P1));
    assert_eq!(take_log(&mut b), format!("|-enditem|p1a: Snorlax|Sitrus Berry|[eat]\n|split|p1\n{heal}{heal}"));
}

/// Figy Berry heals `baseMaxhp / 8`, then confuses a holder whose nature
/// lowers Attack.
#[test]
fn figy_confuses_a_minus_attack_nature() {
    let mut b = battle("Snorlax||Figy Berry|Stench|tackle|Modest||||||", "Snorlax|||Stench|tackle|||||||");
    b.mon_mut(P1).hp = 100;
    let eighth = b.mon(P1).base_maxhp / 8;
    assert!(b.eat_item(P1, false, None, EffectId::None));
    assert_eq!(b.mon(P1).hp, 100 + eighth);
    assert!(b.has_vol(P1, c::CONFUSION));
    let log = take_log(&mut b);
    assert!(log.contains("|[from] item: Figy Berry\n|-start|p1a: Snorlax|confusion\n"), "{log}");
}

/// Leppa Berry: 10 PP to the first empty slot, else the first slot below
/// its max, with the `-activate ... [consumed]` line.
#[test]
fn leppa_restores_pp_and_says_so() {
    let mut b = battle("Snorlax||Leppa Berry|Stench|tackle,bodyslam|||||||", "Snorlax|||Stench|tackle|||||||");
    b.mon_mut(P1).move_slots[1].pp = 0;
    b.run_event(EventId::Update, Holder::Mon(P1), None, EffectId::None, None);
    assert_eq!(b.mon(P1).move_slots[1].pp, 10);
    assert_eq!(take_log(&mut b), "|-enditem|p1a: Snorlax|Leppa Berry|[eat]\n|-activate|p1a: Snorlax|item: Leppa Berry|Body Slam|[consumed]\n");

    let mut b = battle("Snorlax||Leppa Berry|Stench|tackle,bodyslam|||||||", "Snorlax|||Stench|tackle|||||||");
    let max = b.mon(P1).move_slots[1].maxpp as i8;
    b.mon_mut(P1).move_slots[1].pp = max - 3;
    assert!(b.eat_item(P1, true, None, EffectId::None));
    assert_eq!(b.mon(P1).move_slots[1].pp, max);
    assert!(take_log(&mut b).contains("|item: Leppa Berry|Body Slam|[consumed]\n"));
}

/// Leppa Berry's PP lands on the slot object `baseMoveSlots` shares, so
/// it outlasts a switch (`clearVolatile` copies `baseMoveSlots` back).
#[test]
fn leppa_pp_outlasts_a_switch() {
    let mut b = battle("Snorlax||Leppa Berry|Stench|tackle,block|||||||", "Snorlax|||Stench|tackle|||||||");
    let max = b.mon(P1).move_slots[1].maxpp as i8;
    b.mon_mut(P1).move_slots[1].pp = 0;
    b.mon_mut(P1).base_move_slots[1].pp = 0;
    b.run_event(EventId::Update, Holder::Mon(P1), None, EffectId::None, None);
    assert_eq!(b.mon(P1).base_move_slots[1].pp, max);
    b.clear_volatile(P1, true);
    assert_eq!(b.mon(P1).move_slots[1].pp, max);
}

/// White Herb's `onAnyAfterMove` runs on the herb's holder, whoever moved.
#[test]
fn white_herb_clears_its_holders_drops_on_any_after_move() {
    let mut b = battle("Snorlax||White Herb|Stench|tackle|||||||", "Snorlax|||Stench|growl|||||||");
    b.mon_mut(P1).boosts[0] = -1;
    b.mon_mut(P1).boosts[6] = -2;
    b.mon_mut(P1).boosts[1] = 1;
    // `runEvent('AfterMove', foe, target, move)`: the foe is the target.
    b.run_event(EventId::AfterMove, Holder::Mon(P2), Some(P1), EffectId::None, None);
    assert_eq!(b.mon(P1).boosts, [0, 1, 0, 0, 0, 0, 0]);
    assert!(b.mon(P1).item.is_none());
    assert_eq!(take_log(&mut b), "|-enditem|p1a: Snorlax|White Herb\n|-clearnegativeboost|p1a: Snorlax|[silent]\n");
}

/// Gen 4's Mental Herb (gen 3 inherits it): Attract only.
#[test]
fn mental_herb_cures_attract() {
    let mut b = battle("Snorlax||Mental Herb|Stench|tackle|||||||", "Snorlax|||Stench|tackle|||||||");
    // Attract's own handlers belong to another batch.
    b.lax_handlers = true;
    b.run_event(EventId::Update, Holder::Mon(P1), None, EffectId::None, None);
    assert!(take_log(&mut b).is_empty());
    b.add_volatile(P1, c::ATTRACT, Some(P2), EffectId::None);
    b.log.clear();
    b.run_event(EventId::Update, Holder::Mon(P1), None, EffectId::None, None);
    assert!(!b.has_vol(P1, c::ATTRACT));
    assert!(b.mon(P1).item.is_none());
    let log = take_log(&mut b);
    assert!(log.starts_with("|-enditem|p1a: Snorlax|Mental Herb\n"), "{log}");
    assert!(log.ends_with("|-end|p1a: Snorlax|move: Attract|[from] item: Mental Herb\n"), "{log}");
}

/// Lansat Berry's handlers are all registered, but its `onEat` adds
/// `focusenergy`: it stays out of the ported list until that is ported.
#[test]
fn lansat_waits_for_focus_energy() {
    let dex = &ps_dex::GEN3;
    let reg = crate::mods::registry_for(dex);
    assert!(crate::mods::is_ported(dex, reg, EffectId::Item(i::LANSATBERRY)));
    assert_eq!(crate::mods::item_conditions(dex, i::LANSATBERRY).collect::<std::vec::Vec<_>>(), [c::FOCUSENERGY]);
    let ported = crate::mods::is_ported(dex, reg, EffectId::Condition(c::FOCUSENERGY));
    assert_eq!(crate::mods::item_is_ported(dex, reg, i::LANSATBERRY), ported);
    assert_eq!(crate::mods::ported(3).contains(&"item:lansatberry"), ported);
    // Handler-free items are ported as they are.
    assert!(crate::mods::ported(3).contains(&"item:quickclaw"));
    assert!(crate::mods::ported(3).contains(&"item:pokeball"));
}
