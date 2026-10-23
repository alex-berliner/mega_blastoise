//! `ps_sim::events::speed_sort` against PS's own `Battle#speedSort`.
//!
//! `tools/ps-diff/speedsort-fixtures.js` builds one PS battle with a fixed
//! seed and sorts 500 synthetic lists with `comparePriority`, recording the
//! permutation and the PRNG seed after each. Replaying them in order on one
//! `Prng` must give the same permutations and the same seeds: the same
//! order and the same number of tie-shuffle PRNG calls.

use ps_sim::events::{compare_priority, speed_sort, SortKey};
use ps_sim::prng::{Prng, Seed};

const FIXTURE: &str = include_str!("fixtures/speed_sort.json");

#[derive(serde::Deserialize)]
struct Fixture {
    seed: Seed,
    lists: Vec<List>,
}

#[derive(serde::Deserialize)]
struct List {
    items: Vec<Item>,
    perm: Vec<usize>,
    seed_after: Seed,
}

/// A fixture entry; a missing field is PS's `undefined`.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Item {
    order: Option<i64>,
    priority: Option<i32>,
    speed: Option<i32>,
    sub_order: Option<i32>,
    effect_order: Option<i64>,
}

impl Item {
    /// `undefined` and `0` read alike in `comparePriority` (`order || 2^32`,
    /// `priority || 0`, ...).
    fn key(&self) -> SortKey {
        SortKey {
            order: self.order.unwrap_or(0),
            priority: self.priority.unwrap_or(0),
            speed: self.speed.unwrap_or(0),
            sub_order: self.sub_order.unwrap_or(0),
            effect_order: self.effect_order.unwrap_or(0),
        }
    }
}

#[test]
fn speed_sort_matches_ps() {
    let fx: Fixture = serde_json::from_str(FIXTURE).expect("fixture parses");
    assert_eq!(fx.lists.len(), 500);
    let mut prng = Prng::new(fx.seed);
    let mut ties = 0;
    for (n, list) in fx.lists.iter().enumerate() {
        assert!((2..=12).contains(&list.items.len()));
        let mut tagged: Vec<(usize, SortKey)> = list.items.iter().map(Item::key).enumerate().collect();
        let before = prng.calls;
        speed_sort(&mut prng, &mut tagged, |a, b| compare_priority(&a.1, &b.1));
        if prng.calls != before {
            ties += 1;
        }
        let perm: Vec<usize> = tagged.iter().map(|t| t.0).collect();
        assert_eq!(perm, list.perm, "list {n}: permutation");
        assert_eq!(prng.get_seed(), list.seed_after, "list {n}: seed after");
    }
    // The fixture is meant to exercise ties; make sure it does.
    assert!(ties >= 200, "only {ties} lists shuffled a tie");
}

/// `order: 0` and a missing order both sort last, together.
#[test]
fn zero_order_is_missing_order() {
    let a = SortKey { order: 0, ..SortKey::default() };
    let b = SortKey { order: 0, priority: 0, ..SortKey::default() };
    let c = SortKey { order: 1, ..SortKey::default() };
    assert_eq!(compare_priority(&a, &b), core::cmp::Ordering::Equal);
    assert_eq!(compare_priority(&c, &a), core::cmp::Ordering::Less);
}
