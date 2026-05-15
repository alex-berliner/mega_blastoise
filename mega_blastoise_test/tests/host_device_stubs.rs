//! Tests for host-side device interface stubs:
//! `HostHpBarState`, `HostHwObject`, `HostBattleEffects`, `HostBattleController`.
//!
//! These cover the firmware abstractions that the interactive binary couldn't reach.

use gen1_battle::TeamData;
use embassy_futures::select::{select, Either};
use mega_blastoise_core::{
    demo_battle_options, demo_engine_opts, demo_team_blue, demo_team_red, run_battle,
    ActivePrompt, BoardEffects, BoardEvent, BoardEventQueue, FlashDataStore, InputBus, InputSource,
};
use mega_blastoise_test::{
    host_battle_controller::HostBattleController,
    host_battle_effects::HostBattleEffects,
    host_hp_bar::HostHpBarState,
    host_hw_object::HostHwObject,
};

// ── HostHpBarState ────────────────────────────────────────────────────────────

#[test]
fn hp_bar_parses_slash_format() {
    let hp = HostHpBarState::parse("80/100").unwrap();
    assert_eq!(hp.current, 80);
    assert_eq!(hp.max, 100);
    assert_eq!(hp.pct(), 80);
}

#[test]
fn hp_bar_parses_bare_value() {
    let hp = HostHpBarState::parse("42").unwrap();
    assert_eq!(hp.current, 42);
    assert_eq!(hp.max, 42);
    assert_eq!(hp.pct(), 100);
}

#[test]
fn hp_bar_parse_rejects_garbage() {
    assert!(HostHpBarState::parse("nope").is_none());
    assert!(HostHpBarState::parse("").is_none());
}

#[test]
fn hp_bar_zero_max_gives_zero_pct() {
    assert_eq!(HostHpBarState::ZERO.pct(), 0);
}

// ── HostHwObject ──────────────────────────────────────────────────────────────

#[test]
fn hw_object_tracks_state() {
    let mut obj: HostHwObject<HostHpBarState> =
        HostHwObject::new("test", HostHpBarState::ZERO, None);
    let hp = HostHpBarState::parse("60/100").unwrap();
    obj.update(hp);
    assert_eq!(obj.state().current, 60);
}

// ── HostBattleEffects ─────────────────────────────────────────────────────────

#[test]
fn effects_tracks_p1_hp_on_damage() {
    let mut effects = HostBattleEffects::new(None);
    pollster::block_on(effects.on_event(BoardEvent::Damage {
        mon: "Charizard,p1,0".to_string(),
        health: "80/100".to_string(),
    }));
    assert_eq!(effects.p1_hp().current, 80);
    assert_eq!(effects.p1_hp().max, 100);
}

#[test]
fn effects_tracks_p2_hp_on_damage() {
    let mut effects = HostBattleEffects::new(None);
    pollster::block_on(effects.on_event(BoardEvent::Damage {
        mon: "Blastoise,p2,0".to_string(),
        health: "50/150".to_string(),
    }));
    assert_eq!(effects.p2_hp().current, 50);
    assert_eq!(effects.p2_hp().max, 150);
}

#[test]
fn effects_sends_narration_to_log_channel() {
    let bus = InputBus::new();
    let mut effects = HostBattleEffects::new(Some(&bus));
    pollster::block_on(effects.on_event(BoardEvent::Faint {
        mon: "Charizard,p1,0".to_string(),
        team_slot: None,
    }));
    // Should have enqueued a description string, not printed directly.
    let msg = bus.log.try_receive().expect("log channel should have a message");
    assert!(msg.contains("faint") || !msg.is_empty());
}

#[test]
fn effects_suppresses_split_and_prompt_events() {
    let bus = InputBus::new();
    let mut effects = HostBattleEffects::new(Some(&bus));
    pollster::block_on(effects.on_event(BoardEvent::Split { side: "p1".to_string() }));
    assert!(bus.log.try_receive().is_err(), "Split should not reach bus.log");
}

// ── HostBattleController — full automated battle ──────────────────────────────

fn make_battle(data: &FlashDataStore) -> gen1_battle::PublicCoreBattle<'_> {
    let mut battle =
        gen1_battle::PublicCoreBattle::new(demo_battle_options(), data, demo_engine_opts())
            .expect("battle init");
    battle
        .update_team("p1", TeamData { members: demo_team_red(), ..Default::default() })
        .expect("p1 team");
    battle
        .update_team("p2", TeamData { members: demo_team_blue(), ..Default::default() })
        .expect("p2 team");
    battle.start().expect("battle start");
    battle
}

#[test]
fn battle_completes_with_prefed_button_moves() {
    let data = FlashDataStore::new();
    let mut battle = make_battle(&data);
    let bus = InputBus::new();

    let mut controller = HostBattleController::new();
    // Pre-feed enough move presses for both players for a full 4v4 singles battle.
    // Seed is fixed (12345) so the battle always follows the same path.
    // Slot 1 = Earthquake or Ice Beam for every mon — damaging, so the battle ends quickly.
    // Switch requests fall back to auto-pick (first available bench) when the queue is empty.
    for _ in 0..40 {
        controller.buttons_mut().queue_move(1);
    }

    let mut effects = HostBattleEffects::new(None);
    let mut queue = BoardEventQueue::new();

    pollster::block_on(run_battle(
        &mut battle,
        &data,
        &bus,
        controller.run(&bus),
        &mut queue,
        &mut effects,
        |_| {},
    ));

    assert!(battle.ended(), "battle should have ended");
}

#[test]
fn battle_completes_with_log_channel_active() {
    // Same as above but routes events through bus.log, exercising the full
    // HostBattleEffects → bus.log → HostBattleController drain path.
    let data = FlashDataStore::new();
    let mut battle = make_battle(&data);
    let bus = InputBus::new();

    let mut controller = HostBattleController::new();
    // Slot 1 = Earthquake or Ice Beam for every mon — damaging, so the battle ends quickly.
    // Switch requests fall back to auto-pick (first available bench) when the queue is empty.
    for _ in 0..40 {
        controller.buttons_mut().queue_move(1);
    }

    let mut effects = HostBattleEffects::new(Some(&bus));
    let mut queue = BoardEventQueue::new();

    pollster::block_on(run_battle(
        &mut battle,
        &data,
        &bus,
        controller.run(&bus),
        &mut queue,
        &mut effects,
        |_| {},
    ));

    assert!(battle.ended());
}

// ── Button-press unit test ────────────────────────────────────────────────────

/// Verifies that a pre-fed button press resolves a move choice immediately —
/// no stdin, no blocking — and produces the correct choice string.
///
/// Approach: run the controller alongside a "driver" future that injects a real
/// `ActivePrompt` (built from the first request the battle engine generates) then
/// collects the choice from `bus.choices`.  `select` drops the controller as soon
/// as the driver completes, so the test doesn't hang on the controller's infinite loop.
#[test]
fn button_press_sends_move_choice_without_stdin() {
    let data = FlashDataStore::new();
    let mut battle = make_battle(&data);

    // Grab the first real Turn request so the prompt has a genuine move list.
    let (player_id, request) = battle
        .active_requests()
        .next()
        .map(|(pid, req)| (pid.to_string(), req.clone()))
        .expect("battle should have an active request after start");
    let player_data = battle.player_data(&player_id).ok();

    let bus = InputBus::new();
    let mut controller = HostBattleController::new();
    // Queue slot 1 (0-based) — the second move in the list (Earthquake / Ice Beam).
    controller.buttons_mut().queue_move(1);

    let choice = pollster::block_on(async {
        let driver = async {
            bus.prompt
                .send(ActivePrompt { player_id, request, player_data, batch_total: 1 })
                .await;
            bus.choices.receive().await
        };
        match select(controller.run(&bus), driver).await {
            Either::First(()) => panic!("controller exited before choice was produced"),
            Either::Second(choice) => choice,
        }
    });

    // format_move_choice(1) == "move 1"
    assert_eq!(choice, "move 1");
}
