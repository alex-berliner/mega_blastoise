//! WS2812B NeoPixel strip driver — one 24-LED chain on GP20 via PIO0.
//!
//! LED layout (single daisy-chain, both players):
//!   P1: LEDs  0–7  HP bar  |  8–10 party slots  |  11 status
//!   P2: LEDs 12–19 HP bar  | 20–22 party slots  | 23 status
//!
//! Call [`send`] from `BattleEffects::on_event` to queue an update.
//! The task re-renders the full 24-LED frame after each command.
//!
//! Two-strip variant: swap the single `PioWs2812<_, _, 24>` for two
//! `PioWs2812<_, _, 12>` instances on separate SM/GPIO and write each
//! player buffer independently — the per-player state structs are already
//! separated.

use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::{DMA_CH0, PIN_20, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::pio_programs::ws2812::{PioWs2812, PioWs2812Program};
use embassy_rp::Peri;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use smart_leds::RGB8;

// ── Interrupt binding ─────────────────────────────────────────────────────────

bind_interrupts!(struct LedIrqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

// ── Layout constants ──────────────────────────────────────────────────────────

const NUM_LEDS: usize = 24;
const PER_PLAYER: usize = 12;

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

    fn color(self) -> RGB8 {
        match self {
            Self::Paralyzed => RGB8 { r: 255, g: 200, b:   0 },
            Self::Burned    => RGB8 { r: 255, g:  60, b:   0 },
            Self::Frozen    => RGB8 { r:   0, g: 200, b: 255 },
            Self::Poisoned  => RGB8 { r: 150, g:   0, b: 200 },
            Self::Asleep    => RGB8 { r:   0, g:  80, b:   0 },
        }
    }
}

// ── Command channel ───────────────────────────────────────────────────────────

pub enum LedCmd {
    HpUpdate { player: u8, pct: u8 },
    Faint { player: u8 },
    SetStatus { player: u8, status: LedStatus },
    CureStatus { player: u8 },
    Win { winner: u8 },
}

static CMD: Channel<CriticalSectionRawMutex, LedCmd, 8> = Channel::new();

pub fn send(cmd: LedCmd) {
    if CMD.try_send(cmd).is_err() {
        defmt::warn!("led: channel full, cmd dropped");
    }
}

// ── Per-player state ──────────────────────────────────────────────────────────

struct PlayerLedState {
    hp_pct: u8,
    alive_count: u8,
    status: Option<LedStatus>,
}

impl PlayerLedState {
    const fn new() -> Self {
        Self { hp_pct: 100, alive_count: 3, status: None }
    }

    fn render(&self) -> [RGB8; PER_PLAYER] {
        let off = RGB8 { r: 0, g: 0, b: 0 };
        let mut buf = [off; PER_PLAYER];

        // LEDs 0–7: HP bar
        let lit = hp_lit(self.hp_pct);
        let color = hp_color(self.hp_pct);
        for i in 0..lit {
            buf[i] = color;
        }

        // LEDs 8–10: party slots — alive = dim green, fainted = off
        for i in 0..self.alive_count.min(3) as usize {
            buf[8 + i] = RGB8 { r: 0, g: 30, b: 0 };
        }

        // LED 11: status indicator
        buf[11] = self.status.map(|s| s.color()).unwrap_or(off);

        buf
    }
}

fn hp_lit(pct: u8) -> usize {
    if pct == 0 { return 0; }
    ((pct as usize * 8 + 99) / 100).min(8)
}

fn hp_color(pct: u8) -> RGB8 {
    if pct > 50      { RGB8 { r:   0, g: 180, b: 0 } }
    else if pct > 25 { RGB8 { r: 180, g: 150, b: 0 } }
    else             { RGB8 { r: 200, g:   0, b: 0 } }
}

fn build_frame(p1: &PlayerLedState, p2: &PlayerLedState) -> [RGB8; NUM_LEDS] {
    let off = RGB8 { r: 0, g: 0, b: 0 };
    let mut frame = [off; NUM_LEDS];
    let b1 = p1.render();
    let b2 = p2.render();
    frame[..PER_PLAYER].copy_from_slice(&b1);
    frame[PER_PLAYER..].copy_from_slice(&b2);
    frame
}

// ── Embassy task ──────────────────────────────────────────────────────────────

#[embassy_executor::task]
pub async fn task(
    pio0: Peri<'static, PIO0>,
    pin20: Peri<'static, PIN_20>,
    dma: Peri<'static, DMA_CH0>,
) {
    let Pio { mut common, sm0, .. } = Pio::new(pio0, LedIrqs);
    let prg = PioWs2812Program::new(&mut common);
    let mut ws: PioWs2812<'_, PIO0, 0, NUM_LEDS> =
        PioWs2812::new(&mut common, sm0, dma, pin20, &prg);

    let mut p1 = PlayerLedState::new();
    let mut p2 = PlayerLedState::new();

    ws.write(&build_frame(&p1, &p2)).await;

    loop {
        let needs_render = match CMD.receive().await {
            LedCmd::HpUpdate { player, pct } => {
                let st = if player == 1 { &mut p1 } else { &mut p2 };
                st.hp_pct = pct;
                true
            }
            LedCmd::Faint { player } => {
                let st = if player == 1 { &mut p1 } else { &mut p2 };
                st.hp_pct = 0;
                if st.alive_count > 0 { st.alive_count -= 1; }
                st.status = None;
                true
            }
            LedCmd::SetStatus { player, status } => {
                let st = if player == 1 { &mut p1 } else { &mut p2 };
                st.status = Some(status);
                true
            }
            LedCmd::CureStatus { player } => {
                let st = if player == 1 { &mut p1 } else { &mut p2 };
                st.status = None;
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
                let mut frame = [RGB8 { r: 0, g: 0, b: 0 }; NUM_LEDS];
                for i in 0..PER_PLAYER       { frame[i] = c1; }
                for i in PER_PLAYER..NUM_LEDS { frame[i] = c2; }
                ws.write(&frame).await;
                false
            }
        };

        if needs_render {
            ws.write(&build_frame(&p1, &p2)).await;
        }
    }
}
