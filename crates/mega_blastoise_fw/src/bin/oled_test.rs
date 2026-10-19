//! Minimal SSD1306 discovery + draw test.
//!
//! Scans I2C1 (GP18 SDA, GP19 SCL) then I2C0 (GP16 SDA, GP17 SCL) for an
//! SSD1306 at 0x3C or 0x3D.  Draws "HELLO WORLD" on the first display found.
//!
//! Build / flash:
//!   cargo build --bin oled_test
//!   cargo run   --bin oled_test

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::i2c::{Config as I2cConfig, I2c, InterruptHandler};
use embassy_rp::peripherals::{I2C0, I2C1};
use embassy_time::{with_timeout, Duration};
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use mega_blastoise_fw::mem_profile::init_heap;
use mega_blastoise_fw as _;
use rtt_target::{rtt_init, set_defmt_channel};
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306Async};

bind_interrupts!(struct Irqs {
    I2C0_IRQ => InterruptHandler<I2C0>;
    I2C1_IRQ => InterruptHandler<I2C1>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    init_heap();

    let channels = rtt_init! {
        up: { 0: { size: 4096, name: "defmt" } }
    };
    set_defmt_channel(channels.up.0);

    defmt::info!("oled_test: scanning");

    let mut cfg = I2cConfig::default();
    cfg.frequency = 100_000;

    // ── I2C1: GP18 SDA, GP19 SCL (user's OLED) ───────────────────────────────
    let mut bus1 = I2c::new_async(p.I2C1, p.PIN_19, p.PIN_18, Irqs, cfg);
    let mut scan_buf = [0u8; 1];
    let t = Duration::from_millis(50);
    let addr1 = if with_timeout(t, bus1.read_async(0x3Cu16, &mut scan_buf)).await.is_ok() {
        Some(0x3Cu8)
    } else if with_timeout(t, bus1.read_async(0x3Du16, &mut scan_buf)).await.is_ok() {
        Some(0x3Du8)
    } else {
        None
    };

    if let Some(addr) = addr1 {
        defmt::info!("I2C1: found @ 0x{:02X}", addr);
        let mut disp = Ssd1306Async::new(
            I2CDisplayInterface::new_custom_address(bus1, addr),
            DisplaySize128x64,
            DisplayRotation::Rotate0,
        )
        .into_buffered_graphics_mode();
        if disp.init().await.is_ok() {
            defmt::info!("init OK — drawing");
            disp.clear(BinaryColor::Off).ok();
            let style = MonoTextStyleBuilder::new()
                .font(&FONT_6X10)
                .text_color(BinaryColor::On)
                .build();
            Text::with_baseline("HELLO ALEX", Point::new(10, 26), style, Baseline::Top)
                .draw(&mut disp)
                .ok();
            disp.flush().await.ok();
            defmt::info!("done");
        } else {
            defmt::error!("init failed at 0x{:02X}", addr);
        }
        loop {}
    }
    defmt::warn!("I2C1: nothing found");

    // ── I2C0: GP16 SDA, GP17 SCL ─────────────────────────────────────────────
    let mut bus0 = I2c::new_async(p.I2C0, p.PIN_17, p.PIN_16, Irqs, cfg);
    let addr0 = if bus0.read_async(0x3Cu16, &mut scan_buf).await.is_ok() {
        Some(0x3Cu8)
    } else if bus0.read_async(0x3Du16, &mut scan_buf).await.is_ok() {
        Some(0x3Du8)
    } else {
        None
    };

    if let Some(addr) = addr0 {
        defmt::info!("I2C0: found @ 0x{:02X}", addr);
        let mut disp = Ssd1306Async::new(
            I2CDisplayInterface::new_custom_address(bus0, addr),
            DisplaySize128x64,
            DisplayRotation::Rotate0,
        )
        .into_buffered_graphics_mode();
        if disp.init().await.is_ok() {
            defmt::info!("init OK — drawing");
            disp.clear(BinaryColor::Off).ok();
            let style = MonoTextStyleBuilder::new()
                .font(&FONT_6X10)
                .text_color(BinaryColor::On)
                .build();
            Text::with_baseline("HELLO ALEX", Point::new(10, 26), style, Baseline::Top)
                .draw(&mut disp)
                .ok();
            disp.flush().await.ok();
            defmt::info!("done");
        } else {
            defmt::error!("init failed at 0x{:02X}", addr);
        }
        loop {}
    }
    defmt::warn!("I2C0: nothing found");

    defmt::error!("no OLED found on any bus — check wiring");
    loop {}
}
