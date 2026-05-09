use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Size},
    pixelcolor::BinaryColor,
    Pixel,
};

pub const OLED_W: usize = 128;
pub const OLED_H: usize = 64;

// Pixel colors for the OLED simulator
pub const ON: [u8; 4] = [57, 255, 20, 255];   // #39ff14 neon green
pub const OFF: [u8; 4] = [10, 25, 10, 255];    // near-black green

pub struct WasmDisplay {
    pub fb: [[bool; OLED_W]; OLED_H],
}

impl WasmDisplay {
    pub fn new() -> Self {
        Self { fb: [[false; OLED_W]; OLED_H] }
    }

    pub fn clear_all(&mut self) {
        self.fb = [[false; OLED_W]; OLED_H];
    }

    pub fn to_rgba(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(OLED_W * OLED_H * 4);
        for row in &self.fb {
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
        for Pixel(coord, color) in pixels {
            if coord.x >= 0 && coord.y >= 0 {
                let x = coord.x as usize;
                let y = coord.y as usize;
                if x < OLED_W && y < OLED_H {
                    self.fb[y][x] = color.is_on();
                }
            }
        }
        Ok(())
    }
}

impl OriginDimensions for WasmDisplay {
    fn size(&self) -> Size {
        Size::new(OLED_W as u32, OLED_H as u32)
    }
}
