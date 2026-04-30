#![no_std]
#![no_main]

extern crate alloc;

use alloc::boxed::Box;

mod board_effects;
mod pico_battle_input;
mod pn532;
mod usb_input;

use battler::TeamData;
use defmt::info;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::i2c::{Async, InterruptHandler as I2cInterruptHandler, I2c};
use embassy_rp::peripherals::{I2C0, I2C1, USB};
use embassy_rp::usb::{Driver as UsbDriver, InterruptHandler as UsbInterruptHandler};
use embedded_alloc::Heap;
use rtt_target::{rtt_init, set_defmt_channel};
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::{Builder, Config as UsbConfig, UsbDevice};
use board_effects::DefmtBattleEffects;
use mega_blastoise_core::{
    demo_battle_options, demo_engine_opts, demo_team_blue, demo_team_red, run_battle,
    BoardEventQueue, FlashDataStore,
};
use usb_input::UsbBattleInput;
use mega_blastoise_fw as _;

#[global_allocator]
static HEAP: Heap = Heap::empty();

fn init_heap() {
    const HEAP_SIZE: usize = 128 * 1024;
    static mut HEAP_MEM: [u8; HEAP_SIZE] = [0u8; HEAP_SIZE];
    unsafe { HEAP.init(core::ptr::addr_of!(HEAP_MEM) as usize, HEAP_SIZE) }
}

bind_interrupts!(struct Irqs {
    I2C0_IRQ    => I2cInterruptHandler<I2C0>;
    I2C1_IRQ    => I2cInterruptHandler<I2C1>;
    USBCTRL_IRQ => UsbInterruptHandler<USB>;
});

#[embassy_executor::task]
async fn pn532_task_i2c0(bus: &'static mut I2c<'static, I2C0, Async>) {
    pn532::reader_loop(0, bus).await
}

#[embassy_executor::task]
async fn pn532_task_i2c1(bus: &'static mut I2c<'static, I2C1, Async>) {
    pn532::reader_loop(1, bus).await
}

#[embassy_executor::task]
async fn usb_task(usb: &'static mut UsbDevice<'static, UsbDriver<'static, USB>>) -> ! {
    usb.run().await
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    init_heap();

    let channels = rtt_init! {
        up: { 0: { size: 1024, name: "defmt" } }
    };
    set_defmt_channel(channels.up.0);

    // USB CDC for battle CLI
    let driver = UsbDriver::new(p.USB, Irqs);
    let mut config = UsbConfig::new(0xc0de, 0xcafe);
    config.manufacturer = Some("mega-blastoise");
    config.product = Some("Battle CLI");
    config.serial_number = Some("1");
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    let cdc_state  = Box::leak(Box::new(State::new()));
    let config_buf = Box::leak(Box::new([0u8; 256]));
    let bos_buf    = Box::leak(Box::new([0u8; 256]));
    let msos_buf   = Box::leak(Box::new([0u8; 256]));
    let ctrl_buf   = Box::leak(Box::new([0u8; 64]));

    let mut builder = Builder::new(driver, config, config_buf, bos_buf, msos_buf, ctrl_buf);
    let cdc = CdcAcmClass::new(&mut builder, cdc_state, 64);
    let usb = Box::leak(Box::new(builder.build()));
    spawner.spawn(usb_task(usb)).unwrap();
    let (sender, receiver) = cdc.split();
    let mut input = UsbBattleInput::new(sender, receiver);

    // I2C + PN532
    let i2c0 = I2c::new_async(p.I2C0, p.PIN_17, p.PIN_16, Irqs, pn532::i2c_config());
    let i2c1 = I2c::new_async(p.I2C1, p.PIN_19, p.PIN_18, Irqs, pn532::i2c_config());
    let i2c0: &'static mut I2c<'static, I2C0, Async> = Box::leak(Box::new(i2c0));
    let i2c1: &'static mut I2c<'static, I2C1, Async> = Box::leak(Box::new(i2c1));
    spawner.spawn(pn532_task_i2c0(i2c0)).unwrap();
    spawner.spawn(pn532_task_i2c1(i2c1)).unwrap();
    info!("PN532 tasks started (I2C0: GP16/GP17, I2C1: GP18/GP19, addr 0x24)");

    let mut effects = DefmtBattleEffects::new();
    let mut queue = BoardEventQueue::new();

    info!("Initialising data store...");
    let data = FlashDataStore::new();

    let mut battle =
        battler::PublicCoreBattle::new(demo_battle_options(), &data, demo_engine_opts())
            .expect("battle init");
    battle.update_team("p1", TeamData { members: demo_team_red(),  ..Default::default() }).expect("p1");
    battle.update_team("p2", TeamData { members: demo_team_blue(), ..Default::default() }).expect("p2");
    battle.start().expect("battle start");

    info!("Battle started — connect USB serial for CLI input.");

    run_battle(&mut battle, &mut input, &mut queue, &mut effects, |_| {}).await;

    info!("=== Battle over ===");
    loop { cortex_m::asm::wfi(); }
}
