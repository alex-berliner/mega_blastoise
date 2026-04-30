use std::io::{self, Write};

use battler::Request;
use mega_blastoise_core::{
    format_move_choice, format_switch_choice, join_choice_parts, BattleInput,
};

pub struct StdinBattleInput;

impl StdinBattleInput {
    fn player_label(id: &str) -> std::string::String {
        match id {
            "p1" => "Red".into(),
            "p2" => "Blue".into(),
            _ => id.to_string(),
        }
    }

    fn prompt_usize_inclusive(prompt: &str, min: usize, max: usize) -> usize {
        loop {
            print!("{prompt}");
            let _ = io::stdout().flush();
            let mut line = String::new();
            if io::stdin().read_line(&mut line).is_err() {
                continue;
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

impl BattleInput for StdinBattleInput {
    async fn read_choice(&mut self, player_id: &str, request: &Request) -> String {
        let label = Self::player_label(player_id);
        match request {
            Request::Turn(turn) => {
                let mut parts = Vec::new();
                for mon_req in &turn.active {
                    println!(
                        "\n=== {label} ({}) — choose move (1-4) ===",
                        player_id
                    );
                    let n_moves = mon_req.moves.len().min(4);
                    if n_moves == 0 {
                        eprintln!("No moves available; passing.");
                        parts.push("pass".to_string());
                        continue;
                    }
                    for i in 0..n_moves {
                        let m = &mon_req.moves[i];
                        let btn = i + 1;
                        let status = if m.disabled || m.pp == 0 {
                            " (disabled)"
                        } else {
                            ""
                        };
                        println!(
                            "  [{btn}] {}  PP {}/{}{}",
                            m.name, m.pp, m.max_pp, status
                        );
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
                for _need in &sw.needs_switch {
                    println!(
                        "\n=== {label} ({}) — switch (bench 1-6) ===",
                        player_id
                    );
                    let bench = Self::prompt_usize_inclusive(
                        &format!("{label}, which party slot to send in [1-6]? "),
                        1,
                        6,
                    );
                    let team_index = bench - 1;
                    parts.push(format_switch_choice(team_index));
                }
                join_choice_parts(&parts)
            }
            Request::TeamPreview(_) => {
                eprintln!("Team preview not handled in this demo; using random.");
                "random".to_string()
            }
            Request::LearnMove(_) => {
                eprintln!("Learn move not handled; passing.");
                "pass".to_string()
            }
        }
    }
}
