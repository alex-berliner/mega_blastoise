//! The board without the glass: core's `DeviceRunner` on the host, speaking
//! the phone bridge's line protocol (`mega_blastoise_core::online`) on
//! stdin and stdout, so a script can stand in for the phone and the Pico.
//!
//!   stdin   the lines the bridge page sends the board
//!   stdout  the lines the board sends the bridge page
//!   stderr  the console lines (the firmware's CDC log)
//!
//! Nothing is drawn and the narration does not pace itself (`:anim off`'s
//! effect); the board's seat is driven by the runner's random AI when the
//! page says `play N auto`, exactly as on the device. Used by
//! `scripts/bridge-test.sh` and `tools/bridge/relay_test.mjs`.

use std::io::{BufRead, Write};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use mega_blastoise_core::device_runner::{DeviceEvent, DeviceInputChannel, DevicePort, DeviceRunner};
use mega_blastoise_core::device_session::DeviceSession;
use mega_blastoise_core::OledController;

static INPUT: DeviceInputChannel = DeviceInputChannel::new();
static LINK_IN: Channel<CriticalSectionRawMutex, String, 256> = Channel::new();
static START: OnceLock<Instant> = OnceLock::new();

struct HostPort;

impl DevicePort for HostPort {
    fn now_ms(&self) -> u64 {
        START.get_or_init(Instant::now).elapsed().as_millis() as u64
    }

    async fn sleep_ms(&self, ms: u32) {
        // A waker-driven sleep: the thread wakes the task when the time is up.
        struct Timer {
            state: std::sync::Arc<std::sync::Mutex<(bool, Option<std::task::Waker>)>>,
        }
        impl std::future::Future for Timer {
            type Output = ();
            fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<()> {
                let mut s = self.state.lock().unwrap();
                if s.0 {
                    std::task::Poll::Ready(())
                } else {
                    s.1 = Some(cx.waker().clone());
                    std::task::Poll::Pending
                }
            }
        }
        let state = std::sync::Arc::new(std::sync::Mutex::new((false, None::<std::task::Waker>)));
        let t = state.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(ms as u64));
            let mut s = t.lock().unwrap();
            s.0 = true;
            if let Some(w) = s.1.take() {
                w.wake();
            }
        });
        Timer { state }.await
    }

    fn line_out(&self, line: &str) {
        eprintln!("{line}");
    }

    fn present(&self, _: &DeviceSession, _: &OledController, _: bool) {}

    fn seed(&self) -> u64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64)
    }

    fn link_send(&self, line: &str) {
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "{line}");
        let _ = out.flush();
    }

    fn link_try_recv(&self) -> Option<String> {
        LINK_IN.try_receive().ok()
    }

    async fn link_recv(&self) -> String {
        LINK_IN.receive().await
    }
}

fn main() {
    START.get_or_init(Instant::now);
    let _ = INPUT.try_send(DeviceEvent::SetAnim(false));
    std::thread::spawn(|| {
        for line in std::io::stdin().lock().lines() {
            let Ok(line) = line else { break };
            pollster::block_on(LINK_IN.send(line));
        }
        std::process::exit(0);
    });
    std::thread::spawn(|| loop {
        std::thread::sleep(Duration::from_millis(mega_blastoise_core::BOB_TICK_MS as u64));
        let _ = INPUT.try_send(DeviceEvent::Pulse);
    });
    let port = HostPort;
    let runner = DeviceRunner::new(&port, &INPUT);
    pollster::block_on(runner.run());
}
