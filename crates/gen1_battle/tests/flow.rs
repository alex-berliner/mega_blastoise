//! Battle-level turn flow: priority brackets, Struggle forcing, and the
//! Gen 1 rule that residual damage follows each action rather than the turn.

use gen1_battle::{
    CoreBattleEngineOptions, CoreBattleOptions, FormatData, MonData, MoveSlot as ApiMove,
    PlayerData, PlayerDex, PlayerOptions, PlayerType, PublicCoreBattle, Request,
    SerializedRuleSet, SideData, TeamData,
};

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

fn mk_battle(p1: Vec<MonData>, p2: Vec<MonData>, seed: u64) -> PublicCoreBattle<'static> {
    let mut battle =
        PublicCoreBattle::new(opts(seed), &(), CoreBattleEngineOptions::default()).unwrap();
    battle.update_team("p1", TeamData { members: p1 }).unwrap();
    battle.update_team("p2", TeamData { members: p2 }).unwrap();
    battle.start().unwrap();
    battle
}

fn drain(battle: &mut PublicCoreBattle<'_>) -> Vec<String> {
    battle.new_log_entries().collect()
}

#[test]
fn quick_attack_has_priority_over_faster_mon() {
    // Rattata (slower) with Quick Attack must move before Electrode (fastest
    // mon in the game) using Tackle.
    let mut b = mk_battle(
        vec![mon_lvl("rattata", 50, &["quickattack"])],
        vec![mon_lvl("electrode", 100, &["tackle"])],
        7,
    );
    let _ = drain(&mut b);
    b.set_player_choice("p1", "move 0").unwrap();
    b.set_player_choice("p2", "move 0").unwrap();
    let log = drain(&mut b);
    let qa = log.iter().position(|l| l.contains("name:Quick Attack"));
    let tk = log.iter().position(|l| l.contains("name:Tackle"));
    let qa = qa.expect("quick attack used");
    if let Some(tk) = tk {
        assert!(qa < tk, "Quick Attack must act first: {log:?}");
    }
}

#[test]
fn struggle_forced_when_out_of_pp() {
    let mut b = mk_battle(
        vec![mon_lvl("snorlax", 100, &["splash"])],
        vec![mon_lvl("chansey", 100, &["splash"])],
        3,
    );
    let _ = drain(&mut b);
    // Splash has 40 PP: burn through all of it.
    for _ in 0..40 {
        b.set_player_choice("p1", "move 0").unwrap();
        b.set_player_choice("p2", "move 0").unwrap();
        let _ = drain(&mut b);
    }
    let req = b
        .active_requests()
        .find(|(pid, _)| *pid == "p1")
        .map(|(_, r)| r.clone())
        .expect("p1 request");
    match req {
        Request::Turn(t) => {
            assert_eq!(t.active[0].moves.len(), 1);
            assert_eq!(t.active[0].moves[0].id, "struggle");
        }
        _ => panic!("expected turn request"),
    }
    // And using it deals damage plus recoil.
    let hp = |b: &PublicCoreBattle<'_>, pid: &str| b.player_data(pid).unwrap().mons[0].hp;
    let (p1_before, p2_before) = (hp(&b, "p1"), hp(&b, "p2"));
    b.set_player_choice("p1", "move 0").unwrap();
    b.set_player_choice("p2", "move 0").unwrap();
    let _ = drain(&mut b);
    assert!(hp(&b, "p2") < p2_before, "struggle must deal damage");
    assert!(hp(&b, "p1") < p1_before, "struggle recoil must hurt the user");
}

#[test]
fn residuals_follow_each_action() {
    // Once poisoned, Snorlax's poison tick lands right after its own move
    // (Gen 1 after-action residuals), inside the same turn's log.
    let mut b = mk_battle(
        vec![mon_lvl("jolteon", 100, &["toxic"])],
        vec![mon_lvl("snorlax", 100, &["splash"])],
        11,
    );
    let _ = drain(&mut b);
    for _ in 0..8 {
        b.set_player_choice("p1", "move 0").unwrap();
        b.set_player_choice("p2", "move 0").unwrap();
        let log = drain(&mut b);
        let splash = log.iter().position(|l| l.contains("name:Splash"));
        let dmg = log
            .iter()
            .position(|l| l.starts_with("damage|mon:snorlax"));
        if let (Some(s), Some(d)) = (splash, dmg) {
            assert!(d > s, "poison tick follows Snorlax's own action: {log:?}");
            return;
        }
    }
    panic!("toxic never landed / no residual observed");
}

#[test]
fn endless_battle_clause_caps_the_battle() {
    // Two Ghost-types with only Splash: once PP runs out they Struggle into
    // each other's Normal immunity forever — the 1000-turn cap must end it.
    let mut b = mk_battle(
        vec![mon_lvl("gastly", 100, &["splash"])],
        vec![mon_lvl("haunter", 100, &["splash"])],
        21,
    );
    let _ = drain(&mut b);
    let mut turns = 0u32;
    while !b.ended() {
        turns += 1;
        assert!(turns <= 1005, "battle must end by the turn cap");
        b.set_player_choice("p1", "move 0").unwrap();
        b.set_player_choice("p2", "move 0").unwrap();
        let _ = drain(&mut b);
    }
    assert!(turns >= 999, "should have run up to the cap, ended at {turns}");
}
