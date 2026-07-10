//! SSD1306 OLED driver, one display per player.
//!
//! Wiring (default = partner PCB; `breadboard` = the hand-wired rig):
//!   PCB:        P1 on I2C1 (GP2 SDA, GP3 SCL);  P2 on I2C0 (GP4 SDA, GP5 SCL)
//!   breadboard: P1 on I2C0 (GP16 SDA, GP17 SCL); P2 on I2C1 (GP18 SDA, GP19 SCL)
//!
//! NOTE (PCB): the schematic net labels show SCL on GP2/GP4 and SDA on
//! GP3/GP5, but the RP2040's I2C function map is fixed (even GPIO = SDA, odd
//! = SCL), so this driver uses the only hardware-valid orientation. If the
//! copper really is crossed the displays won't ACK — check with i2c_scan.
//!
//! Uses embassy async I²C — the 20 ms framebuffer flush is spread across
//! ~64 small executor yields instead of blocking the task solid.
//!
//! Call [`send`] from `BattleEffects::on_event` to queue a display update.
//!
//! All screen *decisions* live in `mega_blastoise_core::oled_ctl` (shared
//! with the web client); this module only owns the RP2040 plumbing: the
//! command channel, the panels, flush synchronization, and the shadow
//! framebuffer used for RTT/USB dumps.

extern crate alloc;

use embassy_rp::bind_interrupts;
use embassy_rp::i2c::{Config as I2cConfig, I2c, InterruptHandler};
use embassy_rp::Peri;
use embassy_rp::peripherals::{I2C0, I2C1};

/// Per-board bus/pin assignment (see module doc).
#[cfg(feature = "breadboard")]
mod wiring {
    pub use embassy_rp::peripherals::{
        I2C0 as P1Bus, I2C1 as P2Bus, PIN_16 as P1Sda, PIN_17 as P1Scl, PIN_18 as P2Sda,
        PIN_19 as P2Scl,
    };
}
#[cfg(not(feature = "breadboard"))]
mod wiring {
    pub use embassy_rp::peripherals::{
        I2C0 as P2Bus, I2C1 as P1Bus, PIN_2 as P1Sda, PIN_3 as P1Scl, PIN_4 as P2Sda,
        PIN_5 as P2Scl,
    };
}
use embassy_time::{with_timeout, Duration, Timer};
use embassy_futures::join::join;
use embassy_futures::select::{select, Either};
use core::cell::{Cell, RefCell};
use core::sync::atomic::{AtomicBool, Ordering};
use embassy_sync::{blocking_mutex::{raw::CriticalSectionRawMutex, Mutex as BlockingMutex}, channel::Channel, signal::Signal};
use embedded_graphics::{draw_target::DrawTarget, geometry::{OriginDimensions, Size}, pixelcolor::BinaryColor, Pixel};
use mega_blastoise_core::{render_screen, OledController, BOB_TICK_MS};
pub use mega_blastoise_core::OledCmd;
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306Async};

bind_interrupts!(struct OledIrqs {
    I2C0_IRQ => InterruptHandler<I2C0>;
    I2C1_IRQ => InterruptHandler<I2C1>;
});

// ── Shadow framebuffer ────────────────────────────────────────────────────────
//
// Packed 128×64 monochrome buffer: 16 bytes per row (128 bits), row-major, MSB = leftmost pixel.
// Updated after every flush so :oled can snapshot without touching the ssd1306 driver internals.

struct Shadow([[u8; 16]; 64]);

impl Shadow {
    const fn new() -> Self { Self([[0u8; 16]; 64]) }
}

impl DrawTarget for Shadow {
    type Color = BinaryColor;
    type Error = core::convert::Infallible;
    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where I: IntoIterator<Item = Pixel<BinaryColor>> {
        for Pixel(coord, color) in pixels {
            if coord.x >= 0 && coord.y >= 0 {
                let (x, y) = (coord.x as usize, coord.y as usize);
                if x < 128 && y < 64 {
                    let b = &mut self.0[y][x >> 3];
                    let bit = 0x80u8 >> (x & 7);
                    if color.is_on() { *b |= bit; } else { *b &= !bit; }
                }
            }
        }
        Ok(())
    }
}

impl OriginDimensions for Shadow {
    fn size(&self) -> Size { Size::new(128, 64) }
}

static P1_SHADOW: BlockingMutex<CriticalSectionRawMutex, RefCell<[[u8; 16]; 64]>> =
    BlockingMutex::new(RefCell::new([[0u8; 16]; 64]));
static P2_SHADOW: BlockingMutex<CriticalSectionRawMutex, RefCell<[[u8; 16]; 64]>> =
    BlockingMutex::new(RefCell::new([[0u8; 16]; 64]));

/// When true, every framebuffer change is rendered over RTT (defmt).
///
/// Boot default is set by the `oledlog` cargo feature (on with the feature,
/// off without). The `:oledlog on|off` USB command flips it at runtime
/// either way (works in the lobby and during a battle).
static OLED_DUMP: AtomicBool = AtomicBool::new(cfg!(feature = "oledlog"));

pub fn set_oled_dump(on: bool) {
    OLED_DUMP.store(on, Ordering::Relaxed);
}

pub fn oled_dump_enabled() -> bool {
    OLED_DUMP.load(Ordering::Relaxed)
}

/// When true, every framebuffer change is also printed over USB as the
/// `:oled`-style half-block art. Toggled at runtime via `:oled auto on|off`;
/// off by default. The OLED task only raises a notification here (atomic +
/// signal, non-blocking, so it can't stagger panel flushes the way the old
/// pre-flush RTT dump did); the USB side picks it up in `read_line` and does
/// the actual printing (see `UsbBattleInput::write_oled_dump`).
static USB_AUTO_DUMP: AtomicBool = AtomicBool::new(false);
/// Bitmask of players whose framebuffer changed since the last USB dump
/// (bit 0 = p1, bit 1 = p2). Accumulates so bursts coalesce into one dump.
/// Mutex+Cell instead of an atomic: thumbv6m has no RMW atomics (no fetch_or).
static FB_CHANGED: BlockingMutex<CriticalSectionRawMutex, Cell<u8>> =
    BlockingMutex::new(Cell::new(0));
static FB_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

pub fn set_usb_auto_dump(on: bool) {
    USB_AUTO_DUMP.store(on, Ordering::Relaxed);
}

/// Wait until a framebuffer changes while `:oled auto` is on. Returns the
/// changed-player bitmask (bit 0 = p1, bit 1 = p2). Never resolves while
/// auto-dump is off.
pub async fn wait_fb_change() -> u8 {
    loop {
        FB_SIGNAL.wait().await;
        let mask = FB_CHANGED.lock(|m| m.replace(0));
        if mask != 0 {
            return mask;
        }
    }
}

fn store_shadow(player: u8, s: &Shadow) {
    if player == 1 { P1_SHADOW.lock(|fb| *fb.borrow_mut() = s.0); }
    else           { P2_SHADOW.lock(|fb| *fb.borrow_mut() = s.0); }
    if USB_AUTO_DUMP.load(Ordering::Relaxed) {
        let bit = if player == 1 { 1 } else { 2 };
        FB_CHANGED.lock(|m| m.set(m.get() | bit));
        FB_SIGNAL.signal(());
    }
}

/// Emit the framebuffer over RTT for offline capture — but ONLY when
/// `oledlog` is enabled, and ONLY *after* the display has flushed.
///
/// The ~2 KB hex format + defmt write is heavy. If it sits between render
/// and `flush()` it stalls that panel's update, and since each panel hits
/// the stall at a different time the two displays visibly desync (very
/// noticeable with `oledlog` on, gone with it off). Keeping the dump strictly
/// after the flush means the old frame stays up through the dump and both
/// panels switch together; `oledlog` no longer perturbs display timing.
/// No-op when the dump is disabled.
fn dump_rtt(player: u8, s: &Shadow) {
    if OLED_DUMP.load(Ordering::Relaxed) {
        dump_fb_rtt(player, &s.0);
    }
}

/// Emit the framebuffer as ONE compact defmt message: the 1024 packed bytes
/// as a hex string (`oledfb|pN|<2048 hex chars>`).
///
/// The earlier per-row half-block dump was 33 defmt messages (~4 KB) per
/// render — under battle render rates that floods RTT and either wedges the
/// executor (blocking defmt) or drops body rows (non-blocking). A single
/// atomic ~2 KB message survives bursts, so every frame of a whole game can
/// be captured. The host side reconstructs the screen offline (see
/// scripts/oled_render.py).
fn dump_fb_rtt(player: u8, fb: &[[u8; 16]; 64]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut hex = [0u8; 1024 * 2];
    let mut i = 0usize;
    for row in fb {
        for &b in row {
            hex[i] = HEX[(b >> 4) as usize];
            hex[i + 1] = HEX[(b & 0xf) as usize];
            i += 2;
        }
    }
    let s = core::str::from_utf8(&hex).unwrap_or("?");
    defmt::info!("oledfb|p{}|{=str}", player, s);
}

/// Snapshot the current OLED framebuffer for `player` (1 or 2).
/// Packed row-major: bit 7 of byte [y][0] is pixel (0, y).
pub fn read_shadow_fb(player: u8) -> [[u8; 16]; 64] {
    if player == 1 { P1_SHADOW.lock(|fb| *fb.borrow()) }
    else           { P2_SHADOW.lock(|fb| *fb.borrow()) }
}

// ── Command channel ───────────────────────────────────────────────────────────

static CMD: Channel<CriticalSectionRawMutex, OledCmd, 8> = Channel::new();
static READY: AtomicBool = AtomicBool::new(false);

pub fn send(cmd: OledCmd) {
    if !READY.load(Ordering::Relaxed) {
        return;
    }
    if CMD.try_send(cmd).is_err() {
        defmt::warn!("oled: channel full, cmd dropped");
    }
}

// ── Embassy task ──────────────────────────────────────────────────────────────

#[embassy_executor::task]
pub async fn task(
    p1_bus: Peri<'static, wiring::P1Bus>,
    p1_scl: Peri<'static, wiring::P1Scl>,
    p1_sda: Peri<'static, wiring::P1Sda>,
    p2_bus: Peri<'static, wiring::P2Bus>,
    p2_scl: Peri<'static, wiring::P2Scl>,
    p2_sda: Peri<'static, wiring::P2Sda>,
) {
    let mut cfg = I2cConfig::default();
    cfg.frequency = 100_000;

    let bus0 = I2c::new_async(p1_bus, p1_scl, p1_sda, OledIrqs, cfg);
    let bus1 = I2c::new_async(p2_bus, p2_scl, p2_sda, OledIrqs, cfg);

    let mut disp0 = Ssd1306Async::new(
        I2CDisplayInterface::new(bus0),
        DisplaySize128x64,
        DisplayRotation::Rotate0,
    )
    .into_buffered_graphics_mode();

    let mut disp1 = Ssd1306Async::new(
        I2CDisplayInterface::new(bus1),
        DisplaySize128x64,
        DisplayRotation::Rotate0,
    )
    .into_buffered_graphics_mode();

    // Bound init: embassy async I²C blocks forever waiting for an ACK from an
    // absent display, which would wedge this whole task (and starve the other,
    // present display). A timeout turns "no device on this bus" into a clean
    // !ok instead of a hang.
    const INIT_TIMEOUT: Duration = Duration::from_millis(500);
    let p1_ok = matches!(with_timeout(INIT_TIMEOUT, disp0.init()).await, Ok(Ok(())));
    let p2_ok = matches!(with_timeout(INIT_TIMEOUT, disp1.init()).await, Ok(Ok(())));
    if !p1_ok { defmt::warn!("OLED P1 init failed"); }
    if !p2_ok { defmt::warn!("OLED P2 init failed"); }
    if !p1_ok && !p2_ok {
        defmt::warn!("OLED: no displays found — display task exiting");
        return;
    }

    READY.store(true, Ordering::Relaxed);

    // The shared state machine decides every screen; it boots showing the
    // idle lobby (same as the web client).
    let mut ctl = OledController::new();
    let mut s1 = Shadow::new();
    let mut s2 = Shadow::new();

    // Initial draw, then one render/flush pass per command.
    let mut redraw_p1 = p1_ok;
    let mut redraw_p2 = p2_ok;
    loop {
        if redraw_p1 {
            let scr = ctl.screen(1);
            render_screen(&mut disp0, &scr);
            render_screen(&mut s1, &scr);
            store_shadow(1, &s1);
        }
        if redraw_p2 {
            let scr = ctl.screen(2);
            render_screen(&mut disp1, &scr);
            render_screen(&mut s2, &scr);
            store_shadow(2, &s2);
        }
        // I2C0 and I2C1 are independent peripherals — when both panels
        // changed (broadcast flash, win screen) flush them concurrently.
        // Flushing one fully before the other staggers the panels by a whole
        // flush (~tens of ms), which is plainly visible.
        if redraw_p1 && redraw_p2 {
            let _ = join(disp0.flush(), disp1.flush()).await;
        } else if redraw_p1 {
            disp0.flush().await.ok();
        } else if redraw_p2 {
            disp1.flush().await.ok();
        }
        // RTT dump strictly after the flushes, so it can't stagger them.
        if redraw_p1 { dump_rtt(1, &s1); }
        if redraw_p2 { dump_rtt(2, &s2); }

        // Wake on the next command, or on the bob tick to hop the battle
        // sprites — each player's rate scales with their mon's Speed stat.
        let redraw = match select(CMD.receive(), Timer::after_millis(BOB_TICK_MS as u64)).await {
            Either::First(cmd) => ctl.apply(cmd),
            Either::Second(()) => ctl.tick_bob(BOB_TICK_MS),
        };
        redraw_p1 = redraw.includes(1) && p1_ok;
        redraw_p2 = redraw.includes(2) && p2_ok;
    }
}
