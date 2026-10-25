//! Counter: twice the last damage dealt in the battle, when the target's
//! last move was a Normal or Fighting move with base power. No effect
//! state (its `condition` has no handlers).
//!
//! Gen 1 reads the battle-wide `lastDamage` (so it counters damage dealt
//! to another Pokemon before a switch) and judges the target's move from
//! the Counter user's view under Desync Clause Mod: the target side's last
//! announced move (`lastEnemyMove`) and last selected move as the user saw
//! it (`lastEnemySelectedMove`). Counter is in `SKIP_LASTDAMAGE`, so
//! `useMoveInner` keeps `lastDamage` for it (`scripts.rs`).
//!
//! PS: data/mods/gen1/moves.ts counter (lines 146-232)

use ps_dex::{MoveId, Type};
use ps_protocol::{Kw, Msg, TextId};

use crate::choice::SelectedMove;
use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen1::moves;

/// `isCounterable(move)`: a Normal or Fighting move with base power
/// (`null` counts as `{basePower: 0, type: 'Normal'}`). The side's last
/// move is kept as its id: no gen 1 `ModifyMove` changes the type or base
/// power of a move that could be counterable.
fn is_counterable(b: &Battle, m: MoveId) -> bool {
    if m.is_none() {
        return false;
    }
    let def = b.dex.move_def(m);
    matches!(def.type_, Type::Normal | Type::Fighting) && def.base_power > 0
}

/// `damageCallback(pokemon, target)`.
// PS: data/mods/gen1/moves.ts counter.damageCallback
fn damage_callback(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let pokemon = a.target.mon().expect("Counter has a user");
    let target = a.source.expect("Counter has a target");
    let side = &b.sides[target.side as usize];
    let not_counter = |s: SelectedMove| s != SelectedMove::Move(moves::COUNTER);
    // The Counter user's view, which decides whether Counter succeeds.
    let is_last_enemy_selected_move_counterable = not_counter(side.last_enemy_selected_move);
    let is_last_enemy_move_counterable = is_counterable(b, side.last_enemy_move);
    // The target's own view, which only decides the hints.
    let is_last_selected_move_counterable = not_counter(side.last_selected_move);
    let is_last_move_counterable = is_counterable(b, side.last_move);
    let side_id = side.id();
    let is_last_damage_non_zero = b.last_damage > 0;

    let will_counter_succeed =
        is_last_enemy_selected_move_counterable && is_last_enemy_move_counterable && is_last_damage_non_zero;

    if !will_counter_succeed {
        if is_last_damage_non_zero {
            if !is_last_enemy_selected_move_counterable && is_last_selected_move_counterable {
                // The target has Counter in its first slot.
                if is_last_enemy_move_counterable {
                    b.hint(TextId::DesyncClauseActivated, false, None);
                    b.hint(TextId::HintGen1CounterSleep, false, None);
                }
            } else if !is_last_enemy_move_counterable && is_last_move_counterable {
                // The target selected a counterable move that was never
                // announced.
                b.hint(TextId::DesyncClauseActivated, false, Some(side_id));
                b.hint(TextId::HintGen1Counter, false, Some(side_id));
            }
        }
        let mon = b.ident(pokemon);
        b.add(Msg::Fail { mon, what: None, what2: None, kw: Kw::NONE });
        return Some(Relay::FAIL);
    }

    if !is_last_selected_move_counterable {
        // The target saw its own Counter: nothing to explain.
    } else if !is_last_move_counterable {
        // The target selected a non-counterable move that was never
        // announced.
        b.hint(TextId::DesyncClauseActivated, false, Some(side_id));
        b.hint(TextId::HintGen1Counter, false, Some(side_id));
    }

    Some(Relay::Int(2 * i32::from(b.last_damage)))
}

pub static DEF: EffectDef = EffectDef::new(&[Handler::on(EventId::DamageCallback, damage_callback)]);
