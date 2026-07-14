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
    /// This mon is currently active on the field.
    pub active: bool,
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
        active: mon.active,
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

/// Longest prefix of `s` at most `max_bytes` long, ending on a char boundary.
/// Byte-index truncation on names panics on multi-byte characters (a
/// typographic apostrophe in a roster name hard-faulted the firmware from
/// `render_switch_screen`); every display truncation goes through here.
fn prefix_bytes(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
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

// ── Speed comparison badge ────────────────────────────────────────────────────

/// How this player's active mon's Speed compares to the opponent's.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SpeedCmp {
    Faster,
    Even,
    Slower,
}

/// Small boxed indicator on the right side of the active mon: a lightning
/// bolt when faster, an equals sign when tied, an X when slower.
fn draw_speed_badge<D>(display: &mut D, cmp: SpeedCmp)
where
    D: DrawTarget<Color = BinaryColor>,
{
    let stroke = PrimitiveStyle::with_stroke(BinaryColor::On, 1);
    let (bx, by, bw, bh) = (113i32, 24i32, 14u32, 16u32);
    Rectangle::new(Point::new(bx, by), Size::new(bw, bh))
        .into_styled(stroke)
        .draw(display)
        .ok();
    let seg = |d: &mut D, x0: i32, y0: i32, x1: i32, y1: i32| {
        embedded_graphics::primitives::Line::new(
            Point::new(bx + x0, by + y0),
            Point::new(bx + x1, by + y1),
        )
        .into_styled(stroke)
        .draw(d)
        .ok();
    };
    match cmp {
        SpeedCmp::Faster => {
            // Lightning bolt.
            seg(display, 8, 3, 5, 8);
            seg(display, 5, 8, 8, 8);
            seg(display, 8, 8, 5, 13);
        }
        SpeedCmp::Even => {
            seg(display, 4, 6, 9, 6);
            seg(display, 4, 10, 9, 10);
        }
        SpeedCmp::Slower => {
            seg(display, 4, 4, 9, 12);
            seg(display, 9, 4, 4, 12);
        }
    }
}

// ── Shared sprite drawing ─────────────────────────────────────────────────────

/// The mon's 48×48 sprite (or a fallback name box for "FAINTED"/"---"),
/// centered horizontally with its top edge at `top + bob_off`. The bob
/// offset applies to the SPRITE only — the fallback text box stays put.
fn draw_center_sprite<D>(display: &mut D, mon_name: &str, top: i32, bob_off: i32)
where
    D: DrawTarget<Color = BinaryColor>,
{
    if let Some(spr) = crate::sprites::mon_sprite(mon_name) {
        let side = crate::sprites::SPRITE_SIDE;
        let raw = ImageRaw::<BinaryColor>::new(spr.as_slice(), side);
        Image::new(&raw, Point::new((128 - side as i32) / 2, top + bob_off))
            .draw(display).ok();
    } else {
        let name_char = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
        let name_y = top + 19;
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
pub fn render_player_screen<D>(
    display: &mut D,
    mon_name: &str,
    moves: &[MoveSlot],
    sprite_y_off: i32,
    spd: SpeedCmp,
) where
    D: DrawTarget<Color = BinaryColor>,
{
    let move_char = MonoTextStyle::new(&FONT_5X8, BinaryColor::On);
    let move_h = FONT_5X8.character_size.height as i32;

    display.clear(BinaryColor::Off).ok();

    // ── Mon sprite (or fallback name box), centered between the move rows ────
    // Drawn FIRST: when the bob offset shifts the sprite into a move row, its
    // black background must not overwrite the text — moves go on top.
    draw_center_sprite(display, mon_name, move_h, sprite_y_off);
    draw_speed_badge(display, spd);

    // ── Corner moves, on top of the sprite ────────────────────────────────────
    #[cfg(not(feature = "wrapmoves"))]
    {
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
    }
    // wrapmoves variant: move names wrap at spaces/hyphens (and long words
    // hyphenate) into stacked lines — top corners grow downward, bottom
    // corners grow upward. Pairs well with bigsprite's narrow corners.
    #[cfg(feature = "wrapmoves")]
    for (mv, x, top, right) in [
        (moves.first(), 0i32, true, false),
        (moves.get(1), 127, true, true),
        (moves.get(2), 0, false, false),
        (moves.get(3), 127, false, true),
    ] {
        let Some(mv) = mv else { continue };
        let lines = wrap_move_name(&mv.name);
        let n = lines.len() as i32;
        for (j, line) in lines.iter().enumerate() {
            let y = if top {
                j as i32 * move_h
            } else {
                64 - n * move_h + j as i32 * move_h
            };
            let style = if right { tr_style() } else { tl_style() };
            Text::with_text_style(line, Point::new(x, y), move_char, style)
                .draw(display).ok();
        }
    }
}

/// Split a move name for the wrapped corner layout: break at spaces, keep
/// hyphens with the leading part ("Double-" / "Edge"), and hyphenate words
/// longer than 6 characters into 5-char chunks ("Earth-" / "quake").
#[cfg(feature = "wrapmoves")]
fn wrap_move_name(name: &str) -> alloc::vec::Vec<alloc::string::String> {
    use alloc::string::String;
    let mut out = alloc::vec::Vec::new();
    // `natural_hyphen`: the word was followed by a real hyphen in the name
    // ("Double-Edge") — it renders verbatim on the word's last line and does
    // not count toward the hyphenation length.
    let mut push_word = |word: &str, natural_hyphen: bool| {
        let mut rest = word;
        while rest.chars().count() > 6 {
            let split = rest
                .char_indices()
                .nth(5)
                .map(|(i, _)| i)
                .unwrap_or(rest.len());
            let mut chunk = String::from(&rest[..split]);
            chunk.push('-');
            out.push(chunk);
            rest = &rest[split..];
        }
        if !rest.is_empty() || natural_hyphen {
            let mut last = String::from(rest);
            if natural_hyphen {
                last.push('-');
            }
            out.push(last);
        }
    };
    let mut cur = String::new();
    for ch in name.chars() {
        match ch {
            ' ' => {
                if !cur.is_empty() {
                    push_word(&core::mem::take(&mut cur), false);
                }
            }
            '-' => {
                push_word(&core::mem::take(&mut cur), true);
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        push_word(&cur, false);
    }
    out
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
        let name_t = prefix_bytes(mv_name, 13);
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
        let name = prefix_bytes(&slot.name, 10);
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

// ── Controls picker (battle start) ────────────────────────────────────────────

/// Draw the Normal/Concealed controls picker.
///
/// Layout:
/// ```text
///        CONTROLS              ← FONT_6X10, centered
/// ┌────────┐
/// │ NORMAL │  CONCEALED        ← boxed = highlighted (filled when confirmed)
/// └────────┘
/// Buttons pick moves and       ← blurb for the highlighted scheme
/// party slots directly.
/// ```
pub fn render_controls_select<D>(display: &mut D, highlighted: u8, confirmed: bool)
where
    D: DrawTarget<Color = BinaryColor>,
{
    let lg = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    let sm = MonoTextStyle::new(&FONT_5X8, BinaryColor::On);
    display.clear(BinaryColor::Off).ok();

    Text::with_text_style("CONTROLS", Point::new(64, 0), lg, center_style()).draw(display).ok();

    // Two options centered in each half; box around the highlighted one
    // (filled with inverted text once confirmed).
    let opts = ["NORMAL", "CONCEALED"];
    let centers = [32i32, 96];
    let y = 18i32;
    let char_w = FONT_5X8.character_size.width as i32;
    let char_h = FONT_5X8.character_size.height as u32;
    for (k, (label, cx)) in opts.iter().zip(centers).enumerate() {
        let is_sel = k as u8 == highlighted;
        if is_sel {
            let w = label.len() as i32 * char_w + 8;
            let style = if confirmed {
                PrimitiveStyle::with_fill(BinaryColor::On)
            } else {
                PrimitiveStyle::with_stroke(BinaryColor::On, 1)
            };
            Rectangle::new(Point::new(cx - w / 2, y - 4), Size::new(w as u32, char_h + 8))
                .into_styled(style)
                .draw(display)
                .ok();
        }
        let text_color = if is_sel && confirmed { BinaryColor::Off } else { BinaryColor::On };
        let ts = MonoTextStyle::new(&FONT_5X8, text_color);
        Text::with_text_style(label, Point::new(cx, y), ts, center_style()).draw(display).ok();
    }

    // Blurb for the highlighted scheme.
    let blurb: [&str; 3] = if highlighted == 1 {
        ["Hides your inputs:", "hold an action, tap a", "corner. New layout/turn."]
    } else {
        ["Buttons pick moves and", "party slots directly.", ""]
    };
    for (j, line) in blurb.iter().enumerate() {
        if !line.is_empty() {
            Text::with_text_style(line, Point::new(64, 36 + j as i32 * 9), sm, center_style())
                .draw(display)
                .ok();
        }
    }
}

// ── Concealed mode screens ────────────────────────────────────────────────────

/// Draw the concealed action-select screen: the mon's bobbing sprite (same
/// as the normal battle screen) with Attack and Switch on randomized
/// bottom-row positions — no boxes, no instruction text.
///
/// Layout:
/// ```text
///        [48x48 sprite]        ← y=8–55, centered, bobs
/// ATTACK          SWITCH       ← y=56, one label per bottom button position
/// ```
pub fn render_action_select<D>(
    display: &mut D,
    mon_name: &str,
    sprite_y_off: i32,
    attack_pos: u8,
    switch_pos: u8,
    spd: SpeedCmp,
) where
    D: DrawTarget<Color = BinaryColor>,
{
    let sm = MonoTextStyle::new(&FONT_5X8, BinaryColor::On);
    let h = FONT_5X8.character_size.height as i32;
    display.clear(BinaryColor::Off).ok();

    // Sprite first — the labels draw on top when the bob overlaps.
    draw_center_sprite(display, mon_name, h, sprite_y_off);
    draw_speed_badge(display, spd);

    // Bottom row: left / center / right, matching the three bottom buttons.
    for (pos, x, style) in [(0u8, 0i32, tl_style()), (1, 64, center_style()), (2, 127, tr_style())]
    {
        let label = if pos == attack_pos {
            "ATTACK"
        } else if pos == switch_pos {
            "SWITCH"
        } else {
            continue;
        };
        Text::with_text_style(label, Point::new(x, 64 - h), sm, style).draw(display).ok();
    }
}

/// Shared corner-menu chrome: a centered title with up to four labels at the
/// physical corner-button positions (None = dead corner, drawn empty).
fn render_corner_menu<D>(display: &mut D, title: &str, corners: &[Option<&str>; 4])
where
    D: DrawTarget<Color = BinaryColor>,
{
    let lg = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    let sm = MonoTextStyle::new(&FONT_5X8, BinaryColor::On);
    let h = FONT_5X8.character_size.height as i32;
    display.clear(BinaryColor::Off).ok();

    Text::with_text_style(title, Point::new(64, 27), lg, center_style()).draw(display).ok();

    let spots = [
        (0i32, 0i32, false),      // TL
        (127, 0, true),           // TR
        (0, 64 - h, false),       // BL
        (127, 64 - h, true),      // BR
    ];
    for (k, (x, y, right)) in spots.iter().enumerate() {
        if let Some(name) = corners[k] {
            let style = if *right { tr_style() } else { tl_style() };
            Text::with_text_style(prefix_bytes(name, 12), Point::new(*x, *y), sm, style)
                .draw(display)
                .ok();
        }
    }
}

/// Concealed move menu: shuffled move names at the corner-button positions.
pub fn render_concealed_moves<D>(display: &mut D, corners: &[Option<&MoveSlot>; 4])
where
    D: DrawTarget<Color = BinaryColor>,
{
    let labels = corners.map(|c| c.map(|m| m.name.as_str()));
    render_corner_menu(display, "- ATTACK -", &labels);
}

/// Concealed bench menu: shuffled benched mon names at the corner positions.
pub fn render_concealed_switch<D>(display: &mut D, corners: &[Option<&PartySlotData>; 4])
where
    D: DrawTarget<Color = BinaryColor>,
{
    let labels = corners.map(|c| c.map(|s| s.name.as_str()));
    render_corner_menu(display, "- SWITCH -", &labels);
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
/// Shown after a player commits their choice: the mon's bobbing sprite in
/// the same position as the choice screens, with `cancel_hint`
/// ("tap to unready") on the bottom line — pass `""` to omit it.
pub fn render_waiting_screen<D>(
    display: &mut D,
    mon_name: &str,
    sprite_y_off: i32,
    cancel_hint: &str,
    spd: SpeedCmp,
) where
    D: DrawTarget<Color = BinaryColor>,
{
    let sm = MonoTextStyle::new(&FONT_5X8, BinaryColor::On);
    let h = FONT_5X8.character_size.height as i32;
    display.clear(BinaryColor::Off).ok();
    draw_center_sprite(display, mon_name, h, sprite_y_off);
    draw_speed_badge(display, spd);
    if !cancel_hint.is_empty() {
        Text::with_text_style(cancel_hint, Point::new(64, 64 - h), sm, center_style())
            .draw(display).ok();
    }
}

/// Draw the switch-in "sent out" screen: caption up top, the incoming mon's
/// sprite below it.
pub fn render_sent_out<D>(display: &mut D, mon_name: &str, caption: &str)
where
    D: DrawTarget<Color = BinaryColor>,
{
    let sm = MonoTextStyle::new(&FONT_5X8, BinaryColor::On);
    display.clear(BinaryColor::Off).ok();
    Text::with_text_style(prefix_bytes(caption, 25), Point::new(64, 0), sm, center_style())
        .draw(display).ok();
    draw_center_sprite(display, mon_name, 12, 0);
}

/// Split text into two lines at the space nearest its midpoint (hard split
/// on a char boundary when there is no space). Short text stays one line.
fn split_two_lines(text: &str, max_line: usize) -> (&str, &str) {
    let text = text.trim();
    if text.len() <= max_line {
        return (text, "");
    }
    let mid = text.len() / 2;
    let split = text
        .char_indices()
        .filter(|(_, c)| *c == ' ')
        .map(|(i, _)| i)
        .min_by_key(|i| i.abs_diff(mid));
    match split {
        Some(i) => (&text[..i], text[i..].trim_start()),
        None => {
            let l1 = prefix_bytes(text, mid.min(max_line));
            (l1, &text[l1.len()..])
        }
    }
}

/// True when a move does something TO the opponent — self-buffs, heals,
/// screens, and other self/field-only moves hide the recipient sprite on
/// the move-used screen.
fn move_affects_opponent(move_id: &str) -> bool {
    use gen1_battle::MoveEffectKind as K;
    match gen1_battle::move_by_id(move_id) {
        Some(m) => !matches!(
            m.effect_kind,
            K::BoostSelf
                | K::HealHalf
                | K::Rest
                | K::Substitute
                | K::LightScreen
                | K::Reflect
                | K::Mist
                | K::FocusEnergy
                | K::Conversion
                | K::Metronome
                | K::MirrorMove
                | K::NoOp
        ),
        None => true,
    }
}

/// Draw the "used <move>!" screen: caption on two lines up top, then the
/// attacker's sprite, the move's icon (flickered by the caller via
/// `icon_on`), and — when both mons have sprites AND the move actually
/// affects the opponent — the recipient on the icon's right.
///
/// Layout (full case, 48+32+48 = 128 exactly):
/// ```text
/// Red's Tauros used            ← FONT_5X8, centered, y=0
/// Body Slam!                   ← y=8
/// [atk 48x48][icon 32][rcp 48] ← x=0 / 48 / 80, sprites y=16, icon y=24
/// ```
pub fn render_move_used<D>(
    display: &mut D,
    mon_name: &str,
    caption: &str,
    move_id: &str,
    recipient: &str,
    icon_on: bool,
) where
    D: DrawTarget<Color = BinaryColor>,
{
    let sm = MonoTextStyle::new(&FONT_5X8, BinaryColor::On);
    display.clear(BinaryColor::Off).ok();

    let (l1, l2) = split_two_lines(caption, 25);
    Text::with_text_style(prefix_bytes(l1, 25), Point::new(64, 0), sm, center_style())
        .draw(display).ok();
    if !l2.is_empty() {
        Text::with_text_style(prefix_bytes(l2, 25), Point::new(64, 8), sm, center_style())
            .draw(display).ok();
    }

    let icon = crate::move_sprites::move_sprite(move_id);
    let atk_spr = crate::sprites::mon_sprite(mon_name);
    // Self-buffs (Agility, Swords Dance), heals, screens etc. have no
    // recipient to show.
    let rcp_spr = if move_affects_opponent(move_id) {
        crate::sprites::mon_sprite(recipient)
    } else {
        None
    };
    let mon_side = crate::sprites::SPRITE_SIDE;

    // Column layout: attacker | icon | recipient when everything fits;
    // otherwise fall back to attacker + icon, or a centered attacker.
    let (atk_x, icon_x, rcp_x) = match (atk_spr.is_some(), icon.is_some(), rcp_spr.is_some()) {
        (true, true, true) => (0i32, 48i32, Some(80i32)),
        (true, true, false) => (24, 84, None),
        _ => ((128 - mon_side as i32) / 2, 84, None),
    };

    if let Some(spr) = atk_spr {
        let raw = ImageRaw::<BinaryColor>::new(spr.as_slice(), mon_side);
        Image::new(&raw, Point::new(atk_x, 16)).draw(display).ok();
    } else {
        draw_center_sprite(display, mon_name, 16, 0);
    }
    if let (Some(bits), true) = (icon, icon_on) {
        let side = crate::move_sprites::MOVE_SPRITE_SIDE;
        let raw = ImageRaw::<BinaryColor>::new(bits, side);
        Image::new(&raw, Point::new(icon_x, 24)).draw(display).ok();
    }
    if let (Some(spr), Some(x)) = (rcp_spr, rcp_x) {
        let raw = ImageRaw::<BinaryColor>::new(spr.as_slice(), mon_side);
        Image::new(&raw, Point::new(x, 16)).draw(display).ok();
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
        let search_end = prefix_bytes(rest, target + 4).len();
        let at = rest[..search_end]
            .rfind(' ')
            .unwrap_or_else(|| prefix_bytes(rest, target).len());
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

#[cfg(all(test, feature = "wrapmoves"))]
mod wrap_tests {
    use super::wrap_move_name;
    use alloc::string::String;
    use alloc::vec::Vec;

    fn w(name: &str) -> Vec<String> {
        wrap_move_name(name)
    }

    #[test]
    fn wraps_spaces_hyphens_and_long_words() {
        assert_eq!(w("Skull Bash"), ["Skull", "Bash"]);
        assert_eq!(w("Double-Edge"), ["Double-", "Edge"]);
        assert_eq!(w("Earthquake"), ["Earth-", "quake"]);
        assert_eq!(w("Flamethrower"), ["Flame-", "throw-", "er"]);
        assert_eq!(w("Confuse Ray"), ["Confu-", "se", "Ray"]);
        assert_eq!(w("Gust"), ["Gust"]);
        assert_eq!(w("Splash"), ["Splash"]);
        assert_eq!(w("Self-Destruct"), ["Self-", "Destr-", "uct"]);
    }
}

#[cfg(test)]
mod utf8_render_tests {
    use super::*;
    use alloc::string::String;
    use alloc::vec::Vec;

    /// Pixel sink: accepts any draw, panics never.
    struct Sink;
    impl DrawTarget for Sink {
        type Color = BinaryColor;
        type Error = core::convert::Infallible;
        fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
        where
            I: IntoIterator<Item = Pixel<BinaryColor>>,
        {
            for _ in pixels {}
            Ok(())
        }
    }
    impl OriginDimensions for Sink {
        fn size(&self) -> Size {
            Size::new(128, 64)
        }
    }

    fn slot(name: &str) -> PartySlotData {
        PartySlotData {
            name: String::from(name),
            active: false,
            level: 78,
            hp: 100,
            max_hp: 200,
            status: None,
            atk: 1, def: 1, spe: 1, spc: 1,
            types: Vec::new(),
            moves: alloc::vec![(String::from("Farfetch\u{2019}d Special Move"), 5, 10)],
            boost_atk: 0, boost_def: 0, boost_spe: 0, boost_spc: 0,
            item: None,
        }
    }

    /// The overnight hard fault: a typographic apostrophe (multi-byte) at a
    /// truncation boundary. Every screen that truncates text must survive
    /// non-ASCII names.
    #[test]
    fn non_ascii_names_render_without_panic() {
        let mut d = Sink;
        let party = alloc::vec![slot("Farfetch\u{2019}d"), slot("Nidoran\u{2640}")];
        render_switch_screen(&mut d, &party);
        render_pokemon_stats(&mut d, &party[0]);
        render_pokemon_stats_page2(&mut d, &party[0]);
        render_event_text(&mut d, "Red\u{2019}s Farfetch\u{2019}d used Swords Dance!");
        render_event_text(&mut d, "\u{2019}\u{2019}\u{2019}\u{2019}\u{2019}\u{2019}\u{2019}\u{2019}\u{2019}\u{2019}\u{2019}\u{2019}\u{2019}\u{2019}\u{2019}\u{2019}");
    }

    #[test]
    fn prefix_bytes_floors_to_char_boundary() {
        let s = "Farfetch\u{2019}d"; // bytes 8..11 are the apostrophe
        assert_eq!(prefix_bytes(s, 9), "Farfetch");
        assert_eq!(prefix_bytes(s, 10), "Farfetch");
        assert_eq!(prefix_bytes(s, 11), "Farfetch\u{2019}");
        assert_eq!(prefix_bytes(s, 100), s);
    }
}
