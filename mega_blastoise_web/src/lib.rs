mod web_controller;
mod web_display;
mod web_effects;

use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use battler::TeamData;
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Alignment, Baseline, Text, TextStyleBuilder},
};
use js_sys::Date;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

use mega_blastoise_core::{
    battle_options_with_seed, demo_engine_opts, draw_randbat_team, format_active_state,
    render_move_detail, render_pokemon_stats, render_pokemon_stats_page2, render_switch_screen,
    run_battle, BoardEventQueue, ButtonController, FlashDataStore, InputBus, InputSource,
    MoveSlot, PartySlotData,
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

    // Move detail: last-rendered battle pixels (for restore after long press)
    static P1_BATTLE_PIXELS: RefCell<Vec<u8>> = RefCell::new(vec![10, 25, 10, 255].repeat(128 * 64));
    static P2_BATTLE_PIXELS: RefCell<Vec<u8>> = RefCell::new(vec![10, 25, 10, 255].repeat(128 * 64));

    // Current active move list per player (for long-press detail rendering)
    static P1_MOVES: RefCell<Vec<MoveSlot>> = RefCell::new(Vec::new());
    static P2_MOVES: RefCell<Vec<MoveSlot>> = RefCell::new(Vec::new());

    // Full party snapshot per player (for long-press switch button rendering)
    static P1_PARTY: RefCell<Vec<PartySlotData>> = RefCell::new(Vec::new());
    static P2_PARTY: RefCell<Vec<PartySlotData>> = RefCell::new(Vec::new());

    // True while a detail/stats overlay is displayed — suppresses update_pixels writes
    static P1_IN_DETAIL: RefCell<bool> = RefCell::new(false);
    static P2_IN_DETAIL: RefCell<bool> = RefCell::new(false);

    // Active mon name per player (updated on SwitchIn; shown on waiting screen)
    static P1_MON_NAME: RefCell<String> = RefCell::new(String::new());
    static P2_MON_NAME: RefCell<String> = RefCell::new(String::new());

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

}

// ── State accessors (pub(crate)) ──────────────────────────────────────────────

/// Write pixels to the display canvas only — does NOT update P_BATTLE_PIXELS.
/// Used by flash animations so the long-press restore source stays clean.
pub(crate) fn display_only(player: u8, pixels: Vec<u8>) {
    if player == 1 {
        if !P1_IN_DETAIL.with(|d| *d.borrow()) {
            P1_PIXELS.with(|p| *p.borrow_mut() = pixels);
        }
    } else {
        if !P2_IN_DETAIL.with(|d| *d.borrow()) {
            P2_PIXELS.with(|p| *p.borrow_mut() = pixels);
        }
    }
}

pub(crate) fn update_pixels(player: u8, pixels: Vec<u8>) {
    if player == 1 {
        P1_BATTLE_PIXELS.with(|p| *p.borrow_mut() = pixels.clone());
        // Don't overwrite the detail overlay — restore_screen will pick up
        // the updated battle pixels when the player releases.
        if !P1_IN_DETAIL.with(|d| *d.borrow()) {
            P1_PIXELS.with(|p| *p.borrow_mut() = pixels);
        }
    } else {
        P2_BATTLE_PIXELS.with(|p| *p.borrow_mut() = pixels.clone());
        if !P2_IN_DETAIL.with(|d| *d.borrow()) {
            P2_PIXELS.with(|p| *p.borrow_mut() = pixels);
        }
    }
}

pub(crate) fn set_active_mon_name(player: u8, name: &str) {
    if player == 1 { P1_MON_NAME.with(|n| *n.borrow_mut() = name.to_string()); }
    else           { P2_MON_NAME.with(|n| *n.borrow_mut() = name.to_string()); }
}

fn get_active_mon_name(player: u8) -> String {
    if player == 1 { P1_MON_NAME.with(|n| n.borrow().clone()) }
    else           { P2_MON_NAME.with(|n| n.borrow().clone()) }
}

pub(crate) fn update_moves(player: u8, moves: Vec<MoveSlot>) {
    if player == 1 { P1_MOVES.with(|m| *m.borrow_mut() = moves); }
    else           { P2_MOVES.with(|m| *m.borrow_mut() = moves); }
}

pub(crate) fn update_party(player: u8, slots: Vec<PartySlotData>) {
    if player == 1 { P1_PARTY.with(|p| *p.borrow_mut() = slots); }
    else           { P2_PARTY.with(|p| *p.borrow_mut() = slots); }
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
    let party = if player == 1 { P1_PARTY.with(|p| p.borrow().clone()) }
                else           { P2_PARTY.with(|p| p.borrow().clone()) };
    if let Some(slot) = party.get(team_idx) {
        let mut disp = WasmDisplay::new();
        if page == 1 {
            render_pokemon_stats_page2(&mut disp, slot);
        } else {
            render_pokemon_stats(&mut disp, slot);
        }
        let pixels = disp.to_rgba();
        if player == 1 {
            P1_IN_DETAIL.with(|d| *d.borrow_mut() = true);
            P1_PIXELS.with(|p| *p.borrow_mut() = pixels);
        } else {
            P2_IN_DETAIL.with(|d| *d.borrow_mut() = true);
            P2_PIXELS.with(|p| *p.borrow_mut() = pixels);
        }
    }
}

pub(crate) fn show_move_detail(player: u8, slot: usize) {
    let moves = if player == 1 { P1_MOVES.with(|m| m.borrow().clone()) }
                else           { P2_MOVES.with(|m| m.borrow().clone()) };
    if let Some(mv) = moves.get(slot) {
        let mut disp = WasmDisplay::new();
        render_move_detail(&mut disp, mv);
        let pixels = disp.to_rgba();
        if player == 1 {
            P1_IN_DETAIL.with(|d| *d.borrow_mut() = true);
            P1_PIXELS.with(|p| *p.borrow_mut() = pixels);
        } else {
            P2_IN_DETAIL.with(|d| *d.borrow_mut() = true);
            P2_PIXELS.with(|p| *p.borrow_mut() = pixels);
        }
    }
}

pub(crate) fn show_switch_screen(player: u8) {
    let party = if player == 1 { P1_PARTY.with(|p| p.borrow().clone()) }
                else           { P2_PARTY.with(|p| p.borrow().clone()) };
    let mut disp = WasmDisplay::new();
    render_switch_screen(&mut disp, &party);
    // Use update_pixels so P_BATTLE_PIXELS also holds the switch screen —
    // restore_screen can then correctly return to the switch prompt after a
    // long-press stats view. SwitchIn's redraw() will overwrite P_BATTLE_PIXELS
    // with the new battle screen once the switch completes.
    update_pixels(player, disp.to_rgba());
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
    use embedded_graphics::{
        mono_font::{ascii::FONT_6X10, MonoTextStyle},
        pixelcolor::BinaryColor,
        prelude::*,
        text::{Alignment, Baseline, Text, TextStyleBuilder},
    };
    let mut disp = WasmDisplay::new();
    let ts = TextStyleBuilder::new()
        .alignment(Alignment::Center)
        .baseline(Baseline::Top)
        .build();
    Text::with_text_style(
        "Already fainted!",
        Point::new(64, 27),
        MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
        ts,
    ).draw(&mut disp).ok();
    display_only(player, disp.to_rgba());
}

/// Pop one button event from the player's queue without blocking.
pub(crate) fn pop_player_button(player: u8) -> Option<ButtonEvent> {
    if player == 1 {
        P1_QUEUE.with(|q| q.borrow_mut().pop_front())
    } else {
        P2_QUEUE.with(|q| q.borrow_mut().pop_front())
    }
}

/// Overlay the waiting screen on the player's OLED using display_only
/// so P_BATTLE_PIXELS is preserved for restore after cancel.
pub(crate) fn show_waiting_screen(player: u8) {
    use embedded_graphics::{
        mono_font::{ascii::{FONT_5X8, FONT_6X10}, MonoTextStyle},
        pixelcolor::BinaryColor,
        prelude::*,
        text::{Alignment, Baseline, Text, TextStyleBuilder},
    };
    let mon_name = get_active_mon_name(player);
    let mut disp = WasmDisplay::new();
    let ts = TextStyleBuilder::new()
        .alignment(Alignment::Center)
        .baseline(Baseline::Top)
        .build();
    Text::with_text_style(
        &mon_name,
        Point::new(64, 12),
        MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
        ts,
    ).draw(&mut disp).ok();
    Text::with_text_style(
        "Waiting...",
        Point::new(64, 28),
        MonoTextStyle::new(&FONT_5X8, BinaryColor::On),
        ts,
    ).draw(&mut disp).ok();
    Text::with_text_style(
        "tap to unready",
        Point::new(64, 42),
        MonoTextStyle::new(&FONT_5X8, BinaryColor::On),
        ts,
    ).draw(&mut disp).ok();
    display_only(player, disp.to_rgba());
}

pub(crate) fn restore_screen(player: u8) {
    if player == 1 {
        P1_IN_DETAIL.with(|d| *d.borrow_mut() = false);
        let pix = P1_BATTLE_PIXELS.with(|p| p.borrow().clone());
        P1_PIXELS.with(|p| *p.borrow_mut() = pix);
    } else {
        P2_IN_DETAIL.with(|d| *d.borrow_mut() = false);
        let pix = P2_BATTLE_PIXELS.with(|p| p.borrow().clone());
        P2_PIXELS.with(|p| *p.borrow_mut() = pix);
    }
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

fn pack_rgb(r: u8, g: u8, b: u8) -> u32 {
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
    push_button(ButtonEvent::Move { player: 1, slot: 0 });
    push_button(ButtonEvent::Move { player: 2, slot: 0 });
}

#[wasm_bindgen] pub fn submit_text(line: String) {
    let cmd = line.trim();

    // Global commands always available
    match cmd {
        ":reset"    => { wasm_reset(); return; }
        ":anim off" => { ANIM_ENABLED.with(|a| *a.borrow_mut() = false); print_log("[anim] animations OFF"); return; }
        ":anim on"  => { ANIM_ENABLED.with(|a| *a.borrow_mut() = true);  print_log("[anim] animations ON");  return; }
        _ => {}
    }

    if LOBBY_MODE.with(|m| *m.borrow()) {
        match cmd {
            ":ready ai" => {
                // VS AI: P1=AI (Red), P2=human (Blue)
                NEXT_GAME_AI.with(|n| *n.borrow_mut() = Some([true, false]));
                push_button(ButtonEvent::Move { player: 1, slot: 0 });
                push_button(ButtonEvent::Move { player: 2, slot: 0 });
            }
            ":demo" => { enter_demo_mode(); }
            ":ready" | ":ready both" => {
                push_button(ButtonEvent::Move { player: 1, slot: 0 });
                push_button(ButtonEvent::Move { player: 2, slot: 0 });
            }
            ":ready p1" => push_button(ButtonEvent::Move { player: 1, slot: 0 }),
            ":ready p2" => push_button(ButtonEvent::Move { player: 2, slot: 0 }),
            _ => { print_log("  lobby: :ready | :ready p1 | :ready p2 | :ready ai | :demo"); }
        }
        return;
    }

    // In-game: parse move/switch commands.
    // Default player is p2 (Blue, the human in VS AI mode).
    // Prefix with "p1:" to send to Red instead.
    let (player, rest) = if let Some(r) = cmd.strip_prefix("p1:") {
        (1u8, r)
    } else if let Some(r) = cmd.strip_prefix("p2:") {
        (2u8, r)
    } else {
        (2u8, cmd)
    };

    // "s1"-"s3" → switch slot 0-2
    if let Some(n) = rest.strip_prefix('s').or_else(|| rest.strip_prefix('S')) {
        if let Ok(idx) = n.parse::<u8>() {
            if idx >= 1 && idx <= 3 {
                push_button(ButtonEvent::Switch { player, idx: idx - 1 });
                return;
            }
        }
    }

    // "1"-"4" → move slot 0-3
    if let Ok(slot) = rest.parse::<u8>() {
        if slot >= 1 && slot <= 4 {
            push_button(ButtonEvent::Move { player, slot: slot - 1 });
            return;
        }
    }

    print_log("  in-game: 1-4 (move) | s1-s3 (switch) | p1:1 (send to Red)");
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

fn draw_centered(disp: &mut WasmDisplay, line1: &str, line2: &str) {
    disp.clear_all();
    let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    let ts = TextStyleBuilder::new()
        .alignment(Alignment::Center)
        .baseline(Baseline::Top)
        .build();
    Text::with_text_style(line1, Point::new(64, 20), style, ts).draw(disp).ok();
    Text::with_text_style(line2, Point::new(64, 36), style, ts).draw(disp).ok();
}

fn set_lobby_displays() {
    let mut d1 = WasmDisplay::new();
    let mut d2 = WasmDisplay::new();
    draw_centered(&mut d1, "P1  RED", "PRESS READY");
    draw_centered(&mut d2, "P2  BLUE", "PRESS READY");
    update_pixels(1, d1.to_rgba());
    update_pixels(2, d2.to_rgba());
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
            for _ in 0..150u16 {  // 150 × 100 ms = 15 s
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

            // Wait for both players to press a button
            loop {
                let ev = AnyButtonFuture.await;
                let player = match &ev {
                    ButtonEvent::Move   { player, .. }
                    | ButtonEvent::Switch { player, .. } => *player,
                };
                LOBBY_READY.with(|r| r.borrow_mut()[(player - 1) as usize] = true);
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
        // Reset detail overlay state so a held button at end of last game
        // doesn't bleed into this one.
        P1_IN_DETAIL.with(|d| *d.borrow_mut() = false);
        P2_IN_DETAIL.with(|d| *d.borrow_mut() = false);

        // Countdown fanfare: 3 gold flashes
        let gold = pack_rgb(200, 150, 0);
        for _ in 0..3u8 {
            update_leds([gold; 24]);
            sleep_ms(200).await;
            update_leds([0u32; 24]);
            sleep_ms(100).await;
        }

        let seed = (Date::now() as u64) ^ 0xdead_beef_cafe_babe;

        let mut battle = match battler::PublicCoreBattle::new(
            battle_options_with_seed(seed),
            &data,
            demo_engine_opts(),
        ) {
            Ok(b) => b,
            Err(e) => { print_log(&format!("Battle init error: {e}")); continue; }
        };

        let team_red  = draw_randbat_team(seed, 3);
        let team_blue = draw_randbat_team(seed.wrapping_add(0x9e3779b97f4a7c15), 3);

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
            controller.run(&bus),
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
