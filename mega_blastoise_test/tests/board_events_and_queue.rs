//! `BoardEvent` parsing and `BoardEventQueue` log handling.

use mega_blastoise_core::{
    parse_log_line, BoardEvent, BoardEffects, BoardEventQueue, ParsedBattleLogLine,
};

struct Recorder(Vec<BoardEvent>);

impl BoardEffects for Recorder {
    fn on_event(&mut self, event: BoardEvent) {
        self.0.push(event);
    }
}

#[test]
fn parse_turn_line() {
    let e = parse_log_line("turn|turn:5").expect("turn");
    assert!(matches!(e, BoardEvent::Turn { n: 5 }));
}

#[test]
fn parse_switch_in_includes_species_and_player() {
    let line = "switch|player:p1|name:Alpha|species:Charizard|health:100/100";
    let e = parse_log_line(line).expect("switch");
    match e {
        BoardEvent::SwitchIn {
            name,
            species,
            player_id,
        } => {
            assert_eq!(name, "Alpha");
            assert_eq!(species.as_deref(), Some("Charizard"));
            assert_eq!(player_id.as_deref(), Some("p1"));
        }
        _ => panic!("expected SwitchIn"),
    }
}

#[test]
fn queue_skips_private_row_after_split_for_duplicate_switch() {
    let mut q = BoardEventQueue::new();
    let mut r = Recorder(Vec::new());
    let lines = [
        "split|side:0",
        "switch|player:p1|name:A|species:X",
        "switch|player:p1|name:A|species:X",
    ];
    q.push_log_lines(lines.into_iter());
    q.dispatch_all(&mut r);
    assert!(
        matches!(r.0.first(), Some(BoardEvent::Split { .. })),
        "first event should be Split"
    );
    let switch_count = r
        .0
        .iter()
        .filter(|e| matches!(e, BoardEvent::SwitchIn { .. }))
        .count();
    assert_eq!(
        switch_count,
        1,
        "private switch row should be skipped; only one SwitchIn"
    );
}

#[test]
fn parsed_log_line_title_and_keys() {
    let p = ParsedBattleLogLine::parse("damage|mon:a:0 Foo|health:12/100");
    assert_eq!(p.title(), "damage");
    assert_eq!(p.get("health"), Some("12/100"));
}
