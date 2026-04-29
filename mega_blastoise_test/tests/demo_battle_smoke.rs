//! Battler initializes with [`mega_blastoise_core::demo_team_red`] / [`demo_team_blue`] teams.

use battler::{
    BattleType,
    CoreBattleEngineOptions,
    CoreBattleOptions,
    FormatData,
    PlayerData,
    PlayerDex,
    PlayerOptions,
    PlayerType,
    SerializedRuleSet,
    SideData,
    TeamData,
};
use mega_blastoise_core::{demo_team_blue, demo_team_red, FlashDataStore};

fn player(id: &str, name: &str) -> PlayerData {
    PlayerData {
        id: id.to_string(),
        name: name.to_string(),
        player_type: PlayerType::Trainer,
        player_options: PlayerOptions::default(),
        team: TeamData::default(),
        dex: PlayerDex::default(),
    }
}

#[test]
fn battle_starts_with_four_mons_per_player() {
    let data = FlashDataStore::new();
    let options = CoreBattleOptions {
        seed: Some(12345),
        format: FormatData {
            battle_type: BattleType::Singles,
            rules: SerializedRuleSet::new(),
        },
        field: Default::default(),
        side_1: SideData {
            name: "Red".to_string(),
            players: vec![player("p1", "Red")],
        },
        side_2: SideData {
            name: "Blue".to_string(),
            players: vec![player("p2", "Blue")],
        },
    };

    let engine_opts = CoreBattleEngineOptions {
        validate_teams: false,
        auto_continue: true,
        reveal_actual_health: true,
        log_time: false,
        ..Default::default()
    };

    let mut battle =
        battler::PublicCoreBattle::new(options, &data, engine_opts).expect("battle init");

    battle
        .update_team(
            "p1",
            TeamData {
                members: demo_team_red(),
                ..Default::default()
            },
        )
        .expect("p1 team");
    battle
        .update_team(
            "p2",
            TeamData {
                members: demo_team_blue(),
                ..Default::default()
            },
        )
        .expect("p2 team");

    battle.start().expect("battle start");

    let p1 = battle.player_data("p1").expect("p1 data");
    let p2 = battle.player_data("p2").expect("p2 data");
    assert_eq!(p1.mons.len(), 4, "Red should have 4 Pokémon");
    assert_eq!(p2.mons.len(), 4, "Blue should have 4 Pokémon");
}
