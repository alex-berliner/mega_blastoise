//! Host stand-in for firmware [`mega_blastoise_core::BoardEffects`] — println only.

use mega_blastoise_core::{BoardEffects, BoardEvent};

#[derive(Debug, Clone)]
pub struct BoardGameEffects {
    /// Print `Debug` of each [`BoardEvent`] after the description line.
    pub echo_debug: bool,
}

impl Default for BoardGameEffects {
    fn default() -> Self {
        Self { echo_debug: false }
    }
}

impl BoardGameEffects {
    pub fn new() -> Self {
        Self::default()
    }
}

impl BoardEffects for BoardGameEffects {
    async fn on_event(&mut self, event: BoardEvent) {
        println!("{}", event.description());
        if self.echo_debug {
            eprintln!("  {:?}", event);
        }
    }
}
