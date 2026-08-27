//! Stripboard v2 buttons: 16 tactiles, one GPIO each, no matrix.
//!
//! Pin map is BUILD.md §10. Internal pull-ups, LOW = pressed, 20 ms
//! debounce (4 samples at 5 ms — the cadence the sw_test bring-up tester
//! validated on this exact board).
//!
//! Presses become `(player, device_session::Button)` events on [`EVENTS`].
//! Semantics beyond the debounce live in core's `DeviceSession`, not here:
//! this task only turns contact bounce into clean presses, plus the one
//! time-domain job the session cannot do — telling an A tap (release before
//! 500 ms) from an A hold (500 ms reached, fires `AHold` while still down),
//! mirroring the web build's long-press timer.
//!
//! MENU maps to `Button::B` for now: the session has no Menu button yet and
//! B is the session's "back / reopen the picker" key. TODO(menu): give the
//! session a real Menu press.

use core::sync::atomic::{AtomicU8, Ordering};

use embassy_rp::gpio::Input;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Instant, Timer};
use mega_blastoise_core::cursor_nav::Dir;
use mega_blastoise_core::device_session::Button;

pub static EVENTS: Channel<CriticalSectionRawMutex, (u8, Button), 8> = Channel::new();

/// Live held-button bitmask per seat, bit order matching
/// `device_view::draw_input_debug`: 0=Up 1=Down 2=Left 3=Right 4=A 5=B
/// 6=Info 7=Menu. Written here on every debounced edge, read by the debug
/// overlay.
pub static MASKS: [AtomicU8; 2] = [AtomicU8::new(0), AtomicU8::new(0)];

/// Pulsed on every debounced edge — presses AND releases — so the render
/// loop can repaint the debug overlay even when an edge produces no
/// session event (releases mostly don't).
pub static DIRTY: Signal<CriticalSectionRawMutex, ()> = Signal::new();

pub fn masks() -> [u8; 2] {
    [MASKS[0].load(Ordering::Relaxed), MASKS[1].load(Ordering::Relaxed)]
}

// 2 x 5 ms poll = 10 ms debounce. These tactiles settle in ~1-2 ms; 20 ms
// was bring-up caution and read as input lag once the display sped up.
const STABLE: u8 = 2;
const HOLD_MS: u64 = 500; // the game's long-press threshold

/// What a physical key means. `A` is separate because it alone has
/// tap-vs-hold semantics.
#[derive(Clone, Copy)]
pub enum Kind {
    A,
    Plain(Button),
}

pub struct Key {
    pin: Input<'static>,
    player: u8,
    kind: Kind,
    /// This key's bit in [`MASKS`] (the board's fixed key order).
    bit: u8,
    down: bool,
    run: u8,
    pressed_at: Instant,
    hold_sent: bool,
}

impl Key {
    pub fn new(pin: Input<'static>, player: u8, kind: Kind, bit: u8) -> Self {
        Self { pin, player, kind, bit, down: false, run: 0, pressed_at: Instant::MIN, hold_sent: false }
    }

    fn send(&self, b: Button) {
        // try_send: a full queue means the device task is many presses
        // behind; dropping the oldest intent is worse than dropping this one.
        let _ = EVENTS.try_send((self.player, b));
    }

    fn poll(&mut self) {
        let raw_down = self.pin.is_low();
        if raw_down == self.down {
            self.run = 0;
            // A held past the threshold fires AHold once, while still down.
            if self.down && !self.hold_sent && matches!(self.kind, Kind::A) {
                if Instant::now().duration_since(self.pressed_at).as_millis() >= HOLD_MS {
                    self.hold_sent = true;
                    self.send(Button::AHold);
                }
            }
            return;
        }
        self.run += 1;
        if self.run < STABLE {
            return;
        }
        self.run = 0;
        self.down = raw_down;
        let m = &MASKS[(self.player == 2) as usize];
        if raw_down {
            m.fetch_or(1 << self.bit, Ordering::Relaxed);
        } else {
            m.fetch_and(!(1 << self.bit), Ordering::Relaxed);
        }
        DIRTY.signal(());
        match self.kind {
            Kind::Plain(b) => {
                if raw_down {
                    self.send(b);
                }
            }
            Kind::A => {
                if raw_down {
                    self.pressed_at = Instant::now();
                    self.hold_sent = false;
                } else if !self.hold_sent {
                    // Released before the threshold: it was a tap.
                    self.send(Button::A);
                }
            }
        }
    }
}

/// The 16 keys, built by main from the v2 pin map.
#[embassy_executor::task]
pub async fn task(mut keys: [Key; 16]) -> ! {
    defmt::debug!("Buttons v2 ready: 16 direct GPIOs, pull-up, LOW=pressed");
    loop {
        for k in keys.iter_mut() {
            k.poll();
        }
        Timer::after(Duration::from_millis(5)).await;
    }
}

/// One seat's eight keys in the board's fixed order:
/// UP, DOWN, LEFT, RIGHT, A, B, INFO, MENU.
/// MENU maps to B: no session Menu button yet, and B reopens the picker.
pub fn seat_keys(player: u8, pins: [Input<'static>; 8]) -> [Key; 8] {
    let [up, down, left, right, a, b, info, menu] = pins;
    [
        Key::new(up, player, Kind::Plain(Button::Dpad(Dir::Up)), 0),
        Key::new(down, player, Kind::Plain(Button::Dpad(Dir::Down)), 1),
        Key::new(left, player, Kind::Plain(Button::Dpad(Dir::Left)), 2),
        Key::new(right, player, Kind::Plain(Button::Dpad(Dir::Right)), 3),
        Key::new(a, player, Kind::A, 4),
        Key::new(b, player, Kind::Plain(Button::B), 5),
        Key::new(info, player, Kind::Plain(Button::Info), 6),
        Key::new(menu, player, Kind::Plain(Button::B), 7),
    ]
}
