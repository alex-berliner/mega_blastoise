//! Battery sense for board_v2: VBAT through the R1/R2 halving divider into
//! GP28 = ADC2 (BUILD.md §5), sampled slowly in the background.
//!
//! The task keeps a 16-sample average in [`VBAT_MV`]; the port turns it
//! into the shared `display_color::Battery` (curve lives in core so the
//! web sim and the glass agree). Anything under [`ABSENT_MV`] reads as no
//! cell — the tap floats near ground when the slide switch is off or the
//! battery is unplugged (measured ~0.13 V on this board).

use core::sync::atomic::{AtomicU32, Ordering};

use embassy_rp::adc::{Adc, Channel, Config as AdcConfig, InterruptHandler as AdcInterruptHandler};
use embassy_rp::gpio::Pull;
use embassy_rp::peripherals::{ADC, PIN_28};
use embassy_rp::Peri;
use embassy_time::{Duration, Timer};
use mega_blastoise_core::display_color::{lipo_percent, Battery};

embassy_rp::bind_interrupts!(struct AdcIrqs {
    ADC_IRQ_FIFO => AdcInterruptHandler;
});

/// Latest battery-side millivolts (pin reading doubled for the divider).
pub static VBAT_MV: AtomicU32 = AtomicU32::new(0);

/// Below this the tap is floating, not a discharged cell.
const ABSENT_MV: u32 = 2500;

/// The port's read: what the battery indicator should show right now.
pub fn battery() -> Battery {
    let mv = VBAT_MV.load(Ordering::Relaxed);
    if mv < ABSENT_MV {
        Battery::Absent
    } else {
        Battery::Level(lipo_percent(mv))
    }
}

#[embassy_executor::task]
pub async fn task(adc: Peri<'static, ADC>, pin: Peri<'static, PIN_28>) -> ! {
    let mut adc = Adc::new(adc, AdcIrqs, AdcConfig::default());
    let mut ch = Channel::new_pin(pin, Pull::None);
    defmt::debug!("Battery sense ready: GP28/ADC2, divider x2");
    loop {
        let mut sum: u32 = 0;
        for _ in 0..16 {
            sum += adc.read(&mut ch).await.unwrap_or(0) as u32;
            Timer::after(Duration::from_millis(2)).await;
        }
        let pin_mv = (sum / 16) * 3300 / 4095;
        VBAT_MV.store(pin_mv * 2, Ordering::Relaxed);
        Timer::after(Duration::from_secs(2)).await;
    }
}
