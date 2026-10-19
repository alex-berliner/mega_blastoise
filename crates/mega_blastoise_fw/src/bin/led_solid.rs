//! Static LED wiring test — P1 chain solid WHITE, P2 chain solid RED.
//!
//! No animation: each chain gets one unmistakable color so you can tell at a
//! glance which data pin is driving which physical chain. Frames are re-sent
//! twice a second, so re-seating a data wire while powered picks up within
//! half a second.
//!
//! Build / flash (requires the `leds` feature for smart-leds):
//!   cargo rb led_solid --features leds

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
const WHITE: RGB8 = RGB8 { r: 255, g: 255, b: 255 };
const RED: RGB8 = RGB8 { r: 255, g: 0, b: 0 };

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    init_heap();

    let channels = rtt_init! {
        up: { 0: { size: 4096, name: "defmt" } }
    };
    set_defmt_channel(channels.up.0);

    #[cfg(feature = "breadboard")]
    defmt::info!("led_solid: P1 WHITE on GP20, P2 RED on GP22");
    #[cfg(not(feature = "breadboard"))]
    defmt::info!("led_solid: P1 WHITE on GP1, P2 RED on GP0");

    let Pio { mut common, sm0, sm1, .. } = Pio::new(p.PIO0, Irqs);
    let prg = PioWs2812Program::new(&mut common);
    // Same per-board pin map as subsystems::led (PCB: P1=GP1, P2=GP0;
    // breadboard: GP20/GP22).
    #[cfg(feature = "breadboard")]
    let (p1_pin, p2_pin) = (p.PIN_20, p.PIN_22);
    #[cfg(not(feature = "breadboard"))]
    let (p1_pin, p2_pin) = (p.PIN_1, p.PIN_0);
    let mut ws_p1: PioWs2812<'_, PIO0, 0, NUM_LEDS> =
        PioWs2812::new(&mut common, sm0, p.DMA_CH0, p1_pin, &prg);
    let mut ws_p2: PioWs2812<'_, PIO0, 1, NUM_LEDS> =
        PioWs2812::new(&mut common, sm1, p.DMA_CH1, p2_pin, &prg);

    let mut n: u32 = 0;
    loop {
        ws_p1.write(&[WHITE; NUM_LEDS]).await;
        ws_p2.write(&[RED; NUM_LEDS]).await;
        n += 1;
        if n % 10 == 0 {
            defmt::info!("led_solid: frame #{} (P1 white / P2 red)", n);
        }
        Timer::after_millis(500).await;
    }
}
