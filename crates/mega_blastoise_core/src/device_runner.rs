//! The device's game loop — lobby, ready sequence, battle, recap — as ONE
//! platform-neutral runner.
//!
//! This is the spine the fidelity rule demands: everything that decides what
//! the device DOES lives here, once. A platform brings raw IO as a
//! [`DevicePort`] (clock, sleeps, console line out, a frame-presentation
//! hook, entropy, LED-ish side effects) plus a [`DeviceInputChannel`] it
//! pushes classified input into, and calls [`DeviceRunner::run`]. The web
//! build and the firmware run this same future; their diffs are IO skins.
//!
//! History: this logic previously lived in `crates/mega_blastoise_web/src/lib.rs`
//! as ~700 lines of wasm-only glue, so the firmware had no battles behind
//! its single-screen UI at all. Hoisted 2026-08-27.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::cell::RefCell;

use embassy_futures::select::{select, select3, Either, Either3};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;

use crate::battle_effects::BoardEffects;
use crate::battle_input::{ActivePrompt, InputBus, PlayerChoice};
use crate::board_event::{BoardEvent, BoardEventExt};
use crate::choice_collect::{
    ChoiceCollector, ControlMode, Effect as CollectEffect, PadEvent, ReadySequence, SlotOptions,
    COLLECT_TICK_MS,
};
use crate::cursor_nav::NavOut;
use crate::device_session::{Button, Ctx, DeviceSession, Out, TapCtx};
use crate::display::PartySlotData;
use crate::oled_ctl::{oled_cmds_for_event, OledCmd, OledController, BOB_TICK_MS};
use crate::random_ai::RandomAi;
use crate::transport::LocalTransport;
use crate::runner;
use crate::runner::Engine;
use crate::{player_id_to_num, LOBBY_DEMO_DELAY_MS};
use gen3_battle::Ruleset;

// ── The platform's side of the contract ──────────────────────────────────────

/// What a platform must supply. Methods take `&self`: implementations keep
/// their own interior mutability (both targets are single-threaded async).
#[allow(async_fn_in_trait)] // single-threaded async on both targets, by design
pub trait DevicePort {
    /// Monotonic-enough milliseconds (drives debounce grace and tick clocks).
    fn now_ms(&self) -> u64;
    /// Plain clock sleep — never skipped, never animation-gated.
    async fn sleep_ms(&self, ms: u32);
    /// A console/diagnostic line (web log panel, fw RTT/CLI).
    fn line_out(&self, line: &str);
    /// State changed: repaint if and how you like. Called with borrows into
    /// the live state; render or copy inside, do not stash the references.
    fn present(&self, session: &DeviceSession, ctl: &OledController, lobby: bool);
    /// Fresh entropy for a game seed.
    fn seed(&self) -> u64;
    /// A battle event's platform side-channel (LED strips, JS flashes).
    /// Display consequences are NOT this — they flow through the controller.
    fn side_event(&self, _event: &BoardEvent, _parties: &[Vec<PartySlotData>; 2]) {}
    /// The lobby's ambient animation tick (web LED breathing). Optional.
    fn lobby_pulse(&self, _ready: [bool; 2], _ai: [bool; 2]) {}
    /// What the battery sense reads right now. Platforms with no cell (the
    /// web, a bench Pico on USB with the switch off) report Absent.
    fn battery(&self) -> crate::display_color::Battery {
        crate::display_color::Battery::Absent
    }
    /// Which engine plays the next battle (asked once per battle). Raw
    /// configuration only: the web's `engine=` URL parameter; the firmware
    /// takes [`Engine::DEVICE_DEFAULT`].
    fn engine(&self) -> Engine {
        Engine::DEVICE_DEFAULT
    }
}

// ── Input plumbing ───────────────────────────────────────────────────────────

/// One unit of classified platform input. Classification (tap vs 500 ms
/// hold) is the platform's only job; meaning is decided here and below.
pub enum DeviceEvent {
    /// A session-level press: D-pad, A, B, Info, or the A-hold.
    Button { player: u8, button: Button },
    /// A raw tap on the panel, in panel pixels.
    PanelTap { x: u32, y: u32 },
    /// A pre-classified battle pad event (web sticky-latch, `:press` sims).
    Pad(PadEvent),
    /// A typed line: lobby grammar, battle grammar — platform commands like
    /// `:reset` are the platform's to intercept before pushing.
    Line(String),
    /// The demo / VS-AI presets. `demo` also latches auto-restart.
    AiPreset { ai: [bool; 2], demo: bool },
    /// Web debug affordance: gate AI turns.
    SetAiPause(bool),
    /// Animation gate (`:anim on|off`).
    SetAnim(bool),
    /// Platform heartbeat at [`BOB_TICK_MS`]: advances sprite bobs.
    Pulse,
    /// Platform asks for an unconditional present (the fw display task uses
    /// this to heal a frame that was skipped mid-blit).
    Repaint,
}

/// The queue platforms push [`DeviceEvent`]s into. Static-friendly.
pub type DeviceInputChannel = Channel<CriticalSectionRawMutex, DeviceEvent, 32>;

// ── Runner state ─────────────────────────────────────────────────────────────

struct Inner {
    session: DeviceSession,
    ctl: OledController,
    ai_players: [bool; 2],
    demo_mode: bool,
    ai_paused: bool,
    anim: bool,
    next_game_ai: Option<[bool; 2]>,
    skip_dialog: [bool; 2],
    control_modes: [ControlMode; 2],
    parties: [Vec<PartySlotData>; 2],
    /// Pad events produced while the collector loop was not looking
    /// (narration sleeps); drained into it, cleared at each batch start.
    pending_pads: Vec<PadEvent>,
    lobby: bool,
}

pub struct DeviceRunner<'p, P: DevicePort> {
    port: &'p P,
    input: &'p DeviceInputChannel,
    st: RefCell<Inner>,
}

fn idx(player: u8) -> usize {
    (player == 2) as usize
}

/// A typed line that IS a button press: `p1 a`, `p2 l`, `p1 up`, `p1 ah`.
/// Tokens: a, b, i/info, m/menu (the physical MENU key, which maps to B),
/// u/up, d/down, l/left, r/right, ah/hold (the A long-press). Anything
/// else — `p1 ok`, `p1 2`, `p1 s3` — falls through to the lobby/battle
/// grammars untouched. Shared by the fw CLI and the web console.
pub fn parse_button_line(line: &str) -> Option<(u8, Button)> {
    use crate::cursor_nav::Dir;
    let mut it = line.split_whitespace();
    let player = match it.next()? {
        "p1" => 1,
        "p2" => 2,
        _ => return None,
    };
    let button = match it.next()? {
        "a" => Button::A,
        "b" => Button::B,
        "i" | "info" => Button::Info,
        "m" | "menu" => Button::B,
        "u" | "up" => Button::Dpad(Dir::Up),
        "d" | "down" => Button::Dpad(Dir::Down),
        "l" | "left" => Button::Dpad(Dir::Left),
        "r" | "right" => Button::Dpad(Dir::Right),
        "ah" | "hold" => Button::AHold,
        _ => return None,
    };
    if it.next().is_some() {
        return None;
    }
    Some((player, button))
}

/// Rewrite a Line that is really a button press into the Button event, so
/// every consumer downstream sees one input kind.
fn normalize(ev: DeviceEvent) -> DeviceEvent {
    if let DeviceEvent::Line(line) = &ev {
        if let Some((player, button)) = parse_button_line(line) {
            return DeviceEvent::Button { player, button };
        }
    }
    ev
}

fn ready_line(player: u8) -> &'static str {
    if player == 2 { "p2 ok" } else { "p1 ok" }
}

fn cancel_line(player: u8) -> &'static str {
    if player == 2 { "p2 cancel" } else { "p1 cancel" }
}

impl<'p, P: DevicePort> DeviceRunner<'p, P> {
    pub fn new(port: &'p P, input: &'p DeviceInputChannel) -> Self {
        Self {
            port,
            input,
            st: RefCell::new(Inner {
                session: DeviceSession::new(),
                ctl: OledController::new(),
                ai_players: [false; 2],
                demo_mode: false,
                ai_paused: false,
                anim: true,
                next_game_ai: None,
                skip_dialog: [false; 2],
                control_modes: [ControlMode::Normal; 2],
                parties: [Vec::new(), Vec::new()],
                pending_pads: Vec::new(),
                lobby: false,
            }),
        }
    }

    // ── Small shared helpers ────────────────────────────────────────────

    fn present(&self) {
        self.st.borrow_mut().session.battery = self.port.battery();
        let st = self.st.borrow();
        self.port.present(&st.session, &st.ctl, st.lobby);
    }

    /// The one place input is received: typed button lines arrive as the
    /// Button events they mean.
    async fn next_event(&self) -> DeviceEvent {
        normalize(self.input.receive().await)
    }

    fn oled_apply(&self, cmd: OledCmd) {
        // The session lock/unlock bookkeeping that used to live in the web's
        // apply_effects — screen commands and the seat's locked flag must
        // move together or a seat wedges (see the probe war stories).
        match &cmd {
            OledCmd::ShowWaiting { player } | OledCmd::ShowForced { player } => {
                self.st.borrow_mut().session.set_locked(*player, true);
            }
            OledCmd::RestoreScreen { player }
            | OledCmd::ShowSwitchScreen { player }
            | OledCmd::ShowActionSelect { player, .. }
            | OledCmd::ShowConcealedMoves { player, .. }
            | OledCmd::ShowSwitchList { player, .. } => {
                self.st.borrow_mut().session.set_locked(*player, false);
            }
            _ => {}
        }
        self.st.borrow_mut().ctl.apply(cmd);
    }

    fn apply_effects(&self, fx: &mut Vec<CollectEffect>) {
        // No effects, no repaint: the lobby and collector loops call this
        // every 50 ms tick, and presenting unconditionally kept the display
        // pipeline busy full-time (the idle-blit-spam regression).
        if fx.is_empty() {
            return;
        }
        for e in fx.drain(..) {
            match e {
                CollectEffect::Oled(cmd) => self.oled_apply(cmd),
                CollectEffect::Ok(m) => self.port.line_out(&alloc::format!("[OK]  {m}")),
                CollectEffect::Err(m) => self.port.line_out(&alloc::format!("[!!]  {m}")),
                CollectEffect::Dbg(m) => self.port.line_out(&alloc::format!("[>>]  {m}")),
                CollectEffect::Text(t) => {
                    self.port.line_out("");
                    for line in t.lines() {
                        if !line.is_empty() {
                            self.port.line_out(line);
                        }
                    }
                }
            }
        }
        self.present();
    }

    fn is_ai(&self, player: u8) -> bool {
        self.st.borrow().ai_players[idx(player)]
    }

    /// Background events valid in every phase. Returns true if consumed.
    fn background_event(&self, ev: &DeviceEvent) -> bool {
        match ev {
            DeviceEvent::Pulse => {
                // Redraw-gated: an idle lobby must not render 4x a second.
                let redraw = self.st.borrow_mut().ctl.tick_bob(BOB_TICK_MS);
                if redraw.includes(1) || redraw.includes(2) {
                    self.present();
                }
                true
            }
            DeviceEvent::Repaint => {
                self.present();
                true
            }
            DeviceEvent::SetAiPause(v) => {
                self.st.borrow_mut().ai_paused = *v;
                true
            }
            DeviceEvent::SetAnim(v) => {
                self.st.borrow_mut().anim = *v;
                true
            }
            _ => false,
        }
    }

    /// Raw sleep that keeps servicing background events.
    async fn pump_sleep(&self, ms: u32) {
        let mut left = ms;
        while left > 0 {
            let step = left.min(50);
            match select(self.next_event(), self.port.sleep_ms(step)).await {
                Either::First(ev) => {
                    if !self.background_event(&ev) {
                        self.stash_battle_event(ev);
                    }
                }
                Either::Second(()) => left -= step,
            }
        }
    }

    /// Animation-gated, skippable narration sleep — the fw DIALOG_SKIP and
    /// web sleep_ms_skippable, unified. Any seat's press requests a skip;
    /// it lands once every non-AI seat has asked.
    async fn sleep_skippable(&self, ms: u32) {
        if !self.st.borrow().anim {
            return;
        }
        self.st.borrow_mut().skip_dialog = [false, false];
        let mut left = ms;
        while left > 0 {
            let step = left.min(50);
            match select(self.next_event(), self.port.sleep_ms(step)).await {
                Either::First(ev) => {
                    if !self.background_event(&ev) {
                        self.note_skip_request(&ev);
                        self.stash_battle_event(ev);
                    }
                }
                Either::Second(()) => left -= step,
            }
            let done = {
                let st = self.st.borrow();
                (0..2).all(|i| st.skip_dialog[i] || st.ai_players[i])
            };
            if done {
                self.st.borrow_mut().skip_dialog = [false, false];
                return;
            }
        }
    }

    fn note_skip_request(&self, ev: &DeviceEvent) {
        let player = match ev {
            DeviceEvent::Button { player, .. } => *player,
            DeviceEvent::Pad(
                PadEvent::TapMove { player, .. }
                | PadEvent::TapSwitch { player, .. }
                | PadEvent::HoldMove { player, .. }
                | PadEvent::HoldSwitch { player, .. }
                | PadEvent::HoldEnd { player }
                | PadEvent::Chord4 { player }
                | PadEvent::Cancel { player },
            ) => *player,
            DeviceEvent::PanelTap { .. } => 0,
            _ => return,
        };
        let mut st = self.st.borrow_mut();
        if (1..=2).contains(&player) {
            st.skip_dialog[(player - 1) as usize] = true;
        } else {
            st.skip_dialog = [true, true];
        }
    }

    /// Battle-phase input that arrived while nothing was collecting: keep
    /// the meaning (as pad events) for the collector to drain.
    fn stash_battle_event(&self, ev: DeviceEvent) {
        if self.st.borrow().lobby {
            return; // lobby loops consume the queue directly; drop strays
        }
        match ev {
            DeviceEvent::Pad(p) => self.st.borrow_mut().pending_pads.push(p),
            DeviceEvent::Button { player, button } => {
                for pad in self.session_press_to_pads(player, button) {
                    self.st.borrow_mut().pending_pads.push(pad);
                }
            }
            DeviceEvent::PanelTap { x, y } => {
                for pad in self.panel_tap_to_pads(x, y) {
                    self.st.borrow_mut().pending_pads.push(pad);
                }
            }
            _ => {}
        }
    }

    // ── Session press → collector pad events (battle phase) ─────────────

    fn press_ctx(&self, player: u8) -> Ctx {
        let st = self.st.borrow();
        let screen = st.ctl.screen(player);
        Ctx {
            lobby: st.lobby,
            choosing: screen.is_choosing(),
            waiting: matches!(
                screen,
                crate::oled_ctl::Screen::Waiting { .. }
                    | crate::oled_ctl::Screen::WaitingForOpponent { .. }
            ),
        }
    }

    fn session_press_to_pads(&self, player: u8, button: Button) -> Vec<PadEvent> {
        let ctx = self.press_ctx(player);
        let outs = self.st.borrow_mut().session.button(player, button, ctx);
        let pads = self.outs_to_pads(outs);
        self.present();
        pads
    }

    fn panel_tap_to_pads(&self, x: u32, y: u32) -> Vec<PadEvent> {
        let ctx = {
            let st = self.st.borrow();
            let waiting = |p: u8| {
                matches!(
                    st.ctl.screen(p),
                    crate::oled_ctl::Screen::Waiting { .. }
                        | crate::oled_ctl::Screen::WaitingForOpponent { .. }
                )
            };
            TapCtx { lobby: st.lobby, waiting: [waiting(1), waiting(2)] }
        };
        let outs = self.st.borrow_mut().session.panel_tap(x, y, ctx);
        let pads = self.outs_to_pads(outs);
        self.present();
        pads
    }

    fn outs_to_pads(&self, outs: Vec<Out>) -> Vec<PadEvent> {
        let mut pads = Vec::new();
        for out in outs {
            match out {
                Out::Nav(player, nav) => match nav {
                    NavOut::None | NavOut::Redraw => {}
                    NavOut::TapMove(slot) => pads.push(PadEvent::TapMove { player, slot }),
                    NavOut::TapSwitch(idx) => pads.push(PadEvent::TapSwitch { player, idx }),
                    NavOut::HoldMove(slot) => pads.push(PadEvent::HoldMove { player, slot }),
                    NavOut::HoldSwitch(idx) => pads.push(PadEvent::HoldSwitch { player, idx }),
                    NavOut::HoldEnd => pads.push(PadEvent::HoldEnd { player }),
                },
                Out::CancelChoice(player) => pads.push(PadEvent::Cancel { player }),
                // Lobby outs cannot happen here: ctx.lobby is false.
                Out::ReadyLine(_) | Out::CancelLine(_) | Out::LobbyLongPress(_) => {}
            }
        }
        pads
    }

    // ── The game loop ───────────────────────────────────────────────────

    pub async fn run(&self) -> ! {
        loop {
            {
                let mut st = self.st.borrow_mut();
                st.ai_players = [false, false];
                st.ai_paused = false;
                st.lobby = true;
                st.session.set_lobby_flags([false, false], [false, false]);
                st.ctl.apply(OledCmd::LobbyState { player: 1, ready: false, ai: false });
                st.ctl.apply(OledCmd::LobbyState { player: 2, ready: false, ai: false });
            }
            self.present();

            if self.st.borrow().demo_mode {
                // Demo mode: auto-ready after the delay; any input exits.
                self.st.borrow_mut().next_game_ai = Some([true, true]);
                let mut interrupted = false;
                let mut waited = 0u32;
                while waited < LOBBY_DEMO_DELAY_MS as u32 {
                    match select(self.next_event(), self.port.sleep_ms(100)).await {
                        Either::First(ev) => {
                            if !self.background_event(&ev) {
                                interrupted = true;
                                break;
                            }
                        }
                        Either::Second(()) => waited += 100,
                    }
                }
                if interrupted {
                    self.st.borrow_mut().demo_mode = false;
                    continue;
                }
            } else {
                self.port.line_out("═══════════════════════════════════════");
                self.port.line_out("     MEGA BLASTOISE  —  Randbat");
                self.port.line_out("   Both players press a button to start ");
                self.port.line_out("═══════════════════════════════════════");
            }

            // ── Ready sequence: shared state machine, pumped here. ──────
            let (seq_ai, seq_modes, seq_six) = self.run_ready_sequence().await;
            {
                let mut st = self.st.borrow_mut();
                st.lobby = false;
                st.ai_players = seq_ai;
                st.control_modes = seq_modes;
                st.pending_pads.clear();
                st.session.log_reset();
            }
            for player in [1u8, 2] {
                self.oled_apply(OledCmd::SetControlMode {
                    player,
                    concealed: seq_modes[idx(player)] == ControlMode::Concealed,
                });
            }
            self.present();

            // One second between the second READY and the battle.
            self.port.line_out("Both ready!");
            self.pump_sleep(1000).await;

            // Battle-start tutorial, off by default (options menu). The
            // setting says "First game", so it spends itself: it used to
            // replay before every match of the power-on, which is 6.75 s of
            // unskippable pages between rematches.
            if self.st.borrow().session.tutorial() && !(seq_ai[0] && seq_ai[1]) {
                self.st.borrow_mut().session.opts.tutorial = false;
                for page in 0..crate::display::TUTORIAL_PAGES {
                    self.oled_apply(OledCmd::ShowTutorial { page });
                    self.present();
                    self.pump_sleep(2250).await;
                }
            }

            let seed = self.port.seed();
            self.st.borrow_mut().session.begin_battle();
            self.present();

            let ruleset = self.st.borrow().session.ruleset();
            if ruleset == Ruleset::Gen3 {
                self.run_gen3(seed, seq_six).await;
            } else {
                self.run_gen1(seed, seq_six).await;
            }

            self.port.line_out("");
            self.port.line_out("── Battle over — press any button for a new game ───");

            // Post-game QR: any input or 30 s.
            self.oled_apply(OledCmd::ShowQr);
            self.present();
            let mut waited = 0u32;
            while waited < 30_000 {
                match select(self.next_event(), self.port.sleep_ms(100)).await {
                    Either::First(ev) => {
                        if self.background_event(&ev) {
                            continue;
                        }
                        // The recap legend promises "A REMATCH / ? BATTLE
                        // LOG", so honour both: ? opens the log over the
                        // recap (and the D-pad scrolls it), A starts the
                        // next game, and a stray press does neither.
                        match ev {
                            DeviceEvent::Button { player, button } => {
                                let reading = self.st.borrow().session.log_view(player).is_some();
                                if matches!(button, Button::A | Button::TapSeat) && !reading {
                                    break;
                                }
                                let ctx = Ctx { lobby: false, choosing: false, waiting: false };
                                let _ = self.st.borrow_mut().session.button(player, button, ctx);
                                self.present();
                            }
                            DeviceEvent::Pad(_) | DeviceEvent::PanelTap { .. } => break,
                            _ => {}
                        }
                    }
                    Either::Second(()) => waited += 100,
                }
            }
            // Whatever either seat was reading, the next game starts clean.
            self.st.borrow_mut().session.log_reset();
        }
    }

    /// The lobby's ready sequence — press → picker → READY with the 1 s
    /// both-ready grace — pumping the shared [`ReadySequence`].
    async fn run_ready_sequence(&self) -> ([bool; 2], [ControlMode; 2], bool) {
        let mut fx: Vec<CollectEffect> = Vec::new();
        let mut seq = ReadySequence::new(&mut fx);
        if let Some(ai) = self.st.borrow_mut().next_game_ai.take() {
            seq.ai_preset(ai, &mut fx);
        }
        self.apply_effects(&mut fx);
        let mut last_flags = ([false; 2], [false; 2]);
        loop {
            match select(self.next_event(), self.port.sleep_ms(COLLECT_TICK_MS as u32)).await {
                Either::First(ev) => {
                    if !self.background_event(&ev) {
                        self.lobby_event(ev, &mut seq, &mut fx);
                    }
                }
                Either::Second(()) => {}
            }
            let done = seq.tick(self.port.now_ms(), &mut fx);
            let flags = (seq.ready_flags(), seq.ai_flags());
            if flags != last_flags {
                last_flags = flags;
                let (r, a) = flags;
                {
                    let mut st = self.st.borrow_mut();
                    st.session.set_lobby_flags(r, a);
                    st.ctl.apply(OledCmd::LobbyState { player: 1, ready: r[0], ai: a[0] });
                    st.ctl.apply(OledCmd::LobbyState { player: 2, ready: r[1], ai: a[1] });
                }
                self.present();
            }
            self.port.lobby_pulse(flags.0, flags.1);
            self.apply_effects(&mut fx);
            if done {
                break;
            }
        }
        let six = seq.six_v_six() || self.st.borrow().session.six_v_six();
        let (ai, modes) = seq.take();
        (ai, modes, six)
    }

    /// One lobby-phase input into the ready sequence, via the session where
    /// the press has session meaning (menus, B semantics, A-hold).
    fn lobby_event(&self, ev: DeviceEvent, seq: &mut ReadySequence, fx: &mut Vec<CollectEffect>) {
        let now = self.port.now_ms();
        match ev {
            DeviceEvent::Button { player, button } => {
                let ctx = Ctx { lobby: true, choosing: false, waiting: false };
                let outs = self.st.borrow_mut().session.button(player, button, ctx);
                self.lobby_outs(outs, seq, fx);
            }
            DeviceEvent::PanelTap { x, y } => {
                let ctx = TapCtx { lobby: true, waiting: [false, false] };
                let outs = self.st.borrow_mut().session.panel_tap(x, y, ctx);
                self.lobby_outs(outs, seq, fx);
            }
            DeviceEvent::Pad(p) => seq.pad_event(p, now, fx),
            DeviceEvent::Line(line) => seq.typed_line(line.trim(), now, fx),
            DeviceEvent::AiPreset { ai, demo } => {
                if demo {
                    self.st.borrow_mut().demo_mode = true;
                    self.st.borrow_mut().next_game_ai = Some(ai);
                } else {
                    seq.ai_preset(ai, fx);
                }
            }
            DeviceEvent::SetAiPause(_) | DeviceEvent::SetAnim(_) | DeviceEvent::Pulse
            | DeviceEvent::Repaint => {}
        }
        self.present();
    }

    fn lobby_outs(&self, outs: Vec<Out>, seq: &mut ReadySequence, fx: &mut Vec<CollectEffect>) {
        let now = self.port.now_ms();
        for out in outs {
            match out {
                Out::ReadyLine(p) => seq.typed_line(ready_line(p), now, fx),
                Out::CancelLine(p) => seq.typed_line(cancel_line(p), now, fx),
                Out::LobbyLongPress(p) => seq.request_ai_opponent(p, fx),
                Out::Nav(..) | Out::CancelChoice(_) => {}
            }
        }
    }

    /// A Gen 1 battle: always the PS engine (the legacy Gen 1 engine is gone).
    async fn run_gen1(&self, seed: u64, six: bool) {
        self.run_ps(1, seed, six).await;
    }

    async fn run_gen3(&self, seed: u64, six: bool) {
        if self.port.engine() == Engine::Ps {
            // Gen 3 moves to the PS engine at its cut-over (G3-8,
            // `runner::GEN3_ON_PS`), and only with every move ported (a team
            // would lose the moves that are not); the legacy engine plays
            // until then.
            if runner::GEN3_ON_PS && runner::ps_ports_every_move(3) {
                self.run_ps(3, seed, six).await;
                return;
            }
            self.port.line_out("Gen 3: the PS engine is not switched on for Gen 3 yet; using the legacy engine.");
        }
        let Some(mut battle) = runner::gen3::drafted_battle(seed, six) else {
            self.port.line_out("Gen 3: could not draft teams.");
            return;
        };
        for (label, side) in [("White", &battle.sides[0]), ("Red", &battle.sides[1])] {
            let names: Vec<&str> = side.party.iter().map(|m| m.species.name).collect();
            self.port.line_out(&alloc::format!("{label}: {}", names.join(", ")));
        }
        let bus = InputBus::new();
        let mut effects = RunnerEffects { r: self, bus: &bus };
        runner::gen3::run_battle(
            &mut battle,
            &bus,
            self.collect_battle_input(&bus, seed),
            &mut effects,
        )
        .await;
    }

    /// A battle on the PS engine ([`runner::ps`]): `ps_sim` in-process
    /// over the typed [`LocalTransport`], read by `ps_client`. The input
    /// pump and the effects sink are the legacy runners' own.
    async fn run_ps(&self, gen: u8, seed: u64, six: bool) {
        let dex: &'static ps_dex::Dex = if gen == 1 { &ps_dex::GEN1 } else { &ps_dex::GEN3 };
        let teams = crate::randbat::draw_two_ps_teams(dex, seed, if six { 6 } else { 3 });
        if teams.iter().any(|t| t.is_empty()) {
            self.port.line_out("PS engine: could not draft teams.");
            return;
        }
        for (label, team) in [("White", &teams[0]), ("Red", &teams[1])] {
            let names: Vec<&str> = team.iter().map(|s| s.name.as_str()).collect();
            self.port.line_out(&alloc::format!("{label}: {}", names.join(", ")));
        }
        self.port.line_out("── Battle start (PS engine) ────────────");
        let setup = runner::ps::BattleSetup::local(gen, seed, teams);
        let mut transport = LocalTransport::new();
        let bus = InputBus::new();
        let mut effects = RunnerEffects { r: self, bus: &bus };
        runner::ps::run_battle(
            &mut transport,
            &setup,
            &bus,
            self.collect_battle_input(&bus, seed),
            &mut effects,
        )
        .await;
    }

    /// The battle input pump: prompt batches into the shared
    /// [`ChoiceCollector`], session presses translated to pad events, AI
    /// choices injected — the exact loop both platforms used to duplicate.
    async fn collect_battle_input(&self, bus: &InputBus, seed: u64) {
        let mut ai = RandomAi::new(seed ^ 0xbad_c0ffee_dead);
        self.st.borrow_mut().pending_pads.clear();

        loop {
            // Gather the whole prompt batch, narrating engine events.
            let first = loop {
                match select3(bus.prompt.receive(), bus.log.receive(), self.next_event()).await {
                    Either3::First(p) => {
                        while let Ok(line) = bus.log.try_receive() {
                            self.port.line_out(&alloc::format!("[EVT] {line}"));
                        }
                        break p;
                    }
                    Either3::Second(line) => self.port.line_out(&alloc::format!("[EVT] {line}")),
                    Either3::Third(ev) => {
                        if !self.background_event(&ev) {
                            self.note_skip_request(&ev);
                            self.stash_battle_event(ev);
                        }
                    }
                }
            };
            let batch_total = first.batch_total.max(1);
            let mut prompts: Vec<ActivePrompt> = Vec::with_capacity(batch_total);
            prompts.push(first);
            while prompts.len() < batch_total {
                match select(bus.prompt.receive(), bus.log.receive()).await {
                    Either::First(p) => prompts.push(p),
                    Either::Second(line) => self.port.line_out(&alloc::format!("[EVT] {line}")),
                }
            }

            // Party snapshots for platform side channels (LED strips).
            for p in &prompts {
                if !p.slot.party.is_empty() {
                    let player = player_id_to_num(p.player_id.as_str());
                    self.st.borrow_mut().parties[idx(player)] = p.slot.party.clone();
                }
            }

            // The AI-pause gate (web debug affordance).
            let any_ai =
                prompts.iter().any(|p| self.is_ai(player_id_to_num(p.player_id.as_str())));
            while any_ai && self.st.borrow().ai_paused {
                self.pump_sleep(100).await;
            }

            let now = self.port.now_ms();
            let mut batch: Vec<SlotOptions> = Vec::with_capacity(prompts.len());
            for p in &prompts {
                let mut slot = p.slot.clone();
                let player = player_id_to_num(p.player_id.as_str());
                if self.is_ai(player) {
                    let pick = slot.random_choice(ai.rng_mut());
                    slot.set_ai_choice(pick);
                    self.st.borrow_mut().session.set_locked(player, true);
                } else {
                    if self.st.borrow().control_modes[idx(player)] == ControlMode::Concealed {
                        slot.set_concealed(now ^ ((player as u64) << 33));
                    }
                    let n_moves = (slot.n_moves as u8).max(1);
                    let n_party = (slot.party.len().min(6) as u8).max(1);
                    self.st.borrow_mut().session.begin_turn(
                        player,
                        n_moves.min(4),
                        n_party,
                        slot.forced_switch,
                    );
                }
                batch.push(slot);
            }
            // Presses made between turns are stale; typed lines survive on
            // both platforms, pads do not.
            self.st.borrow_mut().pending_pads.clear();
            self.present();

            let mut fx: Vec<CollectEffect> = Vec::new();
            let mut col = ChoiceCollector::new(batch, &mut fx);
            self.apply_effects(&mut fx);

            loop {
                // Anything stashed during narration goes in first.
                let stashed: Vec<PadEvent> =
                    core::mem::take(&mut self.st.borrow_mut().pending_pads);
                for pad in stashed {
                    col.pad_event(pad, self.port.now_ms(), &mut fx);
                }
                match select3(
                    self.next_event(),
                    bus.log.receive(),
                    self.port.sleep_ms(COLLECT_TICK_MS as u32),
                )
                .await
                {
                    Either3::First(ev) => {
                        if !self.background_event(&ev) {
                            match ev {
                                DeviceEvent::Pad(p) => {
                                    col.pad_event(p, self.port.now_ms(), &mut fx)
                                }
                                DeviceEvent::Line(line) => {
                                    col.typed_line(line.trim(), self.port.now_ms(), &mut fx)
                                }
                                DeviceEvent::Button { player, button } => {
                                    for pad in self.session_press_to_pads(player, button) {
                                        col.pad_event(pad, self.port.now_ms(), &mut fx);
                                    }
                                }
                                DeviceEvent::PanelTap { x, y } => {
                                    for pad in self.panel_tap_to_pads(x, y) {
                                        col.pad_event(pad, self.port.now_ms(), &mut fx);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    Either3::Second(line) => {
                        self.port.line_out(&alloc::format!("[EVT] {line}"))
                    }
                    Either3::Third(()) => {}
                }
                let done = col.tick(self.port.now_ms(), &mut fx);
                self.apply_effects(&mut fx);
                if done {
                    break;
                }
            }

            for (player_id, choice) in col.take_choices() {
                let choice = if choice.is_empty() { String::from("pass") } else { choice };
                bus.choices.send(PlayerChoice { player_id, choice }).await;
            }
        }
    }
}

// ── The shared effects sink ──────────────────────────────────────────────────

/// Display consequences and narration pacing for battle events — the halves
/// of the old per-platform sinks that were never platform-specific. LED-ish
/// side channels go through [`DevicePort::side_event`].
struct RunnerEffects<'a, 'p, P: DevicePort> {
    r: &'a DeviceRunner<'p, P>,
    bus: &'a InputBus,
}

impl<P: DevicePort> BoardEffects for RunnerEffects<'_, '_, P> {
    async fn on_event(&mut self, event: BoardEvent) {
        for cmd in oled_cmds_for_event(&event) {
            self.r.oled_apply(cmd);
        }

        // Session-side battle bookkeeping that used to be web glue.
        match &event {
            BoardEvent::Faint { mon, .. } => {
                if let Some(p) = crate::mon_player_num(mon) {
                    let mut st = self.r.st.borrow_mut();
                    let name = crate::mon_display_name(mon);
                    if let Some(slot) =
                        st.parties[idx(p)].iter_mut().find(|s| s.name == name)
                    {
                        slot.hp = 0;
                    }
                }
            }
            BoardEvent::SetStatus { mon, status } => {
                if let Some(p) = crate::mon_player_num(mon) {
                    let mut st = self.r.st.borrow_mut();
                    let name = crate::mon_display_name(mon);
                    if let Some(slot) =
                        st.parties[idx(p)].iter_mut().find(|s| s.name == name)
                    {
                        slot.status = Some(status.clone());
                    }
                }
            }
            BoardEvent::CureStatus { mon, .. } => {
                if let Some(p) = crate::mon_player_num(mon) {
                    let mut st = self.r.st.borrow_mut();
                    let name = crate::mon_display_name(mon);
                    if let Some(slot) =
                        st.parties[idx(p)].iter_mut().find(|s| s.name == name)
                    {
                        slot.status = None;
                    }
                }
            }
            _ => {}
        }

        {
            let st = self.r.st.borrow();
            self.r.port.side_event(&event, &st.parties);
        }

        // Narration into the session log, then the paced hold.
        let line = event.description();
        self.r.st.borrow_mut().session.log_line(&line);
        self.r.present();
        let delay_ms = self.r.st.borrow().session.text_scale(event.anim_delay_ms());
        if delay_ms > 0 {
            self.r.sleep_skippable(delay_ms).await;
        }

        if event.should_narrate() {
            let _ = self.bus.log.try_send(event.description());
        }
    }
}
