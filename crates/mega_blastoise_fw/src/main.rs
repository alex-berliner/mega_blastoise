#![no_std]
#![no_main]

extern crate alloc;

mod board_v2;
mod subsystems;

use embassy_executor::Spawner;
use embassy_rp::gpio::{Input, Level, Output, Pull};
use mega_blastoise_fw::mem_profile::init_heap;
#[cfg(feature = "mem-profile")]
use mega_blastoise_fw::mem_profile::heap_snapshot;
use mega_blastoise_fw as _;
use rtt_target::{rtt_init, set_defmt_channel};
use subsystems::{buttons_v2, st7796};

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    init_heap();
    #[cfg(feature = "mem-profile")]
    heap_snapshot("boot");

    // The heap (mem_profile::HEAP_SIZE) wants the RAM, and nothing streams bulk data over RTT,
    // so the defmt buffer stays small.
    let channels = rtt_init! {
        up: { 0: { size: 16 * 1024, name: "defmt" } }
    };
    set_defmt_channel(channels.up.0);

    // ── Stripboard v2: single ST7796 panel + 16 direct buttons ───────────────
    // DeviceSession drives one shared screen; board_v2::run owns main and
    // never returns.

    let lcd = st7796::Lcd::new(
        p.PIO0,
        p.PIN_8,  // MOSI
        p.PIN_9,  // SCK
        p.DMA_CH2,
        Output::new(p.PIN_6, Level::High), // DC
        Output::new(p.PIN_10, Level::Low), // CS
        Output::new(p.PIN_7, Level::High), // RST
        Output::new(p.PIN_5, Level::Low),  // BL
    );
    spawner.spawn(st7796::task(lcd)).expect("st7796 task spawn");

    // BUILD.md §10, order UP DOWN LEFT RIGHT A B INFO MENU per seat.
    let [k0, k1, k2, k3, k4, k5, k6, k7] = buttons_v2::seat_keys(1, [
        Input::new(p.PIN_4, Pull::Up),
        Input::new(p.PIN_3, Pull::Up),
        Input::new(p.PIN_1, Pull::Up),
        Input::new(p.PIN_2, Pull::Up),
        Input::new(p.PIN_14, Pull::Up),
        Input::new(p.PIN_12, Pull::Up),
        Input::new(p.PIN_13, Pull::Up),
        Input::new(p.PIN_11, Pull::Up),
    ]);
    let [k8, k9, k10, k11, k12, k13, k14, k15] = buttons_v2::seat_keys(2, [
        Input::new(p.PIN_20, Pull::Up),
        Input::new(p.PIN_18, Pull::Up),
        Input::new(p.PIN_17, Pull::Up),
        Input::new(p.PIN_21, Pull::Up),
        Input::new(p.PIN_27, Pull::Up),
        Input::new(p.PIN_22, Pull::Up),
        Input::new(p.PIN_26, Pull::Up),
        Input::new(p.PIN_19, Pull::Up),
    ]);
    spawner
        .spawn(buttons_v2::task([
            k0, k1, k2, k3, k4, k5, k6, k7, k8, k9, k10, k11, k12, k13, k14, k15,
        ]))
        .expect("buttons task spawn");
    spawner.spawn(board_v2::pulse_task()).expect("pulse task spawn");
    subsystems::usb_v2::init(p.USB, &spawner);
    spawner
        .spawn(subsystems::battery::task(p.ADC, p.PIN_28))
        .expect("battery task spawn");

    board_v2::run().await
}
