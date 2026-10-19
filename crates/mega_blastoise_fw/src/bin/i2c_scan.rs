//! Full I²C bus scanner — every 7-bit address (0x08..=0x77) on every
//! candidate OLED pin pair from both boards:
//!   PCB:        P1 = I2C1 GP2 SDA/GP3 SCL,   P2 = I2C0 GP4 SDA/GP5 SCL
//!   breadboard: P1 = I2C0 GP16 SDA/GP17 SCL, P2 = I2C1 GP18 SDA/GP19 SCL
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
    let mut p = p;

    async fn sweep<T: embassy_rp::i2c::Instance>(
        label: &str,
        bus: &mut I2c<'_, T, embassy_rp::i2c::Async>,
        t: Duration,
    ) -> u32 {
        let mut buf = [0u8; 1];
        let mut hits = 0u32;
        for addr in 0x08u16..=0x77 {
            if with_timeout(t, bus.read_async(addr, &mut buf)).await.map(|r| r.is_ok()) == Ok(true) {
                defmt::info!("{}: ACK @ 0x{:02X}", label, addr as u8);
                hits += 1;
            }
        }
        defmt::info!("{}: {} device(s)", label, hits);
        hits
    }

    // ── Both boards' candidate pin pairs ─────────────────────────────────────
    // PCB:        P1 = I2C1 GP2/GP3,   P2 = I2C0 GP4/GP5
    // breadboard: P1 = I2C0 GP16/GP17, P2 = I2C1 GP18/GP19
    defmt::info!("=== PCB pins ===");
    let mut hits_pcb = 0u32;
    {
        let mut bus = I2c::new_async(p.I2C1.reborrow(), p.PIN_3.reborrow(), p.PIN_2.reborrow(), Irqs, cfg);
        hits_pcb += sweep("I2C1 GP2/GP3 (PCB P1)", &mut bus, t).await;
    }
    {
        let mut bus = I2c::new_async(p.I2C0.reborrow(), p.PIN_5.reborrow(), p.PIN_4.reborrow(), Irqs, cfg);
        hits_pcb += sweep("I2C0 GP4/GP5 (PCB P2)", &mut bus, t).await;
    }

    defmt::info!("=== breadboard pins ===");
    let mut bus0 = I2c::new_async(p.I2C0.reborrow(), p.PIN_17.reborrow(), p.PIN_16.reborrow(), Irqs, cfg);
    let hits0 = sweep("I2C0 GP16/GP17 (bb P1)", &mut bus0, t).await;
    drop(bus0);
    let mut bus1 = I2c::new_async(p.I2C1.reborrow(), p.PIN_19.reborrow(), p.PIN_18.reborrow(), Irqs, cfg);
    let hits1 = sweep("I2C1 GP18/GP19 (bb P2)", &mut bus1, t).await;
    drop(bus1);

    if hits_pcb == 0 && hits1 == 0 && hits0 == 0 {
        defmt::error!("NO I2C devices on any pin set — power/SDA/SCL/wiring");
        defmt::error!("(if the PCB nets really cross SDA/SCL, no pair will ACK)");
    }

    // Anywhere a lone device answered, try to bring it up as an SSD1306 so a
    // present panel gets a visible confirmation.
    if hits1 == 1 {
        let bus = I2c::new_async(p.I2C1.reborrow(), p.PIN_19.reborrow(), p.PIN_18.reborrow(), Irqs, cfg);
        try_ssd1306("bb I2C1 GP18/GP19", bus).await;
    }
    if hits0 == 1 {
        let bus = I2c::new_async(p.I2C0.reborrow(), p.PIN_17.reborrow(), p.PIN_16.reborrow(), Irqs, cfg);
        try_ssd1306("bb I2C0 GP16/GP17", bus).await;
    }
    if hits_pcb >= 1 {
        let bus = I2c::new_async(p.I2C1.reborrow(), p.PIN_3.reborrow(), p.PIN_2.reborrow(), Irqs, cfg);
        try_ssd1306("PCB I2C1 GP2/GP3", bus).await;
        let bus = I2c::new_async(p.I2C0.reborrow(), p.PIN_5.reborrow(), p.PIN_4.reborrow(), Irqs, cfg);
        try_ssd1306("PCB I2C0 GP4/GP5", bus).await;
    }

    loop {}
}

async fn try_ssd1306<T: embassy_rp::i2c::Instance>(
    label: &str,
    bus: I2c<'_, T, embassy_rp::i2c::Async>,
) {
    defmt::info!("{}: trying SSD1306 init...", label);
    // SSD1306 default ctor uses 0x3C; a lone/first device is almost always
    // the panel — we just confirm init succeeds.
    let mut disp = Ssd1306Async::new(
        I2CDisplayInterface::new(bus),
        DisplaySize128x64,
        DisplayRotation::Rotate0,
    )
    .into_buffered_graphics_mode();
    match disp.init().await {
        Ok(()) => defmt::info!("{}: SSD1306 init OK", label),
        Err(_) => defmt::warn!("{}: device present but SSD1306 init failed (addr != 0x3C?)", label),
    }
}
