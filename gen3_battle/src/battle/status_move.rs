//! Zero-power moves: every `StatusAction` arm, plus the status and
//! confusion infliction they share with damaging moves' secondaries.

extern crate alloc;

use alloc::vec::Vec;

use crate::ability;
use crate::data::{
    Boost, Status, StatusAction, Weather,
};
use crate::stats::Stat;
use crate::types::Type;

use super::*;

impl Battle {
    /// Resolve a zero-power move. `hit` already includes the accuracy roll;
    /// self-targeted actions cannot miss (their table accuracy is 0, the
    /// never-miss sentinel). A status move with no modelled action — Splash —
    /// does nothing, honestly.
    pub(super) fn status_move(
        &mut self,
        side: usize,
        foe: usize,
        slot: &MoveSlot,
        hit: bool,
        scripted: bool,
        script_stall: Option<bool>,
        events: &mut Vec<Event>,
    ) {
        let Some(action) = slot.entry.status_action else {
            return;
        };
        match action {
            StatusAction::BoostSelf(list) => {
                for &(boost, delta) in list {
                    self.sides[side].mon_mut().apply_boost(boost, delta);
                    events.push(Event::Boosted {
                        side: side as u8 + 1,
                        boost,
                        delta,
                    });
                }
            }
            StatusAction::HealHalf => {
                let mon = self.sides[side].mon_mut();
                let heal = (mon.max_hp / 2).min(mon.max_hp - mon.hp);
                if heal > 0 {
                    mon.hp += heal;
                    events.push(Event::Healed {
                        side: side as u8 + 1,
                        amount: heal,
                    });
                }
            }
            StatusAction::Seed => {
                if self.sides[foe].mon().sub_hp > 0 {
                    return;
                }
                let target = self.sides[foe].mon();
                let grass = target.types().0 == Type::Grass || target.types().1 == Type::Grass;
                if hit && !grass && !target.fainted() && !target.seeded {
                    self.sides[foe].mon_mut().seeded = true;
                    events.push(Event::Seeded {
                        side: foe as u8 + 1,
                    });
                }
            }
            StatusAction::BoostConfuse(list) => {
                if self.sides[foe].mon().sub_hp > 0 {
                    return;
                }
                // Swagger and Flatter: the gift lands (Mist still blocks
                // drops, not raises), then the confusion (Safeguard's job).
                if hit && !self.sides[foe].mon().fainted() {
                    for &(boost, delta) in list {
                        self.sides[foe].mon_mut().apply_boost(boost, delta);
                        events.push(Event::Boosted {
                            side: foe as u8 + 1,
                            boost,
                            delta,
                        });
                    }
                    self.confuse(foe, scripted, events);
                }
            }
            StatusAction::Focus => {
                if !self.sides[side].mon().focused {
                    self.sides[side].mon_mut().focused = true;
                    events.push(Event::Focused {
                        side: side as u8 + 1,
                    });
                }
            }
            StatusAction::Rest => {
                // Nothing sleeps through an Uproar, a self-inflicted sleep
                // included: the din answers the sim's SetStatus event
                // wherever the sleep came from.
                let uproar = (0..2).any(|w| {
                    let m = self.sides[w].mon();
                    m.uproar_ending
                        || (m.rampage.is_some() && m.locked_move == Some("uproar"))
                        || m.rampage.is_some_and(|(i, _)| {
                            m.moves.get(i as usize).is_some_and(|s| s.entry.id == "uproar")
                        })
                });
                let mon = self.sides[side].mon();
                // A mon that cannot be put to sleep cannot Rest either:
                // the sim's Rest goes through setStatus, and Insomnia
                // refuses it there — so the heal never happens at all.
                let sleepless =
                    ability::blocks_status(&self.sides[side].mon().bearer(), Status::Sleep);
                // Rest refuses to run at all on a mon that is ALREADY asleep,
                // and its onTry asks that before anything else. It matters
                // because Sleep Talk can only be used while asleep and will
                // happily reach for Rest: the call fails, and the sleeper
                // does not heal.
                let already = mon.status == Some(Status::Sleep);
                if mon.hp < mon.max_hp && !uproar && !sleepless && !already {
                    let mon = self.sides[side].mon_mut();
                    mon.hp = mon.max_hp;
                    mon.status = Some(Status::Sleep);
                    // The games' Rest sleeps two full turns: clock of 3,
                    // the same shape as the sim's pinned setStatus. A fresh
                    // sleep also shakes off a Nightmare.
                    mon.sleep_n = 3;
                    mon.sleep_skipped = 0;
                    mon.toxic_n = 0;
                    mon.nightmared = false;
                    events.push(Event::Rested {
                        side: side as u8 + 1,
                    });
                }
            }
            StatusAction::Minimize => {
                self.sides[side].mon_mut().minimized = true;
                self.sides[side].mon_mut().apply_boost(Boost::Eva, 1);
                events.push(Event::Boosted {
                    side: side as u8 + 1,
                    boost: Boost::Eva,
                    delta: 1,
                });
            }
            StatusAction::WeatherHeal => {
                let mult = match self.effective_weather() {
                    None => (1, 2),
                    Some(Weather::Sun) => (2, 3),
                    Some(_) => (1, 4),
                };
                let mon = self.sides[side].mon_mut();
                let heal = ((mon.max_hp as u32 * mult.0 / mult.1) as u16).min(mon.max_hp - mon.hp);
                if heal > 0 {
                    mon.hp += heal;
                    events.push(Event::Healed {
                        side: side as u8 + 1,
                        amount: heal,
                    });
                }
            }
            StatusAction::Refresh => {
                let mon = self.sides[side].mon_mut();
                if matches!(
                    mon.status,
                    Some(Status::Burn | Status::Paralysis | Status::Poison | Status::Toxic)
                ) {
                    mon.status = None;
                    mon.toxic_n = 0;
                }
            }
            StatusAction::BellyDrum => {
                let mon = self.sides[side].mon();
                let cost = mon.max_hp / 2;
                if mon.hp > cost && mon.stages[Stat::Atk as usize] < 6 {
                    let mon = self.sides[side].mon_mut();
                    mon.hp -= cost;
                    mon.stages[Stat::Atk as usize] = 6;
                    events.push(Event::Boosted {
                        side: side as u8 + 1,
                        boost: Boost::Atk,
                        delta: 6,
                    });
                } else {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                }
            }
            StatusAction::PsychUp => {
                let stages = self.sides[foe].mon().stages;
                let acc = self.sides[foe].mon().acc_stage;
                let eva = self.sides[foe].mon().eva_stage;
                let mon = self.sides[side].mon_mut();
                mon.stages = stages;
                mon.acc_stage = acc;
                mon.eva_stage = eva;
            }
            StatusAction::Yawn => {
                let target = self.sides[foe].mon();
                if hit
                    && target.sub_hp == 0
                    && target.status.is_none()
                    && target.yawn_n == 0
                    && self.sides[foe].safeguard_n == 0
                    && !target.fainted()
                    && !ability::blocks_yawn(&target.bearer())
                {
                    self.sides[foe].mon_mut().yawn_n = 2;
                    events.push(Event::Drowsy {
                        side: foe as u8 + 1,
                    });
                }
            }
            StatusAction::Wish => {
                if self.sides[side].wish_n == 0 {
                    self.sides[side].wish_n = 2;
                    self.sides[side].wish_amount = self.sides[side].mon().max_hp / 2;
                }
            }
            StatusAction::PerishSong => {
                // The song sweeps the field, but its onHitField loop still
                // runs the Invulnerability event per target: a mon that is
                // underground or in the air is missed and never picks up a
                // count, however loud the singing.
                for who in 0..2 {
                    if self.sides[who].mon().semi_invulnerable().is_some() {
                        continue;
                    }
                    // The song is a sound, and it sweeps the singer's own
                    // side too, so a Soundproof singer is deaf to it.
                    if self.sides[who].mon().ability == "soundproof" {
                        continue;
                    }
                    let mon = self.sides[who].mon_mut();
                    if mon.perish_n == 0 && !mon.fainted() {
                        mon.perish_n = 4;
                    }
                }
            }
            StatusAction::DestinyBond => {
                self.sides[side].mon_mut().destiny = true;
                events.push(Event::DestinyArmed {
                    side: side as u8 + 1,
                });
            }
            StatusAction::MeanLook => {
                let target = self.sides[foe].mon();
                if hit && target.sub_hp == 0 && !target.mean_looked && !target.fainted() {
                    self.sides[foe].mon_mut().mean_looked = true;
                    events.push(Event::NoEscape {
                        side: foe as u8 + 1,
                    });
                }
            }
            StatusAction::Sport(kind) => {
                self.sides[side].mon_mut().sport = Some(kind);
            }
            StatusAction::Spikes => {
                if self.sides[foe].spikes < 3 {
                    self.sides[foe].spikes += 1;
                    events.push(Event::SpikesLaid {
                        side: foe as u8 + 1,
                    });
                } else {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                }
            }
            StatusAction::Memento => {
                if !hit || self.sides[foe].mon().sub_hp > 0 || self.sides[foe].mon().fainted() {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                    return;
                }
                let misted = self.sides[foe].mist_n > 0;
                if !misted {
                    for (boost, delta) in [(Boost::Atk, -2i8), (Boost::SpAtk, -2)] {
                        // Clear Body and its kin hang off the generic
                        // onTryBoost, which does not care whether the drop
                        // came from a secondary or from the move itself, and
                        // deletes the stats one at a time — so a Hyper Cutter
                        // refuses the Attack half and takes the rest.
                        if ability::blocks_drop(
                            &self.sides[foe].mon().bearer(),
                            ability::drop_kind(boost),
                        ) {
                            continue;
                        }
                        self.sides[foe].mon_mut().apply_boost(boost, delta);
                        events.push(Event::Boosted {
                            side: foe as u8 + 1,
                            boost,
                            delta,
                        });
                    }
                }
                self.sides[side].mon_mut().hp = 0;
                self.announce_faint(side, events);
            }
            StatusAction::PainSplit => {
                if !hit || self.sides[foe].mon().sub_hp > 0 || self.sides[foe].mon().fainted() {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                    return;
                }
                let avg = (self.sides[side].mon().hp as u32 + self.sides[foe].mon().hp as u32) / 2;
                for who in [side, foe] {
                    let mon = self.sides[who].mon_mut();
                    mon.hp = (avg as u16).min(mon.max_hp);
                }
                events.push(Event::Healed {
                    side: side as u8 + 1,
                    amount: 0,
                });
            }
            StatusAction::Protect | StatusAction::Endure => {
                // Nothing left to shield against: the sim refuses before it
                // ever reaches the stall gamble.
                if !self.will_act {
                    self.sides[side].mon_mut().stall_counter = 0;
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                    return;
                }
                let counter = self.sides[side].mon().stall_counter;
                let ok = counter == 0
                    || match script_stall {
                        Some(stall) => stall,
                        None => self.rng.below(counter as u32) == 0,
                    };
                if ok {
                    let mon = self.sides[side].mon_mut();
                    if matches!(action, StatusAction::Protect) {
                        mon.protected = true;
                    } else {
                        mon.enduring = true;
                    }
                    mon.stall_counter = if counter == 0 {
                        2
                    } else {
                        (counter * 2).min(8)
                    };
                    events.push(Event::Protected {
                        side: side as u8 + 1,
                    });
                } else {
                    self.sides[side].mon_mut().stall_counter = 0;
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                }
            }
            StatusAction::Identify => {
                if hit && self.sides[foe].mon().sub_hp == 0 && !self.sides[foe].mon().fainted() {
                    self.sides[foe].mon_mut().identified = true;
                }
            }
            StatusAction::LockOn => {
                // Taking aim at a mon already in your sights does nothing:
                // addVolatile refuses a lock that is still up, and the move
                // fails rather than refreshing it.
                if hit && !self.sides[foe].mon().fainted() && self.sides[side].mon().sure_hit == 0 {
                    let aim = self.sides[foe].active as u8;
                    let mon = self.sides[side].mon_mut();
                    mon.sure_hit = 2;
                    mon.sure_hit_on = aim;
                } else {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                }
            }
            StatusAction::ChargeUp => {
                self.sides[side].mon_mut().charged_elec = true;
            }
            // Handled by the call substitution above, before the status
            // dispatch is ever reached; these arms only exist so the match
            // stays exhaustive.
            StatusAction::SleepTalk | StatusAction::Assist => {}
            StatusAction::BatonPass => {
                // The user leaves and hands over its boosts and most of its
                // volatiles. `copyVolatileFrom` copies EVERY volatile the
                // passer holds except the ones flagged noCopy — Disable,
                // Encore, Foresight, Nightmare, Stockpile, Imprison,
                // Minimize, Torment, Toxic's counter, Yawn, Destiny Bond,
                // Defense Curl, Attract, Counter, Mirror Coat, the Choice
                // lock and Flash Fire's catch — so the incoming mon starts
                // clean of those and inherits the rest.
                //
                // The list below is everything copyable that can still be
                // standing when Baton Pass is SELECTED. Locking volatiles
                // (Bide, Rollout, Thrash, a charging two-turner, Uproar,
                // Recharge) are copyable in principle but leave no turn in
                // which the pass could be chosen, and the ones that end
                // before the pass resolves (Rage and Grudge clear in
                // onBeforeMove, Endure and Protect expire the same turn)
                // cannot reach the incoming mon either.
                let incoming = (0..self.sides[side].party.len())
                    .find(|&i| i != self.sides[side].active && !self.sides[side].party[i].fainted());
                match incoming {
                    None => events.push(Event::Failed {
                        side: side as u8 + 1,
                    }),
                    Some(next) => {
                        let passed = self.sides[side].mon().clone();
                        self.switch_out_reset(side);
                        let slot_item = self.hand_slot_item_over(side);
                        self.sides[side].reorder_for_switch(next);
                        self.sides[side].active = next;
                        self.sides[side].mon_mut().last_item = slot_item;
                        {
                            let m = self.sides[side].mon_mut();
                            m.stages = passed.stages;
                            m.acc_stage = passed.acc_stage;
                            m.eva_stage = passed.eva_stage;
                            m.sub_hp = passed.sub_hp;
                            m.seeded = passed.seeded;
                            m.confusion_n = passed.confusion_n;
                            m.perish_n = passed.perish_n;
                            m.cursed = passed.cursed;
                            m.ingrained = passed.ingrained;
                            m.focused = passed.focused;
                            m.mean_looked = passed.mean_looked;
                            m.trapped_n = passed.trapped_n;
                            m.charged_elec = passed.charged_elec;
                            m.taunt_n = passed.taunt_n;
                            // Water Sport and Mud Sport are volatiles on the
                            // HUMMER, not the field, so the halving follows
                            // the pass and dampens the arrival's own moves.
                            m.sport = passed.sport;
                            // Lock-On's condition spells `noCopy: false` out
                            // in the data; the arrival inherits the mark and
                            // the turn left on it.
                            m.sure_hit = passed.sure_hit;
                            // Fury Cutter's doubling counter rides along too.
                            m.fury_n = passed.fury_n;
                            m.fury_fresh = passed.fury_fresh;
                        }
                        events.push(Event::Switched {
                            side: side as u8 + 1,
                            party_index: next,
                        });
                        self.switch_in_greet(side, events);
                    }
                }
            }
            StatusAction::ForceSwitch => {
                // Roar and Whirlwind drag a benched mon out of the other
                // side. The sim samples `possibleSwitches`, which is its own
                // `side.pokemon` order past the active slot; a pinned sample
                // lands on that list's first entry. With nobody to drag in,
                // or the target already down, the move simply fails.
                // Ingrain's roots hold against a drag as well as against a
                // voluntary switch — it is the one thing in this era with an
                // onDragOut — and the sim announces the refusal rather than
                // failing quietly. A bind or a Mean Look does NOT stop one:
                // those only forbid leaving of your own accord.
                let rooted = self.sides[foe].mon().ingrained
                    || ability::blocks_drag(&self.sides[foe].mon().bearer());
                let bench = self.sides[foe].draggable();
                match bench.first().copied() {
                    Some(next) if hit && !rooted && !self.sides[foe].mon().fainted() => {
                        self.switch_out_reset(foe);
                        let slot_item = self.hand_slot_item_over(foe);
                        self.sides[foe].reorder_for_switch(next);
                        self.sides[foe].active = next;
                        self.sides[foe].mon_mut().last_item = slot_item;
                        self.dragged[foe] = true;
                        events.push(Event::Switched {
                            side: foe as u8 + 1,
                            party_index: next,
                        });
                        self.switch_in_greet(foe, events);
                    }
                    _ => events.push(Event::Failed {
                        side: side as u8 + 1,
                    }),
                }
            }
            StatusAction::Spite => {
                // Spite reaches through a substitute in this era. It drains
                // the slot carrying the last-used move's ID — a Transform
                // that rewrote the slots leaves nothing to drain, and fails.
                let drained = if hit {
                    if let Some(id) = self.sides[foe]
                        .mon()
                        .last_used_id
                        .filter(|&i| i != "struggle")
                    {
                        let mon = self.sides[foe].mon_mut();
                        if let Some(ms) = mon.moves.iter_mut().find(|m| m.entry.id == id) {
                            if ms.pp > 0 {
                                // The games shave 2..5 PP; a script pins 2.
                                let cut = if scripted {
                                    2
                                } else {
                                    2 + self.rng.below(4) as u8
                                };
                                ms.pp = ms.pp.saturating_sub(cut);
                                true
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                };
                if !drained {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                }
            }
            StatusAction::Grudge => {
                self.sides[side].mon_mut().grudged = true;
            }
            StatusAction::Torment => {
                if hit && !self.sides[foe].mon().tormented {
                    self.sides[foe].mon_mut().tormented = true;
                    // The landing turn is free: an already-chosen repeat
                    // still runs; only choices made under Torment struggle.
                    self.sides[foe].mon_mut().torment_fresh = true;
                } else {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                }
            }
            StatusAction::Encore => {
                let target = self.sides[foe].mon();
                // The sim reads the move that was actually USED, then looks
                // its slot up by id: a `failencore` move is refused, and so
                // is one the target no longer carries — which is what makes
                // Encore fail on a mon that just Transformed, since the
                // transform rewrote every slot out from under it.
                let encorable = target.last_used_id.is_some_and(|id| {
                    !matches!(
                        id,
                        "encore" | "mimic" | "mirrormove" | "sketch" | "struggle" | "transform"
                    ) && target
                        .moves
                        .iter()
                        .any(|m| m.entry.id == id && m.pp > 0)
                });
                if hit && target.encore_n == 0 && encorable && !target.fainted() {
                    // The games run 3..6 encored turns; a script pins 3.
                    let n = if scripted {
                        3
                    } else {
                        3 + self.rng.below(4) as u8
                    };
                    self.sides[foe].mon_mut().encore_n = n;
                    self.sides[foe].mon_mut().encore_fresh = true;
                } else {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                }
            }
            StatusAction::Disable => {
                // Disable pierces a substitute in this era, but fails when
                // the target's last move has no PP left to seal — and it
                // finds that move BY ID, so a Transform that rewrote the
                // slots leaves nothing to disable.
                let target = self.sides[foe].mon();
                let slot_by_id = target
                    .last_used_id
                    .filter(|&i| i != "struggle")
                    .and_then(|id| target.moves.iter().position(|m| m.entry.id == id));
                let last_has_pp = slot_by_id
                    .and_then(|i| target.moves.get(i))
                    .is_some_and(|m| m.pp > 0);
                if hit && target.disabled_slot.is_none() && last_has_pp && !target.fainted() {
                    // This era's clock is `random(2, 6)`, and it counts the
                    // victim's next N ACTIONS rather than N turns. Measured
                    // against the sim: a Disable from a FASTER mon blocks the
                    // victim's move that same turn and the next, and ends; a
                    // Disable from a slower one lands after the victim has
                    // moved and blocks the following two turns instead. (The
                    // modern four-to-seven belongs to a later generation.)
                    let slot_i = slot_by_id.map(|i| i as u8);
                    let n = if scripted {
                        2
                    } else {
                        2 + self.rng.below(4) as u8
                    };
                    // The sim's clock is `random(2, 6)`, and its onStart adds
                    // ONE MORE turn when the victim has already had its go —
                    // `if (!queue.willMove(victim)) duration++`. That is an
                    // increment, not a skipped tick: the landing turn's own
                    // residual still counts against it either way. Modelling
                    // it as a skip left the move greyed out a turn too long,
                    // which the reference's own request disagreed with.
                    let already_moved = self.acted_this_turn[foe];
                    let mon = self.sides[foe].mon_mut();
                    mon.disabled_slot = slot_i;
                    mon.disable_n = n + u8::from(already_moved);
                    mon.disable_fresh = true;
                    mon.disable_skip_tick = false;
                } else {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                }
            }
            StatusAction::Camouflage => {
                // Fails on anything already carrying Normal (either slot).
                let (t1, t2) = self.sides[side].mon().types();
                if t1 == Type::Normal || t2 == Type::Normal {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                    return;
                }
                self.sides[side].mon_mut().type_override = Some((Type::Normal, Type::None));
            }
            StatusAction::Conversion => {
                // Only a type the user does not already have is eligible;
                // with no eligible move type, Conversion fails.
                let cur = self.sides[side].mon().types();
                let ty = self.sides[side]
                    .mon()
                    .moves
                    .iter()
                    .map(|m| m.move_type())
                    .find(|&t| t != Type::None && t != cur.0 && t != cur.1);
                match ty {
                    Some(t) => {
                        self.sides[side].mon_mut().type_override = Some((t, Type::None));
                    }
                    None => events.push(Event::Failed {
                        side: side as u8 + 1,
                    }),
                }
            }
            StatusAction::Imprison => {
                // Fails unless the foe actually shares a move with the user
                // — the sim refuses a sealless Imprison outright.
                let shares = self.sides[side].mon().moves.iter().any(|m| {
                    self.sides[foe]
                        .mon()
                        .moves
                        .iter()
                        .any(|f| f.entry.id == m.entry.id)
                });
                if shares && !self.sides[side].mon().imprisoning {
                    self.sides[side].mon_mut().imprisoning = true;
                    self.sides[side].mon_mut().imprison_fresh = true;
                } else {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                }
            }
            StatusAction::MirrorMove | StatusAction::Mimic | StatusAction::Sketch => {
                // Handled before the status path; unreachable here.
            }
            StatusAction::Transform => {
                let foe_mon = self.sides[foe].mon().clone();
                // Copying a copy fails: a transformed target refuses it.
                if foe_mon.fainted() || foe_mon.transform_backup.is_some() {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                } else {
                    let mon = self.sides[side].mon_mut();
                    if mon.transform_backup.is_none() {
                        mon.transform_backup = Some(mon.moves.clone());
                        mon.transform_stats = Some((
                            [mon.atk, mon.def, mon.spa, mon.spd, mon.spe],
                            mon.type_override,
                        ));
                    }
                    mon.atk = foe_mon.atk;
                    mon.def = foe_mon.def;
                    mon.spa = foe_mon.spa;
                    mon.spd = foe_mon.spd;
                    mon.spe = foe_mon.spe;
                    mon.stages = foe_mon.stages;
                    mon.acc_stage = foe_mon.acc_stage;
                    mon.eva_stage = foe_mon.eva_stage;
                    mon.type_override = Some(foe_mon.types());
                    // The copy is complete enough to include the ability,
                    // which is how a paralysed mon that copies a Limber one
                    // walks away cured.
                    mon.flash_fire = false;
                    mon.moves = foe_mon
                        .moves
                        .iter()
                        .map(|ms| MoveSlot {
                            entry: ms.entry,
                            // Five PP per copied move, but never more than
                            // the move's own maximum (a copied Sketch has 1).
                            pp: 5.min(ms.entry.pp),
                            typed_as: ms.typed_as,
                        })
                        .collect();
                    // The copy is complete enough to include the ability,
                    // which is how a paralysed mon that copies a Limber one
                    // walks away cured — and it goes back on switching out
                    // like every other borrowed ability.
                    self.set_ability(side, foe_mon.ability);
                    self.ability_update(side);
                }
            }
            StatusAction::Trick => {
                // Sticky Hold refuses the trade outright, and two empty
                // hands have nothing to trade.
                let mine = self.sides[side].mon().item;
                let theirs = self.sides[foe].mon().item;
                // Mail refuses to leave its holder's hands for anything but
                // a Knock Off, a Thief or a Covet: its `onTakeItem` turns
                // everything else away, and Trick takes both items before it
                // hands either over, so one piece of Mail on either side
                // fails the whole trade.
                // A Substitute turns it away too. The sub's
                // `onTryPrimaryHit` asks `getDamage` for a number and a
                // Status move gives it none, so the handler logs a fail and
                // returns null before `onHit` is ever reached. Trick carries
                // no `bypasssub` in this era, unlike Torment, Taunt, Spite
                // and the rest of that set.
                if !hit
                    || self.sides[foe].mon().sub_hp > 0
                    || self.sides[foe].mon().ability == "stickyhold"
                    || mine == "mail"
                    || theirs == "mail"
                    || (mine.is_empty() && theirs.is_empty())
                    || self.sides[foe].mon().fainted()
                {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                    return;
                }
                self.sides[side].mon_mut().item = theirs;
                self.sides[foe].mon_mut().item = mine;
                self.ability_update(side);
                self.ability_update(foe);
            }
            StatusAction::Recycle => {
                let back = self.sides[side].mon().last_item;
                if !self.sides[side].mon().item.is_empty() || back.is_empty() {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                    return;
                }
                let mon = self.sides[side].mon_mut();
                mon.item = back;
                mon.last_item = "";
                self.ability_update(side);
            }
            StatusAction::SkillSwap | StatusAction::RolePlay => {
                // Wonder Guard refuses to move, but the two moves ask
                // different questions of it. Skill Swap is a trade and fails
                // if EITHER side carries the ability (`failskillswap` is
                // tested on both). Role Play only reads the target, so its
                // gate is one-directional: a Wonder Guard user may happily
                // copy the ability away and lose it. The only source-side
                // gate Role Play has is `cantsuppress`, which no gen 3
                // ability carries. Both moves also refuse two mons that
                // already share an ability in this era.
                let mine = self.sides[side].mon().ability;
                let theirs = self.sides[foe].mon().ability;
                let unswappable = theirs == "wonderguard"
                    || (matches!(action, StatusAction::SkillSwap) && mine == "wonderguard");
                if !hit
                    || unswappable
                    || mine == theirs
                    || self.sides[foe].mon().fainted()
                    || self.sides[side].mon().fainted()
                {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                    return;
                }
                let swapping = matches!(action, StatusAction::SkillSwap);
                self.set_ability(side, theirs);
                if swapping {
                    self.set_ability(foe, mine);
                }
                // Losing Flash Fire loses what it caught: the sim ends the
                // old ability, and ending Flash Fire drops its volatile.
                for w in [side, foe] {
                    if self.sides[w].mon().ability != "flashfire" {
                        self.sides[w].mon_mut().flash_fire = false;
                    }
                }
                // A traded-in ability tidies up straight away — a frozen mon
                // handed Magma Armor thaws on the spot.
                self.ability_update(side);
                self.ability_update(foe);
            }
            StatusAction::NoopFail => {
                events.push(Event::Failed {
                    side: side as u8 + 1,
                });
            }
            StatusAction::Attract => {
                // Genders have to differ, and neither may be genderless.
                // The sim asks twice, in `onTryImmunity` and again in the
                // volatile's own `onStart`, and both are the same plain
                // comparison — so a mismatched pair is a move that failed.
                let (them, us) = (self.sides[foe].mon().gender, self.sides[side].mon().gender);
                let charmable = ((them == "M" && us == "F") || (them == "F" && us == "M"))
                    && !ability::blocks_attract(&self.sides[foe].mon().bearer());
                if !hit || !charmable || self.sides[foe].mon().attracted_by.is_some() {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                    return;
                }
                let who = self.sides[side].active;
                self.sides[foe].mon_mut().attracted_by = Some(who);
            }
            StatusAction::MagicCoat => {
                self.sides[side].mon_mut().magic_coat = true;
            }
            StatusAction::Snatch => {
                self.sides[side].mon_mut().snatching = true;
            }
            StatusAction::NoopSuccess => {}
            StatusAction::HealBell => {
                // The chime reaches the WHOLE party, bench included — the
                // sim walks `side.pokemon`, not just the active slot.
                // …but it never reaches a Soundproof one, on the field or
                // on the bench. The chime is a sound, and that ability is
                // deaf to it whoever rang it.
                let chimes = slot.entry.sound;
                for mon in self.sides[side].party.iter_mut() {
                    if chimes && mon.ability == "soundproof" {
                        continue;
                    }
                    mon.status = None;
                    mon.toxic_n = 0;
                    mon.sleep_n = 0;
                    mon.sleep_skipped = 0;
                    mon.nightmared = false;
                }
            }
            StatusAction::Ingrain => {
                if self.sides[side].mon().ingrained {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                } else {
                    self.sides[side].mon_mut().ingrained = true;
                }
            }
            StatusAction::Conversion2 => {
                // Retype to the FIRST type (the sim's pinned sample walks
                // the dex order) that resists the foe's last-used move.
                // The sim walks dex.types.names() — the typechart file's
                // key order — and the pinned sample takes the first.
                const DEX_ORDER: [Type; 17] = [
                    Type::Electric,
                    Type::Ghost,
                    Type::Grass,
                    Type::Steel,
                    Type::Dark,
                    Type::Bug,
                    Type::Dragon,
                    Type::Fighting,
                    Type::Fire,
                    Type::Flying,
                    Type::Ground,
                    Type::Ice,
                    Type::Normal,
                    Type::Poison,
                    Type::Psychic,
                    Type::Rock,
                    Type::Water,
                ];
                // The type it reads is the one the move was USED as, not the
                // one in the table: `attackType = lastMoveUsed.type` off the
                // active move, after every onModifyMove has had its say. Gen
                // 3's Beat Up retypes itself to '???' there, and nothing in
                // the chart resists '???', so a Conversion 2 answering a Beat
                // Up simply fails. Struggle is named outright as Normal.
                let last = self.sides[foe].mon().last_move_used_id.and_then(|id| match id {
                    "struggle" => Some(Type::Normal),
                    "beatup" => Some(Type::None),
                    _ => self.sides[foe]
                        .mon()
                        .moves
                        .iter()
                        .find(|m| m.entry.id == id)
                        .map(|m| m.move_type())
                        .or_else(|| crate::data::move_by_id(id).map(|m| m.move_type)),
                });
                match last {
                    Some(atk_type) if atk_type != Type::None => {
                        let pick = DEX_ORDER
                            .iter()
                            .copied()
                            .find(|&t| crate::types::effectiveness(atk_type, t) < 10);
                        match pick {
                            Some(t) => {
                                self.sides[side].mon_mut().type_override = Some((t, Type::None));
                            }
                            None => events.push(Event::Failed {
                                side: side as u8 + 1,
                            }),
                        }
                    }
                    _ => events.push(Event::Failed {
                        side: side as u8 + 1,
                    }),
                }
            }
            StatusAction::Curse => {
                let (t1, t2) = self.sides[side].mon().types();
                if t1 == Type::Ghost || t2 == Type::Ghost {
                    // The Ghost pays half its max HP to lay the curse.
                    if self.sides[foe].mon().cursed || self.sides[foe].mon().fainted() {
                        events.push(Event::Failed {
                            side: side as u8 + 1,
                        });
                    } else {
                        let cost = (self.sides[side].mon().max_hp / 2).max(1);
                        let user = self.sides[side].mon_mut();
                        let cost = cost.min(user.hp);
                        user.hp -= cost;
                        self.sides[foe].mon_mut().cursed = true;
                        self.announce_faint(side, events);
                    }
                } else {
                    let mon = self.sides[side].mon_mut();
                    mon.apply_boost(Boost::Atk, 1);
                    mon.apply_boost(Boost::Def, 1);
                    mon.apply_boost(Boost::Spe, -1);
                }
            }
            StatusAction::NaturePower => {
                // Handled before the status path; unreachable here.
            }
            StatusAction::Taunt => {
                // Taunt is one of the era's sub-piercers.
                let target = self.sides[foe].mon();
                if hit && target.taunt_n == 0 && !target.fainted() {
                    self.sides[foe].mon_mut().taunt_n = 2;
                }
            }
            StatusAction::Nightmare => {
                let target = self.sides[foe].mon();
                if hit
                    && target.sub_hp == 0
                    && target.status == Some(Status::Sleep)
                    && !target.nightmared
                {
                    self.sides[foe].mon_mut().nightmared = true;
                } else {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                }
            }
            StatusAction::Stockpile => {
                let mon = self.sides[side].mon_mut();
                if mon.stockpile_n < 3 {
                    mon.stockpile_n += 1;
                } else {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                }
            }
            StatusAction::Swallow => {
                let n = self.sides[side].mon().stockpile_n;
                if n == 0 {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                } else {
                    let mon = self.sides[side].mon_mut();
                    mon.stockpile_n = 0;
                    let heal = match n {
                        1 => mon.max_hp / 4,
                        2 => mon.max_hp / 2,
                        _ => mon.max_hp,
                    }
                    .min(mon.max_hp - mon.hp);
                    if heal > 0 {
                        mon.hp += heal;
                        events.push(Event::Healed {
                            side: side as u8 + 1,
                            amount: heal,
                        });
                    }
                }
            }
            StatusAction::Haze => {
                for who in 0..2 {
                    let mon = self.sides[who].mon_mut();
                    mon.stages = [0; 5];
                    mon.acc_stage = 0;
                    mon.eva_stage = 0;
                }
                events.push(Event::HazeCleared);
            }
            StatusAction::Substitute => {
                let mon = self.sides[side].mon();
                let cost = mon.max_hp / 4;
                if mon.sub_hp == 0 && mon.hp > cost {
                    let mon = self.sides[side].mon_mut();
                    mon.hp -= cost;
                    mon.sub_hp = cost;
                    // The substitute's own onStart deletes any partial trap
                    // its owner is under — a Bind, Wrap, Clamp, Fire Spin,
                    // Whirlpool or Sand Tomb ends there and then, and ends
                    // silently: the sim marks its `-end` `[silent]`.
                    mon.trapped_n = 0;
                    events.push(Event::SubStarted {
                        side: side as u8 + 1,
                    });
                } else {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                }
            }
            StatusAction::SetWeather(weather) => {
                if self.weather != Some(weather) {
                    self.weather = Some(weather);
                    self.weather_n = 5;
                    events.push(Event::WeatherStarted { weather });
                    for w in 0..2 {
                        self.forecast(w);
                    }
                }
            }
            StatusAction::Team(cond) => {
                let n = self.sides[side].condition_n(cond);
                if *n == 0 {
                    *n = 5;
                    events.push(Event::SideStarted {
                        side: side as u8 + 1,
                        condition: cond,
                    });
                }
            }
            StatusAction::Confuse => {
                if self.sides[foe].mon().sub_hp > 0 {
                    return;
                }
                let immune = slot.entry.respects_immunity
                    && crate::types::effectiveness_against(
                        slot.move_type(),
                        self.immunity_types(foe, slot.move_type()),
                    ) == 0;
                if hit && !immune {
                    self.confuse(foe, scripted, events);
                }
            }
            StatusAction::Inflict(status) => {
                if self.sides[foe].mon().sub_hp > 0 {
                    return;
                }
                let immune = slot.entry.respects_immunity
                    && crate::types::effectiveness_against(
                        slot.move_type(),
                        self.immunity_types(foe, slot.move_type()),
                    ) == 0;
                if hit && !immune {
                    self.inflict(foe, status, scripted, events);
                }
            }
            StatusAction::BoostFoe(list) => {
                if self.sides[foe].mon().sub_hp > 0 {
                    return;
                }
                let misted = self.sides[foe].mist_n > 0;
                if hit && !self.sides[foe].mon().fainted() {
                    for &(boost, delta) in list {
                        if misted && delta < 0 {
                            continue;
                        }
                        // Clear Body and the single-stat guards refuse a
                        // drop that came from the other side.
                        if delta < 0
                            && ability::blocks_drop(
                                &self.sides[foe].mon().bearer(),
                                ability::drop_kind(boost),
                            )
                        {
                            continue;
                        }
                        self.sides[foe].mon_mut().apply_boost(boost, delta);
                        events.push(Event::Boosted {
                            side: foe as u8 + 1,
                            boost,
                            delta,
                        });
                    }
                }
            }
        }
    }

    /// Land `status` on `foe`'s active mon if Gen 3 rules allow it, setting
    /// the clocks that come with it: Toxic restarts its count, sleep draws
    /// its duration (pinned to the reference sim's floor under a script).
    pub(super) fn inflict(&mut self, foe: usize, status: Status, scripted: bool, events: &mut Vec<Event>) {
        let before = self.sides[foe].mon().status;
        self.inflict_inner(foe, status, scripted, events);
        // Synchronize passes what it just caught back across the field, and
        // it goes FIRST. Both it and the curing berries hang off the same
        // AfterSetStatus event, and the berries are deprioritised there —
        // Lum carries `onAfterSetStatusPriority: -1`. So a Synchronize mon
        // holding one still infects the attacker on its way out of the
        // status. The bounce reads the status that actually took hold, so a
        // blocked or refused one bounces nothing.
        let took = self.sides[foe].mon().status != before;
        if took {
            if let Some(back) =
                ability::synchronizes(&self.sides[foe].mon().bearer(), status)
            {
                let source = 1 - foe;
                if !self.sides[source].mon().fainted() {
                    self.inflict_inner(source, back, scripted, events);
                    self.ability_update(source);
                }
            }
        }
        // Then the target's own tidying: the curing berries and the refusing
        // abilities. A Rawst eaten the instant the burn lands is a whole
        // turn's chip that never happens.
        self.ability_update(foe);
    }

    pub(super) fn inflict_inner(
        &mut self,
        foe: usize,
        status: Status,
        scripted: bool,
        events: &mut Vec<Event>,
    ) {
        // Safeguard shields the whole team from foe-inflicted statuses, with
        // one hole in it that the sim writes out by name: a Yawn already
        // collecting delivers its sleep regardless. Safeguard's job was to
        // stop the drowsiness going on in the first place.
        if self.sides[foe].safeguard_n > 0 && !self.yawn_landing {
            return;
        }
        // No one sleeps through an Uproar.
        if status == Status::Sleep
            && (0..2).any(|w| {
                // NOT `uproar_ending`: the din's own residual sits at
                // subOrder 11 and Yawn's at 19, so by the time Yawn puts its
                // victim under, the Uproar has already ended and has nothing
                // left to say about it.
                self.sides[w].mon().rampage.is_some_and(|(i, _)| {
                        self.sides[w]
                            .mon()
                            .moves
                            .get(i as usize)
                            .is_some_and(|m| m.entry.id == "uproar")
                    })
                    || (self.sides[w].mon().rampage.is_some()
                        && self.sides[w].mon().locked_move == Some("uproar"))
            })
        {
            return;
        }
        if !self.sides[foe].mon().can_receive(status) {
            return;
        }
        // An ability that simply refuses this status.
        if ability::blocks_status(&self.sides[foe].mon().bearer(), status) {
            return;
        }
        // Bright sunlight prevents freezing outright (the sim's Sunny Day
        // onImmunity hook).
        if status == Status::Freeze && self.effective_weather() == Some(Weather::Sun) {
            return;
        }
        let sleep = match status {
            Status::Sleep if scripted => 2,
            Status::Sleep => 2 + self.rng.below(4) as u8,
            _ => 0,
        };
        let target = self.sides[foe].mon_mut();
        target.status = Some(status);
        target.toxic_n = 0;
        target.sleep_n = sleep;
        target.sleep_skipped = 0;
        if status == Status::Sleep {
            // A fresh sleep shakes off a Nightmare.
            target.nightmared = false;
        }
        events.push(Event::Statused {
            side: foe as u8 + 1,
            status,
        });
    }

    /// Confuse `foe`'s active mon if it can be: not fainted, not already
    /// confused. The clock is 2 under a script (the sim's pinned floor),
    /// 2..=5 in play.
    pub(super) fn confuse(&mut self, foe: usize, scripted: bool, events: &mut Vec<Event>) {
        // Safeguard blocks foe-inflicted confusion too in this era.
        if self.sides[foe].safeguard_n > 0 {
            return;
        }
        let target = self.sides[foe].mon();
        if target.fainted() || target.confusion_n > 0 {
            return;
        }
        // Own Tempo simply refuses the volatile.
        if ability::blocks_confusion(&self.sides[foe].mon().bearer()) {
            return;
        }
        let n = if scripted {
            2
        } else {
            2 + self.rng.below(4) as u8
        };
        self.sides[foe].mon_mut().confusion_n = n;
        self.ability_update(foe);
        events.push(Event::ConfusionStarted {
            side: foe as u8 + 1,
        });
    }
}
