//! SSD1306 OLED driver, one display per player.
//!
//! P1: I2C0  (GP16 SDA, GP17 SCL)
//! P2: I2C1  (GP18 SDA, GP19 SCL)
//!
//! Displays use **blocking** I²C — the flush (~20 ms at 400 kHz) blocks the
//! task briefly but does not interfere with the battle loop's async prompts or
//! button scans since updates happen infrequently.
//!
//! Call [`send`] from `BattleEffects::on_event` to queue a display update.

use display_interface::WriteOnlyDataCommand;
use embassy_rp::Peri;
use embassy_rp::i2c::{Blocking, Config as I2cConfig, I2c};
use embassy_rp::peripherals::{I2C0, I2C1, PIN_16, PIN_17, PIN_18, PIN_19};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::{Baseline, Text},
};
use ssd1306::{mode::BufferedGraphicsMode, prelude::*, I2CDisplayInterface, Ssd1306};

// ── Command channel ───────────────────────────────────────────────────────────

pub enum OledCmd {
    /// HP changed; `pct` is 0–100.
    HpUpdate { player: u8, pct: u8 },
    /// New active Pokémon (UTF-8 name, up to 12 bytes).
    ActiveMon { player: u8, name: [u8; 12], len: u8 },
    /// A mon fainted.
    Faint { player: u8 },
    /// Battle ended.
    Win,
}

static CMD: Channel<CriticalSectionRawMutex, OledCmd, 8> = Channel::new();

pub fn send(cmd: OledCmd) {
    CMD.try_send(cmd).ok();
}

// ── Per-player state ──────────────────────────────────────────────────────────

struct PlayerState {
    hp_pct: u8,
    name: [u8; 12],
    name_len: u8,
    fainted: bool,
}

impl PlayerState {
    const fn new() -> Self {
        let mut name = [b' '; 12];
        name[0] = b'-'; name[1] = b'-'; name[2] = b'-';
        Self { hp_pct: 100, name, name_len: 3, fainted: false }
    }

    fn name_str(&self) -> &str {
        core::str::from_utf8(&self.name[..self.name_len as usize]).unwrap_or("?")
    }
}

// ── Rendering ─────────────────────────────────────────────────────────────────

fn render<DI>(
    disp: &mut Ssd1306<DI, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>>,
    header: &str,
    st: &PlayerState,
) where
    DI: display_interface::WriteOnlyDataCommand,
{
    let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    let fill = PrimitiveStyle::with_fill(BinaryColor::On);

    disp.clear(BinaryColor::Off).ok();

    Text::with_baseline(header, Point::new(0, 0), style, Baseline::Top)
        .draw(disp).ok();

    let mon_label = if st.fainted { "FAINTED" } else { st.name_str() };
    Text::with_baseline(mon_label, Point::new(0, 12), style, Baseline::Top)
        .draw(disp).ok();

    let bar_w = st.hp_pct as u32 * 128 / 100;
    if bar_w > 0 {
        Rectangle::new(Point::new(0, 26), Size::new(bar_w, 8))
            .into_styled(fill)
            .draw(disp).ok();
    }

    let mut buf = [0u8; 5];
    let pct_str = fmt_pct(st.hp_pct, &mut buf);
    Text::with_baseline(pct_str, Point::new(0, 36), style, Baseline::Top)
        .draw(disp).ok();

    disp.flush().ok();
}

fn fmt_pct(pct: u8, buf: &mut [u8; 5]) -> &str {
    let mut i = 0usize;
    if pct >= 100 { buf[i]=b'1'; i+=1; buf[i]=b'0'; i+=1; buf[i]=b'0'; i+=1; }
    else if pct >= 10 { buf[i]=b'0'+pct/10; i+=1; buf[i]=b'0'+pct%10; i+=1; }
    else { buf[i]=b'0'+pct; i+=1; }
    buf[i] = b'%'; i += 1;
    core::str::from_utf8(&buf[..i]).unwrap_or("?%")
}

// ── Embassy task ──────────────────────────────────────────────────────────────

#[embassy_executor::task]
pub async fn task(
    i2c0: Peri<'static, I2C0>,
    scl0: Peri<'static, PIN_17>,
    sda0: Peri<'static, PIN_16>,
    i2c1: Peri<'static, I2C1>,
    scl1: Peri<'static, PIN_19>,
    sda1: Peri<'static, PIN_18>,
) {
    let mut cfg = I2cConfig::default();
    cfg.frequency = 400_000;

    let bus0 = I2c::new_blocking(i2c0, scl0, sda0, cfg);
    let bus1 = I2c::new_blocking(i2c1, scl1, sda1, cfg);

    let mut disp0 = Ssd1306::new(
        I2CDisplayInterface::new(bus0),
        DisplaySize128x64,
        DisplayRotation::Rotate0,
    )
    .into_buffered_graphics_mode();

    let mut disp1 = Ssd1306::new(
        I2CDisplayInterface::new(bus1),
        DisplaySize128x64,
        DisplayRotation::Rotate0,
    )
    .into_buffered_graphics_mode();

    if disp0.init().is_err() || disp1.init().is_err() {
        defmt::warn!("OLED init failed — display task exiting");
        return;
    }

    let mut p1 = PlayerState::new();
    let mut p2 = PlayerState::new();

    render(&mut disp0, "P1: Red", &p1);
    render(&mut disp1, "P2: Blue", &p2);

    loop {
        match CMD.receive().await {
            OledCmd::HpUpdate { player, pct } => {
                if player == 1 { p1.hp_pct = pct; render(&mut disp0, "P1: Red", &p1); }
                else           { p2.hp_pct = pct; render(&mut disp1, "P2: Blue", &p2); }
            }
            OledCmd::ActiveMon { player, name, len } => {
                if player == 1 {
                    p1.name = name; p1.name_len = len; p1.fainted = false;
                    render(&mut disp0, "P1: Red", &p1);
                } else {
                    p2.name = name; p2.name_len = len; p2.fainted = false;
                    render(&mut disp1, "P2: Blue", &p2);
                }
            }
            OledCmd::Faint { player } => {
                if player == 1 { p1.fainted = true; render(&mut disp0, "P1: Red", &p1); }
                else           { p2.fainted = true; render(&mut disp1, "P2: Blue", &p2); }
            }
            OledCmd::Win => {
                let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
                disp0.clear(BinaryColor::Off).ok();
                Text::with_baseline("WINNER!", Point::zero(), style, Baseline::Top)
                    .draw(&mut disp0).ok();
                disp0.flush().ok();
                disp1.clear(BinaryColor::Off).ok();
                Text::with_baseline("GG!", Point::zero(), style, Baseline::Top)
                    .draw(&mut disp1).ok();
                disp1.flush().ok();
            }
        }
    }
}
