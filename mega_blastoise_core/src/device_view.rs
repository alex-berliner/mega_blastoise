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

pub const DEV_W: u32 = 320;
pub const DEV_H: u32 = 480;

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

/// Debug overlay: which physical buttons each seat is holding, stamped over
/// the finished frame at half opacity so it can sit on top of any screen.
///
/// `masks[0]` is player 1, `masks[1]` player 2; bit order is the board's
/// fixed key order 0=Up 1=Down 2=Left 3=Right 4=A 5=B 6=Info 7=Menu. Each
/// seat's cluster is drawn in its own corner, rotated with the seat, so the
/// player reads their own D-pad cross and A/B/?/M row upright.
///
/// Debug-only by design: platforms decide whether to call it (the firmware
/// gates it behind a feature), and it deliberately draws OVER the UI.
pub fn draw_input_debug(frame: &mut DeviceFrame, masks: [u8; 2]) {
    const CELL: i32 = 10;
    const GAP: i32 = 2;
    // Seat-local cluster geometry: D-pad cross in a 3x3 cell grid, then the
    // four action cells in a row beside it, vertically centred.
    // (bit, cell x, cell y) in cluster cells.
    const CELLS: [(u8, i32, i32); 8] = [
        (0, 1, 0), // Up
        (1, 1, 2), // Down
        (2, 0, 1), // Left
        (3, 2, 1), // Right
        (4, 3, 1), // A
        (5, 4, 1), // B
        (6, 5, 1), // Info
        (7, 6, 1), // Menu
    ];
    // 3x5 glyphs, bit 2 = leftmost column.
    const GLYPHS: [[u8; 5]; 8] = [
        [0b010, 0b111, 0b010, 0b010, 0b010], // up arrow
        [0b010, 0b010, 0b010, 0b111, 0b010], // down arrow
        [0b001, 0b010, 0b100, 0b010, 0b001], // <
        [0b100, 0b010, 0b001, 0b010, 0b100], // >
        [0b010, 0b101, 0b111, 0b101, 0b101], // A
        [0b110, 0b101, 0b110, 0b101, 0b110], // B
        [0b111, 0b001, 0b010, 0b000, 0b010], // ?
        [0b101, 0b111, 0b101, 0b101, 0b101], // M
    ];
    let ink = rgb565(crate::display_color::C_INK);
    let lit = rgb565(crate::display_color::C_HP_G);

    // Cluster origin in seat-local half coords (320 wide, DEV_H/2 tall):
    // tucked into the seat's lower-left, inside the play frame.
    let (ox, oy) = (14i32, (DEV_H / 2) as i32 - 14 - 3 * CELL);

    for (seat, &mask) in masks.iter().enumerate() {
        for &(bit, cx, cy) in &CELLS {
            let down = mask & (1 << bit) != 0;
            let x0 = ox + cx * (CELL + GAP);
            let y0 = oy + cy * (CELL + GAP);
            for dy in 0..CELL {
                for dx in 0..CELL {
                    let edge = dx == 0 || dy == 0 || dx == CELL - 1 || dy == CELL - 1;
                    let (over, a) = if down {
                        (lit, 12) // held: bright fill, mostly opaque
                    } else if edge {
                        (ink, 8) // idle: a half-transparent outline only
                    } else {
                        continue;
                    };
                    stamp_seat_px(frame, seat, x0 + dx, y0 + dy, over, a);
                }
            }
            // The glyph reads solid on top of either state.
            let g = &GLYPHS[bit as usize];
            for (gy, row) in g.iter().enumerate() {
                for gx in 0..3 {
                    if row & (0b100 >> gx) != 0 {
                        stamp_seat_px(frame, seat, x0 + 2 + gx, y0 + 2 + gy as i32, ink, 16);
                    }
                }
            }
        }
    }
}

/// Blend one seat-local pixel into the panel: seat 0 (player 1) owns the
/// bottom half unrotated, seat 1 (player 2) the top half rotated 180 — the
/// same mapping as [`Region::half`].
fn stamp_seat_px(frame: &mut DeviceFrame, seat: usize, x: i32, y: i32, over: u16, a: u32) {
    let half = (DEV_H / 2) as i32;
    if x < 0 || y < 0 || x >= DEV_W as i32 || y >= half {
        return;
    }
    let (px, py) = if seat == 0 {
        (x, half + y)
    } else {
        (DEV_W as i32 - 1 - x, half - 1 - y)
    };
    let i = (py as u32 * DEV_W + px as u32) as usize;
    frame.px[i] = blend(frame.px[i], over, a);
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
    /// One seat's half. `bottom` picks which half of the panel; `flip`
    /// rotates it 180 for the far seat.
    pub fn half(frame: &'a mut DeviceFrame, bottom: bool, flip: bool) -> Self {
        Self {
            frame,
            w: DEV_W,
            h: DEV_H / 2,
            ox: 0,
            oy: if bottom { DEV_H as i32 / 2 } else { 0 },
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
