//! ST7796 3.5" 320x480 SPI panel — the stripboard v2 display.
//!
//! Wiring (BUILD.md §10): BL GP5, DC GP6, RST GP7, MOSI GP8, SCK GP9,
//! CS GP10. GP8/GP9 fit neither hardware SPI block; this is the PIO bus
//! BUILD.md §10 promises. A two-instruction sideset program clocks mode-0
//! SPI at 25 MHz and a single DMA transfer feeds it each frame, so a full
//! 240x320 blit is ~50 ms — against well over a second for the bit-banged
//! bring-up driver this replaced (the visible wipe down the panel).
//!
//! The core renders a 240x320 [`DeviceFrame`] (the single-screen design's
//! native size, shared with the web build — zero drift). This panel is
//! 320x480, so the frame is centred pixel-for-pixel with a black border:
//! no scaling, no resampling, nothing for the two platforms to disagree
//! about.
//!
//! The task consumes frames from [`FRAME`]; a `Signal` rather than a queue
//! because only the newest frame matters. NOTE: binds PIO0_IRQ_0, as does
//! the `leds` subsystem — `board_v2` and `leds` are mutually exclusive.

use embassy_rp::bind_interrupts;
use embassy_rp::dma::Channel as DmaChannel;
use embassy_rp::gpio::Output;
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio::{
    Config as PioConfig, Direction as PioDirection, FifoJoin, InterruptHandler, Pio,
    ShiftConfig, ShiftDirection, StateMachine,
};
use embassy_rp::Peri;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use fixed::traits::ToFixed;
use mega_blastoise_core::device_view::{DeviceFrame, DEV_H, DEV_W};

bind_interrupts!(struct PioIrqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

pub static FRAME: Signal<CriticalSectionRawMutex, DeviceFrame> = Signal::new();

/// Set after each blit (and once at init). The renderer waits on it before
/// building the next frame, so at most one 150 KB frame is ever in flight —
/// without this, a burst of presses can have three frames alive at once and
/// that is real heap pressure even on the RP2350.
pub static IDLE: Signal<CriticalSectionRawMutex, ()> = Signal::new();

const PANEL_W: u16 = 320;
const PANEL_H: u16 = 480;
const OFF_X: u16 = (PANEL_W - DEV_W as u16) / 2; // 40
const OFF_Y: u16 = (PANEL_H - DEV_H as u16) / 2; // 80

pub struct Lcd {
    sm: StateMachine<'static, PIO0, 0>,
    dma: Peri<'static, embassy_rp::peripherals::DMA_CH2>,
    dc: Output<'static>,
    /// Held low for the whole session; the panel is the bus's only device.
    #[allow(dead_code)]
    cs: Output<'static>,
    rst: Output<'static>,
    bl: Output<'static>,
}

impl Lcd {
    /// PIN_8 = MOSI and PIN_9 = SCK are fixed by the board; they are taken
    /// as parameters only so main keeps ownership of the peripherals table.
    /// DMA_CH2 keeps clear of DMA_CH0/1, spoken for by the led subsystem.
    pub fn new(
        pio: Peri<'static, PIO0>,
        mosi: Peri<'static, embassy_rp::peripherals::PIN_8>,
        sck: Peri<'static, embassy_rp::peripherals::PIN_9>,
        dma: Peri<'static, embassy_rp::peripherals::DMA_CH2>,
        dc: Output<'static>,
        cs: Output<'static>,
        rst: Output<'static>,
        bl: Output<'static>,
    ) -> Self {
        let Pio { mut common, mut sm0, .. } = Pio::new(pio, PioIrqs);

        // Mode-0 SPI TX: one data bit out on the falling edge, latched by
        // the panel on the sideset rise. Two cycles per bit.
        let prg = pio::pio_asm!(
            ".side_set 1",
            ".wrap_target",
            "out pins, 1  side 0",
            "nop          side 1",
            ".wrap",
        );
        let loaded = common.load_program(&prg.program);
        let mosi = common.make_pio_pin(mosi);
        let sck = common.make_pio_pin(sck);

        let mut cfg = PioConfig::default();
        cfg.use_program(&loaded, &[&sck]);
        cfg.set_out_pins(&[&mosi]);
        // 150 MHz sys / 3 = 50 MHz PIO, 2 cycles per bit = 25 MHz SPI.
        // Datasheet write cycle is 15 MHz; every ST7796 module takes far
        // more, and the bring-up risk is worth one divider bump if not.
        cfg.clock_divider = 3u32.to_fixed();
        // Autopull MSB-first with an 8-bit threshold: the SM shifts the top
        // 8 bits of each FIFO word. Byte-wide DMA writes are replicated
        // across the bus lanes (RP2040/RP2350 narrow-write rule), so a u8
        // stream lands its byte in the top lane exactly where OUT reads it.
        cfg.shift_out = ShiftConfig {
            threshold: 8,
            direction: ShiftDirection::Left,
            auto_fill: true,
        };
        cfg.fifo_join = FifoJoin::TxOnly;
        sm0.set_config(&cfg);
        sm0.set_pin_dirs(PioDirection::Out, &[&mosi, &sck]);
        sm0.set_enable(true);

        Self { sm: sm0, dma, dc, cs, rst, bl }
    }

    /// Block until the SM has shifted everything out, so DC may flip.
    async fn drain(&mut self) {
        while !self.sm.tx().empty() {
            embassy_futures::yield_now().await;
        }
        // The FIFO empties one word before the last bits leave the pin.
        Timer::after(Duration::from_micros(1)).await;
    }

    async fn push_byte(&mut self, b: u8) {
        self.sm.tx().wait_push((b as u32) << 24).await;
    }

    async fn cmd(&mut self, c: u8) {
        self.drain().await;
        self.dc.set_low();
        self.push_byte(c).await;
        self.drain().await;
        self.dc.set_high();
    }

    async fn data(&mut self, d: &[u8]) {
        for &b in d {
            self.push_byte(b).await;
        }
    }

    /// The init sequence proven by the rp2350_blink lcd_test bring-up.
    async fn init(&mut self) {
        self.rst.set_high();
        Timer::after(Duration::from_millis(10)).await;
        self.rst.set_low();
        Timer::after(Duration::from_millis(10)).await;
        self.rst.set_high();
        Timer::after(Duration::from_millis(120)).await;
        self.cmd(0x11).await; // SLPOUT
        Timer::after(Duration::from_millis(120)).await;
        self.cmd(0xF0).await; // CSCON: unlock command set 2
        self.data(&[0xC3]).await;
        self.cmd(0xF0).await;
        self.data(&[0x96]).await;
        self.cmd(0x36).await; // MADCTL: portrait, BGR order
        self.data(&[0x48]).await;
        self.cmd(0x3A).await; // COLMOD: RGB565
        self.data(&[0x55]).await;
        self.cmd(0x21).await; // INVON: IPS glass
        self.cmd(0xF0).await; // CSCON: lock again
        self.data(&[0x3C]).await;
        self.cmd(0xF0).await;
        self.data(&[0x69]).await;
        self.cmd(0x13).await; // NORON
        self.cmd(0x29).await; // DISPON
        Timer::after(Duration::from_millis(20)).await;
    }

    async fn window(&mut self, x: u16, y: u16, w: u16, h: u16) {
        let x1 = x + w - 1;
        let y1 = y + h - 1;
        self.cmd(0x2A).await; // CASET
        self.data(&[(x >> 8) as u8, x as u8, (x1 >> 8) as u8, x1 as u8]).await;
        self.cmd(0x2B).await; // RASET
        self.data(&[(y >> 8) as u8, y as u8, (y1 >> 8) as u8, y1 as u8]).await;
        self.cmd(0x2C).await; // RAMWR
    }

    /// One DMA burst of ready-to-wire bytes into the SM.
    async fn dma_bytes(&mut self, bytes: &[u8]) {
        self.sm.tx().dma_push(self.dma.reborrow(), bytes, false).await;
    }

    async fn fill_rect(&mut self, x: u16, y: u16, w: u16, h: u16, color: u16) {
        self.window(x, y, w, h).await;
        // One row's worth of pattern, DMA'd h times: no width*height buffer.
        let hi = (color >> 8) as u8;
        let lo = color as u8;
        let mut row = alloc::vec![0u8; w as usize * 2];
        for px in row.chunks_exact_mut(2) {
            px[0] = hi;
            px[1] = lo;
        }
        for _ in 0..h {
            self.dma_bytes(&row).await;
        }
    }

    /// Blit the 240x320 frame centred on the panel: byte-swap the RGB565
    /// words to wire order in place, then hand the whole thing to DMA.
    async fn blit(&mut self, frame: &mut DeviceFrame) {
        self.window(OFF_X, OFF_Y, DEV_W as u16, DEV_H as u16).await;
        for px in frame.px.iter_mut() {
            *px = px.to_be();
        }
        let bytes = unsafe {
            core::slice::from_raw_parts(frame.px.as_ptr() as *const u8, frame.px.len() * 2)
        };
        self.dma_bytes(bytes).await;
    }
}

extern crate alloc;

#[embassy_executor::task]
pub async fn task(mut lcd: Lcd) -> ! {
    lcd.init().await;
    lcd.fill_rect(0, 0, PANEL_W, PANEL_H, 0x0000).await;
    lcd.bl.set_high();
    defmt::debug!("ST7796 ready: PIO SPI at 25 MHz, DMA per frame");
    IDLE.signal(());
    loop {
        let mut frame = FRAME.wait().await;
        lcd.blit(&mut frame).await;
        drop(frame); // free the 150 KB before inviting the next one
        IDLE.signal(());
    }
}
