#![no_std]
#![no_main]

extern crate alloc;

use alloc::boxed::Box;

mod board_effects;
mod pico_battle_input;
mod pn532;

use battler::TeamData;
use defmt::info;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Input, Pull};
use embassy_rp::i2c::{Async, InterruptHandler, I2c};
use embassy_rp::peripherals::{I2C0, I2C1};
use embedded_alloc::Heap;
use board_effects::DefmtBattleEffects;
use mega_blastoise_core::{
    demo_battle_options, demo_engine_opts, demo_team_blue, demo_team_red, run_battle,
    BoardEventQueue, FlashDataStore,
};
use pico_battle_input::PicoBattleInput;
use mega_blastoise_fw as _;

#[global_allocator]
static HEAP: Heap = Heap::empty();

fn init_heap() {
    const HEAP_SIZE: usize = 128 * 1024;
    static mut HEAP_MEM: [u8; HEAP_SIZE] = [0u8; HEAP_SIZE];
    unsafe { HEAP.init(core::ptr::addr_of!(HEAP_MEM) as usize, HEAP_SIZE) }
}

bind_interrupts!(struct Irqs {
    I2C0_IRQ => InterruptHandler<I2C0>;
    I2C1_IRQ => InterruptHandler<I2C1>;
});

#[embassy_executor::task]
async fn pn532_task_i2c0(bus: &'static mut I2c<'static, I2C0, Async>) {
    pn532::reader_loop(0, bus).await
}

#[embassy_executor::task]
async fn pn532_task_i2c1(bus: &'static mut I2c<'static, I2C1, Async>) {
    pn532::reader_loop(1, bus).await
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    init_heap();

    let i2c0 = I2c::new_async(p.I2C0, p.PIN_17, p.PIN_16, Irqs, pn532::i2c_config());
    let i2c1 = I2c::new_async(p.I2C1, p.PIN_19, p.PIN_18, Irqs, pn532::i2c_config());
    let i2c0: &'static mut I2c<'static, I2C0, Async> = Box::leak(Box::new(i2c0));
    let i2c1: &'static mut I2c<'static, I2C1, Async> = Box::leak(Box::new(i2c1));
    spawner.spawn(pn532_task_i2c0(i2c0)).unwrap();
    spawner.spawn(pn532_task_i2c1(i2c1)).unwrap();
    info!("PN532 tasks started (I2C0: GP16/GP17, I2C1: GP18/GP19, addr 0x24)");

    let move_pins = [
        Input::new(p.PIN_6, Pull::Up),
        Input::new(p.PIN_7, Pull::Up),
        Input::new(p.PIN_8, Pull::Up),
        Input::new(p.PIN_9, Pull::Up),
    ];
    let switch_pins = [
        Input::new(p.PIN_10, Pull::Up),
        Input::new(p.PIN_11, Pull::Up),
        Input::new(p.PIN_12, Pull::Up),
        Input::new(p.PIN_13, Pull::Up),
        Input::new(p.PIN_14, Pull::Up),
        Input::new(p.PIN_15, Pull::Up),
    ];
    let mut input = PicoBattleInput::new(move_pins, switch_pins);
    let mut effects = DefmtBattleEffects::new();
    let mut queue = BoardEventQueue::new();

    info!("=== mega-blastoise PoC (GPIO + 2× PN532 I²C) ===");
    info!("Initialising data store...");

    let data = FlashDataStore::new();

    let mut battle =
        battler::PublicCoreBattle::new(demo_battle_options(), &data, demo_engine_opts())
            .expect("battle init");

    battle
        .update_team("p1", TeamData { members: demo_team_red(), ..Default::default() })
        .expect("set p1 team");
    battle
        .update_team("p2", TeamData { members: demo_team_blue(), ..Default::default() })
        .expect("set p2 team");

    battle.start().expect("battle start");
    info!("Battle started — GPIO move/switch; prompts also emit board events.");

    run_battle(&mut battle, &mut input, &mut queue, &mut effects);

    info!("=== Battle over ===");
    loop {
        cortex_m::asm::wfi();
    }
}
