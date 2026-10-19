//! led_pin_dc — hold the LED data pins at a solid 3.3 V for multimeter
//! tracing (WS2812 data is too fast to see on a DMM).
//!
//! Drives GP0 (P1 chain) and GP1 (P2 chain) constantly HIGH with a defmt
//! heartbeat. Chase the voltage: Pico pin -> DIN pad of the first pixel.
//!
//!   cargo run --bin led_pin_dc

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output};
use embassy_time::Timer;
use mega_blastoise_fw::mem_profile::init_heap;
use mega_blastoise_fw as _;
use rtt_target::{rtt_init, set_defmt_channel};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    init_heap();

    let channels = rtt_init! {
        up: { 0: { size: 1024, name: "defmt" } }
    };
    set_defmt_channel(channels.up.0);

    let _gp0 = Output::new(p.PIN_0, Level::High);
    let _gp1 = Output::new(p.PIN_1, Level::High);
    defmt::info!("led_pin_dc: GP0 and GP1 held HIGH (3.3V) for DMM tracing");

    let mut n = 0u32;
    loop {
        Timer::after_millis(5000).await;
        n += 1;
        defmt::info!("still driving HIGH ({}s)", n * 5);
    }
}
