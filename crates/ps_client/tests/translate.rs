//! Hand-made cases the recorded corpus does not reach: the typed `|split|`
//! form, a text pair whose shared line is a percentage, request positions
//! that differ from team slots, and the request shapes that pre-commit a
//! choice.

mod common;

use ps_client::{choice, choice_string, slot_options, BattleView, BoardEvent, PromptKind};
use ps_protocol::text::json::decode_request;
use ps_protocol::text::line::{decode_line, CodecState};
use ps_protocol::text::names::Tables;
use ps_protocol::{Choice, Health, Hp, Msg, PokemonIdent, Request, SideId};

const TEAM: [&str; 3] = ["Charizard", "Horsea", "Growlithe"];

fn setup() -> (CodecState<Tables>, BattleView) {
    setup_with(&ps_dex::GEN1)
}

fn setup_with(dex: &'static ps_dex::Dex) -> (CodecState<Tables>, BattleView) {
    let mut codec = CodecState::new(common::tables(dex));
    let mut view = BattleView::new(dex);
    for side in [SideId::P1, SideId::P2] {
        for (slot, nick) in TEAM.iter().enumerate() {
            codec.battle_mut().set_nickname(side, nick);
            view.set_nickname(side, slot as u8, nick);
        }
    }
    (codec, view)
}

fn line(codec: &mut CodecState<Tables>, view: &mut BattleView, l: &str) -> Vec<BoardEvent> {
    let msg = decode_line(l, codec).unwrap_or_else(|e| panic!("{l}: {e}"));
    view.apply(&msg)
}

fn mon(name: &str, details: &str, condition: &str, active: bool, moves: &str) -> String {
    format!(
        "{{\"ident\":\"p1: {name}\",\"details\":\"{details}\",\"condition\":\"{condition}\",\"active\":{active},\
         \"stats\":{{\"atk\":198,\"def\":189,\"spa\":199,\"spd\":199,\"spe\":221}},\"moves\":[{moves}],\
         \"baseAbility\":\"noability\",\"item\":\"\",\"pokeball\":\"pokeball\"}}"
    )
}

/// Growlithe (team slot 2) active, so PS lists it first: positions
/// Growlithe, Horsea, Charizard.
fn side_json() -> String {
    format!(
        "\"side\":{{\"name\":\"Player 1\",\"id\":\"p1\",\"pokemon\":[{},{},{}]}}",
        mon("Growlithe", "Growlithe, L91", "200/285", true, "\"bodyslam\",\"reflect\""),
        mon("Horsea", "Horsea, L88", "0 fnt", false, "\"surf\""),
        mon("Charizard", "Charizard, L74", "268/268", false, "\"fireblast\""),
    )
}

fn request(codec: &mut CodecState<Tables>, json: &str) -> Request {
    let (dex, names) = codec.split_mut();
    decode_request(json, dex, names).unwrap_or_else(|e| panic!("{json}: {e}"))
}

fn move_request(active: &str) -> String {
    format!("{{\"active\":[{active}],{}}}", side_json())
}

const TWO_MOVES: &str = "{\"moves\":[\
    {\"move\":\"Body Slam\",\"id\":\"bodyslam\",\"pp\":24,\"maxpp\":24,\"target\":\"normal\",\"disabled\":false},\
    {\"move\":\"Reflect\",\"id\":\"reflect\",\"pp\":0,\"maxpp\":32,\"target\":\"self\",\"disabled\":false}]}";

#[test]
fn a_typed_split_line_shows_the_exact_hp() {
    let (_, mut view) = setup();
    let me = PokemonIdent::active(SideId::P2, 0);
    let details = ps_protocol::Details::new(ps_protocol::SpeciesNum(5), 74, ps_protocol::Gender::M, false);
    view.apply(&Msg::switch(me, details, Health::split_percent(268, 268, None)));
    let ev = view.apply(&Msg::damage(me, Health::split_percent(134, 268, None)));
    assert_eq!(ev, vec![BoardEvent::Damage { mon: "Charizard,p2,0".into(), health: "134/268".into() }]);
    assert_eq!(view.mon(me).unwrap().hp, Hp::new(134, 268));
}

#[test]
fn a_text_split_pair_is_one_event_from_the_secret_line() {
    let (mut codec, mut view) = setup();
    line(&mut codec, &mut view, "|split|p2");
    let sw = line(&mut codec, &mut view, "|switch|p2a: Charizard|Charizard, L74|268/268");
    assert!(matches!(&sw[..], [BoardEvent::SwitchIn { .. }]));
    assert!(line(&mut codec, &mut view, "|switch|p2a: Charizard|Charizard, L74|100/100").is_empty());
    line(&mut codec, &mut view, "|split|p2");
    let ev = line(&mut codec, &mut view, "|-damage|p2a: Charizard|134/268 par");
    assert_eq!(ev, vec![BoardEvent::Damage { mon: "Charizard,p2,0".into(), health: "134/268".into() }]);
    assert!(line(&mut codec, &mut view, "|-damage|p2a: Charizard|50/100 par").is_empty());
    let m = view.mon(PokemonIdent::active(SideId::P2, 0)).unwrap();
    assert_eq!((m.hp, m.status), (Hp::new(134, 268), Some(ps_protocol::Status::Par)));
    // An unsplit line after the pair applies as it stands.
    let ev = line(&mut codec, &mut view, "|-heal|p2a: Charizard|200/268 par");
    assert_eq!(ev.len(), 1);
}

#[test]
fn switch_answers_name_team_slots_and_choices_name_request_positions() {
    let (mut codec, mut view) = setup();
    let req = request(&mut codec, &move_request(TWO_MOVES));
    view.apply_request(&req);
    // Team slot 0 (Charizard) is third in the request.
    assert_eq!(choice(&req, "switch 0"), Some(Choice::Switch(3)));
    assert_eq!(choice_string(&req, "switch 0").as_deref(), Some("switch 3"));
    assert_eq!(choice_string(&req, "switch 2").as_deref(), Some("switch 1"));
    assert_eq!(choice_string(&req, "move 0").as_deref(), Some("move 1"));
    assert_eq!(choice_string(&req, "switch 5"), None, "no such team slot");
    assert_eq!(choice_string(&req, "pass").as_deref(), Some("pass"));

    let s = slot_options(&view, &req).unwrap();
    assert_eq!(s.active_slot, Some(2));
    assert_eq!(s.party_ok, [true, false, false, false, false, false], "Charizard only: Horsea fainted");
    assert_eq!((s.n_moves, s.usable[0], s.usable[1]), (2, true, false), "0 PP is not usable");
    assert_eq!(s.party.len(), 3);
    assert_eq!(s.party[2].name, "Growlithe");
    assert!(s.party[2].active);
    assert_eq!((s.party[2].hp, s.party[2].max_hp), (200, 285));
    assert_eq!(s.party[1].hp, 0);
    assert!(s.auto.is_none() && !s.trapped && !s.forced_switch);
}

#[test]
fn locked_recharging_and_trapped_seats() {
    let (mut codec, mut view) = setup();
    let locked = request(&mut codec, &move_request("{\"moves\":[{\"move\":\"Body Slam\",\"id\":\"bodyslam\"}]}"));
    let ev = view.apply_request(&locked);
    assert!(matches!(ev.last(), Some(BoardEvent::Prompt { kind: PromptKind::ChooseMove, .. })));
    assert_eq!(slot_options(&view, &locked).unwrap().auto.as_deref(), Some("move 0"));
    assert_eq!(choice_string(&locked, "move 0").as_deref(), Some("move 1"));

    let recharge = request(&mut codec, &move_request("{\"moves\":[{\"move\":\"Recharge\",\"id\":\"recharge\"}]}"));
    let ev = view.apply_request(&recharge);
    match &ev[..] {
        [BoardEvent::MovesUpdate { moves, .. }, BoardEvent::Prompt { .. }] => {
            assert_eq!(moves.len(), 1);
            assert_eq!(moves[0].name, "Recharge");
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(slot_options(&view, &recharge).unwrap().auto.as_deref(), Some("move 0"));

    let trapped = request(&mut codec, &move_request(&TWO_MOVES.replace("]}", "],\"trapped\":true}")));
    assert!(slot_options(&view, &trapped).unwrap().trapped);
}

#[test]
fn forced_switch_and_wait_requests() {
    let (mut codec, mut view) = setup();
    let json = format!("{{\"forceSwitch\":[true],{},\"noCancel\":true}}", side_json());
    let sw = request(&mut codec, &json);
    let ev = view.apply_request(&sw);
    assert_eq!(ev.last(), Some(&BoardEvent::Prompt { player_id: "p1".into(), kind: PromptKind::ChooseSwitch }));
    let s = slot_options(&view, &sw).unwrap();
    assert!(s.forced_switch);
    assert_eq!(s.n_moves, 0);

    let wait = request(&mut codec, &format!("{{\"wait\":true,{}}}", side_json()));
    assert!(view.apply_request(&wait).iter().all(|e| !matches!(e, BoardEvent::Prompt { .. })));
    assert!(slot_options(&view, &wait).is_none());
}

#[test]
fn gen1_special_is_one_stat_change_and_the_miss_names_the_defender() {
    let (mut codec, mut view) = setup();
    line(&mut codec, &mut view, "|switch|p1a: Charizard|Charizard, L74|268/268");
    line(&mut codec, &mut view, "|switch|p2a: Horsea|Horsea, L88|232/232");
    let a = line(&mut codec, &mut view, "|-boost|p1a: Charizard|spa|2");
    let b = line(&mut codec, &mut view, "|-boost|p1a: Charizard|spd|2");
    assert_eq!(a, vec![BoardEvent::StatChange { mon: "Charizard,p1,0".into(), stat: "Special".into(), delta: 2 }]);
    assert!(b.is_empty());
    assert_eq!(view.mon(PokemonIdent::active(SideId::P1, 0)).unwrap().boosts[3], 2, "both boosts tracked");
    let miss = line(&mut codec, &mut view, "|-miss|p1a: Charizard");
    assert_eq!(miss, vec![BoardEvent::Miss { mon: "Horsea,p2,0".into() }]);
    let win = line(&mut codec, &mut view, "|win|Player 2");
    assert_eq!(win, vec![BoardEvent::Win { side: Some("1".into()) }]);
}

#[test]
fn baton_pass_keeps_boosts_and_a_plain_switch_clears_them() {
    let (mut codec, mut view) = setup_with(&ps_dex::GEN3);
    line(&mut codec, &mut view, "|switch|p1a: Charizard|Charizard, L74|268/268");
    line(&mut codec, &mut view, "|-boost|p1a: Charizard|atk|2");
    line(&mut codec, &mut view, "|switch|p1a: Horsea|Horsea, L88|232/232|[from] Baton Pass");
    let horsea = PokemonIdent::active(SideId::P1, 1);
    assert_eq!(view.mon(horsea).unwrap().boosts[0], 2, "passed");
    assert_eq!(view.mon(PokemonIdent::active(SideId::P1, 0)).unwrap().boosts[0], 0, "cleared on the passer");
    line(&mut codec, &mut view, "|switch|p1a: Growlithe|Growlithe, L91|285/285");
    line(&mut codec, &mut view, "|switch|p1a: Horsea|Horsea, L88|232/232");
    assert_eq!(view.mon(horsea).unwrap().boosts[0], 0, "a plain switch passes nothing");
}

#[test]
fn the_view_is_small_and_fixed() {
    // Design section 6: fixed RAM per battle. The view holds no heap.
    let size = std::mem::size_of::<BattleView>();
    eprintln!("size_of::<BattleView>() = {size}");
    assert!(size <= 2048, "BattleView is {size} bytes");
}

#[test]
fn trapping_activations_start_effects() {
    let (mut codec, mut view) = setup_with(&ps_dex::GEN3);
    line(&mut codec, &mut view, "|switch|p1a: Charizard|Charizard, L74|268/268");
    line(&mut codec, &mut view, "|switch|p2a: Horsea|Horsea, L88|232/232");
    let t = line(&mut codec, &mut view, "|-activate|p1a: Charizard|trapped");
    assert_eq!(t, vec![BoardEvent::EffectStart { mon: "Charizard,p1,0".into(), what: "trapped".into(), detail: None }]);
    let w = line(&mut codec, &mut view, "|-activate|p1a: Charizard|move: Whirlpool|[of] p2a: Horsea");
    assert_eq!(w, vec![BoardEvent::EffectStart { mon: "Charizard,p1,0".into(), what: "wrap".into(), detail: None }]);
    assert!(line(&mut codec, &mut view, "|-activate|p1a: Charizard|confusion").is_empty());
}
