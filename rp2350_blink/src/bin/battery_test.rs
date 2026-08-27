//! Battery gauge on the ST7796: reads VBAT_SENSE, draws the charge level.
//!
//! The sense chain under test (BUILD.md §5): switched battery through the
//! R1/R2 100k+100k halving divider (any equal pair works, the ratio is what
//! matters), C1 100 nF across the bottom leg, tap into GP28 = ADC2.
//!
//! On screen: a battery outline with a fill that tracks charge (green above
//! half, amber above a fifth, red below), percent and volts beneath it in a
//! hand-rolled 5x7 font. The CDC port (c0de:cafe) logs `VBAT 3.87V 62%`
//! every update for a headless read. GP25 LED toggles per update.
//!
//! Percent comes from a resting LiPo discharge table, linearly interpolated.
//! On charger the cell floats high, so the number reads optimistic; this is
//! a wiring tester, not a fuel gauge.
//!
//! Display driver is the same dependency-free bit-banged SPI as lcd_test.
//!
//! Flash:
//!   cargo build --release --bin battery_test
//!   probe-rs download --chip RP235x --binary-format elf \
//!       target/thumbv8m.main-none-eabihf/release/battery_test
//!   probe-rs reset --chip RP235x

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::adc::{Adc, Channel, Config as AdcConfig, InterruptHandler as AdcInterruptHandler};
use embassy_rp::block::ImageDef;
use embassy_rp::gpio::{Level, Output, Pull};
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver, InterruptHandler as UsbInterruptHandler};
use embassy_time::{Duration, Timer};
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::{Builder, Config, UsbDevice};
use panic_halt as _;
use static_cell::StaticCell;

#[link_section = ".start_block"]
#[used]
pub static IMAGE_DEF: ImageDef = ImageDef::secure_exe();

embassy_rp::bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => UsbInterruptHandler<USB>;
    ADC_IRQ_FIFO => AdcInterruptHandler;
});

#[embassy_executor::task]
async fn usb_task(usb: &'static mut UsbDevice<'static, Driver<'static, USB>>) -> ! {
    usb.run().await
}

const WIDTH: u16 = 320;
const HEIGHT: u16 = 480;

const BLACK: u16 = 0x0000;
const WHITE: u16 = 0xFFFF;
const GREEN: u16 = 0x07E0;
const AMBER: u16 = 0xFD20;
const RED: u16 = 0xF800;

// ---------------------------------------------------------------- display --

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

    async fn init(&mut self) {
        self.rst.set_high();
        Timer::after(Duration::from_millis(10)).await;
        self.rst.set_low();
        Timer::after(Duration::from_millis(10)).await;
        self.rst.set_high();
        Timer::after(Duration::from_millis(120)).await;
        self.cmd(0x11); // SLPOUT
        Timer::after(Duration::from_millis(120)).await;
        self.cmd(0xF0); // CSCON: unlock command set 2 (clone panels want this)
        self.data(&[0xC3]);
        self.cmd(0xF0);
        self.data(&[0x96]);
        self.cmd(0x36); // MADCTL: portrait, BGR order
        self.data(&[0x48]);
        self.cmd(0x3A); // COLMOD: RGB565
        self.data(&[0x55]);
        self.cmd(0x20); // INVOFF: this module is true-color (INVON showed inverted)
        self.cmd(0xF0); // CSCON: lock again
        self.data(&[0x3C]);
        self.cmd(0xF0);
        self.data(&[0x69]);
        self.cmd(0x13); // NORON
        self.cmd(0x29); // DISPON
        Timer::after(Duration::from_millis(20)).await;
    }

    fn window(&mut self, x: u16, y: u16, w: u16, h: u16) {
        let x1 = x + w - 1;
        let y1 = y + h - 1;
        self.cmd(0x2A); // CASET
        self.data(&[(x >> 8) as u8, x as u8, (x1 >> 8) as u8, x1 as u8]);
        self.cmd(0x2B); // RASET
        self.data(&[(y >> 8) as u8, y as u8, (y1 >> 8) as u8, y1 as u8]);
        self.cmd(0x2C); // RAMWR
    }

    /// Solid rectangle, yielding every row so the USB task never starves.
    async fn fill_rect(&mut self, x: u16, y: u16, w: u16, h: u16, color: u16) {
        self.window(x, y, w, h);
        let hi = (color >> 8) as u8;
        let lo = color as u8;
        for _ in 0..h {
            for _ in 0..w {
                self.write_byte(hi);
                self.write_byte(lo);
            }
            Timer::after(Duration::from_micros(1)).await;
        }
    }

    /// Fixed-width 5x7 text scaled up, drawn over its own background so a
    /// shorter number cleanly overwrites a longer one.
    async fn text(&mut self, x: u16, y: u16, scale: u16, fg: u16, bg: u16, s: &[u8]) {
        let mut cx = x;
        for &ch in s {
            let glyph = glyph(ch);
            self.window(cx, y, 6 * scale, 7 * scale);
            for row in 0..7u16 {
                let bits = glyph[row as usize];
                for _ in 0..scale {
                    for col in 0..6u16 {
                        // column 5 is the inter-character gap
                        let on = col < 5 && bits & (0x10 >> col) != 0;
                        let c = if on { fg } else { bg };
                        let hi = (c >> 8) as u8;
                        let lo = c as u8;
                        for _ in 0..scale {
                            self.write_byte(hi);
                            self.write_byte(lo);
                        }
                    }
                    Timer::after(Duration::from_micros(1)).await;
                }
            }
            cx += 6 * scale;
        }
    }
}

/// 5x7 glyphs, bit 4 = leftmost column. Just what the gauge needs.
fn glyph(ch: u8) -> [u8; 7] {
    match ch {
        b'0' => [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
        b'1' => [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E],
        b'2' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F],
        b'3' => [0x1F, 0x02, 0x04, 0x02, 0x01, 0x11, 0x0E],
        b'4' => [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
        b'5' => [0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E],
        b'6' => [0x06, 0x08, 0x10, 0x1E, 0x11, 0x11, 0x0E],
        b'7' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        b'8' => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
        b'9' => [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x02, 0x0C],
        b'.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x0C],
        b'V' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04],
        b'%' => [0x19, 0x19, 0x02, 0x04, 0x08, 0x13, 0x13],
        _ => [0; 7], // space
    }
}

// ----------------------------------------------------------------- gauge ---

// Battery outline geometry, portrait 320x480.
const BAT_X: u16 = 60;
const BAT_Y: u16 = 70;
const BAT_W: u16 = 200;
const BAT_H: u16 = 300;
const BORDER: u16 = 6;
const NUB_W: u16 = 80;
const NUB_H: u16 = 24;

/// Resting-voltage LiPo curve, (millivolts, percent), interpolated between.
const CURVE: [(u32, u32); 10] = [
    (4200, 100),
    (4060, 90),
    (3980, 80),
    (3920, 70),
    (3870, 60),
    (3820, 50),
    (3770, 30),
    (3740, 20),
    (3680, 10),
    (3300, 0),
];

fn percent(mv: u32) -> u32 {
    if mv >= CURVE[0].0 {
        return 100;
    }
    for pair in CURVE.windows(2) {
        let (hv, hp) = pair[0];
        let (lv, lp) = pair[1];
        if mv >= lv {
            return lp + (hp - lp) * (mv - lv) / (hv - lv);
        }
    }
    0
}

// ------------------------------------------------------------------ main ---

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let driver = Driver::new(p.USB, Irqs);

    let mut config = Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("mega-blastoise");
    config.product = Some("RP2350 battery test");
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

    let mut adc = Adc::new(p.ADC, Irqs, AdcConfig::default());
    let mut vbat_ch = Channel::new_pin(p.PIN_28, Pull::None);

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
    lcd.fill_rect(0, 0, WIDTH, HEIGHT, BLACK).await;

    // Static chrome: nub, then the outline as four border bars.
    let nub_x = BAT_X + (BAT_W - NUB_W) / 2;
    lcd.fill_rect(nub_x, BAT_Y - NUB_H, NUB_W, NUB_H, WHITE).await;
    lcd.fill_rect(BAT_X, BAT_Y, BAT_W, BORDER, WHITE).await;
    lcd.fill_rect(BAT_X, BAT_Y + BAT_H - BORDER, BAT_W, BORDER, WHITE).await;
    lcd.fill_rect(BAT_X, BAT_Y, BORDER, BAT_H, WHITE).await;
    lcd.fill_rect(BAT_X + BAT_W - BORDER, BAT_Y, BORDER, BAT_H, WHITE).await;

    bl.set_high();

    let ix = BAT_X + BORDER;
    let iy = BAT_Y + BORDER;
    let iw = BAT_W - 2 * BORDER;
    let ih = BAT_H - 2 * BORDER;

    let mut last_fill: u16 = u16::MAX; // force first draw

    loop {
        // 16-sample average knocks the ADC jitter down to a millivolt or two
        let mut sum: u32 = 0;
        for _ in 0..16 {
            sum += adc.read(&mut vbat_ch).await.unwrap_or(0) as u32;
            Timer::after(Duration::from_millis(2)).await;
        }
        let raw = sum / 16;
        let pin_mv = raw * 3300 / 4095;
        let vbat_mv = pin_mv * 2; // R1 = R2 halves it
        let pct = percent(vbat_mv);

        let color = if pct > 50 {
            GREEN
        } else if pct > 20 {
            AMBER
        } else {
            RED
        };

        // Fill from the bottom: charged part in the level color, rest black.
        let fill_h = (ih as u32 * pct / 100) as u16;
        if fill_h != last_fill {
            last_fill = fill_h;
            lcd.fill_rect(ix, iy, iw, ih - fill_h, BLACK).await;
            lcd.fill_rect(ix, iy + ih - fill_h, iw, fill_h, color).await;
        }

        // "100%" right-padded to 4, "4.20V" always 5 — fixed width overdraws
        let mut pct_s = [b' '; 4];
        let mut n = pct;
        pct_s[3] = b'%';
        let mut i = 3;
        loop {
            i -= 1;
            pct_s[i] = b'0' + (n % 10) as u8;
            n /= 10;
            if n == 0 {
                break;
            }
        }
        let volt_s = [
            b'0' + (vbat_mv / 1000) as u8,
            b'.',
            b'0' + (vbat_mv / 100 % 10) as u8,
            b'0' + (vbat_mv / 10 % 10) as u8,
            b'V',
        ];

        // pct: scale 8 = 48px/char, 4 chars = 192px wide
        lcd.text((WIDTH - 4 * 6 * 8) / 2, 390, 8, color, BLACK, &pct_s).await;
        // volts: scale 4 = 24px/char, 5 chars = 120px wide
        lcd.text((WIDTH - 5 * 6 * 4) / 2, 450, 4, WHITE, BLACK, &volt_s).await;

        if cdc.dtr() {
            let mut line = [0u8; 32];
            line[..5].copy_from_slice(b"VBAT ");
            line[5..10].copy_from_slice(&volt_s);
            line[10] = b' ';
            line[11..15].copy_from_slice(&pct_s);
            line[15] = b'\r';
            line[16] = b'\n';
            let _ = cdc.write_packet(&line[..17]).await;
        }

        led.toggle();
        Timer::after(Duration::from_secs(2)).await;
    }
}
