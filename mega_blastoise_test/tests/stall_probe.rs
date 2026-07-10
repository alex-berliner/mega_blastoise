//! Termination probe: AI-vs-AI battles across many seeds must end.
use gen1_battle::{PublicCoreBattle, Request, TeamData};
use mega_blastoise_core::{
    battle_options_with_seed, demo_engine_opts, draw_two_randbat_teams, FlashDataStore, RandomAi,
};

#[test]
#[ignore] // probe — run explicitly
fn probe_termination() {
    let data = FlashDataStore::new();
    let mut stalls = Vec::new();
    for seed in 0..3000u64 {
        let (t1, t2) = draw_two_randbat_teams(seed, 3);
        let mut b = PublicCoreBattle::new(battle_options_with_seed(seed), &data, demo_engine_opts()).unwrap();
        b.update_team("p1", TeamData { members: t1, ..Default::default() }).unwrap();
        b.update_team("p2", TeamData { members: t2, ..Default::default() }).unwrap();
        b.start().unwrap();
        let mut ai = RandomAi::new(seed ^ 0xabc);
        let mut rounds = 0u32;
        while !b.ended() && rounds < 20000 {
            let reqs: Vec<(String, Request)> = b
                .active_requests()
                .map(|(p, r)| (p.to_string(), r.clone()))
                .collect();
            if reqs.is_empty() { break; }
            for (pid, req) in &reqs {
                let pd = b.player_data(pid).ok();
                let choice = ai.make_choice(req, pd.as_ref());
                let _ = b.set_player_choice(pid, &choice);
            }
            let _ = b.new_log_entries().count();
            rounds += 1;
        }
        if !b.ended() {
            // Dump the stuck state for the first few stalls.
            if stalls.len() < 3 {
                eprintln!("=== seed {seed} stalled after {rounds} rounds ===");
                for pid in ["p1", "p2"] {
                    if let Ok(pd) = b.player_data(pid) {
                        for m in pd.mons.iter().filter(|m| m.hp > 0) {
                            let mv: Vec<String> = m
                                .moves
                                .iter()
                                .map(|x| format!("{}(pp{})", x.name, x.pp))
                                .collect();
                            eprintln!("  {pid} {} hp{}/{} active={} moves={:?}", m.summary.name, m.hp, m.max_hp, m.active, mv);
                        }
                    }
                }
            }
            stalls.push(seed);
        }
    }
    eprintln!("stalled seeds: {stalls:?}");
    assert!(stalls.is_empty(), "{} of 300 battles did not terminate", stalls.len());
}
