//! Minimal async SH1106 driver for the 1.3" I2C OLED panels (PCB build).
//!
//! The 1.3" modules use an SH1106 controller, not the SSD1306 found on the
//! 0.96" panels, and the two differ exactly where a driver hurts: the SH1106
//! has no horizontal addressing mode (the ssd1306 crate's one-burst flush
//! lands in a single page, leaving the rest of the panel showing
//! uninitialized RAM as noise), and its RAM is 132 columns wide with the 128
//! visible ones starting at column 2. Hence: page-mode flush, one burst per
//! page, column offset 2.
//!
//! The API mirrors the slice of the ssd1306 crate's buffered-graphics mode
//! that `subsystems::oled` uses — `DrawTarget` + `init()` + `flush()` — so
//! the display task body is driver-agnostic.

use embassy_rp::i2c::{Async, Error, I2c, Instance};
use embedded_hal_async::i2c::I2c as _;
use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Size},
    pixelcolor::BinaryColor,
    Pixel,
};

const ADDR: u8 = 0x3C;
/// SH1106 RAM is 132 columns; the 128 visible ones start at column 2.
const COL_OFFSET: u8 = 2;

pub struct Sh1106<'d, T: Instance> {
    bus: I2c<'d, T, Async>,
    /// Page-packed framebuffer in the chip's native layout: byte index
    /// `page * 128 + x`, bit `y % 8` (bit 0 = top row of the page).
    fb: [u8; 128 * 8],
}

impl<'d, T: Instance> Sh1106<'d, T> {
    pub fn new(bus: I2c<'d, T, Async>) -> Self {
        Self { bus, fb: [0; 128 * 8] }
    }

    async fn cmd(&mut self, cmds: &[u8]) -> Result<(), Error> {
        let mut buf = [0u8; 4];
        buf[0] = 0x00; // control byte: command stream
        buf[1..1 + cmds.len()].copy_from_slice(cmds);
        self.bus.write(ADDR, &buf[..1 + cmds.len()]).await
    }

    /// SH1106 datasheet init. A1/C8 match the orientation the ssd1306
    /// crate's `Rotate0` used, so the image keeps the same way up.
    pub async fn init(&mut self) -> Result<(), Error> {
        for c in [
            &[0xAE][..],   // display off
            &[0xD5, 0x80], // clock divide ratio
            &[0xA8, 0x3F], // multiplex 64
            &[0xD3, 0x00], // display offset 0
            &[0x40],       // start line 0
            &[0xAD, 0x8B], // charge pump on
            &[0xA1],       // segment remap
            &[0xC8],       // COM scan reversed
            &[0xDA, 0x12], // COM pins alternative
            &[0x81, 0xCF], // contrast
            &[0xD9, 0xF1], // precharge
            &[0xDB, 0x40], // VCOM deselect
            &[0xA4],       // show RAM contents
            &[0xA6],       // normal (non-inverted)
            &[0xAF],       // display on
        ] {
            self.cmd(c).await?;
        }
        Ok(())
    }

    pub async fn flush(&mut self) -> Result<(), Error> {
        for page in 0..8u8 {
            self.cmd(&[0xB0 | page, COL_OFFSET & 0x0F, 0x10 | (COL_OFFSET >> 4)]).await?;
            let mut buf = [0u8; 1 + 128];
            buf[0] = 0x40; // control byte: data stream
            let start = page as usize * 128;
            buf[1..].copy_from_slice(&self.fb[start..start + 128]);
            self.bus.write(ADDR, &buf).await?;
        }
        Ok(())
    }
}

impl<T: Instance> DrawTarget for Sh1106<'_, T> {
    type Color = BinaryColor;
    type Error = core::convert::Infallible;
    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<BinaryColor>>,
    {
        for Pixel(coord, color) in pixels {
            if coord.x >= 0 && coord.y >= 0 {
                let (x, y) = (coord.x as usize, coord.y as usize);
                if x < 128 && y < 64 {
                    let byte = &mut self.fb[(y >> 3) * 128 + x];
                    let bit = 1u8 << (y & 7);
                    if color.is_on() { *byte |= bit; } else { *byte &= !bit; }
                }
            }
        }
        Ok(())
    }
}

impl<T: Instance> OriginDimensions for Sh1106<'_, T> {
    fn size(&self) -> Size {
        Size::new(128, 64)
    }
}
