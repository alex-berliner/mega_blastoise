mod web_controller;
mod web_effects;

use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use battler::TeamData;
use js_sys::Date;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

use mega_blastoise_core::{
    battle_options_with_seed, demo_engine_opts, draw_randbat_team, format_active_state,
    run_battle, BoardEventQueue, ButtonController, FlashDataStore, InputBus, InputSource,
};

use web_controller::WebButtonSource;
use web_effects::WebBattleEffects;

// ── Input channel: JS → Rust ──────────────────────────────────────────────────

thread_local! {
    static INPUT_WAKER: RefCell<Option<Waker>> = RefCell::new(None);
    static INPUT_QUEUE: RefCell<VecDeque<String>> = RefCell::new(VecDeque::new());
}

struct InputLineFuture;

impl Future for InputLineFuture {
    type Output = String;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<String> {
        let line = INPUT_QUEUE.with(|q| q.borrow_mut().pop_front());
        if let Some(line) = line {
            Poll::Ready(line)
        } else {
            INPUT_WAKER.with(|w| *w.borrow_mut() = Some(cx.waker().clone()));
            Poll::Pending
        }
    }
}

pub(crate) async fn read_input_line() -> String {
    InputLineFuture.await
}

/// Called by the JS input handler when the user presses Enter.
#[wasm_bindgen]
pub fn submit_input(line: String) {
    INPUT_QUEUE.with(|q| q.borrow_mut().push_back(line));
    INPUT_WAKER.with(|w| {
        if let Some(waker) = w.borrow_mut().take() {
            waker.wake();
        }
    });
}

// ── Output: Rust → DOM ────────────────────────────────────────────────────────

pub(crate) fn print(line: &str) {
    let doc = match web_sys::window().and_then(|w| w.document()) {
        Some(d) => d,
        None => return,
    };
    if let Some(out) = doc.get_element_by_id("output") {
        out.insert_adjacent_text("beforeend", &format!("{line}\n")).ok();
        out.set_scroll_top(out.scroll_height());
    }
}

fn print_line(line: &str) {
    print(line);
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    spawn_local(run_game_loop());
}

async fn run_game_loop() {
    let data = FlashDataStore::new();

    loop {
        print("═══════════════════════════════════════");
        print("     MEGA BLASTOISE  —  Gen 1 Randbat  ");
        print("═══════════════════════════════════════");
        print("Press Enter to start a new battle...");
        print("");
        let _ = read_input_line().await;

        let seed = (Date::now() as u64) ^ 0xdead_beef_cafe_babe;

        let mut battle = match battler::PublicCoreBattle::new(
            battle_options_with_seed(seed),
            &data,
            demo_engine_opts(),
        ) {
            Ok(b) => b,
            Err(e) => {
                print(&format!("Battle init error: {e}"));
                continue;
            }
        };

        let team_red = draw_randbat_team(seed, 3);
        let team_blue = draw_randbat_team(seed.wrapping_add(0x9e3779b97f4a7c15), 3);

        let setup_ok = battle
            .update_team("p1", TeamData { members: team_red, ..Default::default() })
            .is_ok()
            && battle
                .update_team("p2", TeamData { members: team_blue, ..Default::default() })
                .is_ok()
            && battle.start().is_ok();

        if !setup_ok {
            print("Battle setup failed.");
            continue;
        }

        print("");
        print("═══════════════════════════════════════");
        print(" Randbat  3v3 singles  —  Gen 1");
        print(" Type 1-4 for a move, s1-s3 to switch.");
        print("═══════════════════════════════════════");
        print("");

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
                for line in state.lines() {
                    print(line);
                }
            },
        )
        .await;

        print("");
        print("═══════════════════════════════════════");
        print("              Battle over!             ");
        print("═══════════════════════════════════════");
        print("");
    }
}
