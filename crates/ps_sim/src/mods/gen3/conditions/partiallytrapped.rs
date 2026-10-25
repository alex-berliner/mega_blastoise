//! `partiallytrapped`: Bind, Wrap, Fire Spin, Clamp, Whirlpool and Sand
//! Tomb's 2-5 turns of trapping damage (the volatile's `duration` counts
//! down at the residual, so 3-6 turns of the volatile).
//!
//! Effect state: `source` is the trapper, `sourceEffect` the move;
//! `boundDivisor` (8 with Binding Band, a gen 5 item, else 16) is
//! `data[0]`.
//!
//! PS: data/mods/gen4/conditions.ts partiallytrapped (durationCallback),
//! data/mods/gen5/conditions.ts partiallytrapped (onStart, onResidual),
//! data/conditions.ts partiallytrapped (onEnd, onTrapPokemon)

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;
use ps_protocol::{Arg, ArgValue, Kw, KwFlags, MoveNum, Msg};

/// `this.effectState.boundDivisor`.
const BOUND_DIVISOR: usize = 0;

/// The trapping move (`this.effectState.sourceEffect`).
fn trapping_move(b: &mut Battle) -> ps_dex::MoveId {
    match b.effect_state().source_effect {
        EffectId::Move(m) => m,
        other => panic!("partiallytrapped from {other:?}"),
    }
}

/// `durationCallback(target, source)`: 6 with Grip Claw (a gen 4 item),
/// else 3-6 turns.
// PS: data/mods/gen4/conditions.ts partiallytrapped.durationCallback
fn duration_callback(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let gripclaw = b.dex.item_id("gripclaw");
    if let (Some(g), Some(source)) = (gripclaw, a.source) {
        if b.has_item(source, g) {
            return Some(Relay::Int(6));
        }
    }
    Some(Relay::Int(b.prng.random_range(3, 7) as i32))
}

/// `onStart(pokemon, source)`: the `-activate` line and the divisor.
// PS: data/mods/gen5/conditions.ts partiallytrapped.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let source = a.source.expect("partiallytrapped without a source");
    let m = trapping_move(b);
    let mon = b.ident(pokemon);
    let of = b.ident(source);
    let effect = Arg::new(ArgValue::MoveEffect(MoveNum(m.0)));
    b.add(Msg::Activate { mon: Some(mon), effect, a: None, b: None, kw: Kw { of: Some(of), ..Kw::NONE } });
    let bindingband = b.dex.item_id("bindingband");
    let divisor = if bindingband.is_some_and(|i| b.has_item(source, i)) { 8 } else { 16 };
    b.effect_state().data[BOUND_DIVISOR] = divisor;
    None
}

/// `onResidual(pokemon)`: the trap ends silently when the trapper left,
/// fainted or has not been in for a turn; otherwise the damage.
// PS: data/mods/gen5/conditions.ts partiallytrapped.onResidual
fn on_residual(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let trapper = b.effect_state().source;
    if let Some(t) = trapper {
        let tm = b.mon(t);
        if !tm.is_active || tm.hp == 0 || tm.active_turns == 0 {
            let m = trapping_move(b);
            // `delete pokemon.volatiles['partiallytrapped']`: no End event.
            if let Some(gone) = b.mon_mut(pokemon).volatiles.remove(EffectId::Condition(conditions::PARTIALLYTRAPPED)) {
                b.bury(gone);
            }
            let mon = b.ident(pokemon);
            let effect = Arg::new(ArgValue::Move(MoveNum(m.0)));
            b.add(Msg::EndEffect { mon, effect, kw: Kw::flags(KwFlags::PARTIALLYTRAPPED | KwFlags::SILENT) });
            return None;
        }
    }
    let divisor = i32::from(b.effect_state().data[BOUND_DIVISOR]);
    let damage = super::fraction_of_base_maxhp(b, pokemon, divisor);
    b.damage(damage, None, None, EffectId::None);
    None
}

/// `onEnd(pokemon)`: the `-end` line.
// PS: data/conditions.ts partiallytrapped.onEnd
fn on_end(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let m = trapping_move(b);
    let mon = b.ident(pokemon);
    let effect = Arg::new(ArgValue::Move(MoveNum(m.0)));
    b.add(Msg::EndEffect { mon, effect, kw: Kw::flags(KwFlags::PARTIALLYTRAPPED) });
    None
}

/// `onTrapPokemon(pokemon)`: trapped while the trapper is in (the G-Max
/// moves that trap without one are gen 8).
// PS: data/conditions.ts partiallytrapped.onTrapPokemon
fn on_trap_pokemon(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    if b.effect_state().source.is_some_and(|s| b.mon(s).is_active) {
        b.try_trap(pokemon, false);
    }
    None
}

/// The `[from]` and flags of the trap's `-damage` line: `[from]
/// move: NAME|[partiallytrapped]`, the move from the target's
/// `partiallytrapped` volatile.
// PS: sim/battle.ts spreadDamage (`case 'partiallytrapped'`)
pub fn damage_kw(b: &Battle, target: crate::state::MonRef) -> Kw {
    let st = b.mon(target).volatiles.get(EffectId::Condition(conditions::PARTIALLYTRAPPED));
    let m = match st.map(|s| s.source_effect) {
        Some(EffectId::Move(m)) => m,
        other => panic!("partiallytrapped damage without the volatile's move: {other:?}"),
    };
    Kw { from: Some(Arg::new(ArgValue::MoveEffect(MoveNum(m.0)))), of: None, flags: KwFlags::PARTIALLYTRAPPED, repeat_miss: 0 }
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::DurationCallback, duration_callback),
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::Residual, on_residual).order(10).sub_order(9),
    Handler::on(EventId::End, on_end),
    Handler::on(EventId::TrapPokemon, on_trap_pokemon),
]);
