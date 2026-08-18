//! Everything the single-screen device decides outside a battle turn: which
//! menu owns which half, the shared game options, each seat's cursor and
//! overlays, and what a button press means in each of those states.
//!
//! This is the layer the fidelity rule is about. It was first written as web
//! thread-locals, which meant the web had a gen picker and the firmware did
//! not — the platforms had drifted at the level of *what the device is*. One
//! [`DeviceSession`] in core, driven by [`DeviceSession::button`], is what
//! makes the two platforms the same machine: they forward raw presses in and
//! execute the returned [`Out`]s, and every decision in between lives here.
//!
//! What stays platform-side, deliberately: queues and wakers (IO), narration
//! pacing (timing), and anything read from the live battle screen — those
//! arrive as arguments like [`Ctx`] rather than being reached for.

extern crate alloc;

use alloc::vec::Vec;

use crate::battle_log::BattleLog;
use crate::cursor_nav::{Dir, NavOut};
use crate::device_ui::SeatUi;
use crate::menu::{GameOptions, Gen, Menu, MenuOut, MenuScreen};
use gen3_battle::Ruleset;

/// A physical input, after the platform's debounce/hold classification.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Button {
    A,
    B,
    Info,
    Dpad(Dir),
    /// A held long enough for the lobby's AI request.
    AHold,
    /// A tap on the seat's own half of the panel.
    TapSeat,
}

/// What the platform must do in response to a press. The session never
/// touches IO itself.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Out {
    /// Send this seat's ready-up line to the lobby sequence.
    ReadyLine(u8),
    /// Send this seat's cancel line.
    CancelLine(u8),
    /// Take this seat's committed battle choice back (the collector's
    /// unready). Produced by B — or any confirm — on a seat that is already
    /// waiting; the probe that forced this fix pressed B on a locked seat
    /// and watched `nav.back()` toggle the cursor mode underneath the
    /// waiting screen instead of unreadying.
    CancelChoice(u8),
    /// A battle-navigation outcome for this seat (tap/hold a move or switch).
    Nav(u8, NavOut),
    /// The lobby long-press: this seat asked for an AI opponent.
    LobbyLongPress(u8),
}

/// Context for a direct tap on the panel, which routes per half rather than
/// per button.
#[derive(Clone, Copy, Debug)]
pub struct TapCtx {
    pub lobby: bool,
    /// Each seat's "committed and waiting" state, read off the live battle
    /// screen: a tap on your own half while waiting takes the choice back.
    pub waiting: [bool; 2],
}

/// What the session cannot know on its own this instant, passed per call.
#[derive(Clone, Copy, Debug)]
pub struct Ctx {
    /// The game loop is in its lobby phase (no battle running).
    pub lobby: bool,
    /// This seat's battle screen is a choosing screen (moves or party), which
    /// decides whether `?` explains the cursor or opens the log.
    pub choosing: bool,
    /// This seat has committed a choice and is on the waiting screen. B (or
    /// any confirm) then means "take it back", not navigation.
    pub waiting: bool,
}

/// The device's non-battle state, shared verbatim by both platforms.
pub struct DeviceSession {
    /// The gen picker / options state machine (cursor only; see [`Menu`]).
    pub menu: Menu,
    /// The picker owns the whole panel. Boots true.
    pub menu_active: bool,
    /// Shared settings: either player may change them, they apply to both.
    pub opts: GameOptions,
    /// Per-seat cursor state and overlays.
    pub seats: [SeatUi; 2],
    /// The battle narration log, and each seat's scroll position into it
    /// (`None` = closed).
    pub log: BattleLog,
    log_view: [Option<usize>; 2],
    /// Lobby readiness mirrored from the ready sequence, so B knows whether
    /// it cancels, sends a robot home, or reopens the picker.
    ready: [bool; 2],
    ai: [bool; 2],
}

impl Default for DeviceSession {
    fn default() -> Self {
        Self::new()
    }
}

fn idx(player: u8) -> usize {
    (player == 2) as usize
}

impl DeviceSession {
    pub fn new() -> Self {
        Self {
            menu: Menu::new(),
            menu_active: true,
            opts: GameOptions::default(),
            seats: [SeatUi::default(), SeatUi::default()],
            log: BattleLog::new(),
            log_view: [None, None],
            ready: [false, false],
            ai: [false, false],
        }
    }

    // ── What the menus decided ───────────────────────────────────────────

    /// Which engine the players picked. THE source of truth for both
    /// platforms; the firmware reads the same field the web does.
    pub fn ruleset(&self) -> Ruleset {
        match self.opts.gen {
            Gen::One => Ruleset::Gen1,
            Gen::ThreePreview => Ruleset::Gen3,
        }
    }

    pub fn six_v_six(&self) -> bool {
        self.opts.team_size == 6
    }

    pub fn tutorial(&self) -> bool {
        self.opts.tutorial
    }

    pub fn text_scale(&self, ms: u32) -> u32 {
        self.opts.text_speed.scale(ms)
    }

    // ── Mirrors the platform keeps fresh ─────────────────────────────────

    /// Called each lobby tick with the ready sequence's flags.
    pub fn set_lobby_flags(&mut self, ready: [bool; 2], ai: [bool; 2]) {
        self.ready = ready;
        self.ai = ai;
    }

    pub fn lobby_ready(&self) -> [bool; 2] {
        self.ready
    }

    // ── Battle-turn bookkeeping ──────────────────────────────────────────

    /// A battle is starting. Called by the platform between the lobby and
    /// the first prompt.
    pub fn begin_battle(&mut self) {
        for seat in self.seats.iter_mut() {
            seat.nav.begin_battle();
        }
    }

    pub fn begin_turn(&mut self, player: u8, n_moves: u8, n_party: u8, forced: bool) {
        let s = &mut self.seats[idx(player)];
        s.nav.begin_turn(n_moves, n_party, forced);
        s.locked = false;
        s.chosen = None;
    }

    pub fn set_locked(&mut self, player: u8, locked: bool) {
        self.seats[idx(player)].locked = locked;
    }

    // ── The log overlay ──────────────────────────────────────────────────

    pub fn log_line(&mut self, line: &str) {
        self.log.push(line);
    }

    pub fn log_reset(&mut self) {
        self.log.clear();
        self.log_view = [None, None];
    }

    pub fn log_view(&self, player: u8) -> Option<usize> {
        self.log_view[idx(player)]
    }

    fn log_scroll(&mut self, player: u8, delta: i32) {
        let max = self.log.max_scroll();
        if let Some(off) = self.log_view[idx(player)] {
            self.log_view[idx(player)] = Some((off as i32 + delta).clamp(0, max as i32) as usize);
        }
    }

    // ── The one dispatch ─────────────────────────────────────────────────

    /// What `button` means for `player` right now. The platform executes the
    /// returned outs verbatim; the ordering of these branches IS the device's
    /// input model, so the two platforms cannot disagree about it.
    pub fn button(&mut self, player: u8, button: Button, ctx: Ctx) -> Vec<Out> {
        let mut outs = Vec::new();
        let i = idx(player);

        // The gen picker owns the whole panel while it is up.
        if self.menu_active && ctx.lobby {
            let out = match button {
                Button::A | Button::TapSeat => self.menu.confirm(&mut self.opts),
                Button::B => self.menu.back(),
                Button::Info => self.menu.info(),
                Button::Dpad(d) => self.menu.dpad(d, &mut self.opts),
                Button::AHold => MenuOut::None,
            };
            self.close_menu_on(out, &mut outs);
            return outs;
        }

        // This seat's options overlay: private, and only this seat's.
        if let Some(cursor) = self.seats[i].options {
            let mut m = Menu { screen: MenuScreen::Options, cursor };
            let out = match button {
                Button::A | Button::TapSeat => m.confirm(&mut self.opts),
                Button::B => m.back(),
                Button::Info => m.info(),
                Button::Dpad(d) => m.dpad(d, &mut self.opts),
                // A hold here is a held A on a settings row, not an AI request.
                Button::AHold => MenuOut::None,
            };
            self.seats[i].options =
                if out == MenuOut::EnterLobby { None } else { Some(m.cursor) };
            return outs;
        }

        // The log overlay: B and ? close it, the D-pad scrolls it.
        if self.log_view[i].is_some() {
            match button {
                Button::B | Button::Info => self.log_view[i] = None,
                Button::Dpad(d) => {
                    self.log_scroll(player, if matches!(d, Dir::Up | Dir::Left) { -1 } else { 1 })
                }
                _ => {}
            }
            return outs;
        }

        if ctx.lobby {
            match button {
                // Ready up directly: concealed mode is gone, so there is no
                // picker between the press and being ready.
                Button::A => outs.push(Out::ReadyLine(player)),
                // A tap toggles: it has no second button to pair with, so the
                // thing that readied you is the thing that takes it back.
                Button::TapSeat => outs.push(if self.ready[i] {
                    Out::CancelLine(player)
                } else {
                    Out::ReadyLine(player)
                }),
                Button::B => {
                    if self.ready[i] {
                        // A ready player cancels first.
                        outs.push(Out::CancelLine(player));
                    } else if self.ai[1 - i] {
                        // The other side is a robot you asked for: B sends it
                        // home. A human opponent's readiness is theirs.
                        outs.push(Out::CancelLine(3 - player));
                    } else {
                        // An idle seat's B reopens the picker for both.
                        self.menu_active = true;
                        self.menu.screen = MenuScreen::GenPicker;
                        self.menu.cursor = 0;
                    }
                }
                // ? opens the options on this half only.
                Button::Info => self.seats[i].options = Some(0),
                Button::AHold => outs.push(Out::LobbyLongPress(player)),
                Button::Dpad(_) => {}
            }
            return outs;
        }

        // A committed seat is not navigating: the cursor state machine is
        // for choosing, and this seat has chosen. B and both confirms take
        // the choice back; the D-pad and ? do nothing until the seat is
        // choosing again.
        if ctx.waiting {
            match button {
                Button::A | Button::B | Button::TapSeat => {
                    outs.push(Out::CancelChoice(player));
                }
                Button::Dpad(_) | Button::Info | Button::AHold => {}
            }
            return outs;
        }

        // In battle: the cursor state machine, plus ? meaning "explain" while
        // choosing and "open the log" during playback.
        match button {
            Button::A | Button::TapSeat => self.nav(player, &mut outs, |n| n.confirm()),
            Button::B => self.nav(player, &mut outs, |n| n.back()),
            Button::Dpad(d) => self.nav(player, &mut outs, |n| n.dpad(d)),
            Button::Info => {
                if ctx.choosing {
                    self.nav(player, &mut outs, |n| n.info());
                } else {
                    self.log_view[i] = Some(self.log.bottom());
                }
            }
            Button::AHold => {}
        }
        outs
    }

    /// Point the cursor at an item directly (a tap on a move cell or party
    /// row), bounded by the same limits the D-pad respects. True if it landed.
    pub fn set_cursor(&mut self, player: u8, index: u8, ctx: Ctx) -> bool {
        if ctx.lobby || self.menu_active {
            return false;
        }
        use crate::cursor_nav::NavMode;
        let n = &mut self.seats[idx(player)].nav;
        let limit = match n.mode {
            NavMode::Moves => n.n_moves,
            NavMode::Party => n.n_party,
            NavMode::Detail => 1,
        };
        if index < limit {
            n.cursor = index;
            true
        } else {
            false
        }
    }

    /// A raw tap on the composed 240x320 panel, in panel coordinates.
    ///
    /// This is the single mapping from a point to a meaning — which half was
    /// touched, the far half's 180° flip, and what the touched pixel is over.
    /// It lives here because every hit box is derived from the layout
    /// constants in [`crate::display_color`]; the web previously mirrored
    /// them by hand in JS and was already four pixels stale.
    pub fn panel_tap(&mut self, x: u32, y: u32, ctx: TapCtx) -> Vec<Out> {
        use crate::device_view::{DEV_H, DEV_W};
        use crate::display_color as dc;
        let mut outs = Vec::new();
        if x >= DEV_W || y >= DEV_H {
            return outs;
        }

        // The gen picker owns the whole panel. A tap PICKS the row it landed
        // on and then confirms it — it used to confirm whatever the cursor
        // already sat on, so tapping "GEN 3" started a Gen 1 battle.
        //
        // The picker is drawn into each half, so a tap is mapped through the
        // half it hit; the far half is rotated 180. Rows are laid out by
        // `display_color::render_gen_picker` at y = 48 + i * 46, 38 tall.
        if self.menu_active && ctx.lobby {
            let hy = if y < DEV_H / 2 {
                DEV_H / 2 - 1 - y
            } else {
                y - DEV_H / 2
            };
            // A tap on a menu is a tap on a ROW, not a blind confirm of
            // whatever the cursor happened to be on: tapping GEN 3 used to
            // start a Gen 1 battle. Taps outside every card still confirm the
            // current row, which is what the seat taps did before.
            if let Some(row) = crate::display_color::menu_row_at(
                self.menu.screen,
                hy as i32,
                self.menu.row_count(),
            ) {
                self.menu.point_at(row);
            }
            let out = self.menu.confirm(&mut self.opts);
            self.close_menu_on(out, &mut outs);
            return outs;
        }

        // Which half, and where inside it. The far half is drawn rotated 180,
        // so its taps un-rotate into that seat's own coordinates.
        let (player, hx, hy) = if y < DEV_H / 2 {
            (2u8, DEV_W - 1 - x, DEV_H / 2 - 1 - y)
        } else {
            (1u8, x, y - DEV_H / 2)
        };
        let i = idx(player);

        // This seat's options overlay: a tap confirms the row.
        if self.seats[i].options.is_some() {
            return self.button(player, Button::TapSeat, Ctx { lobby: ctx.lobby, choosing: false, waiting: false });
        }

        if ctx.lobby {
            return self.button(player, Button::TapSeat, Ctx { lobby: true, choosing: false, waiting: false });
        }

        // Committed: a tap on your own half takes the choice back. It says
        // CANCEL outright — this used to replay a tap on the cursor slot,
        // which only unreadies because the Gen 1 collector treats any press
        // while committed that way, and the same trick was what left B dead
        // on the Gen 3 path.
        if ctx.waiting[i] {
            outs.push(Out::CancelChoice(player));
            return outs;
        }

        // Over the move menu or the party list: a tap is a whole decision.
        use crate::cursor_nav::NavMode;
        let target = match self.seats[i].nav.mode {
            NavMode::Moves => {
                let (bx, by) = (dc::LEFT as u32, dc::MENU_Y as u32);
                let (bw, bh) = (148u32, dc::MENU_H);
                if hx < bx || hx >= bx + bw || hy < by || hy >= by + bh {
                    None
                } else {
                    let col = ((hx - bx) * 2 / bw) as u8;
                    let row = ((hy - by) * 2 / bh) as u8;
                    Some(row * 2 + col)
                }
            }
            NavMode::Party => {
                let top = dc::PARTY_Y as u32;
                let pitch = dc::PARTY_PITCH as u32;
                if hy < top || hy >= top + pitch * 6 {
                    None
                } else {
                    Some(((hy - top) / pitch) as u8)
                }
            }
            NavMode::Detail => None,
        };
        if let Some(index) = target {
            let bctx = Ctx { lobby: false, choosing: true, waiting: false };
            outs.extend(self.tap_commit(player, index, bctx));
        }
        outs
    }

    /// A tap that is a whole decision: point at `index` and commit it. An
    /// out-of-range tap commits nothing, rather than whatever the cursor
    /// happened to be on.
    pub fn tap_commit(&mut self, player: u8, index: u8, ctx: Ctx) -> Vec<Out> {
        let mut outs = Vec::new();
        if self.set_cursor(player, index, ctx) {
            self.nav(player, &mut outs, |n| n.confirm());
        }
        outs
    }

    fn nav(
        &mut self,
        player: u8,
        outs: &mut Vec<Out>,
        f: impl FnOnce(&mut crate::cursor_nav::CursorNav) -> NavOut,
    ) {
        let out = f(&mut self.seats[idx(player)].nav);
        if !matches!(out, NavOut::None | NavOut::Redraw) {
            outs.push(Out::Nav(player, out));
        }
    }

    /// Leaving the picker starts a lobby, and a lobby always starts with
    /// nobody ready: readiness from before the picker was opened must not
    /// carry into a match neither player agreed to.
    fn close_menu_on(&mut self, out: MenuOut, outs: &mut Vec<Out>) {
        if out == MenuOut::EnterLobby {
            self.menu_active = false;
            for player in [1u8, 2] {
                if self.ready[idx(player)] {
                    outs.push(Out::CancelLine(player));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOBBY: Ctx = Ctx { lobby: true, choosing: false, waiting: false };
    const BATTLE: Ctx = Ctx { lobby: false, choosing: true, waiting: false };
    const PLAYBACK: Ctx = Ctx { lobby: false, choosing: false, waiting: false };

    #[test]
    fn boots_on_the_gen_picker_and_the_choice_reaches_the_ruleset() {
        let mut s = DeviceSession::new();
        assert!(s.menu_active);
        assert_eq!(s.ruleset(), Ruleset::Gen1);
        // Down then A picks Gen 3 — the same two presses on either platform.
        s.button(1, Button::Dpad(Dir::Down), LOBBY);
        s.button(1, Button::A, LOBBY);
        assert!(!s.menu_active, "picking a game hands the panel to the lobby");
        assert_eq!(s.ruleset(), Ruleset::Gen3);
    }

    /// A tap on the picker picks the row that was TOUCHED. It used to
    /// confirm whatever the cursor sat on, so tapping GEN 3 started Gen 1.
    #[test]
    fn tapping_a_picker_row_picks_that_row() {
        use crate::display_color::GEN_ROW;
        const TAP: TapCtx = TapCtx { lobby: true, waiting: [false; 2] };
        let (top, pitch, h) = GEN_ROW;
        let row_y = |row: i32| crate::DEV_H / 2 + (top + row * pitch + h as i32 / 2) as u32;

        let mut s = DeviceSession::new();
        s.panel_tap(60, row_y(1), TAP);
        assert_eq!(s.ruleset(), Ruleset::Gen3, "tapped GEN 3, got Gen 1");

        let mut s = DeviceSession::new();
        s.menu.point_at(1);
        s.panel_tap(60, row_y(0), TAP);
        assert_eq!(s.ruleset(), Ruleset::Gen1, "tapped GEN 1 with the cursor on GEN 3");
    }

    #[test]
    fn the_options_are_per_seat_over_shared_settings() {
        let mut s = DeviceSession::new();
        s.button(1, Button::A, LOBBY); // leave the picker
        s.button(1, Button::Info, LOBBY);
        assert!(s.seats[0].options.is_some());
        assert!(s.seats[1].options.is_none(), "the other seat stays in the lobby");
        // P1 flips a shared setting from inside their private overlay.
        let before = s.opts.team_size;
        s.button(1, Button::Dpad(Dir::Right), LOBBY);
        assert_ne!(s.opts.team_size, before);
        s.button(1, Button::B, LOBBY);
        assert!(s.seats[0].options.is_none());
    }

    #[test]
    fn lobby_b_cancels_then_sends_the_robot_home_then_reopens_the_picker() {
        let mut s = DeviceSession::new();
        s.button(1, Button::A, LOBBY);

        s.set_lobby_flags([true, false], [false, false]);
        assert_eq!(s.button(1, Button::B, LOBBY), alloc::vec![Out::CancelLine(1)]);

        s.set_lobby_flags([false, true], [false, true]);
        assert_eq!(
            s.button(1, Button::B, LOBBY),
            alloc::vec![Out::CancelLine(2)],
            "B dismisses the AI the player summoned",
        );

        s.set_lobby_flags([false, false], [false, false]);
        assert!(s.button(1, Button::B, LOBBY).is_empty());
        assert!(s.menu_active, "an idle seat's B reopens the picker");
    }

    #[test]
    fn a_tap_toggles_readiness() {
        let mut s = DeviceSession::new();
        s.button(1, Button::A, LOBBY);
        assert_eq!(s.button(1, Button::TapSeat, LOBBY), alloc::vec![Out::ReadyLine(1)]);
        s.set_lobby_flags([true, false], [false, false]);
        assert_eq!(s.button(1, Button::TapSeat, LOBBY), alloc::vec![Out::CancelLine(1)]);
    }

    #[test]
    fn leaving_the_picker_unreadies_whoever_was_ready() {
        let mut s = DeviceSession::new();
        s.button(1, Button::A, LOBBY); // into the lobby
        s.set_lobby_flags([true, false], [false, false]);
        s.button(2, Button::B, LOBBY); // idle seat reopens the picker
        let outs = s.button(2, Button::A, LOBBY); // and confirms back out
        assert!(outs.contains(&Out::CancelLine(1)), "P1's stale readiness is cancelled");
    }

    #[test]
    fn info_explains_while_choosing_and_opens_the_log_in_playback() {
        let mut s = DeviceSession::new();
        s.button(1, Button::A, LOBBY);
        s.begin_turn(1, 4, 3, false);
        s.log_line("Turn 1");

        let outs = s.button(1, Button::Info, BATTLE);
        assert!(matches!(outs.as_slice(), [Out::Nav(1, _)]), "choosing: ? explains the cursor");

        // Reset the detail state, then ask during playback.
        s.begin_turn(1, 4, 3, false);
        s.button(1, Button::Info, PLAYBACK);
        assert!(s.log_view(1).is_some(), "playback: ? opens the log");
        s.button(1, Button::B, PLAYBACK);
        assert!(s.log_view(1).is_none());
    }

    #[test]
    fn a_tap_commit_is_a_bounded_cursor_move_plus_confirm() {
        let mut s = DeviceSession::new();
        s.button(1, Button::A, LOBBY);
        s.begin_turn(1, 2, 3, false);
        let outs = s.tap_commit(1, 1, BATTLE);
        assert!(matches!(outs.as_slice(), [Out::Nav(1, NavOut::TapMove(1))]));
        assert!(s.tap_commit(1, 3, BATTLE).is_empty(), "out-of-range taps commit nothing");
    }
}
