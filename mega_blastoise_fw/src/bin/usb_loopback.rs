//! CDC ACM loopback: every byte received on the USB serial port is echoed back.
//! Each packet is also logged over defmt (RTT on SWD).
//!
//! Build / flash:
//! `cargo build --bin usb_loopback`
//! `cargo run --bin usb_loopback` (uses `.cargo/config` probe-rs runner)

#![no_std]
#![no_main]

extern crate alloc;

use alloc::boxed::Box;

use defmt::info;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver as UsbDriver, InterruptHandler as UsbInterruptHandler};
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::{Builder, Config as UsbConfig, UsbDevice};
use mega_blastoise_fw::mem_profile::init_heap;
use rtt_target::{rtt_init, set_defmt_channel};

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => UsbInterruptHandler<USB>;
});

#[embassy_executor::task]
async fn usb_task(usb: &'static mut UsbDevice<'static, UsbDriver<'static, USB>>) -> ! {
    usb.run().await
}

async fn cdc_echo(
    mut sender: embassy_usb::class::cdc_acm::Sender<'static, UsbDriver<'static, USB>>,
    mut receiver: embassy_usb::class::cdc_acm::Receiver<'static, UsbDriver<'static, USB>>,
) -> ! {
    let mut buf = [0u8; 64];
    loop {
        match receiver.read_packet(&mut buf).await {
            Ok(n) => {
                // Mirror traffic on SWD (defmt / RTT) for debugging alongside USB echo.
                if n > 0 {
                    info!("cdc rx {} B (hex) {=[u8]:02x}", n, &buf[..n]);
                }
                let _ = sender.write_packet(&buf[..n]).await;
            }
            Err(_) => {
                info!("cdc RX stalled — wait for host");
                receiver.wait_connection().await;
                info!("cdc RX ready");
            }
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    init_heap();

    let channels = rtt_init! {
        up: { 0: { size: 1024, name: "defmt" } }
    };
    set_defmt_channel(channels.up.0);

    info!("USB CDC loopback — characters are echoed to the host");

    let p = embassy_rp::init(Default::default());

    let driver = UsbDriver::new(p.USB, Irqs);
    let mut config = UsbConfig::new(0xc0de, 0xcafe);
    config.manufacturer = Some("mega-blastoise");
    config.product = Some("USB loopback test");
    config.serial_number = Some("loopback");
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    let cdc_state = Box::leak(Box::new(State::new()));
    let config_buf = Box::leak(Box::new([0u8; 256]));
    let bos_buf = Box::leak(Box::new([0u8; 256]));
    let msos_buf = Box::leak(Box::new([0u8; 256]));
    let ctrl_buf = Box::leak(Box::new([0u8; 64]));

    let mut builder = Builder::new(driver, config, config_buf, bos_buf, msos_buf, ctrl_buf);
    let cdc = CdcAcmClass::new(&mut builder, cdc_state, 64);
    let usb = Box::leak(Box::new(builder.build()));
    spawner.spawn(usb_task(usb)).unwrap();
    let (sender, receiver) = cdc.split();
    cdc_echo(sender, receiver).await;
}
