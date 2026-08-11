//! Color renderer for the single-screen redesign.
//!
//! One 240x320 panel lies flat between two seats and is split into two
//! 240x160 halves; the far half is drawn rotated 180 degrees so both players
//! read upright. This module renders **one half**. Composition and rotation
//! live in [`crate::device_view`].
//!
//! Same contract as the mono renderer in [`crate::display`]: platforms never
//! choose a `render_*` function, they hand a [`crate::Screen`] (plus the
//! per-seat context the new UI needs) to [`render_half`] and get pixels. The
//! web build and the firmware must produce identical output from identical
//! state.
//!
//! Layout reference: `mega_blastoise_web/www/ui_flow.html`.

use embedded_graphics::{
    draw_target::DrawTarget,
    mono_font::{
        ascii::{FONT_5X8, FONT_6X10, FONT_8X13},
        MonoTextStyle,
    },
    prelude::*,
    pixelcolor::Rgb565,
    primitives::{PrimitiveStyle, Rectangle, PrimitiveStyleBuilder},
    text::{Alignment, Baseline, Text, TextStyleBuilder},
};

use crate::board_event::MoveSlot;
use crate::display::{InvalidReason, PartySlotData};
use crate::sprites_color::{mon_back_sprite_color, mon_sprite_color, ColorSprite};

/// One seat's half of the panel.
pub const HALF_W: u32 = 240;
pub const HALF_H: u32 = 160;

// ── Palette ──────────────────────────────────────────────────────────────────
// Derived from the mockup. Kept as functions so the constants stay readable
// as hex rather than pre-encoded RGB565.

const fn rgb(hex: u32) -> Rgb565 {
    let r = ((hex >> 16) & 0xFF) as u8;
    let g = ((hex >> 8) & 0xFF) as u8;
    let b = (hex & 0xFF) as u8;
    Rgb565::new(r >> 3, g >> 2, b >> 3)
}

pub const C_BG: Rgb565 = rgb(0xF8F0DC);
pub const C_INK: Rgb565 = rgb(0x2B2A24);
pub const C_BOX: Rgb565 = rgb(0xFFFAF0);
pub const C_DIM: Rgb565 = rgb(0x9A917C);
pub const C_HP_G: Rgb565 = rgb(0x4CAF50);
pub const C_HP_Y: Rgb565 = rgb(0xD9B826);
pub const C_HP_R: Rgb565 = rgb(0xD8383E);
pub const C_SEL: Rgb565 = rgb(0x2B2A24);
pub const C_ACCENT: Rgb565 = rgb(0xD8383E);
pub const C_TRACK: Rgb565 = rgb(0xE8DFC8);

// Gen 3 battle furniture. The message box is teal behind a heavy warm
// border; HP plates are parchment with a hard drop shadow; the field is a
// pair of olive platforms.
pub const C_MSG_FILL: Rgb565 = rgb(0x4E8E96);
pub const C_MSG_EDGE: Rgb565 = rgb(0xD8483E);
pub const C_MSG_TEXT: Rgb565 = rgb(0xF8F8F0);
pub const C_SHADOW: Rgb565 = rgb(0xB9AF95);
pub const C_PLATFORM: Rgb565 = rgb(0xC8C08C);
pub const C_PLATFORM_HI: Rgb565 = rgb(0xDCD5A6);
pub const C_HPCHIP: Rgb565 = rgb(0xD03028);

/// Seat colors. Player 1 is White and player 2 is Red, matching the trainer
/// names the engine already uses in its narration.
pub const C_TRIM_P1: Rgb565 = rgb(0xF4F4EC);
pub const C_TRIM_P2: Rgb565 = rgb(0xD8383E);

/// Trim color for a seat: 1 = White, 2 = Red, anything else = neutral.
pub fn seat_trim(seat: u8) -> Rgb565 {
    match seat {
        1 => C_TRIM_P1,
        2 => C_TRIM_P2,
        _ => C_DIM,
    }
}

/// Type badge color, by Gen 1 type display name.
fn type_color(name: &str) -> Rgb565 {
    match name {
        "Water" => rgb(0x3D8FD8),
        "Fire" => rgb(0xE0703D),
        "Electric" => rgb(0xD9B826),
        "Ice" => rgb(0x6CC4CF),
        "Grass" => rgb(0x5CA83C),
        "Psychic" => rgb(0xD06FA8),
        "Fighting" => rgb(0xB0503C),
        "Poison" => rgb(0x9B5CA8),
        "Ground" => rgb(0xC4A85C),
        "Flying" => rgb(0x8FA8D8),
        "Bug" => rgb(0x8FA83C),
        "Rock" => rgb(0xB8A05C),
        "Ghost" => rgb(0x6C5C9B),
        "Dragon" => rgb(0x7038F8),
        _ => rgb(0xA8A08A),
    }
}

fn hp_color(pct: u8) -> Rgb565 {
    if pct > 50 {
        C_HP_G
    } else if pct > 25 {
        C_HP_Y
    } else {
        C_HP_R
    }
}

// ── Per-seat context ─────────────────────────────────────────────────────────

/// Everything a half needs that the shared [`crate::Screen`] does not carry:
/// the foe HUD (open information is the point of this design) and the cursor.
#[derive(Clone, Copy, Default)]
pub struct HalfCtx<'a> {
    /// Which seat this half belongs to — drives the White / Red trim.
    pub seat: u8,
    pub own_name: &'a str,
    pub own_hp: u8,
    pub own_level: u8,
    pub foe_name: &'a str,
    pub foe_hp: u8,
    pub foe_level: u8,
    pub foe_status: Option<&'a str>,
    pub own_status: Option<&'a str>,
    /// Highlighted item on the current screen (move slot or party row).
    pub cursor: u8,
    /// The opponent has committed their choice for this turn.
    pub foe_locked: bool,
    pub bob: bool,
}

// ── Primitives ───────────────────────────────────────────────────────────────

fn fill<D>(d: &mut D, x: i32, y: i32, w: u32, h: u32, c: Rgb565)
where
    D: DrawTarget<Color = Rgb565>,
{
    Rectangle::new(Point::new(x, y), Size::new(w, h))
        .into_styled(PrimitiveStyle::with_fill(c))
        .draw(d)
        .ok();
}

fn panel<D>(d: &mut D, x: i32, y: i32, w: u32, h: u32, bg: Rgb565, border: Rgb565)
where
    D: DrawTarget<Color = Rgb565>,
{
    Rectangle::new(Point::new(x, y), Size::new(w, h))
        .into_styled(
            PrimitiveStyleBuilder::new()
                .fill_color(bg)
                .stroke_color(border)
                .stroke_width(2)
                .build(),
        )
        .draw(d)
        .ok();
}

fn text_at<D>(d: &mut D, s: &str, x: i32, y: i32, font: &'static MonoFont<'static>, c: Rgb565)
where
    D: DrawTarget<Color = Rgb565>,
{
    Text::with_text_style(
        s,
        Point::new(x, y),
        MonoTextStyle::new(font, c),
        TextStyleBuilder::new().baseline(Baseline::Top).build(),
    )
    .draw(d)
    .ok();
}

fn text_center<D>(d: &mut D, s: &str, cx: i32, y: i32, font: &'static MonoFont<'static>, c: Rgb565)
where
    D: DrawTarget<Color = Rgb565>,
{
    Text::with_text_style(
        s,
        Point::new(cx, y),
        MonoTextStyle::new(font, c),
        TextStyleBuilder::new()
            .baseline(Baseline::Top)
            .alignment(Alignment::Center)
            .build(),
    )
    .draw(d)
    .ok();
}

fn text_right<D>(d: &mut D, s: &str, rx: i32, y: i32, font: &'static MonoFont<'static>, c: Rgb565)
where
    D: DrawTarget<Color = Rgb565>,
{
    Text::with_text_style(
        s,
        Point::new(rx, y),
        MonoTextStyle::new(font, c),
        TextStyleBuilder::new()
            .baseline(Baseline::Top)
            .alignment(Alignment::Right)
            .build(),
    )
    .draw(d)
    .ok();
}

use embedded_graphics::mono_font::MonoFont;

// ── Antialiased text ─────────────────────────────────────────────────────────
//
// The mono fonts are 1-bit, so text has hard stair-stepped edges. Rendering a
// double-size glyph into an offscreen mask and box-filtering it back down to
// the target size gives five coverage levels per pixel, which is enough to
// take the jaggedness off diagonals and curves without touching the sprites.

extern crate alloc as _alloc_aa;

struct MaskBuf {
    w: u32,
    h: u32,
    px: _alloc_aa::vec::Vec<bool>,
}

impl MaskBuf {
    fn new(w: u32, h: u32) -> Self {
        Self { w, h, px: _alloc_aa::vec![false; (w * h) as usize] }
    }
    #[inline]
    fn get(&self, x: u32, y: u32) -> bool {
        if x >= self.w || y >= self.h {
            false
        } else {
            self.px[(y * self.w + x) as usize]
        }
    }
}

impl DrawTarget for MaskBuf {
    type Color = embedded_graphics::pixelcolor::BinaryColor;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        for embedded_graphics::Pixel(p, c) in pixels {
            if p.x >= 0 && p.y >= 0 && (p.x as u32) < self.w && (p.y as u32) < self.h {
                if c == embedded_graphics::pixelcolor::BinaryColor::On {
                    self.px[(p.y as u32 * self.w + p.x as u32) as usize] = true;
                }
            }
        }
        Ok(())
    }
}

impl OriginDimensions for MaskBuf {
    fn size(&self) -> Size {
        Size::new(self.w, self.h)
    }
}

use embedded_graphics::geometry::OriginDimensions;

/// Blend two colors, `t` in 0..=4.
fn mix(bg: Rgb565, fg: Rgb565, t: u32) -> Rgb565 {
    let l = |a: u8, b: u8| -> u8 { ((a as u32 * (4 - t) + b as u32 * t) / 4) as u8 };
    Rgb565::new(l(bg.r(), fg.r()), l(bg.g(), fg.g()), l(bg.b(), fg.b()))
}

/// Draw text with softened diagonals.
///
/// Supersampling a double-size glyph and filtering it down would grey the
/// whole stroke and shrink the text, which reads as blurry rather than
/// smooth. Instead the glyph is drawn at full size and crisp, and only the
/// staircase corners on diagonals and curves get a half-strength pixel — the
/// jaggedness goes without the letters losing weight.
///
/// `bg` must be the color actually behind the text; the softened corners are
/// blended toward it, so a wrong value shows as a halo.
pub fn text_aa_font<D>(
    d: &mut D,
    s: &str,
    x: i32,
    y: i32,
    font: &'static MonoFont<'static>,
    fg: Rgb565,
    bg: Rgb565,
) where
    D: DrawTarget<Color = Rgb565>,
{
    if s.is_empty() {
        return;
    }
    let cw = font.character_size.width;
    let ch = font.character_size.height;
    let n = s.chars().count() as u32;
    let mut mask = MaskBuf::new(n * cw + 2, ch + 2);
    Text::with_text_style(
        s,
        Point::new(0, 0),
        MonoTextStyle::new(font, embedded_graphics::pixelcolor::BinaryColor::On),
        TextStyleBuilder::new().baseline(Baseline::Top).build(),
    )
    .draw(&mut mask)
    .ok();

    for my in 0..mask.h {
        for mx in 0..mask.w {
            let px = x + mx as i32;
            let py = y + my as i32;
            if mask.get(mx, my) {
                fill(d, px, py, 1, 1, fg);
                continue;
            }
            // An empty pixel wedged into a staircase corner: on both one
            // side and one of top/bottom, with the matching diagonal set.
            let l = mx > 0 && mask.get(mx - 1, my);
            let r = mask.get(mx + 1, my);
            let u = my > 0 && mask.get(mx, my - 1);
            let dn = mask.get(mx, my + 1);
            let diag = (l && u && mx > 0 && my > 0 && mask.get(mx - 1, my - 1))
                || (r && u && my > 0 && mask.get(mx + 1, my - 1))
                || (l && dn && mx > 0 && mask.get(mx - 1, my + 1))
                || (r && dn && mask.get(mx + 1, my + 1));
            if ((l || r) && (u || dn)) && !diag {
                fill(d, px, py, 1, 1, mix(bg, fg, 2));
            }
        }
    }
}

/// Softened text at the body size.
pub fn text_aa<D>(d: &mut D, s: &str, x: i32, y: i32, fg: Rgb565, bg: Rgb565)
where
    D: DrawTarget<Color = Rgb565>,
{
    text_aa_font(d, s, x, y, &FONT_6X10, fg, bg);
}

/// Centered variant of [`text_aa`].
pub fn text_aa_center<D>(d: &mut D, s: &str, cx: i32, y: i32, fg: Rgb565, bg: Rgb565)
where
    D: DrawTarget<Color = Rgb565>,
{
    let w = s.chars().count() as i32 * 6;
    text_aa(d, s, cx - w / 2, y, fg, bg);
}

/// Truncate to `n` bytes on a char boundary (never split a multi-byte char —
/// a byte-level cut on a curly apostrophe is a real crash source).
fn clip(s: &str, n: usize) -> &str {
    if s.len() <= n {
        return s;
    }
    let mut end = n;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// Small filled pill with light text — type badges, status tags, `LOCKED`.
fn chip<D>(d: &mut D, x: i32, y: i32, label: &str, c: Rgb565) -> i32
where
    D: DrawTarget<Color = Rgb565>,
{
    let w = label.len() as u32 * 5 + 8;
    fill(d, x, y, w, 11, c);
    text_at(d, label, x + 4, y + 2, &FONT_5X8, C_BOX);
    x + w as i32 + 4
}

/// HP track with a colored fill. `w` is the full track width.
fn hp_bar<D>(d: &mut D, x: i32, y: i32, w: u32, pct: u8)
where
    D: DrawTarget<Color = Rgb565>,
{
    fill(d, x, y, w, 8, C_INK);
    fill(d, x + 1, y + 1, w - 2, 6, C_TRACK);
    let inner = w - 2;
    let filled = (inner as u32 * pct.min(100) as u32) / 100;
    if filled > 0 {
        fill(d, x + 1, y + 1, filled, 6, hp_color(pct));
    }
}

/// Integer square root — `f32::sqrt` is not available in `no_std`.
fn isqrt(n: i64) -> i32 {
    if n <= 0 {
        return 0;
    }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x as i32
}

/// Filled ellipse, used for the platforms the mons stand on.
fn ellipse<D>(d: &mut D, cx: i32, cy: i32, rx: i32, ry: i32, c: Rgb565)
where
    D: DrawTarget<Color = Rgb565>,
{
    if rx <= 0 || ry <= 0 {
        return;
    }
    for dy in -ry..=ry {
        // Half-width at this row, from the ellipse equation.
        let t = (ry * ry - dy * dy).max(0);
        let half = isqrt(t as i64 * (rx * rx) as i64 / (ry * ry) as i64);
        if half > 0 {
            fill(d, cx - half, cy + dy, (half * 2) as u32, 1, c);
        }
    }
}

/// The field a mon stands on: a lit platform with a shadowed underside.
pub fn draw_platform<D>(d: &mut D, cx: i32, cy: i32, rx: i32)
where
    D: DrawTarget<Color = Rgb565>,
{
    let ry = (rx / 3).max(4);
    ellipse(d, cx, cy, rx, ry, C_PLATFORM);
    ellipse(d, cx, cy - 2, rx - 3, ry - 2, C_PLATFORM_HI);
}

/// A Gen 3 status plate: parchment panel with a hard drop shadow, the name
/// and level on top, and an HP chip beside the bar. `trim` colors a tab on
/// the plate so each seat's boxes are identifiable at a glance.
#[allow(clippy::too_many_arguments)]
pub fn draw_status_plate<D>(
    d: &mut D,
    x: i32,
    y: i32,
    w: u32,
    name: &str,
    level: u8,
    pct: u8,
    numbers: Option<(u16, u16)>,
    trim: Rgb565,
) where
    D: DrawTarget<Color = Rgb565>,
{
    let h: u32 = if numbers.is_some() { 40 } else { 32 };
    // Drop shadow, offset down-right like the reference art.
    fill(d, x + 3, y + 3, w, h, C_SHADOW);
    panel(d, x, y, w, h, C_BOX, C_INK);
    // Seat tab down the left edge.
    fill(d, x + 2, y + 2, 4, h - 4, trim);

    text_at(d, clip(name, 13), x + 10, y + 4, &FONT_6X10, C_INK);
    if level > 0 {
        let mut b = LvBuf::new();
        text_right(d, b.fmt(level), x + w as i32 - 6, y + 5, &FONT_5X8, C_INK);
    }

    // HP chip, then the bar in a dark track.
    let bar_y = y + 17;
    fill(d, x + 10, bar_y - 1, 16, 10, C_HPCHIP);
    text_at(d, "HP", x + 12, bar_y + 1, &FONT_5X8, C_MSG_TEXT);
    let bar_x = x + 29;
    let bar_w = w - 39;
    fill(d, bar_x, bar_y, bar_w, 8, C_INK);
    fill(d, bar_x + 1, bar_y + 1, bar_w - 2, 6, C_TRACK);
    let filled = ((bar_w - 2) as u32 * pct.min(100) as u32) / 100;
    if filled > 0 {
        fill(d, bar_x + 1, bar_y + 1, filled, 6, hp_color(pct));
    }

    if let Some((cur, max)) = numbers {
        let mut n = NumBuf::new();
        text_right(d, n.pair(cur as u32, max as u32), x + w as i32 - 6, y + 28, &FONT_5X8, C_INK);
    }
}

/// The Gen 3 message box: teal behind a heavy warm border. Returns the inset
/// where text should be drawn, in [`C_MSG_TEXT`].
pub fn draw_message_box<D>(d: &mut D, x: i32, y: i32, w: u32, h: u32)
where
    D: DrawTarget<Color = Rgb565>,
{
    fill(d, x, y, w, h, C_MSG_EDGE);
    fill(d, x + 3, y + 3, w - 6, h - 6, C_INK);
    fill(d, x + 4, y + 4, w - 8, h - 8, C_MSG_FILL);
}

/// Blit a color sprite at an integer scale, skipping transparent pixels.
fn draw_sprite<D>(d: &mut D, spr: &ColorSprite, x: i32, y: i32, scale: u32)
where
    D: DrawTarget<Color = Rgb565>,
{
    for sy in 0..spr.h as u32 {
        for sx in 0..spr.w as u32 {
            let i = spr.index_at(sx, sy);
            if i == 0 {
                continue;
            }
            let c = Rgb565::from(RawU16::new(spr.color(i)));
            let px = x + (sx * scale) as i32;
            let py = y + (sy * scale) as i32;
            fill(d, px, py, scale, scale, c);
        }
    }
}

use embedded_graphics::pixelcolor::raw::RawU16;

/// Blit at half size by point-sampling every other pixel. Pixel art survives
/// a clean 2:1 reduction, and it keeps a native ~54px sprite inside the 34px
/// HUD slot without a second set of assets.
fn draw_sprite_half<D>(d: &mut D, spr: &ColorSprite, x: i32, y: i32)
where
    D: DrawTarget<Color = Rgb565>,
{
    for sy in (0..spr.h as u32).step_by(2) {
        for sx in (0..spr.w as u32).step_by(2) {
            let i = spr.index_at(sx, sy);
            if i == 0 {
                continue;
            }
            let c = Rgb565::from(RawU16::new(spr.color(i)));
            fill(d, x + (sx / 2) as i32, y + (sy / 2) as i32, 1, 1, c);
        }
    }
}

/// Blit horizontally mirrored. Gen 1 front sprites all face the same way, so
/// one side has to be flipped for the two mons to face each other.
fn draw_sprite_mirrored<D>(d: &mut D, spr: &ColorSprite, x: i32, y: i32, scale: u32)
where
    D: DrawTarget<Color = Rgb565>,
{
    for sy in 0..spr.h as u32 {
        for sx in 0..spr.w as u32 {
            let i = spr.index_at(sx, sy);
            if i == 0 {
                continue;
            }
            let c = Rgb565::from(RawU16::new(spr.color(i)));
            let mx = spr.w as u32 - 1 - sx;
            fill(d, x + (mx * scale) as i32, y + (sy * scale) as i32, scale, scale, c);
        }
    }
}

/// Blit a sprite scaled to fit a box, centered, clipped to the box.
///
/// Sprite dimensions vary by era — the Game Boy back sprites were 32x32 and
/// wanted 2x, Gen 3's are 64x64 and want 1x — so callers give a box and let
/// this pick the scale. Clipping is the belt and braces: art can never spill
/// over the move grid no matter what a future sprite set does.
#[allow(clippy::too_many_arguments)]
fn draw_sprite_fit<D>(
    d: &mut D,
    spr: &ColorSprite,
    bx: i32,
    by: i32,
    bw: u32,
    bh: u32,
    mirror: bool,
    bias_bottom: bool,
) where
    D: DrawTarget<Color = Rgb565>,
{
    let (sw, sh) = (spr.w as u32, spr.h as u32);
    if sw == 0 || sh == 0 {
        return;
    }
    // Halves are expressed as a numerator over 2 so half-scale is available
    // without floating point.
    let num: u32 = if sw * 2 <= bw && sh * 2 <= bh {
        4
    } else if sw <= bw && sh <= bh {
        2
    } else {
        1
    };
    let ow = sw * num / 2;
    let oh = sh * num / 2;
    let ox = bx + (bw as i32 - ow as i32) / 2;
    let oy = if bias_bottom { by + bh as i32 - oh as i32 } else { by + (bh as i32 - oh as i32) / 2 };

    for oy_i in 0..oh {
        for ox_i in 0..ow {
            let sx = ox_i * 2 / num;
            let sy = oy_i * 2 / num;
            let sx = if mirror { sw.saturating_sub(1).saturating_sub(sx) } else { sx };
            let i = spr.index_at(sx, sy);
            if i == 0 {
                continue;
            }
            let px = ox + ox_i as i32;
            let py = oy + oy_i as i32;
            if px < bx || py < by || px >= bx + bw as i32 || py >= by + bh as i32 {
                continue;
            }
            fill(d, px, py, 1, 1, Rgb565::from(RawU16::new(spr.color(i))));
        }
    }
}

/// Draw a species' front art centered in a box. Returns false if unknown.
fn front_sprite_in<D>(d: &mut D, name: &str, cx: i32, cy: i32, scale: u32) -> bool
where
    D: DrawTarget<Color = Rgb565>,
{
    match mon_sprite_color(name) {
        Some(s) => {
            draw_sprite(
                d,
                s,
                cx - (s.w as u32 * scale / 2) as i32,
                cy - (s.h as u32 * scale / 2) as i32,
                scale,
            );
            true
        }
        None => false,
    }
}

/// Draw a species' back art centered. Back sprites are natively 32x32 and
/// the Game Boy drew them at 2x, so `scale` is normally 2.
fn back_sprite_in<D>(d: &mut D, name: &str, cx: i32, cy: i32, scale: u32) -> bool
where
    D: DrawTarget<Color = Rgb565>,
{
    match mon_back_sprite_color(name) {
        Some(s) => {
            draw_sprite(
                d,
                s,
                cx - (s.w as u32 * scale / 2) as i32,
                cy - (s.h as u32 * scale / 2) as i32,
                scale,
            );
            true
        }
        None => false,
    }
}

// ── Shared furniture ─────────────────────────────────────────────────────────

/// The permanent foe HUD along the top of every choice screen: sprite, name,
/// level, HP, status, and whether they have locked in.
fn foe_hud<D>(d: &mut D, ctx: &HalfCtx<'_>)
where
    D: DrawTarget<Color = Rgb565>,
{
    // A stripe of this seat's color down the outer edge, so a player can
    // always tell at a glance which half is theirs.
    fill(d, 0, 0, 3, HALF_H, seat_trim(ctx.seat));
    panel(d, 3, 3, 234, 38, C_BOX, C_INK);
    if let Some(s) = mon_sprite_color(ctx.foe_name) {
        // Half scale: native art is ~54px, the HUD slot is 34px.
        let w = (s.w as i32 + 1) / 2;
        let h = (s.h as i32 + 1) / 2;
        draw_sprite_half(d, s, 7 + (34 - w).max(0) / 2, 6 + (34 - h).max(0) / 2);
    }
    text_at(d, "FOE", 45, 8, &FONT_5X8, C_DIM);
    text_at(d, clip(ctx.foe_name, 13), 69, 6, &FONT_8X13, C_INK);
    if ctx.foe_level > 0 {
        let mut buf = LvBuf::new();
        text_right(d, buf.fmt(ctx.foe_level), 232, 8, &FONT_5X8, C_DIM);
    }
    hp_bar(d, 45, 24, 150, ctx.foe_hp);
    if ctx.foe_locked {
        chip(d, 199, 23, "LOCK", C_DIM);
    } else if let Some(st) = ctx.foe_status {
        chip(d, 199, 23, clip(st, 3), C_ACCENT);
    }
}

/// `Lv55` without `alloc::format!` — this runs on the firmware too.
struct LvBuf([u8; 8], usize);

impl LvBuf {
    fn new() -> Self {
        Self([0; 8], 0)
    }
    fn fmt(&mut self, lv: u8) -> &str {
        self.0[0] = b'L';
        self.0[1] = b'v';
        let mut n = lv;
        let mut digits = [0u8; 3];
        let mut c = 0;
        if n == 0 {
            digits[0] = b'0';
            c = 1;
        }
        while n > 0 {
            digits[c] = b'0' + (n % 10);
            n /= 10;
            c += 1;
        }
        for i in 0..c {
            self.0[2 + i] = digits[c - 1 - i];
        }
        self.1 = 2 + c;
        core::str::from_utf8(&self.0[..self.1]).unwrap_or("Lv")
    }
}

/// `12/34`-style pair into a stack buffer.
struct NumBuf([u8; 12], usize);

impl NumBuf {
    fn new() -> Self {
        Self([0; 12], 0)
    }
    fn push_num(&mut self, mut n: u32) {
        let mut digits = [0u8; 6];
        let mut c = 0;
        if n == 0 {
            digits[0] = b'0';
            c = 1;
        }
        while n > 0 && c < 6 {
            digits[c] = b'0' + (n % 10) as u8;
            n /= 10;
            c += 1;
        }
        for i in 0..c {
            if self.1 < self.0.len() {
                self.0[self.1] = digits[c - 1 - i];
                self.1 += 1;
            }
        }
    }
    fn push(&mut self, b: u8) {
        if self.1 < self.0.len() {
            self.0[self.1] = b;
            self.1 += 1;
        }
    }
    fn pair(&mut self, a: u32, b: u32) -> &str {
        self.1 = 0;
        self.push_num(a);
        self.push(b'/');
        self.push_num(b);
        core::str::from_utf8(&self.0[..self.1]).unwrap_or("")
    }
    fn one(&mut self, a: u32) -> &str {
        self.1 = 0;
        self.push_num(a);
        core::str::from_utf8(&self.0[..self.1]).unwrap_or("")
    }
}

/// Bottom-of-half control legend.
fn legend<D>(d: &mut D, items: &[&str])
where
    D: DrawTarget<Color = Rgb565>,
{
    let mut x = 6;
    for it in items {
        text_at(d, it, x, 150, &FONT_5X8, C_DIM);
        x += it.len() as i32 * 5 + 10;
    }
}

// ── Screens ──────────────────────────────────────────────────────────────────

/// Move-choice screen: foe HUD, own back sprite, 2x2 move grid with cursor,
/// and an info bar describing the highlighted move.
pub fn render_choice<D>(d: &mut D, moves: &[MoveSlot], ctx: &HalfCtx<'_>)
where
    D: DrawTarget<Color = Rgb565>,
{
    d.clear(C_BG).ok();
    foe_hud(d, ctx);

    // Own mon, back view, bobbing — boxed into the left column so it can
    // never reach the move grid.
    let bob = if ctx.bob { -2 } else { 0 };
    match mon_back_sprite_color(ctx.own_name) {
        Some(spr) => draw_sprite_fit(d, spr, 4, 44 + bob, 68, 78, false, true),
        None => {
            text_center(d, clip(ctx.own_name, 9), 38, 84, &FONT_6X10, C_INK);
        }
    }
    hp_bar(d, 6, 126, 64, ctx.own_hp);

    // 2x2 move grid.
    const GX: [i32; 2] = [76, 158];
    const GY: [i32; 2] = [46, 88];
    for (i, mv) in moves.iter().take(4).enumerate() {
        let x = GX[i % 2];
        let y = GY[i / 2];
        let sel = i as u8 == ctx.cursor;
        let dead = mv.pp == 0;
        let (bg, fg) = if sel {
            (C_SEL, C_BOX)
        } else if dead {
            (C_BOX, C_DIM)
        } else {
            (C_BOX, C_INK)
        };
        panel(d, x, y, 78, 38, bg, if dead { C_DIM } else { C_INK });
        // Move names run long ("Thunder Punch"): two lines of 12 chars.
        let name = mv.name.as_str();
        if name.len() > 12 {
            let cut = name[..12.min(name.len())]
                .rfind(' ')
                .unwrap_or(11.min(name.len().saturating_sub(1)));
            text_at(d, clip(&name[..cut], 12), x + 5, y + 6, &FONT_6X10, fg);
            text_at(d, clip(name[cut..].trim_start(), 12), x + 5, y + 18, &FONT_6X10, fg);
        } else {
            text_at(d, name, x + 5, y + 12, &FONT_6X10, fg);
        }
        let mut n = NumBuf::new();
        text_right(d, n.pair(mv.pp as u32, mv.max_pp as u32), x + 73, y + 27, &FONT_5X8, if sel { C_BOX } else { C_DIM });
    }

    // Info bar for the highlighted move.
    if let Some(mv) = moves.get(ctx.cursor as usize) {
        panel(d, 76, 128, 160, 18, C_BOX, C_INK);
        let mut x = chip(d, 79, 131, clip(&mv.type_name, 3), type_color(&mv.type_name));
        let mut n = NumBuf::new();
        text_at(d, "P", x, 133, &FONT_5X8, C_DIM);
        text_at(d, mv.power.map(|p| n.one(p)).unwrap_or("--"), x + 7, 133, &FONT_5X8, C_INK);
        x += 30;
        let mut n2 = NumBuf::new();
        text_at(d, "A", x, 133, &FONT_5X8, C_DIM);
        text_at(d, mv.accuracy.map(|a| n2.one(a as u32)).unwrap_or("--"), x + 7, 133, &FONT_5X8, C_INK);
    }

    legend(d, &["+ MOVE", "A LOCK", "B PARTY", "? INFO"]);
}

/// This player has committed; the move stays hidden from the other seat.
pub fn render_locked<D>(d: &mut D, chosen: Option<&str>, ctx: &HalfCtx<'_>)
where
    D: DrawTarget<Color = Rgb565>,
{
    d.clear(C_BG).ok();
    foe_hud(d, ctx);
    let bob = if ctx.bob { -2 } else { 0 };
    if let Some(spr) = mon_back_sprite_color(ctx.own_name) {
        draw_sprite_fit(d, spr, 12, 52 + bob, 84, 82, false, true);
    }
    text_center(d, "LOCKED IN", 160, 60, &FONT_8X13, C_INK);
    if let Some(c) = chosen {
        panel(d, 100, 78, 128, 20, C_BOX, C_INK);
        text_center(d, clip(c, 19), 164, 83, &FONT_6X10, C_INK);
    }
    text_center(
        d,
        if ctx.foe_locked { "both ready..." } else { "rival is choosing..." },
        160,
        108,
        &FONT_5X8,
        C_DIM,
    );
    legend(d, &["B CANCEL"]);
}

/// Party list with a cursor: switching, or the forced pick after a faint.
pub fn render_party<D>(d: &mut D, party: &[PartySlotData], ctx: &HalfCtx<'_>, forced: bool)
where
    D: DrawTarget<Color = Rgb565>,
{
    d.clear(C_BG).ok();
    text_center(d, if forced { "SEND WHO NEXT?" } else { "PARTY" }, 120, 4, &FONT_8X13, C_INK);
    fill(d, 6, 20, 228, 2, C_INK);

    for (i, slot) in party.iter().take(6).enumerate() {
        let y = 26 + i as i32 * 21;
        let sel = i as u8 == ctx.cursor;
        let dead = slot.hp == 0;
        let fg = if sel {
            C_BOX
        } else if dead {
            C_DIM
        } else {
            C_INK
        };
        panel(d, 6, y, 228, 19, if sel { C_SEL } else { C_BOX }, if dead { C_DIM } else { C_INK });
        text_at(d, clip(&slot.name, 11), 11, y + 4, &FONT_6X10, fg);
        let pct = if slot.max_hp == 0 {
            0
        } else {
            ((slot.hp as u32 * 100) / slot.max_hp as u32) as u8
        };
        hp_bar(d, 92, y + 6, 74, pct);
        let mut n = NumBuf::new();
        text_at(d, n.pair(slot.hp as u32, slot.max_hp as u32), 172, y + 5, &FONT_5X8, fg);
        if slot.active {
            text_right(d, "OUT", 230, y + 5, &FONT_5X8, if sel { C_BOX } else { C_DIM });
        } else if let Some(st) = &slot.status {
            text_right(d, clip(st, 3), 230, y + 5, &FONT_5X8, if sel { C_BOX } else { C_ACCENT });
        }
    }

    if forced {
        legend(d, &["+ PICK", "A SEND IN", "? INFO"]);
    } else {
        legend(d, &["+ PICK", "A SWITCH", "B BACK", "? INFO"]);
    }
}

/// True once a name is a real species rather than the controller's boot
/// placeholder — the opening plays on empty ground before either side is in.
fn has_mon(name: &str) -> bool {
    !name.is_empty() && name != "---"
}

/// One seat's half of the shared head-to-head battle scene.
///
/// Both halves draw this at the same time and the far one is rotated 180, so
/// the result is a single battlefield rather than two private views: each
/// player's own mon stands near the centre line reading upright, with its HP
/// plate directly under it, and the rival's mon stands opposite reading upside
/// down from across the table. The caption is drawn in both halves, which puts
/// the narration at the bottom of the screen from either seat.
pub fn render_playback<D>(d: &mut D, caption: &str, ctx: &HalfCtx<'_>)
where
    D: DrawTarget<Color = Rgb565>,
{
    d.clear(C_BG).ok();
    // A stripe of this seat's color down the outer edge, the same marker the
    // choice screens carry, so a half is always identifiable as yours.
    fill(d, 0, 0, 3, HALF_H, seat_trim(ctx.seat));

    // The mon stands toward the seam, so the two of them meet in the middle
    // of the panel instead of each sitting in its own box.
    draw_platform(d, 88, 76, 44);
    let bob = if ctx.bob { -2 } else { 0 };
    if has_mon(ctx.own_name) {
        if let Some(spr) = mon_sprite_color(ctx.own_name) {
            draw_sprite_fit(d, spr, 28, 2 + bob, 120, 76, false, true);
        } else {
            text_center(d, clip(ctx.own_name, 13), 88, 60, &FONT_8X13, C_INK);
        }

        // HP plate directly under the mon, in this seat's frame.
        draw_status_plate(
            d,
            6,
            92,
            152,
            ctx.own_name,
            ctx.own_level,
            ctx.own_hp,
            None,
            seat_trim(ctx.seat),
        );
        if let Some(st) = ctx.own_status {
            chip(d, 166, 94, clip(st, 3), C_ACCENT);
        }
    }

    text_at(d, "A NEXT", 166, 108, &FONT_5X8, C_DIM);
    text_at(d, "? BATTLE LOG", 166, 118, &FONT_5X8, C_DIM);

    // Narration, up to two lines of 36 chars, at the bottom of this seat's
    // half — which is the bottom of the screen from where they are sitting.
    draw_message_box(d, 3, 128, 234, 30);
    let line1_len = 36.min(caption.len());
    let (l1, l2) = if caption.len() <= 36 {
        (caption, "")
    } else {
        let cut = caption[..line1_len].rfind(' ').unwrap_or(line1_len);
        (&caption[..cut], caption[cut..].trim_start())
    };
    text_aa(d, clip(l1, 36), 11, 134, C_MSG_TEXT, C_MSG_FILL);
    if !l2.is_empty() {
        text_aa(d, clip(l2, 36), 11, 145, C_MSG_TEXT, C_MSG_FILL);
    }
}

/// Lobby half (also used, unrotated, for the landscape lobby).
pub fn render_lobby<D>(d: &mut D, ready: bool, ai: bool)
where
    D: DrawTarget<Color = Rgb565>,
{
    d.clear(C_BG).ok();
    if ready && ai {
        text_center(d, "AI", 120, 60, &FONT_8X13, C_ACCENT);
        text_center(d, "a robot rival takes this side", 120, 84, &FONT_5X8, C_DIM);
    } else if ready {
        text_center(d, "READY!", 120, 56, &FONT_8X13, C_HP_G);
        text_center(d, "waiting for rival...", 120, 80, &FONT_6X10, C_DIM);
        text_center(d, "B to cancel", 120, 96, &FONT_5X8, C_DIM);
    } else {
        text_center(d, "PRESS A TO READY", 120, 52, &FONT_8X13, C_INK);
        text_center(d, "HOLD A: FIGHT THE AI", 120, 78, &FONT_6X10, C_DIM);
        text_center(d, "?: RULES + CONTROLS", 120, 94, &FONT_6X10, C_DIM);
    }
}

/// Win / lose / tie recap.
pub fn render_result<D>(d: &mut D, msg: &str, ctx: &HalfCtx<'_>)
where
    D: DrawTarget<Color = Rgb565>,
{
    d.clear(C_BG).ok();
    let win = msg.starts_with("WIN");
    text_center(d, msg, 120, 22, &FONT_8X13, if win { C_HP_G } else { C_INK });
    if let Some(spr) = mon_sprite_color(ctx.own_name) {
        draw_sprite_fit(d, spr, 76, 46, 88, 76, false, true);
    }
    legend(d, &["A REMATCH", "? BATTLE LOG"]);
}

/// Full description for the cursor's move — what `?` opens.
pub fn render_move_info<D>(d: &mut D, mv: &MoveSlot, desc: &str)
where
    D: DrawTarget<Color = Rgb565>,
{
    d.clear(C_BG).ok();
    text_at(d, clip(&mv.name, 22), 6, 5, &FONT_8X13, C_INK);
    fill(d, 6, 21, 228, 2, C_INK);
    let mut x = chip(d, 6, 27, clip(&mv.type_name, 3), type_color(&mv.type_name));
    x = chip(d, x, 27, clip(&mv.category, 4), C_DIM);
    let mut n = NumBuf::new();
    text_at(d, "POW", x, 29, &FONT_5X8, C_DIM);
    text_at(d, mv.power.map(|p| n.one(p)).unwrap_or("---"), x + 20, 29, &FONT_5X8, C_INK);
    let mut n2 = NumBuf::new();
    text_at(d, "ACC", x + 46, 29, &FONT_5X8, C_DIM);
    text_at(d, mv.accuracy.map(|a| n2.one(a as u32)).unwrap_or("---"), x + 66, 29, &FONT_5X8, C_INK);
    let mut n3 = NumBuf::new();
    text_at(d, "PP", x + 92, 29, &FONT_5X8, C_DIM);
    text_at(d, n3.pair(mv.pp as u32, mv.max_pp as u32), x + 108, 29, &FONT_5X8, C_INK);

    // Description, 38 chars per line, word wrapped.
    let mut y = 48;
    let mut rest = desc;
    while !rest.is_empty() && y < 142 {
        let take = if rest.len() <= 38 {
            rest.len()
        } else {
            rest[..38].rfind(' ').unwrap_or(38)
        };
        text_at(d, clip(&rest[..take], 38), 6, y, &FONT_6X10, C_INK);
        rest = rest[take..].trim_start();
        y += 12;
    }
    legend(d, &["B BACK"]);
}

/// Scrollable battle log — the teaching surface behind `?`.
pub fn render_log<D>(d: &mut D, lines: &[&str], top: usize)
where
    D: DrawTarget<Color = Rgb565>,
{
    d.clear(C_BG).ok();
    text_center(d, "BATTLE LOG", 120, 4, &FONT_8X13, C_INK);
    fill(d, 6, 20, 228, 2, C_INK);
    for (i, line) in lines.iter().skip(top).take(11).enumerate() {
        text_at(d, clip(line, 46), 6, 26 + i as i32 * 11, &FONT_5X8, C_INK);
    }
    legend(d, &["+ SCROLL", "B BACK"]);
}

/// Invalid-selection flash.
pub fn render_invalid<D>(d: &mut D, reason: InvalidReason)
where
    D: DrawTarget<Color = Rgb565>,
{
    d.clear(C_BG).ok();
    let (a, b) = match reason {
        InvalidReason::Fainted => ("Already fainted!", ""),
        InvalidReason::AlreadyOut => ("Already out!", ""),
        InvalidReason::NoPp => ("No power remaining", "for that move!"),
        InvalidReason::Trapped => ("Trapped -", "can't switch out!"),
    };
    text_center(d, a, 120, 62, &FONT_8X13, C_ACCENT);
    if !b.is_empty() {
        text_center(d, b, 120, 82, &FONT_8X13, C_ACCENT);
    }
}

/// One row of the options menu.
pub struct OptionRow<'a> {
    pub label: &'a str,
    pub value: &'a str,
}

/// Options menu — lobby only, drawn in landscape (see `device_view`).
pub fn render_options<D>(d: &mut D, rows: &[OptionRow<'_>], cursor: u8, w: u32)
where
    D: DrawTarget<Color = Rgb565>,
{
    d.clear(C_BG).ok();
    let cx = (w / 2) as i32;
    text_center(d, "OPTIONS", cx, 6, &FONT_8X13, C_INK);
    fill(d, 12, 24, w - 24, 2, C_INK);
    for (i, r) in rows.iter().enumerate() {
        let y = 32 + i as i32 * 22;
        let sel = i as u8 == cursor;
        panel(d, 12, y, w - 24, 20, if sel { C_SEL } else { C_BOX }, C_INK);
        let fg = if sel { C_BOX } else { C_INK };
        text_at(d, r.label, 18, y + 5, &FONT_6X10, fg);
        text_right(d, r.value, (w - 18) as i32, y + 5, &FONT_6X10, fg);
    }
    let mut x = 14;
    for it in ["+ CHANGE", "A CONFIRM", "B BACK"] {
        text_at(d, it, x, (HALF_H * 2 - 16) as i32, &FONT_5X8, C_DIM);
        x += it.len() as i32 * 5 + 12;
    }
}

/// Generation picker — the first screen, before the lobby.
pub fn render_gen_picker<D>(d: &mut D, cursor: u8, w: u32, h: u32)
where
    D: DrawTarget<Color = Rgb565>,
{
    d.clear(C_BG).ok();
    let cx = (w / 2) as i32;
    text_center(d, "CHOOSE A GAME", cx, 18, &FONT_8X13, C_INK);
    let opts = [
        ("GEN 1", "Red / Blue rules", true),
        ("GEN 3", "Ruby / Sapphire  (preview)", false),
    ];
    for (i, (name, sub, ready)) in opts.iter().enumerate() {
        let y = 48 + i as i32 * 46;
        let sel = i as u8 == cursor;
        panel(d, 24, y, w - 48, 38, if sel { C_SEL } else { C_BOX }, C_INK);
        let fg = if sel { C_BOX } else { C_INK };
        text_at(d, name, 34, y + 6, &FONT_8X13, fg);
        text_at(d, sub, 34, y + 22, &FONT_5X8, if sel { C_BOX } else { C_DIM });
        if !ready {
            text_right(d, "STUB", (w - 34) as i32, y + 6, &FONT_5X8, C_ACCENT);
        }
    }
    text_center(d, "+ CHOOSE    A START", cx, (h - 18) as i32, &FONT_5X8, C_DIM);
}

/// The shared battle view: both mons front-facing on level ground.
///
/// Used for the opening, for every switch-in, and for turn playback. Both
/// players read this at once, so neither mon is drawn from behind and
/// neither side gets the nearer, larger spot. Gen 3 sprites all face the
/// same way, so the left one is mirrored to square them up. Sides are
/// optional, letting the screen open on empty ground and introduce the two
/// trainers one at a time.
pub fn render_versus<D>(
    d: &mut D,
    left: Option<&str>,
    right: Option<&str>,
    caption: &str,
    w: u32,
    h: u32,
) where
    D: DrawTarget<Color = Rgb565>,
{
    d.clear(C_BG).ok();
    let wi = w as i32;
    let hi = h as i32;

    // Level ground: identical platforms at the same height, mirrored about
    // the centre line, so neither player is "closer" to the action.
    let ground_y = hi - 66;
    let rx = (wi / 5).clamp(34, 74);
    let left_cx = wi / 4;
    let right_cx = wi - wi / 4;
    draw_platform(d, left_cx, ground_y, rx);
    draw_platform(d, right_cx, ground_y, rx);

    let scale: u32 = if w >= 300 { 2 } else { 1 };
    let mut side = |name: &str, cx: i32, mirror: bool| {
        if let Some(spr) = mon_sprite_color(name) {
            let x = cx - (spr.w as u32 * scale / 2) as i32;
            let y = ground_y - (spr.h as u32 * scale) as i32 + 10;
            if mirror {
                draw_sprite_mirrored(d, spr, x, y, scale);
            } else {
                draw_sprite(d, spr, x, y, scale);
            }
        }
        text_aa_center(d, clip(name, 16), cx, ground_y + 16, C_INK, C_BG);
    };
    if let Some(name) = left {
        side(name, left_cx, true);
    }
    if let Some(name) = right {
        side(name, right_cx, false);
    }

    draw_message_box(d, 4, hi - 32, w - 8, 28);
    text_aa(d, clip(caption, 56), 12, hi - 24, C_MSG_TEXT, C_MSG_FILL);
}
