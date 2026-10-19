//! Button-matrix bring-up test — prints every press/release over RTT.
//!
//! Drives the same 4×4 matrix the game uses:
//!   Row 0 = GP5  P1 moves   cols 0–3 → MOVE 1–4
//!   Row 1 = GP7  P1 party   cols 0–2 → PARTY 1–3   (col 3 unused)
//!   Row 2 = GP8  P2 moves   cols 0–3 → MOVE 1–4
//!   Row 3 = GP9  P2 party   cols 0–2 → PARTY 1–3   (col 3 unused)
//!   Cols  = GP10–GP13 (inputs, internal pull-ups, LOW = pressed)
//!
//! Each row is driven LOW one at a time; a column reading LOW = that button is
//! down. Prints `PRESS P1 MOVE 2` on the rising edge and `RELEASE … held=NNNms
//! long=true/false` on release, so you can verify wiring AND the long-press
//! threshold the game uses (≥500 ms).
//!
//! Build / flash:
//!   cargo rb button_test

#![no_std]
#![no_main]

use cortex_m::asm::delay as asm_delay;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_time::{Instant, Timer};
use mega_blastoise_fw as _;
use mega_blastoise_fw::mem_profile::init_heap;
use rtt_target::{rtt_init, set_defmt_channel};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    init_heap();

    let channels = rtt_init! {
        up: { 0: { size: 4096, name: "defmt" } }
    };
    set_defmt_channel(channels.up.0);

    defmt::info!("button_test: rows GP5/GP7/GP8/GP9, cols GP10-13. Press buttons…");

    let mut rows = [
        Output::new(p.PIN_5, Level::High),
        Output::new(p.PIN_7, Level::High),
        Output::new(p.PIN_8, Level::High),
        Output::new(p.PIN_9, Level::High),
    ];
    let cols = [
        Input::new(p.PIN_10, Pull::Up),
        Input::new(p.PIN_11, Pull::Up),
        Input::new(p.PIN_12, Pull::Up),
        Input::new(p.PIN_13, Pull::Up),
    ];

    let mut down = [[false; 4]; 4];
    let mut since = [[0u64; 4]; 4];

    loop {
        for r in 0..4 {
            rows[r].set_low();
            asm_delay(1500); // ≈12 µs settle, same as the game's scan
            for c in 0..4 {
                let pressed = cols[c].is_low();
                if pressed && !down[r][c] {
                    down[r][c] = true;
                    since[r][c] = Instant::now().as_millis();
                    log_edge("PRESS  ", r, c, 0);
                } else if !pressed && down[r][c] {
                    down[r][c] = false;
                    let held = Instant::now().as_millis().saturating_sub(since[r][c]);
                    log_edge("RELEASE", r, c, held);
                }
            }
            rows[r].set_high();
        }
        Timer::after_millis(5).await;
    }
}

fn log_edge(action: &str, row: usize, col: usize, held_ms: u64) {
    let player = if row < 2 { "P1" } else { "P2" };
    let is_move = row % 2 == 0;
    let kind = if is_move { "MOVE" } else { "PARTY" };
    let valid = if is_move { col < 4 } else { col < 3 };
    let n = (col + 1) as u8;
    if !valid {
        defmt::warn!("{=str} {=str} col{=u8}  (UNUSED position — should be nothing here)", action, player, n);
        return;
    }
    if held_ms == 0 {
        defmt::info!("{=str} {=str} {=str} {=u8}", action, player, kind, n);
    } else {
        defmt::info!(
            "{=str} {=str} {=str} {=u8}  held={=u64}ms long={=bool}",
            action, player, kind, n, held_ms, held_ms >= 500
        );
    }
}
