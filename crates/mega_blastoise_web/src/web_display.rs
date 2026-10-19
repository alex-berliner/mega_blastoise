use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Size},
    pixelcolor::BinaryColor,
    Pixel,
};
use mega_blastoise_core::OledFrameBuffer;

pub const OLED_W: usize = 128;
pub const OLED_H: usize = 64;

// Pixel colors for the OLED simulator
pub const ON: [u8; 4] = [57, 255, 20, 255];   // #39ff14 neon green
pub const OFF: [u8; 4] = [10, 25, 10, 255];    // near-black green

pub struct WasmDisplay {
    pub inner: OledFrameBuffer,
}

impl WasmDisplay {
    pub fn new() -> Self {
        Self { inner: OledFrameBuffer::new() }
    }

    pub fn to_rgba(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(OLED_W * OLED_H * 4);
        for row in &self.inner.fb {
            for &pixel in row {
                let c = if pixel { ON } else { OFF };
                out.extend_from_slice(&c);
            }
        }
        out
    }
}

impl DrawTarget for WasmDisplay {
    type Color = BinaryColor;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<BinaryColor>>,
    {
        self.inner.draw_iter(pixels)
    }
}

impl OriginDimensions for WasmDisplay {
    fn size(&self) -> Size {
        Size::new(OLED_W as u32, OLED_H as u32)
    }
}
