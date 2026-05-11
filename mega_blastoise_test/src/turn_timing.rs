//! Timing benchmark for battle turn processing.
//!
//! Run with: cargo test -p mega-blastoise-test turn_timing -- --nocapture

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use battler::{Request, TeamData};
    use mega_blastoise_core::{
        battle_options_with_seed, demo_engine_opts, draw_two_randbat_teams, format_move_choice,
        format_switch_choice, join_choice_parts, FlashDataStore,
    };

    /// Pick a valid choice string automatically — first usable move for Turn, first
    /// available bench slot for Switch.
    fn auto_choice(
        player_id: &str,
        request: &Request,
        battle: &mut battler::PublicCoreBattle<'_>,
    ) -> String {
        match request {
            Request::Turn(turn) => {
                let mut parts = Vec::new();
                for mon_req in &turn.active {
                    let slot = mon_req
                        .moves
                        .iter()
                        .position(|m| !m.disabled && m.pp > 0)
                        .unwrap_or(0);
                    parts.push(format_move_choice(slot));
                }
                join_choice_parts(&parts)
            }
            Request::Switch(sw) => {
                let pd = battle.player_data(player_id).expect("player data for switch");
                let available: Vec<usize> = pd
                    .mons
                    .iter()
                    .enumerate()
                    .filter(|(_, m)| !m.active && m.hp > 0)
                    .map(|(i, _)| i)
                    .collect();
                let parts: Vec<_> = sw
                    .needs_switch
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format_switch_choice(available.get(i).copied().unwrap_or(0)))
                    .collect();
                join_choice_parts(&parts)
            }
            _ => "pass".to_string(),
        }
    }

    #[test]
    fn turn_processing_time() {
        let data = FlashDataStore::new();
        let seed = 0xdead_beef_cafe_u64;

        let (team_red, team_blue) = draw_two_randbat_teams(seed, 3);

        let mut battle =
            battler::PublicCoreBattle::new(battle_options_with_seed(seed), &data, demo_engine_opts())
                .expect("battle init");
        battle
            .update_team("p1", TeamData { members: team_red, ..Default::default() })
            .expect("p1 team");
        battle
            .update_team("p2", TeamData { members: team_blue, ..Default::default() })
            .expect("p2 team");
        battle.start().expect("battle start");

        // Each entry: (turn, player_id, elapsed for set_player_choice)
        let mut rows: Vec<(String, Duration)> = Vec::new();
        let mut n = 0u32;

        println!("\n{:<6} {:<8} {:>12}", "#", "player", "elapsed");
        println!("{}", "-".repeat(30));

        while !battle.ended() {
            let Some((player_id, request)) = battle.active_requests().next() else {
                break;
            };

            n += 1;
            let choice = auto_choice(&player_id, &request, &mut battle);
            let t0 = Instant::now();
            battle.set_player_choice(&player_id, &choice).expect("set_player_choice");
            let elapsed = t0.elapsed();

            println!("{:<6} {:<8} {:>12.3?}", n, player_id, elapsed);
            rows.push((player_id.clone(), elapsed));
        }

        // ── Summary ────────────────────────────────────────────────────────────
        println!("\n{}", "=".repeat(36));
        println!("Total choices   : {}", rows.len());

        if rows.is_empty() {
            return;
        }

        let total: Duration = rows.iter().map(|(_, d)| *d).sum();
        let min = rows.iter().map(|(_, d)| *d).min().unwrap();
        let max = rows.iter().map(|(_, d)| *d).max().unwrap();
        let avg = total / rows.len() as u32;

        println!("Total time      : {total:.3?}");
        println!("Min             : {min:.3?}");
        println!("Max             : {max:.3?}");
        println!("Avg             : {avg:.3?}");

        // The p2 choice is where auto_continue fires the turn — report separately.
        let p2_times: Vec<Duration> =
            rows.iter().filter(|(id, _)| id == "p2").map(|(_, d)| *d).collect();
        if !p2_times.is_empty() {
            let p2_total: Duration = p2_times.iter().sum();
            let p2_max = p2_times.iter().max().unwrap();
            let p2_avg = p2_total / p2_times.len() as u32;
            println!("\np2 (turn-processing) choices: {}", p2_times.len());
            println!("  max : {p2_max:.3?}");
            println!("  avg : {p2_avg:.3?}");
        }
    }
}
