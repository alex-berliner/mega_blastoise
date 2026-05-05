//! Scripted [`BoardEvent`] stream matches what [`BoardEffects`] receives (host pipeline smoke).

use mega_blastoise_core::{BoardEvent, BoardEffects, BoardEventQueue, PromptKind};

struct Recorder(Vec<BoardEvent>);

impl BoardEffects for Recorder {
    fn on_event(&mut self, event: BoardEvent) {
        self.0.push(event);
    }
}

#[test]
fn scripted_board_events_dispatch_in_order() {
    let mut queue = BoardEventQueue::new();
    let mut recorder = Recorder(Vec::new());

    let script = [
        BoardEvent::BattleStart,
        BoardEvent::Prompt {
            player_id: "p1".into(),
            kind: PromptKind::ChooseMove,
        },
        BoardEvent::Move {
            user: Some("Charizard".into()),
            name: "Flamethrower".into(),
        },
        BoardEvent::Damage {
            mon: "b:0 Blastoise".into(),
            health: "120/201".into(),
        },
        BoardEvent::Turn { n: 2 },
        BoardEvent::Win {
            side: Some("0".into()),
        },
    ];

    for e in script {
        queue.push_event(e);
    }
    queue.dispatch_all(&mut recorder);

    assert_eq!(recorder.0.len(), 6);
    assert!(matches!(recorder.0[0], BoardEvent::BattleStart));
    assert!(matches!(
        recorder.0[1],
        BoardEvent::Prompt {
            player_id: ref pid,
            kind: PromptKind::ChooseMove
        } if pid == "p1"
    ));
    assert!(matches!(
        recorder.0[2],
        BoardEvent::Move { ref name, .. } if name == "Flamethrower"
    ));
    assert!(matches!(recorder.0[5], BoardEvent::Win { .. }));
}
