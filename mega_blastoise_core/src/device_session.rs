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
    /// A battle-navigation outcome for this seat (tap/hold a move or switch).
    Nav(u8, NavOut),
    /// The lobby long-press: this seat asked for an AI opponent.
    LobbyLongPress(u8),
}

/// What the session cannot know on its own this instant, passed per call.
#[derive(Clone, Copy, Debug)]
pub struct Ctx {
    /// The game loop is in its lobby phase (no battle running).
    pub lobby: bool,
    /// This seat's battle screen is a choosing screen (moves or party), which
    /// decides whether `?` explains the cursor or opens the log.
    pub choosing: bool,
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

    const LOBBY: Ctx = Ctx { lobby: true, choosing: false };
    const BATTLE: Ctx = Ctx { lobby: false, choosing: true };
    const PLAYBACK: Ctx = Ctx { lobby: false, choosing: false };

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
