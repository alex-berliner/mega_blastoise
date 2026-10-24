//! The client against P1-4's recorded PS battles
//! (`crates/ps_protocol/tests/fixtures/{gen1,gen3}.log.gz`, 20 battles per
//! gen from PS 0.11.11's `BattleStream`).
//!
//! - `requests_agree_with_the_view`: every battle is fed through
//!   [`BattleView`] line by line. At every `|request|`, BEFORE the view
//!   learns from it, the view built from protocol lines alone must hold the
//!   HP, status and active mon the request reports (the final request
//!   included). Every recorded choice must translate back through
//!   [`choice_string`] to the same PS text, and be one [`slot_options`]
//!   allows. The active mon's PP the view tracked from `|move|` lines must
//!   equal the request's.
//! - `golden_*`: the whole `BoardEvent` stream of three battles per gen,
//!   compared to `tests/golden/*.txt`. `PS_CLIENT_BLESS=1` rewrites them.
//!
//! The text codec numbers moves, species, items and abilities by the
//! tables it is given; the test builds those from `ps_dex::GEN1` / `GEN3`, so
//! a decoded `MoveNum(n)` is the dex's move `n`, as on the typed path.

mod common;

use std::fmt::Write as _;
use std::path::Path;

use common::{read_gz, tables};

use ps_client::{choice_string, slot_options, BattleView, BoardEvent, SeatOptions};
use ps_dex::Dex;
use ps_protocol::text::json::decode_request;
use ps_protocol::text::line::{decode_line, CodecState};
use ps_protocol::text::team::unpack;
use ps_protocol::{PokemonIdent, Request, SideId, Status};

/// One recorded battle.
struct Battle {
    index: String,
    lines: Vec<String>,
}

fn battles(gen: u8) -> Vec<Battle> {
    let body = read_gz(&format!("gen{gen}.log.gz"));
    let mut out: Vec<Battle> = Vec::new();
    for line in body.lines() {
        if let Some(rest) = line.strip_prefix("#battle ") {
            let index = rest.split_whitespace().find_map(|w| w.strip_prefix("index=")).unwrap_or("?");
            out.push(Battle { index: index.to_string(), lines: Vec::new() });
        } else if let Some(b) = out.last_mut() {
            b.lines.push(line.to_string());
        }
    }
    out
}

/// The `team` string of a `>player` command.
fn team_of(cmd: &str) -> String {
    let start = cmd.find("\"team\":\"").unwrap() + "\"team\":\"".len();
    let mut out = String::new();
    let mut chars = cmd[start..].chars();
    loop {
        match chars.next().unwrap() {
            '"' => return out,
            '\\' => match chars.next().unwrap() {
                'u' => {
                    let hex: String = chars.by_ref().take(4).collect();
                    out.push(char::from_u32(u32::from_str_radix(&hex, 16).unwrap()).unwrap());
                }
                c => out.push(c),
            },
            c => out.push(c),
        }
    }
}

/// What playing one battle through the client produced.
#[derive(Default)]
struct Run {
    /// The board stream, one rendered event per line, with the protocol
    /// line that caused it before each group.
    stream: String,
    requests: usize,
    choices: usize,
    failures: Vec<String>,
    /// Move requests whose active PP the view had tracked differently.
    pp_mismatches: Vec<String>,
}

fn play(dex: &'static Dex, battle: &Battle) -> Run {
    let tables = tables(dex);
    let mut codec = CodecState::new(tables);
    let mut view = BattleView::new(dex);
    let mut run = Run::default();
    let mut last_request: [Option<Request>; 2] = [None, None];
    let mut seat_options: [Option<SeatOptions>; 2] = [None, None];

    for (n, raw) in battle.lines.iter().enumerate() {
        let line_no = n + 1;
        let fail = |run: &mut Run, what: String| {
            run.failures.push(format!("battle {} line {}: {}\n    {}", battle.index, line_no, what, raw))
        };
        if raw.is_empty() {
            continue;
        }
        if let Some(cmd) = raw.strip_prefix('>') {
            if let Some(rest) = cmd.strip_prefix("player ") {
                let side = SideId::parse(&rest[..2]).unwrap();
                let team = unpack(&team_of(rest)).unwrap();
                for (slot, set) in team.iter().enumerate() {
                    codec.battle_mut().set_nickname(side, set.name.as_str());
                    view.set_nickname(side, slot as u8, set.name.as_str());
                }
            } else if let Some(side) = cmd.get(..2).and_then(SideId::parse) {
                // `>p1 move 3`: the recorded choice for the side's request.
                let ps = cmd[3..].trim();
                let Some(req) = &last_request[side.index()] else {
                    fail(&mut run, "a choice with no request".into());
                    continue;
                };
                let answer = device_answer(req, ps);
                match choice_string(req, &answer) {
                    Some(back) if back == ps => run.choices += 1,
                    other => fail(&mut run, format!("{answer:?} translates to {other:?}")),
                }
                if let Some(opts) = &seat_options[side.index()] {
                    if let Err(e) = allowed(opts, &answer) {
                        fail(&mut run, format!("{answer:?} not allowed by the seat options: {e}"));
                    }
                }
            }
            continue;
        }
        if let Some(json) = raw.strip_prefix("|request|") {
            let (dex_names, names) = codec.split_mut();
            let req = match decode_request(json, dex_names, names) {
                Ok(r) => r,
                Err(e) => {
                    fail(&mut run, format!("request: {e}"));
                    continue;
                }
            };
            run.requests += 1;
            if let Err(e) = agrees(&view, &req) {
                fail(&mut run, e);
            }
            if let Some(e) = pp_differs(&view, &req) {
                run.pp_mismatches.push(format!("battle {} line {}: {e}", battle.index, line_no));
            }
            let events = view.apply_request(&req);
            render(&mut run.stream, raw, &events, true);
            let side = req.side().id;
            seat_options[side.index()] = slot_options(&view, &req);
            last_request[side.index()] = Some(req);
            continue;
        }
        match decode_line(raw, &mut codec) {
            Ok(msg) => {
                let events = view.apply(&msg);
                render(&mut run.stream, raw, &events, false);
            }
            Err(e) => fail(&mut run, format!("decode: {e}")),
        }
    }
    run
}

/// A recorded PS choice in the device's grammar (0-based move slot, team
/// slot), which [`choice_string`] must turn back into the same PS text.
fn device_answer(req: &Request, ps: &str) -> String {
    let mut w = ps.split_whitespace();
    match (w.next(), w.next().and_then(|n| n.parse::<usize>().ok())) {
        (Some("move"), Some(n)) => format!("move {}", n - 1),
        (Some("switch"), Some(n)) => format!("switch {}", req.side().pokemon[n - 1].ident.slot),
        _ => ps.to_string(),
    }
}

fn allowed(o: &SeatOptions, answer: &str) -> Result<(), String> {
    if let Some(auto) = &o.auto {
        return if auto == answer { Ok(()) } else { Err(format!("auto is {auto:?}")) };
    }
    let mut w = answer.split_whitespace();
    match (w.next(), w.next().and_then(|n| n.parse::<usize>().ok())) {
        (Some("move"), Some(k)) if !o.forced_switch => {
            if k < o.n_moves && o.usable[k] {
                Ok(())
            } else {
                Err(format!("move {k} of {} usable {:?}", o.n_moves, o.usable))
            }
        }
        (Some("switch"), Some(t)) => {
            if o.trapped && !o.forced_switch {
                Err("trapped".into())
            } else if o.party_ok[t] {
                Ok(())
            } else {
                Err(format!("party_ok {:?}", o.party_ok))
            }
        }
        _ => Err("unexpected answer".into()),
    }
}

/// The active mon's PP as the view tracked it from `|move|` lines, against
/// the request's.
fn pp_differs(view: &BattleView, req: &Request) -> Option<String> {
    let Request::Move(r) = req else { return None };
    let a = r.active.first()?;
    let (_, m) = view.active(req.side().id)?;
    if a.moves.iter().any(|x| x.pp.is_none()) || m.moves.len() != a.moves.len() {
        return None;
    }
    let mine: Vec<u8> = m.moves.iter().map(|k| k.pp).collect();
    let theirs: Vec<u8> = a.moves.iter().map(|x| x.pp.unwrap().max(0) as u8).collect();
    (mine != theirs).then(|| format!("view PP {mine:?}, request {theirs:?}"))
}

/// The view, built from protocol lines alone, against what `req` reports.
fn agrees(view: &BattleView, req: &Request) -> Result<(), String> {
    let side = req.side().id;
    for p in req.side().pokemon.iter() {
        let ident = PokemonIdent::inactive(side, p.ident.slot);
        let m = view.mon(ident).copied().unwrap_or_default();
        let c = p.condition;
        let who = view.nickname(ident);
        if p.active && view.side(side).active != Some(p.ident.slot) {
            return Err(format!("{side:?} active is slot {:?}, the request says {who}", view.side(side).active));
        }
        if !p.active && view.side(side).active == Some(p.ident.slot) {
            return Err(format!("{side:?} {who} is active in the view, not in the request"));
        }
        if c.status == Some(Status::Fnt) {
            if !m.fainted {
                return Err(format!("{who} fainted, the view has {:?} {:?}", m.hp, m.status));
            }
            continue;
        }
        if m.hp.max == 0 {
            // Never seen: it must be as it was sent.
            if c.hp.cur != c.hp.max || c.status.is_some() {
                return Err(format!("{who} changed unseen: {c:?}"));
            }
            continue;
        }
        if m.fainted || m.hp != c.hp || m.status != c.status {
            return Err(format!(
                "{who}: view {}/{} {:?} fainted={}, request {}/{} {:?}",
                m.hp.cur, m.hp.max, m.status, m.fainted, c.hp.cur, c.hp.max, c.status
            ));
        }
    }
    Ok(())
}

fn render(out: &mut String, cause: &str, events: &[BoardEvent], request: bool) {
    if events.is_empty() {
        return;
    }
    let cause = if request {
        "|request|...".to_string()
    } else {
        cause.chars().take(100).collect()
    };
    writeln!(out, "{cause}").unwrap();
    for e in events {
        writeln!(out, "    {}", fmt_event(e)).unwrap();
    }
}

/// A compact, stable rendering of one event.
fn fmt_event(e: &BoardEvent) -> String {
    let moves = |ms: &[ps_client::MoveSlot]| {
        ms.iter()
            .map(|m| {
                format!(
                    "{} {} {} {}/{} {}/{}",
                    m.name,
                    m.type_name,
                    m.category,
                    m.power.map(|p| p.to_string()).unwrap_or("-".into()),
                    m.accuracy.map(|p| p.to_string()).unwrap_or("-".into()),
                    m.pp,
                    m.max_pp
                )
            })
            .collect::<Vec<_>>()
            .join("; ")
    };
    match e {
        BoardEvent::SwitchIn { name, species, player_id, team_slot, moves: ms, speed } => format!(
            "SwitchIn {} name={name} species={species:?} slot={team_slot:?} speed={speed:?} moves=[{}]",
            player_id.as_deref().unwrap_or("?"),
            moves(ms)
        ),
        BoardEvent::MovesUpdate { player_id, moves: ms } => format!("MovesUpdate {player_id} [{}]", moves(ms)),
        other => format!("{other:?}"),
    }
}

fn check_corpus(gen: u8, dex: &'static Dex) {
    let mut failures = Vec::new();
    let (mut requests, mut choices) = (0, 0);
    let mut pp = Vec::new();
    let all = battles(gen);
    for b in &all {
        let run = play(dex, b);
        requests += run.requests;
        choices += run.choices;
        failures.extend(run.failures);
        pp.extend(run.pp_mismatches);
    }
    eprintln!("gen {gen}: {} move requests where the tracked PP differed from the request's", pp.len());
    for p in pp.iter().take(10) {
        eprintln!("  {p}");
    }
    eprintln!(
        "gen {gen}: {} battles, {requests} requests agreed, {choices} choices round-tripped, {} failures",
        all.len(),
        failures.len()
    );
    for f in failures.iter().take(20) {
        eprintln!("  {f}");
    }
    assert_eq!(all.len(), 20);
    assert!(requests > 1000 && choices > 1000);
    assert!(failures.is_empty(), "gen {gen}: {} failures", failures.len());
    // Exact on this corpus. PP a PS client cannot see (a Pressure never
    // announced to it, Spikes' `pressureTarget`) would show up here; the
    // request corrects it either way, so a new miss is worth a look rather
    // than a hard rule, but none is expected today.
    assert!(pp.is_empty(), "gen {gen}: tracked PP differed {} times", pp.len());
}

#[test]
fn gen1_requests_agree_with_the_view() {
    check_corpus(1, &ps_dex::GEN1);
}

#[test]
fn gen3_requests_agree_with_the_view() {
    check_corpus(3, &ps_dex::GEN3);
}

fn golden(gen: u8, dex: &'static Dex, index: &str) {
    let all = battles(gen);
    let b = all.iter().find(|b| b.index == index).unwrap();
    let run = play(dex, b);
    assert!(run.failures.is_empty(), "{:?}", run.failures);
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/golden/gen{gen}_battle{index}.txt"));
    let header = format!(
        "# BoardEvent stream of gen{gen}.log.gz battle index={index}: each protocol line that\n\
         # yields events, then its events. `|request|...` is a request's prompt events.\n"
    );
    let text = header + &run.stream;
    if std::env::var_os("PS_CLIENT_BLESS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &text).unwrap();
        return;
    }
    let want = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e} (PS_CLIENT_BLESS=1 writes it)", path.display()));
    if want != text {
        let first = want.lines().zip(text.lines()).position(|(a, b)| a != b);
        panic!("{} differs at line {:?}", path.display(), first.map(|i| i + 1));
    }
}

#[test]
fn golden_gen1_tie_and_effects() {
    golden(1, &ps_dex::GEN1, "22");
}

#[test]
fn golden_gen1_transform() {
    golden(1, &ps_dex::GEN1, "39");
}

#[test]
fn golden_gen1_charge_moves() {
    golden(1, &ps_dex::GEN1, "19");
}

#[test]
fn golden_gen3_drag_hazards_transform() {
    golden(3, &ps_dex::GEN3, "25");
}

#[test]
fn golden_gen3_weather_sethp() {
    golden(3, &ps_dex::GEN3, "48");
}

#[test]
fn golden_gen3_setboost() {
    golden(3, &ps_dex::GEN3, "2");
}
