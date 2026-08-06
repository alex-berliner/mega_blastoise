//! Single-screen device view for the web client.
//!
//! Renders the shared [`OledController`] state through core's color renderer
//! onto one 240x320 panel. The browser is raw IO only: it gets an RGBA buffer
//! and paints it, exactly as the firmware will get a framebuffer and flush it.
//!
//! Screen selection maps the existing `Screen` variants onto the new layouts,
//! with one addition the old model has no concept of: which list the player's
//! cursor is currently in, which lives in core's [`CursorNav`].

use mega_blastoise_core::{
    cursor_nav::{CursorNav, NavMode},
    device_view::{DeviceFrame, Orientation, Region},
    display_color as dc,
    OledController, Screen,
};

/// Per-seat state the controller does not carry.
#[derive(Clone, Copy, Default)]
pub struct SeatUi {
    pub nav: CursorNav,
    /// This seat has committed a choice this turn.
    pub locked: bool,
    /// Name of the committed move, for the locked-in screen.
    pub chosen: Option<&'static str>,
}

/// Draw one seat's half.
pub fn render_seat<D>(d: &mut D, ctl: &OledController, player: u8, ui: &SeatUi)
where
    D: embedded_graphics::draw_target::DrawTarget<Color = embedded_graphics::pixelcolor::Rgb565>,
{
    let me = ctl.seat(player);
    let foe = ctl.seat(3 - player);
    let ctx = dc::HalfCtx {
        own_name: me.name,
        own_hp: me.hp,
        own_level: me.level,
        own_status: me.status,
        foe_name: foe.name,
        foe_hp: foe.hp,
        foe_level: foe.level,
        foe_status: foe.status,
        cursor: ui.nav.cursor,
        foe_locked: ui.locked,
        bob: false,
    };

    match ctl.screen(player) {
        Screen::Lobby { ready, ai } => dc::render_lobby(d, ready, ai),

        // The cursor can be pointing into the party list while the battle
        // screen is still the controller's view — that is a presentation
        // state, not a game state, so it never touches the collector.
        Screen::Battle { moves, .. } => match ui.nav.mode {
            NavMode::Party => dc::render_party(d, me.party, &ctx, false),
            _ => dc::render_choice(d, moves, &ctx),
        },

        Screen::Waiting { .. } => dc::render_locked(d, ui.chosen, &ctx),
        Screen::WaitingForOpponent { .. } => dc::render_locked(d, None, &ctx),
        Screen::Switch(party) => dc::render_party(d, party, &ctx, true),
        Screen::Invalid(reason) => dc::render_invalid(d, reason),
        Screen::Win(msg) => dc::render_result(d, msg, &ctx),

        Screen::MoveDetail { mv, .. } => {
            let desc = mega_blastoise_core::move_descs::move_desc(&mv.name).unwrap_or("");
            dc::render_move_info(d, mv, desc)
        }
        Screen::Stats { slot, .. } => dc::render_party(d, core::slice::from_ref(slot), &ctx, false),

        // Every narration state is the same battle scene with a caption.
        Screen::EventText(text) => dc::render_playback(d, text, &ctx),
        Screen::SentOut { caption, .. } => dc::render_playback(d, caption, &ctx),
        Screen::MoveUsed { caption, .. } => dc::render_playback(d, caption, &ctx),

        Screen::Qr => dc::render_playback(d, "Scan the QR on the back to leave feedback. GG!", &ctx),
        Screen::Tutorial(page) => {
            let text = match page {
                0 => "D-pad picks a move. A locks it in.",
                1 => "B opens your party. B again goes back.",
                _ => "? explains whatever you are pointing at.",
            };
            dc::render_playback(d, text, &ctx)
        }

        // Concealed mode is gone in this design; these cannot be reached.
        _ => dc::render_playback(d, "", &ctx),
    }
}

/// Compose the whole panel for the current orientation.
pub fn render_device(
    ctl: &OledController,
    orientation: Orientation,
    ui1: &SeatUi,
    ui2: &SeatUi,
) -> Vec<u8> {
    let mut frame = DeviceFrame::new();
    match orientation {
        Orientation::HeadToHead => {
            {
                let mut top = Region::half(&mut frame, false, true);
                render_seat(&mut top, ctl, 2, ui2);
            }
            let mut bottom = Region::half(&mut frame, true, false);
            render_seat(&mut bottom, ctl, 1, ui1);
        }
        Orientation::SameWay => {
            {
                let mut top = Region::half(&mut frame, false, false);
                render_seat(&mut top, ctl, 2, ui2);
            }
            let mut bottom = Region::half(&mut frame, true, false);
            render_seat(&mut bottom, ctl, 1, ui1);
        }
        // Landscape shows P1's view across the full panel: attract, lobby,
        // and the menus are one-person screens.
        Orientation::Landscape => {
            let mut r = Region::landscape(&mut frame);
            render_seat(&mut r, ctl, 1, ui1);
        }
    }
    frame.to_rgba()
}

/// Menus are landscape, full-panel, one-person screens.
pub fn render_menu(menu: &mega_blastoise_core::menu::Menu) -> Vec<u8> {
    use mega_blastoise_core::menu::{MenuScreen, OPTION_ROWS};
    let mut frame = DeviceFrame::new();
    {
        let mut r = Region::landscape(&mut frame);
        match menu.screen {
            MenuScreen::GenPicker => dc::render_gen_picker(&mut r, menu.cursor, 320, 240),
            MenuScreen::Options => {
                let mut rows: Vec<dc::OptionRow<'_>> = Vec::with_capacity(OPTION_ROWS);
                for i in 0..OPTION_ROWS {
                    let (label, value) = menu.opts.row(i);
                    rows.push(dc::OptionRow { label, value });
                }
                dc::render_options(&mut r, &rows, menu.cursor, 320)
            }
            // The lobby is drawn by the normal path; this is only reached if a
            // caller renders a menu while none is open.
            MenuScreen::Lobby => dc::render_lobby(&mut r, false, false),
        }
    }
    frame.to_rgba()
}
