//! Stripboard v2: the firmware skin over core's `DeviceRunner`.
//!
//! The game — lobby, ready sequence, battles, narration — is
//! `mega_blastoise_core::device_runner`, shared verbatim with the web
//! build. This module is raw IO: the port implementation (embassy clock and
//! timers, defmt console, ST7796 presentation, timing-jitter entropy) and
//! the input channel the button task feeds.

use core::sync::atomic::{AtomicBool, Ordering};

use embassy_time::{Duration, Instant, Timer};
use mega_blastoise_core::device_runner::{
    DeviceEvent, DeviceInputChannel, DevicePort, DeviceRunner,
};
use mega_blastoise_core::device_session::DeviceSession;
use mega_blastoise_core::device_ui::render_session_frame;
use mega_blastoise_core::OledController;

use crate::subsystems::st7796;

/// The one queue every input source pushes into.
pub static INPUT: DeviceInputChannel = DeviceInputChannel::new();

/// True while no frame is rendering/queued/blitting. `present` may only
/// build a frame after winning this flag; the display task returns it after
/// each blit. Keeps exactly one 300 KB frame alive (the heap is sized for
/// that, not for a queue).
pub static FRAME_FREE: AtomicBool = AtomicBool::new(true);
/// A present arrived while the panel was busy; the display task answers by
/// pushing a Pulse, which re-presents from fresh state.
pub static NEED_REPAINT: AtomicBool = AtomicBool::new(false);

struct FwPort;

impl DevicePort for FwPort {
    fn now_ms(&self) -> u64 {
        Instant::now().as_millis()
    }

    async fn sleep_ms(&self, ms: u32) {
        Timer::after(Duration::from_millis(ms as u64)).await;
    }

    fn line_out(&self, line: &str) {
        defmt::info!("{=str}", line);
        crate::subsystems::usb_v2::log_line(line);
    }

    fn present(&self, session: &DeviceSession, ctl: &OledController, lobby: bool) {
        if FRAME_FREE
            .compare_exchange(true, false, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            NEED_REPAINT.store(true, Ordering::Release);
            return;
        }
        #[allow(unused_mut)]
        let mut frame = render_session_frame(session, ctl, lobby);
        #[cfg(feature = "input-debug")]
        mega_blastoise_core::device_view::draw_input_debug(
            &mut frame,
            crate::subsystems::buttons_v2::masks(),
        );
        st7796::FRAME.signal(frame);
    }

    fn seed(&self) -> u64 {
        Instant::now().as_ticks() ^ 0xdead_beef_cafe_babe
    }

    fn battery(&self) -> mega_blastoise_core::display_color::Battery {
        crate::subsystems::battery::battery()
    }
}

/// The sprite-bob heartbeat; also the repaint-heal tick (core presents on
/// every Pulse).
#[embassy_executor::task]
pub async fn pulse_task() -> ! {
    loop {
        Timer::after(Duration::from_millis(mega_blastoise_core::BOB_TICK_MS as u64)).await;
        let _ = INPUT.try_send(DeviceEvent::Pulse);
    }
}

pub async fn run() -> ! {
    defmt::info!("board_v2: device runner up");
    let port = FwPort;
    let runner = DeviceRunner::new(&port, &INPUT);
    runner.run().await
}
