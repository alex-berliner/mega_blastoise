use std::io::{self, Write};

use gen1_battle::Request;
use mega_blastoise_core::{
    format_move_choice, format_switch_choice, join_choice_parts, player_display_name,
    ActivePrompt, InputBus, InputSource,
};

pub struct StdinBattleInput;

impl StdinBattleInput {
    /// Run forever: wait for each prompt on `bus`, handle it synchronously, send the choice back.
    /// Blocking stdin reads are fine here — the test binary is single-threaded and nothing
    /// else needs to run while waiting for the user to type.
    pub async fn run(&mut self, bus: &InputBus) {
        loop {
            let ActivePrompt { player_id, request, .. } = bus.prompt.receive().await;
            let choice = self.handle(&player_id, &request);
            bus.choices.send(choice).await;
        }
    }

    fn handle(&self, player_id: &str, request: &Request) -> String {
        let label = player_display_name(player_id);
        match request {
            Request::Turn(turn) => {
                let mut parts = Vec::new();
                for mon_req in &turn.active {
                    println!("\n=== {label} ({player_id}) — choose move ===");
                    let n_moves = mon_req.moves.len().min(4);
                    if n_moves == 0 {
                        eprintln!("No moves available; passing.");
                        parts.push("pass".to_string());
                        continue;
                    }
                    for i in 0..n_moves {
                        let m = &mon_req.moves[i];
                        let status = if m.disabled || m.pp == 0 { " (disabled)" } else { "" };
                        println!("  [{}] {}  PP {}/{}{}", i + 1, m.name, m.pp, m.max_pp, status);
                    }
                    loop {
                        let btn = Self::prompt_usize_inclusive(
                            &format!("{label}, pick move [1-{n_moves}]: "),
                            1,
                            n_moves,
                        );
                        let slot = btn - 1;
                        let m = &mon_req.moves[slot];
                        if m.disabled || m.pp == 0 {
                            eprintln!("That move cannot be used. Pick another.");
                            continue;
                        }
                        parts.push(format_move_choice(slot));
                        break;
                    }
                }
                join_choice_parts(&parts)
            }
            Request::Switch(sw) => {
                let mut parts = Vec::new();
                for _ in &sw.needs_switch {
                    println!("\n=== {label} ({player_id}) — switch (bench 1-6) ===");
                    let bench = Self::prompt_usize_inclusive(
                        &format!("{label}, which party slot to send in [1-6]? "),
                        1,
                        6,
                    );
                    parts.push(format_switch_choice(bench - 1));
                }
                join_choice_parts(&parts)
            }
            Request::TeamPreview(_) => {
                eprintln!("Team preview not handled; using random.");
                "random".to_string()
            }
            Request::LearnMove(_) => {
                eprintln!("Learn move not handled; passing.");
                "pass".to_string()
            }
        }
    }

    fn prompt_usize_inclusive(prompt: &str, min: usize, max: usize) -> usize {
        loop {
            print!("{prompt}");
            let _ = io::stdout().flush();
            let mut line = String::new();
            match io::stdin().read_line(&mut line) {
                Ok(0) => {
                    eprintln!("\nstdin closed (EOF) — exiting.");
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
            eprintln!("Please enter a number from {min} to {max}.");
        }
    }
}

impl InputSource for StdinBattleInput {
    async fn run(&mut self, bus: &InputBus) {
        StdinBattleInput::run(self, bus).await
    }
}
