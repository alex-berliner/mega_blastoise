//! Shared OLED screen orchestration — the ONE place that decides what each
//! player's display shows.
//!
//! Both the RP2040 firmware and the web client feed [`OledCmd`]s into an
//! [`OledController`] and render whatever [`OledController::screen`] says is
//! current, via [`render_screen`]. Platforms own only the plumbing (channels,
//! flushing, pixel upload); putting the state machine here means the two
//! builds cannot drift (e.g. different boot screens, or a screen existing on
//! one platform but not the other).
//!
//! Typical platform loop:
//! ```text
//! let mut ctl = OledController::new();          // boots showing the idle lobby
//! render_screen(&mut display, &ctl.screen(1));  // initial draw
//! ...
//! match ctl.apply(cmd) {
//!     OledRedraw::None => {}
//!     which => for p in which.players() { render_screen(&mut d[p], &ctl.screen(p)); flush(p); }
//! }
//! ```

extern crate alloc;

use alloc::vec::Vec;

use embedded_graphics::{draw_target::DrawTarget, pixelcolor::BinaryColor};

use crate::board_event::{BoardEvent, MoveSlot};
use crate::display::{
    render_event_text, render_invalid_selection, render_lobby_screen, render_move_detail,
    render_player_screen, render_pokemon_stats, render_pokemon_stats_page2, render_switch_screen,
    render_waiting_for_opponent, render_waiting_screen, render_win_screen, PartySlotData,
};

// ── Commands ──────────────────────────────────────────────────────────────────

/// Everything that can change what an OLED shows. `player` is 1 or 2;
/// [`OledCmd::EventFlash`] additionally accepts 0 = both displays.
///
/// Names and flash text use fixed byte buffers (not `String`) so commands can
/// cross the firmware's static channel without allocation.
pub enum OledCmd {
    /// HP changed; `pct` is 0–100. Data-only (HP renders on the LED strip):
    /// the current view is kept so narration flashes aren't interrupted.
    HpUpdate { player: u8, pct: u8 },
    /// New active Pokémon (UTF-8 name, up to 12 bytes).
    ActiveMon { player: u8, name: [u8; 12], len: u8 },
    /// Move list updated (PP changes after each turn).
    MovesUpdate { player: u8, moves: Vec<MoveSlot> },
    /// A mon fainted.
    Faint { player: u8 },
    /// Battle ended — winner is 1 (p1) or 2 (p2); 0 means tie.
    Win { winner: u8 },
    /// Long-press detail view for a move slot (0-based).
    ShowMoveDetail { player: u8, slot: u8 },
    /// Long-press stats view for a party slot (0-based team index).
    /// `page` 0 = stats, 1 = moves/PP page.
    ShowPokemonStats { player: u8, team_idx: u8, page: u8 },
    /// Update the cached party snapshot used by stats/switch screens.
    PartyUpdate { player: u8, slots: Vec<PartySlotData> },
    /// Restore the normal battle screen after a detail/overlay view.
    RestoreScreen { player: u8 },
    /// Lobby ready state. ready=false → idle instructions;
    /// ready=true,ai=false → "READY!"; ready=true,ai=true → "AI".
    LobbyState { player: u8, ready: bool, ai: bool },
    /// Transient battle-narration overlay (move used, crit, miss, status, …).
    /// Held visible by the caller's animation delay; the next state redraw
    /// (HpUpdate / MovesUpdate / ActiveMon / RestoreScreen) clears it.
    /// `player` 0 = show on both displays.
    EventFlash { player: u8, text: [u8; 48], len: u8 },
    /// "<mon> — waiting… / tap to unready" after this player locked a move.
    ShowWaiting { player: u8 },
    /// Opponent still choosing.
    ShowWaitingForOpponent { player: u8 },
    /// Forced-switch party picker (uses the PartyUpdate snapshot).
    ShowSwitchScreen { player: u8 },
    /// Brief "invalid selection" feedback.
    ShowInvalidSelection { player: u8 },
}

impl OledCmd {
    /// The player field, regardless of variant.
    fn player(&self) -> u8 {
        match self {
            OledCmd::HpUpdate { player, .. }
            | OledCmd::ActiveMon { player, .. }
            | OledCmd::MovesUpdate { player, .. }
            | OledCmd::Faint { player }
            | OledCmd::ShowMoveDetail { player, .. }
            | OledCmd::ShowPokemonStats { player, .. }
            | OledCmd::PartyUpdate { player, .. }
            | OledCmd::RestoreScreen { player }
            | OledCmd::LobbyState { player, .. }
            | OledCmd::EventFlash { player, .. }
            | OledCmd::ShowWaiting { player }
            | OledCmd::ShowWaitingForOpponent { player }
            | OledCmd::ShowSwitchScreen { player }
            | OledCmd::ShowInvalidSelection { player } => *player,
            OledCmd::Win { .. } => 0,
        }
    }
}

/// Copy up to 12 bytes of a Pokémon name into a fixed-size buffer.
pub fn name_buf(name: &str) -> ([u8; 12], u8) {
    let bytes = name.as_bytes();
    let len = bytes.len().min(12) as u8;
    let mut buf = [b' '; 12];
    buf[..len as usize].copy_from_slice(&bytes[..len as usize]);
    (buf, len)
}

/// Copy up to 48 bytes of narration text into a fixed-size buffer.
pub fn flash_buf(text: &str) -> ([u8; 48], u8) {
    let bytes = text.as_bytes();
    let len = bytes.len().min(48) as u8;
    let mut buf = [b' '; 48];
    buf[..len as usize].copy_from_slice(&bytes[..len as usize]);
    (buf, len)
}

/// Map a battle event to the OLED commands it implies — the SAME on every
/// platform. Platforms interleave their own side effects (LEDs, buzzer,
/// animation delays, JS mirrors) around these, but what the displays show
/// comes only from here.
///
/// Combat narration (move used, crit, miss, status, …) flashes on BOTH
/// screens (`player: 0`): it is shared context, not one side's private info.
pub fn oled_cmds_for_event(event: &BoardEvent) -> Vec<OledCmd> {
    use crate::board_event::{mon_player_num, player_id_to_num};
    use crate::hp_bar::HpBarState;

    let mut cmds = Vec::new();
    let mut flash = |text: &str| {
        let (buf, len) = flash_buf(text);
        cmds.push(OledCmd::EventFlash { player: 0, text: buf, len });
    };

    match event {
        BoardEvent::Damage { mon, health } | BoardEvent::Heal { mon, health } => {
            if let (Some(player), Some(hp)) = (mon_player_num(mon), HpBarState::parse(health)) {
                cmds.push(OledCmd::HpUpdate { player, pct: hp.pct() });
            }
        }
        BoardEvent::Faint { mon, .. } => {
            if let Some(player) = mon_player_num(mon) {
                cmds.push(OledCmd::Faint { player });
            }
        }
        BoardEvent::SwitchIn { name, player_id, moves, .. } => {
            if let Some(pid) = player_id {
                let player = player_id_to_num(pid);
                // Update the battle-screen data first, then flash
                // "<trainer> sent out X!" on that player's display — the
                // next prompt's RestoreScreen reveals the updated screen.
                // Per-player (not broadcast) so simultaneous send-ins show
                // each side's own message.
                let (buf, len) = name_buf(name.as_str());
                cmds.push(OledCmd::ActiveMon { player, name: buf, len });
                if !moves.is_empty() {
                    cmds.push(OledCmd::MovesUpdate { player, moves: moves.clone() });
                }
                let (text, tlen) = flash_buf(&event.description());
                cmds.push(OledCmd::EventFlash { player, text, len: tlen });
            }
        }
        BoardEvent::MovesUpdate { player_id, moves } => {
            let player = player_id_to_num(player_id.as_str());
            cmds.push(OledCmd::MovesUpdate { player, moves: moves.clone() });
        }
        BoardEvent::Prompt { player_id, .. } => {
            // Restore the normal view at prompt start in case a long-press
            // detail screen was left open (e.g. USB won the input race).
            let player = player_id_to_num(player_id.as_str());
            cmds.push(OledCmd::RestoreScreen { player });
        }
        BoardEvent::Win { side } => {
            cmds.push(OledCmd::Win { winner: BoardEvent::win_player_num(side) });
        }
        BoardEvent::Tie => {
            cmds.push(OledCmd::Win { winner: 0 });
        }
        // Transient narration flashes for events without a state screen.
        BoardEvent::Move { .. }
        | BoardEvent::SuperEffective { .. }
        | BoardEvent::CriticalHit { .. }
        | BoardEvent::SetStatus { .. }
        | BoardEvent::CureStatus { .. }
        | BoardEvent::Miss { .. }
        | BoardEvent::Immune { .. }
        | BoardEvent::Resisted { .. }
        | BoardEvent::Fail { .. }
        | BoardEvent::Cant { .. } => flash(&event.description()),
        _ => {}
    }
    cmds
}

// ── Current screen (what a display should show right now) ────────────────────

/// A fully-resolved description of one display's current contents.
/// [`render_screen`] turns this into pixels; platforms never pick a
/// `render_*` function themselves.
/// Sprite bob cadence on the battle screen (one 2-px hop per period).
pub const BOB_PERIOD_MS: u64 = 900;

pub enum Screen<'a> {
    Lobby { ready: bool, ai: bool },
    Battle { mon: &'a str, moves: &'a [MoveSlot], bob: bool },
    MoveDetail(&'a MoveSlot),
    Stats { slot: &'a PartySlotData, page: u8 },
    EventText(&'a str),
    Win(&'a str),
    Waiting { mon: &'a str },
    WaitingForOpponent,
    Switch(&'a [PartySlotData]),
    Invalid,
}

/// Render a [`Screen`] onto any 128×64 target. The single dispatch point
/// from screen state to the shared `render_*` functions.
pub fn render_screen<D>(display: &mut D, screen: &Screen<'_>)
where
    D: DrawTarget<Color = BinaryColor>,
{
    match screen {
        Screen::Lobby { ready, ai } => render_lobby_screen(display, *ready, *ai),
        Screen::Battle { mon, moves, bob } => {
            render_player_screen(display, mon, moves, if *bob { -2 } else { 0 })
        }
        Screen::MoveDetail(mv) => render_move_detail(display, mv),
        Screen::Stats { slot, page: 0 } => render_pokemon_stats(display, slot),
        Screen::Stats { slot, .. } => render_pokemon_stats_page2(display, slot),
        Screen::EventText(text) => render_event_text(display, text),
        Screen::Win(msg) => render_win_screen(display, msg),
        Screen::Waiting { mon } => render_waiting_screen(display, mon, "tap to unready"),
        Screen::WaitingForOpponent => render_waiting_for_opponent(display),
        Screen::Switch(party) => render_switch_screen(display, party),
        Screen::Invalid => render_invalid_selection(display),
    }
}

// ── Per-player state ──────────────────────────────────────────────────────────

enum View {
    Lobby { ready: bool, ai: bool },
    Battle,
    MoveDetail(u8),
    Stats { team_idx: u8, page: u8 },
    EventFlash,
    Win,
    Waiting,
    WaitingForOpponent,
    Switch,
    Invalid,
}

struct Player {
    name: [u8; 12],
    name_len: u8,
    hp_pct: u8,
    fainted: bool,
    moves: Vec<MoveSlot>,
    party: Vec<PartySlotData>,
    flash: [u8; 48],
    flash_len: u8,
    view: View,
}

impl Player {
    fn new() -> Self {
        let mut name = [b' '; 12];
        name[0] = b'-'; name[1] = b'-'; name[2] = b'-';
        Self {
            name,
            name_len: 3,
            hp_pct: 100,
            fainted: false,
            moves: Vec::new(),
            party: Vec::new(),
            flash: [b' '; 48],
            flash_len: 0,
            view: View::Lobby { ready: false, ai: false },
        }
    }

    fn name_str(&self) -> &str {
        core::str::from_utf8(&self.name[..self.name_len as usize]).unwrap_or("?")
    }

    fn battle_mon(&self) -> &str {
        if self.fainted { "FAINTED" } else { self.name_str() }
    }
}

// ── Controller ────────────────────────────────────────────────────────────────

/// Which displays a command changed. Returned by [`OledController::apply`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OledRedraw {
    None,
    P1,
    P2,
    Both,
}

impl OledRedraw {
    fn for_player(player: u8) -> Self {
        if player == 1 { OledRedraw::P1 } else { OledRedraw::P2 }
    }

    pub fn includes(self, player: u8) -> bool {
        match self {
            OledRedraw::None => false,
            OledRedraw::P1 => player == 1,
            OledRedraw::P2 => player == 2,
            OledRedraw::Both => true,
        }
    }
}

/// The shared two-display state machine. Starts in the idle lobby (the boot
/// screen on every platform).
pub struct OledController {
    p1: Player,
    p2: Player,
    winner: u8,
    bob_up: bool,
}

impl OledController {
    pub fn new() -> Self {
        Self { p1: Player::new(), p2: Player::new(), winner: 0, bob_up: false }
    }

    /// Advance the battle-screen sprite bob one step. Call every
    /// [`BOB_PERIOD_MS`]; returns which displays show the battle screen and
    /// need a redraw (None while neither does).
    pub fn tick_bob(&mut self) -> OledRedraw {
        self.bob_up = !self.bob_up;
        match (
            matches!(self.p1.view, View::Battle),
            matches!(self.p2.view, View::Battle),
        ) {
            (true, true) => OledRedraw::Both,
            (true, false) => OledRedraw::P1,
            (false, true) => OledRedraw::P2,
            (false, false) => OledRedraw::None,
        }
    }

    fn player_mut(&mut self, player: u8) -> &mut Player {
        if player == 1 { &mut self.p1 } else { &mut self.p2 }
    }

    /// Apply a command to the model. Returns which displays must be redrawn
    /// (render via [`Self::screen`] + [`render_screen`], then flush).
    pub fn apply(&mut self, cmd: OledCmd) -> OledRedraw {
        let player = cmd.player();
        match cmd {
            OledCmd::HpUpdate { pct, .. } => {
                // Data-only update: never steals the view, so a narration
                // flash sequence isn't interrupted by the battle screen
                // between damage events. (HP renders on the LED strip.)
                let p = self.player_mut(player);
                p.hp_pct = pct;
                if matches!(p.view, View::Battle) {
                    OledRedraw::for_player(player)
                } else {
                    OledRedraw::None
                }
            }
            OledCmd::ActiveMon { name, len, .. } => {
                // Data-only, like HpUpdate: the switch-in EventFlash (and the
                // next prompt's RestoreScreen) control what's on screen, so
                // the battle screen doesn't flash between them.
                let p = self.player_mut(player);
                p.name = name;
                p.name_len = len;
                p.fainted = false;
                p.hp_pct = 100;
                if matches!(p.view, View::Battle) {
                    OledRedraw::for_player(player)
                } else {
                    OledRedraw::None
                }
            }
            OledCmd::MovesUpdate { moves, .. } => {
                // Data-only update (PP refresh) — same rule as HpUpdate.
                let p = self.player_mut(player);
                p.moves = moves;
                if matches!(p.view, View::Battle) {
                    OledRedraw::for_player(player)
                } else {
                    OledRedraw::None
                }
            }
            OledCmd::Faint { .. } => {
                let p = self.player_mut(player);
                p.fainted = true;
                p.hp_pct = 0;
                p.moves.clear();
                p.view = View::Battle;
                OledRedraw::for_player(player)
            }
            OledCmd::Win { winner } => {
                self.winner = winner;
                self.p1.view = View::Win;
                self.p2.view = View::Win;
                OledRedraw::Both
            }
            OledCmd::ShowMoveDetail { slot, .. } => {
                let p = self.player_mut(player);
                if (slot as usize) < p.moves.len() {
                    p.view = View::MoveDetail(slot);
                    OledRedraw::for_player(player)
                } else {
                    OledRedraw::None
                }
            }
            OledCmd::ShowPokemonStats { team_idx, page, .. } => {
                let p = self.player_mut(player);
                if (team_idx as usize) < p.party.len() {
                    p.view = View::Stats { team_idx, page };
                    OledRedraw::for_player(player)
                } else {
                    OledRedraw::None
                }
            }
            OledCmd::PartyUpdate { slots, .. } => {
                self.player_mut(player).party = slots;
                OledRedraw::None
            }
            OledCmd::RestoreScreen { .. } => {
                self.player_mut(player).view = View::Battle;
                OledRedraw::for_player(player)
            }
            OledCmd::LobbyState { ready, ai, .. } => {
                self.player_mut(player).view = View::Lobby { ready, ai };
                OledRedraw::for_player(player)
            }
            OledCmd::EventFlash { player, text, len } => {
                let both = player == 0;
                if both || player == 1 {
                    self.p1.flash = text;
                    self.p1.flash_len = len;
                    self.p1.view = View::EventFlash;
                }
                if both || player == 2 {
                    self.p2.flash = text;
                    self.p2.flash_len = len;
                    self.p2.view = View::EventFlash;
                }
                if both { OledRedraw::Both } else { OledRedraw::for_player(player) }
            }
            OledCmd::ShowWaiting { .. } => {
                self.player_mut(player).view = View::Waiting;
                OledRedraw::for_player(player)
            }
            OledCmd::ShowWaitingForOpponent { .. } => {
                self.player_mut(player).view = View::WaitingForOpponent;
                OledRedraw::for_player(player)
            }
            OledCmd::ShowSwitchScreen { .. } => {
                self.player_mut(player).view = View::Switch;
                OledRedraw::for_player(player)
            }
            OledCmd::ShowInvalidSelection { .. } => {
                self.player_mut(player).view = View::Invalid;
                OledRedraw::for_player(player)
            }
        }
    }

    /// What `player`'s display should show right now.
    pub fn screen(&self, player: u8) -> Screen<'_> {
        let p = if player == 1 { &self.p1 } else { &self.p2 };
        match &p.view {
            View::Lobby { ready, ai } => Screen::Lobby { ready: *ready, ai: *ai },
            View::Battle => Screen::Battle { mon: p.battle_mon(), moves: &p.moves, bob: self.bob_up },
            View::MoveDetail(slot) => match p.moves.get(*slot as usize) {
                Some(mv) => Screen::MoveDetail(mv),
                None => Screen::Battle { mon: p.battle_mon(), moves: &p.moves, bob: self.bob_up },
            },
            View::Stats { team_idx, page } => match p.party.get(*team_idx as usize) {
                Some(slot) => Screen::Stats { slot, page: *page },
                None => Screen::Battle { mon: p.battle_mon(), moves: &p.moves, bob: self.bob_up },
            },
            View::EventFlash => Screen::EventText(
                core::str::from_utf8(&p.flash[..p.flash_len as usize]).unwrap_or(""),
            ),
            View::Win => {
                let (msg1, msg2) = BoardEvent::win_messages(self.winner);
                Screen::Win(if player == 1 { msg1 } else { msg2 })
            }
            View::Waiting => Screen::Waiting { mon: p.battle_mon() },
            View::WaitingForOpponent => Screen::WaitingForOpponent,
            View::Switch => Screen::Switch(&p.party),
            View::Invalid => Screen::Invalid,
        }
    }
}
