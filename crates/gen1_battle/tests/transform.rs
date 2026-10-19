//! Transform: the user inherits the target's moves/types/stats for display
//! and play, emits the display-refresh board lines, and reverts fully when it
//! leaves the field (Gen 1 semantics).

use gen1_battle::{
    CoreBattleEngineOptions, CoreBattleOptions, FormatData, MonData, MoveSlot as ApiMove,
    PlayerData, PlayerDex, PlayerOptions, PlayerType, PublicCoreBattle, Request,
    SerializedRuleSet, SideData, TeamData,
};

fn mon(species: &str, moves: &[&str]) -> MonData {
    MonData {
        name: species.to_string(),
        species: species.to_string(),
        level: 100,
        moves: moves
            .iter()
            .map(|id| ApiMove { id: id.to_string(), ..Default::default() })
            .collect(),
        ..Default::default()
    }
}

fn opts(seed: u64) -> CoreBattleOptions {
    let player = |id: &str, name: &str| PlayerData {
        id: id.to_string(),
        name: name.to_string(),
        player_type: PlayerType::Trainer,
        player_options: PlayerOptions::default(),
        team: TeamData::default(),
        dex: PlayerDex::default(),
    };
    CoreBattleOptions {
        seed: Some(seed),
        format: FormatData {
            battle_type: gen1_battle::BattleType::Singles,
            rules: SerializedRuleSet::new(),
        },
        field: Default::default(),
        side_1: SideData { name: "Red".to_string(), players: vec![player("p1", "Red")] },
        side_2: SideData { name: "Blue".to_string(), players: vec![player("p2", "Blue")] },
    }
}

fn p1_turn_moves(battle: &PublicCoreBattle<'_>) -> Vec<String> {
    match battle.active_requests().find(|(pid, _)| *pid == "p1") {
        Some((_, Request::Turn(t))) => t.active[0].moves.iter().map(|m| m.name.clone()).collect(),
        _ => Vec::new(),
    }
}

#[test]
fn transform_inherits_moves_and_reverts_on_switch() {
    let data = ();
    let mut battle =
        PublicCoreBattle::new(opts(7), &data, CoreBattleEngineOptions::default()).unwrap();
    battle
        .update_team(
            "p1",
            TeamData {
                members: vec![mon("ditto", &["transform"]), mon("chansey", &["splash"])],
            },
        )
        .unwrap();
    battle
        .update_team("p2", TeamData { members: vec![mon("chansey", &["splash", "softboiled"])] })
        .unwrap();
    battle.start().unwrap();
    let _ = battle.new_log_entries().count();

    battle.set_player_choice("p1", "move 0").unwrap();
    battle.set_player_choice("p2", "move 0").unwrap();

    let log: Vec<String> = battle.new_log_entries().collect();
    assert!(
        log.iter().any(|l| l.contains("what:transform") && l.contains("move:chansey")),
        "transform start line names the target: {log:?}"
    );
    assert!(
        log.iter().any(|l| l.starts_with("activemon|") && l.contains("name:chansey")),
        "activemon display-refresh line present: {log:?}"
    );

    // Ditto's request now offers Chansey's moves.
    let moves = p1_turn_moves(&battle);
    assert!(
        moves.iter().any(|m| m == "Splash") && moves.iter().any(|m| m == "Soft-Boiled"),
        "transformed request offers the target's moves: {moves:?}"
    );

    // Switch Ditto out — Transform reverts.
    battle.set_player_choice("p1", "switch 1").unwrap();
    battle.set_player_choice("p2", "move 0").unwrap();
    let _ = battle.new_log_entries().count();
    // Switch back in.
    battle.set_player_choice("p1", "switch 0").unwrap();
    battle.set_player_choice("p2", "move 0").unwrap();
    let _ = battle.new_log_entries().count();

    let moves = p1_turn_moves(&battle);
    assert_eq!(moves, vec!["Transform".to_string()], "reverted to Ditto's own moves: {moves:?}");
}
