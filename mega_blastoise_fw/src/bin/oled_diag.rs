//! oled_diag — looping OLED wiring diagnostic for the PCB pin map.
//!
//! Repeats every ~2 s so you can wiggle/reseat wires and watch RTT live:
//!   1. Line-level check: SDA/SCL as inputs with a weak pull-DOWN. A powered
//!      SSD1306 module's onboard pull-ups (4.7–10 k) win and read HIGH, so
//!      LOW = that wire is open, or the module has no VCC/GND.
//!   2. Full address sweep (0x08..=0x77) on both PCB buses.
//!   3. Any 0x3C/0x3D hit: SSD1306 init + test pattern with a live cycle
//!      counter, so a working panel is visibly alive.
//!
//! Pin map (PCB build): P1 = I2C1 GP2 SDA / GP3 SCL,
//!                      P2 = I2C0 GP4 SDA / GP5 SCL.
//!
//!   cargo run --bin oled_diag

#![no_std]
#![no_main]

extern crate alloc;

use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Flex, Pull};
use embassy_rp::i2c::{Config as I2cConfig, I2c, InterruptHandler};
use embassy_rp::peripherals::{I2C0, I2C1};
use embassy_time::{with_timeout, Duration, Timer};
use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::Text,
};
use mega_blastoise_fw::mem_profile::init_heap;
use mega_blastoise_fw as _;
use rtt_target::{rtt_init, set_defmt_channel};
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306Async};

bind_interrupts!(struct Irqs {
    I2C0_IRQ => InterruptHandler<I2C0>;
    I2C1_IRQ => InterruptHandler<I2C1>;
});

/// SDA/SCL idle-level probe against a weak internal pull-down. Returns
/// (sda_high, scl_high); HIGH means an external pull-up is present.
async fn line_check<'a>(label: &str, sda: &mut Flex<'a>, scl: &mut Flex<'a>) -> (bool, bool) {
    for pin in [&mut *sda, &mut *scl] {
        pin.set_as_input();
        pin.set_pull(Pull::Down);
    }
    Timer::after_millis(2).await;
    let (s, c) = (sda.is_high(), scl.is_high());
    defmt::info!(
        "{}: SDA={} SCL={}",
        label,
        if s { "HIGH" } else { "LOW" },
        if c { "HIGH" } else { "LOW" }
    );
    match (s, c) {
        (true, true) => {}
        (false, false) => defmt::warn!(
            "{}: both lines dead — module likely has no VCC or no GND",
            label
        ),
        (false, true) => defmt::warn!("{}: SDA wire open (or shorted low)", label),
        (true, false) => defmt::warn!("{}: SCL wire open (or shorted low)", label),
    }
    (s, c)
}

/// Sweep every 7-bit address; log ACKs. Returns the first SSD1306-plausible
/// address (0x3C/0x3D) plus the total hit count.
async fn sweep<T: embassy_rp::i2c::Instance>(
    label: &str,
    bus: &mut I2c<'_, T, embassy_rp::i2c::Async>,
) -> (Option<u8>, u32) {
    let t = Duration::from_millis(5);
    let mut buf = [0u8; 1];
    let mut hits = 0u32;
    let mut panel = None;
    for addr in 0x08u16..=0x77 {
        if with_timeout(t, bus.read_async(addr, &mut buf)).await.map(|r| r.is_ok()) == Ok(true) {
            defmt::info!("{}: ACK @ 0x{:02X}", label, addr as u8);
            hits += 1;
            if panel.is_none() && (addr == 0x3C || addr == 0x3D) {
                panel = Some(addr as u8);
            }
        }
    }
    (panel, hits)
}

/// Init the panel and draw a bordered "Pn OK #cycle" pattern.
async fn panel_test<T: embassy_rp::i2c::Instance>(
    label: &str,
    bus: I2c<'_, T, embassy_rp::i2c::Async>,
    addr: u8,
    cycle: u32,
) {
    let iface = I2CDisplayInterface::new_custom_address(bus, addr);
    let mut disp = Ssd1306Async::new(iface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    if disp.init().await.is_err() {
        defmt::warn!("{}: ACK @ 0x{:02X} but SSD1306 init failed", label, addr);
        return;
    }
    disp.clear(BinaryColor::Off).ok();
    Rectangle::new(Point::new(0, 0), Size::new(128, 64))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 2))
        .draw(&mut disp)
        .ok();
    let style = MonoTextStyle::new(&FONT_10X20, BinaryColor::On);
    Text::new(label, Point::new(10, 26), style).draw(&mut disp).ok();
    let line2 = alloc::format!("OK #{}", cycle);
    Text::new(&line2, Point::new(10, 52), style).draw(&mut disp).ok();
    match disp.flush().await {
        Ok(()) => defmt::info!("{}: SSD1306 OK @ 0x{:02X} — test pattern on screen", label, addr),
        Err(e) => defmt::warn!(
            "{}: init OK but flush failed: {} (flaky wire?)",
            label,
            defmt::Debug2Format(&e)
        ),
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let mut p = embassy_rp::init(Default::default());
    init_heap();

    let channels = rtt_init! {
        up: { 0: { size: 4096, name: "defmt" } }
    };
    set_defmt_channel(channels.up.0);

    let mut cfg = I2cConfig::default();
    cfg.frequency = 100_000;

    defmt::info!("oled_diag: PCB pins — P1=I2C1 GP2/GP3, P2=I2C0 GP4/GP5");
    let mut cycle = 0u32;
    loop {
        cycle += 1;
        defmt::info!("=== cycle {} ===", cycle);

        // ── P1: GP2 SDA / GP3 SCL on I2C1 ────────────────────────────────
        let (s, c) = {
            let mut sda = Flex::new(p.PIN_2.reborrow());
            let mut scl = Flex::new(p.PIN_3.reborrow());
            line_check("P1 (GP2/GP3)", &mut sda, &mut scl).await
        };
        {
            let mut bus =
                I2c::new_async(p.I2C1.reborrow(), p.PIN_3.reborrow(), p.PIN_2.reborrow(), Irqs, cfg);
            let (panel, hits) = sweep("P1 (I2C1)", &mut bus, ).await;
            drop(bus);
            if let Some(addr) = panel {
                let bus = I2c::new_async(
                    p.I2C1.reborrow(), p.PIN_3.reborrow(), p.PIN_2.reborrow(), Irqs, cfg,
                );
                panel_test("P1", bus, addr, cycle).await;
            } else if hits == 0 && s && c {
                defmt::warn!(
                    "P1: pull-ups present but nothing ACKs — SDA/SCL probably swapped at the module"
                );
            }
        }

        // ── P2: GP4 SDA / GP5 SCL on I2C0 ────────────────────────────────
        let (s, c) = {
            let mut sda = Flex::new(p.PIN_4.reborrow());
            let mut scl = Flex::new(p.PIN_5.reborrow());
            line_check("P2 (GP4/GP5)", &mut sda, &mut scl).await
        };
        {
            let mut bus =
                I2c::new_async(p.I2C0.reborrow(), p.PIN_5.reborrow(), p.PIN_4.reborrow(), Irqs, cfg);
            let (panel, hits) = sweep("P2 (I2C0)", &mut bus).await;
            drop(bus);
            if let Some(addr) = panel {
                let bus = I2c::new_async(
                    p.I2C0.reborrow(), p.PIN_5.reborrow(), p.PIN_4.reborrow(), Irqs, cfg,
                );
                panel_test("P2", bus, addr, cycle).await;
            } else if hits == 0 && s && c {
                defmt::warn!(
                    "P2: pull-ups present but nothing ACKs — SDA/SCL probably swapped at the module"
                );
            }
        }

        Timer::after_millis(2000).await;
    }
}
