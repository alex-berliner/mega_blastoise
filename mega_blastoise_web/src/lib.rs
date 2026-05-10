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
    render_move_detail, render_pokemon_stats, run_battle, BoardEventQueue,
    ButtonController, FlashDataStore, InputBus, InputSource, MoveSlot, PartySlotData,
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

    // Which players are AI-controlled this game (reset each lobby)
    static AI_PLAYERS: RefCell<[bool; 2]> = RefCell::new([false, false]);

    // Demo mode: both AI, auto-restart each game (persists until page reload)
    static DEMO_MODE: RefCell<bool> = RefCell::new(false);

    // AI pause: when true, AI players block in wait_action/wait_switch
    static AI_PAUSED: RefCell<bool> = RefCell::new(false);

}

// ── State accessors (pub(crate)) ──────────────────────────────────────────────

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

pub(crate) fn show_pokemon_stats(player: u8, team_idx: usize) {
    let party = if player == 1 { P1_PARTY.with(|p| p.borrow().clone()) }
                else           { P2_PARTY.with(|p| p.borrow().clone()) };
    if let Some(slot) = party.get(team_idx) {
        let mut disp = WasmDisplay::new();
        render_pokemon_stats(&mut disp, slot);
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

pub(crate) fn set_lobby_mode(active: bool) {
    LOBBY_MODE.with(|m| *m.borrow_mut() = active);
}

fn pack_rgb(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

pub(crate) async fn sleep_ms(ms: u32) {
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
    show_pokemon_stats(player, team_idx as usize);
}

#[wasm_bindgen] pub fn wasm_restore_screen(player: u8) {
    restore_screen(player);
}

fn enter_demo_mode() {
    AI_PLAYERS.with(|a| *a.borrow_mut() = [true, true]);
    DEMO_MODE.with(|d| *d.borrow_mut() = true);
    push_button(ButtonEvent::Move { player: 1, slot: 0 });
    push_button(ButtonEvent::Move { player: 2, slot: 0 });
}

#[wasm_bindgen] pub fn wasm_enter_demo_mode() {
    if !LOBBY_MODE.with(|m| *m.borrow()) { return; }
    enter_demo_mode();
}

#[wasm_bindgen] pub fn wasm_toggle_ai_pause() -> bool {
    let new_state = AI_PAUSED.with(|p| { let v = !*p.borrow(); *p.borrow_mut() = v; v });
    new_state
}

#[wasm_bindgen] pub fn wasm_enter_vs_ai_mode() {
    if !LOBBY_MODE.with(|m| *m.borrow()) { return; }
    // P1 human, P2 AI — single game, returns to normal lobby after
    AI_PLAYERS.with(|a| { let mut a = a.borrow_mut(); a[0] = false; a[1] = true; });
    push_button(ButtonEvent::Move { player: 1, slot: 0 });
    push_button(ButtonEvent::Move { player: 2, slot: 0 });
}

#[wasm_bindgen] pub fn submit_text(line: String) {
    if !LOBBY_MODE.with(|m| *m.borrow()) { return; }
    match line.trim() {
        ":ready ai" | ":demo" => {
            enter_demo_mode();
        }
        ":ready" | ":ready both" | "" => {
            push_button(ButtonEvent::Move { player: 1, slot: 0 });
            push_button(ButtonEvent::Move { player: 2, slot: 0 });
        }
        ":ready p1" => push_button(ButtonEvent::Move { player: 1, slot: 0 }),
        ":ready p2" => push_button(ButtonEvent::Move { player: 2, slot: 0 }),
        _ => {
            push_button(ButtonEvent::Move { player: 1, slot: 0 });
            push_button(ButtonEvent::Move { player: 2, slot: 0 });
        }
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
            // Demo mode: both players are AI; pause briefly, but exit to normal
            // lobby if a human presses any button during the pause.
            AI_PLAYERS.with(|a| *a.borrow_mut() = [true, true]);
            let mut interrupted = false;
            for _ in 0..150u16 {  // 150 × 100 ms = 15 s
                sleep_ms(100).await;
                let pressed = P1_QUEUE.with(|q| !q.borrow().is_empty())
                           || P2_QUEUE.with(|q| !q.borrow().is_empty());
                if pressed { interrupted = true; break; }
            }
            if interrupted {
                DEMO_MODE.with(|d| *d.borrow_mut() = false);
                AI_PLAYERS.with(|a| *a.borrow_mut() = [false, false]);
                set_lobby_mode(false);
                P1_QUEUE.with(|q| q.borrow_mut().clear());
                P2_QUEUE.with(|q| q.borrow_mut().clear());
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
        // Drain any button presses that accumulated during the lobby phase
        // so they don't get consumed as the first battle move.
        P1_QUEUE.with(|q| q.borrow_mut().clear());
        P2_QUEUE.with(|q| q.borrow_mut().clear());

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
