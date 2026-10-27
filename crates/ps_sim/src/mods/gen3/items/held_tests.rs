//! Unit tests for the gen 3 held items (G3-I1): the paths the protocol
//! diff cannot reach (Mail needs Thief, Covet, Knock Off or Trick) and the
//! details a sweep only meets by chance (King's Rock's list ends with
//! Wrap, Shell Bell's heal of at least 1, the fractional accuracy).
//! Battles are gen 3 custom games on the real gen 3 registry.

extern crate std;

use std::boxed::Box;
use std::format;
use std::string::String;

use ps_dex::gen3::{conditions as c, items as i, moves as m};
use ps_protocol::text::encode_log;
use ps_protocol::text::team::unpack;
use ps_protocol::{FormatSpec, SideId};

use crate::effect::{EffectId, EventId, Pseudo};
use crate::queue::ActionMove;
use crate::relay::Relay;
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

/// Makes `id` the active move, used by `user` against `target`.
fn use_move(b: &mut Battle, id: ps_dex::MoveId, user: MonRef, target: MonRef) {
    let mv = b.get_active_move(ActionMove::Move(id));
    b.set_active_move(Some(mv), Some(user), Some(target));
}

/// Mail: `onTakeItem` refuses with no active move and for any move but
/// Knock Off, Thief and Covet.
#[test]
fn mail_is_taken_only_by_knock_off_thief_and_covet() {
    let mut b = battle("Wobbuffet||Mail|Stench|tackle|||||||", "Snorlax|||Stench|tackle|||||||");
    assert_eq!(b.take_item(P1, Some(P2)), Err(Relay::FAIL));
    use_move(&mut b, m::TRICK, P2, P1);
    assert_eq!(b.take_item(P1, Some(P2)), Err(Relay::FAIL));
    assert_eq!(b.mon(P1).item, i::MAIL);
    for id in [m::KNOCKOFF, m::THIEF, m::COVET] {
        let mut b = battle("Wobbuffet||Mail|Stench|tackle|||||||", "Snorlax|||Stench|tackle|||||||");
        use_move(&mut b, id, P2, P1);
        assert_eq!(b.take_item(P1, Some(P2)), Ok(i::MAIL));
        assert!(b.mon(P1).item.is_none());
    }
}

/// King's Rock: `onModifyMove` pushes the 10% flinch onto a listed move
/// (Wrap is the list's last entry; a typed Hidden Power is
/// `hiddenpower`), and onto no other.
#[test]
fn kings_rock_adds_its_flinch_to_listed_moves_only() {
    for (id, listed) in [(m::WRAP, true), (m::AERIALACE, true), (m::HIDDENPOWERFIRE, true), (m::THUNDERBOLT, false)] {
        let mut b = battle("Cloyster||King's Rock|Stench|tackle|||||||", "Snorlax|||Stench|tackle|||||||");
        use_move(&mut b, id, P1, P2);
        let mv = b.active_move.unwrap();
        let r = b.run_event(EventId::ModifyMove, Holder::Mon(P1), Some(P2), b.mv(mv).effect(), Some(b.mv(mv).relay()));
        assert!(matches!(r, Relay::Move(_)));
        let added = b.mv(mv).added_secondary;
        assert_eq!(added.is_some(), listed, "{id:?}");
        if let Some(s) = added {
            assert_eq!((s.chance, s.volatile_status), (Some(10), c::FLINCH));
        }
    }
}

/// King's Rock pushes its flinch onto the same `move.secondaries` the
/// abilities see: Serene Grace (whose `ModifyMove` runs after King's
/// Rock's) doubles a move that had no secondaries of its own, and Shield
/// Dust's `ModifySecondaries` mask covers the flinch after the dex's own.
#[test]
fn kings_rock_flinch_is_on_the_list_the_abilities_see() {
    for (item, doubled) in [("King's Rock", true), ("", false)] {
        let mut b = battle(&format!("Togepi||{item}|Serene Grace|tackle|||||||"), "Snorlax|||Stench|tackle|||||||");
        use_move(&mut b, m::TACKLE, P1, P2);
        let mv = b.active_move.unwrap();
        b.run_event(EventId::ModifyMove, Holder::Mon(P1), Some(P2), b.mv(mv).effect(), Some(b.mv(mv).relay()));
        assert_eq!(b.mv(mv).secondary_chances_doubled, doubled, "{item:?}");
        assert_eq!(b.mv(mv).secondaries(b.dex).count(), usize::from(doubled));
    }
    let mut b = battle("Cloyster||King's Rock|Stench|aerialace|||||||", "Venomoth|||Shield Dust|tackle|||||||");
    use_move(&mut b, m::MUDSHOT, P1, P2);
    let mv = b.active_move.unwrap();
    b.run_event(EventId::ModifyMove, Holder::Mon(P1), Some(P2), b.mv(mv).effect(), Some(b.mv(mv).relay()));
    let list: std::vec::Vec<_> = b.mv(mv).secondaries(b.dex).map(|s| s.volatile_status).collect();
    assert_eq!(list, [ps_dex::ConditionId::NONE, c::FLINCH], "Mud Shot's speed drop, then the flinch");
    let effect = b.mv(mv).effect();
    let r = b.run_event(EventId::ModifySecondaries, Holder::Mon(P2), Some(P1), effect, Some(Relay::Secondaries(0b11)));
    assert_eq!(r, Relay::Secondaries(0));
}

/// Shell Bell: `heal(move.totalDamage / 8, pokemon)` heals at least 1 and
/// names the move's target with `[of]` (the event's source).
#[test]
fn shell_bell_heals_an_eighth_at_least_one_of_the_target() {
    for (total, healed) in [(5, 1), (80, 10), (0, 0)] {
        let mut b = battle("Snorlax||Shell Bell|Stench|tackle|||||||", "Snorlax|||Stench|tackle|||||||");
        b.mon_mut(P1).hp = 100;
        use_move(&mut b, m::TACKLE, P1, P2);
        let mv = b.active_move.unwrap();
        b.mv_mut(mv).total_damage = total;
        b.run_event(EventId::AfterMoveSecondarySelf, Holder::Mon(P1), Some(P2), b.mv(mv).effect(), None);
        assert_eq!(b.mon(P1).hp, 100 + healed, "totalDamage {total}");
        let log = take_log(&mut b);
        if healed > 0 {
            let line = format!("|-heal|p1a: Snorlax|{}/{}|[from] item: Shell Bell|[of] p2a: Snorlax\n", b.mon(P1).hp, b.mon(P1).maxhp);
            assert!(log.contains(&line), "{log}");
        } else {
            assert_eq!(log, "");
        }
    }
}

/// Shell Bell after a calling move: Nature Power's `useMove("swift")`
/// heals once, on Swift's own `AfterMoveSecondarySelf`. Nature Power's
/// event comes after, with `activeMove` still on Swift; it reads its own
/// move object, which dealt no damage, so it heals nothing.
#[test]
fn shell_bell_heals_once_after_a_called_move() {
    let mut b = battle("Snorlax||Shell Bell|Stench|naturepower|||||||", "Snorlax|||Stench|growl|||||||");
    b.mon_mut(P1).hp = 100;
    assert!(b.choose(0, ps_protocol::Choice::Move(1)));
    assert!(b.choose(1, ps_protocol::Choice::Move(1)));
    let dealt = b.mon(P2).maxhp - b.mon(P2).hp;
    assert!(dealt > 0);
    assert_eq!(b.mon(P1).hp, 100 + (dealt / 8).max(1));
    let log = take_log(&mut b);
    assert!(log.contains("|move|p1a: Snorlax|Swift|"), "{log}");
    // One `|split|` heal: its private and public lines.
    assert_eq!(log.matches("[from] item: Shell Bell").count(), 2, "{log}");
}

/// Focus Band: a winning roll saves the holder from a move's damage, and
/// confusion's self-hit is one (PS passes `{id: 'confused', effectType:
/// 'Move'}`); a status's damage is not, though it draws the roll too.
#[test]
fn focus_band_saves_from_a_confusion_hit_but_not_poison() {
    for (effect, saved) in [(EffectId::Pseudo(Pseudo::Confused), true), (EffectId::Move(m::TACKLE), true), (EffectId::Condition(c::PSN), false)] {
        let mut b = battle("Snorlax||Focus Band|Stench|tackle|||||||", "Snorlax|||Stench|tackle|||||||");
        b.mon_mut(P1).hp = 10;
        b.prng.push_overrides([0]);
        let calls = b.prng.calls;
        b.damage(50, Some(P1), Some(P1), effect);
        assert_eq!(b.prng.calls, calls + 1, "{effect:?}");
        assert_eq!(b.mon(P1).hp, if saved { 1 } else { 0 }, "{effect:?}");
        let log = take_log(&mut b);
        assert_eq!(log.contains("|-activate|p1a: Snorlax|item: Focus Band\n"), saved, "{log}");
    }
}

/// Leftovers: `heal(baseMaxhp / 16)` with no `[of]`.
#[test]
fn leftovers_heals_a_sixteenth() {
    let mut b = battle("Snorlax||Leftovers|Stench|tackle|||||||", "Snorlax|||Stench|tackle|||||||");
    b.mon_mut(P1).hp = 100;
    let sixteenth = b.mon(P1).base_maxhp / 16;
    b.field_event(EventId::Residual, None);
    assert_eq!(b.mon(P1).hp, 100 + sixteenth);
    let log = take_log(&mut b);
    let line = format!("|-heal|p1a: Snorlax|{}/{}|[from] item: Leftovers\n", b.mon(P1).hp, b.mon(P1).maxhp);
    assert!(log.contains(&line), "{log}");
}

/// Bright Powder and Lax Incense scale the holder's (the target's)
/// `ModifyAccuracy` relay as a JS double: a fraction stays a fraction, a
/// `true` accuracy is left alone.
#[test]
fn accuracy_items_keep_the_fraction() {
    for (item, factor) in ["Bright Powder", "Lax Incense"].into_iter().zip([0.9, 0.95]) {
        let mut b = battle("Snorlax|||Stench|tackle|||||||", &format!("Snorlax||{item}|Stench|tackle|||||||"));
        use_move(&mut b, m::TACKLE, P1, P2);
        let effect = EffectId::Move(m::TACKLE);
        let acc = 100.0 * (4.0 / 3.0);
        let r = b.run_event(EventId::ModifyAccuracy, Holder::Mon(P2), Some(P1), effect, Some(Relay::float(acc)));
        assert_eq!(r, Relay::float(acc * factor), "{item}");
        let r = b.run_event(EventId::ModifyAccuracy, Holder::Mon(P2), Some(P1), effect, Some(Relay::TRUE));
        assert_eq!(r, Relay::TRUE, "{item}");
        // The holder's own moves are not touched (`onModifyAccuracy`, not
        // `onSourceModifyAccuracy`).
        let r = b.run_event(EventId::ModifyAccuracy, Holder::Mon(P1), Some(P2), effect, Some(Relay::float(100.0)));
        assert_eq!(r, Relay::float(100.0), "{item}");
    }
}

/// The handler-free remainder: every G3-I1 item is in `ported(3)`.
#[test]
fn held_items_are_ported() {
    let ported = crate::mods::ported(3);
    for id in [
        "blackbelt", "blackglasses", "brightpowder", "charcoal", "choiceband", "deepseascale", "deepseatooth", "dragonfang",
        "focusband", "hardstone", "kingsrock", "laxincense", "leftovers", "lightball", "luckypunch", "machobrace", "magnet",
        "mail", "metalcoat", "metalpowder", "miracleseed", "mysticwater", "nevermeltice", "poisonbarb", "scopelens",
        "seaincense", "sharpbeak", "shellbell", "silkscarf", "silverpowder", "softsand", "souldew", "spelltag", "stick",
        "thickclub", "twistedspoon",
    ] {
        assert!(ported.contains(&format!("item:{id}").as_str()), "{id}");
    }
}
