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
    render_action_select, render_concealed_moves, render_concealed_switch,
    render_controls_select, render_event_text, render_invalid_selection, render_lobby_screen,
    render_move_detail, render_player_screen, render_pokemon_stats, render_pokemon_stats_page2,
    render_move_used, render_opponent_mon, render_qr_screen, render_sent_out,
    render_switch_screen, render_waiting_for_opponent,
    render_waiting_screen, render_win_screen, InvalidReason, PartySlotData, SpeedCmp,
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
    /// New active Pokémon (UTF-8 name, up to 12 bytes). `speed` is the mon's
    /// Speed stat — drives the battle-screen sprite bob rate.
    ActiveMon { player: u8, name: [u8; 12], len: u8, speed: u16 },
    /// Move list updated (PP changes after each turn).
    MovesUpdate { player: u8, moves: Vec<MoveSlot> },
    /// A mon fainted.
    Faint { player: u8 },
    /// Battle ended — winner is 1 (p1) or 2 (p2); 0 means tie.
    Win { winner: u8 },
    /// Long-press detail view for a move slot (0-based). `page` 0 = stats;
    /// higher pages show the move's description text (the renderer wraps
    /// the counter around the real page count).
    ShowMoveDetail { player: u8, slot: u8, page: u8 },
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
    /// Brief "invalid selection" feedback; `reason` picks the message.
    ShowInvalidSelection { player: u8, reason: InvalidReason },
    /// Tell the controller which control scheme this player uses for the
    /// current battle. Concealed battle screens hide the move list (the
    /// randomized corner menus are the only move UI in that mode).
    SetControlMode { player: u8, concealed: bool },
    /// Post-game feedback QR code on BOTH displays.
    ShowQr,
    /// Battle-start controls picker. `highlighted` 0 = Normal, 1 = Concealed.
    ShowControlsSelect { player: u8, highlighted: u8, confirmed: bool },
    /// Concealed mode: randomized Attack/Switch on the bottom-row buttons.
    ShowActionSelect { player: u8, attack_pos: u8, switch_pos: u8 },
    /// Concealed move menu: corner → move slot (-1 = dead corner).
    ShowConcealedMoves { player: u8, map: [i8; 4] },
    /// Concealed bench menu: corner → team index (-1 = dead corner).
    ShowConcealedSwitch { player: u8, map: [i8; 4] },
    /// Concealed foe-peek: show the OPPONENT's active mon on this player's
    /// display (held unused bottom button).
    ShowOpponentMon { player: u8 },
    /// Switch-in flash showing the incoming mon's sprite under the
    /// "sent out" caption. `player` 0 = both displays.
    ShowSentOut { player: u8, name: [u8; 12], len: u8, text: [u8; 48], tlen: u8 },
    /// Move-used flash: the attacker's sprite with the move's icon to its
    /// right, under the "used X!" caption. `player` 0 = both displays.
    ShowMoveUsed {
        player: u8,
        attacker: u8,
        name: [u8; 12],
        len: u8,
        move_id: [u8; 16],
        mlen: u8,
        text: [u8; 48],
        tlen: u8,
    },
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
            | OledCmd::ShowInvalidSelection { player, .. }
            | OledCmd::SetControlMode { player, .. }
            | OledCmd::ShowControlsSelect { player, .. }
            | OledCmd::ShowActionSelect { player, .. }
            | OledCmd::ShowConcealedMoves { player, .. }
            | OledCmd::ShowConcealedSwitch { player, .. }
            | OledCmd::ShowOpponentMon { player }
            | OledCmd::ShowSentOut { player, .. }
            | OledCmd::ShowMoveUsed { player, .. } => *player,
            OledCmd::Win { .. } | OledCmd::ShowQr => 0,
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

/// Canonicalize a move's display name to its table id ("Body Slam" →
/// "bodyslam") in a fixed-size buffer: lowercase, alphanumerics only.
pub fn move_id_buf(name: &str) -> ([u8; 16], u8) {
    let mut buf = [b' '; 16];
    let mut len = 0usize;
    for c in name.chars() {
        if len >= 16 {
            break;
        }
        if c.is_ascii_alphanumeric() {
            buf[len] = c.to_ascii_lowercase() as u8;
            len += 1;
        }
    }
    (buf, len as u8)
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
                // The owner's display shows the FAINTED battle state; the
                // OPPONENT gets the "<trainer>'s <mon> fainted!" dialogue.
                cmds.push(OledCmd::Faint { player });
                let (text, len) = flash_buf(&event.description());
                cmds.push(OledCmd::EventFlash { player: 3 - player, text, len });
            }
        }
        BoardEvent::SwitchIn { name, player_id, moves, speed, .. } => {
            if let Some(pid) = player_id {
                let player = player_id_to_num(pid);
                // Update the battle-screen data first, then show the
                // "<trainer> sent out X!" screen WITH the incoming mon's
                // sprite on BOTH displays (shared context, like combat
                // narration) — the next prompt's RestoreScreen reveals the
                // updated battle screen.
                let (buf, len) = name_buf(name.as_str());
                cmds.push(OledCmd::ActiveMon {
                    player,
                    name: buf,
                    len,
                    speed: speed.unwrap_or(150),
                });
                if !moves.is_empty() {
                    cmds.push(OledCmd::MovesUpdate { player, moves: moves.clone() });
                }
                let (text, tlen) = flash_buf(&event.description());
                cmds.push(OledCmd::ShowSentOut { player: 0, name: buf, len, text, tlen });
            }
        }
        BoardEvent::MovesUpdate { player_id, moves } => {
            let player = player_id_to_num(player_id.as_str());
            cmds.push(OledCmd::MovesUpdate { player, moves: moves.clone() });
        }
        BoardEvent::ActiveMonUpdate { mon, name, speed } => {
            // Transform: same mon, new face — sprite/name/speed swap in place.
            if let Some(player) = mon_player_num(mon) {
                let (buf, len) = name_buf(name.as_str());
                cmds.push(OledCmd::ActiveMon {
                    player,
                    name: buf,
                    len,
                    speed: speed.unwrap_or(150),
                });
            }
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
        BoardEvent::Move { user, name, player_id } => {
            // "X used MOVE!" shows the attacker's sprite with the move's
            // icon beside it on BOTH displays (shared context).
            let (text, tlen) = flash_buf(&event.description());
            match (user.as_deref(), player_id.as_deref()) {
                (Some(mon), Some(pid)) => {
                    let (buf, len) = name_buf(mon);
                    let (mid, mlen) = move_id_buf(name);
                    cmds.push(OledCmd::ShowMoveUsed {
                        player: 0,
                        attacker: player_id_to_num(pid),
                        name: buf,
                        len,
                        move_id: mid,
                        mlen,
                        text,
                        tlen,
                    });
                }
                _ => cmds.push(OledCmd::EventFlash { player: 0, text, len: tlen }),
            }
        }
        // Transient narration flashes for events without a state screen.
        BoardEvent::SuperEffective { .. }
        | BoardEvent::CriticalHit { .. }
        | BoardEvent::SetStatus { .. }
        | BoardEvent::CureStatus { .. }
        | BoardEvent::StatChange { .. }
        | BoardEvent::EffectStart { .. }
        | BoardEvent::EffectEnd { .. }
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
/// Base sprite-bob period on the battle screen (one 2-px hop per period).
/// The actual period scales with the active mon's Speed stat — see
/// [`bob_period_ms`].
pub const BOB_BASE_PERIOD_MS: u32 = 900;

/// How often platforms should call [`OledController::tick_bob`].
pub const BOB_TICK_MS: u32 = 75;

/// Bob period for a given Speed stat.
///
/// The bob RATE (hops/sec) interpolates linearly from 1/4× the base rate at
/// `SPEED_LO` to 2× at `SPEED_HI` — linear in frequency, not period, so
/// mid-speed mons look mid-tempo instead of every slow mon bunching at
/// "barely moving". The clamp range matches real level-100 Gen 1 speed
/// stats (~100–320; boosted values can reach ~1000 but scaling across that
/// would flatten the differences that matter).
fn bob_period_ms(speed: u16) -> u32 {
    const SPEED_LO: u32 = 100;
    const SPEED_HI: u32 = 300;
    // Rate multiplier in permille: 250 (1/4×) … 2000 (2×).
    let s = (speed as u32).clamp(SPEED_LO, SPEED_HI);
    let rate_pm = 250 + (2000 - 250) * (s - SPEED_LO) / (SPEED_HI - SPEED_LO);
    BOB_BASE_PERIOD_MS * 1000 / rate_pm
}

/// Move-used icon flicker over the screen's MOVE_MS hold: sevenths of
/// ON, OFF, ON, OFF, then ON for the remaining 3/7.
fn move_icon_on(elapsed_ms: u32) -> bool {
    let total = crate::battle_effects::anim::MOVE_MS;
    if elapsed_ms >= total {
        return true;
    }
    !matches!(elapsed_ms * 7 / total, 1 | 3)
}

pub enum Screen<'a> {
    Lobby { ready: bool, ai: bool },
    Battle { mon: &'a str, moves: &'a [MoveSlot], bob: bool, spd: SpeedCmp },
    MoveDetail { mv: &'a MoveSlot, page: u8 },
    Stats { slot: &'a PartySlotData, page: u8 },
    EventText(&'a str),
    Win(&'a str),
    Waiting { mon: &'a str, bob: bool, spd: SpeedCmp },
    WaitingForOpponent { mon: &'a str, bob: bool },
    Switch(&'a [PartySlotData]),
    Invalid(InvalidReason),
    ControlsSelect { highlighted: u8, confirmed: bool },
    ActionSelect { mon: &'a str, bob: bool, attack_pos: u8, switch_pos: u8, spd: SpeedCmp },
    /// Corner labels for the concealed move menu (None = dead corner).
    ConcealedMoves { corners: [Option<&'a MoveSlot>; 4] },
    /// Corner labels for the concealed bench menu (None = dead corner).
    ConcealedSwitch { corners: [Option<&'a PartySlotData>; 4] },
    /// Concealed foe-peek: the opponent's active mon.
    OpponentMon { mon: &'a str, bob: bool },
    /// Switch-in: caption + the incoming mon's sprite.
    SentOut { mon: &'a str, caption: &'a str },
    /// Move used: caption + attacker sprite + (flickering) move icon +
    /// recipient sprite.
    MoveUsed { mon: &'a str, caption: &'a str, move_id: &'a str, recipient: &'a str, icon_on: bool },
    /// Post-game feedback QR code.
    Qr,
}

/// Render a [`Screen`] onto any 128×64 target. The single dispatch point
/// from screen state to the shared `render_*` functions.
pub fn render_screen<D>(display: &mut D, screen: &Screen<'_>)
where
    D: DrawTarget<Color = BinaryColor>,
{
    match screen {
        Screen::Lobby { ready, ai } => render_lobby_screen(display, *ready, *ai),
        Screen::Battle { mon, moves, bob, spd } => {
            render_player_screen(display, mon, moves, if *bob { -2 } else { 0 }, *spd)
        }
        Screen::MoveDetail { mv, page } => render_move_detail(display, mv, *page),
        // Page 0 = moves (shown first), page 1 = stats.
        Screen::Stats { slot, page: 0 } => render_pokemon_stats_page2(display, slot),
        Screen::Stats { slot, .. } => render_pokemon_stats(display, slot),
        Screen::EventText(text) => render_event_text(display, text),
        Screen::Win(msg) => render_win_screen(display, msg),
        Screen::Waiting { mon, bob, spd } => {
            render_waiting_screen(display, mon, if *bob { -2 } else { 0 }, "tap to unready", *spd)
        }
        Screen::WaitingForOpponent { mon, bob } => {
            render_waiting_for_opponent(display, mon, if *bob { -2 } else { 0 })
        }
        Screen::Switch(party) => render_switch_screen(display, party),
        Screen::Invalid(reason) => render_invalid_selection(display, *reason),
        Screen::ControlsSelect { highlighted, confirmed } => {
            render_controls_select(display, *highlighted, *confirmed)
        }
        Screen::ActionSelect { mon, bob, attack_pos, switch_pos, spd } => render_action_select(
            display,
            mon,
            if *bob { -2 } else { 0 },
            *attack_pos,
            *switch_pos,
            *spd,
        ),
        Screen::ConcealedMoves { corners } => render_concealed_moves(display, corners),
        Screen::ConcealedSwitch { corners } => render_concealed_switch(display, corners),
        Screen::OpponentMon { mon, bob } => {
            render_opponent_mon(display, mon, if *bob { -2 } else { 0 })
        }
        Screen::SentOut { mon, caption } => render_sent_out(display, mon, caption),
        Screen::MoveUsed { mon, caption, move_id, recipient, icon_on } => {
            render_move_used(display, mon, caption, move_id, recipient, *icon_on)
        }
        Screen::Qr => render_qr_screen(display),
    }
}

// ── Per-player state ──────────────────────────────────────────────────────────

enum View {
    Lobby { ready: bool, ai: bool },
    Battle,
    MoveDetail { slot: u8, page: u8 },
    Stats { team_idx: u8, page: u8 },
    EventFlash,
    Win,
    Waiting,
    WaitingForOpponent,
    Switch,
    Invalid(InvalidReason),
    Qr,
    ControlsSelect { highlighted: u8, confirmed: bool },
    ActionSelect { attack_pos: u8, switch_pos: u8 },
    ConcealedMoves { map: [i8; 4] },
    ConcealedSwitch { map: [i8; 4] },
    /// Foe-peek (concealed): the opponent's active mon.
    OpponentMon,
    /// Switch-in flash: `sent_name`/`sent_len` hold the incoming mon's name;
    /// the caption reuses the flash buffer.
    SentOut { name: [u8; 12], len: u8 },
    /// Move-used flash: attacker name + move id; caption in the flash buffer.
    MoveUsed { attacker: u8, name: [u8; 12], len: u8, move_id: [u8; 16], mlen: u8 },
}

struct Player {
    name: [u8; 12],
    name_len: u8,
    hp_pct: u8,
    fainted: bool,
    /// Concealed controls this battle: the battle screen hides the move
    /// list (set via [`OledCmd::SetControlMode`] at battle start).
    concealed: bool,
    moves: Vec<MoveSlot>,
    party: Vec<PartySlotData>,
    flash: [u8; 48],
    flash_len: u8,
    view: View,
    /// Active mon's Speed stat (bob pacing).
    speed: u16,
    /// Bob phase (true = sprite raised 2 px).
    bob_up: bool,
    /// Milliseconds accumulated toward the next bob flip.
    bob_acc_ms: u32,
    /// Milliseconds since a MoveUsed view appeared (drives the icon flicker).
    move_flash_ms: u32,
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
            concealed: false,
            moves: Vec::new(),
            party: Vec::new(),
            flash: [b' '; 48],
            flash_len: 0,
            view: View::Lobby { ready: false, ai: false },
            speed: 150,
            bob_up: false,
            bob_acc_ms: 0,
            move_flash_ms: 0,
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
}

impl OledController {
    pub fn new() -> Self {
        Self { p1: Player::new(), p2: Player::new(), winner: 0 }
    }

    /// Advance the battle-screen sprite bobs by `dt_ms` (call every
    /// [`BOB_TICK_MS`]). Each player's sprite flips at its own rate, scaled
    /// by that mon's Speed stat. Returns which displays showed a flip on the
    /// battle screen and need a redraw.
    pub fn tick_bob(&mut self, dt_ms: u32) -> OledRedraw {
        let mut flip = [false; 2];
        let mut bobbed = [false; 2];
        for (i, p) in [&mut self.p1, &mut self.p2].into_iter().enumerate() {
            p.bob_acc_ms += dt_ms;
            let period = bob_period_ms(p.speed);
            if p.bob_acc_ms >= period {
                p.bob_acc_ms %= period;
                p.bob_up = !p.bob_up;
                bobbed[i] = true;
                // Every sprite-bearing view bobs: battle, concealed action
                // select, and the waiting screens.
                flip[i] = matches!(
                    p.view,
                    View::Battle
                        | View::ActionSelect { .. }
                        | View::Waiting
                        | View::WaitingForOpponent
                );
            }
            // Move-used icon flicker: redraw whenever visibility flips.
            if matches!(p.view, View::MoveUsed { .. }) {
                let before = move_icon_on(p.move_flash_ms);
                p.move_flash_ms = p.move_flash_ms.saturating_add(dt_ms);
                if move_icon_on(p.move_flash_ms) != before {
                    flip[i] = true;
                }
            }
        }
        // The foe-peek view shows the OTHER player's sprite, so that
        // sprite's bob flip redraws the PEEKING display.
        for i in 0..2 {
            let p = if i == 0 { &self.p1 } else { &self.p2 };
            if matches!(p.view, View::OpponentMon) && bobbed[1 - i] {
                flip[i] = true;
            }
        }
        match (flip[0], flip[1]) {
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
            OledCmd::ActiveMon { name, len, speed, .. } => {
                // Data-only, like HpUpdate: the switch-in EventFlash (and the
                // next prompt's RestoreScreen) control what's on screen, so
                // the battle screen doesn't flash between them.
                let p = self.player_mut(player);
                p.name = name;
                p.name_len = len;
                p.fainted = false;
                p.hp_pct = 100;
                p.speed = speed;
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
            OledCmd::ShowMoveDetail { slot, page, .. } => {
                let p = self.player_mut(player);
                if (slot as usize) < p.moves.len() {
                    p.view = View::MoveDetail { slot, page };
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
                let p = self.player_mut(player);
                if let Some(active) = slots.iter().find(|s| s.active && s.hp > 0) {
                    p.speed = active.spe;
                }
                p.party = slots;
                // Speed feeds the badge on sprite screens.
                OledRedraw::for_player(player)
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
            OledCmd::ShowInvalidSelection { reason, .. } => {
                self.player_mut(player).view = View::Invalid(reason);
                OledRedraw::for_player(player)
            }
            OledCmd::SetControlMode { concealed, .. } => {
                let p = self.player_mut(player);
                p.concealed = concealed;
                if matches!(p.view, View::Battle) {
                    OledRedraw::for_player(player)
                } else {
                    OledRedraw::None
                }
            }
            OledCmd::ShowQr => {
                self.p1.view = View::Qr;
                self.p2.view = View::Qr;
                OledRedraw::Both
            }
            OledCmd::ShowControlsSelect { highlighted, confirmed, .. } => {
                self.player_mut(player).view = View::ControlsSelect { highlighted, confirmed };
                OledRedraw::for_player(player)
            }
            OledCmd::ShowActionSelect { attack_pos, switch_pos, .. } => {
                self.player_mut(player).view = View::ActionSelect { attack_pos, switch_pos };
                OledRedraw::for_player(player)
            }
            OledCmd::ShowConcealedMoves { map, .. } => {
                self.player_mut(player).view = View::ConcealedMoves { map };
                OledRedraw::for_player(player)
            }
            OledCmd::ShowConcealedSwitch { map, .. } => {
                self.player_mut(player).view = View::ConcealedSwitch { map };
                OledRedraw::for_player(player)
            }
            OledCmd::ShowOpponentMon { .. } => {
                self.player_mut(player).view = View::OpponentMon;
                OledRedraw::for_player(player)
            }
            OledCmd::ShowSentOut { player, name, len, text, tlen } => {
                let both = player == 0;
                for p in [1u8, 2] {
                    if both || player == p {
                        let pl = self.player_mut(p);
                        pl.flash = text;
                        pl.flash_len = tlen;
                        pl.view = View::SentOut { name, len };
                    }
                }
                if both { OledRedraw::Both } else { OledRedraw::for_player(player) }
            }
            OledCmd::ShowMoveUsed { player, attacker, name, len, move_id, mlen, text, tlen } => {
                let both = player == 0;
                for p in [1u8, 2] {
                    if both || player == p {
                        let pl = self.player_mut(p);
                        pl.flash = text;
                        pl.flash_len = tlen;
                        pl.move_flash_ms = 0;
                        pl.view = View::MoveUsed { attacker, name, len, move_id, mlen };
                    }
                }
                if both { OledRedraw::Both } else { OledRedraw::for_player(player) }
            }
        }
    }

    /// Speed badge for `player`: their active mon's Speed vs the opponent's.
    fn speed_cmp(&self, player: u8) -> SpeedCmp {
        let (own, foe) = if player == 1 {
            (self.p1.speed, self.p2.speed)
        } else {
            (self.p2.speed, self.p1.speed)
        };
        if own > foe {
            SpeedCmp::Faster
        } else if own < foe {
            SpeedCmp::Slower
        } else {
            SpeedCmp::Even
        }
    }

    /// What `player`'s display should show right now.
    pub fn screen(&self, player: u8) -> Screen<'_> {
        let p = if player == 1 { &self.p1 } else { &self.p2 };
        // No speed badge on a fainted mon's screens — nothing left to race.
        let spd = if p.fainted { SpeedCmp::Hidden } else { self.speed_cmp(player) };
        match &p.view {
            View::Lobby { ready, ai } => Screen::Lobby { ready: *ready, ai: *ai },
            View::Battle => Screen::Battle {
                mon: p.battle_mon(),
                // Concealed controls: moves only ever appear on the
                // randomized corner menus, never the battle screen.
                moves: if p.concealed { &[] } else { &p.moves },
                bob: p.bob_up,
                spd,
            },
            View::MoveDetail { slot, page } => match p.moves.get(*slot as usize) {
                Some(mv) => Screen::MoveDetail { mv, page: *page },
                None => Screen::Battle { mon: p.battle_mon(), moves: &p.moves, bob: p.bob_up, spd },
            },
            View::Stats { team_idx, page } => match p.party.get(*team_idx as usize) {
                Some(slot) => Screen::Stats { slot, page: *page },
                None => Screen::Battle { mon: p.battle_mon(), moves: &p.moves, bob: p.bob_up, spd },
            },
            View::EventFlash => Screen::EventText(
                core::str::from_utf8(&p.flash[..p.flash_len as usize]).unwrap_or(""),
            ),
            View::Win => {
                let (msg1, msg2) = BoardEvent::win_messages(self.winner);
                Screen::Win(if player == 1 { msg1 } else { msg2 })
            }
            View::Waiting => Screen::Waiting { mon: p.battle_mon(), bob: p.bob_up, spd },
            View::WaitingForOpponent => {
                Screen::WaitingForOpponent { mon: p.battle_mon(), bob: p.bob_up }
            }
            View::Switch => Screen::Switch(&p.party),
            View::Invalid(reason) => Screen::Invalid(*reason),
            View::Qr => Screen::Qr,
            View::ControlsSelect { highlighted, confirmed } => Screen::ControlsSelect {
                highlighted: *highlighted,
                confirmed: *confirmed,
            },
            View::ActionSelect { attack_pos, switch_pos } => Screen::ActionSelect {
                mon: p.battle_mon(),
                bob: p.bob_up,
                attack_pos: *attack_pos,
                switch_pos: *switch_pos,
                spd,
            },
            View::ConcealedMoves { map } => {
                let mut corners: [Option<&MoveSlot>; 4] = [None; 4];
                for (k, c) in corners.iter_mut().enumerate() {
                    if map[k] >= 0 {
                        *c = p.moves.get(map[k] as usize);
                    }
                }
                Screen::ConcealedMoves { corners }
            }
            View::ConcealedSwitch { map } => {
                let mut corners: [Option<&PartySlotData>; 4] = [None; 4];
                for (k, c) in corners.iter_mut().enumerate() {
                    if map[k] >= 0 {
                        *c = p.party.get(map[k] as usize);
                    }
                }
                Screen::ConcealedSwitch { corners }
            }
            View::OpponentMon => {
                let foe = if player == 1 { &self.p2 } else { &self.p1 };
                Screen::OpponentMon { mon: foe.battle_mon(), bob: foe.bob_up }
            }
            View::SentOut { name, len } => Screen::SentOut {
                mon: core::str::from_utf8(&name[..*len as usize]).unwrap_or("?").trim_end(),
                caption: core::str::from_utf8(&p.flash[..p.flash_len as usize]).unwrap_or(""),
            },
            View::MoveUsed { attacker, name, len, move_id, mlen } => Screen::MoveUsed {
                mon: core::str::from_utf8(&name[..*len as usize]).unwrap_or("?").trim_end(),
                caption: core::str::from_utf8(&p.flash[..p.flash_len as usize]).unwrap_or(""),
                move_id: core::str::from_utf8(&move_id[..*mlen as usize]).unwrap_or(""),
                recipient: if *attacker == 1 { self.p2.battle_mon() } else { self.p1.battle_mon() },
                icon_on: move_icon_on(p.move_flash_ms),
            },
        }
    }
}
