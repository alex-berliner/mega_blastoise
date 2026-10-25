extern crate alloc;

use crate::rng::SimpleRng;

pub struct RandomAi(SimpleRng);

impl RandomAi {
    pub fn new(seed: u64) -> Self {
        Self(SimpleRng::new(seed))
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0.next_u64()
    }

    /// The RNG itself, for the generation-neutral
    /// [`crate::choice_collect::SlotOptions::random_choice`] policy.
    pub fn rng_mut(&mut self) -> &mut SimpleRng {
        &mut self.0
    }

}
