//! WS2812 blast — identical LED data on EVERY GPIO simultaneously.
//!
//! Custom PIO program: instead of side-setting one data pin, each bit is
//! driven onto the whole GP0..GP22 output group with `mov pins, ~null` /
//! `mov pins, null`, so every GPIO carries the exact same 800kHz WS2812
//! waveform at the same time. Probe anywhere; any chain wired to any pin
//! will light. GP23/24/25 (Pico-internal) are untouched.
//!
//! Pattern: all 12 pixels one solid color, repainted at 20Hz, color stepping
//! RED -> GREEN -> BLUE -> WHITE every 2s (logged over RTT).
//!
//! Bit timing matches embassy's ws2812 driver: 10 cycles/bit @ 8MHz PIO clock
//! (T3=3 low tail, T1=2 high start, T2=5 data phase).
//!
//! Build / flash:
//!   cargo rb led_all --features leds

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio::{
    Config, Direction, FifoJoin, InterruptHandler, Pio, ShiftConfig, ShiftDirection,
};
use embassy_time::Timer;
use fixed::types::U24F8;
use mega_blastoise_fw as _;
use mega_blastoise_fw::mem_profile::init_heap;
use pio::{JmpCondition, MovDestination, MovOperation, MovSource, OutDestination};
use rtt_target::{rtt_init, set_defmt_channel};
use smart_leds::RGB8;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

const NUM_LEDS: usize = 12;
const CYCLES_PER_BIT: u32 = 10;

const PALETTE: [(&str, RGB8); 4] = [
    ("RED", RGB8 { r: 60, g: 0, b: 0 }),
    ("GREEN", RGB8 { r: 0, g: 60, b: 0 }),
    ("BLUE", RGB8 { r: 0, g: 0, b: 60 }),
    ("WHITE", RGB8 { r: 35, g: 35, b: 35 }),
];

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    init_heap();

    let channels = rtt_init! {
        up: { 0: { size: 4096, name: "defmt" } }
    };
    set_defmt_channel(channels.up.0);

    defmt::info!("led_all: same WS2812 stream on GP0..GP22 simultaneously");

    let Pio { mut common, mut sm0, .. } = Pio::new(p.PIO0, Irqs);

    // WS2812 timing via MOV to the whole out-pin group (no side-set):
    //   LOW  tail  T3=3: mov pins,null [1] ; out x,1
    //   HIGH start T1=2: mov pins,~null    ; jmp !x do_zero
    //   ONE  data  T2=5: jmp top [4]                (line stays high)
    //   ZERO data  T2=5: do_zero: mov pins,null [4] (line low), wrap to top
    // 10 cycles/bit either way. FIFO-empty stalls on `out` with the line low,
    // which doubles as the >50us latch gap between frames.
    let mut a: pio::Assembler<32> = pio::Assembler::new();
    let mut top = a.label();
    let mut do_zero = a.label();
    let mut wrap_source = a.label();
    a.bind(&mut top);
    a.mov_with_delay(MovDestination::PINS, MovOperation::None, MovSource::NULL, 1);
    a.out(OutDestination::X, 1);
    a.mov(MovDestination::PINS, MovOperation::Invert, MovSource::NULL);
    a.jmp(JmpCondition::XIsZero, &mut do_zero);
    a.jmp_with_delay(JmpCondition::Always, &mut top, 4);
    a.bind(&mut do_zero);
    a.mov_with_delay(MovDestination::PINS, MovOperation::None, MovSource::NULL, 4);
    a.bind(&mut wrap_source);
    let prg = common.load_program(&a.assemble_with_wrap(wrap_source, top));

    let pins = [
        common.make_pio_pin(p.PIN_0),
        common.make_pio_pin(p.PIN_1),
        common.make_pio_pin(p.PIN_2),
        common.make_pio_pin(p.PIN_3),
        common.make_pio_pin(p.PIN_4),
        common.make_pio_pin(p.PIN_5),
        common.make_pio_pin(p.PIN_6),
        common.make_pio_pin(p.PIN_7),
        common.make_pio_pin(p.PIN_8),
        common.make_pio_pin(p.PIN_9),
        common.make_pio_pin(p.PIN_10),
        common.make_pio_pin(p.PIN_11),
        common.make_pio_pin(p.PIN_12),
        common.make_pio_pin(p.PIN_13),
        common.make_pio_pin(p.PIN_14),
        common.make_pio_pin(p.PIN_15),
        common.make_pio_pin(p.PIN_16),
        common.make_pio_pin(p.PIN_17),
        common.make_pio_pin(p.PIN_18),
        common.make_pio_pin(p.PIN_19),
        common.make_pio_pin(p.PIN_20),
        common.make_pio_pin(p.PIN_21),
        common.make_pio_pin(p.PIN_22),
    ];
    let pin_refs: [&_; 23] = core::array::from_fn(|i| &pins[i]);

    let mut cfg = Config::default();
    cfg.set_out_pins(&pin_refs);
    cfg.use_program(&prg, &[]);
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
    sm0.set_pin_dirs(Direction::Out, &pin_refs);
    sm0.set_enable(true);

    let mut dma = p.DMA_CH0;
    let mut tick: u32 = 0;
    loop {
        let (name, color) = PALETTE[(tick / 40) as usize % PALETTE.len()];
        if tick % 40 == 0 {
            defmt::info!("led_all: {} on all GPIOs", name);
        }
        let word =
            (u32::from(color.g) << 24) | (u32::from(color.r) << 16) | (u32::from(color.b) << 8);
        let words = [word; NUM_LEDS];
        sm0.tx().dma_push(dma.reborrow(), &words, false).await;
        tick += 1;
        Timer::after_millis(50).await;
    }
}
