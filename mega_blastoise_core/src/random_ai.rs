extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use battler::{PlayerBattleData, Request};

use crate::rng::SimpleRng;
use crate::{format_move_choice, format_switch_choice, join_choice_parts};

pub struct RandomAi(SimpleRng);

impl RandomAi {
    pub fn new(seed: u64) -> Self {
        Self(SimpleRng::new(seed))
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0.next_u64()
    }

    pub fn make_choice(&mut self, request: &Request, player_data: Option<&PlayerBattleData>) -> String {
        match request {
            Request::Turn(turn) => {
                let mut parts = Vec::new();
                for mon_req in &turn.active {
                    let n = mon_req.moves.len().min(4);
                    if n == 0 {
                        parts.push(String::from("pass"));
                        continue;
                    }
                    let slot = (self.0.next_u64() as usize) % n;
                    parts.push(format_move_choice(slot));
                }
                join_choice_parts(&parts)
            }
            Request::Switch(sw) => {
                let valid: Vec<usize> = match player_data {
                    Some(pd) => pd.mons.iter().enumerate()
                        .filter(|(_, m)| !m.active && m.hp > 0)
                        .map(|(i, _)| i)
                        .collect(),
                    None => (1..6).collect(),
                };
                let mut parts = Vec::new();
                for _ in 0..sw.needs_switch.len() {
                    let idx = if valid.is_empty() { 0 }
                              else { valid[(self.0.next_u64() as usize) % valid.len()] };
                    parts.push(format_switch_choice(idx));
                }
                join_choice_parts(&parts)
            }
            Request::TeamPreview(_) => String::from("random"),
            Request::LearnMove(_) => String::from("pass"),
        }
    }
}
