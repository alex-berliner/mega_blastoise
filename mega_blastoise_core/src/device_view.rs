//! Composition for the single-screen device: two 240x160 halves on one
//! 240x320 panel, with the far seat's half rotated 180 degrees.
//!
//! There is exactly one arrangement. The console sits flat on a table between
//! two players facing each other, and it is never picked up or turned, so
//! every screen — menus included — is drawn as two head-to-head halves. See
//! `architecture/09-single-screen.md`.

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

/// How far the divider reaches into each half. Every private screen keeps its
/// content below this in local coordinates, because the divider is drawn over
/// the composed panel and does not care what a half put there.
pub const DIVIDER_REACH: u32 = 11;

/// Draw the seam between the two seats: the pokeball's band, where the two
/// halves' red frames meet, with the button sitting on the middle of it.
///
/// On the real panel there is no bezel in the middle, so without a drawn
/// divider the two halves read as one confusing screen. The red comes from
/// each half's own frame rather than from here, which is what makes the seam
/// read the same way from either seat: red, the band, and the button.
pub fn draw_split_divider(frame: &mut DeviceFrame) {
    let mid = (DEV_H / 2) as i32;
    let ink = rgb565(crate::display_color::C_INK);
    let button = rgb565(crate::display_color::C_BOX);
    let reach = DIVIDER_REACH as i32;

    for x in 0..DEV_W as i32 {
        for dy in -2..2 {
            frame.set(x as u32, (mid + dy) as u32, ink);
        }
    }

    // The button, straddling the seam so neither seat owns it. Both of its
    // edges are antialiased by 4x4 coverage sampling: a hard threshold on a
    // circle this small leaves visible stair-steps, worst where the white
    // meets the black.
    let cx = (DEV_W / 2) as i32;
    for dy in -reach..=reach {
        for dx in -reach..=reach {
            let (inner, outer) = coverage(dx, dy);
            if outer == 0 {
                continue;
            }
            let x = (cx + dx) as u32;
            let y = (mid + dy) as u32;
            let under = frame.px[(y * DEV_W + x) as usize];
            // Ring over whatever is there, then the button over the ring.
            let c = blend(under, ink, outer);
            frame.set(x, y, blend(c, button, inner));
        }
    }
}

/// Fractional coverage of one pixel by the button's white centre and by its
/// black ring, each 0..=16, from a 4x4 grid of subsamples.
fn coverage(dx: i32, dy: i32) -> (u32, u32) {
    // Eighths of a pixel, so the subsample centres land on exact integers.
    const R_IN: i32 = 52; // 6.5 px
    const R_OUT: i32 = 84; // 10.5 px
    let (mut inner, mut outer) = (0, 0);
    for sy in 0..4 {
        for sx in 0..4 {
            let px = dx * 8 - 4 + 2 * sx + 1;
            let py = dy * 8 - 4 + 2 * sy + 1;
            let r2 = px * px + py * py;
            if r2 <= R_IN * R_IN {
                inner += 1;
            }
            if r2 <= R_OUT * R_OUT {
                outer += 1;
            }
        }
    }
    (inner, outer)
}

/// Mix `over` into `under` at `a`/16, in RGB565 without unpacking to 8 bits.
fn blend(under: u16, over: u16, a: u32) -> u16 {
    if a == 0 {
        return under;
    }
    if a >= 16 {
        return over;
    }
    let mix = |u: u32, o: u32| ((u * (16 - a) + o * a) / 16) as u16;
    let r = mix((under >> 11) as u32 & 0x1F, (over >> 11) as u32 & 0x1F);
    let g = mix((under >> 5) as u32 & 0x3F, (over >> 5) as u32 & 0x3F);
    let b = mix(under as u32 & 0x1F, over as u32 & 0x1F);
    (r << 11) | (g << 5) | b
}

/// Where the shared scene's two mons stand: a band across the seam, one seat
/// per side.
pub const BAND_TOP: i32 = 116;
pub const BAND_H: u32 = 88;
pub const BAND_W: u32 = 112;

/// Draw both seats' mons into that band, each upright to its own seat.
///
/// This is composed rather than drawn per half on purpose. A half can only
/// touch its own 160 rows, so a per-half mon is always pushed onto one side of
/// the seam and the pair ends up staggered; drawing the band over the composed
/// panel is what puts them at the same height, side by side, the way two
/// players facing each other across a table expect to see them.
pub fn draw_scene_mons(
    frame: &mut DeviceFrame,
    p1: &str,
    p2: &str,
    bob1: bool,
    bob2: bool,
    shake: [(i32, i32); 2],
) {
    for (i, (name, bob, flip)) in [(p1, bob1, false), (p2, bob2, true)].into_iter().enumerate() {
        // An attack effect rocks the mon it lands on. The mons are drawn
        // before the effect, so the offset arrives here rather than being
        // applied by the effect itself.
        let (sx, sy) = shake[i];
        let ox = if flip { (DEV_W - BAND_W) as i32 - 4 } else { 4 } + sx;
        let mut r = Region::band(frame, ox, BAND_TOP + sy, BAND_W, BAND_H, flip);
        crate::display_color::draw_field_mon(&mut r, name, bob, BAND_W, BAND_H);
    }
}

fn rgb565(c: Rgb565) -> u16 {
    RawU16::from(c).into_inner()
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

    /// An arbitrary window on the panel, which may straddle the seam. Used by
    /// the shared scene, where the two mons have to sit at the same height on
    /// the panel and a half — able to draw only inside its own 160 rows —
    /// cannot put them there.
    pub fn band(frame: &'a mut DeviceFrame, ox: i32, oy: i32, w: u32, h: u32, flip: bool) -> Self {
        Self { frame, w, h, ox, oy, rot: if flip { Rot::Half } else { Rot::None } }
    }

    #[inline]
    fn map(&self, x: i32, y: i32) -> Option<(u32, u32)> {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 {
            return None;
        }
        let (dx, dy) = match self.rot {
            Rot::None => (x, y),
            Rot::Half => (self.w as i32 - 1 - x, self.h as i32 - 1 - y),
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
