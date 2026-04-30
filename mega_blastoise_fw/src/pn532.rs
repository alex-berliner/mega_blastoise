//! Initial PN532 bring-up on two independent I²C buses (RP2040 `I2C0` + `I2C1`).
//!
//! Wiring (change pins in `main.rs` if needed):
//! - **Reader A (I2C0):** SCL = GP17, SDA = GP16 (keeps GPIO 6–15 free for battle buttons).
//! - **Reader B (I2C1):** SCL = GP19, SDA = GP18.
//!
//! Both modules use the default 7-bit address **0x24** on **different** buses.
//!
//! Each background task sends **GetFirmwareVersion** on an interval and drops the response
//! buffer — enough to exercise framing until real NFC handling exists.

use embassy_rp::i2c::{Async, Error, Instance, I2c};
use embassy_time::Timer;
use embedded_hal_async::i2c::I2c as _;

/// PN532 7-bit I²C address.
pub const PN532_I2C_ADDR: u8 = 0x24;

/// Frame: GetFirmwareVersion (`D4 02`), checksums per NXP / common driver stacks.
const GET_FIRMWARE_VERSION: [u8; 9] = [
    0x00, 0x00, 0xFF, 0x02, 0xFE, 0xD4, 0x02, 0x2A, 0x00,
];

const POLL_INTERVAL_MS: u64 = 250;

/// Reply scratch — sized for short ACK frames; increase when parsing tag payloads.
const READ_BUF_LEN: usize = 32;

async fn poll_once<T: Instance>(i2c: &mut I2c<'_, T, Async>) -> Result<(), Error> {
    let mut buf = [0u8; READ_BUF_LEN];
    i2c.write(PN532_I2C_ADDR, &GET_FIRMWARE_VERSION).await?;
    Timer::after_millis(10).await;
    i2c.read(PN532_I2C_ADDR, &mut buf).await?;
    let _ = buf;
    Ok(())
}

/// Loops forever; run one instance per PN532 from a dedicated embassy task.
pub async fn reader_loop<T: Instance>(reader_id: u8, i2c: &mut I2c<'_, T, Async>) {
    Timer::after_millis(50).await;
    loop {
        match poll_once(i2c).await {
            Ok(()) => {
                defmt::trace!("pn532 #{} ok", reader_id);
            }
            Err(_) => {
                defmt::trace!("pn532 #{} I²C: no ack", reader_id);
            }
        }
        Timer::after_millis(POLL_INTERVAL_MS).await;
    }
}

pub fn i2c_config() -> embassy_rp::i2c::Config {
    let mut c = embassy_rp::i2c::Config::default();
    c.frequency = 100_000;
    c
}
