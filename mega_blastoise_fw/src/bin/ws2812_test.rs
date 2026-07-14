//! Minimal WS2812B (NeoPixel) bring-up test — 12-LED chain on GP20 via PIO0.
//!
//! Cycles through a sequence so you can eyeball wiring and color order:
//!   1. all RED, all GREEN, all BLUE   — verifies color order (driver maps
//!      RGB8 -> GRB, so "red" really shows red if wiring is correct)
//!   2. a single white pixel walking 0 -> 11   — verifies LED count + order
//!   3. a scrolling rainbow                     — looks pretty, confirms PWM
//! ...then repeats forever.
//!
//! Edit `NUM_LEDS` / the data pin below if your strip differs.
//!
//! Build / flash (requires the `leds` feature for smart-leds):
//!   cargo rb ws2812_test --features leds
//!   # equivalently: cargo run --bin ws2812_test --features leds

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::pio_programs::ws2812::{PioWs2812, PioWs2812Program};
use embassy_time::Timer;
use mega_blastoise_fw as _;
use mega_blastoise_fw::mem_profile::init_heap;
use rtt_target::{rtt_init, set_defmt_channel};
use smart_leds::RGB8;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

const NUM_LEDS: usize = 12;
const OFF: RGB8 = RGB8 { r: 0, g: 0, b: 0 };

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    init_heap();

    let channels = rtt_init! {
        up: { 0: { size: 4096, name: "defmt" } }
    };
    set_defmt_channel(channels.up.0);

    #[cfg(feature = "breadboard")]
    defmt::info!("ws2812_test: driving {} LEDs on GP20 + GP22 (PIO0 SM0/SM1)", NUM_LEDS);
    #[cfg(not(feature = "breadboard"))]
    defmt::info!("ws2812_test: driving {} LEDs on GP0 + GP1 (PIO0 SM0/SM1)", NUM_LEDS);

    let Pio { mut common, sm0, sm1, .. } = Pio::new(p.PIO0, Irqs);
    let prg = PioWs2812Program::new(&mut common);
    // Same per-board pin map as subsystems::led (PCB: GP0/GP1 per the
    // schematic's LED_P1/LED_P2 nets; breadboard: GP20/GP22).
    #[cfg(feature = "breadboard")]
    let (p1_pin, p2_pin) = (p.PIN_20, p.PIN_22);
    #[cfg(not(feature = "breadboard"))]
    let (p1_pin, p2_pin) = (p.PIN_0, p.PIN_1);
    let mut ws: PioWs2812<'_, PIO0, 0, NUM_LEDS> =
        PioWs2812::new(&mut common, sm0, p.DMA_CH0, p1_pin, &prg);
    let mut ws2: PioWs2812<'_, PIO0, 1, NUM_LEDS> =
        PioWs2812::new(&mut common, sm1, p.DMA_CH1, p2_pin, &prg);

    // Make sure everything starts dark.
    ws.write(&[OFF; NUM_LEDS]).await;
    ws2.write(&[OFF; NUM_LEDS]).await;
    Timer::after_millis(200).await;

    loop {
        // ── 1. Solid color sweep — verify color order is RGB-correct ──────────
        for (name, color) in [
            ("RED", RGB8 { r: 40, g: 0, b: 0 }),
            ("GREEN", RGB8 { r: 0, g: 40, b: 0 }),
            ("BLUE", RGB8 { r: 0, g: 0, b: 40 }),
        ] {
            defmt::info!("solid {}", name);
            ws.write(&[color; NUM_LEDS]).await;
            ws2.write(&[color; NUM_LEDS]).await;
            Timer::after_millis(700).await;
        }

        // ── 2. Walking white pixel — verify count + ordering ─────────────────
        defmt::info!("walking pixel");
        for i in 0..NUM_LEDS {
            let mut frame = [OFF; NUM_LEDS];
            frame[i] = RGB8 { r: 30, g: 30, b: 30 };
            ws.write(&frame).await;
            ws2.write(&frame).await;
            Timer::after_millis(120).await;
        }

        // ── 3. Scrolling rainbow ─────────────────────────────────────────────
        defmt::info!("rainbow");
        for phase in 0u8..=255 {
            let mut frame = [OFF; NUM_LEDS];
            for (i, px) in frame.iter_mut().enumerate() {
                let hue = phase.wrapping_add((i * 256 / NUM_LEDS) as u8);
                *px = wheel(hue);
            }
            ws.write(&frame).await;
            ws2.write(&frame).await;
            Timer::after_millis(15).await;
        }
    }
}

/// Map 0..=255 to a color wheel (R -> G -> B -> R), dimmed to ~1/6 brightness
/// so it's eye-safe at close range.
fn wheel(mut pos: u8) -> RGB8 {
    let scale = |v: u8| v / 6;
    if pos < 85 {
        RGB8 { r: scale(255 - pos * 3), g: scale(pos * 3), b: 0 }
    } else if pos < 170 {
        pos -= 85;
        RGB8 { r: 0, g: scale(255 - pos * 3), b: scale(pos * 3) }
    } else {
        pos -= 170;
        RGB8 { r: scale(pos * 3), g: 0, b: scale(255 - pos * 3) }
    }
}
