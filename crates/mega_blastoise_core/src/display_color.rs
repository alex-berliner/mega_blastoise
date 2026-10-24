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
//! Layout reference: `crates/mega_blastoise_web/www/ui_flow.html`.

use embedded_graphics::{
    draw_target::DrawTarget,
    mono_font::{
        ascii::{FONT_5X8, FONT_6X10, FONT_8X13, FONT_10X20},
        MonoTextStyle,
    },
    prelude::*,
    pixelcolor::Rgb565,
    primitives::{PrimitiveStyle, Rectangle, PrimitiveStyleBuilder},
    text::{Alignment, Baseline, Text, TextStyleBuilder},
};

use crate::board_event::MoveSlot;
use crate::display::{InvalidReason, PartySlotData};
use crate::sprites_color::{mon_back_sprite_color, mon_sprite_color, species_display_name, ColorSprite};

/// One seat's half of the panel. Native ST7796 size: the stripboard v2
/// glass is 320x480, split into two 320x240 halves.
pub const HALF_W: u32 = 320;
pub const HALF_H: u32 = 240;

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

// The bottom menus in Gen 3 are the same teal as the message box behind a
// blue double border with a gold pin in each corner, rather than the red
// border the narration box uses.
pub const C_MENU_EDGE: Rgb565 = rgb(0x2858B0);
pub const C_MENU_HI: Rgb565 = rgb(0x78B8E8);
pub const C_MENU_PIN: Rgb565 = rgb(0xF0C838);

/// Seat colors. Player 1 is White and player 2 is Red, matching the trainer
/// names the engine already uses in its narration.
pub const C_TRIM_P1: Rgb565 = rgb(0xF4F4EC);
pub const C_TRIM_P2: Rgb565 = rgb(0xD8383E);

/// Trim color for a seat: 1 = White, 2 = Red, anything else = neutral. Every
/// screen the divider runs through passes a real seat, so the two halves of
/// the panel always come together as a pokeball.
pub fn seat_trim(seat: u8) -> Rgb565 {
    match seat {
        1 => C_TRIM_P1,
        2 => C_TRIM_P2,
        _ => C_DIM,
    }
}

/// Type badge color, by Gen 1 type display name.
/// Public form of [`type_color`], for callers outside this module that have
/// a type name or abbreviation in hand.
pub fn type_color_of(name: &str) -> Rgb565 {
    type_color(name)
}

/// One colour per type, whichever way the type is spelled. Screens label
/// chips with the mono renderer's abbreviations and moves carry the full
/// name, so both have to resolve here or badges fall through to grey.
fn type_color(name: &str) -> Rgb565 {
    match name {
        "Water" | "WAT" => rgb(0x3D8FD8),
        "Fire" | "FIR" => rgb(0xE0703D),
        "Electric" | "ELC" => rgb(0xD9B826),
        "Ice" | "ICE" => rgb(0x6CC4CF),
        "Grass" | "GRS" => rgb(0x5CA83C),
        "Psychic" | "PSY" => rgb(0xD06FA8),
        "Fighting" | "FGT" => rgb(0xB0503C),
        "Poison" | "PSN" => rgb(0x9B5CA8),
        "Ground" | "GND" => rgb(0xC4A85C),
        "Flying" | "FLY" => rgb(0x8FA8D8),
        "Bug" | "BUG" => rgb(0x8FA83C),
        "Rock" | "RCK" => rgb(0xB8A05C),
        "Ghost" | "GHO" => rgb(0x6C5C9B),
        "Dragon" | "DRG" => rgb(0x7038F8),
        "Dark" | "DRK" => rgb(0x705848),
        "Steel" | "STL" => rgb(0x9098A8),
        "Fairy" | "FAI" => rgb(0xEE99AC),
        _ => rgb(0xA8A08A),
    }
}

/// The fifteen types Gen 1 actually uses, in chart order.
const GEN1_TYPES: [ps_dex::Type; 15] = {
    use ps_dex::Type::*;
    [
        Normal, Fire, Water, Electric, Grass, Ice, Fighting, Poison, Ground, Flying, Psychic,
        Bug, Rock, Ghost, Dragon,
    ]
};

/// The [`ps_dex::Type`] for a display name or abbreviation: the inverse of
/// [`crate::display::type_abbr`], so a move's printed type can be looked up
/// in the chart.
fn type_from_name(name: &str) -> Option<ps_dex::Type> {
    GEN1_TYPES
        .iter()
        .copied()
        .find(|t| crate::display::type_abbr(*t) == name || t.name() == name)
}

/// Gen 1's type effectiveness ×10 (0, 5, 10 or 20), from the Gen 1 dex's
/// chart (PS's `damageTaken` codes: 1 weak, 2 resists, 3 immune).
fn gen1_effectiveness(attacker: ps_dex::Type, defender: ps_dex::Type) -> u8 {
    match ps_dex::GEN1.typechart.code(attacker, defender) {
        1 => 20,
        2 => 5,
        3 => 0,
        _ => 10,
    }
}

/// Badge colour per status, roughly the Gen 3 palette.
fn status_color(st: &str) -> Rgb565 {
    match st.get(..3).unwrap_or("") {
        "PSN" | "TOX" | "psn" | "tox" => rgb(0xA040A0),
        "BRN" | "brn" => rgb(0xE0703D),
        "PAR" | "par" => rgb(0xC8A020),
        "SLP" | "slp" => rgb(0x8878A0),
        "FRZ" | "frz" => rgb(0x50A8C8),
        _ => C_DIM,
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
    /// Sprite bob phase for the rival, paced by their own mon's Speed. The
    /// rival bobs for the same reason your own does — a still sprite opposite
    /// a moving one reads as a bug.
    pub foe_bob: bool,
    pub own_status: Option<&'a str>,
    /// Current and max HP of your own mon. Gen 3 prints the numbers on your
    /// plate and only the bar on the rival's, and so does this.
    pub own_hp_numbers: Option<(u16, u16)>,
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
    // Inside alignment matters: embedded-graphics centres a stroke by
    // default, so a 2px border on a panel at x paints a pixel at x-1 — which
    // is how panels laid out flush against the safe area leaked into the
    // frame's padding.
    Rectangle::new(Point::new(x, y), Size::new(w, h))
        .into_styled(
            PrimitiveStyleBuilder::new()
                .fill_color(bg)
                .stroke_color(border)
                .stroke_width(2)
                .stroke_alignment(embedded_graphics::primitives::StrokeAlignment::Inside)
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

/// Centred [`text_aa_font`]: mono fonts, so the width is exact.
pub fn text_aa_center_font<D>(
    d: &mut D,
    s: &str,
    cx: i32,
    y: i32,
    font: &'static MonoFont<'static>,
    fg: Rgb565,
    bg: Rgb565,
) where
    D: DrawTarget<Color = Rgb565>,
{
    let w = s.chars().count() as i32 * font.character_size.width as i32;
    text_aa_font(d, s, cx - w / 2, y, font, fg, bg);
}

/// Right-aligned [`text_aa_font`]: the text ends at `rx`.
pub fn text_aa_right_font<D>(
    d: &mut D,
    s: &str,
    rx: i32,
    y: i32,
    font: &'static MonoFont<'static>,
    fg: Rgb565,
    bg: Rgb565,
) where
    D: DrawTarget<Color = Rgb565>,
{
    let w = s.chars().count() as i32 * font.character_size.width as i32;
    text_aa_font(d, s, rx - w, y, font, fg, bg);
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
    text_aa_font(d, label, x + 4, y + 2, &FONT_5X8, C_BOX, c);
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
    status: Option<&str>,
    trim: Rgb565,
) where
    D: DrawTarget<Color = Rgb565>,
{
    let h: u32 = if numbers.is_some() { 34 } else { 28 };
    // Drop shadow, offset down-right like the reference art.
    fill(d, x + 3, y + 3, w, h, C_SHADOW);
    panel(d, x, y, w, h, C_BOX, C_INK);
    // Seat tab down the left edge.
    fill(d, x + 2, y + 2, 4, h - 4, trim);

    text_at(d, clip(species_display_name(name), 13), x + 10, y + 4, &FONT_6X10, C_INK);
    if level > 0 {
        let mut b = LvBuf::new();
        text_right(d, b.fmt(level), x + w as i32 - 6, y + 5, &FONT_5X8, C_INK);
    }

    // The HP tag, or the status badge in its place when there is one — which
    // is where Gen 3 puts it, and it keeps the full name on the row above.
    let bar_y = y + 15;
    let bar_x = match status {
        Some(st) => {
            fill(d, x + 10, bar_y - 1, 23, 10, status_color(st));
            text_aa_font(d, clip(st, 3), x + 13, bar_y + 1, &FONT_5X8, C_MSG_TEXT, status_color(st));
            x + 36
        }
        None => {
            fill(d, x + 10, bar_y - 1, 16, 10, C_HPCHIP);
            text_aa_font(d, "HP", x + 12, bar_y + 1, &FONT_5X8, C_MSG_TEXT, C_HPCHIP);
            x + 29
        }
    };
    let bar_w = (x + w as i32 - 10 - bar_x) as u32;
    fill(d, bar_x, bar_y, bar_w, 8, C_INK);
    fill(d, bar_x + 1, bar_y + 1, bar_w - 2, 6, C_TRACK);
    let filled = ((bar_w - 2) as u32 * pct.min(100) as u32) / 100;
    if filled > 0 {
        fill(d, bar_x + 1, bar_y + 1, filled, 6, hp_color(pct));
    }

    if let Some((cur, max)) = numbers {
        let mut n = NumBuf::new();
        text_right(d, n.pair(cur as u32, max as u32), x + w as i32 - 6, y + 24, &FONT_5X8, C_INK);
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
/// over the move row no matter what a future sprite set does.
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
    // Scale is a numerator over 2 so half-scale is available without
    // floating point. 2x needs the box to be double the art in BOTH
    // dimensions — the battle field's 64-tall boxes never qualify, so a
    // species with a small sprite still cannot tower over the field; only a
    // screen that deliberately offers a double-size box (the win recap) gets
    // enlarged art.
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





// ── Shared furniture ─────────────────────────────────────────────────────────



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
    let mut x = LEFT;
    for it in items {
        text_at(d, it, x, BOTTOM - 8, &FONT_5X8, C_DIM);
        x += it.len() as i32 * 5 + 10;
    }
}

/// A one-line affordance hint on the battle screens, in the gap along the
/// top edge between the rival's plate and their sprite — the only band of a
/// battle half that no plate, art or pane wants. Dim on purpose: it is a
/// reminder for the first game, not furniture.
fn hint<D>(d: &mut D, text: &str)
where
    D: DrawTarget<Color = Rgb565>,
{
    text_aa_font(d, text, LEFT + 118, FRAME + PAD + 2, &FONT_5X8, C_DIM, C_BG);
}

/// First row a half may draw on. The pokeball divider is painted over the
/// composed panel and reaches [`crate::device_view::DIVIDER_REACH`] rows into
/// each half, so anything above this is under the band.
pub const TOP: i32 = crate::device_view::DIVIDER_REACH as i32 + PAD;

/// The bottom band, where Gen 3 puts its menus and its narration — flush
/// against the safe area's floor whatever the panel size.
pub(crate) const MENU_H: u32 = 36;
pub(crate) const MENU_Y: i32 = BOTTOM - MENU_H as i32;

/// Horizontal centre of a half, for centred text.
const CX: i32 = HALF_W as i32 / 2;
/// Vertical centre of a half, for the screens that are one centred block.
const CY: i32 = HALF_H as i32 / 2;

/// First row of the type-effectiveness rows on the move-info screen: three
/// 13px rows plus clearance for the legend under them.
const CHART_Y: i32 = BOTTOM - 52;

/// Party list geometry, shared with the tap hit-test in the web client.
pub(crate) const PARTY_Y: i32 = TOP + 22;
pub(crate) const PARTY_PITCH: i32 = 24;
/// A party row's height (the pitch leaves 4px of air between rows).
pub(crate) const PARTY_ROW_H: u32 = 20;

/// The play area's frame, in this seat's own color: White's half is edged in
/// white and Red's in red. The two halves meet at the seam, so the divider's
/// black band and button land between a white field and a red one — the whole
/// device reads as one pokeball, and each player can still tell at a glance
/// which side is theirs.
///
/// The shared battle scene deliberately has no frame: one per half would cut
/// the single field back into two boxes.
fn play_frame<D>(d: &mut D, seat: u8)
where
    D: DrawTarget<Color = Rgb565>,
{
    d.clear(seat_trim(seat)).ok();
    fill(d, 6, 6, HALF_W - 12, HALF_H - 12, C_INK);
    fill(d, 8, 8, HALF_W - 16, HALF_H - 16, C_BG);
}

/// Re-stamp just the frame's ring, leaving the field inside it alone.
///
/// The compositor calls this after a half has finished drawing, so the border
/// is always the last thing painted and nothing can bleed into it. Screens
/// should still lay out inside [`LEFT`]..[`RIGHT`] — this is a backstop, and a
/// screen that relies on it gets its text cut instead of wrapped.
pub fn draw_play_frame_edge<D>(d: &mut D, seat: u8)
where
    D: DrawTarget<Color = Rgb565>,
{
    let (w, h) = (HALF_W as i32, HALF_H as i32);
    let trim = seat_trim(seat);
    // Outer band, then the hairline, drawn as four sides so the field keeps
    // whatever the screen put there.
    for (x, y, bw, bh) in [(0, 0, w, 8), (0, h - 8, w, 8), (0, 0, 8, h), (w - 8, 0, 8, h)] {
        fill(d, x, y, bw as u32, bh as u32, trim);
    }
    for (x, y, bw, bh) in [(6, 6, w - 12, 2), (6, h - 8, w - 12, 2), (6, 6, 2, h - 12), (w - 8, 6, 2, h - 12)] {
        fill(d, x, y, bw as u32, bh as u32, C_INK);
    }
}

/// The frame's thickness, the gap inside it, and the resulting safe area.
///
/// **Every screen must lay out inside `LEFT..RIGHT` and `TOP..BOTTOM`.** The
/// frame is painted first and stamped again after a half finishes drawing, so
/// anything outside that box is not just ugly, it is invisible — and anything
/// that assumes it can draw *under* the border will be covered.
///
/// `PAD` is part of the safe area on purpose: text that merely avoids
/// overlapping the border still reads as jammed against it, so the bounds
/// carry the breathing room and no screen has to remember to add it. The
/// `border_is_never_drawn_over` test fails the build if a screen strays past
/// them — including into the padding.
pub const FRAME: i32 = 8;
pub const PAD: i32 = 4;
pub const LEFT: i32 = FRAME + PAD;
pub const RIGHT: i32 = HALF_W as i32 - FRAME - PAD;
pub const BOTTOM: i32 = HALF_H as i32 - FRAME - PAD;
/// Width of the safe area, for anything that spans it.
pub const CONTENT_W: u32 = (RIGHT - LEFT) as u32;

/// Breathing room inside a panel, the same idea as [`PAD`] one level down: a
/// panel's 2px border is not clearance, so text starts `BOX_PAD` in from the
/// box's own edge and right-aligned text stops `BOX_PAD` short of it.
pub const BOX_PAD: i32 = 6;



/// The Gen 3 HP row: a red HP tag against the bar, not a bare track.
fn hp_row<D>(d: &mut D, x: i32, y: i32, w: u32, pct: u8)
where
    D: DrawTarget<Color = Rgb565>,
{
    fill(d, x, y - 1, 16, 10, C_HPCHIP);
    text_aa_font(d, "HP", x + 2, y + 1, &FONT_5X8, C_MSG_TEXT, C_HPCHIP);
    hp_bar(d, x + 19, y, w - 19, pct);
}

/// A Gen 3 bottom menu: teal behind a blue double border, pinned at the
/// corners. The move list and its PP/type readout both sit in one.
fn draw_menu_box<D>(d: &mut D, x: i32, y: i32, w: u32, h: u32)
where
    D: DrawTarget<Color = Rgb565>,
{
    fill(d, x, y, w, h, C_MENU_EDGE);
    fill(d, x + 2, y + 2, w - 4, h - 4, C_MENU_HI);
    fill(d, x + 3, y + 3, w - 6, h - 6, C_MSG_FILL);
    for (px, py) in [
        (x + 1, y + 1),
        (x + w as i32 - 3, y + 1),
        (x + 1, y + h as i32 - 3),
        (x + w as i32 - 3, y + h as i32 - 3),
    ] {
        fill(d, px, py, 2, 2, C_MENU_PIN);
    }
}

/// The Gen 3 cursor: a solid triangle to the left of the highlighted row.
fn draw_cursor<D>(d: &mut D, x: i32, y: i32, c: Rgb565)
where
    D: DrawTarget<Color = Rgb565>,
{
    for i in 0..5 {
        let h = 9 - i * 2;
        if h > 0 {
            fill(d, x + i, y + i, 1, h as u32, c);
        }
    }
}

/// The battlefield, laid out the way Gen 3 lays it out: the rival's plate at
/// the top left with their mon opposite it on the far platform, and your own
/// mon on the near platform with your plate at the bottom right. Every screen
/// that keeps the field showing draws this and then puts its own furniture in
/// the bottom third.
/// Where the two mons stand on a half, as centres. The field draws them
/// here and the move animation travels between them, so the two cannot
/// disagree about where a Flamethrower is aimed.
pub const FOE_MON_CENTRE: (i32, i32) = (FOE_SPRITE_X + 44, FRAME + PAD + 32);
pub const OWN_MON_CENTRE: (i32, i32) = (LEFT + 46, MENU_Y - 32);

/// Left edge of the rival's 88-wide sprite box, tucked against the right of
/// the safe area.
const FOE_SPRITE_X: i32 = RIGHT - 90;

fn battle_field<D>(d: &mut D, ctx: &HalfCtx<'_>, show_lock: bool)
where
    D: DrawTarget<Color = Rgb565>,
{
    // Far side: the rival's plate at the top left, their mon opposite it on
    // the far platform. The sprite box is a full 64 tall so native art draws
    // at 1:1 — a pixel short and `draw_sprite_fit` halves it. It starts on the
    // first legal row: the divider only reaches into the middle columns, and
    // this sits well right of them.
    draw_platform(d, FOE_SPRITE_X + 44, 76, 28);
    if let Some(spr) = mon_sprite_color(ctx.foe_name) {
        let bob = if ctx.foe_bob { -2 } else { 0 };
        draw_sprite_fit(d, spr, FOE_SPRITE_X, FRAME + PAD + bob, 88, 64, false, true);
    }
    draw_status_plate(
        d, LEFT, TOP, 112, ctx.foe_name, ctx.foe_level, ctx.foe_hp, None, ctx.foe_status,
        seat_trim(3 - ctx.seat),
    );
    // The rival having locked in is the one thing worth a badge over the art:
    // it is the only signal that the turn is now waiting on you.
    // Only while this seat still has a choice to make: during narration the
    // corner is the caption's own hint, and both are locked in anyway.
    if show_lock && ctx.foe_locked {
        chip(d, RIGHT - 28, FRAME + PAD, "LOCK", C_DIM);
    }

    // Near side: your mon from behind on the close platform, your plate — the
    // one carrying the HP numbers, as in the games — at the bottom right.
    //
    // Back sprites are drawn to the bottom edge of their own frame, so the art
    // has no feet to spare: the mon is stood flush against the top of the move
    // menu, and the bob dips it a few pixels behind the pane rather than
    // lifting it off nothing and leaving a cut edge in mid air.
    draw_platform(d, LEFT + 42, MENU_Y - 2, 34);
    if let Some(spr) = mon_back_sprite_color(ctx.own_name) {
        let dip = if ctx.bob { 0 } else { 3 };
        draw_sprite_fit(d, spr, LEFT, MENU_Y - 64 + dip, 92, 64, false, true);
    }
    // Clear of the pane below it: with the numbers row the plate is 34 tall,
    // and sat flush against the menu the HP figures were cut by its border.
    draw_status_plate(
        d, RIGHT - 124, MENU_Y - 40, 120, ctx.own_name, ctx.own_level, ctx.own_hp,
        ctx.own_hp_numbers, ctx.own_status, seat_trim(ctx.seat),
    );
}



// ── Screens ──────────────────────────────────────────────────────────────────

/// Move-choice screen: your mon on the open field at the left, the rival's
/// card at the right, and the four moves in a row along the bottom edge with
/// an info bar describing the highlighted one.
pub fn render_choice<D>(d: &mut D, moves: &[MoveSlot], ctx: &HalfCtx<'_>)
where
    D: DrawTarget<Color = Rgb565>,
{
    play_frame(d, ctx.seat);
    battle_field(d, ctx, true);
    // The two moves that are not the obvious one. A first-time player finds
    // the D-pad and A on their own; nothing on this screen used to say that
    // the party list and the move description exist at all.
    hint(d, "B PARTY   ? INFO");

    // The move list: a 2x2 grid in a menu box with the cursor beside the
    // highlighted row, and the PP and type of that row in a second box at the
    // right — the Gen 3 arrangement, at this panel's scale.
    let move_w = CONTENT_W - 68; // the PP/type box keeps its 64 + a 4px gap
    draw_menu_box(d, LEFT, MENU_Y, move_w, MENU_H);
    let col = (move_w as i32 - 2 * (BOX_PAD + 8)) / 2;
    for (i, mv) in moves.iter().take(4).enumerate() {
        let x = LEFT + BOX_PAD + 8 + (i as i32 % 2) * col;
        let y = MENU_Y + 7 + (i as i32 / 2) * 15;
        let sel = i as u8 == ctx.cursor;
        // A move with no PP left greys out rather than disappearing, so the
        // grid never reflows under the cursor.
        let fg = if mv.pp == 0 { C_DIM } else { C_MSG_TEXT };
        text_aa_font(d, clip(&mv.name, 12), x, y, &FONT_5X8, fg, C_MSG_FILL);
        if sel {
            draw_cursor(d, x - 9, y - 1, C_MSG_TEXT);
        }
    }

    let pp_x = LEFT + move_w as i32 + 4;
    draw_menu_box(d, pp_x, MENU_Y, 64, MENU_H);
    if let Some(mv) = moves.get(ctx.cursor as usize) {
        let mut n = NumBuf::new();
        text_aa_font(d, "PP", pp_x + BOX_PAD, MENU_Y + 7, &FONT_5X8, C_MSG_TEXT, C_MSG_FILL);
        text_right(
            d,
            n.pair(mv.pp as u32, mv.max_pp as u32),
            RIGHT - BOX_PAD,
            MENU_Y + 7,
            &FONT_5X8,
            C_MSG_TEXT,
        );
        text_aa_font(d, "TYPE/", pp_x + BOX_PAD, MENU_Y + 17, &FONT_5X8, C_MSG_TEXT, C_MSG_FILL);
        text_aa_font(d, clip(&mv.type_name, 7), pp_x + BOX_PAD, MENU_Y + 26, &FONT_5X8, C_MSG_TEXT, C_MSG_FILL);
    }

}

/// This player has committed; the move stays hidden from the other seat.
pub fn render_locked<D>(d: &mut D, chosen: Option<&str>, ctx: &HalfCtx<'_>)
where
    D: DrawTarget<Color = Rgb565>,
{
    play_frame(d, ctx.seat);
    battle_field(d, ctx, true);
    // A committed seat's one remaining move, said out loud: without this the
    // waiting screen reads as "nothing you can do now".
    hint(d, "B TAKE IT BACK");
    // Same field, and the bottom third becomes the narration box instead of
    // the move menu — which is exactly what the games do between choices.
    draw_message_box(d, LEFT, MENU_Y, CONTENT_W, 36);
    let top = match chosen {
        Some(c) => c,
        None => "Waiting...",
    };
    text_aa(d, clip(top, 26), LEFT + 8, MENU_Y + 8, C_MSG_TEXT, C_MSG_FILL);
    let tail = if ctx.foe_locked { "Both ready!" } else { "Rival is choosing..." };
    text_aa(d, tail, LEFT + 8, MENU_Y + 22, C_MSG_TEXT, C_MSG_FILL);
}

/// Party list with a cursor: switching, or the forced pick after a faint.
pub fn render_party<D>(d: &mut D, party: &[PartySlotData], ctx: &HalfCtx<'_>, forced: bool)
where
    D: DrawTarget<Color = Rgb565>,
{
    play_frame(d, ctx.seat);
    text_aa_center_font(d, if forced { "SEND WHO NEXT?" } else { "PARTY" }, CX, TOP, &FONT_8X13, C_INK, C_BG);
    fill(d, LEFT, TOP + 16, CONTENT_W, 2, C_INK);

    let bar_x = LEFT + 84;
    for (i, slot) in party.iter().take(6).enumerate() {
        let y = PARTY_Y + i as i32 * PARTY_PITCH;
        let sel = i as u8 == ctx.cursor;
        let dead = slot.hp == 0;
        let fg = if sel {
            C_BOX
        } else if dead {
            C_DIM
        } else {
            C_INK
        };
        let row_bg = if sel { C_SEL } else { C_BOX };
        panel(d, LEFT, y, CONTENT_W, PARTY_ROW_H, if sel { C_SEL } else { C_BOX }, if dead { C_DIM } else { C_INK });
        text_aa_font(d, clip(species_display_name(&slot.name), 11), LEFT + BOX_PAD, y + 5, &FONT_6X10, fg, row_bg);
        let pct = if slot.max_hp == 0 {
            0
        } else {
            ((slot.hp as u32 * 100) / slot.max_hp as u32) as u8
        };
        hp_bar(d, bar_x, y + 6, 100, pct);
        let mut n = NumBuf::new();
        text_aa_font(d, n.pair(slot.hp as u32, slot.max_hp as u32), bar_x + 108, y + 6, &FONT_5X8, fg, row_bg);
        if slot.active {
            text_aa_right_font(d, "OUT", RIGHT - BOX_PAD, y + 6, &FONT_5X8, if sel { C_BOX } else { C_DIM }, row_bg);
        } else if let Some(st) = &slot.status {
            text_aa_right_font(d, clip(st, 3), RIGHT - BOX_PAD, y + 6, &FONT_5X8, if sel { C_BOX } else { C_ACCENT }, row_bg);
        }
    }

    if forced {
        legend(d, &["+ PICK", "A SEND IN", "? INFO"]);
    } else {
        legend(d, &["+ PICK", "A SWITCH", "B BACK", "? INFO"]);
    }
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
    play_frame(d, ctx.seat);
    battle_field(d, ctx, false);
    // Narration takes the move menu's place, so the field above it does not
    // move between choosing and watching. Two seats, two fields: each player
    // reads the battle the right way up on their own half, which is why the
    // mons are drawn here rather than in a band across the seam.
    draw_message_box(d, LEFT, MENU_Y, CONTENT_W, MENU_H);
    // Top right, the slot the LOCK chip uses on the choosing screen: it is
    // the only corner of a half that no plate, sprite or pane wants.
    text_aa_font(d, "A NEXT", RIGHT - 36, FRAME + PAD, &FONT_5X8, C_DIM, C_BG);
    // The box is the height of the move menu it replaces, so the line reads
    // at the display size while it fits in two rows and drops to the body
    // size only for the long ones. Sized the other way round, every line
    // would be small for the sake of the few that are long.
    let wide8 = ((CONTENT_W as i32 - 16) / 8) as usize;
    let wide6 = ((CONTENT_W as i32 - 16) / 6) as usize;
    let (font, cols, rows, pitch) = if caption.len() <= 2 * wide8 {
        (&FONT_8X13, wide8, 2, 15)
    } else {
        (&FONT_6X10, wide6, 3, 11)
    };
    let mut rest = caption;
    for row in 0..rows {
        if rest.is_empty() {
            break;
        }
        let take = if rest.len() <= cols {
            rest.len()
        } else {
            rest[..cols].rfind(' ').unwrap_or(cols)
        };
        text_aa_font(
            d,
            clip(&rest[..take], cols),
            LEFT + 8,
            MENU_Y + 5 + row as i32 * pitch,
            font,
            C_MSG_TEXT,
            C_MSG_FILL,
        );
        rest = rest[take..].trim_start();
    }
}

/// Lobby half.
pub fn render_lobby<D>(d: &mut D, ready: bool, ai: bool, seat: u8)
where
    D: DrawTarget<Color = Rgb565>,
{
    play_frame(d, seat);
    if ready && ai {
        text_aa_center_font(d, "AI", CX, CY - 20, &FONT_8X13, C_ACCENT, C_BG);
        text_aa_center_font(d, "a robot rival takes this side", CX, CY + 4, &FONT_5X8, C_DIM, C_BG);
    } else if ready {
        text_aa_center_font(d, "READY!", CX, CY - 28, &FONT_10X20, C_HP_G, C_BG);
        text_aa_center_font(d, "waiting for rival...", CX, CY + 4, &FONT_8X13, C_DIM, C_BG);
        text_aa_center_font(d, "B to cancel", CX, CY + 26, &FONT_6X10, C_DIM, C_BG);
    } else {
        // The one instruction on this screen carries the screen, so it is
        // set at the display size rather than the body size.
        text_aa_center_font(d, "PRESS A TO START", CX, CY - 34, &FONT_10X20, C_INK, C_BG);
        text_aa_center_font(d, "HOLD A: FIGHT THE AI", CX, CY + 2, &FONT_8X13, C_DIM, C_BG);
        text_aa_center_font(d, "?: OPTIONS", CX, CY + 24, &FONT_8X13, C_DIM, C_BG);
    }
}

/// Win / lose / tie recap.
pub fn render_result<D>(d: &mut D, msg: &str, ctx: &HalfCtx<'_>)
where
    D: DrawTarget<Color = Rgb565>,
{
    play_frame(d, ctx.seat);
    let win = msg.starts_with("WIN");
    text_aa_center_font(d, msg, CX, TOP + 8, &FONT_8X13, if win { C_HP_G } else { C_INK }, C_BG);
    if let Some(spr) = mon_sprite_color(ctx.own_name) {
        // Twice the box of the old panel: the recap owns the screen, so the
        // winner's art gets drawn at 2x rather than floating in blank space.
        draw_sprite_fit(d, spr, CX - 64, TOP + 34, 128, 128, false, true);
    }
    legend(d, &["A REMATCH", "? BATTLE LOG"]);
}

/// Draw end screen. Same legend as the result screen; both last mons'
/// sprites flank the DRAW! title the way the result screen flanks its own
/// sprite. Same panel for both seats, decided in core.
pub fn render_draw<D>(d: &mut D, ctx: &HalfCtx<'_>)
where
    D: DrawTarget<Color = Rgb565>,
{
    play_frame(d, ctx.seat);
    text_aa_center_font(d, "DRAW!", CX, TOP + 8, &FONT_8X13, C_INK, C_BG);
    // Both mons as front sprites, the recap owns the screen, so they both
    // get the bigger 88-px box rather than floating thin in the half.
    if let Some(spr) = mon_sprite_color(ctx.own_name) {
        // Own on the seat-side (closest to the seat reading it).
        draw_sprite_fit(d, spr, LEFT - 4, TOP + 34, 88, 88, false, true);
    }
    if let Some(spr) = mon_sprite_color(ctx.foe_name) {
        // Foe on the rival-side.
        draw_sprite_fit(d, spr, RIGHT - 84, TOP + 34, 88, 88, false, true);
    }
    legend(d, &["A REMATCH", "? BATTLE LOG"]);
}

/// Full description for the cursor's move — what `?` opens.
pub fn render_move_info<D>(d: &mut D, mv: &MoveSlot, desc: &str, seat: u8)
where
    D: DrawTarget<Color = Rgb565>,
{
    play_frame(d, seat);
    text_aa_font(d, clip(&mv.name, 21), LEFT, TOP, &FONT_8X13, C_INK, C_BG);
    fill(d, LEFT, TOP + 16, CONTENT_W, 2, C_INK);
    let mut x = chip(d, LEFT, TOP + 22, clip(&mv.type_name, 3), type_color(&mv.type_name));
    x = chip(d, x, TOP + 22, clip(&mv.category, 4), C_DIM);
    let mut n = NumBuf::new();
    let row = TOP + 24;
    text_aa_font(d, "POW", x, row, &FONT_5X8, C_DIM, C_BG);
    text_aa_font(d, mv.power.map(|p| n.one(p)).unwrap_or("---"), x + 20, row, &FONT_5X8, C_INK, C_BG);
    let mut n2 = NumBuf::new();
    text_aa_font(d, "ACC", x + 46, row, &FONT_5X8, C_DIM, C_BG);
    text_aa_font(d, mv.accuracy.map(|a| n2.one(a as u32)).unwrap_or("---"), x + 66, row, &FONT_5X8, C_INK, C_BG);
    let mut n3 = NumBuf::new();
    text_aa_font(d, "PP", x + 92, row, &FONT_5X8, C_DIM, C_BG);
    text_aa_font(d, n3.pair(mv.pp as u32, mv.max_pp as u32), x + 108, row, &FONT_5X8, C_INK, C_BG);

    // Description, word wrapped, stopping short of the chart rows.
    let cols = (CONTENT_W / 6) as usize;
    let mut y = TOP + 43;
    let mut rest = desc;
    while !rest.is_empty() && y < CHART_Y - 12 {
        let take = if rest.len() <= cols {
            rest.len()
        } else {
            rest[..cols].rfind(' ').unwrap_or(cols)
        };
        text_aa_font(d, clip(&rest[..take], cols), LEFT, y, &FONT_6X10, C_INK, C_BG);
        rest = rest[take..].trim_start();
        y += 12;
    }

    // What this move is good and bad against. The chart is the thing players
    // actually want from `?`, and reading it off a table beats memorising it.
    if let Some(t) = type_from_name(&mv.type_name) {
        for (row, (label, mult)) in [("2x", 20u8), ("1/2", 5u8), ("0x", 0u8)].iter().enumerate() {
            let y = CHART_Y + row as i32 * 13;
            text_aa_font(d, label, LEFT, y + 2, &FONT_5X8, C_DIM, C_BG);
            let mut x = LEFT + 20;
            for def in GEN1_TYPES {
                if gen1_effectiveness(t, def) != *mult {
                    continue;
                }
                let abbr = crate::display::type_abbr(def);
                if x + 24 > RIGHT {
                    break;
                }
                x = chip(d, x, y, abbr, type_color(abbr));
            }
            if x == LEFT + 20 {
                text_aa_font(d, "-", x, y + 2, &FONT_5X8, C_DIM, C_BG);
            }
        }
    }

    legend(d, &["B BACK"]);
}

/// One party member's summary — what `?` opens from the party list. The
/// party screen itself is a list, so pressing `?` there has to answer "what
/// IS this thing", not draw the same list with one row in it.
pub fn render_stats<D>(d: &mut D, slot: &PartySlotData, seat: u8)
where
    D: DrawTarget<Color = Rgb565>,
{
    play_frame(d, seat);
    text_aa_font(d, clip(species_display_name(&slot.name), 14), LEFT, TOP, &FONT_8X13, C_INK, C_BG);
    if slot.level > 0 {
        let mut b = LvBuf::new();
        text_aa_right_font(d, b.fmt(slot.level), RIGHT, TOP + 2, &FONT_5X8, C_DIM, C_BG);
    }
    fill(d, LEFT, TOP + 16, CONTENT_W, 2, C_INK);

    // Art at the left at native size — the box has to be a full sprite tall
    // or `draw_sprite_fit` halves it — with its types under it.
    if let Some(spr) = mon_sprite_color(&slot.name) {
        draw_sprite_fit(d, spr, LEFT, TOP + 20, 76, 64, false, true);
    }
    let mut tx = LEFT;
    for ty in slot.types.iter().take(2) {
        // The abbreviations are the mono renderer's, so both displays name a
        // type the same way — and `type_color` knows both spellings.
        let abbr = crate::display::type_abbr(*ty);
        tx = chip(d, tx, TOP + 86, abbr, type_color(abbr));
    }

    // HP, then the four stats with their stage boosts.
    let pct = if slot.max_hp == 0 {
        0
    } else {
        ((slot.hp as u32 * 100) / slot.max_hp as u32) as u8
    };
    hp_row(d, 96, TOP + 22, (RIGHT - 96) as u32, pct);
    let mut n = NumBuf::new();
    text_aa_right_font(d, n.pair(slot.hp as u32, slot.max_hp as u32), RIGHT, TOP + 36, &FONT_5X8, C_INK, C_BG);
    if let Some(st) = &slot.status {
        chip(d, 96, TOP + 34, clip(st, 3), status_color(st));
    }

    let stats = [
        ("ATK", slot.atk, slot.boost_atk),
        ("DEF", slot.def, slot.boost_def),
        ("SPD", slot.spe, slot.boost_spe),
        ("SPC", slot.spc, slot.boost_spc),
    ];
    for (i, (label, value, boost)) in stats.iter().enumerate() {
        let x = 96 + (i as i32 % 2) * 68;
        let y = TOP + 50 + (i as i32 / 2) * 12;
        text_aa_font(d, label, x, y, &FONT_5X8, C_DIM, C_BG);
        let mut n = NumBuf::new();
        text_aa_font(d, n.one(*value as u32), x + 22, y, &FONT_5X8, C_INK, C_BG);
        if *boost != 0 {
            let (mark, color) = if *boost > 0 { ("+", C_HP_G) } else { ("-", C_ACCENT) };
            text_aa_font(d, mark, x + 48, y, &FONT_5X8, color, C_BG);
            let mut b = NumBuf::new();
            text_aa_font(d, b.one(boost.unsigned_abs() as u32), x + 54, y, &FONT_5X8, color, C_BG);
        }
    }

    // Moves, with the PP that is actually left on each.
    // Moves in two columns, so all four clear the legend.
    for (i, (name, pp, max_pp)) in slot.moves.iter().take(4).enumerate() {
        let x = LEFT + (i as i32 % 2) * 104;
        let y = TOP + 100 + (i as i32 / 2) * 11;
        let fg = if *pp == 0 { C_DIM } else { C_INK };
        text_aa_font(d, clip(species_display_name(name), 13), x, y, &FONT_5X8, fg, C_BG);
        let mut n = NumBuf::new();
        text_aa_font(d, n.pair(*pp as u32, *max_pp as u32), x + 68, y, &FONT_5X8, C_DIM, C_BG);
    }
    if let Some(item) = &slot.item {
        text_aa_right_font(d, clip(item, 12), RIGHT, TOP + 86, &FONT_5X8, C_DIM, C_BG);
    }

    legend(d, &["B BACK"]);
}

/// Scrollable battle log — the teaching surface behind `?`.
pub fn render_log<D>(d: &mut D, lines: &[&str], top: usize, seat: u8)
where
    D: DrawTarget<Color = Rgb565>,
{
    play_frame(d, seat);
    text_aa_center_font(d, "BATTLE LOG", CX, TOP, &FONT_8X13, C_INK, C_BG);
    fill(d, LEFT, TOP + 16, CONTENT_W, 2, C_INK);
    let rows = ((BOTTOM - (TOP + 22) - 12) / 11) as usize;
    let cols = (CONTENT_W / 5) as usize;
    for (i, line) in lines.iter().skip(top).take(rows).enumerate() {
        text_aa_font(d, clip(line, cols), LEFT, TOP + 22 + i as i32 * 11, &FONT_5X8, C_INK, C_BG);
    }
    legend(d, &["+ SCROLL", "B BACK"]);
}

/// Invalid-selection flash.
pub fn render_invalid<D>(d: &mut D, reason: InvalidReason, seat: u8)
where
    D: DrawTarget<Color = Rgb565>,
{
    play_frame(d, seat);
    let (a, b) = match reason {
        InvalidReason::Fainted => ("Already fainted!", ""),
        InvalidReason::AlreadyOut => ("Already out!", ""),
        InvalidReason::NoPp => ("No power remaining", "for that move!"),
        InvalidReason::Trapped => ("Trapped -", "can't switch out!"),
    };
    text_aa_center_font(d, a, CX, CY - 18, &FONT_8X13, C_ACCENT, C_BG);
    if !b.is_empty() {
        text_aa_center_font(d, b, CX, CY + 2, &FONT_8X13, C_ACCENT, C_BG);
    }
}

/// One row of the options menu.
pub struct OptionRow<'a> {
    pub label: &'a str,
    pub value: &'a str,
}

/// What the device knows about its battery. The platform measures (the fw
/// samples the VBAT divider on GP28; the web has no cell and reports
/// Absent unless a `?batt=NN` sim asks otherwise); this module only draws.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Battery {
    /// No cell seen at the sense tap (or the power switch is off).
    #[default]
    Absent,
    /// A cell at this many percent, from the resting-LiPo curve.
    Level(u8),
}

/// Resting-voltage LiPo curve, (millivolts, percent), interpolated between.
/// Shared so the firmware's gauge and any sim agree on what 3.87 V means.
const LIPO_CURVE: [(u32, u32); 10] = [
    (4200, 100),
    (4060, 90),
    (3980, 80),
    (3920, 70),
    (3870, 60),
    (3820, 50),
    (3770, 30),
    (3740, 20),
    (3680, 10),
    (3300, 0),
];

pub fn lipo_percent(mv: u32) -> u8 {
    if mv >= LIPO_CURVE[0].0 {
        return 100;
    }
    for pair in LIPO_CURVE.windows(2) {
        let (hv, hp) = pair[0];
        let (lv, lp) = pair[1];
        if mv >= lv {
            return (lp + (hp - lp) * (mv - lv) / (hv - lv)) as u8;
        }
    }
    0
}

/// The single battery indicator: a 22x10 battery glyph with a level fill in
/// the HP colors plus a compact label, or a slashed outline with `--` when
/// no cell is present. `rx` is the right edge; the indicator grows leftward.
fn draw_battery<D>(d: &mut D, rx: i32, y: i32, batt: Battery)
where
    D: DrawTarget<Color = Rgb565>,
{
    const BW: i32 = 22;
    const BH: i32 = 10;
    // `rx` is the rightmost pixel the indicator may touch: the nub ends
    // there and the body sits left of it, all inside the safe area.
    let bx = rx - BW - 2;
    panel(d, bx, y, BW as u32, BH as u32, C_BG, C_INK);
    fill(d, rx - 2, y + 2, 2, (BH - 4) as u32, C_INK);
    match batt {
        Battery::Level(pct) => {
            let inner = BW - 4;
            let w = (inner * pct.min(100) as i32) / 100;
            if w > 0 {
                fill(d, bx + 2, y + 2, w as u32, (BH - 4) as u32, hp_color(pct));
            }
            let mut n = NumBuf::new();
            let label = n.one(pct.min(100) as u32);
            text_aa_right_font(d, label, bx - 10, y + 1, &FONT_5X8, C_DIM, C_BG);
            text_aa_font(d, "%", bx - 9, y + 1, &FONT_5X8, C_DIM, C_BG);
        }
        Battery::Absent => {
            // A slash through the empty body, and -- where the number goes.
            for i in 0..BH - 2 {
                fill(d, bx + 2 + (i * (BW - 4)) / (BH - 2), y + 1 + i, 2, 1, C_ACCENT);
            }
            text_aa_right_font(d, "--", bx - 4, y + 1, &FONT_5X8, C_DIM, C_BG);
        }
    }
}

/// Options menu — lobby only, drawn into one seat's half (see `device_view`).
pub fn render_options<D>(
    d: &mut D,
    rows: &[OptionRow<'_>],
    cursor: u8,
    w: u32,
    h: u32,
    seat: u8,
    batt: Battery,
) where
    D: DrawTarget<Color = Rgb565>,
{
    play_frame(d, seat);
    let cx = (w / 2) as i32;
    text_aa_center_font(d, "OPTIONS", cx, TOP, &FONT_8X13, C_INK, C_BG);
    draw_battery(d, w as i32 - FRAME - PAD - 1, TOP + 1, batt);
    fill(d, 12, TOP + 16, w - 24, 2, C_INK);
    for (i, r) in rows.iter().enumerate() {
        let y = OPT_ROW.0 + i as i32 * OPT_ROW.1;
        let sel = i as u8 == cursor;
        panel(d, 12, y, w - 24, OPT_ROW.2, if sel { C_SEL } else { C_BOX }, C_INK);
        let fg = if sel { C_BOX } else { C_INK };
        let row_bg = if sel { C_SEL } else { C_BOX };
        text_aa_font(d, r.label, 18, y + 8, &FONT_6X10, fg, row_bg);
        text_aa_right_font(d, r.value, (w - 18) as i32, y + 8, &FONT_6X10, fg, row_bg);
    }
    let mut x = 14;
    for it in ["+ CHANGE", "A CONFIRM", "B BACK"] {
        text_aa_font(d, it, x, h as i32 - FRAME - PAD - 8, &FONT_5X8, C_DIM, C_BG);
        x += it.len() as i32 * 5 + 12;
    }
}

/// Row geometry of the two full-panel menus: first row's top, the pitch
/// between rows, and a row's height. The tap hit test reads the same numbers
/// the renderer lays out with, because a tap that lands one row off is
/// indistinguishable from the menu ignoring the press: tapping GEN 3 started
/// a Gen 1 battle when the two disagreed.
pub const GEN_ROW: (i32, i32, u32) = (70, 56, 44);
pub const OPT_ROW: (i32, i32, u32) = (40, 30, 26);

/// Which menu row a point in half-local coordinates is over, if any. `rows`
/// is what the menu says it is showing.
pub fn menu_row_at(screen: crate::menu::MenuScreen, y: i32, rows: u8) -> Option<u8> {
    use crate::menu::MenuScreen;
    let (top, pitch, h) = match screen {
        MenuScreen::GenPicker => GEN_ROW,
        MenuScreen::Options | MenuScreen::Lobby => OPT_ROW,
    };
    if y < top {
        return None;
    }
    let row = (y - top) / pitch;
    ((y - top) % pitch < h as i32 && row < rows as i32).then_some(row as u8)
}

/// Generation picker — the first screen, before the lobby.
pub fn render_gen_picker<D>(d: &mut D, cursor: u8, w: u32, h: u32, seat: u8)
where
    D: DrawTarget<Color = Rgb565>,
{
    play_frame(d, seat);
    let cx = (w / 2) as i32;
    text_aa_center_font(d, "CHOOSE A FORMAT", cx, 18, &FONT_8X13, C_INK, C_BG);
    let opts = [
        ("Generation 1", "Red / Blue / Yellow rules", true),
        ("Generation 3", "Ruby / Sapphire / Emerald rules", false),
    ];
    for (i, (name, sub, ready)) in opts.iter().enumerate() {
        let y = GEN_ROW.0 + i as i32 * GEN_ROW.1;
        let sel = i as u8 == cursor;
        panel(d, 24, y, w - 48, GEN_ROW.2, if sel { C_SEL } else { C_BOX }, C_INK);
        let fg = if sel { C_BOX } else { C_INK };
        let card_bg = if sel { C_SEL } else { C_BOX };
        text_aa_font(d, name, 34, y + 9, &FONT_8X13, fg, card_bg);
        text_aa_font(d, sub, 34, y + 26, &FONT_5X8, if sel { C_BOX } else { C_DIM }, card_bg);
        if !ready {
            text_aa_right_font(d, "STUB", (w - 34) as i32, y + 9, &FONT_5X8, C_ACCENT, card_bg);
        }
    }
    text_aa_center_font(d, "+ CHOOSE    A START", cx, h as i32 - FRAME - PAD - 10, &FONT_5X8, C_DIM, C_BG);
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
        text_aa_center(d, clip(species_display_name(name), 16), cx, ground_y + 16, C_INK, C_BG);
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

#[cfg(test)]
mod frame_tests {
    use super::*;
    use crate::board_event::MoveSlot;
    use crate::device_view::HalfFrame;

    fn mv(name: &str, ty: &str) -> MoveSlot {
        MoveSlot {
            name: name.into(),
            type_name: ty.into(),
            category: "Physical".into(),
            power: Some(85),
            accuracy: Some(100),
            pp: 15,
            max_pp: 15,
        }
    }

    fn ctx<'a>(name: &'a str, foe: &'a str) -> HalfCtx<'a> {
        HalfCtx {
            seat: 1,
            own_name: name,
            own_hp: 61,
            own_level: 100,
            own_status: Some("PAR"),
            own_hp_numbers: Some((188, 309)),
            foe_name: foe,
            foe_hp: 44,
            foe_level: 100,
            foe_status: Some("PSN"),
            foe_bob: true,
            cursor: 1,
            foe_locked: true,
            bob: false,
        }
    }

    /// Where a screen may not put a pixel: the frame's ring, plus the middle
    /// columns the pokeball's button reaches into from the seam. Both are
    /// painted after the half, so anything here is invisible.
    fn in_border(x: u32, y: u32) -> bool {
        let (x, y) = (x as i32, y as i32);
        let ring = x < LEFT || x >= RIGHT || y < FRAME + PAD || y >= BOTTOM;
        let reach = crate::device_view::DIVIDER_REACH as i32;
        let button = y < reach && (x - HALF_W as i32 / 2).abs() < reach;
        ring || button
    }

    fn check(label: &str, draw: impl FnOnce(&mut HalfFrame)) {
        let mut got = HalfFrame::new(HALF_W, HALF_H);
        draw(&mut got);
        let mut want = HalfFrame::new(HALF_W, HALF_H);
        play_frame(&mut want, 1);
        for y in 0..HALF_H {
            for x in 0..HALF_W {
                if !in_border(x, y) {
                    continue;
                }
                let i = (y * HALF_W + x) as usize;
                assert_eq!(
                    got.px[i], want.px[i],
                    "{label} drew into the frame at ({x}, {y}) — keep it inside \
                     LEFT..RIGHT / TOP..BOTTOM",
                );
            }
        }
    }

    #[test]
    fn border_is_never_drawn_over() {
        let moves = [
            mv("Sleep Powder", "Grass"),
            mv("Swords Dance", "Normal"),
            mv("Double-Edge", "Normal"),
            mv("Thunder Wave", "Electric"),
        ];
        let party = [crate::display::PartySlotData {
            name: "Charizard".into(),
            active: true,
            level: 100,
            hp: 188,
            max_hp: 309,
            status: Some("PSN".into()),
            atk: 200,
            def: 200,
            spe: 200,
            spc: 200,
            types: alloc::vec::Vec::new(),
            moves: alloc::vec::Vec::new(),
            boost_atk: 0,
            boost_def: 0,
            boost_spe: 0,
            boost_spc: 0,
            item: None,
        }];
        let long = "Aerodactylus";
        let c = ctx(long, long);

        check("render_choice", |f| render_choice(f, &moves, &c));
        check("render_locked", |f| render_locked(f, Some("Sleep Powder"), &c));
        check("render_party", |f| render_party(f, &party, &c, false));
        check("render_stats", |f| render_stats(f, &party[0], 1));
        // render_playback is deliberately absent: the shared scene has no
        // frame at all, because one per half would cut the single field in
        // two. Its constraint is the mon band, not the border.
        check("render_lobby", |f| render_lobby(f, false, false, 1));
        check("render_result", |f| render_result(f, "WINNER!", &c));
        check("render_draw", |f| render_draw(f, &c));
        check("render_invalid", |f| render_invalid(f, InvalidReason::NoPp, 1));
        check("render_move_info", |f| {
            render_move_info(
                f,
                &moves[0],
                "Has a 30% chance to paralyze the target, and a long tail besides.",
                1,
            )
        });
        check("render_log", |f| {
            let lines = [
                "White's Aerodactylus used Sleep Powder, and it was quite effective",
                "  148 damage (2.0x, no crit) against a very long species name",
            ];
            render_log(f, &lines, 0, 1)
        });
        check("render_options", |f| {
            let rows = [
                OptionRow { label: "Team size", value: "3 v 3" },
                OptionRow { label: "Text speed", value: "Normal" },
                OptionRow { label: "Sound", value: "On" },
                OptionRow { label: "Tutorial", value: "First game" },
                OptionRow { label: "Turn timer", value: "60 s" },
            ];
            render_options(f, &rows, 2, HALF_W, HALF_H, 1, Battery::Level(72))
        });
        check("render_gen_picker", |f| render_gen_picker(f, 0, HALF_W, HALF_H, 1));
    }
}
