//! Liquid Ooze: a Pokemon that drains the holder (a draining move, Leech
//! Seed) loses the HP it would have gained.
//!
//! The ability keeps no state of its own.
//!
//! PS: data/mods/gen4/abilities.ts liquidooze (onSourceTryHeal)

use ps_dex::gen3::moves;
use ps_protocol::{AbilityNum, ItemNum, MoveNum, Text, TextArg, TextId, Word};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler, Prefix, Pseudo};
use crate::relay::Relay;
use crate::state::Battle;

/// The `${effect.id}` of the debug line. A condition named after a move
/// (`leechseed`, `wish`) prints as that move's id, the same string.
fn effect_id_arg(b: &Battle, effect: EffectId) -> TextArg {
    match effect {
        EffectId::Move(m) => TextArg::move_id(MoveNum(m.0)),
        EffectId::Item(i) => TextArg::item_id(ItemNum(i.0)),
        EffectId::Ability(a) => TextArg::ability_id(AbilityNum(a.0)),
        EffectId::Pseudo(Pseudo::Drain) => TextArg::word(Word::Drain),
        EffectId::Condition(_) => {
            let key = b.effect_key(effect);
            let Some(m) = b.dex.move_id(key) else {
                panic!("unported: Liquid Ooze's debug line for the {key} condition");
            };
            TextArg::move_id(MoveNum(m.0))
        }
        other => panic!("unported: Liquid Ooze's debug line for {other:?}"),
    }
}

/// `onSourceTryHeal(damage, target, source, effect)`: the debug line; then
/// for `drain` and `leechseed` (unless the move in flight is Dream Eater)
/// `this.damage(damage, null, null, null, true)`, which takes the TryHeal
/// event's target (the healer), its source (the holder) and the ability,
/// and heals nothing (`0`).
fn on_source_try_heal(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let source = a.source?;
    let effect = effect_id_arg(b, a.effect);
    let (t, s) = (b.ident(target), b.ident(source));
    b.debug(Text::with3(TextId::LiquidOozeHeal, TextArg::mon(t), TextArg::mon(s), effect));
    if !matches!(b.effect_key(a.effect), "drain" | "leechseed") {
        return None;
    }
    if b.active_move().is_some_and(|mv| mv.id == moves::DREAMEATER) {
        return None;
    }
    let Some(Relay::Int(damage)) = a.relay else { panic!("TryHeal relay {:?}", a.relay) };
    b.damage_instafaint(damage, None, None, EffectId::None);
    Some(Relay::Int(0))
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::TryHeal, on_source_try_heal).prefix(Prefix::Source)]);
