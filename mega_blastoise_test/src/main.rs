mod board_game_effects;
mod harness;
mod stdin_input;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--self-test") {
        harness::run_self_test_effects();
        return;
    }

    harness::run_interactive();
}
