extern crate alloc;

use embedded_graphics::{
    mono_font::{
        ascii::{FONT_5X8, FONT_6X10},
        MonoTextStyle,
    },
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::{Alignment, Baseline, Text, TextStyle, TextStyleBuilder},
};

use crate::board_event::MoveSlot;

// ── Shared text styles ────────────────────────────────────────────────────────

fn tl_style() -> TextStyle {
    TextStyleBuilder::new().alignment(Alignment::Left).baseline(Baseline::Top).build()
}

fn tr_style() -> TextStyle {
    TextStyleBuilder::new().alignment(Alignment::Right).baseline(Baseline::Top).build()
}

fn center_style() -> TextStyle {
    TextStyleBuilder::new().alignment(Alignment::Center).baseline(Baseline::Top).build()
}

// ── Normal screen ─────────────────────────────────────────────────────────────

/// Draw the normal battle screen onto any 128×64 `DrawTarget`.
///
/// Layout:
/// ```text
/// Move 0              Move 1   ← y=1, FONT_5X8
///
///      ┌─ Mon Name ─┐
///      └────────────┘          ← centered vertically
///
/// Move 2              Move 3   ← y=55, FONT_5X8
/// ```
pub fn render_player_screen<D>(display: &mut D, mon_name: &str, moves: &[MoveSlot])
where
    D: DrawTarget<Color = BinaryColor>,
{
    let move_char = MonoTextStyle::new(&FONT_5X8, BinaryColor::On);
    let name_char = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);

    display.clear(BinaryColor::Off).ok();

    // ── Corner moves ──────────────────────────────────────────────────────────
    if let Some(mv) = moves.first() {
        Text::with_text_style(&mv.name, Point::new(0, 1), move_char, tl_style())
            .draw(display).ok();
    }
    if let Some(mv) = moves.get(1) {
        Text::with_text_style(&mv.name, Point::new(127, 1), move_char, tr_style())
            .draw(display).ok();
    }
    if let Some(mv) = moves.get(2) {
        Text::with_text_style(&mv.name, Point::new(0, 55), move_char, tl_style())
            .draw(display).ok();
    }
    if let Some(mv) = moves.get(3) {
        Text::with_text_style(&mv.name, Point::new(127, 55), move_char, tr_style())
            .draw(display).ok();
    }

    // ── Mon name in a box, centered ───────────────────────────────────────────
    let name_y = 27i32;
    let char_w = FONT_6X10.character_size.width;
    let char_h = FONT_6X10.character_size.height;
    let pad = 3u32;
    let text_w = mon_name.len() as u32 * char_w;
    let box_w = (text_w + pad * 2).max(char_w);
    let box_x = ((128u32.saturating_sub(box_w)) / 2) as i32;
    let box_y = name_y - pad as i32;

    Rectangle::new(Point::new(box_x, box_y), Size::new(box_w, char_h + pad * 2))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
        .draw(display).ok();

    Text::with_text_style(mon_name, Point::new(64, name_y), name_char, center_style())
        .draw(display).ok();
}

// ── Move detail screen ────────────────────────────────────────────────────────

/// Draw the move detail screen onto any 128×64 `DrawTarget`.
///
/// Layout (long-press view):
/// ```text
/// Thunder Wave            ← FONT_6X10
/// ────────────────────
/// Type: Electric
/// Cat:  Status
/// Pow:  ---
/// Acc:  100
/// PP:   19/20             ← FONT_5X8, one line each
/// ```
pub fn render_move_detail<D>(display: &mut D, mv: &MoveSlot)
where
    D: DrawTarget<Color = BinaryColor>,
{
    let name_char = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    let info_char = MonoTextStyle::new(&FONT_5X8, BinaryColor::On);

    display.clear(BinaryColor::Off).ok();

    // Move name
    Text::with_text_style(&mv.name, Point::new(0, 0), name_char, tl_style())
        .draw(display).ok();

    // Separator line
    Rectangle::new(Point::new(0, 11), Size::new(128, 1))
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
        .draw(display).ok();

    // Info lines
    let type_line = alloc::format!("Type: {}", mv.type_name);
    Text::with_text_style(&type_line, Point::new(0, 14), info_char, tl_style())
        .draw(display).ok();

    let cat_line = alloc::format!("Cat:  {}", mv.category);
    Text::with_text_style(&cat_line, Point::new(0, 22), info_char, tl_style())
        .draw(display).ok();

    let pow_line = match mv.power {
        Some(p) => alloc::format!("Pow:  {}", p),
        None => alloc::format!("Pow:  ---"),
    };
    Text::with_text_style(&pow_line, Point::new(0, 30), info_char, tl_style())
        .draw(display).ok();

    let acc_line = match mv.accuracy {
        Some(a) => alloc::format!("Acc:  {}", a),
        None => alloc::format!("Acc:  ---"),
    };
    Text::with_text_style(&acc_line, Point::new(0, 38), info_char, tl_style())
        .draw(display).ok();

    let pp_line = alloc::format!("PP:   {}/{}", mv.pp, mv.max_pp);
    Text::with_text_style(&pp_line, Point::new(0, 46), info_char, tl_style())
        .draw(display).ok();
}
