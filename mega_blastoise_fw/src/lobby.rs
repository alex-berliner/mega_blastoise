//! Lobby: AI demo battle → player ready-up → countdown → real battle.
//!
//! Flow: Demo (AI vs AI, interruptible) → Waiting (per-player ready toggle)
//!       → Countdown (3-2-1) → returns, caller starts real battle.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use battler::{PlayerBattleData, Request, TeamData};
use embassy_futures::select::{select, Either};
use embassy_time::Timer;
use mega_blastoise_core::{
    battle_options_with_seed, demo_engine_opts, draw_randbat_team,
    format_move_choice, format_switch_choice, join_choice_parts,
    ActivePrompt, BoardEventQueue, FlashDataStore, InputBus, InputSource,
};

use crate::battle_effects::BattleEffects;
use crate::pico_battle_input::{LobbyPress, PicoBattleInput};

#[cfg(feature = "leds")]
use crate::subsystems::led::{send as led_send, LedCmd};

#[cfg(feature = "buzzer")]
use crate::subsystems::buzzer::{buzz, BuzzerCmd};

#[cfg(feature = "usb")]
use crate::usb_input::{LobbyUsbCmd, UsbBattleInput};

// ── Demo AI ───────────────────────────────────────────────────────────────────

struct DemoAi {
    rng: u64,
}

impl DemoAi {
    fn new(seed: u64) -> Self {
        Self { rng: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.rng = self.rng.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.rng;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }

    fn make_choice(&mut self, request: &Request, player_data: Option<&PlayerBattleData>) -> String {
        match request {
            Request::Turn(turn) => {
                let mut parts = Vec::new();
                for mon_req in &turn.active {
                    let n = mon_req.moves.len().min(4);
                    if n == 0 {
                        parts.push(String::from("pass"));
                        continue;
                    }
                    let slot = (self.next_u64() as usize) % n;
                    parts.push(format_move_choice(slot));
                }
                join_choice_parts(&parts)
            }
            Request::Switch(sw) => {
                // Pick a random valid switch target: not active, not fainted.
                let valid: Vec<usize> = match player_data {
                    Some(pd) => pd.mons.iter().enumerate()
                        .filter(|(_, m)| !m.active && m.hp > 0)
                        .map(|(i, _)| i)
                        .collect(),
                    None => (1..6).collect(),
                };
                let mut parts = Vec::new();
                for _ in 0..sw.needs_switch.len() {
                    let idx = if valid.is_empty() {
                        0
                    } else {
                        valid[(self.next_u64() as usize) % valid.len()]
                    };
                    parts.push(format_switch_choice(idx));
                }
                join_choice_parts(&parts)
            }
            Request::TeamPreview(_) => String::from("random"),
            Request::LearnMove(_) => String::from("pass"),
        }
    }
}

impl InputSource for DemoAi {
    async fn run(&mut self, bus: &InputBus) {
        loop {
            let ActivePrompt { request, player_data, .. } = bus.prompt.receive().await;
            Timer::after_millis(400 + (self.next_u64() % 600)).await;
            let choice = self.make_choice(&request, player_data.as_ref());
            bus.choices.send(choice).await;
        }
    }
}

// ── Demo battle ───────────────────────────────────────────────────────────────

/// Run one AI-vs-AI demo battle. Returns `true` if interrupted by a button press,
/// `false` if the battle ended naturally (one side won).
async fn run_demo_battle(
    buttons: &mut PicoBattleInput<'_>,
    data: &FlashDataStore,
    queue: &mut BoardEventQueue,
    seed: u64,
) -> bool {
    use mega_blastoise_core::run_battle;

    let mut battle = match battler::PublicCoreBattle::new(
        battle_options_with_seed(seed),
        data,
        demo_engine_opts(),
    ) {
        Ok(b) => b,
        Err(_) => return false,
    };

    let _ = battle.update_team("p1", TeamData {
        members: draw_randbat_team(seed, 3),
        ..Default::default()
    });
    let _ = battle.update_team("p2", TeamData {
        members: draw_randbat_team(seed.wrapping_add(0x9e3779b97f4a7c15), 3),
        ..Default::default()
    });
    if battle.start().is_err() {
        return false;
    }

    let mut demo_effects = BattleEffects::new(None);
    let demo_bus = InputBus::new();
    let mut ai = DemoAi::new(seed ^ 0xdead_beef_cafe_babe);

    let battle_fut = run_battle(
        &mut battle,
        data,
        &demo_bus,
        ai.run(&demo_bus),
        queue,
        &mut demo_effects,
        |_| {},
    );

    match select(battle_fut, buttons.wait_any_lobby_press()).await {
        Either::First(_) => false,
        Either::Second(_) => true,
    }
}

// ── Ready state ───────────────────────────────────────────────────────────────

#[derive(Default)]
struct ReadyState {
    p1: bool,
    p2: bool,
}

impl ReadyState {
    fn both(&self) -> bool {
        self.p1 && self.p2
    }
}

// ── Countdown ─────────────────────────────────────────────────────────────────

#[cfg_attr(feature = "usb", allow(dead_code))]
async fn run_countdown() {
    #[cfg(feature = "leds")]
    led_send(LedCmd::LobbyCountdown);

    for i in (1u8..=3).rev() {
        #[cfg(feature = "buzzer")]
        buzz(BuzzerCmd::CountdownBeep);
        let _ = i; // suppress unused warning when buzzer is off
        Timer::after_secs(1).await;
    }

    #[cfg(feature = "buzzer")]
    buzz(BuzzerCmd::Win);
}

// ── Lobby entrypoints ─────────────────────────────────────────────────────────

/// Run the lobby (USB + button variant). Returns when countdown completes.
#[cfg(feature = "usb")]
pub async fn run_lobby(
    buttons: &mut PicoBattleInput<'_>,
    usb: &mut UsbBattleInput<'_>,
    data: &FlashDataStore,
    queue: &mut BoardEventQueue,
) {
    let mut demo_seed = embassy_time::Instant::now().as_ticks() ^ 0xfeed_f00d_dead_beef;

    'demo: loop {
        #[cfg(feature = "leds")]
        led_send(LedCmd::LobbyIdle);
        usb.write_lobby_line("Demo — press any button or type :ready to start").await;

        // Race the demo battle against USB input so :ready works during demo.
        let mut ready = ReadyState::default();
        match select(
            run_demo_battle(buttons, data, queue, demo_seed),
            usb.read_lobby_cmd(),
        ).await {
            Either::First(false) => {
                // Demo ended naturally — brief pause then loop.
                demo_seed = demo_seed.wrapping_add(0x9e3779b97f4a7c15);
                Timer::after_secs(3).await;
                continue 'demo;
            }
            Either::First(true) => {
                // Button press interrupted demo — enter waiting phase.
            }
            Either::Second(LobbyUsbCmd::ReadyBoth) => {
                ready.p1 = true;
                ready.p2 = true;
            }
            Either::Second(LobbyUsbCmd::ReadyP1) => { ready.p1 = true; }
            Either::Second(LobbyUsbCmd::ReadyP2) => { ready.p2 = true; }
            Either::Second(LobbyUsbCmd::ReadyAi) => {
                // Explicitly requested AI vs AI — restart demo loop immediately.
                demo_seed = demo_seed.wrapping_add(0x9e3779b97f4a7c15);
                continue 'demo;
            }
            Either::Second(LobbyUsbCmd::StopDemo) | Either::Second(LobbyUsbCmd::Unknown) => {
                // :s / :stop or unrecognised input — interrupt demo, enter waiting phase.
            }
        }
        demo_seed = demo_seed.wrapping_add(0x9e3779b97f4a7c15);

        // ── Waiting phase ─────────────────────────────────────────────────────
        loop {
            if ready.both() {
                do_countdown(usb).await;
                return;
            }

            #[cfg(feature = "leds")]
            led_send(LedCmd::LobbyWaiting { p1_ready: ready.p1, p2_ready: ready.p2 });
            usb.write_lobby_ready_status(ready.p1, ready.p2).await;

            loop {
                match select(buttons.wait_lobby_press(), usb.read_lobby_cmd()).await {
                    Either::First(LobbyPress::P1) => { ready.p1 = !ready.p1; break; }
                    Either::First(LobbyPress::P2) => { ready.p2 = !ready.p2; break; }
                    Either::Second(LobbyUsbCmd::ReadyP1) => { ready.p1 = !ready.p1; break; }
                    Either::Second(LobbyUsbCmd::ReadyP2) => { ready.p2 = !ready.p2; break; }
                    Either::Second(LobbyUsbCmd::ReadyBoth) => { ready.p1 = true; ready.p2 = true; break; }
                    Either::Second(LobbyUsbCmd::ReadyAi) => {
                        // Return to demo mode from the waiting phase.
                        demo_seed = demo_seed.wrapping_add(0x9e3779b97f4a7c15);
                        continue 'demo;
                    }
                    Either::Second(LobbyUsbCmd::StopDemo) | Either::Second(LobbyUsbCmd::Unknown) => {}
                }
            }
        }
    }
}

async fn do_countdown(usb: &mut UsbBattleInput<'_>) {
    usb.write_lobby_line("Both ready!").await;
    for i in (1u8..=3).rev() {
        #[cfg(feature = "buzzer")]
        buzz(BuzzerCmd::CountdownBeep);
        usb.write_lobby_line(&alloc::format!("{}...", i)).await;
        Timer::after_secs(1).await;
    }
    #[cfg(feature = "leds")]
    led_send(LedCmd::LobbyCountdown);
    #[cfg(feature = "buzzer")]
    buzz(BuzzerCmd::Win);
    usb.write_lobby_line("GO!").await;
}

/// Run the lobby (button-only variant). Returns when countdown completes.
#[cfg(not(feature = "usb"))]
pub async fn run_lobby(
    buttons: &mut PicoBattleInput<'_>,
    data: &FlashDataStore,
    queue: &mut BoardEventQueue,
) {
    let mut demo_seed = embassy_time::Instant::now().as_ticks() ^ 0xfeed_f00d_dead_beef;

    loop {
        #[cfg(feature = "leds")]
        led_send(LedCmd::LobbyIdle);

        let interrupted = run_demo_battle(buttons, data, queue, demo_seed).await;
        demo_seed = demo_seed.wrapping_add(0x9e3779b97f4a7c15);

        if !interrupted {
            Timer::after_secs(3).await;
            continue;
        }

        let mut ready = ReadyState::default();
        loop {
            #[cfg(feature = "leds")]
            led_send(LedCmd::LobbyWaiting { p1_ready: ready.p1, p2_ready: ready.p2 });

            match buttons.wait_lobby_press().await {
                LobbyPress::P1 => ready.p1 = !ready.p1,
                LobbyPress::P2 => ready.p2 = !ready.p2,
            }

            if ready.both() {
                run_countdown().await;
                return;
            }
        }
    }
}
