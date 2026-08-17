//! A chosen move, from the can't-move gates to the damage tail.

extern crate alloc;

use alloc::vec::Vec;

use crate::ability;
use crate::item;
use crate::damage::{crit_denominator, damage, Attacker, Defender, MoveUse, Roll};
use crate::data::{
    move_by_id, Boost, FixedDamage, MoveEntry, SecondaryEffect, SideCondition, Status, StatusAction, Weather,
};
use crate::stats::Stat;
use crate::types::Type;

use super::*;

impl Battle {
    pub(super) fn use_move(
        &mut self,
        side: usize,
        index: usize,
        script: Option<SeatScript>,
        events: &mut Vec<Event>,
    ) {
        let foe = 1 - side;
        if self.sides[side].mon().fainted() {
            return;
        }
        // Fake Out's window is counted in `activeMoveActions`, which the sim
        // bumps at the top of `runMove` — so a sleep-lost turn burns it (the
        // mon took its action and was stopped inside it), and an intercepting
        // Pursuit does NOT. The interception is fired from the switch-out
        // hook through `useMove`, which is below runMove, and the mon's own
        // queued action is thrown away: it never spent one. So a Cyndaquil
        // that pursued something off the field can still Fake Out next turn.
        let had_acted = self.sides[side].mon().acted;
        if !self.pursuing {
            self.sides[side].mon_mut().acted = true;
        }
        // In Gen 3 a rampage broken by flinch or full paralysis still ends
        // in fatigue confusion, on the spot. (A miss or a protecting target
        // ends it quietly — those sites clear `rampage` directly.)
        fn break_rampage(b: &mut Battle, side: usize, scripted: bool, events: &mut Vec<Event>) {
            if b.sides[side].mon().rampage.is_some() {
                let uproar = b.sides[side].mon().locked_move == Some("uproar");
                let n = if scripted {
                    2
                } else {
                    2 + b.rng.below(4) as u8
                };
                let mon = b.sides[side].mon_mut();
                // An Uproar is not a Thrash. Its lock is a volatile of its
                // own with its own clock, ticked in the residual phase, and
                // nothing that happens to the move it is swinging shortens
                // it: the din carries on through a miss, a flinch and a full
                // paralysis alike. Only the Thrash family ends here, and only
                // the Thrash family fatigues.
                if !uproar {
                    mon.rampage = None;
                }
                if !uproar && mon.confusion_n == 0 && !mon.fainted() {
                    mon.confusion_n = n;
                    events.push(Event::ConfusionStarted {
                        side: side as u8 + 1,
                    });
                }
            }
        }
        // Destiny Bond and Grudge last exactly until the user's next action.
        self.sides[side].mon_mut().destiny = false;
        self.sides[side].mon_mut().grudged = false;

        // An intercepting Pursuit skips every can't-move gate below. The sim
        // fires it from the switch-out hook through `useMove`, which is the
        // inside caller: it never runs the BeforeMove event, so a paralysed
        // or flinched user still gets its strike in. (Sleep and freeze are
        // refused earlier, by the condition's own guard.)
        let mut asleep_now = false;
        if !self.pursuing && !self.calling {
        // Rage ends the moment its holder tries to act. The sim hangs that
        // on BeforeMove at priority 100, above every gate below — above
        // Truant, sleep, freeze, flinch and full paralysis alike — so an
        // action that never happens still ends the rage. Only Rage landing
        // re-arms it.
        self.sides[side].mon_mut().raging = false;

        // Recharging after Hyper Beam and kin: the whole action is spent,
        // gated even above sleep, matching the games' priority order.
        if self.sides[side].mon().must_recharge {
            self.sides[side].mon_mut().must_recharge = false;
            self.sides[side].mon_mut().recharge_fresh = false;
            self.sides[side].mon_mut().stall_counter = 0;
            events.push(Event::Recharging {
                side: side as u8 + 1,
            });
            return;
        }
        // Fast asleep: the sleep clock ticks down before each action, and at
        // zero the mon wakes and moves that same turn.
        if self.sides[side].mon().status == Some(Status::Sleep) {
            let acting = self.acting_slot(side, index);
            let snoring = self.sides[side]
                .mon()
                .moves
                .get(acting)
                .is_some_and(|m| matches!(m.entry.id, "snore" | "sleeptalk"));
            let mon = self.sides[side].mon_mut();
            if ability::early_bird(&mon.bearer()) {
                mon.sleep_n = mon.sleep_n.saturating_sub(1);
            }
            mon.sleep_n = mon.sleep_n.saturating_sub(1);
            if mon.sleep_n == 0 {
                mon.status = None;
                mon.sleep_skipped = 0;
            } else if snoring {
                // Snore attacks straight out of sleep, and Gen 3 refunds the
                // turn on switch-in rather than counting it.
                mon.sleep_skipped += 1;
                asleep_now = true;
                events.push(Event::Cant {
                    side: side as u8 + 1,
                    status: Status::Sleep,
                });
            } else {
                mon.charging = None;
                mon.charge_fresh = false;
                mon.sleep_skipped = 0;
                // A Thrash-family lock is NOT dropped here: the sim lets the
                // sleeper keep it and settles the matter in the residual
                // phase, which is why a mon slept out of its final swing
                // still wakes up confused.
                if mon.rampage.is_some() && mon.locked_move == Some("uproar") {
                    mon.rampage = None;
                }
                mon.rolling = None;
                mon.fury_n = 0;
                mon.stall_counter = 0;
                events.push(Event::Cant {
                    side: side as u8 + 1,
                    status: Status::Sleep,
                });
                return;
            }
        }
        // Truant: every other turn is spent loafing about, and the turn it
        // arrives counts as one unless the battle has not started.
        if self.sides[side].mon().loafing
            && ability::truant(&self.sides[side].mon().bearer())
        {
            // A loaf ABORTS the move, and `twoturnmove.onMoveAborted` drops
            // the lock — its onEnd taking the move's own volatile with it. So
            // a Slakoth that loafs mid-Bounce comes down: it is no longer out
            // of reach, and the Earthquake it was dodging lands.
            self.sides[side].mon_mut().charging = None;
            self.sides[side].mon_mut().charge_fresh = false;
            events.push(Event::Failed {
                side: side as u8 + 1,
            });
            return;
        }
        // Frozen solid: a 1-in-5 thaw each action in play (scripts pin it
        // off, matching the reference runs). Flame Wheel and Sacred Fire
        // pass THROUGH this gate — but the cure itself only lands when the
        // move actually executes, so a flinch or full paralysis after this
        // gate leaves the user frozen.
        if self.sides[side].mon().status == Some(Status::Freeze) {
            let acting = self.acting_slot(side, index);
            let defrost = self.sides[side]
                .mon()
                .moves
                .get(acting)
                .is_some_and(|slot| matches!(slot.entry.id, "flamewheel" | "sacredfire"));
            let lucky = match script {
                Some(_) => false,
                None => self.rng.below(5) == 0,
            };
            if lucky {
                self.sides[side].mon_mut().status = None;
            } else if !defrost {
                self.sides[side].mon_mut().charging = None;
                self.sides[side].mon_mut().rampage = None;
                self.sides[side].mon_mut().rolling = None;
                self.sides[side].mon_mut().fury_n = 0;
                self.sides[side].mon_mut().stall_counter = 0;
                events.push(Event::Cant {
                    side: side as u8 + 1,
                    status: Status::Freeze,
                });
                return;
            }
        }
        // Flinch: the hit that caused it resolved earlier this turn, so a
        // flinched mon that has not moved yet loses its action. Freeze and
        // sleep outrank it in the games' gate order, hence checking second.
        if self.sides[side].mon().flinched {
            self.sides[side].mon_mut().charging = None;
            break_rampage(self, side, script.is_some(), events);
            self.sides[side].mon_mut().rolling = None;
            self.sides[side].mon_mut().fury_n = 0;
            self.sides[side].mon_mut().stall_counter = 0;
            events.push(Event::Flinched {
                side: side as u8 + 1,
            });
            return;
        }
        // Confusion: the clock ticks before the action; at zero it lifts
        // and the move proceeds. Otherwise a coin (the script's selfhit
        // knob) decides between acting and the 40 BP typeless self-hit.
        if self.sides[side].mon().confusion_n > 0 {
            self.sides[side].mon_mut().confusion_n -= 1;
            if self.sides[side].mon().confusion_n == 0 {
                events.push(Event::ConfusionEnded {
                    side: side as u8 + 1,
                });
            } else {
                let (selfhit, random) = match script {
                    Some(s) => (s.selfhit, s.random),
                    None => (self.rng.below(2) == 0, 85 + self.rng.below(16) as u8),
                };
                if selfhit {
                    self.sides[side].mon_mut().charging = None;
                    self.sides[side].mon_mut().rampage = None;
                    self.sides[side].mon_mut().rolling = None;
                    self.sides[side].mon_mut().fury_n = 0;
                        self.sides[side].mon_mut().stall_counter = 0;
                    let amount = self.confusion_self_hit(side, random);
                    events.push(Event::ConfusedHit {
                        side: side as u8 + 1,
                        amount,
                    });
                    self.announce_faint(side, events);
                    return;
                }
            }
        }
        // Attract, at `onBeforeMovePriority: 2` — under confusion's 3 and
        // over paralysis's. Half a charmed mon's actions are lost, and the
        // coin is rolled whether or not anything else was going to stop it.
        if self.sides[side].mon().attracted_by.is_some() {
            let immobile = match script {
                Some(s) => s.immobile,
                None => self.rng.below(2) == 0,
            };
            if immobile {
                self.sides[side].mon_mut().charging = None;
                break_rampage(self, side, script.is_some(), events);
                self.sides[side].mon_mut().rolling = None;
                self.sides[side].mon_mut().fury_n = 0;
                self.sides[side].mon_mut().stall_counter = 0;
                events.push(Event::Infatuated {
                    side: side as u8 + 1,
                });
                return;
            }
        }
        // Full paralysis: a quarter of a paralyzed mon's actions, decided by
        // the script under test and the RNG in play. No PP is spent.
        if self.sides[side].mon().status == Some(Status::Paralysis) {
            let immobile = match script {
                Some(s) => s.immobile,
                None => self.rng.below(4) == 0,
            };
            if immobile {
                self.sides[side].mon_mut().charging = None;
                break_rampage(self, side, script.is_some(), events);
                self.sides[side].mon_mut().rolling = None;
                self.sides[side].mon_mut().fury_n = 0;
                self.sides[side].mon_mut().stall_counter = 0;
                events.push(Event::FullyParalyzed {
                    side: side as u8 + 1,
                });
                return;
            }
        }
        }
        // Encore overrides the choice with the last move used. The sim does
        // that at EXECUTION time, through `OverrideAction`, which re-checks
        // nothing: an Encore that landed earlier this turn — after its victim
        // had already chosen — forces the move through a Torment or a Disable
        // that would otherwise have greyed it out. An Encore already up when
        // the choice was made is a different matter: the request had greyed
        // everything else out already, so either the encored move was the
        // only offer or there was no offer at all and the answer is a real
        // Struggle, which the sim then leaves alone.
        let encore_forced = self.sides[side].mon().encore_n > 0
            && self.sides[side].mon().encore_fresh
            && self.sides[side].mon().last_used.is_some();
        let index = match (
            self.sides[side].mon().encore_n > 0,
            self.sides[side].mon().last_used,
        ) {
            (true, Some(i)) => i as usize,
            _ => index,
        };
        // Mid two-turn move: the release is forced to the charged slot and
        // its PP was already paid on the charge turn. A rampage forces its
        // slot the same way. Bide and a rolling Rollout force theirs too.
        let ramping = self.sides[side].mon().rampage.is_some()
            || self.sides[side].mon().bide.is_some()
            || self.sides[side].mon().rolling.is_some();
        let releasing = self.sides[side].mon().charging.is_some() || ramping;
        let was_charging = self.sides[side].mon().charging.is_some();
        let index = match self.sides[side].mon().charging {
            Some(i) => {
                self.sides[side].mon_mut().charging = None;
                i as usize
            }
            None => match self.sides[side].mon().rampage {
                Some((i, _)) => i as usize,
                None => index,
            },
        };
        // Any other lock forces its own slot too — a rolling Rollout or Ice
        // Ball, a storing Bide. It has to be the SLOT and not just the move
        // entry, because the PP comes off it and Grudge and Spite both read
        // `last_used` to decide what to drain.
        let index = if releasing {
            match self.sides[side].mon().locked_move {
                Some(id) => self.sides[side]
                    .mon()
                    .moves
                    .iter()
                    .position(|m| m.entry.id == id)
                    .unwrap_or(index),
                None => index,
            }
        } else {
            index
        };
        // A Disabled locked move cannot continue — a mid-charge release, a
        // rolling Rollout or a rampage all lose the turn silently (the
        // sim's disable cant; a broken rampage still fatigues).
        if releasing && self.sides[side].mon().disabled_slot == Some(index as u8) {
            let _ = was_charging;
            break_rampage(self, side, script.is_some(), events);
            self.sides[side].mon_mut().rolling = None;
            self.sides[side].mon_mut().stall_counter = 0;
            events.push(Event::Failed {
                side: side as u8 + 1,
            });
            return;
        }
        let forced = self.forced_entry.take();
        let Some(slot) = forced
            .map(|entry| MoveSlot {
                entry,
                pp: 1,
                typed_as: None,
            })
            .or_else(|| self.sides[side].mon().moves.get(index).copied())
        else {
            events.push(Event::Failed {
                side: side as u8 + 1,
            });
            return;
        };
        // A called move — one a Magic Coat threw back or a Snatch took — is
        // run through `useMove`, which sits BELOW `runMove`. Every "you may
        // not use that move" gate in this era is an onBeforeMove or an
        // onDisableMove, and neither event fires down there: a Choice lock, a
        // Taunt, a Disable, a Torment and an Imprison all have no say in it,
        // and the Struggle they would otherwise force is a REQUEST-time
        // substitution a called move never passes through.
        let forced = forced.is_some();
        // A rampage locked in through Mirror Move keeps swinging the CALLED
        // move on its follow-up turns — run it directly, one announced line.
        let slot = if releasing {
            match self.sides[side].mon().locked_move {
                Some(id) if id != slot.entry.id => match crate::data::move_by_id(id) {
                    Some(e) => MoveSlot {
                        entry: e,
                        pp: 1,
                        typed_as: None,
                    },
                    None => slot,
                },
                _ => slot,
            }
        } else {
            slot
        };
        // A storing Bide sits silent — the games log nothing for the two
        // storing turns — and cannot be re-chosen out of.
        if let Some((stored, left)) = self.sides[side].mon().bide {
            if left > 0 {
                self.sides[side].mon_mut().bide = Some((stored, left - 1));
                return;
            }
        }

        // Anything but another Protect/Endure resets the stall gamble.
        if !matches!(
            slot.entry.status_action,
            Some(StatusAction::Protect | StatusAction::Endure)
        ) {
            self.sides[side].mon_mut().stall_counter = 0;
        }
        // Taunt semantics split on WHEN it landed: the turn it lands, the
        // victim's already-chosen status move is simply blocked (a lost
        // turn); a status move CHOSEN while taunted becomes Struggle —
        // typeless 50 power, a quarter recoil — as does running out of PP.
        let status_movish = slot.entry.power == 0
            && slot.entry.fixed.is_none()
            && !slot.entry.ohko
            && !matches!(slot.entry.id, "counter" | "mirrorcoat" | "spitup");
        // Struggle is substituted at REQUEST time: with no usable move,
        // `getMoves()` comes back empty and `chooseMove` queues Struggle, so
        // the dead slot is gone before anyone moves. Every mid-turn gate below
        // is then asked about STRUGGLE, which sails past all of them — Taunt
        // only stops a Status move and Struggle is Physical, Disable only
        // stops the one move it named, and Imprison spells out
        // `move.id !== "struggle"`. So a mon that was already Struggling when
        // the turn opened cannot lose the turn to any of them.
        //
        // It has to be read as the state stood AT THE REQUEST, which is why
        // every `_fresh` flag disqualifies its own reason: a Disable, a
        // Torment or an Imprison that landed earlier THIS turn was not there
        // when the choice was made, so it cannot be what put Struggle in the
        // queue — and those are exactly the lost-turn cases below.
        let struggling_at_request = !releasing && {
            let mon = self.sides[side].mon();
            let picked = mon.moves.get(index).map(|m| m.entry.id);
            self.pp0_at_choice[side]
                || (mon.disabled_slot == Some(index as u8) && !mon.disable_fresh)
                || (mon.tormented && !mon.torment_fresh && mon.last_used_id == picked)
                || (mon.taunt_n == 1 && status_movish)
                || (self.sealed_at_choice[side] && !self.sides[foe].mon().imprison_fresh)
                || mon
                    .choice_locked
                    .is_some_and(|id| picked.is_some_and(|p| p != id))
        };
        if !struggling_at_request && self.sides[side].mon().taunt_n == 2 && status_movish {
            events.push(Event::Failed {
                side: side as u8 + 1,
            });
            return;
        }
        let taunted_out = self.sides[side].mon().taunt_n == 1
            && status_movish
            && !encore_forced
            && !forced;
        // Torment: the same move twice in a row becomes Struggle. So does
        // a Disabled slot, or a move the imprisoning foe also knows.
        let tormented_out = self.sides[side].mon().tormented
            && !self.sides[side].mon().torment_fresh
            && self.sides[side].mon().last_used_id == Some(slot.entry.id)
            && !releasing
            && !encore_forced
            && !forced;
        if !struggling_at_request
            && self.sides[side].mon().disabled_slot == Some(index as u8)
            && self.sides[side].mon().disable_fresh
            && !releasing
            && !forced
        {
            // Disabled mid-turn: the chosen move is simply lost.
            self.sides[side].mon_mut().disable_fresh = false;
            self.sides[side].mon_mut().stall_counter = 0;
            events.push(Event::Failed {
                side: side as u8 + 1,
            });
            return;
        }
        let disabled_out = self.sides[side].mon().disabled_slot == Some(index as u8)
            && !releasing
            && !encore_forced
            && !forced;
        // The seal as it stood when the choice was made, plus the live one —
        // an Imprison that landed EARLIER this turn seals a mon that has not
        // moved yet, and that is the `imprison_fresh` path just below.
        let sealed = self.sealed_at_choice[side]
            || (self.sides[foe].mon().imprisoning
                && self.sides[foe]
                    .mon()
                    .moves
                    .iter()
                    .any(|m| m.entry.id == slot.entry.id));
        if !struggling_at_request
            && sealed
            && self.sides[foe].mon().imprison_fresh
            && !releasing
            && !forced
        {
            // Imprison landed earlier this same turn: the chosen move is
            // simply lost — no move line, no PP, no Struggle.
            self.sides[side].mon_mut().stall_counter = 0;
            events.push(Event::Failed {
                side: side as u8 + 1,
            });
            return;
        }
        let imprisoned_out = sealed && !releasing && !encore_forced && !forced;
        // A Choice Band greys its move out the same way Disable does, so a
        // choice outside the lock is no more usable than a disabled one.
        let choice_locked_out = !releasing
            && !encore_forced
            && !forced
            && self.sides[side]
                .mon()
                .choice_locked
                .is_some_and(|id| id != slot.entry.id);
        if slot.pp == 0
            && !releasing
            && !self.pp0_at_choice[side]
            && !(taunted_out || tormented_out || disabled_out || imprisoned_out
                || choice_locked_out)
        {
            // Drained to zero AFTER the choice was made: the sim's runMove
            // hits "cant: nopp" — a silent lost turn, no Struggle.
            events.push(Event::Failed {
                side: side as u8 + 1,
            });
            return;
        }
        let struggling = (taunted_out
            || tormented_out
            || disabled_out
            || imprisoned_out
            || choice_locked_out
            || (slot.pp == 0 && !releasing && self.pp0_at_choice[side]))
            && !releasing;
        let slot = if struggling {
            MoveSlot {
                entry: &crate::data::STRUGGLE,
                pp: 1,
                typed_as: None,
            }
        } else {
            slot
        };
        if !releasing && !struggling && !forced {
            // Pressure charges an extra point for anything aimed across the
            // field. A move the user turns on itself is free of it.
            let cost = if ability::pressure(&self.sides[1 - side].mon().bearer())
                && !self.sides[1 - side].mon().fainted()
                && slot.entry.pressured
                && (slot.entry.id != "curse" || {
                    let (t1, t2) = self.sides[side].mon().types();
                    t1 == Type::Ghost || t2 == Type::Ghost
                })
            {
                2
            } else {
                1
            };
            let pp = &mut self.sides[side].mon_mut().moves[index].pp;
            *pp = pp.saturating_sub(cost);
        }
        // A defrosting move thaws its user the moment it actually goes off.
        if self.sides[side].mon().status == Some(Status::Freeze)
            && matches!(slot.entry.id, "flamewheel" | "sacredfire")
        {
            self.sides[side].mon_mut().status = None;
        }
        // Note what this mon committed to. The Choice Band does not clamp
        // here: gen 3 has no Choice Band of its own and inherits gen 4's,
        // which hangs the lock off AFTER the move — so it reads the item the
        // mon holds once the move is done, not the one it started with. A
        // Covet that steals a Band locks the thief into Covet.
        self.committed_move[side] = Some(slot.entry.id);
        events.push(Event::Used {
            side: side as u8 + 1,
            move_index: index,
        });
        // `moveUsed` lives in `runMove`, and a called move never goes through
        // it — gen 3's own `useMoveInner` writes `lastMoveUsed` and nothing
        // else. So a Magic Coat bounce and a Snatch both leave their user's
        // memory of what it last did completely alone: the bouncer's lastMove
        // is still Magic Coat, which is what a Torment greys out next turn,
        // and a Sketch aimed at a thief copies SNATCH rather than the move it
        // took. Pursuit is the exception the sim writes by hand, and it comes
        // through here with `forced` false, so it still records.
        //
        // (One knowing gap: our single field stands in for both `lastMove`
        // and `lastMoveUsed`, so a Conversion 2 answering a bounced move sees
        // nothing where the sim would see the move. No seed reaches it.)
        if !forced {
            let mon = self.sides[side].mon_mut();
            mon.last_used = if struggling { None } else { Some(index as u8) };
            mon.last_used_id = Some(slot.entry.id);
            mon.last_missed = false;
        }
        // `lastMoveUsed` is a different register and `useMoveInner` writes it
        // for EVERY move that goes off, called ones included.
        self.sides[side].mon_mut().last_move_used_id = Some(slot.entry.id);

        // No living foe to aim at: the sim logs the move and stops there
        // (`-notarget`), PP already spent. Self- and field-aimed moves go
        // off regardless of whether the other slot is empty.
        if slot.entry.needs_target
            && self.sides[foe].mon().fainted()
            && !matches!(slot.entry.id, "futuresight" | "doomdesire")
        {
            return;
        }

        // Nature Power becomes Swift in the sim's default arena; Hidden
        // Power under the fuzz's uniform maxed IVs is Dark 70.
        let chosen_id = slot.entry.id;
        let slot = if slot.entry.id == "metronome" {
            // The sim samples the num-sorted eligible list; pinned, that is
            // its first entry — Pound. Play keeps a real random pick.
            let called: &'static MoveEntry = match script {
                Some(_) => move_by_id("pound").expect("pound"),
                None => {
                    let mut pick = move_by_id("pound").expect("pound");
                    for _ in 0..16 {
                        let i = self.rng.below(crate::data::MOVES.len() as u32) as usize;
                        let cand = &crate::data::MOVES[i];
                        if !matches!(
                            cand.id,
                            "metronome" | "struggle" | "mirrormove" | "sketch" | "mimic"
                        ) {
                            pick = cand;
                            break;
                        }
                    }
                    pick
                }
            };
            // Both lines are announced: Metronome, then the call.
            events.push(Event::Used {
                side: side as u8 + 1,
                move_index: index,
            });
            MoveSlot {
                entry: called,
                pp: 1,
                typed_as: None,
            }
        } else if slot.entry.id == "sleeptalk" {
            // Only out of a sleep, and it calls one of the user's OWN moves:
            // anything flagged nosleeptalk or charge is skipped, and a
            // pinned sample lands on the first survivor. A pick with no PP
            // left is a silent lost turn rather than a call.
            if !asleep_now {
                events.push(Event::Failed { side: side as u8 + 1 });
                return;
            }
            let pick = self.sides[side]
                .mon()
                .moves
                .iter()
                .find(|m| !m.entry.no_sleep_talk && !m.entry.charge)
                .copied();
            match pick {
                None => {
                    events.push(Event::Failed { side: side as u8 + 1 });
                    return;
                }
                Some(m) if m.pp == 0 => return,
                Some(m) => {
                    events.push(Event::Used { side: side as u8 + 1, move_index: index });
                    MoveSlot { entry: m.entry, pp: 1, typed_as: None }
                }
            }
        } else if slot.entry.id == "assist" {
            // Assist rummages through the REST of the party, in the sim's
            // own `side.pokemon` order, and calls the first move that is not
            // on the noassist list.
            let called = {
                let s = &self.sides[side];
                s.order
                    .iter()
                    .copied()
                    .filter(|&i| i != s.active)
                    .flat_map(|i| s.party[i].moves.iter())
                    .find(|m| !m.entry.no_assist)
                    .map(|m| m.entry)
            };
            match called {
                None => {
                    events.push(Event::Failed { side: side as u8 + 1 });
                    return;
                }
                Some(e) => {
                    events.push(Event::Used { side: side as u8 + 1, move_index: index });
                    MoveSlot { entry: e, pp: 1, typed_as: None }
                }
            }
        } else if slot.entry.id == "naturepower" {
            // Nature Power rolls its own 95 accuracy; a miss stops before
            // Swift is even called (one log line, not two).
            let np_hit = match script {
                Some(s) => s.hit,
                None => self.rng.below(100) < 95,
            };
            if !np_hit {
                return;
            }
            // The games announce both: Nature Power, then the called Swift.
            events.push(Event::Used {
                side: side as u8 + 1,
                move_index: index,
            });
            MoveSlot {
                entry: move_by_id("swift").expect("swift"),
                pp: 1,
                typed_as: None,
            }
        } else if slot.entry.id == "hiddenpower" && slot.typed_as.is_none() {
            MoveSlot {
                entry: slot.entry,
                pp: slot.pp,
                typed_as: Some(Type::Dark),
            }
        } else {
            slot
        };

        // A called move can itself be a caller: Assist and Sleep Talk both
        // reach Nature Power, and the sim's useMove simply recurses. Run the
        // one chain that exists in this era rather than stopping a link
        // short, which left the call announced and nothing coming out of it.
        let slot = if slot.entry.id == "naturepower" {
            let np_hit = match script {
                Some(s) => s.hit,
                None => self.rng.below(100) < 95,
            };
            if !np_hit {
                return;
            }
            events.push(Event::Used {
                side: side as u8 + 1,
                move_index: index,
            });
            MoveSlot {
                entry: move_by_id("swift").expect("swift"),
                pp: 1,
                typed_as: None,
            }
        } else {
            slot
        };

        // Mirror Move plays back the move the foe last HIT this mon with
        // (the sim's attacked-by book, damaging or status alike) — refusing
        // the noMirror set and a move the foe no longer knows. It swaps in
        // EARLY, before every gate, so the called move runs the whole
        // pipeline: its own accuracy, protect, immunity, fixed-damage arms.
        let slot = if slot.entry.id == "mirrormove" {
            // The attacker has to still BE there with a move to its name:
            // the sim reads `lastAttackedBy.source.lastMove`, and switching
            // out wipes that, so a mon that hit and left leaves nothing to
            // mirror even after it comes back.
            let same_attacker = self.sides[side].mon().last_hit_by_slot
                == Some(self.sides[foe].active)
                && self.sides[foe].mon().last_used_id.is_some();
            let hit_by = self.sides[side].mon().last_hit_by.filter(|_| same_attacker);
            let callable = hit_by
                .filter(|id| {
                    !matches!(
                        *id,
                        "assist"
                            | "curse"
                            | "doomdesire"
                            | "focuspunch"
                            | "futuresight"
                            | "magiccoat"
                            | "metronome"
                            | "mimic"
                            | "mirrormove"
                            | "naturepower"
                            | "psychup"
                            | "roleplay"
                            | "sketch"
                            | "sleeptalk"
                            | "spikes"
                            | "spitup"
                            | "taunt"
                            | "teeterdance"
                            | "transform"
                    ) && self.sides[foe]
                        .mon()
                        .moves
                        .iter()
                        .any(|m| m.entry.id == *id)
                })
                .and_then(crate::data::move_by_id);
            match callable {
                None => {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                    return;
                }
                Some(e) => {
                    // Both lines are announced: Mirror Move, then the call.
                    events.push(Event::Used {
                        side: side as u8 + 1,
                        move_index: index,
                    });
                    MoveSlot {
                        entry: e,
                        pp: 1,
                        typed_as: None,
                    }
                }
            }
        } else {
            slot
        };
        // Whether a caller — Metronome, Sleep Talk, Assist, Nature Power,
        // Mirror Move — put this move here rather than the player.
        let called = slot.entry.id != chosen_id;
        // `useMoveInner` writes `lastMoveUsed` at its very top, for EVERY
        // invocation and before any success check, and a called move
        // re-enters it. So the register ends up holding the CALLEE, not the
        // caller: a Conversion 2 answering an Assist reads the move the
        // Assist reached for, right down a nested Assist into Nature Power
        // into Swift. The write above this chain covers the caller that
        // found nothing and bailed, which is what the sim leaves there too.
        if called {
            self.sides[side].mon_mut().last_move_used_id = Some(slot.entry.id);
        }
        // Counter and Mirror Coat read a volatile that only
        // `beforeTurnCallback` creates, and the queue attaches a
        // `beforeTurnMove` action ONLY for a seat's own top-level move. A
        // called one therefore finds no volatile, fails its `onTry`, and —
        // in this era, whose `tryMoveHit` skips the `-fail` the modern one
        // adds — does so in silence.
        if called && matches!(slot.entry.id, "counter" | "mirrorcoat") {
            return;
        }

        // Snatch answers `onAnyPrepareHit`, which fires once per `useMove` —
        // so it tests the move actually being prepared, and that is the move
        // AFTER the callers have had their say. An Assist or a Sleep Talk
        // carries no snatch flag itself, but the move it reaches for
        // re-enters useMove and gets its own PrepareHit, so a Recover pulled
        // out of an Assist is stolen just the same. The original user has
        // already spent its PP and had both lines logged; what it loses is
        // everything after this.
        if slot.entry.snatchable
            && !self.sides[foe].mon().fainted()
            && self.sides[foe].mon().snatching
        {
            self.sides[foe].mon_mut().snatching = false;
            // Snatch runs a DeductPP event of its own against the mon it is
            // robbing, and Pressure answers it: the thief pays a SECOND point
            // for the Snatch it already spent one on. The sim charges that to
            // Snatch's slot, not to the move being taken.
            if ability::pressure(&self.sides[side].mon().bearer()) {
                if let Some(ms) = self.sides[foe]
                    .mon_mut()
                    .moves
                    .iter_mut()
                    .find(|m| m.entry.id == "snatch")
                {
                    ms.pp = ms.pp.saturating_sub(1);
                }
            }
            self.pending_call = Some((foe, slot.entry));
            return;
        }

        // Fury Cutter's count belongs to the move that ACTUALLY goes off,
        // not the one announced: a Mirror Move or a Sleep Talk that plays
        // Fury Cutter back keeps the ramp climbing, which is why this sits
        // after the call substitutions rather than with the other
        // announced-move bookkeeping.
        if slot.entry.id != "furycutter" {
            self.sides[side].mon_mut().fury_n = 0;
        }

        // The charge turn of a two-turn move: announce, tuck the slot away,
        // and stop. Skull Bash's era perk raises Defense on the way down.
        let instant_solar = slot.entry.id == "solarbeam" && self.effective_weather() == Some(Weather::Sun);
        if slot.entry.charge && !releasing && !instant_solar {
            events.push(Event::Charging {
                side: side as u8 + 1,
            });
            if slot.entry.id == "skullbash" {
                self.sides[side].mon_mut().apply_boost(Boost::Def, 1);
                events.push(Event::Boosted {
                    side: side as u8 + 1,
                    boost: Boost::Def,
                    delta: 1,
                });
            }
            self.sides[side].mon_mut().charging = Some(index as u8);
            self.sides[side].mon_mut().charge_fresh = true;
            self.sides[side].mon_mut().locked_move = Some(slot.entry.id);
            return;
        }

        // Rollout and Ice Ball lock five doubling uses; Bide stores for two
        // turns; Uproar rolls like a rampage without the hangover.
        // A Rollout swung out of SLEEP leaves no lock at all. The volatile is
        // added in `onModifyMove`, which bails on `pokemon.status === 'slp'`,
        // so a Sleep Talk that reaches for Rollout gets one thirty-power hit
        // and nothing else — no lock, no counter, and the sleeper keeps its
        // own move for next turn.
        if matches!(slot.entry.id, "rollout" | "iceball")
            && self.sides[side].mon().rolling.is_none()
            && !asleep_now
        {
            self.sides[side].mon_mut().rolling = Some(0);
            self.sides[side].mon_mut().locked_move = Some(slot.entry.id);
        }
        if slot.entry.id == "bide" && self.sides[side].mon().bide.is_none() {
            self.sides[side].mon_mut().bide = Some((0, 2));
            self.sides[side].mon_mut().locked_move = Some(slot.entry.id);
            events.push(Event::Charging {
                side: side as u8 + 1,
            });
            return;
        }
        // The unleash: double everything stored, typeless, at the foe.
        if slot.entry.id == "bide" {
            let (stored, _) = self.sides[side].mon().bide.unwrap();
            self.sides[side].mon_mut().bide = None;
            let amount = stored.saturating_mul(2);
            if amount == 0 {
                events.push(Event::Failed {
                    side: side as u8 + 1,
                });
                return;
            }
            self.flat_hit(side, foe, &slot, amount, Some(false), false, false, script, events);
            return;
        }
        // The thrash family locks in: the games roll 2..3 total attacks
        // (a script pins the floor). The lock starts on first use. Uproar
        // rides the same lock for its pinned 2 (2..5 in play) turns.
        if matches!(
            slot.entry.id,
            "thrash" | "petaldance" | "outrage" | "uproar"
        ) {
            if ramping {
                // Every swing that actually goes off re-arms the lock's own
                // two-turn clock, but only while swings are still owed: the
                // last one lets the clock run out, and that is what fatigues.
                let mon = self.sides[side].mon_mut();
                if mon.rampage.is_some_and(|(_, owed)| owed >= 2) {
                    mon.rampage_dur = 2;
                }
            } else {
                let total: u8 = match script {
                    Some(_) => 2,
                    None if slot.entry.id == "uproar" => 2 + self.rng.below(4) as u8,
                    None => 2 + self.rng.below(2) as u8,
                };
                let mon = self.sides[side].mon_mut();
                // Uproar keeps counting its own turns down as it attacks;
                // the Thrash family hands its countdown to the residual
                // phase and stores the swings owed here instead.
                mon.rampage = Some((index as u8, total));
                mon.rampage_dur = 2;
                mon.locked_move = Some(slot.entry.id);
            }
        }

        // Spit Up with an empty bank simply fails; otherwise the bank is
        // spent whatever happens next.
        if slot.entry.id == "spitup" {
            if self.sides[side].mon().stockpile_n == 0 {
                events.push(Event::Failed {
                    side: side as u8 + 1,
                });
                return;
            }
        }

        // Focus Punch loses its focus — and the turn — if anything hit the
        // user before it moved. The sim checks in the move's own onTry,
        // after every gate has passed and the PP is already spent.
        if slot.entry.id == "focuspunch"
            && (self.taken_physical[side] > 0 || self.taken_special[side] > 0)
        {
            events.push(Event::Failed {
                side: side as u8 + 1,
            });
            return;
        }

        // Fake Out only works on the user's first action on the field.
        if slot.entry.id == "fakeout" && had_acted {
            events.push(Event::Failed {
                side: side as u8 + 1,
            });
            return;
        }
        // Dream Eater only bites a sleeping target.
        if slot.entry.id == "dreameater" && self.sides[foe].mon().status != Some(Status::Sleep) {
            events.push(Event::Failed {
                side: side as u8 + 1,
            });
            return;
        }
        // Present: the sim's random(10) picks heal or a power tier; the
        // pinned roll routes to the secondary knob — floor is the HEAL
        // branch (a quarter of the target's max, failing at full HP).
        if slot.entry.id == "present" {
            let heal_branch = match script {
                Some(sc) => sc.secondary,
                None => self.rng.below(10) < 2,
            };
            if heal_branch {
                let target = self.sides[foe].mon_mut();
                let amount = (target.max_hp / 4).max(1).min(target.max_hp - target.hp);
                if amount == 0 {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                } else {
                    target.hp += amount;
                    events.push(Event::Healed {
                        side: foe as u8 + 1,
                        amount,
                    });
                }
                return;
            }
        }

        // Brick Break smashes the target's screens before it hits, unless
        // the target is outright immune.
        if slot.entry.id == "brickbreak"
            && crate::types::effectiveness_against(
                slot.move_type(),
                self.immunity_types(foe, slot.move_type()),
            ) != 0
        {
            for cond in [SideCondition::Reflect, SideCondition::LightScreen] {
                let n = self.sides[foe].condition_n(cond);
                if *n > 0 {
                    *n = 0;
                    events.push(Event::SideEnded {
                        side: foe as u8 + 1,
                        condition: cond,
                    });
                }
            }
        }

        // Explosion/Self-Destruct: the user faints ON USE, before the hit
        // resolves — a miss or an immune target changes nothing about that.
        if slot.entry.selfdestruct
            && ability::damp_present(
                &self.sides[side].mon().bearer(),
                &self.sides[foe].mon().bearer(),
            )
        {
            events.push(Event::Failed {
                side: side as u8 + 1,
            });
            return;
        }
        let boom = slot.entry.selfdestruct;
        // The user's own faint is queued before the hit for these, which is
        // what keeps a Grudge from draining their PP.
        self.self_destructed = boom;
        if boom {
            self.sides[side].mon_mut().hp = 0;
        }

        // Accuracy: 0 in the table means the move never misses. A scripted
        // seat's hit/miss is decided by the script, but only for moves that
        // CAN miss, matching how the reference sim's accuracy step works.
        // Unscripted rolls fold in the accuracy/evasion stages the Gen 3
        // way: one combined stage, (3+s)/3 above zero and 3/(3-s) below.
        // Thunder rides the weather: unmissable in rain, halved in sun.
        let acc = if slot.entry.id == "thunder" {
            match self.effective_weather() {
                Some(Weather::Rain) => 0,
                Some(Weather::Sun) => 50,
                _ => slot.entry.accuracy,
            }
        } else {
            slot.entry.accuracy
        };
        // Abilities move the accuracy BEFORE the stages, in one chain: the
        // sim runs Compound Eyes, Sand Veil and Hustle on the same event and
        // applies the result once. A move that cannot miss is left alone —
        // the sim's accuracy is `true` there, not a number to modify.
        let acc = if acc == 0 {
            0
        } else {
            let chain = ability::accuracy_chain(
                &self.sides[side].mon().bearer(),
                &self.sides[foe].mon().bearer(),
                slot.move_type(),
                self.effective_weather() == Some(Weather::Sandstorm),
            );
            let after = chain.apply(acc as u32);
            item::accuracy_after_item(&self.sides[foe].mon().holder(), after).clamp(1, 100) as u8
        };
        // Pursuit's onModifyMove sets accuracy true against a mon that is
        // already leaving, so the interception cannot whiff.
        let sure = (self.sides[side].mon().sure_hit > 0
            && self.sides[side].mon().sure_hit_on as usize == self.sides[foe].active)
            || (self.pursuing && slot.entry.id == "pursuit");
        // Nothing consumes the lock: the sim's volatile simply runs out its
        // two-turn duration, so clearing it here both let a second Mind
        // Reader re-apply it and cut it short by a turn.
        let hit = sure
            || match script {
                Some(s) => acc == 0 || s.hit,
                None => {
                    acc == 0 || {
                        let eva = if self.sides[foe].mon().identified {
                            0
                        } else {
                            self.sides[foe].mon().eva_stage
                        };
                        let s = (self.sides[side].mon().acc_stage - eva).clamp(-6, 6) as i32;
                        let eff = if s >= 0 {
                            acc as u32 * (3 + s as u32) / 3
                        } else {
                            acc as u32 * 3 / (3 - s) as u32
                        };
                        self.rng.below(100) < eff
                    }
                }
            };
        // A semi-invulnerable target (mid Fly/Dig/Bounce/Dive) dodges
        // everything aimed at it — no accuracy roll happens — except its
        // pierce moves, which land and double their power. Self-targeted
        // actions ignore the dodge; they never aim at the foe.
        let mut pierce_mult: u16 = 1;
        let mut pierce_power_mult: u16 = 1;
        let self_targeted = matches!(
            slot.entry.status_action,
            Some(
                StatusAction::BoostSelf(_)
                    | StatusAction::HealHalf
                    | StatusAction::Team(_)
                    | StatusAction::SetWeather(_)
                    | StatusAction::Protect
                    | StatusAction::Endure
                    | StatusAction::Focus
                    | StatusAction::Rest
                    | StatusAction::BellyDrum
                    | StatusAction::Stockpile
                    | StatusAction::Swallow
                    | StatusAction::WeatherHeal
                    | StatusAction::Refresh
                    | StatusAction::Wish
                    | StatusAction::DestinyBond
                    | StatusAction::Grudge
                    | StatusAction::ChargeUp
                    | StatusAction::Sport(_)
                    | StatusAction::Spikes
                    | StatusAction::Haze
                    | StatusAction::PerishSong
                    | StatusAction::Minimize
                    // Psych Up is NOT in this list: it reads the foe's book,
                    // and `target: "normal"` means it is aimed there. So it
                    // answers the Invulnerability event like any other aimed
                    // move and misses a mon that is mid-Bounce or underground.
                    // It stays out of Protect's way on its own — the entry
                    // carries no `protect` flag in this era.
                    // These read `target: "self"` in this era, and gen 3's
                    // `useMoveInner` rewrites the target to the user before
                    // the Invulnerability event is asked — so a foe that is
                    // mid-Bounce cannot make any of them miss.
                    | StatusAction::Recycle
                    | StatusAction::MirrorMove
                    | StatusAction::NaturePower
                    | StatusAction::Camouflage
                    | StatusAction::Conversion
                    | StatusAction::Imprison
                    | StatusAction::Substitute
                    | StatusAction::Ingrain
                    | StatusAction::HealBell
                    | StatusAction::MagicCoat
                    | StatusAction::Snatch
                    | StatusAction::NoopSuccess
                    | StatusAction::BatonPass
                    | StatusAction::SleepTalk
                    | StatusAction::Assist
            )
        ) || (matches!(slot.entry.status_action, Some(StatusAction::Curse))
            && {
                // Non-Ghost Curse retargets SELF (the sim's nonGhostTarget):
                // no shield and no semi-invulnerable foe can stop it.
                let (t1, t2) = self.sides[side].mon().types();
                t1 != Type::Ghost && t2 != Type::Ghost
            });
        // A shield only stops what carries the protect flag — Sketch,
        // Transform and the delayed hits all go straight through one.
        if !self_targeted && slot.entry.protectable && self.sides[foe].mon().protected {
            // A shielded target breaks a rampage the way a miss does —
            // quietly on first use, with fatigue confusion once the lock
            // is running — and a rolling Rollout resets to a fresh choice.
            if ramping {
                break_rampage(self, side, script.is_some(), events);
            } else {
                self.sides[side].mon_mut().rampage = None;
            }
            self.sides[side].mon_mut().rolling = None;
            self.sides[side].mon_mut().fury_n = 0;
            events.push(Event::Failed {
                side: side as u8 + 1,
            });
            // The kicks crash into the shield all the same.
            if matches!(slot.entry.id, "highjumpkick" | "jumpkick") {
                self.kick_crash(side, foe, &slot, script, events);
                return;
            }
            if boom {
                self.resolve_faints(side, foe, events);
            }
            return;
        }
        // A delayed hit is aimed at a slot, not a mon: the sim exempts
        // anything flagged `futuremove` from both the no-target check and
        // the invulnerability one, so it launches at a foe that is
        // underground and at an empty slot alike.
        let futuremove = matches!(slot.entry.id, "futuresight" | "doomdesire");
        if !self_targeted && !futuremove {
            if let Some(via) = self.sides[foe].mon().semi_invulnerable() {
                let pierces = match via {
                    "fly" | "bounce" => {
                        matches!(
                            slot.entry.id,
                            "gust" | "twister" | "thunder" | "skyuppercut"
                        )
                    }
                    "dig" => matches!(slot.entry.id, "earthquake" | "magnitude"),
                    "dive" => matches!(slot.entry.id, "surf" | "whirlpool"),
                    _ => false,
                };
                // A taken aim (Mind Reader, Lock-On) reaches a mon that is
                // not even on the field: the sim's lockon condition answers
                // the Invulnerability event as well as the accuracy one.
                if !pierces && !sure {
                    // A dodge IS a miss: same bookkeeping — including the
                    // fatigue confusion of a broken rampage lock — and the
                    // kicks still crash for half what they would have dealt.
                    self.sides[side].mon_mut().last_missed = true;
                    if ramping {
                        break_rampage(self, side, script.is_some(), events);
                    } else {
                        self.sides[side].mon_mut().rampage = None;
                    }
                    self.sides[side].mon_mut().fury_n = 0;
                    self.sides[side].mon_mut().rolling = None;
                    if matches!(slot.entry.id, "highjumpkick" | "jumpkick") {
                        self.kick_crash(side, foe, &slot, script, events);
                        return;
                    }
                    if boom {
                        self.resolve_faints(side, foe, events);
                    }
                    return;
                }
                // WHERE the pierce doubles depends on the hideout: Bounce
                // doubles gust/twister at BASE POWER, while Fly, Dig and
                // Dive all double at the sim's ModifyDamage stage.
                if via == "bounce" && matches!(slot.entry.id, "gust" | "twister") {
                    pierce_power_mult = 2;
                } else if matches!(
                    slot.entry.id,
                    "gust" | "twister" | "earthquake" | "magnitude" | "surf" | "whirlpool"
                ) {
                    pierce_mult = 2;
                }
            }
        }

        // One-hit KO: fails outright against a higher-level target, is
        // stopped by type immunity, and otherwise its hit IS the KO.
        if slot.entry.ohko && ability::blocks_ohko(&self.sides[foe].mon().bearer()) {
            events.push(Event::Damage {
                side: foe as u8 + 1,
                amount: 0,
                effectiveness: 0,
                crit: false,
            });
            return;
        }
        if slot.entry.ohko {
            let eff = crate::types::effectiveness_against(
                slot.move_type(),
                self.immunity_types(foe, slot.move_type()),
            );
            if eff == 0
                || self.wonder_guard_blocks(foe, slot.move_type())
                || ability::immune_to_type(&self.sides[foe].mon().bearer(), slot.move_type())
            {
                events.push(Event::Damage {
                    side: foe as u8 + 1,
                    amount: 0,
                    effectiveness: 0,
                    crit: false,
                });
                return;
            }
            if self.sides[foe].mon().level > self.sides[side].mon().level {
                events.push(Event::Failed {
                    side: side as u8 + 1,
                });
                return;
            }
            if !hit {
                return;
            }
            if self.sides[foe].mon().sub_hp > 0 {
                let amount = self.sides[foe].mon().sub_hp;
                self.sides[foe].mon_mut().sub_hp = 0;
                events.push(Event::SubDamage {
                    side: foe as u8 + 1,
                    amount,
                });
                events.push(Event::SubBroke {
                    side: foe as u8 + 1,
                });
                return;
            }
            let survives = self.survives_at_one(foe);
            let mon = self.sides[foe].mon_mut();
            let amount = if survives {
                mon.hp.saturating_sub(1)
            } else {
                mon.hp
            };
            mon.hp -= amount;
            mon.last_hit_by = Some(slot.entry.id);
            let who = self.sides[side].active;
            self.sides[foe].mon_mut().last_hit_by_slot = Some(who);
            events.push(Event::Damage {
                side: foe as u8 + 1,
                amount,
                effectiveness: 100,
                crit: false,
            });
            // A one-hit KO is still a hit, and its victim still answers it:
            // Rough Skin grazes the mon that just killed it, and Horn Drill
            // is a hand on your skin like anything else.
            self.on_damaged(side, foe, &slot, slot.move_type(), amount, script, events);
            self.shell_bell(side, amount, events);
            self.resolve_faints(side, foe, events);
            return;
        }

        // Fixed damage skips the formula but not the type chart: Seismic
        // Toss still bounces off a Ghost in this era.
        if let Some(kind) = slot.entry.fixed {
            if !hit {
                return;
            }
            let eff = crate::types::effectiveness_against(
                slot.move_type(),
                self.immunity_types(foe, slot.move_type()),
            );
            if eff == 0 || self.wonder_guard_blocks(foe, slot.move_type()) {
                events.push(Event::Damage {
                    side: foe as u8 + 1,
                    amount: 0,
                    effectiveness: 0,
                    crit: false,
                });
                return;
            }
            let amount = match kind {
                FixedDamage::Flat(n) => n,
                FixedDamage::Level => self.sides[side].mon().level as u16,
                FixedDamage::Half => (self.sides[foe].mon().hp / 2).max(1),
            };
            self.flat_hit(side, foe, &slot, amount, None, true, true, script, events);
            return;
        }

        // Endeavor drags the target's HP down to the user's — through the
        // chart, never a substitute, and failing upward.
        if slot.entry.id == "endeavor" {
            if !hit {
                return;
            }
            if crate::types::effectiveness_against(
                slot.move_type(),
                self.immunity_types(foe, slot.move_type()),
            ) == 0
            {
                events.push(Event::Damage {
                    side: foe as u8 + 1,
                    amount: 0,
                    effectiveness: 0,
                    crit: false,
                });
                return;
            }
            let (uhp, thp) = (self.sides[side].mon().hp, self.sides[foe].mon().hp);
            if self.sides[foe].mon().sub_hp > 0 || uhp >= thp {
                if self.sides[foe].mon().sub_hp == 0 {
                    self.note_hit_by(side, foe, slot.entry.id);
                }
                events.push(Event::Failed {
                    side: side as u8 + 1,
                });
                return;
            }
            let amount = thp - uhp;
            self.sides[foe].mon_mut().hp = uhp;
            self.note_hit_by(side, foe, slot.entry.id);
            self.taken_physical[foe] = amount;
            events.push(Event::Damage {
                side: foe as u8 + 1,
                amount,
                effectiveness: 100,
                crit: false,
            });
            self.kings_rock(side, foe, &slot, script);
            self.took_a_hit(foe, amount, events);
            self.on_damaged(side, foe, &slot, slot.move_type(), amount, script, events);
            self.shell_bell(side, amount, events);
            self.resolve_faints(side, foe, events);
            return;
        }

        // Psywave's spread collapses the same way: level/2 or level*3/2.
        if slot.entry.id == "psywave" {
            if !hit {
                return;
            }
            if crate::types::effectiveness_against(
                slot.move_type(),
                self.immunity_types(foe, slot.move_type()),
            ) == 0
            {
                events.push(Event::Damage {
                    side: foe as u8 + 1,
                    amount: 0,
                    effectiveness: 0,
                    crit: false,
                });
                return;
            }
            let level = self.sides[side].mon().level as u32;
            let i = match script {
                Some(s) => {
                    if s.secondary {
                        0
                    } else {
                        10
                    }
                }
                None => self.rng.below(11),
            };
            let amount = ((level * (10 * i as u32 + 50)) / 100).max(1) as u16;
            self.flat_hit(side, foe, &slot, amount, Some(true), true, true, script, events);
            return;
        }

        // Counter and Mirror Coat bounce back double the last hit this mon
        // took this turn — physical for Counter, special for Mirror Coat —
        // through the type chart (a Ghost shrugs Counter off) and into a
        // substitute if one stands. Nothing taken means they fail.
        if matches!(slot.entry.id, "counter" | "mirrorcoat") {
            if !hit {
                return;
            }
            let taken = if slot.entry.id == "counter" {
                self.taken_physical[side]
            } else {
                self.taken_special[side]
            };
            if taken == 0 {
                // Nothing recorded against the target: with nothing to
                // bounce back the sim fails this in the move's own `onTry`,
                // which runs before the hit step ever writes the
                // attacked-by book. A Mirror Move aimed back finds nothing.
                events.push(Event::Failed {
                    side: side as u8 + 1,
                });
                return;
            }
            let eff = crate::types::effectiveness_against(
                slot.move_type(),
                self.immunity_types(foe, slot.move_type()),
            );
            if eff == 0 {
                events.push(Event::Damage {
                    side: foe as u8 + 1,
                    amount: 0,
                    effectiveness: 0,
                    crit: false,
                });
                return;
            }
            let amount = taken.saturating_mul(2);
            self.flat_hit(side, foe, &slot, amount, None, false, true, script, events);
            return;
        }

        // Future Sight and Doom Desire: aim a delayed hit two turns out.
        // The DAMAGE is computed now, at launch — typeless (no STAB, no
        // chart), never a crit, the launch turn's roll, today's stats and
        // screens — and stored; only accuracy waits for the landing.
        if matches!(slot.entry.id, "futuresight" | "doomdesire") {
            if self.sides[foe].incoming.is_some() {
                events.push(Event::Failed {
                    side: side as u8 + 1,
                });
                return;
            }
            let (mut attacker, mut defender) = self.attack_pair(side);
            let power = if slot.entry.id == "doomdesire" {
                120
            } else {
                // Future Sight is special: route the special stats through
                // the physical slots, since a typeless move reads those.
                attacker.atk = attacker.sp_atk;
                attacker.atk_stage = attacker.sp_atk_stage;
                attacker.burned = false;
                defender.def = defender.sp_def;
                defender.def_stage = defender.sp_def_stage;
                defender.reflect = defender.light_screen;
                80
            };
            // The damage is computed here, at launch, but it is still a
            // move of its own type as far as the abilities and items are
            // concerned: Doom Desire is Steel and therefore physical, so a
            // Choice Band swings it half again as hard even though the hit
            // itself lands typeless two turns later.
            let real_type = slot.move_type();
            // The sim hands getDamage a moveData whose `type` is '???' and
            // whose CATEGORY is hardcoded — Special for Future Sight, Physical
            // for Doom Desire. So the category still follows the real type,
            // but every handler gated on the move's TYPE sees nothing it
            // recognises and declines: the type-boosting items, the pinch
            // abilities, Thick Fat. A Twisted Spoon does not sharpen a Future
            // Sight even though Future Sight is Psychic.
            let physical = ability::physical_category(real_type);
            let calc_type = Type::None;
            let user_b = self.sides[side].mon().bearer();
            let foe_b = self.sides[foe].mon().bearer();
            let user_i = self.sides[side].mon().holder();
            let foe_i = self.sides[foe].mon().holder();
            attacker.stat_mod = ability::attack_chain(&user_b, physical);
        attacker.stat_pre = ability::hustle_chain(&user_b, physical);
            attacker
                .stat_mod
                .extend(item::attack_chain(&user_i, calc_type, physical));
            attacker.ignores_burn = ability::ignores_burn_drop(&user_b);
            defender.stat_mod = ability::defence_chain(&foe_b, physical);
            defender.stat_mod.extend(item::defence_chain(&foe_i, physical));
            let mut bp = ability::Chain::new();
            if ability::pinch_boost(&user_b, calc_type) {
                bp.mul(ability::X1_5);
            }
            if ability::thick_fat_cut(&foe_b, calc_type) {
                bp.mul(ability::X0_5);
            }
            let power = bp.apply(power as u32).max(1).min(u16::MAX as u32) as u16;
            let random = match script {
                Some(s) => s.random,
                None => 85 + self.rng.below(16) as u8,
            };
            let m = MoveUse {
                move_type: Type::None,
                power,
                halve_def: false,
                late_mult: 1,
                special: false,
                weather: 0,
                phase1: ability::Chain::new(),
            };
            let dealt = damage(
                &attacker,
                &defender,
                &m,
                Roll {
                    crit: false,
                    random,
                },
            ) as u16;
            self.sides[foe].incoming = Some((3, dealt, slot.entry.id));
            events.push(Event::Charging {
                side: side as u8 + 1,
            });
            return;
        }

        // Mimic and Sketch write the foe's last move into the slot — after
        // the dodge and protect gates above, where the sim's copy happens.
        if matches!(slot.entry.id, "mimic" | "sketch") {
            // These aim at the foe and so go in its attacked-by book, the
            // same as any other move that got this far — which matters
            // because both sit in Mirror Move's noMirror list, and a Sketch
            // landing after a Crabhammer is what makes the reply fail.
            if hit && self.sides[foe].mon().sub_hp == 0 {
                self.note_hit_by(side, foe, slot.entry.id);
            }
            // The foe's last move BY ID — a Transform that rewrote its
            // slots doesn't change what it last used.
            let foe_last = self.sides[foe]
                .mon()
                .last_used_id
                .filter(|&i| i != "struggle")
                .and_then(crate::data::move_by_id);
            match (slot.entry.id, foe_last) {
                (_, None) => {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                    return;
                }
                // The user must not be transformed, must not already know
                // the move, and Sketch refuses the nosketch set (itself and
                // Struggle) just as Mimic refuses its own failmimic set.
                (_, Some(e))
                    if e.id == slot.entry.id
                        || self.sides[side].mon().transform_backup.is_some()
                        || self.sides[side].mon().moves.iter().any(|m| m.entry.id == e.id)
                        || (slot.entry.id == "sketch"
                            && matches!(e.id, "sketch" | "struggle")) =>
                {
                    events.push(Event::Failed {
                        side: side as u8 + 1,
                    });
                    return;
                }
                ("mimic", Some(e)) => {
                    // A five-PP overlay; the original slot returns when the
                    // mon leaves the field or faints. A substitute blocks
                    // the copy outright (the sim's, flags notwithstanding),
                    // a TRANSFORMED user cannot Mimic at all, and the
                    // failmimic set (Mimic, Metronome, Sketch, Struggle)
                    // refuses to be copied.
                    if self.sides[foe].mon().sub_hp > 0
                        || self.sides[side].mon().transform_backup.is_some()
                        || matches!(e.id, "mimic" | "metronome" | "sketch" | "struggle")
                    {
                        events.push(Event::Failed {
                            side: side as u8 + 1,
                        });
                        return;
                    }
                    // The copy goes into the slot holding MIMIC, found by
                    // name — `source.moves.indexOf('mimic')` — and not into
                    // the slot that invoked the move. When a Sleep Talk
                    // reaches for Mimic those are two different slots, and
                    // the sim overwrites Mimic's. With no such slot the
                    // index comes back negative and the move fails.
                    let Some(home) = self.sides[side]
                        .mon()
                        .moves
                        .iter()
                        .position(|m| m.entry.id == "mimic")
                    else {
                        events.push(Event::Failed {
                            side: side as u8 + 1,
                        });
                        return;
                    };
                    let orig = self.sides[side].mon().moves[home];
                    let mon = self.sides[side].mon_mut();
                    mon.mimic_backup = Some((home as u8, orig));
                    mon.moves[home] = MoveSlot {
                        entry: e,
                        pp: 5,
                        typed_as: None,
                    };
                    return;
                }
                ("sketch", Some(e)) => {
                    // A Substitute stops a Sketch in this era even though
                    // Sketch bypasses one for the purpose of landing. The
                    // clause is in the gen 4 layer, which gen 3 inherits, and
                    // gen 5 dropped it — so the modern wording is no guide.
                    // The PP is already spent, which leaves the slot holding
                    // a Sketch with nothing left in it.
                    if self.sides[foe].mon().sub_hp > 0 {
                        events.push(Event::Failed {
                            side: side as u8 + 1,
                        });
                        return;
                    }
                    // Same as Mimic: `source.moves.indexOf('sketch')` picks
                    // the slot, so a Sleep-Talked Sketch overwrites SKETCH
                    // and leaves Sleep Talk alone.
                    let Some(home) = self.sides[side]
                        .mon()
                        .moves
                        .iter()
                        .position(|m| m.entry.id == "sketch")
                    else {
                        events.push(Event::Failed {
                            side: side as u8 + 1,
                        });
                        return;
                    };
                    self.sides[side].mon_mut().moves[home] = MoveSlot {
                        entry: e,
                        pp: e.pp,
                        typed_as: None,
                    };
                    return;
                }
                _ => unreachable!(),
            }
        }

        // Snore only works out of a snore-filled sleep — checked HERE, after
        // the call substitution, because Assist and Metronome can hand it to
        // a wide-awake mon and the gate has to see the move that will
        // actually go off.
        if slot.entry.id == "snore" && !asleep_now {
            events.push(Event::Failed {
                side: side as u8 + 1,
            });
            return;
        }

        // A zero-power move is its status action, nothing more. A
        // foe-aimed one that gets this far still goes in the target's
        // attacked-by book (the sim records at the hit loop, whether or
        // not the effect then succeeds) — that is what Mirror Move reads.
        if slot.entry.power == 0 {
            // …except one the target is outright IMMUNE to (Leech Seed on
            // Grass, a chart-zero Thunder Wave or Glare): the sim filters
            // those at the type-immunity step, before the book is written.
            let immune = match slot.entry.id {
                "leechseed" => {
                    let (t1, t2) = self.sides[foe].mon().types();
                    t1 == Type::Grass || t2 == Type::Grass
                }
                "thunderwave" | "glare" => {
                    crate::types::effectiveness_against(
                        slot.move_type(),
                        self.immunity_types(foe, slot.move_type()),
                    ) == 0
                }
                // Attract's GENDER check is an `onTryImmunity`, which turns
                // the move away before the hit loop — so a mismatched pair
                // never enters the attacked-by book and a Mirror Move aimed
                // afterwards finds nothing. Only the gender half belongs
                // here: Oblivious refuses it from inside `moveHit`, where the
                // book has already been written.
                "attract" => {
                    let (them, us) =
                        (self.sides[foe].mon().gender, self.sides[side].mon().gender);
                    !((them == "M" && us == "F") || (them == "F" && us == "M"))
                }
                _ => false,
            };
            // ...and a handful of self-aimed moves ride the NoopFail action
            // without being in the self-target list: they never touch the foe.
            // Psych Up is aimed at the FOE — it reads that mon's stages —
            // even though it is grouped with the self-targeted actions here.
            // It has to write the attacked-by book, because it is on Mirror
            // Move's refusal list and a Psych Up landing on you is what makes
            // your next Mirror Move fail.
            let self_aimed = (self_targeted
                && !matches!(slot.entry.status_action, Some(StatusAction::PsychUp)))
                || matches!(
                    slot.entry.id,
                    "batonpass" | "assist" | "sleeptalk" | "recycle"
                );
            // Magic Coat is a TryHit handler at priority 2, so it speaks
            // before the try-hit abilities and before the immunity is
            // announced — but after the accuracy roll, which is why a move
            // that missed is never thrown back. The volley is one-way: the
            // sim marks the returned move `hasBounced`, so a second Magic
            // Coat standing opposite cannot send it round again.
            if !self_aimed
                && hit
                && slot.entry.reflectable
                && !self.bounced
                && !self.sides[foe].mon().fainted()
                && self.sides[foe].mon().magic_coat
            {
                self.sides[foe].mon_mut().magic_coat = false;
                self.pending_call = Some((foe, slot.entry));
                return;
            }
            // The try-hit abilities answer a status move too: Growl is a
            // sound move whatever its power, and Will-O-Wisp is a Fire one.
            if !self_aimed && hit {
                match ability::absorbs(
                    &self.sides[foe].mon().bearer(),
                    slot.entry.id,
                    slot.move_type(),
                    slot.entry.sound,
                ) {
                    ability::Absorb::None => {}
                    ability::Absorb::FlashFire => {
                        self.sides[foe].mon_mut().flash_fire = true;
                        return;
                    }
                    ability::Absorb::Drain => {
                        let mon = self.sides[foe].mon_mut();
                        let amount = (mon.max_hp / 4).max(1).min(mon.max_hp - mon.hp);
                        if amount > 0 {
                            mon.hp += amount;
                            events.push(Event::Healed {
                                side: foe as u8 + 1,
                                amount,
                            });
                        }
                        return;
                    }
                    ability::Absorb::Immune => {
                        events.push(Event::Damage {
                            side: foe as u8 + 1,
                            amount: 0,
                            effectiveness: 0,
                            crit: false,
                        });
                        return;
                    }
                }
            }
            if !self_aimed && hit && !immune && self.sides[foe].mon().sub_hp == 0 {
                self.note_hit_by(side, foe, slot.entry.id);
            }
            self.status_move(
                side,
                foe,
                &slot,
                hit,
                script.is_some(),
                script.map(|s| s.stall),
                events,
            );
            // Defense Curl primes Rollout. The volatile is applied by
            // `moveHit` from the move's `volatileStatus`, which is a separate
            // step from the boost — so a mon already at +6 Defence still
            // curls, and the doubling still lands. This lives on the status
            // path because Defense Curl is a zero-power move and never
            // reaches the damage tail.
            if slot.entry.id == "defensecurl" && hit {
                self.sides[side].mon_mut().curled = true;
            }
            return;
        }
        // Type immunity preempts the accuracy step entirely: the sim logs
        // |-immune| without ever rolling to hit, so a scripted miss never
        // happens against an immune target — and the kicks never crash.
        {
            let move_type = if slot.entry.id == "weatherball" {
                match self.effective_weather() {
                    Some(Weather::Sun) => Type::Fire,
                    Some(Weather::Rain) => Type::Water,
                    Some(Weather::Sandstorm) => Type::Rock,
                    Some(Weather::Hail) => Type::Ice,
                    None => Type::Normal,
                }
            } else {
                slot.move_type()
            };
            let dtypes = self.immunity_types(foe, move_type);
            let foe_b = self.sides[foe].mon().bearer();
            // Levitate is part of the immunity step itself in the sim, not a
            // try-hit handler: it makes the mon ungrounded, and Ground has
            // nothing to stand on.
            let chart_immune = crate::types::effectiveness_against(move_type, dtypes) == 0
                || ability::immune_to_type(&foe_b, move_type);
            // Then the try-hit abilities, which gen 3 runs AFTER the chart
            // rather than before it. Wonder Guard is asked here because this
            // is the only place that knows what the chart said.
            let effective = crate::types::effectiveness_against(move_type, dtypes) > 100;
            // A move that MISSED is never absorbed: the reference logs the
            // miss and stops, so a Water Absorb mon gets nothing out of an
            // Octazooka that went wide. The chart's own immunity is not
            // conditional that way — it is decided before anyone aims.
            let absorb = if chart_immune {
                ability::Absorb::None
            } else if foe_b.ability == "wonderguard" && !effective && move_type != Type::None {
                ability::Absorb::Immune
            } else if !hit {
                ability::Absorb::None
            } else {
                ability::absorbs(&foe_b, slot.entry.id, move_type, slot.entry.sound)
            };
            match absorb {
                ability::Absorb::Drain => {
                    let mon = self.sides[foe].mon_mut();
                    let amount = (mon.max_hp / 4).max(1).min(mon.max_hp - mon.hp);
                    if amount > 0 {
                        mon.hp += amount;
                        events.push(Event::Healed {
                            side: foe as u8 + 1,
                            amount,
                        });
                    } else {
                        events.push(Event::Damage {
                            side: foe as u8 + 1,
                            amount: 0,
                            effectiveness: 0,
                            crit: false,
                        });
                    }
                }
                ability::Absorb::FlashFire => {
                    self.sides[foe].mon_mut().flash_fire = true;
                }
                ability::Absorb::Immune => {
                    events.push(Event::Damage {
                        side: foe as u8 + 1,
                        amount: 0,
                        effectiveness: 0,
                        crit: false,
                    });
                }
                ability::Absorb::None => {}
            }
            if chart_immune || absorb != ability::Absorb::None {
                if chart_immune {
                    events.push(Event::Damage {
                        side: foe as u8 + 1,
                        amount: 0,
                        effectiveness: 0,
                        crit: false,
                    });
                }
                // An immune target never locks a rampage in — and breaks
                // a running one the way a miss does.
                if ramping {
                    break_rampage(self, side, script.is_some(), events);
                } else {
                    self.sides[side].mon_mut().rampage = None;
                }
                self.sides[side].mon_mut().rolling = None;
                self.sides[side].mon_mut().fury_n = 0;
                if boom {
                    self.resolve_faints(side, foe, events);
                }
                return;
            }
        }
        if !hit {
            // A first-use miss ends a rampage quietly, but a miss once the
            // lock is running (the [from] lockedmove turns) still ends in
            // fatigue confusion. Fury Cutter's ramp and a Rollout reset.
            self.sides[side].mon_mut().last_missed = true;
            if ramping {
                break_rampage(self, side, script.is_some(), events);
            } else {
                // A FIRST-use miss leaves no lock at all: the din's volatile
                // is a self-effect applied in `moveHit`, which a move that
                // missed never reaches. Only a miss once the lock is already
                // running keeps it, and that is the `ramping` branch above.
                self.sides[side].mon_mut().rampage = None;
            }
            self.sides[side].mon_mut().fury_n = 0;
            self.sides[side].mon_mut().rolling = None;
            // The kicks crash for half the damage they would have dealt.
            if matches!(slot.entry.id, "highjumpkick" | "jumpkick") {
                self.kick_crash(side, foe, &slot, script, events);
                return;
            }
            if boom {
                self.resolve_faints(side, foe, events);
            }
            return;
        }

        // Beat Up rallies every healthy party member — the sim builds
        // `move.allies` from everyone unfainted and unstatused — and fails
        // only when that leaves nobody at all. That failure lives in the
        // damage calculation, which sits PAST the accuracy roll, so a Beat
        // Up with nobody behind it can miss like anything else; and when it
        // does not miss, the target still counts as attacked. Gen 3 ends its
        // tryMoveHit with `target.gotAttacked(move, damage, pokemon)`,
        // reached by every move that got this far whatever it then does, so
        // a fizzle is Mirror Move-able exactly like a hit.
        if slot.entry.id == "beatup" && self.beatup_allies(side).is_empty() {
            self.note_hit_by(side, foe, slot.entry.id);
            events.push(Event::Failed {
                side: side as u8 + 1,
            });
            return;
        }

        // Uproar's din wakes every active sleeper — the gen 3 sim cures in
        // moveHit's TryHit, which only runs once the move got PAST immunity
        // and the accuracy roll: an immune target or a miss wakes nobody.
        if slot.entry.id == "uproar" {
            for w in 0..2 {
                if self.sides[w].mon().status == Some(Status::Sleep) {
                    let mon = self.sides[w].mon_mut();
                    mon.status = None;
                    mon.sleep_n = 0;
                    mon.nightmared = false;
                }
            }
        }

        let (crit, random) = match script {
            Some(s) => (s.crit, s.random),
            None => (
                self.rng.below(crit_denominator(
                    slot.entry.high_crit as u8
                        + if self.sides[side].mon().focused { 2 } else { 0 }
                        + item::crit_stages(&self.sides[side].mon().holder()),
                )) == 0,
                85 + self.rng.below(16) as u8,
            ),
        };
        // Battle Armor and Shell Armor refuse the critical hit outright,
        // however it was rolled.
        let crit = crit && !ability::blocks_crit(&self.sides[foe].mon().bearer());
        // How many times this move strikes. The 2-5 spread is the games'
        // weighted table (2 and 3 hits three-eighths each, 4 and 5 an eighth
        // each); a script pins the count for the tests.
        // Beat Up strikes once per rallied ally.
        let beatup_allies = if slot.entry.id == "beatup" {
            self.beatup_allies(side)
        } else {
            Vec::new()
        };
        let hits = if slot.entry.id == "beatup" {
            beatup_allies.len() as u16
        } else if slot.entry.id == "triplekick" {
            // Each kick re-rolls accuracy in the sim. Under a script the
            // follow-up rolls read the secondary knob — false stops after
            // the first kick — UNLESS the roll is not a roll at all: an
            // accuracy that has saturated at a hundred is certain, and a
            // Compound Eyes user lands all three whatever the knob says.
            // Lock-On is certain for the same reason and for EVERY kick: its
            // `onSourceAccuracy` answers `true`, and the loop only rolls when
            // the accuracy it is handed is still a number.
            let certain = sure || acc == 0 || acc >= 100;
            match script {
                Some(s) if !certain => {
                    if s.secondary {
                        3
                    } else {
                        1
                    }
                }
                _ => 3,
            }
        } else {
            match slot.entry.multihit {
                None => 1,
                Some((lo, hi)) if lo == hi => lo,
                Some(_) => match script {
                    // An unset (zero) hits knob means the table minimum, 2 —
                    // the same reading the reference harness uses.
                    Some(s) => {
                        if s.hits > 0 {
                            s.hits as u16
                        } else {
                            2
                        }
                    }
                    None => [2u16, 2, 2, 3, 3, 3, 4, 5][self.rng.below(8) as usize],
                },
            }
        };

        let mut total = 0u16;
        // What the move actually took off the target, which is not the same
        // as what it dealt. A hit soaked by a Substitute returns the sim's
        // HIT_SUBSTITUTE, which is zero, so it adds nothing to
        // `move.totalDamage` — and it is `move.totalDamage` that Shell Bell
        // reads. Recoil is paid inline inside the substitute hook, so recoil
        // does see the soaked hits and keeps reading `total`.
        let mut past_sub = 0u16;
        for hit_i in 0..hits {
            let (mut attacker, mut defender) = self.attack_pair(side);
            // Beat Up strikes with each ally's BASE Attack against the
            // target's BASE Defence — no stages, no burn — as a typeless
            // SPECIAL hit (Light Screen counts, and a zero base stays zero).
            if slot.entry.id == "beatup" {
                // Each strike swings the NEXT ally's base Attack; the sim
                // shifts them off `move.allies` one per hit.
                attacker.sp_atk = beatup_allies[hit_i as usize % beatup_allies.len()];
                attacker.sp_atk_stage = 0;
                defender.sp_def = self.sides[foe].mon().species.base.def as u16;
                defender.sp_def_stage = 0;
            }
            // Weather Ball wears the sky: retyped and doubled under weather.
            let move_type = if slot.entry.id == "weatherball" {
                match self.effective_weather() {
                    Some(Weather::Sun) => Type::Fire,
                    Some(Weather::Rain) => Type::Water,
                    Some(Weather::Sandstorm) => Type::Rock,
                    Some(Weather::Hail) => Type::Ice,
                    None => Type::Normal,
                }
            } else if slot.entry.id == "beatup" {
                Type::None // typeless: no STAB, no effectiveness
            } else {
                slot.move_type()
            };
            // Foresight lifts the Ghost immunity to Normal and Fighting.
            if self.sides[foe].mon().identified
                && matches!(move_type, Type::Normal | Type::Fighting)
            {
                let strip = |t: Type| if t == Type::Ghost { Type::None } else { t };
                defender.types = (strip(defender.types.0), strip(defender.types.1));
            }
            let weather_mod = match (self.effective_weather(), move_type) {
                (Some(Weather::Rain), Type::Water) | (Some(Weather::Sun), Type::Fire) => 1,
                (Some(Weather::Rain), Type::Fire) | (Some(Weather::Sun), Type::Water) => -1,
                _ => 0,
            };
            // Mud/Water Sport halves the matching type at BASE POWER (the
            // sim's onBasePower chain), not at the damage stage — the floor
            // lands one point differently. In this era the hum is a VOLATILE
            // on each mon rather than a field effect, and every holder
            // contributes its own halving: with both actives sporting,
            // Electric comes through at a quarter.
            let sporters = (0..2)
                .filter(|&w| self.sides[w].mon().sport == Some(move_type))
                .count();
            // The stomping moves land doubled on a minimized target.
            let stomp_mult: u16 = if self.sides[foe].mon().minimized
                && matches!(
                    slot.entry.id,
                    "stomp" | "extrasensory" | "needlearm" | "astonish"
                ) {
                2
            } else {
                1
            };
            // Solar Beam sputters outside the sun it was made for.
            let solar_cut = slot.entry.id == "solarbeam"
                && matches!(
                    self.effective_weather(),
                    Some(Weather::Rain | Weather::Sandstorm | Weather::Hail)
                );
            // Conditional powers the era defines by id.
            let base_power = match slot.entry.id {
                "triplekick" => 10 * (hit_i + 1),
                "present" => 120,
                "beatup" => 10,
                "pursuit" if self.pursuing => slot.entry.power * 2,
                "lowkick" => {
                    // Weight tiers, in hectograms.
                    let w = self.sides[foe].mon().species.weight_hg;
                    match w {
                        0..=99 => 20,
                        100..=249 => 40,
                        250..=499 => 60,
                        500..=999 => 80,
                        1000..=1999 => 100,
                        _ => 120,
                    }
                }
                "return" => 102, // the sim's default full happiness
                "frustration" => 1,
                "eruption" | "waterspout" => {
                    let mon = self.sides[side].mon();
                    ((150 * mon.hp as u32) / mon.max_hp as u32).max(1) as u16
                }
                "facade"
                    if matches!(
                        self.sides[side].mon().status,
                        Some(Status::Burn | Status::Poison | Status::Toxic | Status::Paralysis)
                    ) =>
                {
                    slot.entry.power * 2
                }
                "smellingsalts" if self.sides[foe].mon().status == Some(Status::Paralysis) => {
                    slot.entry.power * 2
                }
                "revenge" if self.taken_physical[side] > 0 || self.taken_special[side] > 0 => {
                    slot.entry.power * 2
                }
                "weatherball" if self.effective_weather().is_some() => 100,
                "hiddenpower" => 70,
                "magnitude" => {
                    // The spread collapses under a script: the secondary knob
                    // picks Magnitude 4 (10) or 10 (150); play rolls the table.
                    let i = match script {
                        Some(s) => {
                            if s.secondary {
                                0
                            } else {
                                99
                            }
                        }
                        None => self.rng.below(100) as u8,
                    };
                    match i {
                        0..=4 => 10,
                        5..=14 => 30,
                        15..=34 => 50,
                        35..=64 => 70,
                        65..=84 => 90,
                        85..=94 => 110,
                        _ => 150,
                    }
                }
                "rollout" | "iceball" => {
                    let n = self.sides[side].mon().rolling.unwrap_or(0)
                        + self.sides[side].mon().curled as u8;
                    30u16 << n.min(5)
                }
                "furycutter" => {
                    let n = self.sides[side].mon().fury_n;
                    (10u16 << n).min(160)
                }
                "spitup" => 100 * self.sides[side].mon().stockpile_n as u16,
                "flail" | "reversal" => {
                    let mon = self.sides[side].mon();
                    let p = 48 * mon.hp as u32 / mon.max_hp as u32;
                    match p {
                        0..=1 => 200,
                        2..=4 => 150,
                        5..=9 => 100,
                        10..=16 => 80,
                        17..=32 => 40,
                        _ => 20,
                    }
                }
                _ => slot.entry.power,
            };
            // Charge doubles the next Electric move, then is spent.
            let charge_mult: u16 =
                if move_type == Type::Electric && self.sides[side].mon().charged_elec {
                    self.sides[side].mon_mut().charged_elec = false;
                    2
                } else {
                    1
                };
            // Every base-power modifier goes through ONE chain and is
            // applied once, in the sim's handler order: Charge's doubling,
            // then a Sport's hum, then a pinch ability, then Thick Fat, then
            // Solar Beam's weather sulk. Halving and then boosting in turn
            // is not the same arithmetic as doing both at once, and the
            // difference shows up as a point of damage.
            let user_b = self.sides[side].mon().bearer();
            let foe_b = self.sides[foe].mon().bearer();
            let mut bp = ability::Chain::new();
            if charge_mult == 2 {
                bp.mul(ability::X2);
            }
            for _ in 0..sporters {
                bp.mul(ability::X0_5);
            }
            if ability::pinch_boost(&user_b, move_type) {
                bp.mul(ability::X1_5);
            }
            if ability::thick_fat_cut(&foe_b, move_type) {
                bp.mul(ability::X0_5);
            }
            if solar_cut {
                bp.mul(ability::X0_5);
            }
            let power = bp
                .apply((base_power * pierce_power_mult) as u32)
                .max(1)
                .min(u16::MAX as u32) as u16;
            // The stats, once their stages are in. Gen 3 reads the category
            // off the move's type, so that is what decides whether an
            // Attack ability speaks at all.
            let physical = slot.entry.id != "beatup" && ability::physical_category(move_type);
            let user_i = self.sides[side].mon().holder();
            let foe_i = self.sides[foe].mon().holder();
            attacker.stat_mod = ability::attack_chain(&user_b, physical);
            attacker.stat_pre = ability::hustle_chain(&user_b, physical);
            attacker.stat_mod
                .extend(item::attack_chain(&user_i, move_type, physical));
            attacker.ignores_burn = ability::ignores_burn_drop(&user_b);
            defender.stat_mod = ability::defence_chain(&foe_b, physical);
            defender.stat_mod.extend(item::defence_chain(&foe_i, physical));
            let mut phase1 = ability::Chain::new();
            if self.sides[side].mon().flash_fire && move_type == Type::Fire {
                phase1.mul(ability::X1_5);
            }
            let m = MoveUse {
                move_type,
                power,
                halve_def: slot.entry.selfdestruct,
                weather: weather_mod,
                // Pierce and stomp double at the sim's ModifyDamage stage,
                // just before the roll — not on base power.
                late_mult: pierce_mult * stomp_mult,
                special: slot.entry.id == "beatup",
                phase1,
            };
            let dealt = damage(&attacker, &defender, &m, Roll { crit, random });
            if dealt == 0 {
                break; // immune: later strikes land no better
            }

            let eff = crate::types::effectiveness_against(
                m.move_type,
                self.immunity_types(foe, m.move_type),
            );
            let survives = self.survives_at_one(foe);
            let target = self.sides[foe].mon_mut();
            let hit_sub = target.sub_hp > 0;
            let amount = if hit_sub {
                let amount = (dealt as u16).min(target.sub_hp);
                target.sub_hp -= amount;
                amount
            } else {
                let cap = if slot.entry.id == "falseswipe" || survives {
                    target.hp.saturating_sub(1)
                } else {
                    target.hp
                };
                let amount = (dealt as u16).min(cap);
                target.hp -= amount;
                amount
            };
            total += amount;
            if !hit_sub {
                past_sub += amount;
            }
            if hit_sub {
                events.push(Event::SubDamage {
                    side: foe as u8 + 1,
                    amount,
                });
                if self.sides[foe].mon().sub_hp == 0 {
                    events.push(Event::SubBroke {
                        side: foe as u8 + 1,
                    });
                }
            } else {
                match m.category() {
                    crate::types::Category::Physical => self.taken_physical[foe] = amount,
                    _ => self.taken_special[foe] = amount,
                }
                events.push(Event::Damage {
                    side: foe as u8 + 1,
                    amount,
                    effectiveness: eff,
                    crit,
                });
                // What just hit this mon (Mirror Move's playback source).
                self.note_hit_by(side, foe, slot.entry.id);
                // A biding target banks what it just took.
                if let Some((stored, left)) = self.sides[foe].mon().bide {
                    self.sides[foe].mon_mut().bide = Some((stored.saturating_add(amount), left));
                }
                // A raging target's Attack climbs with every hit it takes.
                if self.sides[foe].mon().raging && !self.sides[foe].mon().fainted() {
                    self.sides[foe].mon_mut().apply_boost(Boost::Atk, 1);
                    events.push(Event::Boosted {
                        side: foe as u8 + 1,
                        boost: Boost::Atk,
                        delta: 1,
                    });
                }
                // Rage's own rage state begins only once it actually lands.
                if slot.entry.id == "rage" {
                    self.sides[side].mon_mut().raging = true;
                }
                // The faint is NOT announced here. `faintMessages` runs at
                // the end of the hit, after the secondaries and after the
                // target has answered being hit — and announcing it early
                // hands a transformed mon its own ability back before Rough
                // Skin is asked, so a Sharpedo wearing a copied Battle Armor
                // grazed with a Rough Skin it no longer had.
            }
            // Drain heals off the damage actually dealt: floor, but at
            // least 1 — EXCEPT off a substitute, where the sim's sub hook
            // heals with a CEILING instead.
            // `spreadDamage` gates the whole drain block on the damage
            // actually DEALT — `if (targetDamage && effect.effectType ===
            // 'Move')` — and the sim is careful to let a genuine zero through
            // to that test rather than flooring it up to one. So a blow that
            // Focus Band or Endure clamped to nothing at one HP heals the
            // drainer nothing: the min-of-one below lives INSIDE the gate and
            // never runs. Liquid Ooze is under the same gate, so it stays
            // silent too.
            if let Some((num, den)) = slot.entry.drain.filter(|_| amount > 0) {
                let heal = if hit_sub {
                    ((amount * num + den - 1) / den).max(1)
                } else {
                    (amount * num / den).max(1)
                };
                // Liquid Ooze turns the sip into a swig of poison: the same
                // number, taken off the drainer instead of given to it.
                if ability::ooze_reverses_drain(&self.sides[foe].mon().bearer(), slot.entry.id) {
                    let user = self.sides[side].mon_mut();
                    let hurt = heal.min(user.hp);
                    user.hp -= hurt;
                    events.push(Event::Recoil {
                        side: side as u8 + 1,
                        amount: hurt,
                    });
                    self.resolve_faints(side, foe, events);
                    continue;
                }
                let user = self.sides[side].mon_mut();
                let heal = heal.min(user.max_hp - user.hp);
                if heal > 0 {
                    user.hp += heal;
                    events.push(Event::Drained {
                        side: side as u8 + 1,
                        amount: heal,
                    });
                }
            }
            if !hit_sub {
                // The move's own secondary lands FIRST, and only then does
                // the target answer having been hit. Colour Change turning a
                // mon Fire the instant a Blaze Kick lands would otherwise
                // make it immune to the burn that same Blaze Kick is about
                // to inflict.
                self.hit_effects(side, foe, &slot, script, events);
                self.on_damaged(side, foe, &slot, move_type, amount, script, events);
                self.resolve_faints(side, foe, events);
            } else {
                // A substitute soaks the hit but not the whole secondary. The
                // sim nulls the TARGET out rather than cancelling the move —
                // `targets[i] = null` — and its secondary step skips only on
                // a strict `false`, so the half of a secondary aimed at the
                // ATTACKER still lands. Ancient Power off a broken sub still
                // raises all five of the user's stats.
                self.self_boost_only(side, &slot, script, events);
            }
            // Gen 3 runs its own multi-hit loop and reads BOTH mons at the
            // top of it: `for (i = 0; i < hits && target.hp && pokemon.hp;
            // i++)`. An attacker killed by Rough Skin or Liquid Ooze between
            // strikes stops there, mid-flurry.
            if self.sides[foe].mon().fainted() || self.sides[side].mon().fainted() {
                break;
            }
        }
        if slot.entry.id == "spitup" {
            self.sides[side].mon_mut().stockpile_n = 0;
        }
        if total == 0 {
            if boom {
                self.resolve_faints(side, foe, events);
            }
            return;
        }

        // This runs BEFORE the thaw below, matching the reference sim: a
        // Fire move's burn chance is blocked by the freeze it is about to
        // cure, because the target still carries frz when secondaries apply.

        // Fury Cutter ramps on every use that actually REACHES the mon. In
        // gen 3 the bump lives in the move's own `onHit`, and a hit a
        // Substitute ate never gets there — the sim nulls the target and
        // skips onHit entirely. One `addVolatile` sets both the multiplier
        // and the two-turn clock, so a sub-only swing extends neither.
        if slot.entry.id == "furycutter" && past_sub > 0 {
            let mon = self.sides[side].mon_mut();
            mon.fury_n = (mon.fury_n + 1).min(4);
            mon.fury_fresh = true;
        }
        // Rollout counts its landed uses and lets go after five — and its
        // base-power callback carries the same sleep guard, so a Sleep Talk
        // swing neither starts a count nor advances one.
        if matches!(slot.entry.id, "rollout" | "iceball")
            && self.sides[side].mon().rolling.is_some()
            && !asleep_now
        {
            let mon = self.sides[side].mon_mut();
            let n = mon.rolling.unwrap_or(0) + 1;
            mon.rolling = if n >= 5 { None } else { Some(n) };
            mon.rolling_fresh = true;
        }
        // Rapid Spin flings off the user's own bind and Leech Seed.
        if slot.entry.id == "rapidspin" {
            let user = self.sides[side].mon_mut();
            user.seeded = false;
            user.trapped_n = 0;
            // It sweeps the floor as well as the user: the sim's onHit runs
            // removeSideCondition('spikes') before it touches the volatiles,
            // so every later switch-in on this side walks in clean.
            self.sides[side].spikes = 0;
        }

        // Superpower and kin always pay their stat bill on a landed hit.
        if let Some(list) = slot.entry.self_drop {
            if !self.sides[side].mon().fainted() {
                for &(boost, delta) in list {
                    self.sides[side].mon_mut().apply_boost(boost, delta);
                    events.push(Event::Boosted {
                        side: side as u8 + 1,
                        boost,
                        delta,
                    });
                }
            }
        }

        // Recoil comes off the damage actually dealt: floored (this era's
        // rule; the fuzzer rejected round-to-nearest), but at least 1 — and
        // it can knock the user out.
        if let Some((num, den)) = slot.entry.recoil
            .filter(|_| !ability::ignores_recoil(&self.sides[side].mon().bearer(), slot.entry.id))
        {
            let hurt = (total * num / den).max(1);
            let user = self.sides[side].mon_mut();
            let hurt = hurt.min(user.hp);
            user.hp -= hurt;
            events.push(Event::Recoil {
                side: side as u8 + 1,
                amount: hurt,
            });
        }

        // Thief and Covet pocket what the target was holding, but only if
        // the thief's own hands are empty; Knock Off simply strikes it away.
        // Sticky Hold keeps hold of it either way, and in this era Knock Off
        // gets no extra power for the trouble.
        // `if (moveData.onAfterHit && pokemon.hp)` — the sim asks after the
        // THIEF, not after the victim, and the faint is not processed until
        // the move is over. So a Covet or a Thief that knocks its target out
        // walks off with the item anyway, and so does a Knock Off.
        if total > 0 && !self.sides[side].mon().fainted() {
            let theirs = self.sides[foe].mon().item;
            let held = self.sides[foe].mon().ability == "stickyhold";
            match slot.entry.id {
                // `takeItem` refuses outright when EITHER side of the
                // exchange has had an item knocked off it in this era.
                "thief" | "covet"
                    if self.sides[side].mon().item.is_empty()
                        && !theirs.is_empty()
                        && !held
                        && !self.sides[side].mon().item_knocked_off
                        && !self.sides[foe].mon().item_knocked_off =>
                {
                    self.sides[foe].mon_mut().item = "";
                    self.sides[side].mon_mut().item = theirs;
                    // A stolen berry is a berry in hand: the sim's setItem
                    // runs an update, so a thief that pockets a Rawst eats
                    // it on the spot and walks away unburned.
                    self.ability_update(side);
                }
                "knockoff" if !theirs.is_empty() && !held => {
                    let victim = self.sides[foe].mon_mut();
                    victim.item = "";
                    victim.item_knocked_off = true;
                }
                _ => {}
            }
        }
        self.shell_bell(side, past_sub, events);


        // A landed Hyper Beam costs the next action.
        if slot.entry.recharge {
            let mon = self.sides[side].mon_mut();
            mon.must_recharge = true;
            mon.recharge_fresh = true;
        }

        self.resolve_faints(side, foe, events);
    }

    /// The confusion self-hit: 40 base power, typeless, physical, against
    /// the mon's own Defense — stages and burn apply, nothing else does.
    /// What the target's ability does about having just been hit. Color
    /// Change takes on the move's type; Rough Skin grazes whatever touched
    /// it; the status ones each get a third of a chance, Effect Spore a
    /// tenth. A scripted run pins those rolls off — they are not one of the
    /// scenario's knobs, and the reference harness leaves a denominator of
    /// three or ten alone.
    /// Everything a mon does in answer to having HP taken off it by a move,
    /// wherever in the pipeline that happened. The sim reaches these through
    /// the Damage event, which every damaging path runs, so an OHKO, a
    /// Seismic Toss, a Counter, an Endeavor and a delayed Future Sight all
    /// stoke a raging target and all bank into a Bide — exactly as an
    /// ordinary hit does.
    /// Land a flat, chart-neutral hit: the shape Bide's unleash, the
    /// fixed-damage moves, Psywave and Counter all share. The Substitute eats
    /// it first (and the mon behind takes nothing at all); Endure and Focus
    /// Band cap it at one HP; what was actually dealt goes into the turn's
    /// taken-damage register and the target's attacked-by book, and the
    /// shared hit tail runs. Four copies of this block each missed a
    /// different fix this month, which is why it is one function now.
    ///
    /// `kings_rock` and `contact_tail` are per-arm because the copies
    /// genuinely differed: Bide runs neither (even though Bide is in the
    /// King's Rock move list — the sim was never consulted on that pairing,
    /// so the unification must not quietly change it), and Counter skips the
    /// Rock (its move is not in the list, so the call would only be a no-op
    /// bought with a flag).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn flat_hit(
        &mut self,
        side: usize,
        foe: usize,
        slot: &MoveSlot,
        amount: u16,
        special_register: Option<bool>,
        kings_rock: bool,
        contact_tail: bool,
        script: Option<SeatScript>,
        events: &mut Vec<Event>,
    ) {
        let survives = self.survives_at_one(foe);
        let target = self.sides[foe].mon_mut();
        if target.sub_hp > 0 {
            let amount = amount.min(target.sub_hp);
            target.sub_hp -= amount;
            events.push(Event::SubDamage {
                side: foe as u8 + 1,
                amount,
            });
            if self.sides[foe].mon().sub_hp == 0 {
                events.push(Event::SubBroke {
                    side: foe as u8 + 1,
                });
            }
            return;
        }
        let cap = if survives {
            target.hp.saturating_sub(1)
        } else {
            target.hp
        };
        let amount = amount.min(cap);
        target.hp -= amount;
        let special = special_register.unwrap_or_else(|| {
            !matches!(
                crate::types::category_of(slot.move_type()),
                crate::types::Category::Physical
            )
        });
        if special {
            self.taken_special[foe] = amount;
        } else {
            self.taken_physical[foe] = amount;
        }
        events.push(Event::Damage {
            side: foe as u8 + 1,
            amount,
            effectiveness: 100,
            crit: false,
        });
        self.note_hit_by(side, foe, slot.entry.id);
        if kings_rock {
            self.kings_rock(side, foe, slot, script);
        }
        self.took_a_hit(foe, amount, events);
        if contact_tail {
            self.on_damaged(side, foe, slot, slot.move_type(), amount, script, events);
            self.shell_bell(side, amount, events);
        }
        self.resolve_faints(side, foe, events);
    }

    /// Write the target's attacked-by book: what hit it and from which
    /// field slot. Mirror Move reads it; the end-of-turn purge clears it
    /// when the attacker leaves the field.
    pub(super) fn note_hit_by(&mut self, side: usize, foe: usize, id: &'static str) {
        let from = self.sides[side].active;
        let mon = self.sides[foe].mon_mut();
        mon.last_hit_by = Some(id);
        mon.last_hit_by_slot = Some(from);
    }

    pub(super) fn took_a_hit(&mut self, foe: usize, amount: u16, events: &mut Vec<Event>) {
        if let Some((stored, left)) = self.sides[foe].mon().bide {
            self.sides[foe].mon_mut().bide = Some((stored.saturating_add(amount), left));
        }
        if self.sides[foe].mon().raging && !self.sides[foe].mon().fainted() {
            self.sides[foe].mon_mut().apply_boost(Boost::Atk, 1);
            events.push(Event::Boosted {
                side: foe as u8 + 1,
                boost: Boost::Atk,
                delta: 1,
            });
        }
    }

    /// What a mon's ability does about having just been hit. Every one of
    /// these is an `onDamagingHit` in the sim and every one opens with the
    /// same guard — `if (damage && ...)`. A hit that took nothing off, a
    /// False Swipe into a mon already at one HP, wakes none of them: no
    /// Rough Skin graze, no Static, no Color Change.
    pub(super) fn on_damaged(
        &mut self,
        side: usize,
        foe: usize,
        slot: &MoveSlot,
        move_type: Type,
        dealt: u16,
        script: Option<SeatScript>,
        events: &mut Vec<Event>,
    ) {
        if dealt == 0 {
            return;
        }
        let hit_b = self.sides[foe].mon().bearer();
        if ability::color_change(&hit_b) && !self.sides[foe].mon().fainted() && move_type != Type::None
        {
            // Only a type the target does NOT already have takes hold: a
            // dual-type that already counts as this type keeps both halves.
            let (t1, t2) = self.sides[foe].mon().types();
            if t1 != move_type && t2 != move_type {
                self.sides[foe].mon_mut().type_override = Some((move_type, Type::None));
            }
        }
        if !slot.entry.contact {
            return;
        }
        if ability::rough_skin(&hit_b) {
            let attacker = self.sides[side].mon_mut();
            let graze = ((attacker.max_hp / 16).max(1)).min(attacker.hp);
            attacker.hp -= graze;
            events.push(Event::Recoil {
                side: side as u8 + 1,
                amount: graze,
            });
        }
        let (touch, odds) = ability::on_touch(&hit_b);
        let proc = match script {
            Some(_) => false,
            None => touch != ability::OnTouch::None && self.rng.below(odds) == 0,
        };
        if proc {
            match touch {
                ability::OnTouch::Status(st) => self.inflict(side, st, script.is_some(), events),
                // The sim samples sleep, paralysis and poison in that order.
                ability::OnTouch::Spore => {
                    let st = match self.rng.below(3) {
                        0 => Status::Sleep,
                        1 => Status::Paralysis,
                        _ => Status::Poison,
                    };
                    self.inflict(side, st, script.is_some(), events);
                }
                // Attract is not modelled yet, so Cute Charm has nothing to
                // do with its third of a chance.
                ability::OnTouch::Attract | ability::OnTouch::None => {}
            }
        }
    }

    /// Shell Bell hands back an eighth of everything the move dealt, rounded
    /// down, at the sim's after-secondary-self stage — which is after the
    /// recoil, so a full-health attacker still gets part of its kick back.
    pub(super) fn shell_bell(&mut self, side: usize, dealt: u16, events: &mut Vec<Event>) {
        // Nothing heals a corpse: the sim's heal() refuses a target with no
        // HP left, and an Explosion that pays its user back to life keeps a
        // decided battle running.
        if dealt == 0
            || self.sides[side].mon().fainted()
            || !item::shell_bell(&self.sides[side].mon().holder())
        {
            return;
        }
        // The sim's heal() rounds a fraction of one UP: anything above zero
        // gives back at least a point, so a four-damage hit still pays one.
        let mon = self.sides[side].mon_mut();
        let amount = (dealt / 8).max(1).min(mon.max_hp - mon.hp);
        if amount > 0 {
            mon.hp += amount;
            events.push(Event::Healed {
                side: side as u8 + 1,
                amount,
            });
        }
    }

    pub(super) fn confusion_self_hit(&mut self, side: usize, random: u8) -> u16 {
        let mon = self.sides[side].mon();
        // The sim runs this through its ordinary damage call, which means
        // the Attack and Defence abilities both speak — a confused Huge
        // Power mon hits itself twice as hard.
        let bearer = mon.bearer();
        let holder = mon.holder();
        // A typeless forty-power physical hit, which means the held items
        // speak as well as the abilities: a confused Choice Band swings half
        // again as hard at itself.
        let mut atk_chain = ability::attack_chain(&bearer, true);
        atk_chain.extend(item::attack_chain(&holder, Type::None, true));
        let mut def_chain = ability::defence_chain(&bearer, true);
        def_chain.extend(item::defence_chain(&holder, true));
        let atk = atk_chain.apply(ability::hustle_chain(&bearer, true).apply(
            crate::stats::apply_stage(mon.atk, mon.stages[Stat::Atk as usize]) as u32,
        ));
        let def = def_chain
            .apply(crate::stats::apply_stage(mon.def, mon.stages[Stat::Def as usize]) as u32)
            .max(1);
        let mut dmg = ((2 * mon.level as u32 / 5 + 2) * 40 * atk / def) / 50;
        if mon.burned() && !ability::ignores_burn_drop(&bearer) {
            dmg /= 2;
        }
        if dmg == 0 {
            dmg = 1;
        }
        dmg += 2;
        dmg = (dmg * random.clamp(85, 100) as u32) / 100;
        let dmg = dmg.max(1) as u16;
        // The self-hit goes through `damage()` like any move, so Endure and
        // Focus Band are both asked — and asked about the mon hitting itself,
        // which is the one whose seat carries the coin. Its own Substitute is
        // no shield here: the damage is dealt to the mon directly.
        let survives = self.survives_lethal(side, false);
        let mon = self.sides[side].mon();
        let cap = if survives {
            mon.hp.saturating_sub(1)
        } else {
            mon.hp
        };
        let dmg = dmg.min(cap);
        self.sides[side].mon_mut().hp -= dmg;
        dmg
    }

    /// A King's Rock hangs a second secondary off the move — ten percent,
    /// flinch — by pushing it onto `move.secondaries` at onModifyMove. It is
    /// rolled on its own after the move's own secondary, Shield Dust refuses
    /// it the same way, and Serene Grace doubles it, since by the time it is
    /// rolled it is just another entry in the list.
    ///
    /// The list it answers to includes the FIXED-damage moves — Night Shade,
    /// Seismic Toss, Sonic Boom, Dragon Rage — as well as Endeavor and
    /// Psywave, none of which go anywhere near the ordinary damage loop. So
    /// this is called from their arms too, and not from the paths where a
    /// Substitute soaked the hit: the sim logs no flinch there.
    pub(super) fn kings_rock(
        &mut self,
        side: usize,
        foe: usize,
        slot: &MoveSlot,
        script: Option<SeatScript>,
    ) {
        if !item::kings_rock_flinches(&self.sides[side].mon().holder(), slot.entry.id) {
            return;
        }
        let dusted = ability::blocks_secondary(&self.sides[foe].mon().bearer());
        let chance = if ability::doubles_secondary(&self.sides[side].mon().bearer()) {
            20
        } else {
            10
        };
        let proc = !dusted
            && match script {
                Some(s) => s.secondary,
                None => self.rng.below(100) < chance,
            };
        if proc
            && !self.sides[foe].mon().fainted()
            && !self.sides[foe].mon().focusing
            && !ability::blocks_flinch(&self.sides[foe].mon().bearer())
        {
            self.sides[foe].mon_mut().flinched = true;
        }
    }

    pub(super) fn hit_effects(
        &mut self,
        side: usize,
        foe: usize,
        slot: &MoveSlot,
        script: Option<SeatScript>,
        events: &mut Vec<Event>,
    ) {
        // A 100% secondary (Zap Cannon's paralysis) is a certainty, not a
        // roll, so the script has no say in it — matching the reference sim,
        // whose sub-certain roll can't come up short of 100.
        // Shield Dust refuses every secondary a move turns on ITS TARGET.
        // The self-aimed drops (Overheat's own Special Attack) are a
        // different field entirely and are not touched.
        // Shield Dust filters the secondaries aimed at its bearer and keeps
        // the ones the move turns on the ATTACKER — Steel Wing still raises
        // its own Defence through it.
        let dusted = ability::blocks_secondary(&self.sides[foe].mon().bearer())
            && !matches!(
                slot.entry.secondary.map(|sec| sec.effect),
                Some(SecondaryEffect::SelfBoosts(_))
            );
        // Serene Grace doubles the printed chance before it is rolled.
        let doubled = ability::doubles_secondary(&self.sides[side].mon().bearer());
        let chance =
            |sec: crate::data::Secondary| (sec.chance as u32 * if doubled { 2 } else { 1 }).min(255);
        let certain = slot.entry.secondary.is_some_and(|sec| chance(sec) >= 100);
        let proc = !dusted
            && (certain
                || match script {
                    Some(s) => s.secondary,
                    None => slot
                        .entry
                        .secondary
                        .map(|sec| self.rng.below(100) < chance(sec))
                        .unwrap_or(false),
                });
        if proc {
            match slot.entry.secondary.map(|sec| sec.effect) {
                Some(SecondaryEffect::Status(status)) => {
                    self.inflict(foe, status, script.is_some(), events);
                }
                Some(SecondaryEffect::Boosts(list)) => {
                    // Mist blocks foe-caused stat drops for the whole team.
                    let misted = self.sides[foe].mist_n > 0;
                    if !self.sides[foe].mon().fainted() {
                        for &(boost, delta) in list {
                            if misted && delta < 0 {
                                continue;
                            }
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
                Some(SecondaryEffect::Flinch) => {
                    // A mon tightening its focus cannot be flinched at all —
                    // the volatile is refused, not merely out-prioritized.
                    if !self.sides[foe].mon().fainted()
                        && !self.sides[foe].mon().focusing
                        && !ability::blocks_flinch(&self.sides[foe].mon().bearer())
                    {
                        self.sides[foe].mon_mut().flinched = true;
                    }
                }
                Some(SecondaryEffect::Confuse) => {
                    self.confuse(foe, script.is_some(), events);
                }
                Some(SecondaryEffect::TriAttack) => {
                    let status = match script {
                        Some(_) => Status::Burn, // the sim's pinned sample
                        None => *[Status::Burn, Status::Paralysis, Status::Freeze]
                            .iter()
                            .nth(self.rng.below(3) as usize)
                            .unwrap(),
                    };
                    self.inflict(foe, status, script.is_some(), events);
                }
                Some(SecondaryEffect::SelfBoosts(list)) => {
                    if !self.sides[side].mon().fainted() {
                        for &(boost, delta) in list {
                            self.sides[side].mon_mut().apply_boost(boost, delta);
                            events.push(Event::Boosted {
                                side: side as u8 + 1,
                                boost,
                                delta,
                            });
                        }
                    }
                }
                None => {}
            }
        }

        self.kings_rock(side, foe, slot, script);

        // A Fire-type hit thaws a frozen target — game rule, not RNG. A
        // knocked-out target keeps its freeze; there is nothing left to thaw.
        if self.sides[foe].mon().status == Some(Status::Freeze)
            && slot.move_type() == Type::Fire
            && !self.sides[foe].mon().fainted()
        {
            self.sides[foe].mon_mut().status = None;
        }

        // Smelling Salts spends the paralysis it fed on — unless the hit
        // was also the KO; a corpse keeps its status.
        if slot.entry.id == "smellingsalts"
            && self.sides[foe].mon().status == Some(Status::Paralysis)
            && !self.sides[foe].mon().fainted()
        {
            self.sides[foe].mon_mut().status = None;
        }

        // Wrap and kin bind the target: the games roll 3..6 end-of-turn
        // ticks (a script pins the floor), and a fresh bind cannot stack
        // on a running one.
        if slot.entry.trap
            && !self.sides[foe].mon().fainted()
            && self.sides[foe].mon().trapped_n == 0
        {
            let n = match script {
                Some(_) => 3,
                None => 3 + self.rng.below(4) as u8,
            };
            self.sides[foe].mon_mut().trapped_n = n;
            events.push(Event::Trapped {
                side: foe as u8 + 1,
            });
        }
    }

    /// The kicks crash for half the damage they would have dealt. The sim
    /// runs `getDamage` for real here, so the whole calculation answers: the
    /// crit the seat was due, both mons' abilities, and both mons' held
    /// items — a Black Belt makes the crash hurt more, a Metal Powder makes
    /// it hurt less.
    pub(super) fn kick_crash(
        &mut self,
        side: usize,
        foe: usize,
        slot: &MoveSlot,
        script: Option<SeatScript>,
        events: &mut Vec<Event>,
    ) {
        // The whole crash is gated on the target not being immune to
        // Fighting: `if (target.runImmunity('Fighting'))` wraps every line of
        // it, so a kick that whiffs past a Ghost costs its user NOTHING —
        // not the clamped minimum of one, which is what a zeroed damage
        // calculation would otherwise floor to. It is a plain type check, so
        // Levitate and Wonder Guard have no say, and a Foresight that
        // stripped the Ghost half puts the crash back.
        {
            let mut dtypes = self.sides[foe].mon().types();
            if self.sides[foe].mon().identified {
                let strip = |t: Type| if t == Type::Ghost { Type::None } else { t };
                dtypes = (strip(dtypes.0), strip(dtypes.1));
            }
            if crate::types::effectiveness_against(Type::Fighting, dtypes) == 0 {
                return;
            }
        }
        let (random, crit) = match script {
            Some(s) => (s.random, s.crit),
            None => (85 + self.rng.below(16) as u8, self.rng.below(16) == 0),
        };
        // The crash is a real damage calculation, so the target's armour
        // answers it: a Shell Armor mon refuses the critical hit here just
        // as it would on a landed one, and the kick hurts half as much.
        let crit = crit && !ability::blocks_crit(&self.sides[foe].mon().bearer());
        let (mut attacker, mut defender) = self.attack_pair(side);
        let user_b = self.sides[side].mon().bearer();
        let foe_b = self.sides[foe].mon().bearer();
        let physical = ability::physical_category(slot.move_type());
        let user_i = self.sides[side].mon().holder();
        let foe_i = self.sides[foe].mon().holder();
        attacker.stat_mod = ability::attack_chain(&user_b, physical);
            attacker.stat_pre = ability::hustle_chain(&user_b, physical);
        attacker
            .stat_mod
            .extend(item::attack_chain(&user_i, slot.move_type(), physical));
        attacker.ignores_burn = ability::ignores_burn_drop(&user_b);
        defender.stat_mod = ability::defence_chain(&foe_b, physical);
        defender.stat_mod.extend(item::defence_chain(&foe_i, physical));
        let m = MoveUse {
            move_type: slot.move_type(),
            power: slot.entry.power,
            halve_def: false,
            late_mult: 1,
            special: false,
            weather: 0,
            phase1: ability::Chain::new(),
        };
        let would = damage(&attacker, &defender, &m, Roll { crit, random });
        // The sim clamps the crash into [1, target's max HP / 2].
        let cap = (self.sides[foe].mon().max_hp / 2).max(1);
        let crash = ((would / 2) as u16).max(1).min(cap);
        let user = self.sides[side].mon_mut();
        let crash = crash.min(user.hp);
        user.hp -= crash;
        events.push(Event::Recoil {
            side: side as u8 + 1,
            amount: crash,
        });
        self.resolve_faints(side, foe, events);
    }

    pub(super) fn attack_pair(&self, side: usize) -> (Attacker, Defender) {
        let a = self.sides[side].mon();
        let d = self.sides[1 - side].mon();
        (
            Attacker {
                level: a.level,
                atk: a.atk,
                sp_atk: a.spa,
                atk_stage: a.stage(Stat::Atk),
                sp_atk_stage: a.stage(Stat::SpAtk),
                types: a.types(),
                burned: a.burned(),
                stat_mod: ability::Chain::new(),
                stat_pre: ability::Chain::new(),
                ignores_burn: false,
            },
            Defender {
                def: d.def,
                sp_def: d.spd,
                def_stage: d.stage(Stat::Def),
                sp_def_stage: d.stage(Stat::SpDef),
                types: d.types(),
                reflect: self.sides[1 - side].reflect_n > 0,
                light_screen: self.sides[1 - side].light_screen_n > 0,
                stat_mod: ability::Chain::new(),
            },
        )
    }

    /// Base Attack of everyone Beat Up can call: unfainted and unstatused,
    /// the active first (the sim's `side.pokemon` keeps it at the front) and
    /// the rest in party order. Order only decides which strike lands first,
    /// since every ally gets one.
    pub(super) fn beatup_allies(&self, side: usize) -> Vec<u16> {
        let s = &self.sides[side];
        let ok = |m: &Mon| !m.fainted() && m.status.is_none();
        let mut out = Vec::new();
        if ok(s.mon()) {
            out.push(s.mon().species.base.atk as u16);
        }
        for (i, m) in s.party.iter().enumerate() {
            if i != s.active && ok(m) {
                out.push(m.species.base.atk as u16);
            }
        }
        out
    }

    /// The per-strike aftermath of a landed hit: the secondary (script or
    /// RNG-decided), then the Fire thaw. Runs once per strike of a
    /// multi-hit move, which is also how the reference sim rolls it.
    /// The one part of a secondary that survives a Substitute: the half a
    /// move turns on its OWN user. The roll is the same roll — Serene Grace
    /// doubles it, a certainty is no roll at all, a script decides it — but
    /// Shield Dust never had anything to say about a self-aimed boost, and
    /// the target is not there to be affected.
    pub(super) fn self_boost_only(
        &mut self,
        side: usize,
        slot: &MoveSlot,
        script: Option<SeatScript>,
        events: &mut Vec<Event>,
    ) {
        let Some(SecondaryEffect::SelfBoosts(list)) = slot.entry.secondary.map(|sec| sec.effect)
        else {
            return;
        };
        let doubled = ability::doubles_secondary(&self.sides[side].mon().bearer());
        let chance =
            |sec: crate::data::Secondary| (sec.chance as u32 * if doubled { 2 } else { 1 }).min(255);
        let certain = slot.entry.secondary.is_some_and(|sec| chance(sec) >= 100);
        let proc = certain
            || match script {
                Some(s) => s.secondary,
                None => slot
                    .entry
                    .secondary
                    .map(|sec| self.rng.below(100) < chance(sec))
                    .unwrap_or(false),
            };
        if proc && !self.sides[side].mon().fainted() {
            for &(boost, delta) in list {
                self.sides[side].mon_mut().apply_boost(boost, delta);
                events.push(Event::Boosted {
                    side: side as u8 + 1,
                    boost,
                    delta,
                });
            }
        }
    }

    /// Whether a lethal blow leaves the mon standing at one HP. Endure always
    /// does; Focus Band answers a 1-in-10 the sim rolls on EVERY damaging hit,
    /// lethal or not, and only for damage whose source is a move.
    ///
    /// A confusion self-hit COUNTS as one. The sim deals it through the
    /// ordinary `damage()` call with a fabricated effect that spells
    /// `effectType: 'Move'` out, so both handlers gate open on it exactly as
    /// they do for a real hit. Recoil and the residuals genuinely do go
    /// through unclamped, because those carry a Recoil or a Status effect.
    pub(super) fn survives_at_one(&mut self, side: usize) -> bool {
        self.survives_lethal(side, true)
    }

    /// As above, but `through_sub` says whether a Substitute standing in front
    /// of this mon can swallow the blow first. It can for an incoming move,
    /// which never reaches the mon's own HP and so never calls `damage()` — no
    /// handler is asked and the Band's coin is not spent. It cannot for a
    /// confusion self-hit, which calls `damage()` on the mon directly and
    /// walks straight past its own Substitute.
    pub(super) fn survives_lethal(&mut self, side: usize, through_sub: bool) -> bool {
        let mon = self.sides[side].mon();
        if through_sub && mon.sub_hp > 0 {
            return false;
        }
        if mon.enduring {
            return true;
        }
        if mon.item != "focusband" {
            return false;
        }
        // The coin belongs to the mon being HIT, not the one attacking, so
        // it is read off the target's own seat rather than off the script the
        // acting side was handed.
        match self.turn_seats[side] {
            Some(s) => s.band,
            None => self.rng.below(10) == 0,
        }
    }
}
