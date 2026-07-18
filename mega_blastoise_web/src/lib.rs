mod web_display;
mod web_effects;

use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use gen1_battle::TeamData;
use js_sys::Date;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

use embassy_futures::select::{select, select3, Either, Either3};
use mega_blastoise_core::{
    battle_options_with_seed, demo_engine_opts, draw_two_randbat_teams, format_active_state,
    party_slot_from_mon, render_screen, run_battle, ActivePrompt, BoardEventQueue,
    ChoiceCollector, CollectEffect, ControlMode, FlashDataStore, InputBus,
    OledCmd, OledController, PadEvent, PartySlotData, PlayerChoice, RandomAi, ReadySequence, SlotOptions,
    BATTLE_HELP, COLLECT_TICK_MS, LOBBY_DEMO_DELAY_MS,
};
use web_effects::WebBattleEffects;
use web_display::WasmDisplay;

// ── Global state ──────────────────────────────────────────────────────────────

thread_local! {
    static P1_PIXELS: RefCell<Vec<u8>> = RefCell::new(vec![10, 25, 10, 255].repeat(128 * 64));
    static P2_PIXELS: RefCell<Vec<u8>> = RefCell::new(vec![10, 25, 10, 255].repeat(128 * 64));
    static LED_STATE: RefCell<[u32; 24]> = RefCell::new([0u32; 24]);

    // Per-player button queues — both players can pre-queue independently
    static P1_QUEUE: RefCell<VecDeque<ButtonEvent>> = RefCell::new(VecDeque::new());
    static P1_WAKER: RefCell<Option<Waker>> = RefCell::new(None);
    static P2_QUEUE: RefCell<VecDeque<ButtonEvent>> = RefCell::new(VecDeque::new());
    static P2_WAKER: RefCell<Option<Waker>> = RefCell::new(None);

    // Battle-input queue: classified pad events and typed lines, consumed by
    // the shared ChoiceCollector loop (mirrors the firmware's USB+matrix IO).
    static BATTLE_INPUT: RefCell<VecDeque<BattleInput>> = RefCell::new(VecDeque::new());
    static BATTLE_INPUT_WAKER: RefCell<Option<Waker>> = RefCell::new(None);

    // Sticky holds (web only): a PC mouse can't hold one button while
    // clicking another, so a hold LATCHES — it stays "held down" until the
    // button is clicked again or the player commits an option. Entries are
    // (is_switch, idx), innermost last; releases are swallowed while latched.
    static HELD_LATCH: RefCell<[Vec<(bool, u8)>; 2]> = RefCell::new([Vec::new(), Vec::new()]);

    // Unlatching an OUTER button while an inner one is still latched defers
    // its HoldEnd until the inner unlatches (the screen must stay on the
    // inner view until then).
    static PENDING_END: RefCell<[u8; 2]> = RefCell::new([0; 2]);

    // Control scheme each player chose this battle (drives the web-only
    // "action buttons open instantly on click" behavior for Concealed).
    static CONTROL_MODES: RefCell<[ControlMode; 2]> = RefCell::new([ControlMode::Normal; 2]);

    // Web stand-in for the hidden 4-corner chord: a mouse can't press four
    // buttons at once, so tapping all four corners within 2s counts.
    // [player][slot] = last tap time (ms).
    static CHORD_TAPS: RefCell<[[u64; 4]; 2]> = RefCell::new([[0; 4]; 2]);

    // Lobby LED animation mode
    static LOBBY_MODE: RefCell<bool> = RefCell::new(false);

    // Ready flags mirrored from the ready sequence for the lobby LED frame.
    static LOBBY_READY: RefCell<[bool; 2]> = RefCell::new([false, false]);

    // Flash events: [p1_type, p2_type]; 1 = super-effective, 2 = crit; consumed on read
    static FLASH: RefCell<[u8; 2]> = RefCell::new([0, 0]);

    // The shared two-display state machine (mega_blastoise_core::oled_ctl) —
    // identical to the firmware's. All screen decisions happen in here;
    // this file only repaints canvases when it says so. See oled_apply().
    static OLED_CTL: RefCell<OledController> = RefCell::new(OledController::new());

    // Full party snapshot per player (for AI switch picks + party LED sync;
    // the OLED controller keeps its own copy via OledCmd::PartyUpdate)
    static P1_PARTY: RefCell<Vec<PartySlotData>> = RefCell::new(Vec::new());
    static P2_PARTY: RefCell<Vec<PartySlotData>> = RefCell::new(Vec::new());

    // Which players are AI-controlled this game (reset each lobby)
    static AI_PLAYERS: RefCell<[bool; 2]> = RefCell::new([false, false]);

    // Demo mode: both AI, auto-restart each game (persists until page reload)
    static DEMO_MODE: RefCell<bool> = RefCell::new(false);

    // AI pause: when true, AI players block in wait_action/wait_switch
    static AI_PAUSED: RefCell<bool> = RefCell::new(false);

    // Pending AI config for the next game: set by VS AI button, survives the
    // loop-top reset of AI_PLAYERS, applied just before the battle starts.
    static NEXT_GAME_AI: RefCell<Option<[bool; 2]>> = RefCell::new(None);

    // When false, all animation sleeps are skipped (useful for CLI testing).
    static ANIM_ENABLED: RefCell<bool> = RefCell::new(true);

    // A button press during battle dialog skips the current animation delay
    // (consumed at the start of each delay so stale presses don't skip).
    static SKIP_DIALOG: RefCell<bool> = RefCell::new(false);

}

// ── State accessors (pub(crate)) ──────────────────────────────────────────────

/// Apply a display command to the shared controller and repaint whichever
/// canvases it changed. The ONLY write path to the OLED canvases — mirrors
/// the firmware's OLED task loop (apply → render `ctl.screen(p)` → flush).
pub(crate) fn oled_apply(cmd: OledCmd) {
    let redraw = OLED_CTL.with(|c| c.borrow_mut().apply(cmd));
    for player in [1u8, 2] {
        if redraw.includes(player) {
            let mut disp = WasmDisplay::new();
            OLED_CTL.with(|c| render_screen(&mut disp, &c.borrow().screen(player)));
            let pixels = disp.to_rgba();
            if player == 1 { P1_PIXELS.with(|p| *p.borrow_mut() = pixels); }
            else           { P2_PIXELS.with(|p| *p.borrow_mut() = pixels); }
        }
    }
}

/// Advance the battle-screen sprite bobs — called every BOB_TICK_MS from JS,
/// mirroring the firmware's OLED-task tick. Each player's bob rate scales
/// with their active mon's Speed stat.
#[wasm_bindgen]
pub fn wasm_tick_bob() {
    let redraw = OLED_CTL.with(|c| c.borrow_mut().tick_bob(mega_blastoise_core::BOB_TICK_MS));
    for player in [1u8, 2] {
        if redraw.includes(player) {
            let mut disp = WasmDisplay::new();
            OLED_CTL.with(|c| render_screen(&mut disp, &c.borrow().screen(player)));
            let pixels = disp.to_rgba();
            if player == 1 { P1_PIXELS.with(|p| *p.borrow_mut() = pixels); }
            else           { P2_PIXELS.with(|p| *p.borrow_mut() = pixels); }
        }
    }
}

pub(crate) fn is_ai_paused() -> bool {
    AI_PAUSED.with(|p| *p.borrow())
}

pub(crate) fn is_ai_player(player: u8) -> bool {
    AI_PLAYERS.with(|a| a.borrow()[(player - 1) as usize])
}

pub(crate) fn ai_pick_move(n_moves: usize) -> usize {
    (js_sys::Math::random() * n_moves as f64) as usize
}

pub(crate) fn ai_pick_switch(player: u8) -> usize {
    let party = if player == 1 { P1_PARTY.with(|p| p.borrow().clone()) }
                else           { P2_PARTY.with(|p| p.borrow().clone()) };
    // Try slots 1, 2, 0 — prefer bench slots over active (slot 0 is usually active)
    for &idx in &[1usize, 2, 0] {
        if let Some(slot) = party.get(idx) {
            if slot.hp > 0 { return idx; }
        }
    }
    0
}

pub(crate) fn clear_input_queues() {
    P1_QUEUE.with(|q| q.borrow_mut().clear());
    P2_QUEUE.with(|q| q.borrow_mut().clear());
}

/// Returns true if the party slot at `idx` is alive (hp > 0) or data is unavailable.
pub(crate) fn party_slot_alive(player: u8, idx: usize) -> bool {
    let party = if player == 1 { P1_PARTY.with(|p| p.borrow().clone()) }
                else           { P2_PARTY.with(|p| p.borrow().clone()) };
    party.get(idx).map(|s| s.hp > 0).unwrap_or(true)
}


pub(crate) fn update_leds(leds: [u32; 24]) {
    LED_STATE.with(|l| *l.borrow_mut() = leds);
}

fn status_led_color(status: Option<&str>) -> u32 {
    fn rgb(r: u8, g: u8, b: u8) -> u32 { ((r as u32) << 16) | ((g as u32) << 8) | b as u32 }
    match status {
        None | Some("") => rgb(0,   160, 0),
        Some("par")     => rgb(220, 180, 0),
        Some("brn")     => rgb(220, 20,  0),
        Some("frz")     => rgb(60,  80,  255),
        Some("psn")     => rgb(180, 60,  220),
        Some("tox")     => rgb(80,  0,   100),
        Some("slp")     => rgb(220, 80,  80),
        _               => rgb(0,   160, 0),
    }
}

/// Update HP bar (indices 0-7) for one player only.
/// Party LEDs (8-10 / 20-22) are managed separately by sync_party_leds.
pub(crate) fn update_hp_leds(player: u8, frame: [u32; 8]) {
    LED_STATE.with(|l| {
        let mut leds = l.borrow_mut();
        let base = if player == 1 { 0 } else { 12 };
        for i in 0..8 { leds[base + i] = frame[i]; }
    });
}

/// Rewrite party LEDs (3 slots per player) from P_PARTY status data.
/// Each slot is colored by status (or green if healthy, off if fainted).
/// The old dedicated status LED position (11/23) is zeroed out.
pub(crate) fn sync_party_leds(player: u8) {
    let party = if player == 1 { P1_PARTY.with(|p| p.borrow().clone()) }
                else           { P2_PARTY.with(|p| p.borrow().clone()) };
    LED_STATE.with(|l| {
        let mut leds = l.borrow_mut();
        let base = if player == 1 { 8 } else { 20 };
        let green = status_led_color(None);
        for i in 0..3usize {
            // P2's board is oriented in reverse, so mirror the slot→LED mapping.
            let party_idx = if player == 2 { 2 - i } else { i };
            leds[base + i] = if party.is_empty() {
                green // party not yet populated — default to all healthy
            } else {
                match party.get(party_idx) {
                    Some(slot) if slot.hp > 0 => status_led_color(slot.status.as_deref()),
                    _ => 0,
                }
            };
        }
        leds[base + 3] = 0; // old status-LED position — always off
    });
}

/// Patch a party slot's status in P_PARTY without a full replace.
/// Call after SetStatus/CureStatus so sync_party_leds sees the new value.
pub(crate) fn update_party_slot_status(player: u8, mon_name: &str, status: Option<String>) {
    if player == 1 {
        P1_PARTY.with(|p| {
            if let Some(slot) = p.borrow_mut().iter_mut().find(|s| s.name == mon_name) {
                slot.status = status;
            }
        });
    } else {
        P2_PARTY.with(|p| {
            if let Some(slot) = p.borrow_mut().iter_mut().find(|s| s.name == mon_name) {
                slot.status = status;
            }
        });
    }
}

/// Patch a party slot's HP in P_PARTY (used on Faint to set hp=0).
pub(crate) fn update_party_slot_hp(player: u8, mon_name: &str, hp: u16) {
    if player == 1 {
        P1_PARTY.with(|p| {
            if let Some(slot) = p.borrow_mut().iter_mut().find(|s| s.name == mon_name) {
                slot.hp = hp;
            }
        });
    } else {
        P2_PARTY.with(|p| {
            if let Some(slot) = p.borrow_mut().iter_mut().find(|s| s.name == mon_name) {
                slot.hp = hp;
            }
        });
    }
}

pub(crate) fn set_lobby_mode(active: bool) {
    LOBBY_MODE.with(|m| *m.borrow_mut() = active);
}

pub(crate) fn pack_rgb(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

/// Sleep unconditionally — the collector tick and countdown clocks must run
/// even with `:anim off` (sleep_ms below is animation-gated).
pub(crate) async fn sleep_ms_raw(ms: u32) {
    let promise = js_sys::Promise::new(&mut |resolve: js_sys::Function, _| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms as i32)
            .unwrap();
    });
    wasm_bindgen_futures::JsFuture::from(promise).await.ok();
}

pub(crate) async fn sleep_ms(ms: u32) {
    if !ANIM_ENABLED.with(|a| *a.borrow()) { return; }
    let promise = js_sys::Promise::new(&mut |resolve: js_sys::Function, _| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms as i32)
            .unwrap();
    });
    wasm_bindgen_futures::JsFuture::from(promise).await.ok();
}

pub(crate) fn request_dialog_skip() {
    SKIP_DIALOG.with(|s| *s.borrow_mut() = true);
}

/// Animation-gated sleep that any button press cuts short (mirrors the
/// firmware's DIALOG_SKIP): polls in 50 ms slices, consuming the skip flag
/// at the start so only presses made DURING the dialog skip it.
pub(crate) async fn sleep_ms_skippable(ms: u32) {
    if !ANIM_ENABLED.with(|a| *a.borrow()) { return; }
    SKIP_DIALOG.with(|s| *s.borrow_mut() = false);
    let mut left = ms;
    while left > 0 {
        let step = left.min(50);
        sleep_ms_raw(step).await;
        if SKIP_DIALOG.with(|s| core::mem::take(&mut *s.borrow_mut())) {
            return;
        }
        left -= step;
    }
}

pub(crate) fn set_flash(player: u8, flash_type: u8) {
    FLASH.with(|f| f.borrow_mut()[(player - 1) as usize] = flash_type);
}

fn lobby_led_frame() -> [u32; 24] {
    let ready = LOBBY_READY.with(|r| *r.borrow());
    let t = (Date::now() as u64 / 30) as u8;
    let v = (if t < 128 { t / 2 } else { (255u8.wrapping_sub(t)) / 2 }) as u32 * 2;
    // Per-player identity colors, mirroring the fw lobby: P1 white, P2 red.
    let breathe_p1 = pack_rgb(v as u8, v as u8, v as u8);
    let breathe_p2 = pack_rgb(v as u8, 0, 0);
    let done = pack_rgb(0, 200, 50);
    let mut frame = [0u32; 24];
    let c1 = if ready[0] { done } else { breathe_p1 };
    let c2 = if ready[1] { done } else { breathe_p2 };
    for i in 0..12  { frame[i] = c1; }
    for i in 12..24 { frame[i] = c2; }
    frame
}

/// `?` / `:help` — same layout as the firmware's device help; the battle
/// section is the shared BATTLE_HELP so the grammars can't drift.
fn print_help() {
    print_log("[help] Web commands");
    print_log("  Lobby:");
    for l in [
        ":ready            both players ready (human)",
        ":ready p1|p2      one player ready (human)",
        ":ready ai         P1 is AI (play as P2; also the RED VS AI button)",
        ":demo             AI vs AI demo (also the DEMO button)",
    ] {
        print_log(&format!("    {l}"));
    }
    print_log("  Any time:");
    for l in [
        ":help / :h / ?    this list",
        ":anim on|off      battle animations",
        ":reset            reload (new battle)",
    ] {
        print_log(&format!("    {l}"));
    }
    print_log("  In battle:");
    for l in BATTLE_HELP {
        print_log(&format!("    {l}"));
    }
}

pub(crate) fn print_log(line: &str) {
    let doc = match web_sys::window().and_then(|w| w.document()) {
        Some(d) => d,
        None => return,
    };
    if let Some(el) = doc.get_element_by_id("log") {
        el.insert_adjacent_text("beforeend", &format!("{line}\n")).ok();
        el.set_scroll_top(el.scroll_height());
    }
}

// ── Button events ─────────────────────────────────────────────────────────────

pub enum ButtonEvent {
    Move   { player: u8, slot: u8 },
    Switch { player: u8, idx:  u8 },
    /// Lobby long-press: `player` wants an AI opponent.
    LongPress { player: u8 },
    /// HIDDEN: the 4-corner chord (web: all four corners tapped within 2s).
    Chord { player: u8 },
    /// A demo / VS-AI button set NEXT_GAME_AI; the lobby loop applies it.
    AiPreset,
    /// A typed lobby line for the ready sequence (picker grammar, sims).
    Line(String),
}

// Wait for either player's button (lobby start)
pub(crate) struct AnyButtonFuture;

impl Future for AnyButtonFuture {
    type Output = ButtonEvent;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<ButtonEvent> {
        let ev = P1_QUEUE.with(|q| q.borrow_mut().pop_front())
            .or_else(|| P2_QUEUE.with(|q| q.borrow_mut().pop_front()));
        if let Some(ev) = ev {
            Poll::Ready(ev)
        } else {
            P1_WAKER.with(|w| *w.borrow_mut() = Some(cx.waker().clone()));
            P2_WAKER.with(|w| *w.borrow_mut() = Some(cx.waker().clone()));
            Poll::Pending
        }
    }
}

// Wait for a specific player's button
pub(crate) struct PlayerButtonFuture(pub u8);

impl Future for PlayerButtonFuture {
    type Output = ButtonEvent;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<ButtonEvent> {
        let player = self.0;
        let ev = if player == 1 {
            P1_QUEUE.with(|q| q.borrow_mut().pop_front())
        } else {
            P2_QUEUE.with(|q| q.borrow_mut().pop_front())
        };
        if let Some(ev) = ev {
            Poll::Ready(ev)
        } else {
            if player == 1 {
                P1_WAKER.with(|w| *w.borrow_mut() = Some(cx.waker().clone()));
            } else {
                P2_WAKER.with(|w| *w.borrow_mut() = Some(cx.waker().clone()));
            }
            Poll::Pending
        }
    }
}

/// One unit of battle input from the page: a classified pad event or a line
/// typed into the terminal.
enum BattleInput {
    Pad(PadEvent),
    Line(String),
}

fn push_battle_input(inp: BattleInput) {
    BATTLE_INPUT.with(|q| q.borrow_mut().push_back(inp));
    BATTLE_INPUT_WAKER.with(|w| {
        if let Some(waker) = w.borrow_mut().take() {
            waker.wake();
        }
    });
}

/// Resolves with the next battle input (pad event or typed line).
struct BattleInputFuture;

impl Future for BattleInputFuture {
    type Output = BattleInput;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<BattleInput> {
        if let Some(inp) = BATTLE_INPUT.with(|q| q.borrow_mut().pop_front()) {
            Poll::Ready(inp)
        } else {
            BATTLE_INPUT_WAKER.with(|w| *w.borrow_mut() = Some(cx.waker().clone()));
            Poll::Pending
        }
    }
}

fn push_button(ev: ButtonEvent) {
    let player = match &ev {
        ButtonEvent::Move   { player, .. }
        | ButtonEvent::Switch { player, .. }
        | ButtonEvent::LongPress { player }
        | ButtonEvent::Chord { player } => *player,
        ButtonEvent::AiPreset | ButtonEvent::Line(_) => 1,
    };
    if player == 1 {
        P1_QUEUE.with(|q| q.borrow_mut().push_back(ev));
        P1_WAKER.with(|w| { if let Some(waker) = w.borrow_mut().take() { waker.wake(); } });
    } else {
        P2_QUEUE.with(|q| q.borrow_mut().push_back(ev));
        P2_WAKER.with(|w| { if let Some(waker) = w.borrow_mut().take() { waker.wake(); } });
    }
}

// ── Sticky hold latch (web only) ──────────────────────────────────────────────

fn latch_i(player: u8) -> usize {
    (player.clamp(1, 2) - 1) as usize
}

/// If `(is_switch, idx)` is latched, unlatch it and emit what it owes:
/// the INNERMOST latch emits its HoldEnd immediately (plus any deferred
/// outer ends); an outer latch under a still-latched inner defers its
/// HoldEnd until the inner unlatches, so the inner view stays up.
/// Returns true if the button was latched.
fn unlatch_if_held(player: u8, is_switch: bool, idx: u8) -> bool {
    let i = latch_i(player);
    let (was_latched, ends) = HELD_LATCH.with(|l| {
        let v = &mut l.borrow_mut()[i];
        match v.iter().position(|&e| e == (is_switch, idx)) {
            None => (false, 0u8),
            Some(pos) if pos + 1 == v.len() => {
                v.remove(pos);
                let deferred = PENDING_END.with(|p| core::mem::take(&mut p.borrow_mut()[i]));
                (true, 1 + deferred)
            }
            Some(pos) => {
                v.remove(pos);
                PENDING_END.with(|p| p.borrow_mut()[i] += 1);
                (true, 0)
            }
        }
    });
    for _ in 0..ends {
        push_battle_input(BattleInput::Pad(PadEvent::HoldEnd { player }));
    }
    was_latched
}

fn latch_held(player: u8, is_switch: bool, idx: u8) {
    HELD_LATCH.with(|l| l.borrow_mut()[latch_i(player)].push((is_switch, idx)));
}

fn any_latched(player: u8) -> bool {
    HELD_LATCH.with(|l| !l.borrow()[latch_i(player)].is_empty())
}

pub(crate) fn clear_hold_latch(player: u8) {
    HELD_LATCH.with(|l| l.borrow_mut()[latch_i(player)].clear());
    PENDING_END.with(|p| p.borrow_mut()[latch_i(player)] = 0);
}

pub(crate) fn clear_hold_latches() {
    clear_hold_latch(1);
    clear_hold_latch(2);
}

/// Latched buttons per player, for the page to render as "held" — bit 0-3 =
/// move buttons, bit 4-6 = party buttons.
#[wasm_bindgen]
pub fn wasm_held_buttons(player: u8) -> u8 {
    HELD_LATCH.with(|l| {
        l.borrow()[latch_i(player)]
            .iter()
            .fold(0u8, |m, &(sw, idx)| m | 1 << (if sw { 4 + idx } else { idx }))
    })
}

// ── WASM exports ──────────────────────────────────────────────────────────────

/// Track lobby corner taps; true when this tap completes the 4-corner set
/// within the 2s window (fires the hidden chord).
fn chord_tap(player: u8, slot: u8) -> bool {
    if slot > 3 {
        return false;
    }
    let now = now_ms();
    CHORD_TAPS.with(|t| {
        let taps = &mut t.borrow_mut()[latch_i(player)];
        taps[slot as usize] = now;
        if taps.iter().all(|&t0| now.saturating_sub(t0) <= 2000) {
            *taps = [0; 4];
            true
        } else {
            false
        }
    })
}

#[wasm_bindgen] pub fn press_move(player: u8, slot: u8) {
    if LOBBY_MODE.with(|m| *m.borrow()) {
        if chord_tap(player, slot) {
            push_button(ButtonEvent::Chord { player });
        } else {
            push_button(ButtonEvent::Move { player, slot });
        }
    } else if unlatch_if_held(player, false, slot) {
        // Clicking a latched button releases the sticky hold.
    } else {
        request_dialog_skip();
        push_battle_input(BattleInput::Pad(PadEvent::TapMove { player, slot }));
    }
}

#[wasm_bindgen] pub fn press_switch(player: u8, idx: u8) {
    if LOBBY_MODE.with(|m| *m.borrow()) {
        push_button(ButtonEvent::Switch { player, idx });
    } else if unlatch_if_held(player, true, idx) {
        // Clicking a latched button releases the sticky hold.
    } else {
        // (Concealed menus toggle on plain taps now — no hold conversion.)
        request_dialog_skip();
        push_battle_input(BattleInput::Pad(PadEvent::TapSwitch { player, idx }));
    }
}

/// A move button crossed the 500 ms hold threshold (battle only — the lobby
/// long-press goes through wasm_lobby_long_press). On the web the hold
/// LATCHES: the pointer-up release is swallowed (see [`hold_end`]) and the
/// button stays held until clicked again or an option is committed.
#[wasm_bindgen] pub fn hold_move(player: u8, slot: u8) {
    if unlatch_if_held(player, false, slot) {
        return;
    }
    latch_held(player, false, slot);
    push_battle_input(BattleInput::Pad(PadEvent::HoldMove { player, slot }));
}

/// A party button crossed the 500 ms hold threshold.
#[wasm_bindgen] pub fn hold_switch(player: u8, idx: u8) {
    if unlatch_if_held(player, true, idx) {
        return;
    }
    latch_held(player, true, idx);
    push_battle_input(BattleInput::Pad(PadEvent::HoldSwitch { player, idx }));
}

/// The held button was released. Swallowed while a sticky latch is active —
/// on the web, letting go of the mouse doesn't end a hold.
#[wasm_bindgen] pub fn hold_end(player: u8) {
    if any_latched(player) {
        return;
    }
    push_battle_input(BattleInput::Pad(PadEvent::HoldEnd { player }));
}

fn enter_demo_mode() {
    DEMO_MODE.with(|d| *d.borrow_mut() = true);
    NEXT_GAME_AI.with(|n| *n.borrow_mut() = Some([true, true]));
    push_button(ButtonEvent::AiPreset);
}

#[wasm_bindgen] pub fn wasm_enter_demo_mode() {
    if !LOBBY_MODE.with(|m| *m.borrow()) { return; }
    enter_demo_mode();
}

#[wasm_bindgen] pub fn wasm_reset() {
    if let Some(win) = web_sys::window() {
        win.location().reload().ok();
    }
}

#[wasm_bindgen] pub fn wasm_toggle_ai_pause() -> bool {
    let new_state = AI_PAUSED.with(|p| { let v = !*p.borrow(); *p.borrow_mut() = v; v });
    new_state
}

#[wasm_bindgen] pub fn wasm_enter_vs_ai_mode() {
    if !LOBBY_MODE.with(|m| *m.borrow()) { return; }
    // p1=AI (White), p2=human (Red): p2 still picks their controls.
    NEXT_GAME_AI.with(|n| *n.borrow_mut() = Some([true, false]));
    push_button(ButtonEvent::AiPreset);
}

#[wasm_bindgen] pub fn is_lobby_mode() -> bool {
    LOBBY_MODE.with(|m| *m.borrow())
}

/// Long-press lobby handler: `player` pressed long → their opponent becomes
/// AI-controlled (ready) and the presser proceeds to the controls picker.
#[wasm_bindgen] pub fn wasm_lobby_long_press(player: u8) {
    if !LOBBY_MODE.with(|m| *m.borrow()) { return; }
    push_button(ButtonEvent::LongPress { player });
}

#[wasm_bindgen] pub fn submit_text(line: String) {
    let cmd = line.trim();

    // Global commands always available
    match cmd {
        ":reset" | ":restart" => { wasm_reset(); return; }
        ":anim off" => { ANIM_ENABLED.with(|a| *a.borrow_mut() = false); print_log("[anim] animations OFF"); return; }
        ":anim on"  => { ANIM_ENABLED.with(|a| *a.borrow_mut() = true);  print_log("[anim] animations ON");  return; }
        "?" | ":help" | ":h" => { print_help(); return; }
        _ => {}
    }

    if LOBBY_MODE.with(|m| *m.borrow()) {
        match cmd {
            ":ready ai" | ":vs ai" | ":red vs ai" | ":blue vs ai" => {
                // VS AI: P1=AI (White), P2=human (Red) — P2 still picks controls.
                NEXT_GAME_AI.with(|n| *n.borrow_mut() = Some([true, false]));
                push_button(ButtonEvent::AiPreset);
            }
            ":demo" => { enter_demo_mode(); }
            // Ready commands skip the picker: current highlight (Normal by
            // default; `p1 concealed` first to change it).
            ":ready" | ":ready both" => {
                push_button(ButtonEvent::Line(String::from("p1 ok")));
                push_button(ButtonEvent::Line(String::from("p2 ok")));
            }
            ":ready p1" => push_button(ButtonEvent::Line(String::from("p1 ok"))),
            ":ready p2" => push_button(ButtonEvent::Line(String::from("p2 ok"))),
            // Anything else goes to the ready sequence: picker grammar
            // (pN normal|concealed|ok) and :press/:hold/:release sims.
            other => push_button(ButtonEvent::Line(String::from(other))),
        }
        return;
    }

    // In-game: the line goes to the shared collector — identical grammar and
    // feedback to the firmware's USB CLI ("p1 2", "p2 s3", bare when only one
    // player is choosing; typing for a committed player unreadies them).
    push_battle_input(BattleInput::Line(String::from(cmd)));
}

#[wasm_bindgen] pub fn get_p1_pixels() -> Vec<u8> {
    P1_PIXELS.with(|p| p.borrow().clone())
}

#[wasm_bindgen] pub fn get_p2_pixels() -> Vec<u8> {
    P2_PIXELS.with(|p| p.borrow().clone())
}

#[wasm_bindgen] pub fn get_led_state() -> Vec<u32> {
    if LOBBY_MODE.with(|m| *m.borrow()) {
        return lobby_led_frame().to_vec();
    }
    LED_STATE.with(|l| l.borrow().to_vec())
}

#[wasm_bindgen] pub fn get_flash_state() -> Vec<u8> {
    FLASH.with(|f| {
        let state = f.borrow().to_vec();
        *f.borrow_mut() = [0, 0];
        state
    })
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    spawn_local(run_game_loop());
}

// ── OLED helpers ──────────────────────────────────────────────────────────────

fn draw_lobby_screen(player: u8, ready: bool, ai: bool) {
    oled_apply(OledCmd::LobbyState { player, ready, ai });
}

fn set_lobby_displays() {
    draw_lobby_screen(1, false, false);
    draw_lobby_screen(2, false, false);
    // LEDs driven by lobby_led_frame() while LOBBY_MODE is active
}

// ── Game loop ─────────────────────────────────────────────────────────────────

async fn run_game_loop() {
    let data = FlashDataStore::new();

    loop {
        AI_PLAYERS.with(|a| *a.borrow_mut() = [false, false]);
        LOBBY_READY.with(|r| *r.borrow_mut() = [false, false]);
        AI_PAUSED.with(|p| *p.borrow_mut() = false);
        set_lobby_displays();
        set_lobby_mode(true);

        if DEMO_MODE.with(|d| *d.borrow()) {
            // Demo mode: auto-ready lobby with 15 s pause; exit if a button is pressed.
            // Set the AI config for this iteration via NEXT_GAME_AI like any other caller.
            NEXT_GAME_AI.with(|n| *n.borrow_mut() = Some([true, true]));
            let mut interrupted = false;
            for _ in 0..(LOBBY_DEMO_DELAY_MS / 100) as u16 {
                sleep_ms(100).await;
                let pressed = P1_QUEUE.with(|q| !q.borrow().is_empty())
                           || P2_QUEUE.with(|q| !q.borrow().is_empty());
                if pressed { interrupted = true; break; }
            }
            if interrupted {
                // Exit demo mode. Don't clear queues — the queued events (including
                // any VS AI ready signals) will be consumed naturally by the normal
                // lobby on the next iteration. NEXT_GAME_AI carries any VS AI intent
                // forward; AI_PLAYERS is reset at the top of the next iteration.
                DEMO_MODE.with(|d| *d.borrow_mut() = false);
                continue;
            }
        } else {
            print_log("═══════════════════════════════════════");
            print_log("     MEGA BLASTOISE  —  Gen 1 Randbat  ");
            print_log("   Both players press a button to start ");
            print_log("═══════════════════════════════════════");
            print_log(&format!(
                "  build {} · {}",
                env!("GIT_HASH"),
                env!("BUILD_DATETIME"),
            ));
            print_log("");

        }

        // ── Ready sequence: press → controls picker → READY, per player,
        //    with a 1s both-ready grace. Shared state machine with the fw. ────
        let (seq_ai, seq_modes, seq_six) = {
            let mut fx: Vec<CollectEffect> = Vec::new();
            let mut seq = ReadySequence::new(&mut fx);
            // AI intent from the demo / VS-AI buttons (or a pending preset).
            if let Some(ai) = NEXT_GAME_AI.with(|n| n.borrow_mut().take()) {
                seq.ai_preset(ai, &mut fx);
            }
            apply_effects(&mut fx);
            loop {
                match select(AnyButtonFuture, sleep_ms_raw(COLLECT_TICK_MS as u32)).await {
                    Either::First(ButtonEvent::Move { player, slot }) => {
                        seq.pad_event(PadEvent::TapMove { player, slot }, now_ms(), &mut fx)
                    }
                    Either::First(ButtonEvent::Switch { player, idx }) => {
                        seq.pad_event(PadEvent::TapSwitch { player, idx }, now_ms(), &mut fx)
                    }
                    Either::First(ButtonEvent::LongPress { player }) => {
                        seq.request_ai_opponent(player, &mut fx)
                    }
                    Either::First(ButtonEvent::Chord { player }) => {
                        seq.pad_event(PadEvent::Chord4 { player }, now_ms(), &mut fx)
                    }
                    Either::First(ButtonEvent::AiPreset) => {
                        if let Some(ai) = NEXT_GAME_AI.with(|n| n.borrow_mut().take()) {
                            seq.ai_preset(ai, &mut fx);
                        }
                    }
                    Either::First(ButtonEvent::Line(line)) => seq.typed_line(line.trim(), now_ms(), &mut fx),
                    Either::Second(()) => {}
                }
                let done = seq.tick(now_ms(), &mut fx);
                LOBBY_READY.with(|r| *r.borrow_mut() = seq.ready_flags());
                apply_effects(&mut fx);
                if done {
                    break;
                }
            }
            let six = seq.six_v_six();
            let (ai, modes) = seq.take();
            (ai, modes, six)
        };
        set_lobby_mode(false);
        AI_PLAYERS.with(|a| *a.borrow_mut() = seq_ai);
        let modes = seq_modes;
        // Drain any button presses that accumulated during the lobby phase
        // so they don't get consumed as the first battle move.
        P1_QUEUE.with(|q| q.borrow_mut().clear());
        P2_QUEUE.with(|q| q.borrow_mut().clear());
        // (No detail-overlay reset needed: the shared controller redraws over
        // any leftover overlay on the first battle state command.)

        // Controls were chosen during the ready sequence.
        BATTLE_INPUT.with(|q| q.borrow_mut().clear());
        clear_hold_latches();
        CONTROL_MODES.with(|m| *m.borrow_mut() = modes);
        // Tell the shared display controller each player's scheme (concealed
        // battle screens hide the move list) — mirrors the firmware.
        for player in [1u8, 2] {
            oled_apply(OledCmd::SetControlMode {
                player,
                concealed: modes[latch_i(player)] == ControlMode::Concealed,
            });
        }

        // Countdown — same text and 500 ms cadence as the firmware lobby,
        // with a gold LED flash per tick as the web's buzzer stand-in.
        print_log("Both ready!");
        let gold = pack_rgb(200, 150, 0);
        for i in (1u8..=3).rev() {
            print_log(&format!("{}...", i));
            update_leds([gold; 24]);
            sleep_ms_raw(250).await;
            update_leds([0u32; 24]);
            sleep_ms_raw(250).await;
        }
        print_log("GO!");

        // Battle-start tutorial, 3 pages, 2.25 s each, not skippable.
        // Real games with a human only — never before demo / AI-vs-AI games.
        if !(seq_ai[0] && seq_ai[1]) {
            for page in 0..mega_blastoise_core::display::TUTORIAL_PAGES {
                oled_apply(OledCmd::ShowTutorial { page });
                sleep_ms_raw(2250).await;
            }
        }

        let seed = (Date::now() as u64) ^ 0xdead_beef_cafe_babe;

        let mut battle = match gen1_battle::PublicCoreBattle::new(
            battle_options_with_seed(seed),
            &data,
            demo_engine_opts(),
        ) {
            Ok(b) => b,
            Err(e) => { print_log(&format!("Battle init error: {e}")); continue; }
        };

        let (team_red, team_blue) =
            draw_two_randbat_teams(seed, if seq_six { 6 } else { 3 });

        let ok = battle.update_team("p1", TeamData { members: team_red,  ..Default::default() }).is_ok()
               && battle.update_team("p2", TeamData { members: team_blue, ..Default::default() }).is_ok()
               && battle.start().is_ok();
        if !ok { print_log("Battle setup failed."); continue; }

        print_log("── Battle start ────────────────────────");
        print_log("");

        let bus = InputBus::new();
        let mut queue = BoardEventQueue::new();
        let mut effects = WebBattleEffects::new(&bus);

        run_battle(
            &mut battle,
            &data,
            &bus,
            collect_battle_input(&bus, seed, modes),
            &mut queue,
            &mut effects,
            |b| {
                let state = format_active_state(b);
                for line in state.lines() { print_log(line); }
            },
        )
        .await;

        print_log("");
        print_log("── Battle over — press any button for a new game ───");
        print_log("");

        // Post-game feedback QR on both displays: stays up until any button
        // press or 30 seconds, whichever comes first (mirrors the firmware).
        BATTLE_INPUT.with(|q| q.borrow_mut().clear());
        oled_apply(OledCmd::ShowQr);
        let mut waited_ms = 0u32;
        while waited_ms < 30_000 {
            match select(BattleInputFuture, sleep_ms_raw(100)).await {
                Either::First(_) => break,
                Either::Second(()) => waited_ms += 100,
            }
        }
    }
}

// ── Battle input collection ───────────────────────────────────────────────────
//
// The exact mirror of the firmware's USB battle loop: gather the prompt
// batch, hand everything to core's ChoiceCollector (where ALL semantics
// live), and shuttle raw IO — pad events, typed lines, the tick clock, and
// the collector's effects.

fn now_ms() -> u64 {
    Date::now() as u64
}

fn apply_effects(fx: &mut Vec<CollectEffect>) {
    for e in fx.drain(..) {
        match e {
            CollectEffect::Oled(cmd) => {
                // Committing an option (or landing on a screen where nothing
                // is held) releases any sticky web holds.
                match &cmd {
                    OledCmd::ShowWaiting { player }
                    | OledCmd::ShowActionSelect { player, .. }
                    | OledCmd::ShowConcealedMoves { player, .. }
                    | OledCmd::ShowSwitchList { player, .. }
                    | OledCmd::ShowOpponentMon { player }
                    | OledCmd::ShowControlsSelect { player, .. } => clear_hold_latch(*player),
                    _ => {}
                }
                oled_apply(cmd)
            }
            CollectEffect::Ok(m) => print_log(&format!("[OK]  {m}")),
            CollectEffect::Err(m) => print_log(&format!("[!!]  {m}")),
            CollectEffect::Dbg(m) => print_log(&format!("[>>]  {m}")),
            CollectEffect::Text(t) => {
                print_log("");
                for line in t.lines() {
                    if !line.is_empty() {
                        print_log(line);
                    }
                }
            }
        }
    }
}

async fn collect_battle_input(bus: &InputBus, seed: u64, modes: [ControlMode; 2]) {
    let mut ai = RandomAi::new(seed ^ 0xbad_c0ffee_dead);
    // Nothing from a previous battle is input for this one.
    BATTLE_INPUT.with(|q| q.borrow_mut().clear());
    clear_hold_latches();

    loop {
        // ── Gather the whole prompt batch, narrating engine events while
        //    we wait (same "[EVT] " framing as the firmware CLI). ──────────
        let first = loop {
            match select(bus.prompt.receive(), bus.log.receive()).await {
                Either::First(p) => {
                    while let Ok(line) = bus.log.try_receive() {
                        print_log(&format!("[EVT] {line}"));
                    }
                    break p;
                }
                Either::Second(line) => print_log(&format!("[EVT] {line}")),
            }
        };
        let batch_total = first.batch_total.max(1);
        let mut prompts: Vec<ActivePrompt> = Vec::with_capacity(batch_total);
        prompts.push(first);
        while prompts.len() < batch_total {
            match select(bus.prompt.receive(), bus.log.receive()).await {
                Either::First(p) => prompts.push(p),
                Either::Second(line) => print_log(&format!("[EVT] {line}")),
            }
        }

        // Web-only side state: party snapshots for the LED strip.
        for p in &prompts {
            if let Some(pd) = &p.player_data {
                let player = mega_blastoise_core::player_id_to_num(p.player_id.as_str());
                let slots: Vec<PartySlotData> = pd.mons.iter().map(party_slot_from_mon).collect();
                if player == 1 { P1_PARTY.with(|s| *s.borrow_mut() = slots.clone()); }
                else           { P2_PARTY.with(|s| *s.borrow_mut() = slots.clone()); }
                sync_party_leds(player);
            }
        }

        // PAUSE gates AI turns (web-only debug affordance).
        let any_ai = prompts.iter().any(|p| {
            is_ai_player(mega_blastoise_core::player_id_to_num(p.player_id.as_str()))
        });
        while any_ai && is_ai_paused() {
            sleep_ms_raw(100).await;
        }

        let mut batch: Vec<SlotOptions> = Vec::with_capacity(prompts.len());
        for p in &prompts {
            let mut slot = SlotOptions::from_prompt(p);
            let player = mega_blastoise_core::player_id_to_num(p.player_id.as_str());
            if is_ai_player(player) {
                slot.set_ai_choice(ai.make_choice(&p.request, p.player_data.as_ref()));
            } else if modes[latch_i(player)] == ControlMode::Concealed {
                // Fresh randomized layouts every combat turn.
                slot.set_concealed(now_ms() ^ ((player as u64) << 33));
            }
            batch.push(slot);
        }
        // Pad presses made between turns are dropped, exactly like the
        // firmware (its matrix scan only listens while collecting); typed
        // lines buffer across the gap on both platforms.
        BATTLE_INPUT.with(|q| q.borrow_mut().retain(|i| matches!(i, BattleInput::Line(_))));

        let mut fx: Vec<CollectEffect> = Vec::new();
        let mut col = ChoiceCollector::new(batch, &mut fx);
        apply_effects(&mut fx);

        loop {
            match select3(
                BattleInputFuture,
                bus.log.receive(),
                sleep_ms_raw(COLLECT_TICK_MS as u32),
            )
            .await
            {
                Either3::First(BattleInput::Pad(ev)) => col.pad_event(ev, now_ms(), &mut fx),
                Either3::First(BattleInput::Line(line)) => col.typed_line(line.trim(), now_ms(), &mut fx),
                Either3::Second(line) => print_log(&format!("[EVT] {line}")),
                Either3::Third(()) => {}
            }
            let done = col.tick(now_ms(), &mut fx);
            apply_effects(&mut fx);
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
