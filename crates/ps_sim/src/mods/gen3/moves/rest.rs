//! Rest: the user falls asleep for exactly two turns and heals to full.
//! Fails when it is asleep already, at full HP, or has Insomnia or Vital
//! Spirit. No effect state.
//!
//! PS: data/moves.ts rest (onTry, onHit)

use ps_protocol::{AbilityNum, Arg, ArgValue, Kw, Msg, Word};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::mods::gen3::conditions::slp::TIME;
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::{abilities, conditions};

/// `onTry(source)`.
// PS: data/moves.ts rest.onTry
fn on_try(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let source = a.target.mon()?;
    // Comatose is gen 7: no gen 3 Pokemon has it.
    let comatose = b.dex.ability_id("comatose");
    if b.mon(source).status == conditions::SLP || comatose.is_some_and(|c| b.has_ability(source, c)) {
        return Some(Relay::FAIL);
    }
    if b.mon(source).hp == b.mon(source).maxhp {
        let mon = b.ident(source);
        b.add(Msg::Fail { mon, what: Some(Arg::new(ArgValue::Word(Word::Heal))), what2: None, kw: Kw::NONE });
        return Some(Relay::Null);
    }
    for ability in [abilities::INSOMNIA, abilities::VITALSPIRIT] {
        if b.has_ability(source, ability) {
            // `-fail|SOURCE|[from] ability: NAME|[of] SOURCE`.
            let mon = b.ident(source);
            let from = Arg::new(ArgValue::AbilityEffect(AbilityNum(ability.0)));
            b.add(Msg::Fail { mon, what: None, what2: None, kw: Kw::from_of(from, mon) });
            return Some(Relay::Null);
        }
    }
    None
}

/// `onHit(target, source, move)`: sleep, `time = 3`, and the heal
/// (`startTime` is never read in gen 3).
// PS: data/moves.ts rest.onHit
fn on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let result = b.set_status(target, conditions::SLP, a.source, a.effect, false);
    if !result.truthy() {
        return Some(result);
    }
    b.mon_mut(target).status_state.data[TIME] = 3;
    let maxhp = i32::from(b.mon(target).maxhp);
    b.heal(maxhp, None, None, EffectId::None);
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Try, on_try),
    Handler::on(EventId::Hit, on_hit),
]);
