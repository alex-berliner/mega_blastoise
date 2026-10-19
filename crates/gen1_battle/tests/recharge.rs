//! Hyper Beam recharge: the recharging mon's next request offers a single
//! selectable "Recharge" move (not an auto-submitted lock), and no recharge
//! happens when the beam KOs the target.

use gen1_battle::{
    CoreBattleEngineOptions, CoreBattleOptions, FormatData, MonData, MoveSlot as ApiMove,
    PlayerData, PlayerDex, PlayerOptions, PlayerType, PublicCoreBattle, Request,
    SerializedRuleSet, SideData, TeamData,
};

fn mon(species: &str, moves: &[&str]) -> MonData {
    mon_lvl(species, 100, moves)
}

fn mon_lvl(species: &str, level: u8, moves: &[&str]) -> MonData {
    MonData {
        name: species.to_string(),
        species: species.to_string(),
        level,
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

/// Run one turn of Hyper Beam vs Splash for `seed`; return p1's follow-up
/// Turn request move list if p1 acted (None if the beam missed or state is odd).
fn hyper_beam_turn(seed: u64) -> Option<Vec<String>> {
    let data = ();
    let mut battle =
        PublicCoreBattle::new(opts(seed), &data, CoreBattleEngineOptions::default()).ok()?;
    // Chansey vs Chansey: massive HP pools, no KO from one Hyper Beam.
    battle.update_team("p1", TeamData { members: vec![mon("chansey", &["hyperbeam"])] }).ok()?;
    battle.update_team("p2", TeamData { members: vec![mon("chansey", &["splash"])] }).ok()?;
    battle.start().ok()?;

    battle.set_player_choice("p1", "move 0").ok()?;
    battle.set_player_choice("p2", "move 0").ok()?;

    // Did the beam connect? (drain log, look for p1 damage on p2)
    let hit = battle.new_log_entries().any(|l| l.contains("damage") && l.contains("p2"));
    if !hit {
        return None;
    }

    let req = battle
        .active_requests()
        .find(|(pid, _)| *pid == "p1")
        .map(|(_, r)| r.clone())?;
    match req {
        Request::Turn(t) => {
            Some(t.active[0].moves.iter().map(|m| m.name.clone()).collect())
        }
        _ => None,
    }
}

#[test]
fn recharge_is_a_single_selectable_move() {
    let mut checked = 0;
    for seed in 0..40u64 {
        if let Some(moves) = hyper_beam_turn(seed) {
            assert_eq!(moves, vec!["Recharge".to_string()], "seed {seed}");
            checked += 1;
        }
    }
    assert!(checked > 0, "Hyper Beam never connected across all seeds");
}

#[test]
fn no_recharge_after_beam_koes_target() {
    let data = ();
    for seed in 0..40u64 {
        let mut battle =
            PublicCoreBattle::new(opts(seed), &data, CoreBattleEngineOptions::default()).unwrap();
        // Tauros nukes a level-5 Abra; a connecting beam KOs it — no recharge.
        battle
            .update_team("p1", TeamData { members: vec![mon("tauros", &["hyperbeam"])] })
            .unwrap();
        battle
            .update_team(
                "p2",
                TeamData {
                    members: vec![mon_lvl("abra", 5, &["splash"]), mon("chansey", &["splash"])],
                },
            )
            .unwrap();
        battle.start().unwrap();
        battle.set_player_choice("p1", "move 0").unwrap();
        battle.set_player_choice("p2", "move 0").unwrap();

        let fainted = battle.new_log_entries().any(|l| l.contains("faint"));
        if !fainted {
            continue; // missed — irrelevant seed
        }
        // p2 replaces its mon; p1's next Turn request must be its real moves.
        if let Some((_, Request::Switch(_))) =
            battle.active_requests().find(|(pid, _)| *pid == "p2")
        {
            battle.set_player_choice("p2", "switch 1").unwrap();
        }
        let req = battle
            .active_requests()
            .find(|(pid, _)| *pid == "p1")
            .map(|(_, r)| r.clone())
            .expect("p1 request after KO");
        if let Request::Turn(t) = req {
            assert_eq!(t.active[0].moves[0].name, "Hyper Beam", "seed {seed}");
            return;
        }
    }
    panic!("Hyper Beam never KO'd across all seeds");
}
