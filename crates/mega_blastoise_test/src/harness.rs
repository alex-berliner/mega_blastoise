//! Shared battle setup and harness entrypoints.

use mega_blastoise_core::runner::ps::{run_battle, BattleSetup};
use mega_blastoise_core::transport::LocalTransport;
use mega_blastoise_core::{draw_two_ps_teams, InputBus, InputSource};

use crate::host_battle_controller::HostBattleController;
use crate::host_battle_effects::HostBattleEffects;

pub fn run_interactive() {
    let mut controller = HostBattleController::new();

    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0xdeadbeef);

    let teams = draw_two_ps_teams(&ps_dex::GEN1, seed, 3);
    let setup = BattleSetup::local(1, seed, teams);
    let mut transport = LocalTransport::new();

    println!("=== Randbat (3v3 singles) ===\n");
    println!("Each side has three random Gen 1 Pokémon. Pick moves each turn; switches use bench slots.\n");

    let bus = InputBus::new();
    let mut effects = HostBattleEffects::new(Some(&bus));
    pollster::block_on(run_battle(&mut transport, &setup, &bus, controller.run(&bus), &mut effects));

    println!("\n=== Battle over ===");
}
