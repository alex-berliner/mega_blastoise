//! Post-game feedback QR code, generated at build time by `build.rs` from
//! `FEEDBACK_URL` (a compile-time constant — change it there and rebuild).
//! Stored as a packed row-major bit matrix; `true` = dark module.

include!(concat!(env!("OUT_DIR"), "/qr.rs"));

/// Is the module at `(x, y)` dark? Row-major, `x`/`y` in `0..QR_SIZE`.
pub fn qr_module(x: usize, y: usize) -> bool {
    let i = y * QR_SIZE + x;
    QR_BITS[i / 8] & (0x80 >> (i % 8)) != 0
}
