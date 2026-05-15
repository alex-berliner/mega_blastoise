//! Lobby: AI demo battle → player ready-up → countdown → real battle.
//!
//! The lobby logic lives entirely in `run_lobby_inner`, which knows nothing
//! about USB or GPIO — it only sees `LobbyEvent` values from a `LobbyInput`.
//! Hardware-specific glue (USB commands, button matrix) lives in the impl
//! structs below and is kept out of the game logic.

extern crate alloc;

use gen1_battle::TeamData;
use embassy_futures::select::{select, Either};
use embassy_time::{Duration, Instant, Timer};
use mega_blastoise_core::{
    battle_options_with_seed, demo_engine_opts, draw_two_randbat_teams,
    ActivePrompt, BoardEventQueue, FlashDataStore, InputBus, InputSource, RandomAi,
    LOBBY_DEMO_DELAY_MS, TEAM_SEED_SALT,
};

use crate::battle_effects::BattleEffects;
use crate::pico_battle_input::{LobbyPress, PicoBattleInput};

#[cfg(feature = "leds")]
use crate::subsystems::led::{send as led_send, LedCmd};

#[cfg(feature = "buzzer")]
use crate::subsystems::buzzer::{buzz, BuzzerCmd};

#[cfg(feature = "oled")]
use crate::subsystems::oled::{send as oled_send, OledCmd};

#[cfg(feature = "usb")]
use crate::usb_input::{LobbyUsbCmd, UsbBattleInput};

// ── LobbyInput abstraction ────────────────────────────────────────────────────

pub enum LobbyEvent {
    P1, P2,
    BothReady,
    UnreadyP1, UnreadyP2, UnreadyBoth,
    P1Ai, VsAi,
    Demo, Stop,
}

pub trait LobbyInput {
    async fn wait_event(&mut self) -> LobbyEvent;
    async fn write_line(&mut self, s: &str);
    async fn write_status(&mut self, p1_ready: bool, p2_ready: bool);
}

// ── USB + button implementation ───────────────────────────────────────────────

#[cfg(feature = "usb")]
pub struct UsbButtonLobbyInput<'a, 'u, 'b> {
    usb: &'a mut UsbBattleInput<'u>,
    buttons: &'a mut PicoBattleInput<'b>,
}

#[cfg(feature = "usb")]
impl<'a, 'u, 'b> UsbButtonLobbyInput<'a, 'u, 'b> {
    pub fn new(usb: &'a mut UsbBattleInput<'u>, buttons: &'a mut PicoBattleInput<'b>) -> Self {
        Self { usb, buttons }
    }
}

#[cfg(feature = "usb")]
impl LobbyInput for UsbButtonLobbyInput<'_, '_, '_> {
    async fn wait_event(&mut self) -> LobbyEvent {
        match select(self.buttons.wait_lobby_press(), self.usb.read_lobby_cmd()).await {
            Either::First(LobbyPress::P1)    => LobbyEvent::P1,
            Either::First(LobbyPress::P2)    => LobbyEvent::P2,
            Either::First(LobbyPress::P1Long) => LobbyEvent::VsAi,
            Either::First(LobbyPress::P2Long) => LobbyEvent::P1Ai,
            Either::Second(cmd) => match cmd {
                LobbyUsbCmd::ReadyP1    => LobbyEvent::P1,
                LobbyUsbCmd::ReadyP2    => LobbyEvent::P2,
                LobbyUsbCmd::ReadyBoth  => LobbyEvent::BothReady,
                LobbyUsbCmd::UnreadyP1  => LobbyEvent::UnreadyP1,
                LobbyUsbCmd::UnreadyP2  => LobbyEvent::UnreadyP2,
                LobbyUsbCmd::UnreadyBoth => LobbyEvent::UnreadyBoth,
                LobbyUsbCmd::P1Ai       => LobbyEvent::P1Ai,
                LobbyUsbCmd::VsAi       => LobbyEvent::VsAi,
                LobbyUsbCmd::Demo       => LobbyEvent::Demo,
                LobbyUsbCmd::StopDemo | LobbyUsbCmd::Unknown => LobbyEvent::Stop,
            },
        }
    }

    async fn write_line(&mut self, s: &str) {
        self.usb.write_lobby_line(s).await;
    }

    async fn write_status(&mut self, p1_ready: bool, p2_ready: bool) {
        self.usb.write_lobby_ready_status(p1_ready, p2_ready).await;
    }
}

// ── Button-only implementation ────────────────────────────────────────────────

#[cfg(not(feature = "usb"))]
pub struct ButtonOnlyLobbyInput<'a, 'b> {
    buttons: &'a mut PicoBattleInput<'b>,
}

#[cfg(not(feature = "usb"))]
impl<'a, 'b> ButtonOnlyLobbyInput<'a, 'b> {
    pub fn new(buttons: &'a mut PicoBattleInput<'b>) -> Self {
        Self { buttons }
    }
}

#[cfg(not(feature = "usb"))]
impl LobbyInput for ButtonOnlyLobbyInput<'_, '_> {
    async fn wait_event(&mut self) -> LobbyEvent {
        match self.buttons.wait_lobby_press().await {
            LobbyPress::P1     => LobbyEvent::P1,
            LobbyPress::P2     => LobbyEvent::P2,
            LobbyPress::P1Long => LobbyEvent::VsAi,
            LobbyPress::P2Long => LobbyEvent::P1Ai,
        }
    }

    async fn write_line(&mut self, _s: &str) {}
    async fn write_status(&mut self, _p1_ready: bool, _p2_ready: bool) {}
}

// ── Demo AI ───────────────────────────────────────────────────────────────────

struct DemoAi(RandomAi);

impl DemoAi {
    fn new(seed: u64) -> Self {
        Self(RandomAi::new(seed))
    }
}

impl InputSource for DemoAi {
    async fn run(&mut self, bus: &InputBus) {
        loop {
            #[cfg(feature = "trace")]
            defmt::info!("[trace] DemoAi: waiting for prompt");
            let ActivePrompt { request, player_data, .. } = bus.prompt.receive().await;
            #[cfg(feature = "trace")]
            defmt::info!("[trace] DemoAi: got prompt");
            Timer::after_millis(400 + (self.0.next_u64() % 600)).await;
            let choice = self.0.make_choice(&request, player_data.as_ref());
            #[cfg(feature = "trace")]
            defmt::info!("[trace] DemoAi: sending choice: {}", choice.as_str());
            bus.choices.send(choice).await;
            #[cfg(feature = "trace")]
            defmt::info!("[trace] DemoAi: choice sent");
        }
    }
}

// ── Demo battle ───────────────────────────────────────────────────────────────

/// Run one AI-vs-AI demo battle to completion. Caller races this against input
/// via `select` to implement the interrupt.
async fn run_demo_battle(data: &FlashDataStore, queue: &mut BoardEventQueue, seed: u64) {
    use mega_blastoise_core::run_battle;

    #[cfg(feature = "trace")]
    defmt::info!("[trace] run_demo_battle: start seed={}", seed);

    let mut battle = match gen1_battle::PublicCoreBattle::new(
        battle_options_with_seed(seed),
        data,
        demo_engine_opts(),
    ) {
        Ok(b) => b,
        Err(_) => {
            #[cfg(feature = "trace")]
            defmt::info!("[trace] run_demo_battle: battle init failed");
            return;
        }
    };

    let (team_red, team_blue) = draw_two_randbat_teams(seed, 3);
    let _ = battle.update_team("p1", TeamData { members: team_red,  ..Default::default() });
    let _ = battle.update_team("p2", TeamData { members: team_blue, ..Default::default() });
    if battle.start().is_err() {
        #[cfg(feature = "trace")]
        defmt::info!("[trace] run_demo_battle: battle start failed");
        return;
    }

    #[cfg(feature = "trace")]
    defmt::info!("[trace] run_demo_battle: battle started");

    let mut demo_effects = BattleEffects::new(None);
    let demo_bus = InputBus::new();
    let mut ai = DemoAi::new(seed ^ 0xdead_beef_cafe_babe);

    let _ = run_battle(
        &mut battle,
        data,
        &demo_bus,
        ai.run(&demo_bus),
        queue,
        &mut demo_effects,
        |_| {},
    ).await;

    #[cfg(feature = "trace")]
    defmt::info!("[trace] run_demo_battle: done");
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

async fn do_countdown(input: &mut impl LobbyInput) {
    input.write_line("Both ready!").await;
    for i in (1u8..=3).rev() {
        #[cfg(feature = "buzzer")]
        buzz(BuzzerCmd::CountdownBeep);
        input.write_line(&alloc::format!("{}...", i)).await;
        Timer::after_secs(1).await;
    }
    #[cfg(feature = "leds")]
    led_send(LedCmd::LobbyCountdown);
    #[cfg(feature = "buzzer")]
    buzz(BuzzerCmd::Win);
    input.write_line("GO!").await;
}

// ── OLED helper ───────────────────────────────────────────────────────────────

#[cfg(feature = "oled")]
fn oled_lobby_update(p1_ready: bool, p2_ready: bool, p1_ai: bool, p2_ai: bool) {
    oled_send(OledCmd::LobbyState { player: 1, ready: p1_ready, ai: p1_ai });
    oled_send(OledCmd::LobbyState { player: 2, ready: p2_ready, ai: p2_ai });
}

// ── Lobby logic ───────────────────────────────────────────────────────────────

async fn run_lobby_inner(
    input: &mut impl LobbyInput,
    data: &FlashDataStore,
    queue: &mut BoardEventQueue,
) -> [bool; 2] {
    let mut demo_seed = embassy_time::Instant::now().as_ticks() ^ 0xfeed_f00d_dead_beef;
    let mut p1_ai;
    let mut p2_ai;

    'demo: loop {
        p1_ai = false;
        p2_ai = false;
        #[cfg(feature = "leds")]
        led_send(LedCmd::LobbyIdle);
        input.write_line("Demo — press any button or :ready / :ready ai to start").await;

        let mut ready = ReadyState::default();

        // Countdown to demo start; log at each 5-second step, bail early on input.
        // Stop events (stale USB bytes, unknown commands) are ignored so they
        // don't interrupt the countdown.
        const STEP_MS: u64 = 5_000;
        let steps = (LOBBY_DEMO_DELAY_MS / STEP_MS) as u32;
        let mut remaining = steps;
        let pre_event = 'wait: loop {
            let secs = remaining as u64 * STEP_MS / 1000;
            input.write_line(&alloc::format!("Demo starting in {} seconds", secs)).await;
            let deadline = Instant::now() + Duration::from_millis(STEP_MS);
            loop {
                match select(Timer::at(deadline), input.wait_event()).await {
                    Either::Second(LobbyEvent::Stop) => {}
                    Either::Second(e) => break 'wait Some(e),
                    Either::First(_) => break,
                }
            }
            remaining -= 1;
            if remaining == 0 { break None; }
        };

        let event = match pre_event {
            Some(e) => {
                #[cfg(feature = "oled")]
                oled_lobby_update(false, false, false, false);
                e
            }
            None => {
                input.write_line("Demo starting!").await;
                // Idle delay elapsed — race demo battle against next input.
                match select(
                    run_demo_battle(data, queue, demo_seed),
                    input.wait_event(),
                ).await {
                    Either::First(_) => {
                        demo_seed = demo_seed.wrapping_add(TEAM_SEED_SALT);
                        #[cfg(feature = "oled")]
                        oled_lobby_update(false, false, false, false);
                        Timer::after_secs(3).await;
                        continue 'demo;
                    }
                    Either::Second(e) => {
                        #[cfg(feature = "oled")]
                        oled_lobby_update(false, false, false, false);
                        e
                    }
                }
            }
        };
        demo_seed = demo_seed.wrapping_add(TEAM_SEED_SALT);

        // Apply the interrupting event to initial ready state.
        match event {
            LobbyEvent::P1          => { ready.p1 = true; }
            LobbyEvent::P2          => { ready.p2 = true; }
            LobbyEvent::BothReady   => { ready.p1 = true; ready.p2 = true; }
            LobbyEvent::P1Ai        => { ready.p1 = true; ready.p2 = true; p1_ai = true; }
            LobbyEvent::VsAi        => { ready.p1 = true; ready.p2 = true; p2_ai = true; }
            LobbyEvent::Demo        => { continue 'demo; }
            LobbyEvent::UnreadyP1 | LobbyEvent::UnreadyP2 |
            LobbyEvent::UnreadyBoth | LobbyEvent::Stop => {}
        }
        #[cfg(feature = "oled")]
        oled_lobby_update(ready.p1, ready.p2, p1_ai, p2_ai);

        // ── Waiting phase ─────────────────────────────────────────────────────
        loop {
            if ready.both() {
                do_countdown(input).await;
                return [p1_ai, p2_ai];
            }

            #[cfg(feature = "leds")]
            led_send(LedCmd::LobbyWaiting { p1_ready: ready.p1, p2_ready: ready.p2 });
            input.write_status(ready.p1, ready.p2).await;

            match input.wait_event().await {
                LobbyEvent::P1 => {
                    ready.p1 = !ready.p1;
                    if !ready.p1 { p1_ai = false; p2_ai = false; }
                }
                LobbyEvent::P2 => {
                    ready.p2 = !ready.p2;
                    if !ready.p2 { p1_ai = false; p2_ai = false; }
                }
                LobbyEvent::BothReady   => { ready.p1 = true; ready.p2 = true; }
                LobbyEvent::UnreadyP1   => { ready.p1 = false; p1_ai = false; p2_ai = false; }
                LobbyEvent::UnreadyP2   => { ready.p2 = false; p1_ai = false; p2_ai = false; }
                LobbyEvent::UnreadyBoth => { ready.p1 = false; ready.p2 = false; p1_ai = false; p2_ai = false; }
                LobbyEvent::P1Ai        => { ready.p1 = true; ready.p2 = true; p1_ai = true; p2_ai = false; }
                LobbyEvent::VsAi        => { ready.p1 = true; ready.p2 = true; p1_ai = false; p2_ai = true; }
                LobbyEvent::Demo        => { continue 'demo; }
                LobbyEvent::Stop        => {}
            }
            #[cfg(feature = "oled")]
            oled_lobby_update(ready.p1, ready.p2, p1_ai, p2_ai);
        }
    }
}

// ── Public entrypoints ────────────────────────────────────────────────────────

#[cfg(feature = "usb")]
pub async fn run_lobby(
    buttons: &mut PicoBattleInput<'_>,
    usb: &mut UsbBattleInput<'_>,
    data: &FlashDataStore,
    queue: &mut BoardEventQueue,
) -> [bool; 2] {
    let mut input = UsbButtonLobbyInput::new(usb, buttons);
    run_lobby_inner(&mut input, data, queue).await
}

#[cfg(not(feature = "usb"))]
pub async fn run_lobby(
    buttons: &mut PicoBattleInput<'_>,
    data: &FlashDataStore,
    queue: &mut BoardEventQueue,
) -> [bool; 2] {
    let mut input = ButtonOnlyLobbyInput::new(buttons);
    run_lobby_inner(&mut input, data, queue).await
}
