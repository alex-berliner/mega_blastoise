//! I²C + PN532 NFC reader subsystem init.
//!
//! Call [`init`] to configure both I²C buses and spawn a background reader task per module.
//!
//! Default pin mapping (change here if rewired):
//! - I2C0: SCL = GP17, SDA = GP16
//! - I2C1: SCL = GP19, SDA = GP18

extern crate alloc;

use alloc::boxed::Box;

use embassy_executor::Spawner;
use embassy_rp::Peri;
use embassy_rp::i2c::{Async, InterruptHandler as I2cInterruptHandler, I2c};
use embassy_rp::peripherals::{I2C0, I2C1, PIN_16, PIN_17, PIN_18, PIN_19};

use crate::pn532;

embassy_rp::bind_interrupts!(struct NfcIrqs {
    I2C0_IRQ => I2cInterruptHandler<I2C0>;
    I2C1_IRQ => I2cInterruptHandler<I2C1>;
});

#[embassy_executor::task]
async fn pn532_task_i2c0(bus: &'static mut I2c<'static, I2C0, Async>) {
    pn532::reader_loop(0, bus).await
}

#[embassy_executor::task]
async fn pn532_task_i2c1(bus: &'static mut I2c<'static, I2C1, Async>) {
    pn532::reader_loop(1, bus).await
}

/// Initialise I²C buses and spawn PN532 reader tasks.
pub fn init(
    i2c0_periph: Peri<'static, I2C0>,
    i2c1_periph: Peri<'static, I2C1>,
    pin16: Peri<'static, PIN_16>,
    pin17: Peri<'static, PIN_17>,
    pin18: Peri<'static, PIN_18>,
    pin19: Peri<'static, PIN_19>,
    spawner: &Spawner,
) {
    let i2c0 = I2c::new_async(i2c0_periph, pin17, pin16, NfcIrqs, pn532::i2c_config());
    let i2c1 = I2c::new_async(i2c1_periph, pin19, pin18, NfcIrqs, pn532::i2c_config());

    let i2c0: &'static mut I2c<'static, I2C0, Async> = Box::leak(Box::new(i2c0));
    let i2c1: &'static mut I2c<'static, I2C1, Async> = Box::leak(Box::new(i2c1));

    spawner.spawn(pn532_task_i2c0(i2c0)).unwrap();
    spawner.spawn(pn532_task_i2c1(i2c1)).unwrap();
}
