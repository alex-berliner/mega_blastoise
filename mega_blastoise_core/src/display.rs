extern crate alloc;

use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Size},
    image::{Image, ImageRaw},
    mono_font::{
        ascii::{FONT_5X8, FONT_6X10},
        MonoTextStyle,
    },
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::{Alignment, Baseline, Text, TextStyle, TextStyleBuilder},
    Pixel,
};

use crate::board_event::MoveSlot;

// ── Party slot snapshot ───────────────────────────────────────────────────────

/// Compact snapshot of a party Pokémon's in-battle stats, used by all display
/// targets for the long-press party-stats view.
///
/// Derived from [`gen1_battle::MonBattleData`] via [`party_slot_from_mon`]; cheap
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
    pub types: alloc::vec::Vec<gen1_battle::Type>,
    /// Move name + (pp, max_pp) for each slot, in order.
    pub moves: alloc::vec::Vec<(alloc::string::String, u8, u8)>,
    /// Stat stage boosts (-6 to +6).
    pub boost_atk: i8,
    pub boost_def: i8,
    pub boost_spe: i8,
    pub boost_spc: i8,
    /// Held item name, if any.
    pub item: Option<alloc::string::String>,
}

/// Convert the display-relevant fields of a [`gen1_battle::MonBattleData`] into a
/// [`PartySlotData`].  Call this once at prompt time; store the result.
pub fn party_slot_from_mon(mon: &gen1_battle::MonBattleData) -> PartySlotData {
    use gen1_battle::Stat;
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
        moves: mon.moves.iter().map(|m| (m.name.clone(), m.pp, m.max_pp)).collect(),
        boost_atk: mon.boosts.atk,
        boost_def: mon.boosts.def,
        boost_spe: mon.boosts.spe,
        boost_spc: mon.boosts.spa,
        item: mon.item.clone(),
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
/// Move 0              Move 1   ← y=0,  FONT_5X8
///        [48x48 sprite]        ← y=8–55, centered
/// Move 2              Move 3   ← y=56, FONT_5X8
/// ```
/// The mon's sprite fills the band between the move rows; when the name has
/// no sprite ("FAINTED", "---") it falls back to the name in a centered box.
pub fn render_player_screen<D>(display: &mut D, mon_name: &str, moves: &[MoveSlot])
where
    D: DrawTarget<Color = BinaryColor>,
{
    let move_char = MonoTextStyle::new(&FONT_5X8, BinaryColor::On);
    let move_h = FONT_5X8.character_size.height as i32;

    display.clear(BinaryColor::Off).ok();

    // ── Corner moves ──────────────────────────────────────────────────────────
    if let Some(mv) = moves.first() {
        Text::with_text_style(&mv.name, Point::new(0, 0), move_char, tl_style())
            .draw(display).ok();
    }
    if let Some(mv) = moves.get(1) {
        Text::with_text_style(&mv.name, Point::new(127, 0), move_char, tr_style())
            .draw(display).ok();
    }
    if let Some(mv) = moves.get(2) {
        Text::with_text_style(&mv.name, Point::new(0, 64 - move_h), move_char, tl_style())
            .draw(display).ok();
    }
    if let Some(mv) = moves.get(3) {
        Text::with_text_style(&mv.name, Point::new(127, 64 - move_h), move_char, tr_style())
            .draw(display).ok();
    }

    // ── Mon sprite, centered between the move rows ────────────────────────────
    if let Some(spr) = crate::sprites::mon_sprite(mon_name) {
        let side = crate::sprites::SPRITE_SIDE;
        let raw = ImageRaw::<BinaryColor>::new(spr.as_slice(), side);
        Image::new(&raw, Point::new((128 - side as i32) / 2, move_h))
            .draw(display).ok();
        return;
    }

    // ── Fallback: name in a box, centered ─────────────────────────────────────
    let name_char = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
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

// ── Shared header for pokémon stat/move pages ─────────────────────────────────

fn type_abbr(t: gen1_battle::Type) -> &'static str {
    match t {
        gen1_battle::Type::Normal   => "NRM",
        gen1_battle::Type::Fighting => "FGT",
        gen1_battle::Type::Flying   => "FLY",
        gen1_battle::Type::Poison   => "PSN",
        gen1_battle::Type::Ground   => "GND",
        gen1_battle::Type::Rock     => "RCK",
        gen1_battle::Type::Bug      => "BUG",
        gen1_battle::Type::Ghost    => "GHO",
        gen1_battle::Type::Steel    => "STL",
        gen1_battle::Type::Fire     => "FIR",
        gen1_battle::Type::Water    => "WAT",
        gen1_battle::Type::Grass    => "GRS",
        gen1_battle::Type::Electric => "ELC",
        gen1_battle::Type::Psychic  => "PSY",
        gen1_battle::Type::Ice      => "ICE",
        gen1_battle::Type::Dragon   => "DRG",
        gen1_battle::Type::Dark     => "DRK",
        gen1_battle::Type::Fairy    => "FAI",
        _                       => "???",
    }
}

fn draw_mon_header<D>(display: &mut D, slot: &PartySlotData)
where
    D: DrawTarget<Color = BinaryColor>,
{
    let hdr = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    let name = slot.name.as_str();
    let type_str = match slot.types.len() {
        0 => alloc::format!(""),
        1 => alloc::format!("{}", type_abbr(slot.types[0])),
        _ => alloc::format!("{}/{}", type_abbr(slot.types[0]), type_abbr(slot.types[1])),
    };
    Text::with_text_style(name,      Point::new(0,   0), hdr, tl_style()).draw(display).ok();
    Text::with_text_style(&type_str, Point::new(127, 0), hdr, tr_style()).draw(display).ok();
    Rectangle::new(Point::new(0, 12), Size::new(128, 1))
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
        .draw(display).ok();
}

// ── Party stats screen — page 1: current stats with boosts ───────────────────

/// Draw the stats page (long-press page 0): name/types header, HP+status, stats+boosts.
///
/// Layout:
/// ```text
/// Pikachu       Electric  ← name left, type right (FONT_6X10)
/// ──────────────────────
/// HP: 42/75         PAR  ← HP left, status right (FONT_5X8)
/// Atk: 55  +2
/// Def: 40
/// Spe: 90  -1
/// Spc: 50
/// ```
pub fn render_pokemon_stats<D>(display: &mut D, slot: &PartySlotData)
where
    D: DrawTarget<Color = BinaryColor>,
{
    let info = MonoTextStyle::new(&FONT_5X8, BinaryColor::On);
    display.clear(BinaryColor::Off).ok();
    draw_mon_header(display, slot);

    let status_abbr = if slot.hp == 0 {
        " FNT"
    } else {
        match slot.status.as_deref() {
            Some("par") => " PAR",
            Some("brn") => " BRN",
            Some("psn") | Some("tox") => " PSN",
            Some("slp") => " SLP",
            Some("frz") => " FRZ",
            _           => "",
        }
    };
    let hp_line  = alloc::format!("HP: {}/{}{}", slot.hp, slot.max_hp, status_abbr);
    let lv_line  = alloc::format!("Lv.{}", slot.level);
    Text::with_text_style(&hp_line, Point::new(0,   15), info, tl_style()).draw(display).ok();
    Text::with_text_style(&lv_line, Point::new(127, 15), info, tr_style()).draw(display).ok();

    let stats: &[(&str, u16, i8)] = &[
        ("Atk", slot.atk, slot.boost_atk),
        ("Def", slot.def, slot.boost_def),
        ("Spc", slot.spc, slot.boost_spc),
        ("Spe", slot.spe, slot.boost_spe),
    ];
    for (i, (label, val, boost)) in stats.iter().enumerate() {
        let y = 25 + i as i32 * 10;
        let b = if *boost >= 0 { alloc::format!("+{}", boost) } else { alloc::format!("{}", boost) };
        let line = alloc::format!("{}: {} ({})", label, val, b);
        Text::with_text_style(&line, Point::new(0, y), info, tl_style()).draw(display).ok();
    }
}

// ── Party stats screen — page 2: moves + held item ───────────────────────────

/// Draw the moves page (long-press page 1): name/types header, held item, moves with PP.
///
/// Layout:
/// ```text
/// Pikachu       Electric
/// ──────────────────────
/// Item: —
/// Surf           10/16
/// Thunderbolt     5/8
/// Ice Beam       15/16
/// Double-Edge     8/16
/// ```
pub fn render_pokemon_stats_page2<D>(display: &mut D, slot: &PartySlotData)
where
    D: DrawTarget<Color = BinaryColor>,
{
    let info = MonoTextStyle::new(&FONT_5X8, BinaryColor::On);
    display.clear(BinaryColor::Off).ok();
    draw_mon_header(display, slot);

    let item_str = match slot.item.as_deref() {
        Some(s) if !s.is_empty() => alloc::format!("Item: {}", s),
        _                        => alloc::format!("Item: -"),
    };
    Text::with_text_style(&item_str, Point::new(0, 15), info, tl_style()).draw(display).ok();

    for (i, (mv_name, pp, max_pp)) in slot.moves.iter().enumerate().take(4) {
        let y = 25 + i as i32 * 10;
        let name_t = if mv_name.len() > 13 { &mv_name[..13] } else { mv_name.as_str() };
        let pp_str = alloc::format!("{}/{}", pp, max_pp);
        Text::with_text_style(name_t, Point::new(0,   y), info, tl_style()).draw(display).ok();
        Text::with_text_style(&pp_str, Point::new(127, y), info, tr_style()).draw(display).ok();
    }
}

// ── Switch prompt screen ──────────────────────────────────────────────────────

/// Draw the forced-switch prompt screen onto any 128×64 `DrawTarget`.
///
/// Layout:
/// ```text
/// -- SWITCH --            ← FONT_6X10, centered
/// ──────────────────────
/// 1 Pikachu         75%  ← FONT_5X8, slot# + name left, HP/status right
/// 2 Charizard       FNT
/// 3 Blastoise       PAR
/// ```
pub fn render_switch_screen<D>(display: &mut D, party: &[PartySlotData])
where
    D: DrawTarget<Color = BinaryColor>,
{
    let header_char = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    let slot_char   = MonoTextStyle::new(&FONT_5X8,  BinaryColor::On);

    display.clear(BinaryColor::Off).ok();

    Text::with_text_style("-- SWITCH --", Point::new(64, 0), header_char, center_style())
        .draw(display).ok();

    Rectangle::new(Point::new(0, 12), Size::new(128, 1))
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
        .draw(display).ok();

    for (i, slot) in party.iter().enumerate().take(6) {
        let y = 15 + i as i32 * 9;
        let name = if slot.name.len() > 10 { &slot.name[..10] } else { slot.name.as_str() };
        let left = alloc::format!("{} {}", i + 1, name);
        let hp_str = alloc::format!("{}/{}", slot.hp, slot.max_hp);
        let right = if slot.hp == 0 {
            alloc::format!("FNT")
        } else {
            match slot.status.as_deref() {
                Some("par") => alloc::format!("PAR {}", hp_str),
                Some("brn") => alloc::format!("BRN {}", hp_str),
                Some("psn") | Some("tox") => alloc::format!("PSN {}", hp_str),
                Some("slp") => alloc::format!("SLP {}", hp_str),
                Some("frz") => alloc::format!("FRZ {}", hp_str),
                _ => hp_str,
            }
        };
        Text::with_text_style(&left,  Point::new(0,   y), slot_char, tl_style()).draw(display).ok();
        Text::with_text_style(&right, Point::new(127, y), slot_char, tr_style()).draw(display).ok();
    }
}

// ── Win screen ────────────────────────────────────────────────────────────────

/// Draw a win/loss/tie message centered on any 128×64 `DrawTarget`.
///
/// Typical messages: `"WINNER!"`, `"GG!"`, `"TIE!"`.
pub fn render_win_screen<D>(display: &mut D, msg: &str)
where
    D: DrawTarget<Color = BinaryColor>,
{
    let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    display.clear(BinaryColor::Off).ok();
    Text::with_text_style(msg, Point::new(64, 27), style, center_style()).draw(display).ok();
}

// ── In-game overlay screens ───────────────────────────────────────────────────

/// Draw an "invalid selection" flash onto any 128×64 `DrawTarget`.
///
/// Shown briefly when the player tries to switch to a fainted Pokémon.
pub fn render_invalid_selection<D>(display: &mut D)
where
    D: DrawTarget<Color = BinaryColor>,
{
    let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    display.clear(BinaryColor::Off).ok();
    Text::with_text_style("Already fainted!", Point::new(64, 27), style, center_style()).draw(display).ok();
}

/// Draw the "submitted, waiting" overlay onto any 128×64 `DrawTarget`.
///
/// Shown after a player commits their choice while waiting for the turn to resolve.
/// `cancel_hint` is shown at the bottom line; pass `""` to omit it.
pub fn render_waiting_screen<D>(display: &mut D, mon_name: &str, cancel_hint: &str)
where
    D: DrawTarget<Color = BinaryColor>,
{
    let lg = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    let sm = MonoTextStyle::new(&FONT_5X8, BinaryColor::On);
    display.clear(BinaryColor::Off).ok();
    Text::with_text_style(mon_name,      Point::new(64, 12), lg, center_style()).draw(display).ok();
    Text::with_text_style("Waiting...",  Point::new(64, 28), sm, center_style()).draw(display).ok();
    if !cancel_hint.is_empty() {
        Text::with_text_style(cancel_hint, Point::new(64, 42), sm, center_style()).draw(display).ok();
    }
}

/// Draw the "waiting for other player" overlay onto any 128×64 `DrawTarget`.
///
/// Shown on one player's screen while the other player is still choosing.
pub fn render_waiting_for_opponent<D>(display: &mut D)
where
    D: DrawTarget<Color = BinaryColor>,
{
    let sm = MonoTextStyle::new(&FONT_5X8, BinaryColor::On);
    display.clear(BinaryColor::Off).ok();
    Text::with_text_style("Waiting for",  Point::new(64, 20), sm, center_style()).draw(display).ok();
    Text::with_text_style("opponent...", Point::new(64, 32), sm, center_style()).draw(display).ok();
}

// ── Battle event flash screen ─────────────────────────────────────────────────

/// Draw a battle event narration centered on any 128×64 `DrawTarget`.
///
/// Word-wraps `text` to at most 3 lines of FONT_5X8, centered vertically.
/// Used for move/faint/status event overlays shown briefly during a turn.
pub fn render_event_text<D>(display: &mut D, text: &str)
where
    D: DrawTarget<Color = BinaryColor>,
{
    let style = MonoTextStyle::new(&FONT_5X8, BinaryColor::On);
    display.clear(BinaryColor::Off).ok();

    // Target chars per line: evenly divide for long text, 21-char cap for short.
    let target = if text.len() > 25 { (text.len() + 2) / 3 } else { 21 };

    let mut lines = [""; 3];
    let mut n = 0usize;
    let mut rest = text;
    while !rest.is_empty() && n < 3 {
        if rest.len() <= target || n == 2 {
            lines[n] = rest;
            n += 1;
            break;
        }
        let search_end = (target + 4).min(rest.len());
        let at = rest[..search_end].rfind(' ').unwrap_or(target.min(rest.len()));
        lines[n] = rest[..at].trim();
        n += 1;
        rest = rest[at..].trim_start();
    }

    let start_y: i32 = match n {
        1 => 28,
        2 => 23,
        _ => 17,
    };
    for i in 0..n {
        if !lines[i].is_empty() {
            Text::with_text_style(lines[i], Point::new(64, start_y + i as i32 * 10), style, center_style())
                .draw(display).ok();
        }
    }
}

// ── Lobby screen ──────────────────────────────────────────────────────────────

/// Draw the lobby ready state onto any 128×64 `DrawTarget`.
///
/// - `ready=false` → idle: "PRESS TO READY" / "HOLD: FIGHT AI"
/// - `ready=true, ai=false` → "READY!"
/// - `ready=true, ai=true`  → "AI" (this side is AI-controlled)
pub fn render_lobby_screen<D>(display: &mut D, ready: bool, ai: bool)
where
    D: DrawTarget<Color = BinaryColor>,
{
    let style_lg = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    let style_sm = MonoTextStyle::new(&FONT_5X8, BinaryColor::On);
    display.clear(BinaryColor::Off).ok();
    if !ready {
        Text::with_text_style("PRESS TO READY", Point::new(64, 16), style_lg, center_style()).draw(display).ok();
        Text::with_text_style("HOLD: FIGHT AI", Point::new(64, 36), style_sm, center_style()).draw(display).ok();
    } else if ai {
        Text::with_text_style("AI",     Point::new(64, 27), style_lg, center_style()).draw(display).ok();
    } else {
        Text::with_text_style("READY!", Point::new(64, 27), style_lg, center_style()).draw(display).ok();
    }
}

// ── Generic 128×64 framebuffer ────────────────────────────────────────────────

/// Target-independent 128×64 monochrome pixel buffer.
///
/// Implements `DrawTarget<Color = BinaryColor>` so all `render_*` functions can
/// write into it directly.  Wrap it in a target-specific newtype that adds
/// output methods (`to_rgba()` for web, `render()` for CLI, …).
pub struct OledFrameBuffer {
    pub fb: [[bool; 128]; 64],
}

impl OledFrameBuffer {
    pub const fn new() -> Self {
        Self { fb: [[false; 128]; 64] }
    }
}

impl DrawTarget for OledFrameBuffer {
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
                if x < 128 && y < 64 {
                    self.fb[y][x] = color.is_on();
                }
            }
        }
        Ok(())
    }
}

impl OriginDimensions for OledFrameBuffer {
    fn size(&self) -> Size {
        Size::new(128, 64)
    }
}
