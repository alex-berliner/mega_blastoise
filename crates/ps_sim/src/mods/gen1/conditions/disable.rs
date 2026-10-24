//! `disable`: gen 1's Disable picks a random move slot with PP left and
//! blocks it for 1-8 of the holder's move attempts (0-7 lost turns). The
//! count runs down in `onBeforeMove`, not at the end of the turn; gen 1
//! drops the base `duration` and `durationCallback`.
//!
//! Effect state:
//! - `move`: `move_` (the disabled move's id);
//! - `slotIndex`: `data[0]`;
//! - `time`: `data[1]`.
//!
//! `disabledSource` is not kept: gen 1 checks a disabled move by slot, and
//! nothing in gen 1 reads the source.
//!
//! PS: data/mods/gen1/moves.ts disable.condition (lines 255-289:
//! `onStart` 258-266, `onBeforeMove` 267-281, `onDisableMove` 282-288);
//! data/moves.ts disable.condition.onEnd (lines 3695-3697)

use ps_protocol::{Arg, ArgValue, Kw, MoveNum, Msg, Text, TextArg, TextId};

use crate::effect::{EffectDef, EffectId, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen1::{conditions, moves};

fn disable_arg() -> Arg {
    Arg::new(ArgValue::Move(MoveNum(moves::DISABLE.0)))
}

/// `onStart(pokemon)`: `this.sample` of the slots with PP, then
/// `-start|MON|Disable|MOVE` and `time = this.random(1, 9)`.
// PS: data/mods/gen1/moves.ts disable.condition.onStart
fn on_start(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let mut slots = [0u8; 4];
    let mut n = 0;
    for (i, ms) in b.mon(pokemon).move_slots.iter().enumerate() {
        if !ms.id.is_none() && ms.pp > 0 {
            slots[n] = i as u8;
            n += 1;
        }
    }
    // The move's `onTryHit` made sure there is one.
    assert!(n > 0, "disable.onStart: no move with PP");
    let slot_index = *b.prng.sample(&slots[..n]);
    let id = b.mon(pokemon).move_slots[slot_index as usize].id;
    b.debug(Text::with2(TextId::DisableDisablingMove, TextArg::mv(MoveNum(id.0)), TextArg::int(i32::from(slot_index))));
    let mon = b.ident(pokemon);
    b.add(Msg::StartEffect {
        mon,
        effect: disable_arg(),
        a: Some(Arg::new(ArgValue::Move(MoveNum(id.0)))),
        b: None,
        kw: Kw::NONE,
    });
    let time = b.prng.random_range(1, 9);
    let st = b.effect_state();
    st.move_ = id;
    st.data[0] = i16::from(slot_index);
    st.data[1] = time as i16;
    None
}

/// `onEnd(pokemon)`: `-end|MON|Disable`.
// PS: data/moves.ts disable.condition.onEnd
fn on_end(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let mon = b.ident(pokemon);
    b.add(Msg::EndEffect { mon, effect: disable_arg(), kw: Kw::NONE });
    None
}

/// `onBeforeMove(pokemon, target, move)`: one attempt used up; at 0 the
/// volatile ends and the move goes ahead. Otherwise the disabled move (Bide
/// while Bide is stored) is `cant|MON|Disable|MOVE` and ends a charging
/// two-turn move.
// PS: data/mods/gen1/moves.ts disable.condition.onBeforeMove
fn on_before_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let st = b.mon_mut(pokemon).volatiles.get_mut(EffectId::Condition(conditions::DISABLE)).expect("disable");
    st.data[1] -= 1;
    if st.data[1] == 0 {
        b.remove_volatile(pokemon, conditions::DISABLE);
        return None;
    }
    // `if (pokemon.volatiles['bide']) move = this.dex.getActiveMove('bide');`
    let id = if b.has_vol(pokemon, conditions::BIDE) {
        moves::BIDE
    } else {
        match a.effect {
            EffectId::Move(m) => m,
            // A move id that names no move (`cannotmove`) is never the
            // disabled one.
            _ => return None,
        }
    };
    if id != b.effect_state().move_ {
        return None;
    }
    let mon = b.ident(pokemon);
    b.add(Msg::Cant { mon, reason: disable_arg(), mv: Some(Arg::new(ArgValue::Move(MoveNum(id.0)))) });
    b.remove_volatile(pokemon, conditions::TWOTURNMOVE);
    Some(Relay::FAIL)
}

/// `onDisableMove(pokemon)`: the slot is disabled, if the Pokemon still has
/// that many slots.
// PS: data/mods/gen1/moves.ts disable.condition.onDisableMove
fn on_disable_move(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon()?;
    let slot_index = b.effect_state().data[0] as usize;
    let len = b.mon(pokemon).move_slots.iter().filter(|ms| !ms.id.is_none()).count();
    if len > slot_index {
        b.mon_mut(pokemon).move_slots[slot_index].disabled = 1;
    }
    None
}

pub static DEF: EffectDef = EffectDef::new(&[
    Handler::on(EventId::Start, on_start),
    Handler::on(EventId::End, on_end),
    Handler::on(EventId::BeforeMove, on_before_move).priority(6),
    Handler::on(EventId::DisableMove, on_disable_move),
]);
