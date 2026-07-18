//! Lobby: AI demo battle → player ready-up → countdown → real battle.
//!
//! The lobby logic lives entirely in `run_lobby_inner`, which knows nothing
//! about USB or GPIO — it only sees `LobbyEvent` values from a `LobbyInput`.
//! Hardware-specific glue (USB commands, button matrix) lives in the impl
//! structs below and is kept out of the game logic.

extern crate alloc;

use gen1_battle::{MonData, TeamData};
use embassy_futures::select::{select, select3, Either, Either3};
use embassy_time::{Duration, Instant, Timer};
use mega_blastoise_core::{
    battle_options_with_seed, demo_engine_opts, draw_two_randbat_teams, parse_lobby_cmd,
    parse_team_spec, ActivePrompt, BoardEventQueue, CollectEffect, ControlMode, FlashDataStore,
    InputBus, InputSource, LobbyCmd, PadEvent, PlayerChoice, RandomAi, ReadySequence,
    COLLECT_TICK_MS, LOBBY_DEMO_DELAY_MS, TEAM_SEED_SALT,
};

use crate::battle_effects::BattleEffects;
use crate::pico_battle_input::{apply_oled_effects, LobbyPress, PadScan, PicoBattleInput};

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
    P1Ai, P2Ai, VsAi,
    Demo, Stop,
    /// A `:team pN …` upload. Stored and used for the next real battle in
    /// place of a random team. Does not affect ready state.
    TeamUpload { player: u8, team: alloc::vec::Vec<MonData> },
}

/// Result of a lobby session.
pub struct LobbyResult {
    /// Which players are AI-controlled.
    pub ai_players: [bool; 2],
    /// Control scheme each player picked during the ready sequence.
    pub modes: [ControlMode; 2],
    /// HIDDEN: 6v6 teams (4-corner chord during the ready sequence).
    pub six_v_six: bool,
    /// Uploaded team for p1, if `:team p1 …` was issued (else random).
    pub team_p1: Option<alloc::vec::Vec<MonData>>,
    /// Uploaded team for p2, if `:team p2 …` was issued (else random).
    pub team_p2: Option<alloc::vec::Vec<MonData>>,
}

/// Why [`LobbyInput::drive_ready`] returned before the sequence completed.
pub enum SeqOutcome {
    /// Both players are ready (controls chosen); countdown can run.
    Done,
    /// `:demo` — restart the demo loop.
    Demo,
    /// `:s` / `:stop` — no-op during the ready sequence.
    Stop,
    /// `:team pN …` upload — stash and resume the sequence.
    TeamUpload { player: u8, team: alloc::vec::Vec<MonData> },
}

pub trait LobbyInput {
    async fn wait_event(&mut self) -> LobbyEvent;
    async fn write_line(&mut self, s: &str);
    /// Drive the shared [`ReadySequence`] (press → controls picker → READY)
    /// with this platform's IO until it completes or a lobby command
    /// interrupts it.
    async fn drive_ready(&mut self, seq: &mut ReadySequence) -> SeqOutcome;
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
        // Noise (unrecognised commands, stale USB bytes, malformed :team) is
        // swallowed here so callers never see it; `Stop` is exclusively a
        // deliberate `:s` / `:stop`.
        loop {
            return match select(self.buttons.wait_lobby_press(), self.usb.read_lobby_cmd()).await {
                Either::First(LobbyPress::P1)    => LobbyEvent::P1,
                Either::First(LobbyPress::P2)    => LobbyEvent::P2,
                // Long-press = "give me an AI opponent": the presser stays human.
                Either::First(LobbyPress::P1Long) => LobbyEvent::P2Ai,
                Either::First(LobbyPress::P2Long) => LobbyEvent::P1Ai,
                Either::Second(cmd) => match cmd {
                    LobbyUsbCmd::ReadyP1    => LobbyEvent::P1,
                    LobbyUsbCmd::ReadyP2    => LobbyEvent::P2,
                    LobbyUsbCmd::ReadyBoth  => LobbyEvent::BothReady,
                    LobbyUsbCmd::UnreadyP1  => LobbyEvent::UnreadyP1,
                    LobbyUsbCmd::UnreadyP2  => LobbyEvent::UnreadyP2,
                    LobbyUsbCmd::UnreadyBoth => LobbyEvent::UnreadyBoth,
                    LobbyUsbCmd::P1Ai       => LobbyEvent::P1Ai,
                    LobbyUsbCmd::P2Ai       => LobbyEvent::P2Ai,
                    LobbyUsbCmd::VsAi       => LobbyEvent::VsAi,
                    LobbyUsbCmd::Demo       => LobbyEvent::Demo,
                    LobbyUsbCmd::UploadTeam => match self.usb.take_pending_team() {
                        Some((player, team)) => LobbyEvent::TeamUpload { player, team },
                        None => continue, // malformed :team — ignore
                    },
                    LobbyUsbCmd::StopDemo => LobbyEvent::Stop,
                    LobbyUsbCmd::Unknown  => continue,
                },
            };
        }
    }

    async fn write_line(&mut self, s: &str) {
        self.usb.write_lobby_line(s).await;
    }

    async fn drive_ready(&mut self, seq: &mut ReadySequence) -> SeqOutcome {
        let mut fx: alloc::vec::Vec<CollectEffect> = alloc::vec::Vec::new();
        let mut scan = PadScan::default();
        let mut last_ready = seq.ready_flags();
        #[cfg(feature = "leds")]
        led_send(LedCmd::LobbyWaiting { p1_ready: last_ready[0], p2_ready: last_ready[1] });
        loop {
            match select3(
                self.usb.read_line(),
                self.buttons.next_pad_event(&mut scan),
                Timer::after_millis(COLLECT_TICK_MS),
            )
            .await
            {
                Either3::First(line) => {
                    let line_t = line.trim();
                    match parse_lobby_cmd(line_t) {
                        LobbyCmd::ReadyP1 => seq.set_ready_cmd(1, &mut fx),
                        LobbyCmd::ReadyP2 => seq.set_ready_cmd(2, &mut fx),
                        LobbyCmd::ReadyBoth => {
                            seq.set_ready_cmd(1, &mut fx);
                            seq.set_ready_cmd(2, &mut fx);
                        }
                        LobbyCmd::UnreadyP1 => seq.set_unready_cmd(1, &mut fx),
                        LobbyCmd::UnreadyP2 => seq.set_unready_cmd(2, &mut fx),
                        LobbyCmd::UnreadyBoth => {
                            seq.set_unready_cmd(1, &mut fx);
                            seq.set_unready_cmd(2, &mut fx);
                        }
                        // "pN is AI" == the OTHER player requested an AI foe.
                        LobbyCmd::P1Ai => seq.request_ai_opponent(2, &mut fx),
                        LobbyCmd::P2Ai => seq.request_ai_opponent(1, &mut fx),
                        LobbyCmd::VsAi => seq.ai_preset([true, true], &mut fx),
                        LobbyCmd::Demo => return SeqOutcome::Demo,
                        LobbyCmd::StopDemo => return SeqOutcome::Stop,
                        LobbyCmd::UploadTeam => match parse_team_spec(line_t) {
                            Some((player, team)) => {
                                return SeqOutcome::TeamUpload { player, team }
                            }
                            None => {
                                self.usb
                                    .write_lobby_line(
                                        "Bad :team syntax. Use: :team p1 species:move:move,species:...",
                                    )
                                    .await
                            }
                        },
                        LobbyCmd::Unknown => seq.typed_line(line_t, Instant::now().as_millis(), &mut fx),
                    }
                }
                Either3::Second(ev) => seq.pad_event(ev, Instant::now().as_millis(), &mut fx),
                Either3::Third(()) => {}
            }
            let done = seq.tick(Instant::now().as_millis(), &mut fx);
            for e in fx.drain(..) {
                match e {
                    CollectEffect::Oled(_cmd) => {
                        #[cfg(feature = "oled")]
                        oled_send(_cmd);
                    }
                    CollectEffect::Ok(m) | CollectEffect::Err(m) | CollectEffect::Dbg(m) => {
                        self.usb.write_lobby_line(&m).await
                    }
                    CollectEffect::Text(t) => self.usb.write_lobby_line(&t).await,
                }
            }
            let ready = seq.ready_flags();
            if ready != last_ready {
                last_ready = ready;
                self.usb.write_lobby_ready_status(ready[0], ready[1]).await;
                #[cfg(feature = "leds")]
                led_send(LedCmd::LobbyWaiting { p1_ready: ready[0], p2_ready: ready[1] });
            }
            if done {
                return SeqOutcome::Done;
            }
        }
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
            LobbyPress::P1Long => LobbyEvent::P2Ai,
            LobbyPress::P2Long => LobbyEvent::P1Ai,
        }
    }

    async fn write_line(&mut self, _s: &str) {}

    async fn drive_ready(&mut self, seq: &mut ReadySequence) -> SeqOutcome {
        let mut fx: alloc::vec::Vec<CollectEffect> = alloc::vec::Vec::new();
        let mut scan = PadScan::default();
        #[cfg(feature = "leds")]
        let mut last_ready = seq.ready_flags();
        loop {
            match select(self.buttons.next_pad_event(&mut scan), Timer::after_millis(COLLECT_TICK_MS))
                .await
            {
                Either::First(ev) => seq.pad_event(ev, Instant::now().as_millis(), &mut fx),
                Either::Second(()) => {}
            }
            let done = seq.tick(Instant::now().as_millis(), &mut fx);
            apply_oled_effects(&mut fx);
            #[cfg(feature = "leds")]
            {
                let ready = seq.ready_flags();
                if ready != last_ready {
                    last_ready = ready;
                    led_send(LedCmd::LobbyWaiting { p1_ready: ready[0], p2_ready: ready[1] });
                }
            }
            if done {
                return SeqOutcome::Done;
            }
        }
    }
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
            let ActivePrompt { player_id, request, player_data, .. } = bus.prompt.receive().await;
            #[cfg(feature = "trace")]
            defmt::info!("[trace] DemoAi: got prompt");
            // Cosmetic pacing so the demo is watchable. Skipped under `trace`
            // (used for fast hardware verification runs).
            #[cfg(not(feature = "trace"))]
            Timer::after_millis(400 + (self.0.next_u64() % 600)).await;
            #[cfg(feature = "trace")]
            let _ = self.0.next_u64(); // keep RNG stream identical
            let choice = self.0.make_choice(&request, player_data.as_ref());
            #[cfg(feature = "trace")]
            defmt::info!("[trace] DemoAi: sending choice: {}", choice.as_str());
            bus.choices.send(PlayerChoice { player_id, choice }).await;
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

    // Demo battles always run normal-mode screens (a previous battle may
    // have left a player's display controller in concealed mode).
    #[cfg(feature = "oled")]
    for player in [1u8, 2] {
        oled_send(OledCmd::SetControlMode { player, concealed: false });
    }

    // Demo battle drives no hardware LEDs — the lobby idle animation owns the
    // strips while we're here.
    let mut demo_effects = BattleEffects::new(None, false);
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

// ── Countdown ─────────────────────────────────────────────────────────────────

async fn do_countdown(input: &mut impl LobbyInput) {
    input.write_line("Both ready!").await;
    for i in (1u8..=3).rev() {
        #[cfg(feature = "buzzer")]
        buzz(BuzzerCmd::CountdownBeep);
        input.write_line(&alloc::format!("{}...", i)).await;
        Timer::after_millis(500).await;
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
) -> LobbyResult {
    let mut demo_seed = embassy_time::Instant::now().as_ticks() ^ 0xfeed_f00d_dead_beef;
    // Uploaded test teams persist across the whole lobby session until the
    // real battle starts.
    let mut uploaded_p1: Option<alloc::vec::Vec<MonData>> = None;
    let mut uploaded_p2: Option<alloc::vec::Vec<MonData>> = None;

    'demo: loop {
        #[cfg(feature = "leds")]
        led_send(LedCmd::LobbyIdle);
        // Reset both screens to the idle lobby state on every lobby entry,
        // like the web client's set_lobby_displays() — without this the win
        // screen (or boot placeholder) lingers through the demo countdown.
        #[cfg(feature = "oled")]
        oled_lobby_update(false, false, false, false);
        input.write_line("Demo — press any button or :ready / :ready ai to start").await;

        // Countdown to demo start; log at each 5-second step, bail early on input.
        // `Stop` (a deliberate `:s`) cancels the countdown: it breaks out as a
        // normal event, hits the `Stop => {}` arm below, and falls into the
        // waiting phase with nobody ready. Noise never reaches here —
        // `wait_event` swallows it.
        const STEP_MS: u64 = 5_000;
        let steps = (LOBBY_DEMO_DELAY_MS / STEP_MS) as u32;
        let mut remaining = steps;
        let pre_event = 'wait: loop {
            let secs = remaining as u64 * STEP_MS / 1000;
            input.write_line(&alloc::format!("Demo starting in {} seconds", secs)).await;
            let deadline = Instant::now() + Duration::from_millis(STEP_MS);
            loop {
                match select(Timer::at(deadline), input.wait_event()).await {
                    Either::Second(LobbyEvent::TeamUpload { player, team }) => {
                        if player == 0 { uploaded_p1 = Some(team); }
                        else { uploaded_p2 = Some(team); }
                    }
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
                // Idle delay elapsed — race the demo battle against the next
                // input event. Noise (e.g. a Linux host's ModemManager probing
                // the CDC port on every enumeration) is swallowed inside
                // `wait_event`, so anything that resolves the select here is a
                // deliberate action — including `:s`, which cancels the demo
                // and drops to the waiting phase via the `Stop => {}` arm.
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

        // ── Ready sequence: press → controls picker → READY, per player ──────
        // The interrupting event seeds it; drive_ready runs it to completion.
        let mut seq_fx: alloc::vec::Vec<CollectEffect> = alloc::vec::Vec::new();
        let mut seq = ReadySequence::new(&mut seq_fx);
        match event {
            LobbyEvent::P1 => seq.pad_event(PadEvent::TapMove { player: 1, slot: 0 }, Instant::now().as_millis(), &mut seq_fx),
            LobbyEvent::P2 => seq.pad_event(PadEvent::TapMove { player: 2, slot: 0 }, Instant::now().as_millis(), &mut seq_fx),
            LobbyEvent::BothReady => {
                seq.set_ready_cmd(1, &mut seq_fx);
                seq.set_ready_cmd(2, &mut seq_fx);
            }
            // "pN is AI" == the other player asked for an AI opponent.
            LobbyEvent::P1Ai => seq.request_ai_opponent(2, &mut seq_fx),
            LobbyEvent::P2Ai => seq.request_ai_opponent(1, &mut seq_fx),
            LobbyEvent::VsAi => seq.ai_preset([true, true], &mut seq_fx),
            LobbyEvent::Demo => { continue 'demo; }
            LobbyEvent::TeamUpload { player, team } => {
                if player == 0 { uploaded_p1 = Some(team); }
                else { uploaded_p2 = Some(team); }
                continue 'demo;
            }
            LobbyEvent::UnreadyP1 | LobbyEvent::UnreadyP2 |
            LobbyEvent::UnreadyBoth | LobbyEvent::Stop => {}
        }
        apply_oled_effects(&mut seq_fx);

        loop {
            match input.drive_ready(&mut seq).await {
                SeqOutcome::Done => break,
                SeqOutcome::Demo => continue 'demo,
                SeqOutcome::Stop => {}
                SeqOutcome::TeamUpload { player, team } => {
                    if player == 0 { uploaded_p1 = Some(team); }
                    else { uploaded_p2 = Some(team); }
                }
            }
        }
        let six_v_six = seq.six_v_six();
        let (ai_players, modes) = seq.take();
        do_countdown(input).await;
        return LobbyResult {
            ai_players,
            modes,
            six_v_six,
            team_p1: uploaded_p1,
            team_p2: uploaded_p2,
        };
    }
}

// ── Public entrypoints ────────────────────────────────────────────────────────

#[cfg(feature = "usb")]
pub async fn run_lobby(
    buttons: &mut PicoBattleInput<'_>,
    usb: &mut UsbBattleInput<'_>,
    data: &FlashDataStore,
    queue: &mut BoardEventQueue,
) -> LobbyResult {
    let mut input = UsbButtonLobbyInput::new(usb, buttons);
    run_lobby_inner(&mut input, data, queue).await
}

#[cfg(not(feature = "usb"))]
pub async fn run_lobby(
    buttons: &mut PicoBattleInput<'_>,
    data: &FlashDataStore,
    queue: &mut BoardEventQueue,
) -> LobbyResult {
    let mut input = ButtonOnlyLobbyInput::new(buttons);
    run_lobby_inner(&mut input, data, queue).await
}
