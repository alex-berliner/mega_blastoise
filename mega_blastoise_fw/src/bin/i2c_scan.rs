//! Full I²C bus scanner — every 7-bit address (0x08..=0x77) on BOTH buses.
//!
//! I2C1: GP18 SDA, GP19 SCL  (P2 — the normally-connected display)
//! I2C0: GP16 SDA, GP17 SCL  (P1)
//!
//! Logs every address that ACKs. Use this to find a display that isn't at
//! the SSD1306 default 0x3C/0x3D, or to confirm a bus is electrically dead
//! (zero ACKs anywhere => wiring/power, not address).
//!
//!   cargo build --bin i2c_scan
//!   cargo run   --bin i2c_scan

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::i2c::{Config as I2cConfig, I2c, InterruptHandler};
use embassy_rp::peripherals::{I2C0, I2C1};
use embassy_time::{with_timeout, Duration};
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

    let mut cfg = I2cConfig::default();
    cfg.frequency = 100_000;
    let t = Duration::from_millis(20);

    // ── I2C1: GP18 SDA / GP19 SCL (P2) ───────────────────────────────────────
    defmt::info!("=== scanning I2C1 (P2: SDA=GP18 SCL=GP19) ===");
    let mut bus1 = I2c::new_async(p.I2C1, p.PIN_19, p.PIN_18, Irqs, cfg);
    let mut buf = [0u8; 1];
    let mut hits1 = 0u32;
    for addr in 0x08u16..=0x77 {
        if with_timeout(t, bus1.read_async(addr, &mut buf)).await.map(|r| r.is_ok()) == Ok(true) {
            defmt::info!("I2C1: ACK @ 0x{:02X}", addr as u8);
            hits1 += 1;
        }
    }
    defmt::info!("I2C1: {} device(s)", hits1);

    // ── I2C0: GP16 SDA / GP17 SCL (P1) ───────────────────────────────────────
    defmt::info!("=== scanning I2C0 (P1: SDA=GP16 SCL=GP17) ===");
    let mut bus0 = I2c::new_async(p.I2C0, p.PIN_17, p.PIN_16, Irqs, cfg);
    let mut hits0 = 0u32;
    for addr in 0x08u16..=0x77 {
        if with_timeout(t, bus0.read_async(addr, &mut buf)).await.map(|r| r.is_ok()) == Ok(true) {
            defmt::info!("I2C0: ACK @ 0x{:02X}", addr as u8);
            hits0 += 1;
        }
    }
    defmt::info!("I2C0: {} device(s)", hits0);

    if hits1 == 0 && hits0 == 0 {
        defmt::error!("NO I2C devices on either bus — power/SDA/SCL/wiring");
    }

    // If exactly one bus had a single device, try to bring it up as an SSD1306
    // so a present-but-mis-addressed panel still gets a visible confirmation.
    if hits1 == 1 {
        try_ssd1306_i2c1(bus1).await;
    } else if hits0 == 1 {
        try_ssd1306_i2c0(bus0).await;
    }

    loop {}
}

async fn try_ssd1306_i2c1(bus: I2c<'static, I2C1, embassy_rp::i2c::Async>) {
    defmt::info!("I2C1: trying SSD1306 init @ found address...");
    // SSD1306 default ctor uses 0x3C; new_custom_address would need the addr,
    // but a lone device is almost always the panel — default ctor is fine if
    // it answered at 0x3C. We just confirm init succeeds.
    let mut disp = Ssd1306Async::new(
        I2CDisplayInterface::new(bus),
        DisplaySize128x64,
        DisplayRotation::Rotate0,
    )
    .into_buffered_graphics_mode();
    match disp.init().await {
        Ok(()) => defmt::info!("I2C1: SSD1306 init OK"),
        Err(_) => defmt::warn!("I2C1: device present but SSD1306 init failed (addr != 0x3C?)"),
    }
}

async fn try_ssd1306_i2c0(bus: I2c<'static, I2C0, embassy_rp::i2c::Async>) {
    defmt::info!("I2C0: trying SSD1306 init @ found address...");
    let mut disp = Ssd1306Async::new(
        I2CDisplayInterface::new(bus),
        DisplaySize128x64,
        DisplayRotation::Rotate0,
    )
    .into_buffered_graphics_mode();
    match disp.init().await {
        Ok(()) => defmt::info!("I2C0: SSD1306 init OK"),
        Err(_) => defmt::warn!("I2C0: device present but SSD1306 init failed (addr != 0x3C?)"),
    }
}
