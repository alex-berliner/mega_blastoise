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

    // Own mon, back view, bobbing.
    let bob = if ctx.bob { -2 } else { 0 };
    if !back_sprite_in(d, ctx.own_name, 38, 88 + bob, 2) {
        text_center(d, clip(ctx.own_name, 9), 38, 84, &FONT_6X10, C_INK);
    }
    hp_bar(d, 8, 126, 60, ctx.own_hp);

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
    back_sprite_in(d, ctx.own_name, 60, 92 + bob, 2);
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
    legend(d, &["B CHANGE CHOICE"]);
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

/// Turn playback: narration on top, both mons on the field, each seat seeing
/// its own mon from behind and the foe from the front.
pub fn render_playback<D>(d: &mut D, caption: &str, ctx: &HalfCtx<'_>)
where
    D: DrawTarget<Color = Rgb565>,
{
    d.clear(C_BG).ok();

    // Narration, up to two lines of 36 chars.
    panel(d, 3, 3, 234, 36, C_BOX, C_INK);
    let line1_len = 36.min(caption.len());
    let (l1, l2) = if caption.len() <= 36 {
        (caption, "")
    } else {
        let cut = caption[..line1_len].rfind(' ').unwrap_or(line1_len);
        (&caption[..cut], caption[cut..].trim_start())
    };
    text_at(d, clip(l1, 36), 9, 9, &FONT_6X10, C_INK);
    if !l2.is_empty() {
        text_at(d, clip(l2, 36), 9, 21, &FONT_6X10, C_INK);
    }

    // Foe: front art, upper right; their HP box upper left.
    front_sprite_in(d, ctx.foe_name, 186, 74, 1);
    panel(d, 6, 46, 118, 26, C_BOX, C_INK);
    text_at(d, clip(ctx.foe_name, 12), 11, 49, &FONT_5X8, C_INK);
    hp_bar(d, 11, 60, 108, ctx.foe_hp);

    // Own: back art at 2x, lower left; own HP box lower right.
    let bob = if ctx.bob { -2 } else { 0 };
    back_sprite_in(d, ctx.own_name, 56, 118 + bob, 2);
    panel(d, 116, 100, 118, 26, C_BOX, C_INK);
    text_at(d, clip(ctx.own_name, 12), 121, 103, &FONT_5X8, C_INK);
    hp_bar(d, 121, 114, 108, ctx.own_hp);

    legend(d, &["A NEXT", "? BATTLE LOG"]);
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
        text_center(d, "B to un-ready", 120, 96, &FONT_5X8, C_DIM);
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
    front_sprite_in(d, ctx.own_name, 120, 92, 1);
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

/// Turn playback laid out for the full panel in landscape (320x240).
///
/// Playback is the one moment both players watch the same thing, so it gets
/// the whole screen instead of two 240x160 halves. Sprites are drawn at 2x
/// and the narration runs across the top at full width.
pub fn render_playback_wide<D>(d: &mut D, caption: &str, ctx: &HalfCtx<'_>, w: u32, h: u32)
where
    D: DrawTarget<Color = Rgb565>,
{
    d.clear(C_BG).ok();
    let wi = w as i32;
    let hi = h as i32;

    // Narration across the top, up to two lines of 48 characters.
    panel(d, 4, 4, w - 8, 42, C_BOX, C_INK);
    let (l1, l2) = if caption.len() <= 48 {
        (caption, "")
    } else {
        let cut = caption[..48].rfind(' ').unwrap_or(48);
        (&caption[..cut], caption[cut..].trim_start())
    };
    text_at(d, clip(l1, 48), 12, 11, &FONT_8X13, C_INK);
    if !l2.is_empty() {
        text_at(d, clip(l2, 56), 12, 28, &FONT_6X10, C_INK);
    }

    // Foe upper right, own mon lower left — the Game Boy arrangement, with
    // each side's status box on the opposite side so nothing overlaps.
    let foe_cx = wi - 78;
    let foe_cy = 96;
    if mon_sprite_color(ctx.foe_name).is_some() {
        front_sprite_in(d, ctx.foe_name, foe_cx, foe_cy, 2);
    }
    panel(d, 10, 56, 150, 34, C_BOX, C_INK);
    text_at(d, clip(ctx.foe_name, 15), 16, 60, &FONT_6X10, C_INK);
    hp_bar(d, 16, 74, 138, ctx.foe_hp);

    let bob = if ctx.bob { -3 } else { 0 };
    back_sprite_in(d, ctx.own_name, 82, hi - 62 + bob, 3);
    panel(d, wi - 162, hi - 78, 152, 34, C_BOX, C_INK);
    text_at(d, clip(ctx.own_name, 15), wi - 156, hi - 74, &FONT_6X10, C_INK);
    hp_bar(d, wi - 156, hi - 60, 140, ctx.own_hp);

    text_at(d, "A NEXT", 10, hi - 16, &FONT_5X8, C_DIM);
    text_at(d, "? BATTLE LOG", 62, hi - 16, &FONT_5X8, C_DIM);
}

/// Battle-begin versus screen: both mons on the field facing each other.
///
/// Deliberately not one seat's view — this is the shared moment before the
/// first turn, so neither mon is drawn from behind. Gen 1 front sprites all
/// face the same direction, so the right-hand one is mirrored to square them
/// up against each other.
pub fn render_battle_begin<D>(
    d: &mut D,
    left_name: &str,
    right_name: &str,
    left_level: u8,
    right_level: u8,
    caption: &str,
    w: u32,
    h: u32,
) where
    D: DrawTarget<Color = Rgb565>,
{
    d.clear(C_BG).ok();
    let wi = w as i32;
    let hi = h as i32;
    let cx = wi / 2;

    // Ground line, so the two mons read as standing on the same field.
    fill(d, 12, hi / 2 + 46, w - 24, 2, C_DIM);

    let sprite_y = hi / 2 + 6;
    if let Some(s) = mon_sprite_color(left_name) {
        draw_sprite(
            d,
            s,
            wi / 4 - (s.w as u32 * 2 / 2) as i32,
            sprite_y - (s.h as u32 * 2) as i32 + 40,
            2,
        );
    }
    if let Some(s) = mon_sprite_color(right_name) {
        draw_sprite_mirrored(
            d,
            s,
            wi * 3 / 4 - (s.w as u32 * 2 / 2) as i32,
            sprite_y - (s.h as u32 * 2) as i32 + 40,
            2,
        );
    }

    // Name plates under each side.
    let plate_w = (w / 2) - 30;
    panel(d, 16, hi - 62, plate_w, 30, C_BOX, C_INK);
    text_at(d, clip(left_name, 14), 22, hi - 57, &FONT_8X13, C_INK);
    if left_level > 0 {
        let mut b = LvBuf::new();
        text_right(d, b.fmt(left_level), 16 + plate_w as i32 - 6, hi - 45, &FONT_5X8, C_DIM);
    }

    panel(d, cx + 14, hi - 62, plate_w, 30, C_BOX, C_INK);
    text_at(d, clip(right_name, 14), cx + 20, hi - 57, &FONT_8X13, C_INK);
    if right_level > 0 {
        let mut b = LvBuf::new();
        text_right(d, b.fmt(right_level), cx + 14 + plate_w as i32 - 6, hi - 45, &FONT_5X8, C_DIM);
    }

    // VS between them, and the engine's line across the top.
    text_center(d, "VS", cx, hi - 56, &FONT_8X13, C_ACCENT);
    panel(d, 4, 4, w - 8, 26, C_BOX, C_INK);
    text_center(d, clip(caption, 48), cx, 11, &FONT_6X10, C_INK);
}
