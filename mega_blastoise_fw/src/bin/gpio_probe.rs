//! GPIO wiring discovery — figure out which pins an unknown button PCB uses.
//!
//! Watches every external GPIO (GP0–GP22, GP26–GP28) two ways at once:
//!
//! 1. PASSIVE: all pins are inputs with pull-ups. A button wired pin→GND
//!    prints `GP7 LOW (pressed?)` on press and `GP7 HIGH (released)` on
//!    release. Pins already low at boot are listed once (either wired to GND
//!    or an active-high button — if the latter, its presses print as HIGH).
//!
//! 2. ACTIVE (matrix discovery): a matrix button connects two GPIOs to each
//!    other, which passive pull-up reads can't see. Each cycle every pin is
//!    briefly driven LOW (open-drain style — never driven high) while the
//!    others are read; a pin that follows it low means they're connected:
//!    `PAIR GP5 <-> GP10 (connected)`. Hold a button and the pair prints;
//!    release and it clears. Row/column assignments fall out of which pin
//!    pairs appear per button.
//!
//! Run:  cargo run --bin gpio_probe          (flashes + streams RTT)
//! Skipped pins: GP23/24/25/29 (Pico board internals: SMPS, VBUS sense,
//! onboard LED, VSYS sense).

#![no_std]
#![no_main]

use cortex_m::asm::delay as asm_delay;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Flex, Pull};
use embassy_time::Timer;
use mega_blastoise_fw as _;
use mega_blastoise_fw::mem_profile::init_heap;
use rtt_target::{rtt_init, set_defmt_channel};

const N: usize = 26;
/// GPIO number for each slot (external pins only).
const GPIO_NUM: [u8; N] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 26, 27, 28,
];

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    init_heap();

    let channels = rtt_init! {
        up: { 0: { size: 8192, name: "defmt" } }
    };
    set_defmt_channel(channels.up.0);

    let mut pins: [Flex; N] = [
        Flex::new(p.PIN_0),
        Flex::new(p.PIN_1),
        Flex::new(p.PIN_2),
        Flex::new(p.PIN_3),
        Flex::new(p.PIN_4),
        Flex::new(p.PIN_5),
        Flex::new(p.PIN_6),
        Flex::new(p.PIN_7),
        Flex::new(p.PIN_8),
        Flex::new(p.PIN_9),
        Flex::new(p.PIN_10),
        Flex::new(p.PIN_11),
        Flex::new(p.PIN_12),
        Flex::new(p.PIN_13),
        Flex::new(p.PIN_14),
        Flex::new(p.PIN_15),
        Flex::new(p.PIN_16),
        Flex::new(p.PIN_17),
        Flex::new(p.PIN_18),
        Flex::new(p.PIN_19),
        Flex::new(p.PIN_20),
        Flex::new(p.PIN_21),
        Flex::new(p.PIN_22),
        Flex::new(p.PIN_26),
        Flex::new(p.PIN_27),
        Flex::new(p.PIN_28),
    ];

    for pin in pins.iter_mut() {
        pin.set_pull(Pull::Up);
        pin.set_as_input();
    }
    Timer::after_millis(5).await;

    // Baseline: what does "at rest" look like?
    let mut baseline = [false; N]; // true = high
    for (i, pin) in pins.iter_mut().enumerate() {
        baseline[i] = pin.is_high();
    }
    defmt::info!("gpio_probe: watching GP0-22, GP26-28 (pull-ups on). Press buttons…");
    for i in 0..N {
        if !baseline[i] {
            defmt::warn!(
                "GP{} is LOW at rest — wired to GND, or an active-high button (press prints HIGH)",
                GPIO_NUM[i]
            );
        }
    }

    let mut last = baseline;
    let mut pairs_last = [[false; N]; N];

    loop {
        // ── PASSIVE: level changes on plain inputs ────────────────────────────
        for i in 0..N {
            let high = pins[i].is_high();
            if high != last[i] {
                if high {
                    defmt::info!("GP{} HIGH (released)", GPIO_NUM[i]);
                } else {
                    defmt::info!("GP{} LOW  (pressed?)", GPIO_NUM[i]);
                }
                last[i] = high;
            }
        }

        // ── ACTIVE: drive each pin low, see who follows (matrix wiring) ──────
        let mut pairs_now = [[false; N]; N];
        for i in 0..N {
            if !last[i] {
                continue; // already low — driving it teaches nothing
            }
            pins[i].set_low();
            pins[i].set_as_output();
            asm_delay(2500); // ≈20 µs settle
            for j in 0..N {
                if j == i || !last[j] {
                    continue;
                }
                if pins[j].is_low() {
                    let (a, b) = if i < j { (i, j) } else { (j, i) };
                    pairs_now[a][b] = true;
                }
            }
            pins[i].set_as_input(); // back to pulled-up input (never driven high)
            asm_delay(2500);
        }
        for a in 0..N {
            for b in 0..N {
                if pairs_now[a][b] && !pairs_last[a][b] {
                    defmt::info!("PAIR GP{} <-> GP{} (connected)", GPIO_NUM[a], GPIO_NUM[b]);
                }
                if !pairs_now[a][b] && pairs_last[a][b] {
                    defmt::info!("PAIR GP{} <-> GP{} released", GPIO_NUM[a], GPIO_NUM[b]);
                }
            }
        }
        pairs_last = pairs_now;

        Timer::after_millis(25).await;
    }
}
