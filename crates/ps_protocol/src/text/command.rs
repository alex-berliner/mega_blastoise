//! `>player`, `>start`, `>p1` line codec.

use alloc::string::{String, ToString};

use crate::command::{Choice, Command, FormatSpec};
use crate::ident::SideId;
use crate::team::PackedTeam;
use crate::{BoundedStr, NAME_CAP};

use super::line::CodecError;

/// Encodes one [`Command`] as a `BattleStream` line (PS's `_writeLine`).
pub fn encode_command(cmd: &Command) -> Result<String, CodecError> {
    let mut s = String::new();
    match cmd {
        Command::Start { format, seed } => {
            s.push_str(">start ");
            s.push_str(&format_to_json(format, seed));
            Ok(s)
        }
        Command::Player { side, name, avatar, rating, team: _ } => {
            let mut buf = String::new();
            buf.push_str(">player ");
            buf.push_str(side.as_str());
            buf.push(' ');
            buf.push_str(&json_string("name"));
            buf.push(':');
            push_json_str(&mut buf, name);
            buf.push(',');
            buf.push_str(&json_string("avatar"));
            buf.push(':');
            push_json_str(&mut buf, avatar);
            buf.push(',');
            buf.push_str(&json_string("rating"));
            buf.push(':');
            push_json_str(&mut buf, rating);
            buf.push(',');
            buf.push_str(&json_string("team"));
            buf.push(':');
            buf.push_str("\"PACKED\""); // placeholder until `team::pack` is wired up
            Ok(buf)
        }
        Command::Choose { side, choice } => {
            s.push('>');
            s.push_str(side.as_str());
            s.push(' ');
            s.push_str(choice_kw(*choice));
            s.push(' ');
            s.push_str(&choice_arg(*choice));
            Ok(s)
        }
        Command::ForceWin(side) => {
            s.push_str(">forcewin ");
            s.push_str(side.as_str());
            Ok(s)
        }
        Command::ForceTie => Ok(">forcetie".to_string()),
    }
}

fn format_to_json(spec: &FormatSpec, seed: &[u16; 4]) -> String {
    let mut s = String::from("{");
    s.push_str(&json_key("formatid"));
    push_quoted(&mut s, &format_id_str(spec));
    s.push(',');
    s.push_str(&json_key("seed"));
    s.push('[');
    for (i, n) in seed.iter().enumerate() {
        if i > 0 { s.push(','); }
        s.push_str(&n.to_string());
    }
    s.push(']');
    s.push('}');
    s
}

fn format_id_str(spec: &FormatSpec) -> String {
    let mut s = String::new();
    spec.write_id(|p| s.push_str(p));
    s
}

fn json_key(key: &str) -> String {
    let mut s = String::with_capacity(key.len() + 4);
    s.push('"');
    s.push_str(key);
    s.push_str("\":");
    s
}

fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    push_quoted(&mut out, s);
    out
}

fn push_quoted(buf: &mut String, s: &str) {
    buf.push('"');
    for c in s.chars() {
        match c {
            '"' => buf.push_str("\\\""),
            '\\' => buf.push_str("\\\\"),
            c => buf.push(c),
        }
    }
    buf.push('"');
}

fn push_json_str(buf: &mut String, val: &BoundedStr<NAME_CAP>) {
    push_quoted(buf, val.as_str());
}

fn choice_kw(c: Choice) -> &'static str {
    match c {
        Choice::Move(_) => "move",
        Choice::Switch(_) => "switch",
        Choice::Pass => "pass",
        Choice::Default => "default",
        Choice::Undo => "undo",
    }
}

fn choice_arg(c: Choice) -> String {
    match c {
        Choice::Move(n) => n.to_string(),
        Choice::Switch(n) => n.to_string(),
        Choice::Pass | Choice::Default | Choice::Undo => String::new(),
    }
}

/// Decodes a `BattleStream` line. Stub for P1-4.
pub fn decode_command(_line: &str) -> Result<Command, CodecError> {
    Err(CodecError::BadCmd("not implemented".into()))
}

// re-exported alias kept for tests
pub type CmdSide = SideId;

// Preserves the `_team` reference visibly so we can wire `team::pack` later.
#[allow(dead_code)]
fn _team_ref(_: &PackedTeam) {}
