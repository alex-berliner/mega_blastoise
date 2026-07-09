mod web_controller;
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

use mega_blastoise_core::{
    battle_options_with_seed, demo_engine_opts, draw_two_randbat_teams, format_active_state,
    parse_web_game_cmd, render_screen, run_battle, BoardEventQueue, ButtonController,
    FlashDataStore, InputBus, OledCmd, OledController, PartySlotData, WebGameInput,
    LOBBY_DEMO_DELAY_MS,
};

use web_controller::WebButtonSource;
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

    // Lobby LED animation mode
    static LOBBY_MODE: RefCell<bool> = RefCell::new(false);

    // Per-player lobby ready state
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

    // Set whenever update_pixels() writes new battle content (not overlays).
    // Consumed by consume_battle_transitions() so JS can cancel held long-press cycles.
    static BATTLE_DIRTY: RefCell<[bool; 2]> = RefCell::new([false, false]);

}

// ── State accessors (pub(crate)) ──────────────────────────────────────────────

/// Apply a display command to the shared controller and repaint whichever
/// canvases it changed. The ONLY write path to the OLED canvases — mirrors
/// the firmware's OLED task loop (apply → render `ctl.screen(p)` → flush).
pub(crate) fn oled_apply(cmd: OledCmd) {
    // Battle-state screens (not transient overlays) mark BATTLE_DIRTY so JS
    // can cancel a held long-press cycle when the screen transitions away.
    let dirty = matches!(
        cmd,
        OledCmd::HpUpdate { .. }
            | OledCmd::ActiveMon { .. }
            | OledCmd::MovesUpdate { .. }
            | OledCmd::Faint { .. }
            | OledCmd::Win { .. }
            | OledCmd::LobbyState { .. }
            | OledCmd::ShowSwitchScreen { .. }
    );
    let redraw = OLED_CTL.with(|c| c.borrow_mut().apply(cmd));
    for player in [1u8, 2] {
        if redraw.includes(player) {
            let mut disp = WasmDisplay::new();
            OLED_CTL.with(|c| render_screen(&mut disp, &c.borrow().screen(player)));
            let pixels = disp.to_rgba();
            if dirty {
                BATTLE_DIRTY.with(|d| d.borrow_mut()[(player - 1) as usize] = true);
            }
            if player == 1 { P1_PIXELS.with(|p| *p.borrow_mut() = pixels); }
            else           { P2_PIXELS.with(|p| *p.borrow_mut() = pixels); }
        }
    }
}

/// Advance the battle-screen sprite bob — called every 900 ms from JS,
/// mirroring the firmware's OLED-task tick.
#[wasm_bindgen]
pub fn wasm_tick_bob() {
    let redraw = OLED_CTL.with(|c| c.borrow_mut().tick_bob());
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

pub(crate) fn update_party(player: u8, slots: Vec<PartySlotData>) {
    if player == 1 { P1_PARTY.with(|p| *p.borrow_mut() = slots.clone()); }
    else           { P2_PARTY.with(|p| *p.borrow_mut() = slots.clone()); }
    oled_apply(OledCmd::PartyUpdate { player, slots });
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

pub(crate) fn show_pokemon_stats(player: u8, team_idx: usize, page: u8) {
    oled_apply(OledCmd::ShowPokemonStats { player, team_idx: team_idx as u8, page });
}

pub(crate) fn show_move_detail(player: u8, slot: usize) {
    oled_apply(OledCmd::ShowMoveDetail { player, slot: slot as u8 });
}

pub(crate) fn show_switch_screen(player: u8) {
    oled_apply(OledCmd::ShowSwitchScreen { player });
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

pub(crate) fn show_invalid_selection(player: u8) {
    oled_apply(OledCmd::ShowInvalidSelection { player });
}

/// Pop one button event from the player's queue without blocking.
pub(crate) fn pop_player_button(player: u8) -> Option<ButtonEvent> {
    if player == 1 {
        P1_QUEUE.with(|q| q.borrow_mut().pop_front())
    } else {
        P2_QUEUE.with(|q| q.borrow_mut().pop_front())
    }
}

pub(crate) fn show_waiting_screen(player: u8) {
    oled_apply(OledCmd::ShowWaiting { player });
}

pub(crate) fn show_waiting_for_opponent_screen(player: u8) {
    oled_apply(OledCmd::ShowWaitingForOpponent { player });
}

pub(crate) fn restore_screen(player: u8) {
    oled_apply(OledCmd::RestoreScreen { player });
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

pub(crate) fn set_flash(player: u8, flash_type: u8) {
    FLASH.with(|f| f.borrow_mut()[(player - 1) as usize] = flash_type);
}

fn lobby_led_frame() -> [u32; 24] {
    let ready = LOBBY_READY.with(|r| *r.borrow());
    let t = (Date::now() as u64 / 30) as u8;
    let v = (if t < 128 { t / 2 } else { (255u8.wrapping_sub(t)) / 2 }) as u32 * 2;
    let breathe = ((v / 3) << 16) | v;
    let done = pack_rgb(0, 200, 50);
    let mut frame = [0u32; 24];
    let c1 = if ready[0] { done } else { breathe };
    let c2 = if ready[1] { done } else { breathe };
    for i in 0..12  { frame[i] = c1; }
    for i in 12..24 { frame[i] = c2; }
    frame
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

fn push_button(ev: ButtonEvent) {
    let player = match &ev {
        ButtonEvent::Move   { player, .. }
        | ButtonEvent::Switch { player, .. } => *player,
    };
    if player == 1 {
        P1_QUEUE.with(|q| q.borrow_mut().push_back(ev));
        P1_WAKER.with(|w| { if let Some(waker) = w.borrow_mut().take() { waker.wake(); } });
    } else {
        P2_QUEUE.with(|q| q.borrow_mut().push_back(ev));
        P2_WAKER.with(|w| { if let Some(waker) = w.borrow_mut().take() { waker.wake(); } });
    }
}

// ── WASM exports ──────────────────────────────────────────────────────────────

#[wasm_bindgen] pub fn press_move(player: u8, slot: u8) {
    push_button(ButtonEvent::Move { player, slot });
}

#[wasm_bindgen] pub fn press_switch(player: u8, idx: u8) {
    push_button(ButtonEvent::Switch { player, idx });
}

#[wasm_bindgen] pub fn wasm_show_move_detail(player: u8, slot: u8) {
    show_move_detail(player, slot as usize);
}

#[wasm_bindgen] pub fn wasm_show_pokemon_stats(player: u8, team_idx: u8) {
    show_pokemon_stats(player, team_idx as usize, 0);
}

#[wasm_bindgen] pub fn wasm_show_pokemon_stats_page(player: u8, team_idx: u8, page: u8) {
    show_pokemon_stats(player, team_idx as usize, page);
}

#[wasm_bindgen] pub fn wasm_restore_screen(player: u8) {
    restore_screen(player);
}

fn enter_demo_mode() {
    DEMO_MODE.with(|d| *d.borrow_mut() = true);
    NEXT_GAME_AI.with(|n| *n.borrow_mut() = Some([true, true]));
    LOBBY_READY.with(|r| *r.borrow_mut() = [false, false]);
    push_button(ButtonEvent::Move { player: 1, slot: 0 });
    push_button(ButtonEvent::Move { player: 2, slot: 0 });
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
    // p1=AI (Red), p2=human (Blue)
    NEXT_GAME_AI.with(|n| *n.borrow_mut() = Some([true, false]));
    LOBBY_READY.with(|r| *r.borrow_mut() = [false, false]);
    push_button(ButtonEvent::Move { player: 1, slot: 0 });
    push_button(ButtonEvent::Move { player: 2, slot: 0 });
}

#[wasm_bindgen] pub fn is_lobby_mode() -> bool {
    LOBBY_MODE.with(|m| *m.borrow())
}

/// Long-press lobby handler: `player` pressed long → their opponent is AI-controlled.
/// Immediately draws the correct lobby screens and queues both players as ready.
#[wasm_bindgen] pub fn wasm_lobby_long_press(player: u8) {
    if !LOBBY_MODE.with(|m| *m.borrow()) { return; }
    // player is human, opponent is AI
    let ai = if player == 1 { [false, true] } else { [true, false] };
    NEXT_GAME_AI.with(|n| *n.borrow_mut() = Some(ai));
    LOBBY_READY.with(|r| *r.borrow_mut() = [false, false]);
    draw_lobby_screen(1, true, ai[0]);
    draw_lobby_screen(2, true, ai[1]);
    push_button(ButtonEvent::Move { player: 1, slot: 0 });
    push_button(ButtonEvent::Move { player: 2, slot: 0 });
}

#[wasm_bindgen] pub fn submit_text(line: String) {
    let cmd = line.trim();

    // Global commands always available
    match cmd {
        ":reset" | ":restart" => { wasm_reset(); return; }
        ":anim off" => { ANIM_ENABLED.with(|a| *a.borrow_mut() = false); print_log("[anim] animations OFF"); return; }
        ":anim on"  => { ANIM_ENABLED.with(|a| *a.borrow_mut() = true);  print_log("[anim] animations ON");  return; }
        _ => {}
    }

    if LOBBY_MODE.with(|m| *m.borrow()) {
        match cmd {
            ":ready ai" | ":vs ai" | ":blue vs ai" => {
                // VS AI: P1=AI (Red), P2=human (Blue)
                NEXT_GAME_AI.with(|n| *n.borrow_mut() = Some([true, false]));
                LOBBY_READY.with(|r| *r.borrow_mut() = [false, false]);
                push_button(ButtonEvent::Move { player: 1, slot: 0 });
                push_button(ButtonEvent::Move { player: 2, slot: 0 });
            }
            ":demo" => { enter_demo_mode(); }
            ":ready" | ":ready both" => {
                LOBBY_READY.with(|r| *r.borrow_mut() = [false, false]);
                push_button(ButtonEvent::Move { player: 1, slot: 0 });
                push_button(ButtonEvent::Move { player: 2, slot: 0 });
            }
            ":ready p1" => push_button(ButtonEvent::Move { player: 1, slot: 0 }),
            ":ready p2" => push_button(ButtonEvent::Move { player: 2, slot: 0 }),
            "?" | ":help" | ":h" => print_log("  lobby: :ready | :ready p1 | :ready p2 | :ready ai | :demo | :reset"),
            _ => print_log("  unknown command — type ? for help"),
        }
        return;
    }

    // In-game: parse move/switch commands.
    match parse_web_game_cmd(cmd) {
        WebGameInput::Move { player, slot } => push_button(ButtonEvent::Move { player, slot }),
        WebGameInput::Switch { player, idx }  => push_button(ButtonEvent::Switch { player, idx }),
        WebGameInput::Unknown => print_log("  in-game: 1-4 (move) | s1-s3 (switch) | p1:1 (send to Red)"),
    }
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

/// Returns a bitmask of players whose battle OLED content changed this frame:
/// bit 0 = P1, bit 1 = P2. Clears the flags on read. JS uses this to cancel
/// any active long-press cycle when the screen transitions away.
#[wasm_bindgen] pub fn consume_battle_transitions() -> u8 {
    BATTLE_DIRTY.with(|d| {
        let mut flags = d.borrow_mut();
        let bits = (flags[0] as u8) | ((flags[1] as u8) << 1);
        *flags = [false, false];
        bits
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

fn print_line(line: &str) {
    print_log(line);
}

// ── Game loop ─────────────────────────────────────────────────────────────────

async fn run_game_loop() {
    let data = FlashDataStore::new();

    loop {
        LOBBY_READY.with(|r| *r.borrow_mut() = [false, false]);
        AI_PLAYERS.with(|a| *a.borrow_mut() = [false, false]);
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

            // Wait for both players to press a button (or one to long-press for AI opponent).
            // Pressing while already ready unreadies the player and cancels any AI config.
            loop {
                let ev = AnyButtonFuture.await;
                let player = match &ev {
                    ButtonEvent::Move   { player, .. }
                    | ButtonEvent::Switch { player, .. } => *player,
                };
                let already_ready = LOBBY_READY.with(|r| r.borrow()[(player - 1) as usize]);
                if already_ready {
                    LOBBY_READY.with(|r| r.borrow_mut()[(player - 1) as usize] = false);
                    NEXT_GAME_AI.with(|n| *n.borrow_mut() = None);
                    draw_lobby_screen(player, false, false);
                } else {
                    LOBBY_READY.with(|r| r.borrow_mut()[(player - 1) as usize] = true);
                    let is_ai = NEXT_GAME_AI.with(|n| {
                        n.borrow().map(|a| a[(player - 1) as usize]).unwrap_or(false)
                    });
                    draw_lobby_screen(player, true, is_ai);
                }
                if LOBBY_READY.with(|r| { let rr = r.borrow(); rr[0] && rr[1] }) { break; }
            }
        }
        set_lobby_mode(false);
        // Apply any VS AI mode that was requested during the lobby.
        if let Some(ai) = NEXT_GAME_AI.with(|n| n.borrow_mut().take()) {
            AI_PLAYERS.with(|a| *a.borrow_mut() = ai);
        }
        // Drain any button presses that accumulated during the lobby phase
        // so they don't get consumed as the first battle move.
        P1_QUEUE.with(|q| q.borrow_mut().clear());
        P2_QUEUE.with(|q| q.borrow_mut().clear());
        // (No detail-overlay reset needed: the shared controller redraws over
        // any leftover overlay on the first battle state command.)

        // Countdown fanfare: 3 gold flashes
        let gold = pack_rgb(200, 150, 0);
        for _ in 0..3u8 {
            update_leds([gold; 24]);
            sleep_ms(200).await;
            update_leds([0u32; 24]);
            sleep_ms(100).await;
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

        let (team_red, team_blue) = draw_two_randbat_teams(seed, 3);

        let ok = battle.update_team("p1", TeamData { members: team_red,  ..Default::default() }).is_ok()
               && battle.update_team("p2", TeamData { members: team_blue, ..Default::default() }).is_ok()
               && battle.start().is_ok();
        if !ok { print_log("Battle setup failed."); continue; }

        print_log("── Battle start ────────────────────────");
        print_log("");

        let bus = InputBus::new();
        let mut queue = BoardEventQueue::new();
        let mut effects = WebBattleEffects::new(&bus);
        let mut controller = ButtonController::with_log_sink(WebButtonSource, print_line);

        run_battle(
            &mut battle,
            &data,
            &bus,
            controller.run_parallel(&bus),
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
    }
}
