use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Size},
    pixelcolor::BinaryColor,
    Pixel,
};

/// 128×64 framebuffer that renders to stdout using Unicode half-block characters.
///
/// Implements `DrawTarget<Color = BinaryColor>`, so any embedded-graphics
/// primitive or text that targets the real `Ssd1306` can be redirected here
/// during host-side development without hardware.
pub struct CliDisplay {
    fb: [[bool; 128]; 64],
}

impl CliDisplay {
    pub fn new() -> Self {
        Self { fb: [[false; 128]; 64] }
    }

    /// Print the framebuffer to stdout as 32 terminal rows (2 pixel rows per line).
    ///
    /// Mapping: ▀ top-on/bottom-off, ▄ top-off/bottom-on, █ both-on, ' ' both-off.
    pub fn render(&self) {
        for row in 0..32usize {
            let mut line = String::with_capacity(128 + 1);
            for col in 0..128usize {
                let top = self.fb[row * 2][col];
                let bottom = self.fb[row * 2 + 1][col];
                line.push(match (top, bottom) {
                    (false, false) => ' ',
                    (true, false) => '▀',
                    (false, true) => '▄',
                    (true, true) => '█',
                });
            }
            println!("{line}");
        }
    }
}

impl Default for CliDisplay {
    fn default() -> Self {
        Self::new()
    }
}

impl DrawTarget for CliDisplay {
    type Color = BinaryColor;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<BinaryColor>>,
    {
        for Pixel(coord, color) in pixels {
            if coord.x >= 0 && coord.y >= 0 {
                let x = coord.x as usize;
                let y = coord.y as usize;
                if x < 128 && y < 64 {
                    self.fb[y][x] = color.is_on();
                }
            }
        }
        Ok(())
    }
}

impl OriginDimensions for CliDisplay {
    fn size(&self) -> Size {
        Size::new(128, 64)
    }
}
