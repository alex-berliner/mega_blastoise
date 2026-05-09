//! Piezo buzzer driver.  GP21 → PWM slice 2, channel B.
//!
//! Call [`buzz`] from any synchronous context (e.g. `BattleEffects::on_event`)
//! to enqueue a tone.  The [`task`] plays it asynchronously so the battle loop
//! is never blocked.

use embassy_rp::Peri;
use embassy_rp::peripherals::{PIN_21, PWM_SLICE2};
use embassy_rp::pwm::{Config as PwmConfig, Pwm};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::Timer;

pub enum BuzzerCmd {
    Hit,
    SuperEffective,
    Crit,
    Faint,
    Win,
    CountdownBeep,
}

static CMD: Channel<CriticalSectionRawMutex, BuzzerCmd, 4> = Channel::new();

/// Enqueue a buzzer event; warns via RTT if the queue is full.
pub fn buzz(cmd: BuzzerCmd) {
    if CMD.try_send(cmd).is_err() {
        defmt::warn!("buzzer: channel full, cmd dropped");
    }
}

#[embassy_executor::task]
pub async fn task(slice: Peri<'static, PWM_SLICE2>, pin: Peri<'static, PIN_21>) {
    let mut pwm = Pwm::new_output_b(slice, pin, PwmConfig::default());
    loop {
        match CMD.receive().await {
            BuzzerCmd::Hit => tone(&mut pwm, 880, 50).await,
            BuzzerCmd::SuperEffective => {
                tone(&mut pwm, 880, 30).await;
                Timer::after_millis(20).await;
                tone(&mut pwm, 1320, 70).await;
            }
            BuzzerCmd::Crit => {
                tone(&mut pwm, 1760, 30).await;
                Timer::after_millis(10).await;
                tone(&mut pwm, 1760, 60).await;
            }
            BuzzerCmd::Faint => {
                tone(&mut pwm, 440, 100).await;
                Timer::after_millis(20).await;
                tone(&mut pwm, 330, 100).await;
                Timer::after_millis(20).await;
                tone(&mut pwm, 220, 200).await;
            }
            BuzzerCmd::Win => {
                for &(freq, dur) in &[(523u32, 100u64), (659, 100), (784, 100), (1047, 300)] {
                    tone(&mut pwm, freq, dur).await;
                    Timer::after_millis(20).await;
                }
            }
            BuzzerCmd::CountdownBeep => tone(&mut pwm, 660, 80).await,
        }
    }
}

async fn tone(pwm: &mut Pwm<'_>, freq_hz: u32, ms: u64) {
    let sys_clk = 125_000_000u32;
    // Pick smallest integer divider so top = sys_clk / (freq * div) - 1 fits in u16.
    let mut div = 1u32;
    while sys_clk / (freq_hz * div) > 65535 {
        div += 1;
    }
    let top = (sys_clk / (freq_hz * div)).saturating_sub(1) as u16;

    let mut cfg = PwmConfig::default();
    cfg.top = top;
    cfg.compare_b = top / 2;
    cfg.divider = (div as u8).into();
    pwm.set_config(&cfg);

    Timer::after_millis(ms).await;

    let mut off = PwmConfig::default();
    off.top = top;
    off.compare_b = 0;
    pwm.set_config(&off);
}
