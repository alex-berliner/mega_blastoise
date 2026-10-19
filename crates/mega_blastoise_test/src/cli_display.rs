use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Size},
    pixelcolor::BinaryColor,
    Pixel,
};
use mega_blastoise_core::OledFrameBuffer;

/// 128×64 framebuffer that renders to stdout using Unicode half-block characters.
///
/// Wraps [`OledFrameBuffer`] and adds a terminal output method.
pub struct CliDisplay {
    inner: OledFrameBuffer,
}

impl CliDisplay {
    pub fn new() -> Self {
        Self { inner: OledFrameBuffer::new() }
    }

    /// Print the framebuffer to stdout as 32 terminal rows (2 pixel rows per line).
    ///
    /// Mapping: ▀ top-on/bottom-off, ▄ top-off/bottom-on, █ both-on, ' ' both-off.
    pub fn render(&self) {
        for row in 0..32usize {
            let mut line = String::with_capacity(128 + 1);
            for col in 0..128usize {
                let top = self.inner.fb[row * 2][col];
                let bottom = self.inner.fb[row * 2 + 1][col];
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
        self.inner.draw_iter(pixels)
    }
}

impl OriginDimensions for CliDisplay {
    fn size(&self) -> Size {
        Size::new(128, 64)
    }
}
