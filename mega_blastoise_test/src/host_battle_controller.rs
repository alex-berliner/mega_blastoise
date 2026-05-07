/// Host mirror of `mega_blastoise_fw::battle_controller::BattleController`.
///
/// Races a pre-fed simulated button queue against stdin, mirroring the firmware's
/// `select(usb_read_line(), buttons.wait_move(...))` pattern. When the button queue
/// has a valid press it resolves immediately (button wins); when empty the controller
/// falls back to blocking stdin (USB/stdin wins). Pre-feed presses via
/// `controller.buttons.queue_move(slot)` / `queue_switch(idx)` before running a
/// battle to drive fully automated tests without user input.
use std::collections::VecDeque;
use std::io::{self, Write};

use battler::{PlayerBattleData, Request};
use embassy_futures::select::{select, Either};
use mega_blastoise_core::{
    format_move_choice, format_switch_choice, join_choice_parts, ActivePrompt, InputBus, InputSource,
};

// ── Simulated button source ───────────────────────────────────────────────────

/// Pre-fed button queue — mirrors the GPIO button matrix of `PicoBattleInput`.
/// Move and switch queues are kept separate so pre-feeding one kind does not
/// accidentally consume entries intended for the other.
pub struct SimButtonSource {
    moves: VecDeque<usize>,
    switches: VecDeque<usize>,
}

impl SimButtonSource {
    pub fn new() -> Self {
        Self { moves: VecDeque::new(), switches: VecDeque::new() }
    }

    pub fn queue_move(&mut self, slot: usize) {
        self.moves.push_back(slot);
    }

    pub fn queue_switch(&mut self, party_index: usize) {
        self.switches.push_back(party_index);
    }

    /// Non-blocking: returns the next valid move slot from the queue, or `None`.
    /// Discards out-of-range or unusable slots silently (mirrors firmware behaviour).
    fn try_next_move(&mut self, n: usize, is_usable: impl Fn(usize) -> bool) -> Option<usize> {
        while let Some(&slot) = self.moves.front() {
            self.moves.pop_front();
            if slot < n && is_usable(slot) {
                return Some(slot);
            }
        }
        None
    }

    /// Non-blocking: returns the next party index from the switch queue, or `None`.
    fn try_next_switch(&mut self) -> Option<usize> {
        self.switches.pop_front()
    }
}

// ── Controller ────────────────────────────────────────────────────────────────

pub struct HostBattleController {
    pub buttons: SimButtonSource,
}

impl HostBattleController {
    pub fn new() -> Self {
        Self { buttons: SimButtonSource::new() }
    }

    pub async fn run_inner(&mut self, bus: &InputBus) {
        loop {
            // Drain bus.log while waiting for the next prompt — mirrors UsbBattleInput::run_inner.
            let prompt = loop {
                match select(bus.prompt.receive(), bus.log.receive()).await {
                    Either::First(p) => {
                        while let Ok(line) = bus.log.try_receive() {
                            println!("[EVT] {line}");
                        }
                        break p;
                    }
                    Either::Second(line) => println!("[EVT] {line}"),
                }
            };

            let ActivePrompt { player_id, request, player_data } = prompt;
            let choice = self.handle(&player_id, &request, player_data).await;
            bus.choices.send(choice).await;

            while let Ok(line) = bus.log.try_receive() {
                println!("[EVT] {line}");
            }
        }
    }

    async fn handle(&mut self, player_id: &str, request: &Request, player_data: Option<PlayerBattleData>) -> String {
        let label = Self::player_label(player_id);
        match request {
            Request::Turn(turn) => {
                let mut parts = Vec::new();
                for mon_req in &turn.active {
                    let n = mon_req.moves.len().min(4);

                    println!("\n══ {label} ({player_id}) — choose move ══");
                    if n == 0 {
                        println!("  No moves — passing.");
                        parts.push("pass".to_string());
                        continue;
                    }
                    for i in 0..n {
                        let m = &mon_req.moves[i];
                        let tag = if m.disabled {
                            " [DISABLED]"
                        } else if m.pp == 0 {
                            " [NO PP]"
                        } else {
                            ""
                        };
                        println!("  [{}] {:<20}  PP {}/{}{}", i + 1, m.name, m.pp, m.max_pp, tag);
                    }

                    let is_usable =
                        |i: usize| !mon_req.moves[i].disabled && mon_req.moves[i].pp > 0;

                    loop {
                        // Button branch: resolves immediately when queued, else pends.
                        // Stdin branch: blocks until user types.
                        // select polls button first; if Pending it polls stdin (which blocks).
                        let slot = match select(
                            button_move_future(&mut self.buttons, n, is_usable),
                            stdin_number(&format!("{label}, move [1-{n}]: "), 1, n),
                        )
                        .await
                        {
                            Either::First(slot) => {
                                println!("[BTN] Move {} — {}", slot + 1, mon_req.moves[slot].name);
                                slot
                            }
                            Either::Second(btn) => {
                                let slot = btn - 1;
                                let m = &mon_req.moves[slot];
                                if m.disabled || m.pp == 0 {
                                    println!("[!!] That move is not available.");
                                    continue;
                                }
                                println!("[USB] Move {} — {}", slot + 1, m.name);
                                slot
                            }
                        };
                        parts.push(format_move_choice(slot));
                        break;
                    }
                }
                join_choice_parts(&parts)
            }

            Request::Switch(sw) => {
                let mut parts = Vec::new();
                for _ in &sw.needs_switch {
                    println!("\n══ {label} ({player_id}) — switch [1-6] ══");

                    // Button queue → auto-pick from player_data → stdin.
                    let idx = if let Some(idx) = self.buttons.try_next_switch() {
                        println!("[BTN] Switching in slot {}", idx + 1);
                        idx
                    } else if let Some(auto) = first_available_bench(&player_data) {
                        println!("[AUTO] Switching in slot {} (first available bench)", auto + 1);
                        auto
                    } else {
                        let btn = stdin_number(&format!("{label}, switch slot [1-6]: "), 1, 6).await;
                        println!("[USB] Switching in slot {btn}");
                        btn - 1
                    };
                    parts.push(format_switch_choice(idx));
                }
                join_choice_parts(&parts)
            }

            Request::TeamPreview(_) => "random".to_string(),
            Request::LearnMove(_) => "pass".to_string(),
        }
    }

    fn player_label(id: &str) -> &'static str {
        match id {
            "p1" => "Red",
            "p2" => "Blue",
            _ => "?",
        }
    }
}

impl InputSource for HostBattleController {
    async fn run(&mut self, bus: &InputBus) {
        self.run_inner(bus).await
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Returns the party index of the first non-active, non-fainted bench Pokémon,
/// or `None` if player_data is absent or all bench slots are exhausted.
fn first_available_bench(player_data: &Option<PlayerBattleData>) -> Option<usize> {
    let pd = player_data.as_ref()?;
    pd.mons.iter().enumerate().find_map(|(i, m)| {
        if !m.active && m.hp > 0 { Some(i) } else { None }
    })
}

// ── Async shims ───────────────────────────────────────────────────────────────

/// Resolves immediately with the slot if a button press is queued; otherwise pends forever
/// so the stdin branch of the enclosing `select` wins.
async fn button_move_future(
    src: &mut SimButtonSource,
    n: usize,
    is_usable: impl Fn(usize) -> bool,
) -> usize {
    if let Some(slot) = src.try_next_move(n, is_usable) {
        return slot;
    }
    core::future::pending().await
}

/// Blocking stdin read wrapped in an async fn — safe with single-threaded `pollster`.
async fn stdin_number(prompt: &str, min: usize, max: usize) -> usize {
    loop {
        print!("{prompt}");
        let _ = io::stdout().flush();
        let mut line = String::new();
        match io::stdin().read_line(&mut line) {
            Ok(0) => {
                eprintln!("\nstdin EOF — exiting.");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("stdin error: {e} — exiting.");
                std::process::exit(1);
            }
            Ok(_) => {}
        }
        if let Ok(n) = line.trim().parse::<usize>() {
            if (min..=max).contains(&n) {
                return n;
            }
        }
        eprintln!("Enter a number from {min} to {max}.");
    }
}
