//! WS2812B NeoPixel driver — two independent 12-LED strips, one per player.
//!
//! Each player has its own data GPIO and PIO state machine (P1 = GP20/SM0,
//! P2 = GP22/SM1, both on PIO0 sharing one loaded program). Per-strip layout:
//!
//!   LEDs 0–7   HP bar of the active mon (green >50%, yellow >25%, red ≤25%)
//!   LEDs 8–10  the 3 party-member indicators (see below)
//!   LED  11    unused — kept dark, reserved for debug
//!
//! Each party indicator encodes one team member:
//!   * green if healthy, or the status-effect color if afflicted
//!   * brightness scales with that member's remaining HP
//!   * off once the member has fainted (or before it has been seen)
//!
//! Call [`send`] from `BattleEffects::on_event` to queue an update. The task
//! re-renders both strips after each command. HP / status / cure commands
//! target each player's *active* mon, tracked from the last `SwitchIn`.

use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::{DMA_CH0, DMA_CH1, PIO0};

/// Per-board data-pin assignment: the partner PCB routes LED_P1/LED_P2 to
/// GP0/GP1 (schematic nets); the hand-wired board uses GP20/GP22.
#[cfg(feature = "breadboard")]
mod wiring {
    pub use embassy_rp::peripherals::{PIN_20 as P1Pin, PIN_22 as P2Pin};
}
#[cfg(not(feature = "breadboard"))]
mod wiring {
    pub use embassy_rp::peripherals::{PIN_0 as P1Pin, PIN_1 as P2Pin};
}
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::pio_programs::ws2812::{PioWs2812, PioWs2812Program};
use embassy_rp::Peri;
use embassy_futures::select::{select, Either};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::Timer;
use mega_blastoise_core::{hp_bar_color, hp_bar_count};
use smart_leds::RGB8;

// ── Interrupt binding ─────────────────────────────────────────────────────────

bind_interrupts!(struct LedIrqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

// ── Layout constants ──────────────────────────────────────────────────────────

/// Physical LEDs per strip.
const NUM_LEDS: usize = 12;
/// LEDs we actually drive; LED index 11 is the dark debug reserve.
const USED_LEDS: usize = 11;
/// First party-indicator LED; slots occupy `PARTY..PARTY+3` (8,9,10).
const PARTY: usize = 8;
const TEAM_SIZE: usize = 3;

const OFF: RGB8 = RGB8 { r: 0, g: 0, b: 0 };
/// Party dot color for a healthy (un-statused) member, before HP dimming.
const OK_GREEN: RGB8 = RGB8 { r: 0, g: 90, b: 0 };

/// Master brightness, applied to every frame just before it is written. Keeps
/// current draw (and thus voltage droop / color shift on long or weakly-powered
/// strips) low, and keeps the whole thing easy on the eyes. Percent of full.
const BRIGHTNESS_PCT: u16 = 35;

/// Scale a color by the master brightness.
fn cap(c: RGB8) -> RGB8 {
    let s = |v: u8| ((v as u16 * BRIGHTNESS_PCT) / 100) as u8;
    RGB8 { r: s(c.r), g: s(c.g), b: s(c.b) }
}

// ── Status type ───────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub enum LedStatus {
    Paralyzed,
    Burned,
    Frozen,
    Poisoned,
    Asleep,
}

impl LedStatus {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "par" => Some(Self::Paralyzed),
            "brn" => Some(Self::Burned),
            "frz" => Some(Self::Frozen),
            "psn" | "tox" => Some(Self::Poisoned),
            "slp" => Some(Self::Asleep),
            _ => None,
        }
    }

    /// Party-dot base color (moderate brightness; HP dimming is applied on top).
    fn color(self) -> RGB8 {
        match self {
            Self::Paralyzed => RGB8 { r: 70, g: 55, b:  0 }, // yellow
            Self::Burned    => RGB8 { r: 75, g: 18, b:  0 }, // orange
            Self::Frozen    => RGB8 { r:  0, g: 50, b: 70 }, // cyan
            Self::Poisoned  => RGB8 { r: 45, g:  0, b: 55 }, // purple
            Self::Asleep    => RGB8 { r: 28, g: 28, b: 28 }, // dim white
        }
    }
}

// ── Command channel ───────────────────────────────────────────────────────────

pub enum LedCmd {
    /// Reset a player's strip for a new battle: mark the first `size` team
    /// members alive + full HP (green), clear the rest. Sent at battle start.
    TeamInit { player: u8, size: u8 },
    HpUpdate { player: u8, pct: u8 },
    /// Mon switched in; `slot` is the 0-based team index (also becomes active).
    SwitchIn { player: u8, slot: u8 },
    /// Mon fainted; `slot` is the 0-based team index.
    Faint { player: u8, slot: u8 },
    SetStatus { player: u8, status: LedStatus },
    CureStatus { player: u8 },
    Win { winner: u8 },
    // ── Lobby ──────────────────────────────────────────────────────────────────
    LobbyIdle,
    LobbyWaiting { p1_ready: bool, p2_ready: bool },
    LobbyCountdown,
}

static CMD: Channel<CriticalSectionRawMutex, LedCmd, 8> = Channel::new();

pub fn send(cmd: LedCmd) {
    if CMD.try_send(cmd).is_err() {
        defmt::warn!("led: channel full, cmd dropped");
    }
}

// ── Per-player state ──────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
struct MemberState {
    /// Seen on the field at least once; before that the dot stays dark.
    seen: bool,
    /// 0 = fainted (dot off).
    hp_pct: u8,
    status: Option<LedStatus>,
}

impl MemberState {
    const fn new() -> Self {
        Self { seen: false, hp_pct: 100, status: None }
    }
}

struct PlayerLedState {
    members: [MemberState; TEAM_SIZE],
    /// Team slot of the active mon; drives the 0–7 HP bar.
    active: usize,
}

impl PlayerLedState {
    const fn new() -> Self {
        Self { members: [MemberState::new(); TEAM_SIZE], active: 0 }
    }

    fn render(&self) -> [RGB8; NUM_LEDS] {
        let mut buf = [OFF; NUM_LEDS];

        // LEDs 0–7: HP bar of the active mon.
        let active = &self.members[self.active];
        let lit = hp_bar_count(active.hp_pct);
        let color = hp_color_rgb(active.hp_pct);
        for px in buf.iter_mut().take(lit) {
            *px = color;
        }

        // LEDs 8–10: per-member party / status / HP indicators.
        for (i, m) in self.members.iter().enumerate() {
            buf[PARTY + i] = party_color(m);
        }

        // LED 11 stays OFF (debug reserve).

        // Master brightness on the driven LEDs (11 stays dark either way).
        for px in buf.iter_mut().take(USED_LEDS) {
            *px = cap(*px);
        }
        buf
    }
}

/// Party-dot color for one member: off if unseen/fainted, else green (or the
/// status color) dimmed by remaining HP.
fn party_color(m: &MemberState) -> RGB8 {
    if !m.seen || m.hp_pct == 0 {
        return OFF;
    }
    let base = m.status.map(|s| s.color()).unwrap_or(OK_GREEN);
    dim(base, m.hp_pct)
}

/// Scale a color by HP percent, floored so a near-dead member is still visible.
fn dim(c: RGB8, hp_pct: u8) -> RGB8 {
    // factor ranges 35..=100 across 0..=100% HP.
    let f = 35 + (65u16 * hp_pct.min(100) as u16) / 100;
    let scale = |v: u8| ((v as u16 * f) / 100) as u8;
    RGB8 { r: scale(c.r), g: scale(c.g), b: scale(c.b) }
}

fn hp_color_rgb(pct: u8) -> RGB8 {
    let (r, g, b) = hp_bar_color(pct);
    RGB8 { r, g, b }
}

/// Fill LEDs 0..USED_LEDS with `color`, leaving the debug reserve dark.
fn solid(color: RGB8) -> [RGB8; NUM_LEDS] {
    let color = cap(color);
    let mut frame = [OFF; NUM_LEDS];
    for px in frame.iter_mut().take(USED_LEDS) {
        *px = color;
    }
    frame
}

// ── Embassy task ──────────────────────────────────────────────────────────────

#[embassy_executor::task]
pub async fn task(
    pio0: Peri<'static, PIO0>,
    p1_pin: Peri<'static, wiring::P1Pin>,
    p2_pin: Peri<'static, wiring::P2Pin>,
    p1_dma: Peri<'static, DMA_CH0>,
    p2_dma: Peri<'static, DMA_CH1>,
) {
    let Pio { mut common, sm0, sm1, .. } = Pio::new(pio0, LedIrqs);
    let prg = PioWs2812Program::new(&mut common);
    let mut ws1: PioWs2812<'_, PIO0, 0, NUM_LEDS> =
        PioWs2812::new(&mut common, sm0, p1_dma, p1_pin, &prg);
    let mut ws2: PioWs2812<'_, PIO0, 1, NUM_LEDS> =
        PioWs2812::new(&mut common, sm1, p2_dma, p2_pin, &prg);

    let mut p1 = PlayerLedState::new();
    let mut p2 = PlayerLedState::new();

    ws1.write(&p1.render()).await;
    ws2.write(&p2.render()).await;

    let mut pending: Option<LedCmd> = None;
    loop {
        let cmd = match pending.take() {
            Some(c) => c,
            None => CMD.receive().await,
        };

        let needs_render = match cmd {
            LedCmd::TeamInit { player, size } => {
                let st = pick(&mut p1, &mut p2, player);
                *st = PlayerLedState::new();
                for m in st.members.iter_mut().take(size as usize) {
                    m.seen = true;
                    m.hp_pct = 100;
                }
                true
            }
            LedCmd::HpUpdate { player, pct } => {
                let st = pick(&mut p1, &mut p2, player);
                let a = st.active;
                st.members[a].hp_pct = pct;
                st.members[a].seen = true;
                true
            }
            LedCmd::SwitchIn { player, slot } => {
                let st = pick(&mut p1, &mut p2, player);
                if (slot as usize) < TEAM_SIZE {
                    st.active = slot as usize;
                    st.members[slot as usize].seen = true;
                    st.members[slot as usize].hp_pct = 100;
                }
                true
            }
            LedCmd::Faint { player, slot } => {
                let st = pick(&mut p1, &mut p2, player);
                if (slot as usize) < TEAM_SIZE {
                    st.members[slot as usize].hp_pct = 0;
                    st.members[slot as usize].status = None;
                }
                true
            }
            LedCmd::SetStatus { player, status } => {
                let st = pick(&mut p1, &mut p2, player);
                let a = st.active;
                st.members[a].status = Some(status);
                true
            }
            LedCmd::CureStatus { player } => {
                let st = pick(&mut p1, &mut p2, player);
                let a = st.active;
                st.members[a].status = None;
                true
            }
            LedCmd::Win { winner } => {
                let gold = RGB8 { r: 200, g: 150, b: 0 };
                let dim  = RGB8 { r:  40, g:   0, b: 0 };
                let grey = RGB8 { r:  60, g:  60, b: 60 };
                let (c1, c2) = match winner {
                    1 => (gold, dim),
                    2 => (dim, gold),
                    _ => (grey, grey),
                };
                ws1.write(&solid(c1)).await;
                ws2.write(&solid(c2)).await;
                false
            }

            LedCmd::LobbyIdle => {
                // Slow "breathing" glow until the next command arrives. Each
                // frame is one small monotonic step of a triangle wave, so even
                // if the demo battle briefly starves this task the animation
                // just pauses and resumes — it never jumps, so no strobe. The
                // base color is pre-cap; both BRIGHTNESS_PCT and the per-frame
                // breath factor scale it down further.
                const STEPS: u8 = 60; // full breath ≈ STEPS * frame_ms
                const HALF: u8 = STEPS / 2;
                let base = RGB8 { r: 60, g: 10, b: 230 };
                let mut step: u8 = 0;
                loop {
                    // Triangle 0..HALF..0 → breath factor 25..100 %.
                    let tri = if step <= HALF { step } else { STEPS - step };
                    let factor = 25 + (tri as u16 * 75 / HALF as u16);
                    let s = |v: u8| ((v as u16 * factor) / 100) as u8;
                    let frame = solid(RGB8 { r: s(base.r), g: s(base.g), b: s(base.b) });
                    ws1.write(&frame).await;
                    ws2.write(&frame).await;
                    step = (step + 1) % STEPS;
                    match select(Timer::after_millis(45), CMD.receive()).await {
                        Either::First(_) => {}                       // next breath frame
                        Either::Second(c) => { pending = Some(c); break; }
                    }
                }
                false
            }

            LedCmd::LobbyWaiting { p1_ready, p2_ready } => {
                let ready   = RGB8 { r: 0, g: 80, b: 0 };
                let waiting = RGB8 { r: 0, g: 0,  b: 15 };
                ws1.write(&solid(if p1_ready { ready } else { waiting })).await;
                ws2.write(&solid(if p2_ready { ready } else { waiting })).await;
                false
            }

            LedCmd::LobbyCountdown => {
                // Three gentle white fade-pulses (no hard strobe): each ramps
                // up then down over ~600 ms.
                for _ in 0..3u8 {
                    for step in 0..=20u8 {
                        let tri = if step <= 10 { step } else { 20 - step }; // 0..10..0
                        let v = tri * 9; // 0..90
                        let f = solid(RGB8 { r: v, g: v, b: v });
                        ws1.write(&f).await;
                        ws2.write(&f).await;
                        Timer::after_millis(28).await;
                    }
                }
                false
            }
        };

        if needs_render {
            ws1.write(&p1.render()).await;
            ws2.write(&p2.render()).await;
        }
    }
}

fn pick<'a>(p1: &'a mut PlayerLedState, p2: &'a mut PlayerLedState, player: u8) -> &'a mut PlayerLedState {
    if player == 1 { p1 } else { p2 }
}

