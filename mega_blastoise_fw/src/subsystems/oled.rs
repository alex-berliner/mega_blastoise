//! SSD1306 OLED driver, one display per player.
//!
//! P1: I2C0  (GP16 SDA, GP17 SCL)
//! P2: I2C1  (GP18 SDA, GP19 SCL)
//!
//! Uses embassy async I²C — the 20 ms framebuffer flush is spread across
//! ~64 small executor yields instead of blocking the task solid.
//!
//! Call [`send`] from `BattleEffects::on_event` to queue a display update.

extern crate alloc;

use alloc::vec::Vec;

use embassy_rp::bind_interrupts;
use embassy_rp::i2c::{Config as I2cConfig, I2c, InterruptHandler};
use embassy_rp::Peri;
use embassy_rp::peripherals::{I2C0, I2C1, PIN_16, PIN_17, PIN_18, PIN_19};
use core::sync::atomic::{AtomicBool, Ordering};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use mega_blastoise_core::{party_slot_from_mon, render_move_detail, render_player_screen, render_pokemon_stats, MoveSlot, PartySlotData};
use display_interface::AsyncWriteOnlyDataCommand;
use ssd1306::{mode::BufferedGraphicsModeAsync, prelude::*, I2CDisplayInterface, Ssd1306Async};

bind_interrupts!(struct OledIrqs {
    I2C0_IRQ => InterruptHandler<I2C0>;
    I2C1_IRQ => InterruptHandler<I2C1>;
});

// ── Command channel ───────────────────────────────────────────────────────────

pub enum OledCmd {
    /// HP changed; `pct` is 0–100.
    HpUpdate { player: u8, pct: u8 },
    /// New active Pokémon (UTF-8 name, up to 12 bytes).
    ActiveMon { player: u8, name: [u8; 12], len: u8 },
    /// Move list updated (PP changes after each turn).
    MovesUpdate { player: u8, moves: Vec<MoveSlot> },
    /// A mon fainted.
    Faint { player: u8 },
    /// Battle ended — winner is 1 (p1) or 2 (p2); 0 means tie.
    Win { winner: u8 },
    /// Long-press detail view for a move slot (0-based).
    ShowMoveDetail { player: u8, slot: u8 },
    /// Long-press stats view for a party slot (0-based team index).
    ShowPokemonStats { player: u8, team_idx: u8 },
    /// Update the cached party snapshot used by ShowPokemonStats.
    PartyUpdate { player: u8, slots: Vec<PartySlotData> },
    /// Restore normal battle screen after detail view.
    RestoreScreen { player: u8 },
    /// Lobby ready state for a player. ready=false → idle instructions;
    /// ready=true,ai=false → "READY!"; ready=true,ai=true → "AI" (AI-controlled side).
    LobbyState { player: u8, ready: bool, ai: bool },
}

static CMD: Channel<CriticalSectionRawMutex, OledCmd, 8> = Channel::new();
static READY: AtomicBool = AtomicBool::new(false);

pub fn send(cmd: OledCmd) {
    if !READY.load(Ordering::Relaxed) {
        return;
    }
    if CMD.try_send(cmd).is_err() {
        defmt::warn!("oled: channel full, cmd dropped");
    }
}

// ── Per-player state ──────────────────────────────────────────────────────────

struct PlayerState {
    hp_pct: u8,
    name: [u8; 12],
    name_len: u8,
    fainted: bool,
    moves: Vec<MoveSlot>,
    party: Vec<PartySlotData>,
}

impl PlayerState {
    fn new() -> Self {
        let mut name = [b' '; 12];
        name[0] = b'-'; name[1] = b'-'; name[2] = b'-';
        Self { hp_pct: 100, name, name_len: 3, fainted: false, moves: Vec::new(), party: Vec::new() }
    }

    fn name_str(&self) -> &str {
        core::str::from_utf8(&self.name[..self.name_len as usize]).unwrap_or("?")
    }
}

// ── Rendering ─────────────────────────────────────────────────────────────────

async fn draw_lobby_screen<DI>(
    disp: &mut Ssd1306Async<DI, DisplaySize128x64, BufferedGraphicsModeAsync<DisplaySize128x64>>,
    ready: bool,
    ai: bool,
)
where
    DI: AsyncWriteOnlyDataCommand,
{
    let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    disp.clear(BinaryColor::Off).ok();
    if !ready {
        // Centered on 128px wide display: "PRESS READY" and "HOLD FOR AI" (11 chars × 6px = 66px)
        Text::with_baseline("PRESS TO READY", Point::new(22, 20), style, Baseline::Top).draw(disp).ok();
        Text::with_baseline("HOLD: FIGHT AI", Point::new(22, 36), style, Baseline::Top).draw(disp).ok();
    } else if ai {
        // "AI" centered (2 chars × 6px = 12px, offset = 64 - 6 = 58)
        Text::with_baseline("AI", Point::new(58, 27), style, Baseline::Top).draw(disp).ok();
    } else {
        // "READY!" centered (6 chars × 6px = 36px, offset = 64 - 18 = 46)
        Text::with_baseline("READY!", Point::new(46, 27), style, Baseline::Top).draw(disp).ok();
    }
    disp.flush().await.ok();
}

async fn redraw<DI>(
    disp: &mut Ssd1306Async<DI, DisplaySize128x64, BufferedGraphicsModeAsync<DisplaySize128x64>>,
    st: &PlayerState,
)
where
    DI: AsyncWriteOnlyDataCommand,
{
    let mon = if st.fainted { "FAINTED" } else { st.name_str() };
    render_player_screen(disp, mon, &st.moves);
    disp.flush().await.ok();
}

// ── Embassy task ──────────────────────────────────────────────────────────────

#[embassy_executor::task]
pub async fn task(
    i2c0: Peri<'static, I2C0>,
    scl0: Peri<'static, PIN_17>,
    sda0: Peri<'static, PIN_16>,
    i2c1: Peri<'static, I2C1>,
    scl1: Peri<'static, PIN_19>,
    sda1: Peri<'static, PIN_18>,
) {
    let mut cfg = I2cConfig::default();
    cfg.frequency = 400_000;

    let bus0 = I2c::new_async(i2c0, scl0, sda0, OledIrqs, cfg);
    let bus1 = I2c::new_async(i2c1, scl1, sda1, OledIrqs, cfg);

    let mut disp0 = Ssd1306Async::new(
        I2CDisplayInterface::new(bus0),
        DisplaySize128x64,
        DisplayRotation::Rotate0,
    )
    .into_buffered_graphics_mode();

    let mut disp1 = Ssd1306Async::new(
        I2CDisplayInterface::new(bus1),
        DisplaySize128x64,
        DisplayRotation::Rotate0,
    )
    .into_buffered_graphics_mode();

    if disp0.init().await.is_err() || disp1.init().await.is_err() {
        defmt::warn!("OLED init failed — display task exiting");
        return;
    }

    READY.store(true, Ordering::Relaxed);

    let mut p1 = PlayerState::new();
    let mut p2 = PlayerState::new();

    redraw(&mut disp0, &p1).await;
    redraw(&mut disp1, &p2).await;

    loop {
        match CMD.receive().await {
            OledCmd::HpUpdate { player, pct } => {
                if player == 1 { p1.hp_pct = pct; redraw(&mut disp0, &p1).await; }
                else           { p2.hp_pct = pct; redraw(&mut disp1, &p2).await; }
            }
            OledCmd::ActiveMon { player, name, len } => {
                if player == 1 {
                    p1.name = name; p1.name_len = len; p1.fainted = false; p1.hp_pct = 100;
                    redraw(&mut disp0, &p1).await;
                } else {
                    p2.name = name; p2.name_len = len; p2.fainted = false; p2.hp_pct = 100;
                    redraw(&mut disp1, &p2).await;
                }
            }
            OledCmd::MovesUpdate { player, moves } => {
                if player == 1 { p1.moves = moves; redraw(&mut disp0, &p1).await; }
                else           { p2.moves = moves; redraw(&mut disp1, &p2).await; }
            }
            OledCmd::Faint { player } => {
                if player == 1 {
                    p1.fainted = true; p1.hp_pct = 0; p1.moves.clear();
                    redraw(&mut disp0, &p1).await;
                } else {
                    p2.fainted = true; p2.hp_pct = 0; p2.moves.clear();
                    redraw(&mut disp1, &p2).await;
                }
            }
            OledCmd::ShowMoveDetail { player, slot } => {
                if player == 1 {
                    if let Some(mv) = p1.moves.get(slot as usize) {
                        render_move_detail(&mut disp0, mv);
                        disp0.flush().await.ok();
                    }
                } else if let Some(mv) = p2.moves.get(slot as usize) {
                    render_move_detail(&mut disp1, mv);
                    disp1.flush().await.ok();
                }
            }
            OledCmd::ShowPokemonStats { player, team_idx } => {
                if player == 1 {
                    if let Some(slot) = p1.party.get(team_idx as usize) {
                        render_pokemon_stats(&mut disp0, slot);
                        disp0.flush().await.ok();
                    }
                } else if let Some(slot) = p2.party.get(team_idx as usize) {
                    render_pokemon_stats(&mut disp1, slot);
                    disp1.flush().await.ok();
                }
            }
            OledCmd::PartyUpdate { player, slots } => {
                if player == 1 { p1.party = slots; }
                else           { p2.party = slots; }
            }
            OledCmd::RestoreScreen { player } => {
                if player == 1 { redraw(&mut disp0, &p1).await; }
                else           { redraw(&mut disp1, &p2).await; }
            }
            OledCmd::LobbyState { player, ready, ai } => {
                if player == 1 { draw_lobby_screen(&mut disp0, ready, ai).await; }
                else           { draw_lobby_screen(&mut disp1, ready, ai).await; }
            }
            OledCmd::Win { winner } => {
                let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
                let (msg0, msg1) = match winner {
                    1 => ("WINNER!", "GG!"),
                    2 => ("GG!", "WINNER!"),
                    _ => ("TIE!", "TIE!"),
                };
                disp0.clear(BinaryColor::Off).ok();
                Text::with_baseline(msg0, Point::zero(), style, Baseline::Top)
                    .draw(&mut disp0).ok();
                disp0.flush().await.ok();
                disp1.clear(BinaryColor::Off).ok();
                Text::with_baseline(msg1, Point::zero(), style, Baseline::Top)
                    .draw(&mut disp1).ok();
                disp1.flush().await.ok();
            }
        }
    }
}
