//! The post-game QR screen must render a well-formed code: a lit (light)
//! panel with dark modules punched out, correct polarity for phone cameras.

use mega_blastoise_core::qr::{qr_module, QR_SIZE};
use mega_blastoise_core::{render_screen, OledFrameBuffer, Screen};

#[test]
fn qr_screen_renders_dark_modules_on_lit_panel() {
    let mut fb = OledFrameBuffer::new();
    render_screen(&mut fb, &Screen::Qr);

    // Mirror render_qr_screen's layout math.
    let scale = (64 / (QR_SIZE + 2)).max(1);
    let quiet = scale;
    let side = QR_SIZE * scale + 2 * quiet;
    let (x0, y0) = (2usize, (64 - side) / 2);

    // Quiet border is lit.
    assert!(fb.fb[y0][x0], "quiet zone must be lit");
    // Every module matches the generated matrix (dark = unlit pixel).
    for my in 0..QR_SIZE {
        for mx in 0..QR_SIZE {
            let px = fb.fb[y0 + quiet + my * scale][x0 + quiet + mx * scale];
            assert_eq!(px, !qr_module(mx, my), "module ({mx},{my}) polarity");
        }
    }
    // A QR's top-left finder corner is always dark.
    assert!(qr_module(0, 0), "finder corner should be a dark module");
}
