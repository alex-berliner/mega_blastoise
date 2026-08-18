//! The end-of-turn residual buckets, one per `comparePriority` key.

extern crate alloc;

use alloc::vec::Vec;

use crate::ability;
use crate::item;
use crate::data::{
    Boost, Status, Weather,
};

use super::*;

impl Battle {
    /// One bucket of the residual phase for one mon.
    ///
    /// The sim gathers every residual handler on the field into ONE list and
    /// sorts it with `comparePriority`: order, then priority, then SPEED,
    /// then subOrder. Speed outranking subOrder is what makes it look like a
    /// per-mon walk — a faster mon's whole set runs before a slower mon's.
    /// But when the two are exactly LEVEL, as they are after a Transform,
    /// speed stops separating them and subOrder takes over: the burn on one
    /// mon runs before the bind on the other. Splitting the phase into
    /// buckets lets the caller run them either way round.
    pub(super) fn residual_bucket(
        &mut self,
        bucket: usize,
        side: usize,
        scripted: bool,
        mut events: &mut Vec<Event>,
    ) {
        match bucket {
            0 => {
            // Ingrain sips a sixteenth of max HP back (the games' order 7,
            // ahead of Leech Seed).
            if self.sides[side].mon().ingrained && !self.sides[side].mon().fainted() {
                let want = (self.sides[side].mon().max_hp / 16).max(1);
                self.heal(side, want, events);
            }
            }
            1 => {
            // Rain Dish sips while it rains; Shed Skin gets a third of a
            // chance at shrugging a status off; Speed Boost climbs a
            // stage for every turn spent on the field. All three sit at
            // the sim's residual order 10, subOrder 3.
        if !self.sides[side].mon().fainted() {
                let bearer = self.sides[side].mon().bearer();
                if ability::rain_dish(&bearer) && self.effective_weather() == Some(Weather::Rain)
                {
                    let want = (self.sides[side].mon().max_hp / 16).max(1);
                    self.heal(side, want, events);
                }
                // A scripted run pins this roll off: it is not one of
                // the scenario's knobs, and the reference harness leaves
                // a 33-in-100 alone the same way.
                if ability::sheds_skin(&bearer)
                    && self.sides[side].mon().status.is_some()
                    && !scripted
                    && self.rng.below(100) < 33
                {
                    let mon = self.sides[side].mon_mut();
                    mon.status = None;
                    mon.sleep_n = 0;
                    mon.sleep_skipped = 0;
                    mon.toxic_n = 0;
                }
                if ability::speed_boosts(&bearer) && self.sides[side].mon().active_turns > 0 {
                    self.boost(side, Boost::Spe, 1, events);
                }
        }
            }
            2 => {
                // Then the items, at subOrder 4. Gen 3 berries wait for
                // this phase rather than firing the moment the holder is
                // hurt, which is why a mon can be knocked out with a
                // Sitrus still in hand.
        if !self.sides[side].mon().fainted() {
                let holder = self.sides[side].mon().holder();
                if item::leftovers(&holder) {
                    let want = (self.sides[side].mon().max_hp / 16).max(1);
                    self.heal(side, want, events);
                }
                self.ripen(side, &mut events);
        }
            }
            3 => {
            // Leech Seed bleeds an eighth of max HP to the opposing active.
            // A seed with nobody to feed does nothing at all: the sim
            // bails on "Nothing to leech into" before it takes a point,
            // so a seeder that just fainted spares its victim entirely.
            if self.sides[side].mon().seeded
                && !self.sides[side].mon().fainted()
                && !self.sides[1 - side].mon().fainted()
            {
                let drain = (self.sides[side].mon().max_hp / 8).max(1);
                let mon = self.sides[side].mon_mut();
                let drain = drain.min(mon.hp);
                mon.hp -= drain;
                events.push(Event::SeedDrain {
                    side: side as u8 + 1,
                    amount: drain,
                });
                // Liquid Ooze turns the seed sour: the sip the seeder
                // was owed is taken off it instead. The sim hangs that
                // on the HEAL, not on the move, so Leech Seed's drip
                // poisons exactly as a Giga Drain does.
                let ooze = ability::ooze_reverses_drain(
                    &self.sides[side].mon().bearer(),
                    "leechseed",
                );
                let foe = self.sides[1 - side].mon_mut();
                if !foe.fainted() {
                    if ooze {
                        let hurt = drain.min(foe.hp);
                        foe.hp -= hurt;
                        events.push(Event::Residual {
                            side: (1 - side) as u8 + 1,
                            amount: hurt,
                            status: Status::Poison,
                        });
                    } else {
                        let heal = drain.min(foe.max_hp - foe.hp);
                        if heal > 0 {
                            foe.hp += heal;
                            events.push(Event::Healed {
                                side: (1 - side) as u8 + 1,
                                amount: heal,
                            });
                        }
                    }
                }
                self.announce_faint(side, &mut events);
                self.announce_faint(1 - side, &mut events);
            }
            }
            4 => {
            // Burn and poison tick 1/8 max HP, Toxic a growing sixteenth.
            let mon = self.sides[side].mon();
            if mon.fainted() {
                return;
            }
            if let Some(status @ (Status::Burn | Status::Poison | Status::Toxic)) = mon.status {
                let amount = if status == Status::Toxic {
                    let mon = self.sides[side].mon_mut();
                    mon.toxic_n = (mon.toxic_n + 1).min(15);
                    (mon.max_hp / 16).max(1) * mon.toxic_n as u16
                } else {
                    (self.sides[side].mon().max_hp / 8).max(1)
                };
                let mon = self.sides[side].mon_mut();
                let amount = amount.min(mon.hp);
                mon.hp -= amount;
                events.push(Event::Residual {
                    side: side as u8 + 1,
                    amount,
                    status,
                });
                self.announce_faint(side, &mut events);
            }
            }
            5 => {
            // Nightmare rides the sleep: a quarter per turn while it lasts.
            if self.sides[side].mon().nightmared && !self.sides[side].mon().fainted() {
                if self.sides[side].mon().status == Some(Status::Sleep) {
                    let mon = self.sides[side].mon_mut();
                    let amount = (mon.max_hp / 4).max(1).min(mon.hp);
                    mon.hp -= amount;
                    events.push(Event::Residual {
                        side: side as u8 + 1,
                        amount,
                        status: Status::Sleep,
                    });
                    self.announce_faint(side, &mut events);
                } else {
                    self.sides[side].mon_mut().nightmared = false;
                }
            }
            }
            6 => {
            // Ghost-Curse chips a quarter of max HP in this mon's slot.
            if self.sides[side].mon().cursed && !self.sides[side].mon().fainted() {
                let mon = self.sides[side].mon_mut();
                let amount = ((mon.max_hp / 4).max(1)).min(mon.hp);
                mon.hp -= amount;
                events.push(Event::Residual {
                    side: side as u8 + 1,
                    amount,
                    status: Status::Poison,
                });
                self.announce_faint(side, &mut events);
            }
            }
            7 => {
            // The bind (order 10, after the status tick): the clock ticks
            // down, and every surviving tick chips a sixteenth.
            //
            // Unless the trapper is GONE. The volatile lingered after its
            // user left the field, and this is where the sim deletes it:
            // silently, before the chip — `onResidual` checks the source
            // first and bails. No damage, no end-of-trap line.
            if self.sides[side].mon().trap_stale {
                let mon = self.sides[side].mon_mut();
                mon.trapped_n = 0;
                mon.trap_stale = false;
            } else if self.sides[side].mon().trapped_n > 0 && !self.sides[side].mon().fainted() {
                let mon = self.sides[side].mon_mut();
                mon.trapped_n -= 1;
                if mon.trapped_n > 0 {
                    // The chip goes straight to the mon: a residual is
                    // not a hit, so a substitute never soaks it. (The
                    // only way to be behind one and still bound is a
                    // Baton Pass, which carries both.)
                    let amount = ((mon.max_hp / 16).max(1)).min(mon.hp);
                    mon.hp -= amount;
                    events.push(Event::TrapDamage {
                        side: side as u8 + 1,
                        amount,
                    });
                    self.announce_faint(side, &mut events);
                } else {
                    events.push(Event::TrapEnded {
                        side: side as u8 + 1,
                    });
                }
            }
            }
            8 => {
            // The recharge is a two-turn volatile, so it runs out at the
            // end of the turn it was owed on — spent or not. A turn that
            // a faint cancelled still burns it, which is why a mon whose
            // Hyper Beam turn was cut short is free the turn after next
            // rather than a turn later.
            if self.sides[side].mon().must_recharge {
                let mon = self.sides[side].mon_mut();
                if mon.recharge_fresh {
                    mon.recharge_fresh = false;
                } else {
                    mon.must_recharge = false;
                }
            }
            }
            9 => {
            // The din runs its own clock here, at the sim's subOrder 11.
            // That is BEFORE Yawn at 19, which is exactly why a Yawn
            // coming due on the turn an Uproar ends still puts its
            // victim under: by then there is no noise left to stop it.
            // Counting the turns down at the moment the mon shouted
            // instead ended the racket half a turn early, and a Grass
            // Whistle aimed at it landed that it should have slept
            // straight through.
            if let Some((slot_i, left)) = self.sides[side].mon().rampage {
                // The lock's move is the move that was RESOLVED, not the
                // slot that was chosen: an Uproar reached through Assist,
                // Metronome or Mirror Move sits in a slot holding some
                // other move entirely, so `locked_move` is what has to be
                // asked.
                let uproar = self.sides[side].mon().locked_move == Some("uproar");
                if uproar {
                    let mon = self.sides[side].mon_mut();
                    let left = left.saturating_sub(1);
                    if left == 0 {
                        mon.rampage = None;
                        mon.uproar_ending = true;
                    } else {
                        mon.rampage = Some((slot_i, left));
                    }
                }
            }
            }
            10 => {
            // Yawn rides THIS mon's residual slot, right after its bind:
            // a faster mon's yawn resolves before a slower mon's poison,
            // and a battle already decided leaves the drowsy awake.
            if self.sides[side].mon().yawn_n > 0 && !self.sides[side].mon().fainted() {
                self.sides[side].mon_mut().yawn_n -= 1;
                if self.sides[side].mon().yawn_n == 0 {
                    self.yawn_landing = true;
                    self.inflict(side, Status::Sleep, scripted, &mut events);
                    self.yawn_landing = false;
                }
            }
            }
            11 => {
            // The Thrash-family lock ticks last of all, carrying no
            // residual order of its own. When its two-turn clock runs
            // out the mon is confused whatever it did with the turn;
            // falling asleep calms it only if the sleep arrives while
            // the clock still has time on it.
            if let Some((slot_i, owed)) = self.sides[side].mon().rampage {
                let uproar = self.sides[side].mon().locked_move == Some("uproar");
                if !uproar {
                    let n = if scripted {
                        2
                    } else {
                        2 + self.rng.below(4) as u8
                    };
                    let mon = self.sides[side].mon_mut();
                    mon.rampage_dur = mon.rampage_dur.saturating_sub(1);
                    if mon.rampage_dur == 0 {
                        mon.rampage = None;
                        if owed <= 1 && mon.confusion_n == 0 && !mon.fainted() {
                            mon.confusion_n = n;
                            events.push(Event::ConfusionStarted {
                                side: side as u8 + 1,
                            });
                        }
                    } else if mon.status == Some(Status::Sleep) {
                        mon.rampage = None;
                    } else {
                        mon.rampage = Some((slot_i, owed.saturating_sub(1)));
                    }
                }
            }
            }
            12 => {
        // Truant loafs every other turn, and the toggle flips here
        // whether or not the mon acted; the sim puts it at residual
        // order 27, well after everything above.
        if !self.sides[side].mon().fainted() {
            let bearer = self.sides[side].mon().bearer();
                if ability::truant(&bearer) {
                    let mon = self.sides[side].mon_mut();
                    mon.loafing = !mon.loafing;
                }
        }
            }
            _ => {}
        }
    }
}
