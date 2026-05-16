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
use core::cell::RefCell;
use core::sync::atomic::{AtomicBool, Ordering};
use embassy_sync::{blocking_mutex::{raw::CriticalSectionRawMutex, Mutex as BlockingMutex}, channel::Channel};
use embedded_graphics::{draw_target::DrawTarget, geometry::{OriginDimensions, Size}, pixelcolor::BinaryColor, Pixel};
use mega_blastoise_core::{render_event_text, render_lobby_screen, render_move_detail, render_player_screen, render_pokemon_stats, render_win_screen, BoardEvent, MoveSlot, PartySlotData};
use display_interface::AsyncWriteOnlyDataCommand;
use ssd1306::{mode::BufferedGraphicsModeAsync, prelude::*, I2CDisplayInterface, Ssd1306Async};

bind_interrupts!(struct OledIrqs {
    I2C0_IRQ => InterruptHandler<I2C0>;
    I2C1_IRQ => InterruptHandler<I2C1>;
});

// ── Shadow framebuffer ────────────────────────────────────────────────────────
//
// Packed 128×64 monochrome buffer: 16 bytes per row (128 bits), row-major, MSB = leftmost pixel.
// Updated after every flush so :oled can snapshot without touching the ssd1306 driver internals.

struct Shadow([[u8; 16]; 64]);

impl Shadow {
    const fn new() -> Self { Self([[0u8; 16]; 64]) }
}

impl DrawTarget for Shadow {
    type Color = BinaryColor;
    type Error = core::convert::Infallible;
    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where I: IntoIterator<Item = Pixel<BinaryColor>> {
        for Pixel(coord, color) in pixels {
            if coord.x >= 0 && coord.y >= 0 {
                let (x, y) = (coord.x as usize, coord.y as usize);
                if x < 128 && y < 64 {
                    let b = &mut self.0[y][x >> 3];
                    let bit = 0x80u8 >> (x & 7);
                    if color.is_on() { *b |= bit; } else { *b &= !bit; }
                }
            }
        }
        Ok(())
    }
}

impl OriginDimensions for Shadow {
    fn size(&self) -> Size { Size::new(128, 64) }
}

static P1_SHADOW: BlockingMutex<CriticalSectionRawMutex, RefCell<[[u8; 16]; 64]>> =
    BlockingMutex::new(RefCell::new([[0u8; 16]; 64]));
static P2_SHADOW: BlockingMutex<CriticalSectionRawMutex, RefCell<[[u8; 16]; 64]>> =
    BlockingMutex::new(RefCell::new([[0u8; 16]; 64]));

/// When true, every framebuffer change is rendered over RTT (defmt).
///
/// Boot default is set by the `oledlog` cargo feature (on with the feature,
/// off without). The `:oledlog on|off` USB command flips it at runtime
/// either way (works in the lobby and during a battle).
static OLED_DUMP: AtomicBool = AtomicBool::new(cfg!(feature = "oledlog"));

pub fn set_oled_dump(on: bool) {
    OLED_DUMP.store(on, Ordering::Relaxed);
}

pub fn oled_dump_enabled() -> bool {
    OLED_DUMP.load(Ordering::Relaxed)
}

fn store_shadow(player: u8, s: &Shadow) {
    if player == 1 { P1_SHADOW.lock(|fb| *fb.borrow_mut() = s.0); }
    else           { P2_SHADOW.lock(|fb| *fb.borrow_mut() = s.0); }
    if OLED_DUMP.load(Ordering::Relaxed) {
        dump_fb_rtt(player, &s.0);
    }
}

/// Emit the framebuffer as ONE compact defmt message: the 1024 packed bytes
/// as a hex string (`oledfb|pN|<2048 hex chars>`).
///
/// The earlier per-row half-block dump was 33 defmt messages (~4 KB) per
/// render — under battle render rates that floods RTT and either wedges the
/// executor (blocking defmt) or drops body rows (non-blocking). A single
/// atomic ~2 KB message survives bursts, so every frame of a whole game can
/// be captured. The host side reconstructs the screen offline (see
/// scripts/oled_render.py).
fn dump_fb_rtt(player: u8, fb: &[[u8; 16]; 64]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut hex = [0u8; 1024 * 2];
    let mut i = 0usize;
    for row in fb {
        for &b in row {
            hex[i] = HEX[(b >> 4) as usize];
            hex[i + 1] = HEX[(b & 0xf) as usize];
            i += 2;
        }
    }
    let s = core::str::from_utf8(&hex).unwrap_or("?");
    defmt::info!("oledfb|p{}|{=str}", player, s);
}

/// Snapshot the current OLED framebuffer for `player` (1 or 2).
/// Packed row-major: bit 7 of byte [y][0] is pixel (0, y).
pub fn read_shadow_fb(player: u8) -> [[u8; 16]; 64] {
    if player == 1 { P1_SHADOW.lock(|fb| *fb.borrow()) }
    else           { P2_SHADOW.lock(|fb| *fb.borrow()) }
}

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
    /// Transient battle-narration overlay for events without a dedicated state
    /// screen (move used, crit, miss, status, …). Held visible by the caller's
    /// animation delay; the next state redraw (HpUpdate / RestoreScreen /
    /// SwitchIn) clears it. `player` 0 = show on both displays.
    EventFlash { player: u8, text: [u8; 48], len: u8 },
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
    shadow: &mut Shadow,
    player: u8,
    ready: bool,
    ai: bool,
)
where
    DI: AsyncWriteOnlyDataCommand,
{
    render_lobby_screen(disp, ready, ai);
    render_lobby_screen(shadow, ready, ai);
    store_shadow(player, shadow);
    disp.flush().await.ok();
}

async fn redraw<DI>(
    disp: &mut Ssd1306Async<DI, DisplaySize128x64, BufferedGraphicsModeAsync<DisplaySize128x64>>,
    shadow: &mut Shadow,
    player: u8,
    st: &PlayerState,
)
where
    DI: AsyncWriteOnlyDataCommand,
{
    let mon = if st.fainted { "FAINTED" } else { st.name_str() };
    render_player_screen(disp, mon, &st.moves);
    render_player_screen(shadow, mon, &st.moves);
    store_shadow(player, shadow);
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
    cfg.frequency = 100_000;

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

    let p1_ok = disp0.init().await.is_ok();
    let p2_ok = disp1.init().await.is_ok();
    if !p1_ok { defmt::warn!("OLED P1 init failed"); }
    if !p2_ok { defmt::warn!("OLED P2 init failed"); }
    if !p1_ok && !p2_ok {
        defmt::warn!("OLED: no displays found — display task exiting");
        return;
    }

    READY.store(true, Ordering::Relaxed);

    let mut p1 = PlayerState::new();
    let mut p2 = PlayerState::new();
    let mut s1 = Shadow::new();
    let mut s2 = Shadow::new();

    if p1_ok { redraw(&mut disp0, &mut s1, 1, &p1).await; }
    if p2_ok { redraw(&mut disp1, &mut s2, 2, &p2).await; }

    loop {
        match CMD.receive().await {
            OledCmd::HpUpdate { player, pct } => {
                if player == 1 { p1.hp_pct = pct; if p1_ok { redraw(&mut disp0, &mut s1, 1, &p1).await; } }
                else           { p2.hp_pct = pct; if p2_ok { redraw(&mut disp1, &mut s2, 2, &p2).await; } }
            }
            OledCmd::ActiveMon { player, name, len } => {
                if player == 1 {
                    p1.name = name; p1.name_len = len; p1.fainted = false; p1.hp_pct = 100;
                    if p1_ok { redraw(&mut disp0, &mut s1, 1, &p1).await; }
                } else {
                    p2.name = name; p2.name_len = len; p2.fainted = false; p2.hp_pct = 100;
                    if p2_ok { redraw(&mut disp1, &mut s2, 2, &p2).await; }
                }
            }
            OledCmd::MovesUpdate { player, moves } => {
                if player == 1 { p1.moves = moves; if p1_ok { redraw(&mut disp0, &mut s1, 1, &p1).await; } }
                else           { p2.moves = moves; if p2_ok { redraw(&mut disp1, &mut s2, 2, &p2).await; } }
            }
            OledCmd::Faint { player } => {
                if player == 1 {
                    p1.fainted = true; p1.hp_pct = 0; p1.moves.clear();
                    if p1_ok { redraw(&mut disp0, &mut s1, 1, &p1).await; }
                } else {
                    p2.fainted = true; p2.hp_pct = 0; p2.moves.clear();
                    if p2_ok { redraw(&mut disp1, &mut s2, 2, &p2).await; }
                }
            }
            OledCmd::ShowMoveDetail { player, slot } => {
                if player == 1 && p1_ok {
                    if let Some(mv) = p1.moves.get(slot as usize) {
                        render_move_detail(&mut disp0, mv);
                        render_move_detail(&mut s1, mv);
                        store_shadow(1, &s1);
                        disp0.flush().await.ok();
                    }
                } else if player != 1 && p2_ok {
                    if let Some(mv) = p2.moves.get(slot as usize) {
                        render_move_detail(&mut disp1, mv);
                        render_move_detail(&mut s2, mv);
                        store_shadow(2, &s2);
                        disp1.flush().await.ok();
                    }
                }
            }
            OledCmd::ShowPokemonStats { player, team_idx } => {
                if player == 1 && p1_ok {
                    if let Some(slot) = p1.party.get(team_idx as usize) {
                        render_pokemon_stats(&mut disp0, slot);
                        render_pokemon_stats(&mut s1, slot);
                        store_shadow(1, &s1);
                        disp0.flush().await.ok();
                    }
                } else if player != 1 && p2_ok {
                    if let Some(slot) = p2.party.get(team_idx as usize) {
                        render_pokemon_stats(&mut disp1, slot);
                        render_pokemon_stats(&mut s2, slot);
                        store_shadow(2, &s2);
                        disp1.flush().await.ok();
                    }
                }
            }
            OledCmd::PartyUpdate { player, slots } => {
                if player == 1 { p1.party = slots; }
                else           { p2.party = slots; }
            }
            OledCmd::RestoreScreen { player } => {
                if player == 1 { if p1_ok { redraw(&mut disp0, &mut s1, 1, &p1).await; } }
                else           { if p2_ok { redraw(&mut disp1, &mut s2, 2, &p2).await; } }
            }
            OledCmd::LobbyState { player, ready, ai } => {
                if player == 1 { if p1_ok { draw_lobby_screen(&mut disp0, &mut s1, 1, ready, ai).await; } }
                else           { if p2_ok { draw_lobby_screen(&mut disp1, &mut s2, 2, ready, ai).await; } }
            }
            OledCmd::EventFlash { player, text, len } => {
                let s = core::str::from_utf8(&text[..len as usize]).unwrap_or("");
                if (player == 1 || player == 0) && p1_ok {
                    render_event_text(&mut disp0, s);
                    render_event_text(&mut s1, s);
                    store_shadow(1, &s1);
                    disp0.flush().await.ok();
                }
                if (player != 1 || player == 0) && p2_ok {
                    render_event_text(&mut disp1, s);
                    render_event_text(&mut s2, s);
                    store_shadow(2, &s2);
                    disp1.flush().await.ok();
                }
            }
            OledCmd::Win { winner } => {
                let (msg0, msg1) = BoardEvent::win_messages(winner);
                if p1_ok {
                    render_win_screen(&mut disp0, msg0);
                    render_win_screen(&mut s1, msg0);
                    store_shadow(1, &s1);
                    disp0.flush().await.ok();
                }
                if p2_ok {
                    render_win_screen(&mut disp1, msg1);
                    render_win_screen(&mut s2, msg1);
                    store_shadow(2, &s2);
                    disp1.flush().await.ok();
                }
            }
        }
    }
}
