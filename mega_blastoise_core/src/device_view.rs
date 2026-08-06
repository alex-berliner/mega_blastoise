//! Composition for the single-screen device: two 240x160 halves on one
//! 240x320 panel, with the far seat's half rotated 180 degrees.
//!
//! Orientation is a runtime setting, not a build-time one, because which
//! arrangement actually feels best across a table is an open question — see
//! `architecture/09-single-screen.md`. Every mode here is reachable from the
//! debug toggles so it can be settled by playing rather than by arguing.

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Size},
    pixelcolor::{raw::RawU16, Rgb565},
    prelude::RawData,
    Pixel,
};

pub const DEV_W: u32 = 240;
pub const DEV_H: u32 = 320;

/// How the two halves are arranged on the panel.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Orientation {
    /// Tabletop head-to-head: far half rotated 180 so both seats read upright.
    /// This is the hardware arrangement.
    HeadToHead,
    /// Both halves upright — one person holding the device, or a phone.
    SameWay,
    /// One full-panel landscape view: attract mode, the gen picker, the lobby
    /// and the options menu. Rendered 320x240 and rotated onto the panel.
    Landscape,
}

impl Orientation {
    pub fn as_str(self) -> &'static str {
        match self {
            Orientation::HeadToHead => "head-to-head",
            Orientation::SameWay => "same-way",
            Orientation::Landscape => "landscape",
        }
    }
}

/// A 240x320 RGB565 framebuffer. Platforms flush it to a panel (firmware) or
/// convert it to RGBA for a canvas (web).
pub struct DeviceFrame {
    pub px: Vec<u16>,
}

impl Default for DeviceFrame {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceFrame {
    pub fn new() -> Self {
        Self { px: vec![0u16; (DEV_W * DEV_H) as usize] }
    }

    #[inline]
    pub fn set(&mut self, x: u32, y: u32, c: u16) {
        if x < DEV_W && y < DEV_H {
            self.px[(y * DEV_W + x) as usize] = c;
        }
    }

    /// RGBA8888, row-major — what `putImageData` wants.
    pub fn to_rgba(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.px.len() * 4);
        for &c in &self.px {
            let r = ((c >> 11) & 0x1F) as u8;
            let g = ((c >> 5) & 0x3F) as u8;
            let b = (c & 0x1F) as u8;
            out.push((r << 3) | (r >> 2));
            out.push((g << 2) | (g >> 4));
            out.push((b << 3) | (b >> 2));
            out.push(255);
        }
        out
    }
}

/// A transformed window onto a [`DeviceFrame`]: draws in local coordinates
/// and maps them to panel coordinates, optionally rotated.
pub struct Region<'a> {
    frame: &'a mut DeviceFrame,
    w: u32,
    h: u32,
    ox: i32,
    oy: i32,
    rot: Rot,
}

#[derive(Clone, Copy)]
enum Rot {
    None,
    Half,
    /// Local space is landscape (w x h) mapped onto a portrait panel.
    Quarter,
}

impl<'a> Region<'a> {
    /// A 240x160 half. `bottom` picks which half of the panel; `flip` rotates
    /// it 180 for the far seat.
    pub fn half(frame: &'a mut DeviceFrame, bottom: bool, flip: bool) -> Self {
        Self {
            frame,
            w: 240,
            h: 160,
            ox: 0,
            oy: if bottom { 160 } else { 0 },
            rot: if flip { Rot::Half } else { Rot::None },
        }
    }

    /// A full-panel landscape view: local space is 320 wide by 240 tall.
    pub fn landscape(frame: &'a mut DeviceFrame) -> Self {
        Self { frame, w: 320, h: 240, ox: 0, oy: 0, rot: Rot::Quarter }
    }

    #[inline]
    fn map(&self, x: i32, y: i32) -> Option<(u32, u32)> {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 {
            return None;
        }
        let (dx, dy) = match self.rot {
            Rot::None => (x, y),
            Rot::Half => (self.w as i32 - 1 - x, self.h as i32 - 1 - y),
            // Rotate the landscape view a quarter turn onto the portrait panel.
            Rot::Quarter => (self.h as i32 - 1 - y, x),
        };
        Some(((dx + self.ox) as u32, (dy + self.oy) as u32))
    }
}

impl DrawTarget for Region<'_> {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(p, c) in pixels {
            if let Some((dx, dy)) = self.map(p.x, p.y) {
                self.frame.set(dx, dy, RawU16::from(c).into_inner());
            }
        }
        Ok(())
    }
}

impl OriginDimensions for Region<'_> {
    fn size(&self) -> Size {
        Size::new(self.w, self.h)
    }
}

/// Standalone 240x160 half buffer, for rendering one seat in isolation
/// (screenshot tooling, tests).
pub struct HalfFrame {
    pub px: Vec<u16>,
    pub w: u32,
    pub h: u32,
}

impl HalfFrame {
    pub fn new(w: u32, h: u32) -> Self {
        Self { px: vec![0u16; (w * h) as usize], w, h }
    }

    pub fn to_rgba(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.px.len() * 4);
        for &c in &self.px {
            let r = ((c >> 11) & 0x1F) as u8;
            let g = ((c >> 5) & 0x3F) as u8;
            let b = (c & 0x1F) as u8;
            out.push((r << 3) | (r >> 2));
            out.push((g << 2) | (g >> 4));
            out.push((b << 3) | (b >> 2));
            out.push(255);
        }
        out
    }
}

impl DrawTarget for HalfFrame {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(p, c) in pixels {
            if p.x >= 0 && p.y >= 0 && (p.x as u32) < self.w && (p.y as u32) < self.h {
                self.px[(p.y as u32 * self.w + p.x as u32) as usize] =
                    RawU16::from(c).into_inner();
            }
        }
        Ok(())
    }
}

impl OriginDimensions for HalfFrame {
    fn size(&self) -> Size {
        Size::new(self.w, self.h)
    }
}
