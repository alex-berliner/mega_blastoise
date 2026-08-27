//! The web platform skin over core's [`DeviceRunner`].
//!
//! Everything that decides what the device DOES — lobby, ready sequence,
//! battles, narration pacing, what a button means — lives in
//! `mega_blastoise_core::device_runner`, shared verbatim with the firmware.
//! This crate is raw IO only:
//!
//!   * wasm exports classify pointer/keyboard input and push
//!     [`DeviceEvent`]s into the shared channel;
//!   * a [`DevicePort`] impl supplies the JS clock, timeouts, the console
//!     log, entropy, and LED/flash side channels;
//!   * `present` renders the current state into pixel caches the page polls.
//!
//! The one genuinely web-only behavior kept here is the sticky hold latch:
//! a mouse cannot hold one button while clicking another, so holds latch
//! and a second click releases them. It produces the same [`PadEvent`]s a
//! held physical button would.

mod web_display;

use std::cell::RefCell;

use js_sys::Date;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

use mega_blastoise_core::device_runner::{DeviceEvent, DeviceInputChannel, DevicePort, DeviceRunner};
use mega_blastoise_core::device_session::{Button, DeviceSession};
use mega_blastoise_core::{
    hp_bar_color, hp_bar_count, mon_display_name, mon_player_num, render_screen, BoardEvent,
    FlashDataStore, HpBarState, OledController, PadEvent, PartySlotData, Screen, BATTLE_HELP,
};
use web_display::WasmDisplay;

// ── Shared input channel ─────────────────────────────────────────────────────

static INPUT: DeviceInputChannel = DeviceInputChannel::new();

fn send(ev: DeviceEvent) {
    // try_send: a full queue means the runner is far behind; dropping the
    // newest press is the least-wrong option (mirrors the fw button task).
    let _ = INPUT.try_send(ev);
}

// ── Presentation caches (written by present(), read by exports) ──────────────

thread_local! {
    static DEVICE_PIXELS: RefCell<Vec<u8>> = RefCell::new(vec![0; 320 * 480 * 4]);
    static P1_PIXELS: RefCell<Vec<u8>> = RefCell::new(vec![10, 25, 10, 255].repeat(128 * 64));
    static P2_PIXELS: RefCell<Vec<u8>> = RefCell::new(vec![10, 25, 10, 255].repeat(128 * 64));
    static LED_STATE: RefCell<[u32; 24]> = RefCell::new([0u32; 24]);
    static FLASH: RefCell<[u8; 2]> = RefCell::new([0, 0]);

    static LOBBY: RefCell<bool> = RefCell::new(true);
    static MENU_ACTIVE: RefCell<bool> = RefCell::new(true);
    static WAITING: RefCell<[bool; 2]> = RefCell::new([false, false]);
    static SCREEN_STATES: RefCell<[String; 3]> =
        RefCell::new([String::new(), String::new(), String::new()]);
    static LAST_TRACE: RefCell<[String; 3]> =
        RefCell::new([String::new(), String::new(), String::new()]);
    static READY_FLAGS: RefCell<[bool; 2]> = RefCell::new([false, false]);

    static AI_PAUSED: RefCell<bool> = RefCell::new(false);
    static ANIM: RefCell<bool> = RefCell::new(true);

    // Sticky holds (web only): see module docs. Entries are
    // (is_switch, idx), innermost last; releases are swallowed while latched.
    static HELD_LATCH: RefCell<[Vec<(bool, u8)>; 2]> = RefCell::new([Vec::new(), Vec::new()]);
    static PENDING_END: RefCell<[u8; 2]> = RefCell::new([0; 2]);

    // HP-bar LED state per player (side channel driven by side_event).
    static HP_PCT: RefCell<[u8; 2]> = RefCell::new([100, 100]);
}

fn latch_i(player: u8) -> usize {
    (player.clamp(1, 2) - 1) as usize
}

// ── Console / log panel ──────────────────────────────────────────────────────

fn console_log(line: &str) {
    web_sys::console::log_1(&line.into());
}

fn print_log(line: &str) {
    console_log(line);
    let doc = match web_sys::window().and_then(|w| w.document()) {
        Some(d) => d,
        None => return,
    };
    if let Some(el) = doc.get_element_by_id("log") {
        el.insert_adjacent_text("beforeend", &format!("{line}\n")).ok();
        el.set_scroll_top(el.scroll_height());
    }
}

// ── Screen-state naming (device.js console tracing + the headless probe) ─────

/// Controller screens the single-screen view does not draw. Kept as a list
/// rather than a wildcard so adding an arm in `device_ui` makes this stale
/// entry obvious.
const UNMAPPED_SCREENS: [&str; 5] = [
    "CONTROLS_PICK",
    "ACTION_PICK",
    "CONCEALED_MOVES",
    "CONCEALED_SWITCH",
    "FOE_PEEK",
];

fn seat_state_name(session: &DeviceSession, ctl: &OledController, lobby: bool, player: u8) -> String {
    if session.menu_active && lobby {
        return String::from("MENU_GEN_PICKER");
    }
    let seat = &session.seats[(player == 2) as usize];
    if seat.options.is_some() {
        return String::from("MENU_OPTIONS");
    }
    if session.log_view(player).is_some() {
        return String::from("BATTLE_LOG");
    }
    let in_party = matches!(seat.nav.mode, mega_blastoise_core::cursor_nav::NavMode::Party);
    let locked = seat.locked;
    let name = ctl.screen(player).name();
    if locked && name == "MOVES" {
        return String::from("LOCKED_IN");
    }
    let name = if name == "MOVES" && in_party { "PARTY" } else { name };
    if UNMAPPED_SCREENS.contains(&name) {
        format!("UNMAPPED:{name}")
    } else {
        String::from(name)
    }
}

fn view_state_name(session: &DeviceSession, lobby: bool) -> String {
    use mega_blastoise_core::menu::MenuScreen;
    if session.menu_active && lobby {
        return String::from(match session.menu.screen {
            MenuScreen::GenPicker => "MENU_GEN_PICKER",
            MenuScreen::Options => "MENU_OPTIONS",
            MenuScreen::Lobby => "MENU_LOBBY",
        });
    }
    String::from("SPLIT_HALVES")
}

fn trace_screen_states(now: &[String; 3]) {
    LAST_TRACE.with(|l| {
        let mut last = l.borrow_mut();
        for (i, label) in ["p1", "p2", "view"].iter().enumerate() {
            if last[i] == now[i] {
                continue;
            }
            if last[i].is_empty() {
                console_log(&format!("[SCREEN] {label}: {}", now[i]));
            } else {
                console_log(&format!("[SCREEN] {label}: {} -> {}", last[i], now[i]));
            }
            last[i] = now[i].clone();
        }
    });
}

// ── LED side channels ────────────────────────────────────────────────────────

fn pack_rgb(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

fn status_led_color(status: Option<&str>) -> u32 {
    match status {
        None | Some("") => pack_rgb(0, 160, 0),
        Some("par") => pack_rgb(220, 180, 0),
        Some("brn") => pack_rgb(220, 20, 0),
        Some("frz") => pack_rgb(60, 80, 255),
        Some("psn") => pack_rgb(180, 60, 220),
        Some("tox") => pack_rgb(80, 0, 100),
        Some("slp") => pack_rgb(220, 80, 80),
        _ => pack_rgb(0, 160, 0),
    }
}

fn flush_hp_leds(parties: &[Vec<PartySlotData>; 2]) {
    LED_STATE.with(|l| {
        let mut leds = l.borrow_mut();
        let pct = HP_PCT.with(|h| *h.borrow());
        for player in [1usize, 2] {
            let base = if player == 1 { 0 } else { 12 };
            let p = pct[player - 1];
            let lit = hp_bar_count(p);
            let (r, g, b) = hp_bar_color(p);
            let color = pack_rgb(r, g, b);
            for i in 0..8 {
                leds[base + i] = if i < lit { color } else { 0 };
            }
            // Party dots: status-colored, off when fainted. P2's board is
            // oriented in reverse, so mirror the slot→LED mapping.
            let pbase = if player == 1 { 8 } else { 20 };
            let party = &parties[player - 1];
            for i in 0..3usize {
                let party_idx = if player == 2 { 2 - i } else { i };
                leds[pbase + i] = if party.is_empty() {
                    status_led_color(None)
                } else {
                    match party.get(party_idx) {
                        Some(slot) if slot.hp > 0 => status_led_color(slot.status.as_deref()),
                        _ => 0,
                    }
                };
            }
            leds[pbase + 3] = 0;
        }
    });
}

fn win_leds(winner: u8) -> [u32; 24] {
    let gold = pack_rgb(200, 150, 0);
    let dim = pack_rgb(40, 0, 0);
    let grey = pack_rgb(60, 60, 60);
    let (c1, c2) = match winner {
        1 => (gold, dim),
        2 => (dim, gold),
        _ => (grey, grey),
    };
    let mut frame = [0u32; 24];
    for slot in frame.iter_mut().take(12) {
        *slot = c1;
    }
    for slot in frame.iter_mut().skip(12) {
        *slot = c2;
    }
    frame
}

// ── The port ─────────────────────────────────────────────────────────────────

struct WebPort;

/// The `seed` URL parameter, when present and numeric — pins the battle for
/// headless probes.
/// The `batt` URL parameter: a number simulates that charge level, anything
/// else (or nothing) reports Absent — the web has no cell.
fn battery_param() -> mega_blastoise_core::display_color::Battery {
    use mega_blastoise_core::display_color::Battery;
    let get = || {
        let search = web_sys::window()?.location().search().ok()?;
        let rest = search.strip_prefix('?')?;
        rest.split('&')
            .find_map(|kv| kv.strip_prefix("batt="))
            .and_then(|v| v.parse::<u8>().ok())
    };
    match get() {
        Some(pct) => Battery::Level(pct.min(100)),
        None => Battery::Absent,
    }
}

fn fixed_seed_param() -> Option<u64> {
    let search = web_sys::window()?.location().search().ok()?;
    let rest = search.strip_prefix('?')?;
    rest.split('&')
        .find_map(|kv| kv.strip_prefix("seed="))
        .and_then(|v| v.parse().ok())
}

impl DevicePort for WebPort {
    fn now_ms(&self) -> u64 {
        Date::now() as u64
    }

    async fn sleep_ms(&self, ms: u32) {
        let promise = js_sys::Promise::new(&mut |resolve: js_sys::Function, _| {
            web_sys::window()
                .unwrap()
                .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms as i32)
                .unwrap();
        });
        wasm_bindgen_futures::JsFuture::from(promise).await.ok();
    }

    fn line_out(&self, line: &str) {
        print_log(line);
    }

    fn present(&self, session: &DeviceSession, ctl: &OledController, lobby: bool) {
        LOBBY.with(|c| *c.borrow_mut() = lobby);
        MENU_ACTIVE.with(|c| *c.borrow_mut() = session.menu_active);
        READY_FLAGS.with(|c| *c.borrow_mut() = session.lobby_ready());
        WAITING.with(|c| {
            let waiting = |p: u8| {
                matches!(
                    ctl.screen(p),
                    Screen::Waiting { .. } | Screen::WaitingForOpponent { .. }
                )
            };
            *c.borrow_mut() = [waiting(1), waiting(2)];
        });

        // The single-screen panel the device page polls.
        DEVICE_PIXELS.with(|p| {
            *p.borrow_mut() = mega_blastoise_core::device_ui::render_session(session, ctl, lobby)
        });
        // The legacy two-OLED page keeps working through the same present.
        for player in [1u8, 2] {
            let mut disp = WasmDisplay::new();
            render_screen(&mut disp, &ctl.screen(player));
            let pixels = disp.to_rgba();
            if player == 1 {
                P1_PIXELS.with(|p| *p.borrow_mut() = pixels);
            } else {
                P2_PIXELS.with(|p| *p.borrow_mut() = pixels);
            }
        }

        let states = [
            seat_state_name(session, ctl, lobby, 1),
            seat_state_name(session, ctl, lobby, 2),
            view_state_name(session, lobby),
        ];
        trace_screen_states(&states);
        SCREEN_STATES.with(|s| *s.borrow_mut() = states);
    }

    fn seed(&self) -> u64 {
        fixed_seed_param().unwrap_or_else(|| (Date::now() as u64) ^ 0xdead_beef_cafe_babe)
    }

    fn side_event(&self, event: &BoardEvent, parties: &[Vec<PartySlotData>; 2]) {
        match event {
            BoardEvent::Damage { mon, health } | BoardEvent::Heal { mon, health } => {
                if let (Some(p), Some(pct)) =
                    (mon_player_num(mon), HpBarState::parse(health).map(|h| h.pct()))
                {
                    HP_PCT.with(|h| h.borrow_mut()[(p - 1) as usize] = pct);
                    flush_hp_leds(parties);
                }
            }
            BoardEvent::SwitchIn { player_id, .. } => {
                if let Some(pid) = player_id {
                    let p = mega_blastoise_core::player_id_to_num(pid);
                    HP_PCT.with(|h| h.borrow_mut()[(p - 1) as usize] = 100);
                    flush_hp_leds(parties);
                }
            }
            BoardEvent::Faint { mon, .. } => {
                if let Some(p) = mon_player_num(mon) {
                    HP_PCT.with(|h| h.borrow_mut()[(p - 1) as usize] = 0);
                    let _ = mon_display_name(mon);
                    flush_hp_leds(parties);
                }
            }
            BoardEvent::SetStatus { .. } | BoardEvent::CureStatus { .. } => {
                flush_hp_leds(parties);
            }
            BoardEvent::SuperEffective { mon } => {
                if let Some(p) = mon_player_num(mon) {
                    FLASH.with(|f| f.borrow_mut()[(p - 1) as usize] = 1);
                }
            }
            BoardEvent::CriticalHit { mon } => {
                if let Some(p) = mon_player_num(mon) {
                    FLASH.with(|f| f.borrow_mut()[(p - 1) as usize] = 2);
                }
            }
            BoardEvent::Win { side } => {
                LED_STATE.with(|l| *l.borrow_mut() = win_leds(BoardEvent::win_player_num(side)));
            }
            BoardEvent::Tie => {
                LED_STATE.with(|l| *l.borrow_mut() = win_leds(0));
            }
            _ => {}
        }
    }

    fn battery(&self) -> mega_blastoise_core::display_color::Battery {
        battery_param()
    }

    fn lobby_pulse(&self, ready: [bool; 2], _ai: [bool; 2]) {
        let t = (Date::now() as u64 / 30) as u8;
        let v = (if t < 128 { t / 2 } else { (255u8.wrapping_sub(t)) / 2 }) as u32 * 2;
        let breathe_p1 = pack_rgb(v as u8, v as u8, v as u8);
        let breathe_p2 = pack_rgb(v as u8, 0, 0);
        let done = pack_rgb(0, 200, 50);
        LED_STATE.with(|l| {
            let mut frame = l.borrow_mut();
            let c1 = if ready[0] { done } else { breathe_p1 };
            let c2 = if ready[1] { done } else { breathe_p2 };
            for slot in frame.iter_mut().take(12) {
                *slot = c1;
            }
            for slot in frame.iter_mut().skip(12) {
                *slot = c2;
            }
        });
    }
}

// ── Entry point ──────────────────────────────────────────────────────────────

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    print_log(&format!("  build {} · {}", env!("GIT_HASH"), env!("BUILD_DATETIME")));
    spawn_local(async {
        let port = WebPort;
        let runner = DeviceRunner::new(&port, &INPUT);
        let data = FlashDataStore::new();
        runner.run(&data).await
    });
}

// ── Sticky hold latch (web only) ─────────────────────────────────────────────

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
        send(DeviceEvent::Pad(PadEvent::HoldEnd { player }));
    }
    was_latched
}

fn latch_held(player: u8, is_switch: bool, idx: u8) {
    HELD_LATCH.with(|l| l.borrow_mut()[latch_i(player)].push((is_switch, idx)));
}

fn any_latched(player: u8) -> bool {
    HELD_LATCH.with(|l| !l.borrow()[latch_i(player)].is_empty())
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

// ── WASM exports: input ──────────────────────────────────────────────────────

#[wasm_bindgen]
pub fn press_move(player: u8, slot: u8) {
    if is_lobby_mode() {
        send(DeviceEvent::Pad(PadEvent::TapMove { player, slot }));
    } else if unlatch_if_held(player, false, slot) {
        // Clicking a latched button releases the sticky hold.
    } else {
        send(DeviceEvent::Pad(PadEvent::TapMove { player, slot }));
    }
}

#[wasm_bindgen]
pub fn press_switch(player: u8, idx: u8) {
    if is_lobby_mode() {
        send(DeviceEvent::Pad(PadEvent::TapSwitch { player, idx }));
    } else if unlatch_if_held(player, true, idx) {
        // Clicking a latched button releases the sticky hold.
    } else {
        send(DeviceEvent::Pad(PadEvent::TapSwitch { player, idx }));
    }
}

#[wasm_bindgen]
pub fn hold_move(player: u8, slot: u8) {
    if unlatch_if_held(player, false, slot) {
        return;
    }
    latch_held(player, false, slot);
    send(DeviceEvent::Pad(PadEvent::HoldMove { player, slot }));
}

#[wasm_bindgen]
pub fn hold_switch(player: u8, idx: u8) {
    if unlatch_if_held(player, true, idx) {
        return;
    }
    latch_held(player, true, idx);
    send(DeviceEvent::Pad(PadEvent::HoldSwitch { player, idx }));
}

#[wasm_bindgen]
pub fn hold_end(player: u8) {
    if any_latched(player) {
        return;
    }
    send(DeviceEvent::Pad(PadEvent::HoldEnd { player }));
}

#[wasm_bindgen]
pub fn nav_a(player: u8) {
    send(DeviceEvent::Button { player, button: Button::A });
}

#[wasm_bindgen]
pub fn nav_b(player: u8) {
    send(DeviceEvent::Button { player, button: Button::B });
}

#[wasm_bindgen]
pub fn nav_info(player: u8) {
    send(DeviceEvent::Button { player, button: Button::Info });
}

#[wasm_bindgen]
pub fn nav_a_hold(player: u8) {
    send(DeviceEvent::Button { player, button: Button::AHold });
}

/// D-pad: 0 up, 1 down, 2 left, 3 right.
#[wasm_bindgen]
pub fn nav_dpad(player: u8, dir: u8) {
    use mega_blastoise_core::cursor_nav::Dir;
    let d = match dir {
        0 => Dir::Up,
        1 => Dir::Down,
        2 => Dir::Left,
        _ => Dir::Right,
    };
    send(DeviceEvent::Button { player, button: Button::Dpad(d) });
}

/// Tapping the panel after committing cancels it.
#[wasm_bindgen]
pub fn nav_cancel(player: u8) {
    if is_lobby_mode() || menu_active() {
        return;
    }
    if seat_is_waiting(player) {
        send(DeviceEvent::Pad(PadEvent::Cancel { player }));
    }
}

/// A raw tap on the panel, in 320x480 panel coordinates.
#[wasm_bindgen]
pub fn panel_tap(x: u32, y: u32) {
    send(DeviceEvent::PanelTap { x, y });
}

#[wasm_bindgen]
pub fn wasm_lobby_long_press(player: u8) {
    if !is_lobby_mode() {
        return;
    }
    send(DeviceEvent::Button { player, button: Button::AHold });
}

#[wasm_bindgen]
pub fn wasm_enter_demo_mode() {
    if !is_lobby_mode() {
        return;
    }
    send(DeviceEvent::AiPreset { ai: [true, true], demo: true });
}

#[wasm_bindgen]
pub fn wasm_enter_vs_ai_mode() {
    if !is_lobby_mode() {
        return;
    }
    // p1=AI (White), p2=human (Red): p2 still picks their controls.
    send(DeviceEvent::AiPreset { ai: [true, false], demo: false });
}

#[wasm_bindgen]
pub fn wasm_reset() {
    if let Some(win) = web_sys::window() {
        win.location().reload().ok();
    }
}

#[wasm_bindgen]
pub fn wasm_toggle_ai_pause() -> bool {
    let new_state = AI_PAUSED.with(|p| {
        let v = !*p.borrow();
        *p.borrow_mut() = v;
        v
    });
    send(DeviceEvent::SetAiPause(new_state));
    new_state
}

#[wasm_bindgen]
pub fn wasm_tick_bob() {
    send(DeviceEvent::Pulse);
}

// ── WASM exports: typed lines ────────────────────────────────────────────────

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
        ":screens          what each seat is looking at right now",
        ":reset            reload (new battle)",
    ] {
        print_log(&format!("    {l}"));
    }
    print_log("  In battle:");
    for l in BATTLE_HELP {
        print_log(&format!("    {l}"));
    }
}

#[wasm_bindgen]
pub fn submit_text(line: String) {
    let cmd = line.trim();

    // Platform commands stay on the platform; game meaning goes to the
    // runner as events.
    match cmd {
        ":reset" | ":restart" => {
            wasm_reset();
            return;
        }
        ":anim off" => {
            ANIM.with(|a| *a.borrow_mut() = false);
            send(DeviceEvent::SetAnim(false));
            print_log("[anim] animations OFF");
            return;
        }
        ":anim on" => {
            ANIM.with(|a| *a.borrow_mut() = true);
            send(DeviceEvent::SetAnim(true));
            print_log("[anim] animations ON");
            return;
        }
        "?" | ":help" | ":h" => {
            print_help();
            return;
        }
        ":screens" | ":screen" => {
            let s = SCREEN_STATES.with(|s| s.borrow().clone());
            print_log(&format!("[screen] p1 {} | p2 {} | view {}", s[0], s[1], s[2]));
            return;
        }
        _ => {}
    }

    if is_lobby_mode() {
        match cmd {
            ":ready ai" | ":vs ai" | ":red vs ai" | ":blue vs ai" => {
                send(DeviceEvent::AiPreset { ai: [true, false], demo: false });
            }
            ":demo" => {
                send(DeviceEvent::AiPreset { ai: [true, true], demo: true });
            }
            ":ready" | ":ready both" => {
                send(DeviceEvent::Line(String::from("p1 ok")));
                send(DeviceEvent::Line(String::from("p2 ok")));
            }
            ":ready p1" => send(DeviceEvent::Line(String::from("p1 ok"))),
            ":ready p2" => send(DeviceEvent::Line(String::from("p2 ok"))),
            other => send(DeviceEvent::Line(String::from(other))),
        }
        return;
    }

    send(DeviceEvent::Line(String::from(cmd)));
}

// ── WASM exports: state reads (all served from the present() caches) ─────────

#[wasm_bindgen]
pub fn get_device_pixels() -> Vec<u8> {
    DEVICE_PIXELS.with(|p| p.borrow().clone())
}

#[wasm_bindgen]
pub fn get_p1_pixels() -> Vec<u8> {
    P1_PIXELS.with(|p| p.borrow().clone())
}

#[wasm_bindgen]
pub fn get_p2_pixels() -> Vec<u8> {
    P2_PIXELS.with(|p| p.borrow().clone())
}

#[wasm_bindgen]
pub fn get_led_state() -> Vec<u32> {
    LED_STATE.with(|l| l.borrow().to_vec())
}

#[wasm_bindgen]
pub fn get_flash_state() -> Vec<u8> {
    FLASH.with(|f| {
        let state = f.borrow().to_vec();
        *f.borrow_mut() = [0, 0];
        state
    })
}

#[wasm_bindgen]
pub fn is_lobby_mode() -> bool {
    LOBBY.with(|m| *m.borrow())
}

#[wasm_bindgen]
pub fn menu_active() -> bool {
    MENU_ACTIVE.with(|m| *m.borrow())
}

#[wasm_bindgen]
pub fn seat_is_waiting(player: u8) -> bool {
    WAITING.with(|w| w.borrow()[latch_i(player)])
}

/// Current screen state by seat: 1 or 2 for a player, anything else for the
/// panel as a whole.
#[wasm_bindgen]
pub fn screen_state(player: u8) -> String {
    SCREEN_STATES.with(|s| {
        let s = s.borrow();
        match player {
            1 => s[0].clone(),
            2 => s[1].clone(),
            _ => s[2].clone(),
        }
    })
}

/// Hold thresholds, exported so the page classifies presses with the same
/// numbers the firmware's scan uses.
#[wasm_bindgen]
pub fn hold_threshold_ms() -> u32 {
    mega_blastoise_core::HOLD_THRESHOLD_MS as u32
}

#[wasm_bindgen]
pub fn ai_hold_ms() -> u32 {
    mega_blastoise_core::AI_HOLD_MS as u32
}
