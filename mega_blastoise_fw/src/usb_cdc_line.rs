//! Shared CDC ACM RX discipline: chunk writes, CRLF echo, RTT log of completed RX lines.
//!
//! Matches the `usb_loopback` test binary: flush on `\r` or `\n`; after `\r`, the next `\n`
//! is ignored so `\r\n` is one logical line.

use defmt::info;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_usb::class::cdc_acm::Sender;

pub const TX_CHUNK: usize = 64;

pub fn log_usb_rx_line_str_to_rtt(line: &str) {
    if line.is_empty() {
        return;
    }
    info!("usb rx line: {}", line);
}

pub fn log_usb_rx_line_bytes_to_rtt(line: &[u8]) {
    if line.is_empty() {
        return;
    }
    match core::str::from_utf8(line) {
        Ok(s) => info!("usb rx line: {}", s),
        Err(_) => info!("usb rx line (non-utf8) {} B: {=[u8]:02x}", line.len(), line),
    }
}

pub async fn write_all<'d>(
    sender: &mut Sender<'d, Driver<'d, USB>>,
    mut data: &[u8],
) {
    while !data.is_empty() {
        let n = data.len().min(TX_CHUNK);
        let _ = sender.write_packet(&data[..n]).await;
        data = &data[n..];
    }
}

pub async fn write_crlf<'d>(sender: &mut Sender<'d, Driver<'d, USB>>) {
    write_all(sender, b"\r\n").await;
}
