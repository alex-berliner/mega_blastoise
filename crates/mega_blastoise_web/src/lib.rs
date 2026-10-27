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
//!     log and entropy;
//!   * `present` renders the current state into the pixel cache the page polls.

use std::cell::RefCell;

use js_sys::Date;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

use mega_blastoise_core::device_runner::{DeviceEvent, DeviceInputChannel, DevicePort, DeviceRunner};
use mega_blastoise_core::device_session::{Button, DeviceSession};
use mega_blastoise_core::{OledController, PadEvent, Screen, BATTLE_HELP};

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

    static LOBBY: RefCell<bool> = RefCell::new(true);
    static MENU_ACTIVE: RefCell<bool> = RefCell::new(true);
    static WAITING: RefCell<[bool; 2]> = RefCell::new([false, false]);
    static SCREEN_STATES: RefCell<[String; 3]> =
        RefCell::new([String::new(), String::new(), String::new()]);
    static LAST_TRACE: RefCell<[String; 3]> =
        RefCell::new([String::new(), String::new(), String::new()]);
    static READY_FLAGS: RefCell<[bool; 2]> = RefCell::new([false, false]);

    static ANIM: RefCell<bool> = RefCell::new(true);
}

fn seat_i(player: u8) -> usize {
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

    fn battery(&self) -> mega_blastoise_core::display_color::Battery {
        battery_param()
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
        runner.run().await
    });
}

// ── WASM exports: input ──────────────────────────────────────────────────────

/// Commit a move slot directly (the headless probe's shortcut past the
/// cursor).
#[wasm_bindgen]
pub fn press_move(player: u8, slot: u8) {
    send(DeviceEvent::Pad(PadEvent::TapMove { player, slot }));
}

/// Commit a party slot directly (the headless probe's shortcut past the
/// cursor).
#[wasm_bindgen]
pub fn press_switch(player: u8, idx: u8) {
    send(DeviceEvent::Pad(PadEvent::TapSwitch { player, idx }));
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

/// The seat's MENU key (the Y cap on the page). Its meaning is core's: the
/// press goes in as the raw `pN menu` line, which core's button grammar
/// (`device_runner::parse_button_line`) turns into a session press, exactly
/// as a typed `p1 menu` on the firmware console would.
#[wasm_bindgen]
pub fn nav_menu(player: u8) {
    let player = player.clamp(1, 2);
    send(DeviceEvent::Line(format!("p{player} menu")));
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
pub fn is_lobby_mode() -> bool {
    LOBBY.with(|m| *m.borrow())
}

#[wasm_bindgen]
pub fn menu_active() -> bool {
    MENU_ACTIVE.with(|m| *m.borrow())
}

#[wasm_bindgen]
pub fn seat_is_waiting(player: u8) -> bool {
    WAITING.with(|w| w.borrow()[seat_i(player)])
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
