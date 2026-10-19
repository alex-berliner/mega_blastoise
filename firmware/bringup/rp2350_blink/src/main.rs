//! RP2350 bring-up smoke test.
//!
//! Two independent signs of life, because the first thing a new board has to
//! answer is "did anything run at all":
//!
//!   * GP25 blinks at 2 Hz. That is the on-board LED on a Pico 2. On a Pico 2 W
//!     the LED hangs off the wireless chip instead, so the pin does nothing
//!     visible there and the serial output below is the one to watch.
//!   * A USB CDC port prints a counter once a second. Enumerating at all proves
//!     the crystal, the PLLs and the USB block are alive, which a blinking pin
//!     alone does not.
//!
//! Nothing here is shared with the game firmware: this is a throwaway target
//! for proving the chip and the flashing path, not a port.

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::block::ImageDef;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_time::{Duration, Timer};
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::{Builder, Config, UsbDevice};
use panic_halt as _;
use static_cell::StaticCell;

/// The boot ROM refuses to start an image without this block, and it has to
/// land in the first 4K of flash — `memory.x` puts the section there.
#[link_section = ".start_block"]
#[used]
pub static IMAGE_DEF: ImageDef = ImageDef::secure_exe();

embassy_rp::bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

#[embassy_executor::task]
async fn usb_task(usb: &'static mut UsbDevice<'static, Driver<'static, USB>>) -> ! {
    usb.run().await
}

#[embassy_executor::task]
async fn blink_task(mut led: Output<'static>) -> ! {
    loop {
        led.toggle();
        Timer::after(Duration::from_millis(250)).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    spawner.spawn(blink_task(Output::new(p.PIN_25, Level::Low))).unwrap();

    let driver = Driver::new(p.USB, Irqs);

    let mut config = Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("mega-blastoise");
    config.product = Some("RP2350 bring-up");
    config.serial_number = Some("1");
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    static CDC_STATE: StaticCell<State> = StaticCell::new();
    static CONFIG_BUF: StaticCell<[u8; 256]> = StaticCell::new();
    static BOS_BUF: StaticCell<[u8; 256]> = StaticCell::new();
    static CTRL_BUF: StaticCell<[u8; 64]> = StaticCell::new();
    static USB_DEV: StaticCell<UsbDevice<'static, Driver<'static, USB>>> = StaticCell::new();

    let mut builder = Builder::new(
        driver,
        config,
        CONFIG_BUF.init([0; 256]),
        BOS_BUF.init([0; 256]),
        &mut [], // no MS OS descriptors
        CTRL_BUF.init([0; 64]),
    );
    let mut cdc = CdcAcmClass::new(&mut builder, CDC_STATE.init(State::new()), 64);
    spawner.spawn(usb_task(USB_DEV.init(builder.build()))).unwrap();

    let mut ticks: u32 = 0;
    loop {
        // Waiting for DTR keeps the counter from running away into a port
        // nobody has opened yet, so the first line you see is tick 0.
        cdc.wait_connection().await;
        let mut buf = [0u8; 32];
        let line = fmt_tick(&mut buf, ticks);
        if cdc.write_packet(line).await.is_err() {
            continue;
        }
        ticks = ticks.wrapping_add(1);
        Timer::after(Duration::from_secs(1)).await;
    }
}

/// `tick <n>\r\n` without pulling in a formatter.
fn fmt_tick(buf: &mut [u8; 32], n: u32) -> &[u8] {
    const PREFIX: &[u8] = b"rp2350 alive, tick ";
    buf[..PREFIX.len()].copy_from_slice(PREFIX);
    let mut i = PREFIX.len();

    let mut digits = [0u8; 10];
    let mut d = 0;
    let mut v = n;
    loop {
        digits[d] = b'0' + (v % 10) as u8;
        d += 1;
        v /= 10;
        if v == 0 {
            break;
        }
    }
    while d > 0 {
        d -= 1;
        buf[i] = digits[d];
        i += 1;
    }
    buf[i] = b'\r';
    buf[i + 1] = b'\n';
    &buf[..i + 2]
}
