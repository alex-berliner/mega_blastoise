use gen3_battle::battle::{Battle, Choice, Event, SeatScript, Side, TurnScript};
use gen3_battle::{Invest, Mon, Nature};

#[test]
fn a_self_destruct_that_wins_the_battle_ends_it() {
    let inv = Invest { iv: 31, ev: 0 };
    let fast = Mon::new("milotic", 70, Nature::Hardy, inv, &["selfdestruct"]).unwrap();
    let slow = Mon::new("ampharos", 70, Nature::Hardy, inv, &["thundershock"]).unwrap();
    let mut b = Battle::new(Side::new(vec![slow]), Side::new(vec![fast]), 1);
    let seat = SeatScript {
        hit: true,
        crit: false,
        random: 100,
        secondary: false,
        immobile: false,
        hits: 0,
        selfhit: false,
        stall: false,
    };
    let script = TurnScript { seats: [Some(seat), Some(seat)] };
    let events = b.step_with([Choice::Move(0), Choice::Move(0)], &script);
    let used: Vec<u8> = events
        .iter()
        .filter_map(|e| match e {
            Event::Used { side, .. } => Some(*side),
            _ => None,
        })
        .collect();
    println!("over={} used={:?}", b.over(), used);
    println!("p1 pp={} p2 hp={}", b.sides[0].mon().moves[0].pp, b.sides[1].mon().hp);
    assert_eq!(used, vec![2], "only the exploder acts; the battle is already over");
}
