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

// ── Party slot snapshot ───────────────────────────────────────────────────────

/// Compact snapshot of a party Pokémon's in-battle stats, used by all display
/// targets for the long-press party-stats view.
///
/// Derived from [`battler::MonBattleData`] via [`party_slot_from_mon`]; cheap
/// enough to cache on embedded hardware for the duration of a battle prompt.
#[derive(Clone)]
pub struct PartySlotData {
    pub name: alloc::string::String,
    pub level: u8,
    pub hp: u16,
    pub max_hp: u16,
    pub status: Option<alloc::string::String>,
    pub atk: u16,
    pub def: u16,
    pub spe: u16,
    pub spc: u16,
    pub types: alloc::vec::Vec<battler::Type>,
}

/// Convert the display-relevant fields of a [`battler::MonBattleData`] into a
/// [`PartySlotData`].  Call this once at prompt time; store the result.
pub fn party_slot_from_mon(mon: &battler::MonBattleData) -> PartySlotData {
    use battler::Stat;
    let get = |s: Stat| mon.stats.get(&s).copied().unwrap_or(0u16);
    PartySlotData {
        name: mon.summary.name.clone(),
        level: mon.summary.level,
        hp: mon.hp,
        max_hp: mon.max_hp,
        status: mon.status.clone(),
        atk: get(Stat::Atk),
        def: get(Stat::Def),
        spe: get(Stat::Spe),
        spc: get(Stat::SpAtk),
        types: mon.types.clone(),
    }
}

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
/// Move 0              Move 1   ← y=1,  FONT_5X8
///      ┌─ Mon Name ─┐          ← y=24–39 (box)
/// ████████░░░░░░░░░            ← y=44, h=4 (HP bar)
/// Move 2              Move 3   ← y=55, FONT_5X8
/// ```
pub fn render_player_screen<D>(display: &mut D, mon_name: &str, moves: &[MoveSlot], hp_pct: u8)
where
    D: DrawTarget<Color = BinaryColor>,
{
    let move_char = MonoTextStyle::new(&FONT_5X8, BinaryColor::On);
    let name_char = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    let fill = PrimitiveStyle::with_fill(BinaryColor::On);

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

    // ── HP bar ────────────────────────────────────────────────────────────────
    let bar_w = hp_pct as u32 * 128 / 100;
    if bar_w > 0 {
        Rectangle::new(Point::new(0, 44), Size::new(bar_w, 4))
            .into_styled(fill)
            .draw(display).ok();
    }
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

// ── Party stats screen ────────────────────────────────────────────────────────

/// Draw the party-stats screen (long-press switch view) onto any 128×64 `DrawTarget`.
///
/// Layout:
/// ```text
/// Pikachu        Lv.25   ← FONT_6X10
/// ────────────────────
/// HP:42/75    [PAR]      ← FONT_5X8
/// Atk:55  Def:40
/// Spe:90  Spc:50
/// Electric / Flying
/// ```
pub fn render_pokemon_stats<D>(display: &mut D, slot: &PartySlotData)
where
    D: DrawTarget<Color = BinaryColor>,
{
    let name_char = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    let info_char = MonoTextStyle::new(&FONT_5X8, BinaryColor::On);

    display.clear(BinaryColor::Off).ok();

    // Name + level
    let lv_text = alloc::format!("Lv.{}", slot.level);
    let name = if slot.hp == 0 { "FAINTED" } else { slot.name.as_str() };
    Text::with_text_style(name, Point::new(0, 0), name_char, tl_style())
        .draw(display).ok();
    Text::with_text_style(&lv_text, Point::new(127, 0), name_char, tr_style())
        .draw(display).ok();

    Rectangle::new(Point::new(0, 11), Size::new(128, 1))
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
        .draw(display).ok();

    // HP + status
    let status_abbr = match slot.status.as_deref() {
        Some("par") => " PAR",
        Some("brn") => " BRN",
        Some("psn") | Some("tox") => " PSN",
        Some("slp") => " SLP",
        Some("frz") => " FRZ",
        _ => "",
    };
    let hp_line = alloc::format!("HP:{}/{}{}", slot.hp, slot.max_hp, status_abbr);
    Text::with_text_style(&hp_line, Point::new(0, 14), info_char, tl_style())
        .draw(display).ok();

    // Battle stats
    let atk_def = alloc::format!("Atk:{}  Def:{}", slot.atk, slot.def);
    Text::with_text_style(&atk_def, Point::new(0, 24), info_char, tl_style())
        .draw(display).ok();

    let spe_spc = alloc::format!("Spe:{}  Spc:{}", slot.spe, slot.spc);
    Text::with_text_style(&spe_spc, Point::new(0, 33), info_char, tl_style())
        .draw(display).ok();

    // Types
    let type_parts: alloc::vec::Vec<alloc::string::String> = slot.types.iter()
        .map(|t| alloc::format!("{t:?}"))
        .collect();
    Text::with_text_style(&type_parts.join(" / "), Point::new(0, 43), info_char, tl_style())
        .draw(display).ok();
}
