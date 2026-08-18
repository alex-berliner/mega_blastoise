//! Composition for the single-screen device: which renderer draws each half.
//!
//! Lives in core because everything that appears on the panel must be decided
//! identically on both platforms. The web build and the firmware both call
//! [`render_device`] / [`render_menu`] and only differ in where the RGBA goes.
extern crate alloc;

use alloc::vec::Vec;

use crate::{
    cursor_nav::{CursorNav, NavMode},
    device_view::{DeviceFrame, Region},
    display_color as dc,
    OledController, Screen,
};

/// Draw `f` into both seats' halves, with the far seat's copy rotated 180.
/// Every screen the device shows is composed this way: the players sit across
/// from each other and the console never turns.
fn both_halves(frame: &mut DeviceFrame, mut f: impl FnMut(&mut Region<'_>, u8)) {
    {
        let mut top = Region::half(frame, false, true);
        f(&mut top, 2);
    }
    let mut bottom = Region::half(frame, true, false);
    f(&mut bottom, 1);
}

/// A seat's battle-log overlay: the shared lines plus that seat's scroll
/// offset, or `None` when the seat has the log closed.
#[derive(Clone, Copy)]
pub struct LogView<'a> {
    pub lines: &'a [alloc::string::String],
    pub offset: Option<usize>,
}

/// Per-seat state the controller does not carry.
#[derive(Clone, Copy, Default)]
pub struct SeatUi {
    pub nav: CursorNav,
    /// This seat has committed a choice this turn.
    pub locked: bool,
    /// Name of the committed move, for the locked-in screen.
    pub chosen: Option<&'static str>,
    /// Cursor row when this seat has the options open, `None` when closed.
    /// Per seat, not global: one player reading the options must not drag the
    /// other out of the lobby. The settings themselves are still shared.
    pub options: Option<u8>,
}

/// Build the options rows for the renderer. The labels and values are
/// `&'static str`, so this borrows nothing from `opts`.
fn option_rows(opts: &crate::menu::GameOptions) -> Vec<dc::OptionRow<'static>> {
    let mut rows = Vec::with_capacity(crate::menu::OPTION_ROWS);
    for i in 0..crate::menu::OPTION_ROWS {
        let (label, value) = opts.row(i);
        rows.push(dc::OptionRow { label, value });
    }
    rows
}

/// This seat's move animation for the current frame, if it is watching one.
/// A private overlay (the log, the options) hides the field, and an effect
/// drawn over a screen that is not showing the mons has nothing to travel
/// between.
fn seat_anim(ctl: &OledController, player: u8, ui: &SeatUi, log: LogView<'_>) -> Option<crate::move_anim::Anim> {
    if ui.options.is_some() || log.offset.is_some() {
        return None;
    }
    match ctl.screen(player) {
        Screen::MoveUsed { move_id, attacker, elapsed_ms, .. } => crate::move_anim::anim(
            move_id,
            attacker,
            elapsed_ms,
            crate::battle_effects::anim::MOVE_MS,
        ),
        _ => None,
    }
}

/// Draw one seat's half. Returns true when it drew the shared battle scene,
/// which is what decides whether the two halves join into one field.
pub fn render_seat<D>(
    d: &mut D,
    ctl: &OledController,
    player: u8,
    ui: &SeatUi,
    log: LogView<'_>,
    opts: &crate::menu::GameOptions,
    foe_locked: bool,
) where
    D: embedded_graphics::draw_target::DrawTarget<Color = embedded_graphics::pixelcolor::Rgb565>,
{
    // The options are a per-seat overlay for the same reason the log is: the
    // other seat keeps whatever it was looking at.
    if let Some(cursor) = ui.options {
        dc::render_options(d, &option_rows(opts), cursor, dc::HALF_W, dc::HALF_H, player);
        return;
    }
    // The log is a per-seat overlay: one player can read it while the other
    // keeps playing, so it never stalls the game.
    if let Some(offset) = log.offset {
        let lines: Vec<&str> = log.lines.iter().map(|s| s.as_str()).collect();
        dc::render_log(d, &lines, offset, player);
        return;
    }
    let me = ctl.seat(player);
    let foe = ctl.seat(3 - player);
    let ctx = dc::HalfCtx {
        seat: player,
        own_name: me.name,
        own_hp: me.hp,
        own_level: me.level,
        own_status: me.status,
        // The controller reports a percentage; the exact numbers live on the
        // active party slot, which is where the plate gets them.
        own_hp_numbers: me
            .party
            .iter()
            .find(|p| p.active)
            .map(|p| (p.hp, p.max_hp)),
        foe_name: foe.name,
        foe_hp: foe.hp,
        foe_level: foe.level,
        foe_status: foe.status,
        foe_bob: foe.bob,
        cursor: ui.nav.cursor,
        // The RIVAL's commit, not this seat's — reading it off `ui` meant a
        // seat watched itself lock in and never saw its opponent do it.
        foe_locked,
        bob: me.bob,
    };

    match ctl.screen(player) {
        Screen::Lobby { ready, ai } => dc::render_lobby(d, ready, ai, player),

        // The cursor can be pointing into the party list while the battle
        // screen is still the controller's view — that is a presentation
        // state, not a game state, so it never touches the collector.
        // A seat that has committed shows the locked screen even while the
        // controller still reports the prompt — which is what an AI seat does
        // for the whole turn, since its choice is set the moment the turn
        // opens and it never touches the UI.
        Screen::Battle { .. } if ui.locked => dc::render_locked(d, ui.chosen, &ctx),
        Screen::Battle { moves, .. } => match ui.nav.mode {
            NavMode::Party => dc::render_party(d, me.party, &ctx, false),
            _ => dc::render_choice(d, moves, &ctx),
        },

        Screen::Waiting { .. } => dc::render_locked(d, ui.chosen, &ctx),
        Screen::WaitingForOpponent { .. } => dc::render_locked(d, None, &ctx),
        Screen::Switch(party) => dc::render_party(d, party, &ctx, true),
        Screen::Invalid(reason) => dc::render_invalid(d, reason, player),
        Screen::Win(msg) => dc::render_result(d, msg, &ctx),

        Screen::MoveDetail { mv, .. } => {
            let desc = crate::move_descs::move_desc(&mv.name).unwrap_or("");
            dc::render_move_info(d, mv, desc, player)
        }
        Screen::Stats { slot, .. } => dc::render_stats(d, slot, player),

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

/// Compose the whole panel: the two seats, head-to-head.
pub fn render_device(
    ctl: &OledController,
    ui1: &SeatUi,
    ui2: &SeatUi,
    log1: LogView<'_>,
    log2: LogView<'_>,
    opts: &crate::menu::GameOptions,
) -> Vec<u8> {
    let mut frame = DeviceFrame::new();
    both_halves(&mut frame, |r, seat| {
        let (ui, log) = if seat == 1 { (ui1, log1) } else { (ui2, log2) };
        let foe_locked = if seat == 1 { ui2.locked } else { ui1.locked };
        render_seat(r, ctl, seat, ui, log, opts, foe_locked);
        // The border is the last thing painted on a half, so no screen can
        // bleed into it however its own layout drifts.
        dc::draw_play_frame_edge(r, seat);
        // An attack plays out on BOTH halves at once, each from that seat's
        // own point of view: the effect travels from the attacker's mon to
        // its target as that player sees them. There is no shared field any
        // more, so there is nothing across the seam to animate.
        if let Some(a) = seat_anim(ctl, seat, ui, log) {
            let own = dc::OWN_MON_CENTRE;
            let foe = dc::FOE_MON_CENTRE;
            let (user, target) = if a.attacker == seat { (own, foe) } else { (foe, own) };
            crate::move_anim::draw(r, &a, user, target);
        }
    });
    crate::device_view::draw_split_divider(&mut frame);
    // The wash is the one thing left that belongs to the whole panel: both
    // players are watching the same move land.
    let flash = [1u8, 2]
        .iter()
        .filter_map(|&seat| seat_anim(ctl, seat, if seat == 1 { ui1 } else { ui2 }, if seat == 1 { log1 } else { log2 }))
        .map(|a| crate::move_anim::flash_amount(&a))
        .max()
        .unwrap_or(0);
    if flash > 0 {
        crate::move_anim::white_out(&mut frame, flash);
    }
    frame.to_rgba()
}

/// The gen picker is the one menu that is not per seat: it chooses the game
/// both players are about to be in, so it owns the whole panel and is drawn
/// into both halves, each the right way up, and either player can drive it.
pub fn render_menu(
    menu: &crate::menu::Menu,
    opts: &crate::menu::GameOptions,
) -> Vec<u8> {
    use crate::menu::MenuScreen;
    use crate::display_color::{HALF_H, HALF_W};
    let rows = option_rows(opts);
    let mut frame = DeviceFrame::new();
    both_halves(&mut frame, |r, seat| match menu.screen {
        MenuScreen::GenPicker => dc::render_gen_picker(r, menu.cursor, HALF_W, HALF_H, seat),
        MenuScreen::Options => dc::render_options(r, &rows, menu.cursor, HALF_W, HALF_H, seat),
        // The lobby is drawn by the normal path; this is only reached if a
        // caller renders a menu while none is open.
        MenuScreen::Lobby => dc::render_lobby(r, false, false, seat),
    });
    both_halves(&mut frame, |r, seat| dc::draw_play_frame_edge(r, seat));
    crate::device_view::draw_split_divider(&mut frame);
    frame.to_rgba()
}

/// Compose the whole panel from the session, given the platform's one fact
/// (lobby phase). The menu-vs-battle decision lives here with the rest of the
/// display logic rather than at each platform's call site.
pub fn render_session(
    session: &crate::device_session::DeviceSession,
    ctl: &OledController,
    lobby: bool,
) -> Vec<u8> {
    // A menu can only own the screen while the lobby is idle — a battle
    // (including the attract demo) always wins the panel.
    if session.menu_active && lobby {
        return render_menu(&session.menu, &session.opts);
    }
    let lines = session.log.lines();
    let l1 = LogView { lines, offset: session.log_view(1) };
    let l2 = LogView { lines, offset: session.log_view(2) };
    render_device(ctl, &session.seats[0], &session.seats[1], l1, l2, &session.opts)
}
