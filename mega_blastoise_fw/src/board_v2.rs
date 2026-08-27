//! Stripboard v2 game loop — the single-screen device.
//!
//! Milestone: lobby + menus live on the ST7796, driven by the 16 real
//! buttons. All meaning lives in core's `DeviceSession` (shared verbatim
//! with the web build — zero drift); this module only ferries presses in
//! and frames out, exactly like `mega_blastoise_web/src/lib.rs::press`.
//!
//! Not wired yet: the battle engine behind the lobby (Outs::Nav /
//! CancelChoice are logged and dropped), the attract demo, USB CLI. Next
//! milestone.

use defmt::info;
use mega_blastoise_core::device_session::{Button, Ctx, DeviceSession, Out};
use mega_blastoise_core::device_ui::render_session_frame;
use mega_blastoise_core::{OledCmd, OledController};

use crate::subsystems::{buttons_v2, st7796};

pub async fn run() -> ! {
    let mut session = DeviceSession::new();
    let mut ctl = OledController::new();
    let mut ready = [false; 2];
    let mut ai = [false; 2];

    info!("board_v2: lobby up");

    // Waiting for IDLE before every render keeps exactly one frame in
    // flight (see st7796::IDLE); presses arriving during a blit pile up in
    // the channel and coalesce into the next frame.
    loop {
        st7796::IDLE.wait().await;
        #[allow(unused_mut)]
        let mut frame = render_session_frame(&session, &ctl, true);
        #[cfg(feature = "input-debug")]
        mega_blastoise_core::device_view::draw_input_debug(&mut frame, buttons_v2::masks());
        st7796::FRAME.signal(frame);

        // A session event or (input-debug) any button edge wakes the loop;
        // releases repaint the overlay even though they carry no event.
        let first = {
            use embassy_futures::select::{select, Either};
            match select(buttons_v2::EVENTS.receive(), buttons_v2::DIRTY.wait()).await {
                Either::First(e) => Some(e),
                Either::Second(()) => None,
            }
        };
        let Some(first) = first else { continue };
        let mut apply = |(player, button): (u8, Button),
                         session: &mut DeviceSession,
                         ctl: &mut OledController,
                         ready: &mut [bool; 2],
                         ai: &mut [bool; 2]| {
            // Milestone: always lobby, nobody is choosing or waiting — the
            // battle engine is not behind the session yet.
            let ctx = Ctx { lobby: true, choosing: false, waiting: false };
            let i = (player == 2) as usize;
            for out in session.button(player, button, ctx) {
                match out {
                    Out::ReadyLine(p) => {
                        ready[i] = true;
                        info!("board_v2: P{=u8} ready", p);
                    }
                    Out::CancelLine(p) => {
                        ready[i] = false;
                        ai[i] = false;
                        info!("board_v2: P{=u8} cancel", p);
                    }
                    Out::LobbyLongPress(p) => {
                        // "HOLD A: FIGHT THE AI" — the robot takes the OTHER
                        // side; the holder readies up as themselves.
                        ready[i] = true;
                        ai[1 - i] = true;
                        ready[1 - i] = true;
                        info!("board_v2: P{=u8} requested an AI rival", p);
                    }
                    Out::Nav(p, _) | Out::CancelChoice(p) => {
                        info!("board_v2: P{=u8} battle out (engine not wired yet)", p);
                    }
                }
            }
            // Both stores need the flags: the session for what B means on
            // the next press, the controller for what the lobby SHOWS —
            // render_seat reads Screen::Lobby off the controller, so ready
            // never reached the glass without this.
            session.set_lobby_flags(*ready, *ai);
            for p in [1u8, 2] {
                let j = (p == 2) as usize;
                let _ = ctl.apply(OledCmd::LobbyState { player: p, ready: ready[j], ai: ai[j] });
            }
        };
        apply(first, &mut session, &mut ctl, &mut ready, &mut ai);
        while let Ok(more) = buttons_v2::EVENTS.try_receive() {
            apply(more, &mut session, &mut ctl, &mut ready, &mut ai);
        }
    }
}
