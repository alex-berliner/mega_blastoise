//! The coverage gate's CLI surface (V-1): `ps_sim_cli --handlers <gen>`
//! and, in a `hit-count` build, `ps_sim_cli --hits FILE`
//! (`scripts/ps-coverage.sh` reads both).
//!
//! `cargo test -p ps_harness --test coverage` checks `--handlers` and that
//! `--hits` is refused without the feature; `--features hit-count` checks
//! the counts themselves.

use std::io::Write;
use std::process::{Command, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_ps_sim_cli");

fn handlers(gen: u8) -> String {
    let out = Command::new(BIN).args(["--handlers", &gen.to_string()]).output().expect("run ps_sim_cli");
    assert!(out.status.success(), "--handlers {gen} exited {:?}", out.status);
    String::from_utf8(out.stdout).unwrap()
}

/// One line per registered handler: `kind:id`, the PS key, the focus moves.
#[test]
fn handlers_list_every_registered_handler() {
    for (gen, dex) in [(1, &ps_dex::GEN1), (3, &ps_dex::GEN3)] {
        let text = handlers(gen);
        let registered: usize = ps_sim::mods::registry_for(dex).entries.iter().map(|(_, d)| d.handlers.len()).sum();
        assert_eq!(text.lines().count(), registered, "gen {gen}:\n{text}");
        for line in text.lines() {
            assert_eq!(line.split('\t').count(), 3, "gen {gen}: `{line}`");
        }
    }
    let gen1 = handlers(1);
    for line in [
        "move:bide\tbeforeMoveCallback\tbide",
        "condition:partialtrappinglock\tdurationCallback\tbind,clamp,firespin,wrap",
        "condition:twoturnmove\tonStart\tdig,fly,razorwind,skullbash,skyattack,solarbeam",
        "condition:trapped\tonTrapPokemon\t-",
        "condition:toxic\tonSourceInvulnerability\ttoxic",
    ] {
        assert!(gen1.lines().any(|l| l == line), "missing `{line}`:\n{gen1}");
    }
    assert!(gen1.lines().any(|l| l.starts_with("rule:sleepclausemod\tonSetStatus\t") && l.contains("spore")));
}

#[cfg(not(feature = "hit-count"))]
#[test]
fn hits_needs_the_feature() {
    let out = Command::new(BIN).args(["--hits", "unused"]).stdin(Stdio::null()).output().expect("run ps_sim_cli");
    assert_eq!(out.status.code(), Some(2));
}

/// Two battles: both sides use Bide in the first, Tackle in the second.
/// The hit file counts the first battle's Bide handlers once each per use,
/// in one battle, and both battles.
#[cfg(feature = "hit-count")]
#[test]
fn hits_count_calls_and_battles() {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("coverage-hits");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("h-{pid}.tsv");
    let team = |mv: &str| format!("Pikachu|||NoAbility|{mv}||255,255,255,255,255,255||30,30,30,30,30,30||50|");
    let start = ">start {\"formatid\":\"gen1customgame\",\"seed\":[1,2,3,4]}\n";
    let player = |side: &str, team: &str| format!(">player {side} {{\"name\":\"{side}\",\"team\":\"{team}\"}}\n");
    let payload = [
        start.to_string(),
        player("p1", &team("bide")),
        player("p2", &team("bide")),
        ">p1 move 1\n>p2 move 1\n".to_string(),
        start.to_string(),
        player("p1", &team("tackle")),
        player("p2", &team("tackle")),
    ]
    .concat();
    let mut child = Command::new(BIN)
        .args(["--hits", path.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .expect("spawn ps_sim_cli");
    child.stdin.as_mut().unwrap().write_all(payload.as_bytes()).unwrap();
    drop(child.stdin.take());
    assert!(child.wait().unwrap().success());

    let files: Vec<_> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path()).collect();
    assert_eq!(files.len(), 1, "{files:?}");
    let name = files[0].file_name().unwrap().to_str().unwrap().to_string();
    assert!(name.starts_with(&format!("h-{}-", child.id())), "{name}");
    let text = std::fs::read_to_string(&files[0]).unwrap();
    for line in ["battles\t1\t2", "hit\t1\tmove:bide\tbeforeMoveCallback\t2\t1", "hit\t1\tcondition:bide\tonStart\t2\t1"] {
        assert!(text.lines().any(|l| l == line), "missing `{line}`:\n{text}");
    }
}
