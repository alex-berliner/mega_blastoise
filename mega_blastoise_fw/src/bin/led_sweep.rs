//! WS2812 data-pin sweep — drives an LED frame out of EVERY GPIO in turn.
//!
//! Sanity test for "which pin is this chain actually wired to?": one PIO
//! state machine gets re-pointed at each GPIO for ~0.25s, pushing a solid
//! color frame the whole time. A chain holds the last frame it received, so
//! after a sweep each reachable chain shows the color of the pin that fed it.
//!
//! Color code (pin number mod 8):
//!   0=RED 1=GREEN 2=BLUE 3=YELLOW 4=CYAN 5=MAGENTA 6=WHITE 7=ORANGE
//! e.g. GP0 red, GP1 green, GP8 red again. RTT logs each step.
//!
//! Sweeps GP0-GP22 and GP26-GP28 (skips the Pico-internal GP23/24/25).
//! The buzzer (GP21) will chirp briefly each sweep — that's expected.
//!
//! Build / flash:
//!   cargo rb led_sweep --features leds

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::{
    Common, Config, Direction, FifoJoin, InterruptHandler, LoadedProgram, Pio, ShiftConfig,
    ShiftDirection, StateMachine,
};
use embassy_rp::Peri;
use embassy_time::Timer;
use fixed::types::U24F8;
use mega_blastoise_fw as _;
use mega_blastoise_fw::mem_profile::init_heap;
use rtt_target::{rtt_init, set_defmt_channel};
use smart_leds::RGB8;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

const NUM_LEDS: usize = 12;

// Same waveform constants as embassy's PioWs2812 driver.
const T1: u8 = 2;
const T2: u8 = 5;
const T3: u8 = 3;
const CYCLES_PER_BIT: u32 = (T1 + T2 + T3) as u32;

const PALETTE: [(&str, RGB8); 8] = [
    ("RED", RGB8 { r: 60, g: 0, b: 0 }),
    ("GREEN", RGB8 { r: 0, g: 60, b: 0 }),
    ("BLUE", RGB8 { r: 0, g: 0, b: 60 }),
    ("YELLOW", RGB8 { r: 50, g: 40, b: 0 }),
    ("CYAN", RGB8 { r: 0, g: 45, b: 45 }),
    ("MAGENTA", RGB8 { r: 50, g: 0, b: 50 }),
    ("WHITE", RGB8 { r: 35, g: 35, b: 35 }),
    ("ORANGE", RGB8 { r: 60, g: 20, b: 0 }),
];

/// The ws2812 PIO program, verbatim from embassy_rp::pio_programs::ws2812
/// (its LoadedProgram is private, and we need to re-`use_program` per pin).
fn load_ws2812_program<'d>(common: &mut Common<'d, PIO0>) -> LoadedProgram<'d, PIO0> {
    let side_set = pio::SideSet::new(false, 1, false);
    let mut a: pio::Assembler<32> = pio::Assembler::new_with_side_set(side_set);

    let mut wrap_target = a.label();
    let mut wrap_source = a.label();
    let mut do_zero = a.label();
    a.set_with_side_set(pio::SetDestination::PINDIRS, 1, 0);
    a.bind(&mut wrap_target);
    a.out_with_delay_and_side_set(pio::OutDestination::X, 1, T3 - 1, 0);
    a.jmp_with_delay_and_side_set(pio::JmpCondition::XIsZero, &mut do_zero, T1 - 1, 1);
    a.jmp_with_delay_and_side_set(pio::JmpCondition::Always, &mut wrap_target, T2 - 1, 1);
    a.bind(&mut do_zero);
    a.nop_with_delay_and_side_set(T2 - 1, 0);
    a.bind(&mut wrap_source);

    let prg = a.assemble_with_wrap(wrap_source, wrap_target);
    common.load_program(&prg)
}

async fn send_frame(
    sm: &mut StateMachine<'static, PIO0, 0>,
    dma: &mut Peri<'static, DMA_CH0>,
    color: RGB8,
) {
    let mut words = [0u32; NUM_LEDS];
    for w in words.iter_mut() {
        *w = (u32::from(color.g) << 24) | (u32::from(color.r) << 16) | (u32::from(color.b) << 8);
    }
    sm.tx().dma_push(dma.reborrow(), &words, false).await;
    // Latch: line rests low once the FIFO drains (stop bit is side-set 0).
    Timer::after_micros(400).await;
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    init_heap();

    let channels = rtt_init! {
        up: { 0: { size: 4096, name: "defmt" } }
    };
    set_defmt_channel(channels.up.0);

    defmt::info!("led_sweep: WS2812 frame out of every GPIO, 0.25s each");

    let Pio { mut common, mut sm0, .. } = Pio::new(p.PIO0, Irqs);
    let prg = load_ws2812_program(&mut common);
    let mut dma = p.DMA_CH0;

    // Every sweepable GPIO, pre-muxed to PIO0. GP23/24/25 are Pico-internal
    // (SMPS mode, VBUS sense, onboard LED) and stay untouched.
    let pins = [
        (0u8, common.make_pio_pin(p.PIN_0)),
        (1, common.make_pio_pin(p.PIN_1)),
        (2, common.make_pio_pin(p.PIN_2)),
        (3, common.make_pio_pin(p.PIN_3)),
        (4, common.make_pio_pin(p.PIN_4)),
        (5, common.make_pio_pin(p.PIN_5)),
        (6, common.make_pio_pin(p.PIN_6)),
        (7, common.make_pio_pin(p.PIN_7)),
        (8, common.make_pio_pin(p.PIN_8)),
        (9, common.make_pio_pin(p.PIN_9)),
        (10, common.make_pio_pin(p.PIN_10)),
        (11, common.make_pio_pin(p.PIN_11)),
        (12, common.make_pio_pin(p.PIN_12)),
        (13, common.make_pio_pin(p.PIN_13)),
        (14, common.make_pio_pin(p.PIN_14)),
        (15, common.make_pio_pin(p.PIN_15)),
        (16, common.make_pio_pin(p.PIN_16)),
        (17, common.make_pio_pin(p.PIN_17)),
        (18, common.make_pio_pin(p.PIN_18)),
        (19, common.make_pio_pin(p.PIN_19)),
        (20, common.make_pio_pin(p.PIN_20)),
        (21, common.make_pio_pin(p.PIN_21)),
        (22, common.make_pio_pin(p.PIN_22)),
        (26, common.make_pio_pin(p.PIN_26)),
        (27, common.make_pio_pin(p.PIN_27)),
        (28, common.make_pio_pin(p.PIN_28)),
    ];

    let mut sweep: u32 = 0;
    loop {
        sweep += 1;
        defmt::info!("--- sweep #{} ---", sweep);
        for (gp, pin) in pins.iter() {
            let (color_name, color) = PALETTE[(*gp as usize) % PALETTE.len()];
            defmt::info!("GP{}: {}", gp, color_name);

            sm0.set_enable(false);
            let mut cfg = Config::default();
            cfg.set_out_pins(&[pin]);
            cfg.set_set_pins(&[pin]);
            cfg.use_program(&prg, &[pin]);
            let clock_freq = U24F8::from_num(embassy_rp::clocks::clk_sys_freq() / 1000);
            let bit_freq = U24F8::from_num(800) * CYCLES_PER_BIT;
            cfg.clock_divider = clock_freq / bit_freq;
            cfg.fifo_join = FifoJoin::TxOnly;
            cfg.shift_out = ShiftConfig {
                auto_fill: true,
                threshold: 24,
                direction: ShiftDirection::Left,
            };
            sm0.set_config(&cfg);
            sm0.set_pin_dirs(Direction::Out, &[pin]);
            sm0.clear_fifos();
            sm0.restart();
            sm0.set_enable(true);

            // ~0.25s of frames on this pin.
            for _ in 0..5 {
                send_frame(&mut sm0, &mut dma, color).await;
                Timer::after_millis(50).await;
            }
        }
        defmt::info!("--- sweep #{} done; chains hold last color. 3s pause ---", sweep);
        Timer::after_millis(3000).await;
    }
}
