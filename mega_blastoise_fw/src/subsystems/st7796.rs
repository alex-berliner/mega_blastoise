//! ST7796 3.5" 320x480 SPI panel — the stripboard v2 display.
//!
//! Wiring (BUILD.md §10): BL GP5, DC GP6, RST GP7, MOSI GP8, SCK GP9,
//! CS GP10. GP8/GP9 fit neither hardware SPI block; this is the PIO bus
//! BUILD.md §10 promises. A two-instruction sideset program clocks mode-0
//! SPI at 25 MHz and a single DMA transfer feeds it each frame, so a full
//! 320x480 blit is ~100 ms — against well over a second for the bit-banged
//! bring-up driver this replaced (the visible wipe down the panel).
//!
//! The core renders a 320x480 [`DeviceFrame`] (the single-screen design's
//! native size, shared with the web build — zero drift), which matches this
//! panel one-to-one; the centring offsets below are zero and exist so the
//! math stays honest if either size ever moves again.
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

const PANEL_W: u16 = 320;
const PANEL_H: u16 = 480;
const OFF_X: u16 = (PANEL_W - DEV_W as u16) / 2; // 0
const OFF_Y: u16 = (PANEL_H - DEV_H as u16) / 2; // 0

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
        // 150 MHz sys / 2 = 75 MHz PIO, 2 cycles per bit = 37.5 MHz SPI.
        // Datasheet write cycle is 15 MHz, but ST7796 modules routinely run
        // 40+ over short wires (TFT_eSPI ships 55). Proven at 25 (divider
        // 3) first; raise the divider if the panel ever shows tearing or
        // shifted pixels.
        cfg.clock_divider = 2u32.to_fixed();
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
        // MADCTL: portrait, BGR order, rotated 180 (0x88, not 0x48): as
        // mounted, the panel's native bottom faces player 2's band, and the
        // frame's bottom half is seat 1 — the rotation puts P1's half on
        // P1's edge of the board.
        self.cmd(0x36).await;
        self.data(&[0x88]).await;
        self.cmd(0x3A).await; // COLMOD: RGB565
        self.data(&[0x55]).await;
        // INVOFF, not the usual IPS INVON: webcam-vs-web comparison showed
        // this module renders true colors uninverted (cream came out navy,
        // red came out cyan with 0x21).
        self.cmd(0x20).await;
        // Power, VCOM and gamma. Without these the panel runs its reset
        // defaults, which read washed out (the webcam-vs-web red check).
        // Values are the widely shipped ST7796 set (TFT_eSPI / LovyanGFX).
        self.cmd(0xB4).await; // display inversion control: 1-dot
        self.data(&[0x01]).await;
        self.cmd(0xC2).await; // power control 3
        self.data(&[0xA7]).await;
        self.cmd(0xC5).await; // VCOM
        self.data(&[0x18]).await;
        self.cmd(0xE8).await; // display output ctrl adjust
        self.data(&[0x40, 0x8A, 0x00, 0x00, 0x29, 0x19, 0xA5, 0x33]).await;
        self.cmd(0xE0).await; // positive gamma
        self.data(&[
            0xF0, 0x09, 0x0B, 0x06, 0x04, 0x15, 0x2F, 0x54, 0x42, 0x3C, 0x17, 0x14,
            0x18, 0x1B,
        ])
        .await;
        self.cmd(0xE1).await; // negative gamma
        self.data(&[
            0xE0, 0x09, 0x0B, 0x06, 0x04, 0x03, 0x2B, 0x43, 0x42, 0x3B, 0x16, 0x14,
            0x17, 0x1B,
        ])
        .await;
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

    /// Send the whole frame as one window + one DMA burst.
    async fn blit_full(&mut self, px: &[u16]) {
        self.window(OFF_X, OFF_Y, DEV_W as u16, DEV_H as u16).await;
        let bytes =
            unsafe { core::slice::from_raw_parts(px.as_ptr() as *const u8, px.len() * 2) };
        self.dma_bytes(bytes).await;
    }

    /// Dirty-rectangle blit: byte-swap to wire order in place, hash the
    /// frame in 16x16 tiles against `hashes`, and send only the spans of
    /// tiles that changed — the panel's GRAM keeps everything else. A
    /// cursor move costs a couple of tiles (~1 ms) instead of a full 65 ms
    /// frame; a mostly-new frame falls back to one full-frame DMA rather
    /// than paying window overhead hundreds of times.
    async fn blit(
        &mut self,
        frame: &mut DeviceFrame,
        hashes: &mut [u32; N_TILES],
        bounce: &mut [u8],
    ) {
        for px in frame.px.iter_mut() {
            *px = px.to_be();
        }
        let px = &frame.px;

        let mut dirty = [false; N_TILES];
        let mut n_dirty = 0usize;
        for ty in 0..TILES_Y {
            for tx in 0..TILES_X {
                let h = tile_hash(px, tx, ty);
                let i = ty * TILES_X + tx;
                if h != hashes[i] {
                    hashes[i] = h;
                    dirty[i] = true;
                    n_dirty += 1;
                }
            }
        }
        if n_dirty == 0 {
            return;
        }
        // Past ~60% dirty, one burst beats many windows.
        if n_dirty * 10 > N_TILES * 6 {
            self.blit_full(px).await;
            return;
        }

        for ty in 0..TILES_Y {
            let mut tx = 0;
            while tx < TILES_X {
                if !dirty[ty * TILES_X + tx] {
                    tx += 1;
                    continue;
                }
                let start = tx;
                while tx < TILES_X && dirty[ty * TILES_X + tx] {
                    tx += 1;
                }
                let x0 = start * TILE;
                let w = (tx - start) * TILE;
                let y0 = ty * TILE;
                self.window(x0 as u16, y0 as u16, w as u16, TILE as u16).await;
                // The span's rows are strided in the framebuffer; gather
                // them into the bounce buffer so one DMA covers the span.
                let bytes = w * 2;
                for row in 0..TILE {
                    let src = (y0 + row) * DEV_W as usize + x0;
                    let s = unsafe {
                        core::slice::from_raw_parts(px[src..src + w].as_ptr() as *const u8, bytes)
                    };
                    bounce[row * bytes..(row + 1) * bytes].copy_from_slice(s);
                }
                self.dma_bytes(&bounce[..TILE * bytes]).await;
            }
        }
    }
}

// ── Dirty-tile bookkeeping ───────────────────────────────────────────────────

const TILE: usize = 16;
const TILES_X: usize = DEV_W as usize / TILE; // 20
const TILES_Y: usize = DEV_H as usize / TILE; // 30
const N_TILES: usize = TILES_X * TILES_Y; // 600 — 2.4 KB of hashes

/// FNV-1a over one tile's (wire-order) pixels.
fn tile_hash(px: &[u16], tx: usize, ty: usize) -> u32 {
    let mut h: u32 = 2166136261;
    for row in 0..TILE {
        let base = (ty * TILE + row) * DEV_W as usize + tx * TILE;
        for &p in &px[base..base + TILE] {
            h = (h ^ p as u32).wrapping_mul(16777619);
        }
    }
    h
}

extern crate alloc;

#[embassy_executor::task]
pub async fn task(mut lcd: Lcd) -> ! {
    lcd.init().await;
    lcd.fill_rect(0, 0, PANEL_W, PANEL_H, 0x0000).await;
    lcd.bl.set_high();
    defmt::debug!("ST7796 ready: PIO SPI + DMA, dirty-tile blits");
    // Zeroed hashes make the first frame all-dirty: a natural full paint.
    let mut hashes = alloc::boxed::Box::new([0u32; N_TILES]);
    let mut bounce = alloc::vec![0u8; DEV_W as usize * TILE * 2];
    loop {
        let mut frame = FRAME.wait().await;
        lcd.blit(&mut frame, &mut hashes, &mut bounce).await;
        drop(frame); // free the 300 KB before releasing the flag
        crate::board_v2::FRAME_FREE.store(true, core::sync::atomic::Ordering::Release);
        // A present landed while we were blitting: pulse the runner so it
        // re-presents from fresh state now that the frame slot is free.
        if crate::board_v2::NEED_REPAINT.swap(false, core::sync::atomic::Ordering::AcqRel) {
            let _ = crate::board_v2::INPUT
                .try_send(mega_blastoise_core::device_runner::DeviceEvent::Pulse);
        }
    }
}
