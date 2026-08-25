//! Display bring-up test for the ST7796 3.5" 320x480 SPI module on the
//! stripboard v2.
//!
//! Wiring under test (BUILD.md §10, reversed-module routing):
//!
//!   LCD_BL   GP5    backlight, driven high
//!   LCD_DC   GP6    low = command, high = data
//!   LCD_RST  GP7    active low
//!   LCD_MOSI GP8
//!   LCD_SCK  GP9
//!   LCD_CS   GP10   held low for the whole session
//!
//! GP8/GP9 fit neither hardware SPI block (the production plan is PIO), so
//! this tester bit-bangs mode-0 SPI on the SIO. Slow but dependency-free, and
//! it proves every wire and the panel itself just the same. A full 320x480
//! fill lands in well under a second.
//!
//! What you should see: backlight on, then a repeating cycle of solid
//! RED, GREEN, BLUE, WHITE, BLACK, two seconds each. The CDC port
//! (c0de:cafe) names each color as it goes up, so a wrong or swapped color
//! (MADCTL/BGR or missing INVON on this IPS glass) is diagnosable from the
//! log. GP25 LED toggles on every color change: LED alive + panel dark
//! points at the panel, not the code.
//!
//! Flash:
//!   cargo build --release --bin lcd_test
//!   probe-rs download --chip RP235x --binary-format elf \
//!       target/thumbv8m.main-none-eabihf/release/lcd_test
//!   probe-rs reset --chip RP235x

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

const WIDTH: u16 = 320;
const HEIGHT: u16 = 480;

struct Lcd {
    sck: Output<'static>,
    mosi: Output<'static>,
    dc: Output<'static>,
    #[allow(dead_code)] // held low from construction on
    cs: Output<'static>,
    rst: Output<'static>,
}

impl Lcd {
    /// Mode-0 SPI, MSB first: data set while SCK low, latched on the rise.
    fn write_byte(&mut self, mut b: u8) {
        for _ in 0..8 {
            if b & 0x80 != 0 {
                self.mosi.set_high();
            } else {
                self.mosi.set_low();
            }
            self.sck.set_high();
            b <<= 1;
            self.sck.set_low();
        }
    }

    fn cmd(&mut self, c: u8) {
        self.dc.set_low();
        self.write_byte(c);
        self.dc.set_high();
    }

    fn data(&mut self, d: &[u8]) {
        for &b in d {
            self.write_byte(b);
        }
    }

    async fn hard_reset(&mut self) {
        self.rst.set_high();
        Timer::after(Duration::from_millis(10)).await;
        self.rst.set_low();
        Timer::after(Duration::from_millis(10)).await;
        self.rst.set_high();
        Timer::after(Duration::from_millis(120)).await;
    }

    async fn init(&mut self) {
        self.hard_reset().await;
        self.cmd(0x11); // SLPOUT
        Timer::after(Duration::from_millis(120)).await;
        self.cmd(0xF0); // CSCON: unlock command set 2 (clone panels want this)
        self.data(&[0xC3]);
        self.cmd(0xF0);
        self.data(&[0x96]);
        self.cmd(0x36); // MADCTL: portrait, BGR order — the usual ST7796 glass
        self.data(&[0x48]);
        self.cmd(0x3A); // COLMOD: RGB565
        self.data(&[0x55]);
        self.cmd(0x21); // INVON: IPS glass shows inverted without it
        self.cmd(0xF0); // CSCON: lock again
        self.data(&[0x3C]);
        self.cmd(0xF0);
        self.data(&[0x69]);
        self.cmd(0x13); // NORON
        self.cmd(0x29); // DISPON
        Timer::after(Duration::from_millis(20)).await;
    }

    /// Solid fill, yielding every row so the USB task never starves.
    async fn fill(&mut self, color: u16) {
        self.cmd(0x2A); // CASET
        self.data(&[0, 0, ((WIDTH - 1) >> 8) as u8, ((WIDTH - 1) & 0xFF) as u8]);
        self.cmd(0x2B); // RASET
        self.data(&[0, 0, ((HEIGHT - 1) >> 8) as u8, ((HEIGHT - 1) & 0xFF) as u8]);
        self.cmd(0x2C); // RAMWR
        let hi = (color >> 8) as u8;
        let lo = color as u8;
        for _ in 0..HEIGHT {
            for _ in 0..WIDTH {
                self.write_byte(hi);
                self.write_byte(lo);
            }
            Timer::after(Duration::from_micros(1)).await;
        }
    }
}

const COLORS: [(&[u8], u16); 5] = [
    (b"LCD: RED   0xF800\r\n", 0xF800),
    (b"LCD: GREEN 0x07E0\r\n", 0x07E0),
    (b"LCD: BLUE  0x001F\r\n", 0x001F),
    (b"LCD: WHITE 0xFFFF\r\n", 0xFFFF),
    (b"LCD: BLACK 0x0000\r\n", 0x0000),
];

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let driver = Driver::new(p.USB, Irqs);

    let mut config = Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("mega-blastoise");
    config.product = Some("RP2350 lcd test");
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
    let mut bl = Output::new(p.PIN_5, Level::Low);
    let mut lcd = Lcd {
        sck: Output::new(p.PIN_9, Level::Low),
        mosi: Output::new(p.PIN_8, Level::Low),
        dc: Output::new(p.PIN_6, Level::High),
        cs: Output::new(p.PIN_10, Level::Low),
        rst: Output::new(p.PIN_7, Level::High),
    };

    lcd.init().await;
    bl.set_high();

    loop {
        for (name, color) in COLORS {
            led.toggle();
            if cdc.dtr() {
                let _ = cdc.write_packet(name).await;
            }
            lcd.fill(color).await;
            Timer::after(Duration::from_secs(2)).await;
        }
    }
}
