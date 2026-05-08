//! USB CDC-ACM subsystem init.
//!
//! Call [`init`] to set up the USB peripheral, spawn the device task, and return
//! a ready [`UsbButtonInput`].

extern crate alloc;

use alloc::boxed::Box;

use embassy_executor::Spawner;
use embassy_rp::Peri;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver as UsbDriver, InterruptHandler as UsbInterruptHandler};
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::{Builder, Config as UsbConfig, UsbDevice};

use crate::usb_input::UsbButtonInput;

embassy_rp::bind_interrupts!(struct UsbIrqs {
    USBCTRL_IRQ => UsbInterruptHandler<USB>;
});

#[embassy_executor::task]
async fn usb_device_task(usb: &'static mut UsbDevice<'static, UsbDriver<'static, USB>>) -> ! {
    usb.run().await
}

/// Initialise USB CDC-ACM and return the button input driver.
pub fn init(usb_periph: Peri<'static, USB>, spawner: &Spawner) -> UsbButtonInput<'static> {
    let driver = UsbDriver::new(usb_periph, UsbIrqs);

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
    let usb: &'static mut UsbDevice<'static, UsbDriver<'static, USB>> =
        Box::leak(Box::new(builder.build()));

    spawner.spawn(usb_device_task(usb)).unwrap();

    let (sender, receiver) = cdc.split();
    UsbButtonInput::new(sender, receiver)
}
