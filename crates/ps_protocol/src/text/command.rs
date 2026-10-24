//! `>start`, `>player` and `>pN` command lines.
//!
//! PS: `sim/battle-stream.ts` (`_writeLine`, `_writeLines`), `sim/side.ts`
//! (`choose`, `chooseMove`, `chooseSwitch`, `choosePass`, `autoChoose`),
//! `sim/battle.ts` (`setPlayer`, `choose`, `undoChoice`), `sim/teams.ts`
//! (`pack`), `sim/SIM-PROTOCOL.md` ("Sending decisions").
//!
//! A command line is the text of the stream, `>` included:
//! [`encode_command`] writes the bytes PS reads and [`decode_command`]
//! reads them back. [`decode_cmd_line`] reads any `>` line, including the
//! recorder's block separators ([`BlockHeader`]), which are not commands.
//!
//! # The commands
//!
//! PS's `_writeLine` switches on the word after the `>`:
//!
//! | Line | [`Command`] |
//! |---|---|
//! | `>start {"formatid":"gen3customgame","seed":[1,2,3,4]}` | [`Command::Start`] |
//! | `>player p1 {"name":"Alice","team":"<packed>"}` | [`Command::Player`] |
//! | `>p1 move 2` | [`Command::Choose`] with [`Choice::Move`] |
//! | `>p1 move thunderbolt` | [`Command::Choose`] with [`Choice::MoveName`] |
//! | `>p1 switch 3` | [`Command::Choose`] with [`Choice::Switch`] |
//! | `>p1 switch pikachu` | [`Command::Choose`] with [`Choice::SwitchName`] |
//! | `>p1 pass` | [`Command::Choose`] with [`Choice::Pass`] |
//! | `>p1 default` | [`Command::Choose`] with [`Choice::Default`] |
//! | `>p1 undo` | [`Command::Choose`] with [`Choice::Undo`] |
//! | `>forcewin p1` | [`Command::ForceWin`] |
//! | `>forcetie` | [`Command::ForceTie`] |
//!
//! PS accepts more (`>p3`, `>p4`, `>forcelose`, `>reseed`, `>tiebreak`,
//! `>chat`, `>eval`, `>requestlog`, ...). The codec refuses those with an
//! error that names them, so a caller can tell "not a command we know"
//! from "a command we know but do not model". `>chat` is the one that
//! would need a protocol message (`|chat|`) to exist before it can be
//! carried, so it is refused by name.
//!
//! Two PS shapes the codec cannot hold are refused as well: a `>player`
//! without a team (PS would generate a random one) and a `>start` whose
//! seed is not four 16-bit numbers.
//!
//! # The choices
//!
//! PS's `side.choose` splits the line on `,` (one chunk per active
//! Pokemon), then on the first space (the verb and its data), and reads
//! the verb: `move`, `switch`, `shift`, `team`, `pass`/`skip`,
//! `auto`/`default`. `>pN undo` never reaches `choose`: PS's
//! `_writeLine` takes it for `undoChoice`.
//!
//! The device plays singles, so the codec models one choice per line and
//! refuses a line with a `,` in it. It also refuses PS's synonyms `skip`
//! and `auto` rather than rewriting them, so that decoding a line and
//! encoding it again gives the same bytes for every line it accepts.

use alloc::format;
use alloc::string::{String, ToString};

use crate::command::{Choice, Command, FormatSpec};
use crate::ident::SideId;
use crate::team::PackedTeam;
use crate::Name;

use super::json::{parse_value, push_json_str};
use super::line::CodecError;
use super::team::{pack, unpack};

/// The block separators the recorder writes between PS's output blocks.
///
/// PS's `BattleStream` pushes its output as `update` and `sideupdate`
/// blocks. `tools/ps-diff/record-logs.js` writes the kind of the block on
/// a line of its own, prefixed with `>` so the transcript says which
/// block the lines below it belong to. They are not commands (PS's
/// `_writeLine` has no case for them), so they get their own type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockHeader {
    /// `>update`: an update block.
    Update,
    /// `>sideupdate pN`: a side's update block.
    SideUpdate(SideId),
}

/// One `>` line: a PS command or a block separator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CmdLine {
    /// A PS command.
    Command(Command),
    /// A block separator.
    Block(BlockHeader),
}

/// Encodes one [`Command`] as a `BattleStream` line (PS's `_writeLine`).
pub fn encode_command(cmd: &Command) -> Result<String, CodecError> {
    Ok(encode_command_text(cmd))
}

/// The bytes of a command line, without the `Result` the public encoder
/// carries.
fn encode_command_text(cmd: &Command) -> String {
    let mut s = String::new();
    match cmd {
        Command::Start { format, seed } => {
            s.push_str(">start ");
            s.push_str(&start_json(format, seed));
        }
        Command::Player { side, name, team } => {
            s.push_str(">player ");
            s.push_str(side.as_str());
            s.push(' ');
            s.push_str(&player_json(name, team));
        }
        Command::Choose { side, choice } => {
            s.push('>');
            s.push_str(side.as_str());
            s.push(' ');
            s.push_str(&choice_text(*choice));
        }
        Command::ForceWin(side) => {
            s.push_str(">forcewin ");
            s.push_str(side.as_str());
        }
        Command::ForceTie => s.push_str(">forcetie"),
    }
    s
}

/// `{"formatid":"...","seed":[a,b,c,d]}`: the two keys PS's `>start`
/// lines carry, in PS's order.
fn start_json(spec: &FormatSpec, seed: &[u16; 4]) -> String {
    let mut s = String::new();
    s.push_str("{\"formatid\":");
    push_json_str(&mut s, &format_id_str(spec));
    s.push_str(",\"seed\":[");
    for (i, n) in seed.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&n.to_string());
    }
    s.push_str("]}");
    s
}

fn format_id_str(spec: &FormatSpec) -> String {
    let mut s = String::new();
    spec.write_id(|p| s.push_str(p));
    s
}

/// `{"name":"...","team":"..."}`: the two keys PS's `>player` lines carry,
/// in PS's order. PS's `setPlayer` also reads `avatar` and `rating`, but
/// no recorded line has them and the device has neither, so the encoder
/// writes exactly the keys PS records.
fn player_json(name: &Name, team: &PackedTeam) -> String {
    let mut s = String::new();
    s.push_str("{\"name\":");
    push_json_str(&mut s, name.as_str());
    s.push_str(",\"team\":");
    push_json_str(&mut s, &pack(team));
    s.push('}');
    s
}

/// The choice part of a `>pN` line.
fn choice_text(c: Choice) -> String {
    match c {
        Choice::Move(n) => format!("move {n}"),
        Choice::MoveName(n) => format!("move {}", n.as_str()),
        Choice::Switch(n) => format!("switch {n}"),
        Choice::SwitchName(n) => format!("switch {}", n.as_str()),
        Choice::Pass => String::from("pass"),
        Choice::Default => String::from("default"),
        Choice::Undo => String::from("undo"),
    }
}

/// Decodes one command line, `>` included (PS's `_writeLine`).
pub fn decode_command(line: &str) -> Result<Command, CodecError> {
    let rest = strip_prompt(line)?;
    let (kind, rest) = split_first(rest, ' ').unwrap_or((rest, ""));
    match kind {
        "start" => decode_start(rest),
        "player" => decode_player(rest),
        "p1" | "p2" => decode_choose(kind, rest),
        "forcewin" => {
            let side = SideId::parse(rest)
                .ok_or_else(|| CodecError::BadCmd(format!(">forcewin needs a side, got {:?}", rest)))?;
            Ok(Command::ForceWin(side))
        }
        "forcetie" => {
            if !rest.is_empty() {
                return Err(CodecError::BadCmd(format!(">forcetie takes no argument, got {:?}", rest)));
            }
            Ok(Command::ForceTie)
        }
        "chat" => Err(CodecError::BadCmd(
            ">chat needs a |chat| protocol message, which does not exist yet".into(),
        )),
        other => Err(CodecError::BadCmd(format!("unknown command >{}", other))),
    }
}

/// `>start {"formatid":"...","seed":[a,b,c,d]}`.
fn decode_start(rest: &str) -> Result<Command, CodecError> {
    let v = parse_value(rest)?;
    let formatid = v
        .get("formatid")
        .ok_or_else(|| CodecError::BadCmd(">start without a formatid".into()))?
        .string("formatid")?;
    let format = FormatSpec::parse(formatid)
        .ok_or_else(|| CodecError::BadCmd(format!(">start formatid {:?}", formatid)))?;
    let seed_json = v
        .get("seed")
        .ok_or_else(|| CodecError::BadCmd(">start without a seed".into()))?
        .arr("seed")?;
    if seed_json.len() != 4 {
        return Err(CodecError::BadCmd(format!(">start seed has {} entries, want 4", seed_json.len())));
    }
    let mut seed = [0u16; 4];
    for (i, n) in seed_json.iter().enumerate() {
        let n = n.uint("seed")?;
        seed[i] = u16::try_from(n).map_err(|_| CodecError::BadCmd(format!(">start seed entry {}", n)))?;
    }
    Ok(Command::Start { format, seed })
}

/// `>player pN {"name":"...","team":"<packed>"}`.
fn decode_player(rest: &str) -> Result<Command, CodecError> {
    let (slot, json) = split_first(rest, ' ')
        .ok_or_else(|| CodecError::BadCmd(format!(">player without a slot: {:?}", rest)))?;
    let side = SideId::parse(slot)
        .ok_or_else(|| CodecError::BadCmd(format!(">player slot {:?}", slot)))?;
    let v = parse_value(json)?;
    // PS's `setPlayer` reads `name`, `team`, `avatar` and `rating`; the
    // device has no avatar and no rating, so only the two keys PS's
    // recorded lines carry are read.
    let name = match v.get("name") {
        Some(n) => n.string("name")?,
        None => "",
    };
    let name = if name.is_empty() { default_player_name(side) } else { name };
    let name = Name::from_str(name)
        .map_err(|_| CodecError::BadCmd(format!(">player name too long: {:?}", name)))?;
    let packed = v
        .get("team")
        .ok_or_else(|| CodecError::BadCmd(">player without a team".into()))?
        .string("team")?;
    let team = unpack(packed).map_err(|e| CodecError::BadCmd(format!(">player team: {}", e)))?;
    Ok(Command::Player { side, name, team })
}

/// PS's `setPlayer` names a side without a name `Player ${slotNum + 1}`.
fn default_player_name(side: SideId) -> &'static str {
    match side {
        SideId::P1 => "Player 1",
        SideId::P2 => "Player 2",
    }
}

/// `>pN <choice>`: PS's `battle.choose` -> `side.choose`.
fn decode_choose(side_kind: &str, rest: &str) -> Result<Command, CodecError> {
    let side = SideId::parse(side_kind)
        .ok_or_else(|| CodecError::BadCmd(format!(">p slot {:?}", side_kind)))?;
    // PS's `choose` splits the line on `,` first: one chunk per active
    // Pokemon. The device plays singles, so a line with more than one
    // chunk is not modelled.
    if rest.contains(',') {
        return Err(CodecError::BadCmd(format!(">{} has more than one choice: {:?}", side_kind, rest)));
    }
    let choice = parse_choice(rest)?;
    Ok(Command::Choose { side, choice })
}

/// One choice chunk: PS's `side.choose` trims the chunk, splits it on the
/// first space, then trims the data, and reads the verb.
fn parse_choice(chunk: &str) -> Result<Choice, CodecError> {
    let chunk = chunk.trim();
    let (kind, data) = match split_first(chunk, ' ') {
        Some((k, d)) => (k, d.trim()),
        None => (chunk, ""),
    };
    match kind {
        "move" => {
            if data.is_empty() {
                return Err(CodecError::BadCmd(">move without a move".into()));
            }
            match slot_arg(data) {
                SlotArg::Number(n) => Ok(Choice::Move(n)),
                SlotArg::TooBig => Err(CodecError::BadCmd(format!(">move {} does not fit a slot", data))),
                SlotArg::Name => Ok(Choice::MoveName(name_arg(data)?)),
            }
        }
        "switch" => {
            if data.is_empty() {
                return Err(CodecError::BadCmd(">switch without a Pokemon".into()));
            }
            match slot_arg(data) {
                SlotArg::Number(n) => Ok(Choice::Switch(n)),
                SlotArg::TooBig => Err(CodecError::BadCmd(format!(">switch {} does not fit a slot", data))),
                SlotArg::Name => Ok(Choice::SwitchName(name_arg(data)?)),
            }
        }
        "pass" => {
            if !data.is_empty() {
                return Err(CodecError::BadCmd(format!(">pass takes no argument, got {:?}", data)));
            }
            Ok(Choice::Pass)
        }
        "default" => {
            if !data.is_empty() {
                return Err(CodecError::BadCmd(format!(">default takes no argument, got {:?}", data)));
            }
            Ok(Choice::Default)
        }
        "undo" => {
            // PS's `_writeLine` takes `undo` for `undoChoice` before the
            // choice ever reaches `side.choose`.
            if !data.is_empty() {
                return Err(CodecError::BadCmd(format!(">undo takes no argument, got {:?}", data)));
            }
            Ok(Choice::Undo)
        }
        "skip" => Err(CodecError::BadCmd(
            ">skip is PS's synonym for >pass; the codec models PS's own form".into(),
        )),
        "auto" => Err(CodecError::BadCmd(
            ">auto is PS's synonym for >default; the codec models PS's own form".into(),
        )),
        "shift" => Err(CodecError::BadCmd(">shift is a triples choice".into())),
        "team" => Err(CodecError::BadCmd(">team is a Team Preview choice".into())),
        other => Err(CodecError::BadCmd(format!(">{} is not a choice", other))),
    }
}

/// What a choice's data is: a slot number, a number the type cannot hold,
/// or a name.
enum SlotArg {
    /// A 1-based slot number.
    Number(u8),
    /// A number too big for the type.
    TooBig,
    /// A name.
    Name,
}

/// A slot number: PS's `chooseMove` reads a move number with
/// `/^[0-9]+$/`, and `chooseSwitch` with `parseInt` (which is as
/// lenient). The codec takes the strict form: a run of digits is a
/// number, anything else is a name.
///
/// The number is 1-based, as PS writes it, and PS answers an out-of-range
/// one with its own error, so `move 0` and `switch 0` are passed on as
/// written. A number too big for the type is refused: the codec cannot
/// carry it, and PS's answer for it ("you do not have a Pokemon in slot
/// N") needs a number the type does not hold.
fn slot_arg(s: &str) -> SlotArg {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return SlotArg::Name;
    }
    match u8::try_from(s.parse::<u32>().unwrap_or(u32::MAX)) {
        Ok(n) => SlotArg::Number(n),
        Err(_) => SlotArg::TooBig,
    }
}

/// The name of a move or a Pokemon, as the line wrote it.
fn name_arg(s: &str) -> Result<Name, CodecError> {
    Name::from_str(s).map_err(|_| CodecError::BadCmd(format!("name too long: {:?}", s)))
}

/// The text after PS's `>` prompt.
fn strip_prompt(line: &str) -> Result<&str, CodecError> {
    line.strip_prefix('>').ok_or_else(|| CodecError::BadCmd(format!("no > prompt: {:?}", line)))
}

/// Encodes a block separator.
pub fn encode_block_header(header: &BlockHeader) -> String {
    match header {
        BlockHeader::Update => String::from(">update"),
        BlockHeader::SideUpdate(side) => format!(">sideupdate {}", side.as_str()),
    }
}

/// Decodes a block separator: `>update` or `>sideupdate pN`.
pub fn decode_block_header(line: &str) -> Result<BlockHeader, CodecError> {
    let rest = strip_prompt(line)?;
    let (kind, rest) = split_first(rest, ' ').unwrap_or((rest, ""));
    match kind {
        "update" => {
            if !rest.is_empty() {
                return Err(CodecError::BadCmd(format!(">update takes no argument, got {:?}", rest)));
            }
            Ok(BlockHeader::Update)
        }
        "sideupdate" => {
            let side = SideId::parse(rest)
                .ok_or_else(|| CodecError::BadCmd(format!(">sideupdate needs a side, got {:?}", rest)))?;
            Ok(BlockHeader::SideUpdate(side))
        }
        other => Err(CodecError::BadCmd(format!("not a block separator: >{}", other))),
    }
}

/// Decodes any `>` line: a PS command or a block separator.
pub fn decode_cmd_line(line: &str) -> Result<CmdLine, CodecError> {
    match decode_command(line) {
        Ok(cmd) => Ok(CmdLine::Command(cmd)),
        Err(cmd_err) => match decode_block_header(line) {
            Ok(header) => Ok(CmdLine::Block(header)),
            Err(_) => Err(cmd_err),
        },
    }
}

/// Encodes any `>` line: a PS command or a block separator.
pub fn encode_cmd_line(line: &CmdLine) -> String {
    match line {
        CmdLine::Command(cmd) => encode_command_text(cmd),
        CmdLine::Block(header) => encode_block_header(header),
    }
}

/// The text after the first `sep`, and the text before it.
fn split_first(s: &str, sep: char) -> Option<(&str, &str)> {
    let i = s.find(sep)?;
    Some((&s[..i], &s[i + sep.len_utf8()..]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::{FormatId, RuleId};

    fn known_team() -> PackedTeam {
        // A two-mon team: Pikachu with moves, item, ability, gender and
        // shiny; Snorlax plain. PS's order: name|species|item|ability|moves|
        // nature|evs|gender|ivs|shiny|level|happiness (11 separators,
        // 12 fields). PS only writes `0` as empty in evs and `31` as empty
        // in ivs.
        let line = "Pika|Pikachu|LightBall|Static|thunderbolt,surf|Timid|,,,252,4,252|F||S|50|]Snorlax||Leftovers|ThickFat|bodyslam|||||||";
        unpack(line).unwrap()
    }

    #[test]
    fn start_encode_decode_round_trip() {
        let cmd = Command::Start {
            format: FormatSpec::gen1_harness(),
            seed: [2535, 52155, 27866, 24143],
        };
        let s = encode_command(&cmd).unwrap();
        assert_eq!(s, ">start {\"formatid\":\"gen1customgame@@@Sleep Clause Mod,Freeze Clause Mod\",\"seed\":[2535,52155,27866,24143]}");
        assert_eq!(decode_command(&s).unwrap(), cmd);
    }

    #[test]
    fn start_with_no_custom_rules_round_trips() {
        let cmd = Command::Start { format: FormatSpec::gen3_harness(), seed: [1, 2, 3, 4] };
        let s = encode_command(&cmd).unwrap();
        assert_eq!(s, ">start {\"formatid\":\"gen3customgame\",\"seed\":[1,2,3,4]}");
        assert_eq!(decode_command(&s).unwrap(), cmd);
    }

    #[test]
    fn start_needs_a_format_and_four_seed_numbers() {
        assert!(decode_command(">start {\"formatid\":\"gen3customgame\",\"seed\":[1,2,3]}").is_err());
        assert!(decode_command(">start {\"formatid\":\"gen3customgame\",\"seed\":[1,2,3,4,5]}").is_err());
        assert!(decode_command(">start {\"formatid\":\"gen3customgame\"}").is_err());
        assert!(decode_command(">start {\"formatid\":\"gen9ou\",\"seed\":[1,2,3,4]}").is_err());
        // The reader is PS's: the keys may come in any order.
        let cmd = decode_command(">start {\"seed\":[1,2,3,4],\"formatid\":\"gen3customgame\"}").unwrap();
        assert_eq!(cmd, Command::Start { format: FormatSpec::gen3_harness(), seed: [1, 2, 3, 4] });
    }

    #[test]
    fn player_encode_writes_only_name_and_team_keys() {
        let team = known_team();
        let cmd = Command::Player {
            side: SideId::P2,
            name: Name::from_str("Player 2").unwrap(),
            team,
        };
        let s = encode_command(&cmd).unwrap();
        assert!(s.starts_with(">player p2 {\"name\":\"Player 2\",\"team\":\""));
        assert!(s.ends_with("\"}"));
        // PS's recording has no avatar/rating keys.
        assert!(!s.contains("avatar"));
        assert!(!s.contains("rating"));
        assert_eq!(decode_command(&s).unwrap(), cmd);
    }

    #[test]
    fn player_round_trips_recorded_teams_with_quoted_unicode() {
        // `Farfetch’d` is a real PS-recorded pkg of name `Farfetch’d`.
        let name = "Farfetch\u{2019}d";
        let cmd_str = format!(
            ">player p2 {{\"name\":\"Player 2\",\"team\":\"{}|||NoAbility|agility,slash,bodyslam,swordsdance||255,255,255,255,255,255||30,30,30,30,30,30||78|\"}}",
            name
        );
        let cmd = decode_command(&cmd_str).unwrap();
        match &cmd {
            Command::Player { side, name, .. } => {
                assert_eq!(*side, SideId::P2);
                assert_eq!(name.as_str(), "Player 2");
            }
            _ => panic!(),
        }
        let re = encode_command(&cmd).unwrap();
        assert_eq!(re, cmd_str);
    }

    #[test]
    fn player_without_a_name_gets_pss_default() {
        // PS's `setPlayer`: `options.name || 'Player ' + (slotNum + 1)`.
        let team = known_team();
        let cmd = decode_command(&format!(
            ">player p1 {{\"team\":\"{}\"}}",
            pack(&team)
        ))
        .unwrap();
        match &cmd {
            Command::Player { name, .. } => assert_eq!(name.as_str(), "Player 1"),
            _ => panic!(),
        }
    }

    #[test]
    fn player_without_a_team_is_refused() {
        assert!(decode_command(">player p1 {\"name\":\"Player 1\"}").is_err());
        assert!(decode_command(">player p1").is_err());
        assert!(decode_command(">player p3 {\"name\":\"x\",\"team\":\"y\"}").is_err());
    }

    #[test]
    fn choice_numeric_round_trips() {
        let cases = [
            (Choice::Move(3), ">p1 move 3"),
            (Choice::Switch(2), ">p1 switch 2"),
            (Choice::Pass, ">p1 pass"),
            (Choice::Default, ">p1 default"),
            (Choice::Undo, ">p1 undo"),
        ];
        for (c, want) in cases {
            let cmd = Command::Choose { side: SideId::P1, choice: c };
            let s = encode_command(&cmd).unwrap();
            assert_eq!(s, want);
            assert_eq!(decode_command(&s).unwrap(), cmd);
        }
    }

    #[test]
    fn choice_by_name_round_trips() {        let cmd = Command::Choose {
            side: SideId::P2,
            choice: Choice::MoveName(Name::from_str("thunderbolt").unwrap()),
        };
        let s = encode_command(&cmd).unwrap();
        assert_eq!(s, ">p2 move thunderbolt");
        assert_eq!(decode_command(&s).unwrap(), cmd);

        let cmd = Command::Choose {
            side: SideId::P1,
            choice: Choice::SwitchName(Name::from_str("pikachu").unwrap()),
        };
        let s = encode_command(&cmd).unwrap();
        assert_eq!(s, ">p1 switch pikachu");
        assert_eq!(decode_command(&s).unwrap(), cmd);
    }

    #[test]
    fn ps_trims_the_choice_chunk() {
        // PS's `side.choose` trims the chunk before it reads the verb.
        let cmd = Command::Choose { side: SideId::P1, choice: Choice::Move(1) };
        assert_eq!(decode_command(">p1  move 1 ").unwrap(), cmd);
        assert_eq!(decode_command(">p1 move 1").unwrap(), cmd);
    }

    #[test]
    fn out_of_range_slots_are_pss_business() {
        // PS answers `move 0` with "you don't have a move 0" and `switch 0`
        // with a name lookup that never matches; the codec passes them on.
        assert_eq!(
            decode_command(">p1 move 0").unwrap(),
            Command::Choose { side: SideId::P1, choice: Choice::Move(0) }
        );
        assert_eq!(
            decode_command(">p1 switch 0").unwrap(),
            Command::Choose { side: SideId::P1, choice: Choice::Switch(0) }
        );
        // A number the type cannot hold is refused.
        assert!(decode_command(">p1 move 256").is_err());
        assert!(decode_command(">p1 switch 256").is_err());
    }

    #[test]
    fn forcewin_and_forcetie_round_trip() {
        let cmd = Command::ForceWin(SideId::P1);
        let s = encode_command(&cmd).unwrap();
        assert_eq!(s, ">forcewin p1");
        assert_eq!(decode_command(&s).unwrap(), cmd);

        let cmd = Command::ForceTie;
        let s = encode_command(&cmd).unwrap();
        assert_eq!(s, ">forcetie");
        assert_eq!(decode_command(&s).unwrap(), cmd);

        // Wrong shape is refused.
        assert!(decode_command(">forcewin").is_err());
        assert!(decode_command(">forcetie p1").is_err());
    }

    #[test]
    fn unknown_command_is_rejected() {
        assert!(decode_command("update").is_err()); // no > prompt
        assert!(decode_command(">update").is_err()); // a block separator
        assert!(decode_command(">nope hello").is_err());
        assert!(decode_command(">p1 team 1,2,3").is_err()); // Team Preview out of scope
        assert!(decode_command(">p1 shift").is_err());
        assert!(decode_command(">p1 skip").is_err()); // PS's synonym for pass
        assert!(decode_command(">p1 auto").is_err()); // PS's synonym for default
        assert!(decode_command(">p1 pass hello").is_err());
        // Multi-choice (doubles) refused in this singles-only codec.
        assert!(decode_command(">p1 move 1, switch 2").is_err());
        // PS's other commands are refused by name.
        assert!(decode_command(">chat hello").is_err());
        assert!(decode_command(">forcelose p1").is_err());
    }

    #[test]
    fn block_headers_round_trip() {
        for (line, header) in [
            (">update", BlockHeader::Update),
            (">sideupdate p1", BlockHeader::SideUpdate(SideId::P1)),
            (">sideupdate p2", BlockHeader::SideUpdate(SideId::P2)),
        ] {
            assert_eq!(decode_block_header(line).unwrap(), header);
            assert_eq!(encode_block_header(&header), line);
            assert_eq!(decode_cmd_line(line).unwrap(), CmdLine::Block(header));
            assert_eq!(encode_cmd_line(&CmdLine::Block(header)), line);
        }
        assert!(decode_block_header(">sideupdate p3").is_err());
        assert!(decode_block_header(">update x").is_err());
    }

    #[test]
    fn cmd_line_reads_commands_and_blocks() {
        let cmd = Command::Choose { side: SideId::P2, choice: Choice::Move(1) };
        let line = encode_command(&cmd).unwrap();
        assert_eq!(decode_cmd_line(&line).unwrap(), CmdLine::Command(cmd.clone()));
        assert_eq!(encode_cmd_line(&CmdLine::Command(cmd)), line);
    }

    #[test]
    fn full_recorded_envelope_round_trips() {
        let start = decode_command(">start {\"formatid\":\"gen1customgame@@@Sleep Clause Mod,Freeze Clause Mod\",\"seed\":[2535,52155,27866,24143]}").unwrap();
        match start {
            Command::Start { format, seed } => {
                assert_eq!(seed, [2535, 52155, 27866, 24143]);
                assert_eq!(format.base, FormatId::Gen1CustomGame);
                assert_eq!(
                    format.custom_rules.iter().map(|c| c.rule).collect::<alloc::vec::Vec<_>>(),
                    alloc::vec![RuleId::SleepClauseMod, RuleId::FreezeClauseMod],
                );
            }
            _ => panic!(),
        }

        let choose = decode_command(">p1 move 3").unwrap();
        assert_eq!(choose, Command::Choose { side: SideId::P1, choice: Choice::Move(3) });
    }
}
