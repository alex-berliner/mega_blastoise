//! Unit tests for the OLED controller's end-of-battle draw path.
//!
//! P2-3 (`Showdown split: Draw end screen`) gives `BoardEvent::Tie` its
//! distinct View::Draw / Screen::Draw variant on BOTH seats. The result and
//! draw screens are decided solely in `crate::oled_ctl.rs` (ZERO-DRIFT in
//! `CLAUDE.md`), so a regression in `oled_cmds_for_event` shows here first.

use mega_blastoise_core::board_event::BoardEvent;
use mega_blastoise_core::oled_ctl::{OledCmd, OledController, OledRedraw};
use mega_blastoise_core::Screen;

fn feed_active_mon(ctl: &mut OledController, player: u8, name: &str, len: u8) {
    let mut buf = [b' '; 16];
    for (i, b) in name.as_bytes().iter().take(len as usize).enumerate() {
        buf[i] = *b;
    }
    ctl.apply(OledCmd::ActiveMon {
        player,
        name: buf,
        len,
        speed: 100,
    });
}

/// `BoardEvent::Tie` drives the new `View::Draw` on BOTH seats with the
/// last-known active-mon names of each seat ready to render. The screen
/// name flips from "WIN" to "DRAW" so a host trace can tell them apart.
#[test]
fn tie_drives_a_draw_screen_on_both_seats() {
    let mut ctl = OledController::new();
    // Give each seat a real mon name. The draw screen's sprite lookup
    // sees these, not whatever default the controller started with.
    feed_active_mon(&mut ctl, 1, "Gastly", 6);
    feed_active_mon(&mut ctl, 2, "Haunter", 7);

    // Apply the Tie event via the public pipeline (the same one the runner
    // and `BoardEffects` call) so the test exercises the real mapping, not
    // a hand-constructed command.
    let redraws: Vec<OledRedraw> = mega_blastoise_core::oled_ctl::oled_cmds_for_event(&BoardEvent::Tie)
        .into_iter()
        .map(|cmd| ctl.apply(cmd))
        .collect();
    assert!(
        redraws.iter().any(|r| matches!(r, OledRedraw::Both)),
        "Tie must redraw both seats, got {redraws:?}"
    );

    let p1 = ctl.screen(1);
    let p2 = ctl.screen(2);
    assert!(matches!(p1, Screen::Draw { .. }), "seat 1 sees Draw");
    assert!(matches!(p2, Screen::Draw { .. }), "seat 2 sees Draw");
    assert_eq!(p1.name(), "DRAW", "seat 1 screen name is DRAW");
    assert_eq!(p2.name(), "DRAW", "seat 2 screen name is DRAW");
    // Both seats carry BOTH sprites' names so the colour renderer can
    // frame the title with whichever mons were standing.
    match (p1, p2) {
        (Screen::Draw { own_name: own1, foe_name: foe1 }, Screen::Draw { own_name: own2, foe_name: foe2 }) => {
            assert_eq!(own1, "Gastly", "seat 1 own is its own mon");
            assert_eq!(foe1, "Haunter", "seat 1 foe is the opponent");
            assert_eq!(own2, "Haunter", "seat 2 own is its own mon");
            assert_eq!(foe2, "Gastly", "seat 2 foe is the opponent");
        }
        _ => unreachable!("matches already checked"),
    }
}

/// `BoardEvent::Win` still drives the existing `View::Win` on both seats
/// with the same WIN/GG messages. The new Draw command is additive, the
/// Win path is unchanged.
#[test]
fn win_drives_the_existing_result_screen_on_both_seats() {
    let mut ctl = OledController::new();

    let redraws: Vec<OledRedraw> = mega_blastoise_core::oled_ctl::oled_cmds_for_event(&BoardEvent::Win {
        side: Some("0".into()),
    })
    .into_iter()
    .map(|cmd| ctl.apply(cmd))
    .collect();
    assert!(
        redraws.iter().any(|r| matches!(r, OledRedraw::Both)),
        "Win must redraw both seats, got {redraws:?}"
    );

    // Seats 1 (whose side is "0") and 2 (whose side is "1") read different
    // status strings from BoardEvent::win_messages, pin them both so the
    // numbers don't quietly swap.
    let p1 = ctl.screen(1);
    let p2 = ctl.screen(2);
    assert!(matches!(p1, Screen::Win("WINNER!")), "seat 1 reads WINNER!");
    assert!(matches!(p2, Screen::Win("GG!")), "seat 2 reads GG!");
    assert_eq!(p1.name(), "WIN");
    assert_eq!(p2.name(), "WIN");
}

/// Tie on an empty controller (no ActiveMon set) still produces a Draw
/// screen, the names fall back to whatever the controller initialised
/// with, which is `"---"`, and render_draw must simply not panic on that.
#[test]
fn tie_with_no_active_mon_still_yields_a_draw_screen() {
    let mut ctl = OledController::new();
    for cmd in mega_blastoise_core::oled_ctl::oled_cmds_for_event(&BoardEvent::Tie) {
        ctl.apply(cmd);
    }
    let p1 = ctl.screen(1);
    let p2 = ctl.screen(2);
    assert!(matches!(p1, Screen::Draw { .. }));
    assert!(matches!(p2, Screen::Draw { .. }));
}
