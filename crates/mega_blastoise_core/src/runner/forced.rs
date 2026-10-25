//! Forced moves on the PS engine, end to end: seeded Gen 1 battles over
//! [`LocalTransport`], every prompt put through the same path the device
//! takes (board events into an [`OledController`], the batch into a
//! [`ChoiceCollector`]), and every single-move request checked for what the
//! seat shows.
//!
//! PS offers one move without PP (`getMoveRequestData`): Recharge, a hard
//! lock (Thrash, Petal Dance, a charging move, Rage), a semi-lock (Gen 1's
//! Bide, a partial-trapping move), Struggle, or Gen 1's `Fight` for a
//! frozen, asleep or partially trapped mon. A hard lock is `trapped`; the
//! rest leave the switch open. So:
//! - forced (`trapped`, or no bench mon to come in): the seat is locked in
//!   at once, on the forced screen naming the move;
//! - switchable: the seat chooses, from a menu of that one move (named) or
//!   a switch, and PS takes the switch.

extern crate std;

use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::future::Future;
use std::collections::BTreeMap;

use super::ps::{run_battle, BattleSetup};
use crate::battle_effects::BoardEffects;
use crate::battle_input::{InputBus, PlayerChoice};
use crate::board_event::BoardEvent;
use crate::choice_collect::{ChoiceCollector, Effect, SlotOptions};
use crate::oled_ctl::{oled_cmds_for_event, OledController, Screen};
use crate::rng::SimpleRng;
use crate::transport::LocalTransport;

fn block_on<F: Future>(f: F) -> F::Output {
    use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
    fn noop(_: *const ()) {}
    fn clone(p: *const ()) -> RawWaker {
        RawWaker::new(p, &VTABLE)
    }
    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, noop, noop, noop);
    let waker = unsafe { Waker::from_raw(RawWaker::new(core::ptr::null(), &VTABLE)) };
    let mut cx = Context::from_waker(&waker);
    let mut f = core::pin::pin!(f);
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

/// The screens and the event log, shared by the board and the robot.
#[derive(Default)]
struct Shared {
    ctl: OledController,
    events: Vec<BoardEvent>,
    /// What each kind of single-move prompt was seen showing.
    seen: BTreeMap<String, u32>,
    /// A switch answered out of a single-move menu: (seat, event index).
    switched: Option<(String, usize)>,
}

impl Default for OledController {
    fn default() -> Self {
        OledController::new()
    }
}

struct Board(Rc<RefCell<Shared>>);

impl BoardEffects for Board {
    async fn on_event(&mut self, event: BoardEvent) {
        let mut sh = self.0.borrow_mut();
        for cmd in oled_cmds_for_event(&event) {
            sh.ctl.apply(cmd);
        }
        sh.events.push(event);
    }
}

fn tally(sh: &mut Shared, key: String) {
    *sh.seen.entry(key).or_insert(0) += 1;
}

/// One prompt batch through the collector, as the device builds it, then
/// the checks, then the answers.
fn answer(sh: &mut Shared, batch: Vec<SlotOptions>, ids: &[String], rng: &mut SimpleRng) -> Vec<String> {
    // A switch out of a single-move menu must have reached the field.
    if let Some((pid, from)) = sh.switched.take() {
        if ids.contains(&pid) {
            let ok = sh.events[from..]
                .iter()
                .any(|e| matches!(e, BoardEvent::SwitchIn { player_id: Some(p), .. } if *p == pid));
            assert!(ok, "{pid}'s switch out of a single-move menu never came in");
            tally(sh, "switched out".to_string());
        } else {
            sh.switched = Some((pid, from));
        }
    }

    let mut fx = Vec::new();
    let _collector = ChoiceCollector::new(batch.clone(), &mut fx);
    for e in fx {
        if let Effect::Oled(cmd) = e {
            sh.ctl.apply(cmd);
        }
    }

    let mut out = Vec::new();
    for (mut slot, pid) in batch.into_iter().zip(ids) {
        let player = slot.player_num;
        let can_switch = slot.party_ok.iter().any(|&ok| ok);
        if slot.auto.as_deref() == Some("move 0") {
            let name = match sh.ctl.screen(player) {
                Screen::Waiting { forced: Some(name), .. } => name.to_string(),
                other => panic!("{pid}: a forced seat shows {}", other.name()),
            };
            assert!(!name.is_empty(), "{pid}: the forced move is named");
            tally(sh, alloc::format!("forced {name}{}", if can_switch { "" } else { " (no bench)" }));
        } else if slot.auto.is_none() && !slot.forced_switch && slot.n_moves == 1 {
            let name = match sh.ctl.screen(player) {
                Screen::Battle { moves, .. } => {
                    assert_eq!(moves.len(), 1, "{pid}: a single-move menu shows one move");
                    moves[0].name.clone()
                }
                other => panic!("{pid}: a single-move seat shows {}", other.name()),
            };
            if matches!(name.as_str(), "Fight" | "Struggle") {
                assert!(can_switch && !slot.trapped, "{pid}: {name} with no switch open is forced");
            }
            if can_switch && !slot.trapped {
                tally(sh, alloc::format!("choose {name} or switch"));
                // Half the time, take the switch PS leaves open.
                if rng.next_u64() % 2 == 0 {
                    let bench: Vec<usize> = (0..6).filter(|&i| slot.party_ok[i]).collect();
                    let pick = bench[(rng.next_u64() % bench.len() as u64) as usize];
                    sh.switched = Some((pid.clone(), sh.events.len()));
                    out.push(crate::battle_input::format_switch_choice(pick));
                    continue;
                }
            }
        }
        out.push(slot.random_choice(rng));
    }
    out
}

async fn robot(bus: &InputBus, shared: Rc<RefCell<Shared>>, seed: u64) {
    let mut rng = SimpleRng::new(seed);
    loop {
        let first = bus.prompt.receive().await;
        let mut prompts = vec![first];
        while prompts.len() < prompts[0].batch_total {
            prompts.push(bus.prompt.receive().await);
        }
        let ids: Vec<String> = prompts.iter().map(|p| p.player_id.clone()).collect();
        let batch: Vec<SlotOptions> = prompts.into_iter().map(|p| p.slot).collect();
        let answers = answer(&mut shared.borrow_mut(), batch, &ids, &mut rng);
        for (player_id, choice) in ids.into_iter().zip(answers) {
            bus.choices.send(PlayerChoice { player_id, choice }).await;
        }
    }
}

/// Plays seeded battles (teams from `teams(seed)`) until every kind in
/// `wanted` has been seen, at most `max` seeds.
fn sweep(wanted: &[&str], max: u64, teams: impl Fn(u64) -> [ps_protocol::PackedTeam; 2]) {
    let shared = Rc::new(RefCell::new(Shared::default()));
    let mut seed = 0u64;
    while !wanted.iter().all(|k| shared.borrow().seen.contains_key(*k)) {
        assert!(seed < max, "seeds 0..{seed} saw only {:?}", shared.borrow().seen);
        {
            let mut sh = shared.borrow_mut();
            sh.ctl = OledController::new();
            sh.events.clear();
            sh.switched = None;
        }
        let setup = BattleSetup::local(1, seed, teams(seed));
        let bus = InputBus::new();
        let mut board = Board(shared.clone());
        block_on(run_battle(&mut LocalTransport::new(), &setup, &bus, robot(&bus, shared.clone(), seed ^ 0xf0ce), &mut board));
        let sh = shared.borrow();
        assert!(
            sh.events.iter().any(|e| matches!(e, BoardEvent::Win { .. } | BoardEvent::Tie)),
            "seed {seed} ended without a result"
        );
        seed += 1;
    }
    std::println!("seeds 0..{seed}: {:?}", shared.borrow().seen);
}

/// Random-battle teams: Hyper Beam's recharge, and sleep and freeze's
/// `Fight` (switchable with a bench, forced without one).
#[test]
fn randbat_battles_reach_recharge_and_fight() {
    sweep(
        &["forced Recharge", "choose Fight or switch", "switched out", "forced Fight (no bench)"],
        400,
        |seed| crate::randbat::draw_two_ps_teams(&ps_dex::GEN1, seed, 6),
    );
}

/// Random-battle sets hardly carry a hard lock (Sky Attack, on four sets),
/// so these teams do: Thrash and Petal Dance, Rage, the charging moves
/// (hard locks: forced), and Bide and Wrap. Gen 1's Bide and a
/// partial-trapping lock are `onSemiLockMove` (`data/mods/gen1/moves.ts`,
/// `conditions.ts`), which leaves the switch open, and Wrap's target gets
/// `Fight`.
#[test]
fn lock_moves_put_the_seat_on_the_forced_screen() {
    let teams = |_| {
        let p1 = ":team p1 Nidoking:thrash:bide:rage:wrap,Vileplume:petaldance:solarbeam:skullbash:razorwind,\
                  Dodrio:skyattack:fly:dig:thrash";
        let p2 = ":team p2 Tauros:rage:bide:hyperbeam:wrap,Exeggutor:petaldance:solarbeam:sleeppowder:hypnosis,\
                  Dragonite:wrap:skullbash:thrash:hyperbeam";
        let team = |spec: &str| crate::cli_parse::parse_team_spec(spec).expect("team spec").1;
        [team(p1), team(p2)]
    };
    sweep(
        &[
            "forced Thrash",
            "forced Petal Dance",
            "forced Rage",
            "forced Solar Beam",
            "forced Recharge",
            "choose Bide or switch",
            "choose Wrap or switch",
            "choose Fight or switch",
            "switched out",
        ],
        200,
        teams,
    );
}
