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
    run_battle, BoardEventQueue, ButtonController, FlashDataStore, InputBus, InputSource,
    MoveSlot,
};

use web_controller::WebButtonSource;
use web_effects::WebBattleEffects;
use web_display::WasmDisplay;

// ── Global state ──────────────────────────────────────────────────────────────

thread_local! {
    static P1_PIXELS: RefCell<Vec<u8>> = RefCell::new(vec![10, 25, 10, 255].repeat(128 * 64));
    static P2_PIXELS: RefCell<Vec<u8>> = RefCell::new(vec![10, 25, 10, 255].repeat(128 * 64));
    static LED_STATE: RefCell<[u32; 24]> = RefCell::new([0u32; 24]);
    static ACTIVE_PLAYER: RefCell<u8> = RefCell::new(0);

    // Button event queue
    static BUTTON_QUEUE: RefCell<VecDeque<ButtonEvent>> = RefCell::new(VecDeque::new());
    static BUTTON_WAKER: RefCell<Option<Waker>> = RefCell::new(None);

    // Text input (unused as a future; submit_text pushes to button queue instead)
    static _TEXT_QUEUE: RefCell<VecDeque<String>> = RefCell::new(VecDeque::new());

    // Lobby LED animation mode
    static LOBBY_MODE: RefCell<bool> = RefCell::new(false);

    // Flash events: [p1_type, p2_type]; 1 = super-effective, 2 = crit; consumed on read
    static FLASH: RefCell<[u8; 2]> = RefCell::new([0, 0]);

    // Move names for button labels
    static P1_MOVE_NAMES: RefCell<[String; 4]> = RefCell::new(
        ["—".to_string(), "—".to_string(), "—".to_string(), "—".to_string()]
    );
    static P2_MOVE_NAMES: RefCell<[String; 4]> = RefCell::new(
        ["—".to_string(), "—".to_string(), "—".to_string(), "—".to_string()]
    );
}

// ── State accessors (pub(crate)) ──────────────────────────────────────────────

pub(crate) fn update_pixels(player: u8, pixels: Vec<u8>) {
    if player == 1 { P1_PIXELS.with(|p| *p.borrow_mut() = pixels); }
    else { P2_PIXELS.with(|p| *p.borrow_mut() = pixels); }
}

pub(crate) fn update_leds(leds: [u32; 24]) {
    LED_STATE.with(|l| *l.borrow_mut() = leds);
}

pub(crate) fn set_active_player(player: u8) {
    ACTIVE_PLAYER.with(|a| *a.borrow_mut() = player);
}

pub(crate) fn set_lobby_mode(active: bool) {
    LOBBY_MODE.with(|m| *m.borrow_mut() = active);
}

pub(crate) fn set_flash(player: u8, flash_type: u8) {
    FLASH.with(|f| f.borrow_mut()[(player - 1) as usize] = flash_type);
}

pub(crate) fn update_move_names(player: u8, moves: &[MoveSlot]) {
    let names: [String; 4] = std::array::from_fn(|i| {
        moves.get(i).map(|m| m.name.clone()).unwrap_or_else(|| "—".into())
    });
    if player == 1 {
        P1_MOVE_NAMES.with(|m| *m.borrow_mut() = names);
    } else {
        P2_MOVE_NAMES.with(|m| *m.borrow_mut() = names);
    }
}

fn lobby_led_frame() -> [u32; 24] {
    let t = (Date::now() as u64 / 30) as u8;
    let v = if t < 128 { t / 2 } else { (255u8.wrapping_sub(t)) / 2 };
    let color = ((v as u32 / 3) << 16) | v as u32;
    [color; 24]
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

pub(crate) struct ButtonFuture;

impl Future for ButtonFuture {
    type Output = ButtonEvent;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<ButtonEvent> {
        let ev = BUTTON_QUEUE.with(|q| q.borrow_mut().pop_front());
        if let Some(ev) = ev {
            Poll::Ready(ev)
        } else {
            BUTTON_WAKER.with(|w| *w.borrow_mut() = Some(cx.waker().clone()));
            Poll::Pending
        }
    }
}

fn push_button(ev: ButtonEvent) {
    BUTTON_QUEUE.with(|q| q.borrow_mut().push_back(ev));
    BUTTON_WAKER.with(|w| {
        if let Some(waker) = w.borrow_mut().take() { waker.wake(); }
    });
}

// ── WASM exports ──────────────────────────────────────────────────────────────

#[wasm_bindgen] pub fn press_move(player: u8, slot: u8) {
    push_button(ButtonEvent::Move { player, slot });
}

#[wasm_bindgen] pub fn press_switch(player: u8, idx: u8) {
    push_button(ButtonEvent::Switch { player, idx });
}

#[wasm_bindgen] pub fn submit_text(_line: String) {
    // Treat Enter as a button press to start/advance the lobby.
    push_button(ButtonEvent::Move { player: 1, slot: 0 });
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

#[wasm_bindgen] pub fn get_move_names(player: u8) -> String {
    if player == 1 {
        P1_MOVE_NAMES.with(|m| m.borrow().iter().cloned().collect::<Vec<_>>().join("\n"))
    } else {
        P2_MOVE_NAMES.with(|m| m.borrow().iter().cloned().collect::<Vec<_>>().join("\n"))
    }
}

#[wasm_bindgen] pub fn get_active_player() -> u8 {
    ACTIVE_PLAYER.with(|a| *a.borrow())
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
    draw_centered(&mut d1, "P1  RED", "ANY BUTTON");
    draw_centered(&mut d2, "P2  BLUE", "ANY BUTTON");
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
        set_lobby_displays();
        set_lobby_mode(true);
        set_active_player(0);
        update_move_names(1, &[]);
        update_move_names(2, &[]);

        print_log("═══════════════════════════════════════");
        print_log("     MEGA BLASTOISE  —  Gen 1 Randbat  ");
        print_log("   Press any button or Enter to start  ");
        print_log("═══════════════════════════════════════");
        print_log("");

        // Wait for any button press or Enter key
        let _ = ButtonFuture.await;
        set_lobby_mode(false);

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

        set_active_player(0);
        print_log("");
        print_log("── Battle over — press any button for a new game ───");
        print_log("");
    }
}
