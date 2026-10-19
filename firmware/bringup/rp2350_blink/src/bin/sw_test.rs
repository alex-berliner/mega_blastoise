//! Switch bring-up test for all 16 buttons on the stripboard v2.
//!
//! GPIO map is BUILD.md §10 (net names) + layout.py (SW numbers):
//!
//!   P1: SW1 UP GP4, SW2 DOWN GP3, SW3 LEFT GP1, SW4 RIGHT GP2,
//!       SW5 A GP14, SW6 B GP12, SW7 INFO GP13, SW8 MENU GP11
//!   P2: SW9 UP GP20, SW10 DOWN GP18, SW11 LEFT GP17, SW12 RIGHT GP21,
//!       SW13 A GP27, SW14 B GP22, SW15 INFO GP26, SW16 MENU GP19
//!
//! All are direct-wired tactiles to ground: internal pull-up, LOW = pressed.
//! Two independent signs of life, same philosophy as the blink target:
//!
//!   * GP25 LED lights while either switch is held — bench check with no host.
//!   * The USB CDC port (c0de:cafe) prints PRESS/RELEASE edges with hold time
//!     and the game's ≥500 ms long-press verdict.
//!
//! Sampling is 5 ms with a 4-sample (20 ms) debounce, so tactile bounce cannot
//! double-print an edge.
//!
//! Flash:
//!   cargo build --release --bin sw_test
//!   probe-rs download --chip RP235x --binary-format elf \
//!       target/thumbv8m.main-none-eabihf/release/sw_test
//!   probe-rs reset --chip RP235x

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::block::ImageDef;
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_time::{Duration, Instant, Timer};
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::{Builder, Config, UsbDevice};
use panic_halt as _;
use static_cell::StaticCell;

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

/// One debounced switch: `state` flips only after `STABLE` identical samples.
struct Switch {
    pin: Input<'static>,
    name: &'static str,
    down: bool,
    run: u8,
    pressed_at: Instant,
}

const STABLE: u8 = 4; // 4 × 5 ms poll = 20 ms debounce

impl Switch {
    fn new(pin: Input<'static>, name: &'static str) -> Self {
        Self { pin, name, down: false, run: 0, pressed_at: Instant::MIN }
    }

    /// Returns Some(held_ms) on a release edge, Some(0) on a press edge.
    fn poll(&mut self) -> Option<u64> {
        let raw_down = self.pin.is_low();
        if raw_down == self.down {
            self.run = 0;
            return None;
        }
        self.run += 1;
        if self.run < STABLE {
            return None;
        }
        self.run = 0;
        self.down = raw_down;
        if raw_down {
            self.pressed_at = Instant::now();
            Some(0)
        } else {
            Some(Instant::now().duration_since(self.pressed_at).as_millis().max(1))
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let driver = Driver::new(p.USB, Irqs);

    let mut config = Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("mega-blastoise");
    config.product = Some("RP2350 switch test");
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

    let mut led = Output::new(p.PIN_25, Level::Low);
    let mut switches = [
        Switch::new(Input::new(p.PIN_4, Pull::Up), "SW1 P1_UP GP4"),
        Switch::new(Input::new(p.PIN_3, Pull::Up), "SW2 P1_DOWN GP3"),
        Switch::new(Input::new(p.PIN_1, Pull::Up), "SW3 P1_LEFT GP1"),
        Switch::new(Input::new(p.PIN_2, Pull::Up), "SW4 P1_RIGHT GP2"),
        Switch::new(Input::new(p.PIN_14, Pull::Up), "SW5 P1_A GP14"),
        Switch::new(Input::new(p.PIN_12, Pull::Up), "SW6 P1_B GP12"),
        Switch::new(Input::new(p.PIN_13, Pull::Up), "SW7 P1_INFO GP13"),
        Switch::new(Input::new(p.PIN_11, Pull::Up), "SW8 P1_MENU GP11"),
        Switch::new(Input::new(p.PIN_20, Pull::Up), "SW9 P2_UP GP20"),
        Switch::new(Input::new(p.PIN_18, Pull::Up), "SW10 P2_DOWN GP18"),
        Switch::new(Input::new(p.PIN_17, Pull::Up), "SW11 P2_LEFT GP17"),
        Switch::new(Input::new(p.PIN_21, Pull::Up), "SW12 P2_RIGHT GP21"),
        Switch::new(Input::new(p.PIN_27, Pull::Up), "SW13 P2_A GP27"),
        Switch::new(Input::new(p.PIN_22, Pull::Up), "SW14 P2_B GP22"),
        Switch::new(Input::new(p.PIN_26, Pull::Up), "SW15 P2_INFO GP26"),
        Switch::new(Input::new(p.PIN_19, Pull::Up), "SW16 P2_MENU GP19"),
    ];

    let mut banner_sent = false;
    loop {
        let mut any_down = false;
        for sw in switches.iter_mut() {
            if let Some(held) = sw.poll() {
                let mut buf = [0u8; 64];
                let line = fmt_edge(&mut buf, sw.name, held);
                if cdc.dtr() {
                    let _ = cdc.write_packet(line).await;
                }
            }
            any_down |= sw.down;
        }
        led.set_level(if any_down { Level::High } else { Level::Low });

        if cdc.dtr() && !banner_sent {
            banner_sent = true;
            let _ = cdc
                // one CDC packet is 64 bytes, so the banner stays terse
                .write_packet(b"sw_test: SW1-SW16, stripboard v2 map, LOW=pressed\r\n")
                .await;
        } else if !cdc.dtr() {
            banner_sent = false; // re-banner on the next host connect
        }

        Timer::after(Duration::from_millis(5)).await;
    }
}

/// `PRESS <name>\r\n` or `RELEASE <name> held=<n>ms long=<y/n>\r\n`,
/// no formatter needed.
fn fmt_edge<'a>(buf: &'a mut [u8; 64], name: &str, held_ms: u64) -> &'a [u8] {
    fn put(buf: &mut [u8; 64], i: &mut usize, s: &[u8]) {
        buf[*i..*i + s.len()].copy_from_slice(s);
        *i += s.len();
    }
    let mut i = 0;
    if held_ms == 0 {
        put(buf, &mut i, b"PRESS   ");
        put(buf, &mut i, name.as_bytes());
    } else {
        put(buf, &mut i, b"RELEASE ");
        put(buf, &mut i, name.as_bytes());
        put(buf, &mut i, b" held=");
        let mut digits = [0u8; 20];
        let mut d = 0;
        let mut v = held_ms;
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
        put(buf, &mut i, b"ms long=");
        put(buf, &mut i, if held_ms >= 500 { b"true" } else { b"false" });
    }
    buf[i] = b'\r';
    buf[i + 1] = b'\n';
    &buf[..i + 2]
}
