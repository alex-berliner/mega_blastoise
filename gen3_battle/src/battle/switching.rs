//! Mons entering and leaving: switch resets, the greeting order, entry
//! hazards, faint replacement, and the per-action ability/item upkeep.

extern crate alloc;

use alloc::vec::Vec;

use crate::ability;
use crate::item;
use crate::data::{
    Boost, Status, Weather,
};
use crate::stats::Stat;
use crate::types::Type;

use super::*;

impl Battle {
    /// Whether `side`'s active mon may switch out at all. The sim marks a
    /// mon `trapped` for two separate reasons and both belong here: an
    /// effect holding it in place (a bind, Mean Look), and a move holding
    /// its own turn — `getLockedMove` covers a rampage, a rolling
    /// Rollout/Ice Ball, a storing Bide, an Uproar, a charge turn and a
    /// Hyper Beam recharge, and every one of those sets `trapped = true`.
    /// Overwrite a mon's ability, remembering what it was born with. Trace,
    /// Transform, Role Play and Skill Swap all come through here.
    pub(super) fn set_ability(&mut self, side: usize, id: &'static str) {
        let mon = self.sides[side].mon_mut();
        if mon.ability_backup.is_none() {
            mon.ability_backup = Some(mon.ability);
        }
        mon.ability = id;
        // Truant handed over mid-battle does NOT start loafing on the spot.
        // Gen 3's own Truant sets `onStart: void 0`, and `setAbility` only
        // raises a Start event from gen 4 on, so nothing at all runs when the
        // ability changes hands: the loaf state is written only by Truant's
        // switch-in and by its residual. A Traced or Skill Swapped Truant
        // therefore acts this turn and first loafs on the next, which is
        // exactly what the residual flip below produces from `false`.
        if id == "truant" {
            self.sides[side].mon_mut().loafing = false;
        }
    }

    /// The sim's `AfterMove`, as far as a Choice Band is concerned: the lock
    /// goes on once the move is done, and reads whatever the mon is holding
    /// by then. The lock also lapses on its own the moment its holder stops
    /// carrying a Choice item or stops knowing the move — which is how a
    /// Knock Off frees its victim, and why a Struggle forced out by the lock
    /// never sticks as the locked move.
    /// Run whatever a Magic Coat or a Snatch put in someone else's hands.
    /// The sim reaches these through `useMove`, which is below the gates and
    /// below the PP charge, so the move simply happens.
    pub(super) fn run_pending_call(&mut self, script: &TurnScript, events: &mut Vec<Event>) {
        let mut guard = 0;
        while let Some((who, entry)) = self.pending_call.take() {
            guard += 1;
            if guard > 4 || self.sides[who].mon().fainted() || self.over() {
                break;
            }
            self.forced_entry = Some(entry);
            self.calling = true;
            self.bounced = true;
            self.use_move(who, 0, script.seats[who], events);
            self.calling = false;
            self.bounced = false;
        }
    }

    pub(super) fn settle_choice_lock(&mut self, side: usize) {
        let committed = self.committed_move[side].take();
        // Charge's volatile is dropped by `onAfterMove` on ANY move but
        // Charge itself, so a charge spent on something that is not Electric
        // is simply spent. Ours only cleared it when an Electric move cashed
        // it in, which left a Raikou holding the doubling indefinitely.
        if committed.is_some_and(|id| id != "charge") {
            self.sides[side].mon_mut().charged_elec = false;
        }
        let mon = self.sides[side].mon();
        let banded = mon.item == "choiceband";
        if let Some(id) = committed {
            if banded && mon.choice_locked.is_none() {
                self.sides[side].mon_mut().choice_locked = Some(id);
            }
        }
        let mon = self.sides[side].mon();
        let lapsed = mon.choice_locked.is_some_and(|id| {
            !banded || !mon.moves.iter().any(|m| m.entry.id == id)
        });
        if lapsed {
            self.sides[side].mon_mut().choice_locked = None;
        }
    }

    /// `if (this.gen < 5) this.eachEvent('Update')`, the line the sim runs at
    /// the end of every action in this era. It is what eats a Chesto off the
    /// sleep Rest just laid down, refills a slot a Spite emptied before its
    /// owner has to move, and shakes off a status an ability refuses — all
    /// within the turn, rather than at the start of the next one.

    /// `lastItem` belongs to the field SLOT in this era, not to the mon.
    /// `switchIn` hands the outgoing mon's over to the incoming one and
    /// blanks the outgoing mon's, so a Recycle restores whatever the last
    /// occupant of that slot used up — even if that was somebody else. The
    /// handover is unconditional: a mon returning to a slot whose last
    /// occupant consumed nothing comes back with nothing.
    pub(super) fn hand_slot_item_over(&mut self, side: usize) -> &'static str {
        let out = self.sides[side].mon_mut();
        let carried = out.last_item;
        out.last_item = "";
        carried
    }

    pub(super) fn end_of_action(&mut self) {
        // `eachEvent('Update')` is the LAST statement of the sim's `runAction`,
        // and `runAction` has already returned by then if the action decided
        // the battle: `faintMessages(); if (this.ended) return true;` sits
        // above it. So a Grudge that empties a move on the winning blow leaves
        // the holder's Leppa Berry uneaten — nothing gets a chance to tidy up
        // after the last mon has gone down.
        if self.over() {
            return;
        }
        for side in 0..2 {
            self.ability_update(side);
        }
    }

    /// Forecast, which is the sim's `onWeatherChange` and nothing else — it
    /// answers a sky that CHANGED and a mon that just arrived, and is silent
    /// the rest of the time. Running it more often than that would have it
    /// stomping every other thing that writes a type: a Conversion 2 that
    /// turned Castform into a Ghost has to stay a Ghost.
    ///
    /// Forecast, which is the sim's `onWeatherChange`: Castform wears the
    /// sky. Every forme carries the same seventy across the board, so the
    /// only thing that actually changes is the TYPE, and this is a type
    /// override and nothing more. Air Lock and Cloud Nine put it back to
    /// Normal without clearing the weather, since `effectiveWeather` is what
    /// it reads; sandstorm is not one of the three it answers to. A
    /// transformed Castform stops answering altogether.
    pub(super) fn forecast(&mut self, side: usize) {
        let mon = self.sides[side].mon();
        if mon.ability != "forecast"
            || !mon.species.id.starts_with("castform")
            || mon.transform_stats.is_some()
        {
            return;
        }
        let worn = match self.effective_weather() {
            Some(Weather::Sun) => Type::Fire,
            Some(Weather::Rain) => Type::Water,
            Some(Weather::Hail) => Type::Ice,
            _ => Type::Normal,
        };
        self.sides[side].mon_mut().type_override = Some((worn, Type::None));
    }

    /// The sim's `onUpdate`, which runs between actions and is where the
    /// refusing abilities do their tidying: they do not merely block a
    /// status arriving, they shed one already there. A mon that walks into
    /// the battle asleep with Insomnia is awake before it has to act.
    pub(super) fn ability_update(&mut self, side: usize) {
        let mon = self.sides[side].mon();
        if mon.fainted() {
            return;
        }
        let bearer = mon.bearer();
        if mon.status.is_some_and(|st| ability::blocks_status(&bearer, st)) {
            let mon = self.sides[side].mon_mut();
            mon.status = None;
            mon.sleep_n = 0;
            mon.sleep_skipped = 0;
            mon.toxic_n = 0;
        }
        if ability::blocks_confusion(&bearer) {
            self.sides[side].mon_mut().confusion_n = 0;
        }
        // The curing berries are `onUpdate` items too: they are eaten the
        // moment the status lands, not at the end of the turn like the
        // healing ones.
        let holder = self.sides[side].mon().holder();
        let cure_status = self
            .sides[side]
            .mon()
            .status
            .is_some_and(|st| item::cures_status(&holder, st));
        let cure_confusion =
            self.sides[side].mon().confusion_n > 0 && item::cures_confusion(&holder);
        if cure_status || cure_confusion {
            let mon = self.sides[side].mon_mut();
            mon.last_item = mon.item;
            mon.item = "";
            if cure_status {
                mon.status = None;
                mon.sleep_n = 0;
                mon.sleep_skipped = 0;
                mon.toxic_n = 0;
            }
            if cure_confusion {
                mon.confusion_n = 0;
            }
        }
        // Oblivious sheds a charm as well as refusing one, on its own
        // `onUpdate` — so a mon that comes by the ability mid-battle stops
        // being infatuated.
        if ability::blocks_attract(&self.sides[side].mon().bearer()) {
            self.sides[side].mon_mut().attracted_by = None;
        }
        // Attract's own `onUpdate`: the volatile goes the moment the mon
        // that charmed it is no longer the one standing opposite.
        if let Some(who) = self.sides[side].mon().attracted_by {
            if self.sides[1 - side].active != who || self.sides[1 - side].mon().fainted() {
                self.sides[side].mon_mut().attracted_by = None;
            }
        }
        // A Mental Herb is spent breaking the charm, and does nothing else in
        // this era.
        if self.sides[side].mon().item == "mentalherb"
            && self.sides[side].mon().attracted_by.is_some()
        {
            let mon = self.sides[side].mon_mut();
            mon.last_item = mon.item;
            mon.item = "";
            mon.attracted_by = None;
        }
        // A Leppa Berry is an `onUpdate` item too. It waits for a slot to
        // reach zero, then puts ten points back into the FIRST slot at zero
        // — or, failing that, the first slot short of full, which is how a
        // Leppa handed over by Trick lands on a mon that never ran dry.
        // (A Mimicked slot's five-point maximum is not modelled: the engine
        // carries no per-slot maximum, so the move's own PP stands in.)
        if self.sides[side].mon().item == "leppaberry" {
            let mon = self.sides[side].mon();
            let empty = mon.moves.iter().any(|m| m.pp == 0);
            let target = mon
                .moves
                .iter()
                .position(|m| m.pp == 0)
                .or_else(|| mon.moves.iter().position(|m| m.pp < m.entry.pp));
            if let (true, Some(i)) = (empty, target) {
                let mon = self.sides[side].mon_mut();
                mon.last_item = mon.item;
                mon.item = "";
                let slot = &mut mon.moves[i];
                slot.pp = (slot.pp + 10).min(slot.entry.pp);
            }
        }
    }

    /// The White Herb hands back every stage its holder has lost, all at
    /// once, and is spent doing it. The sim hangs it off `onStart`,
    /// `onAnySwitchIn`, `onAnyAfterMove` and the residual phase, so it
    /// answers an Intimidate as the intimidator lands and a Growl as soon as
    /// the Growl is over, rather than waiting for the end of the turn like
    /// the berries. It reads negative stages only: a mon at +2 Attack and -1
    /// Speed keeps the +2.
    pub(super) fn white_herb(&mut self, side: usize) {
        if self.sides[side].mon().fainted() || self.sides[side].mon().item != "whiteherb" {
            return;
        }
        let mon = self.sides[side].mon();
        let dropped =
            mon.stages.iter().any(|&st| st < 0) || mon.acc_stage < 0 || mon.eva_stage < 0;
        if !dropped {
            return;
        }
        let mon = self.sides[side].mon_mut();
        mon.last_item = mon.item;
        mon.item = "";
        for st in mon.stages.iter_mut() {
            if *st < 0 {
                *st = 0;
            }
        }
        mon.acc_stage = mon.acc_stage.max(0);
        mon.eva_stage = mon.eva_stage.max(0);
    }

    /// Eat the held berry if the residual phase finds the holder low enough.
    pub(super) fn ripen(&mut self, side: usize, events: &mut Vec<Event>) {
        if self.sides[side].mon().fainted() {
            return;
        }
        let ripe = item::ripens(&self.sides[side].mon().holder());
        if ripe == item::Ripe::None {
            return;
        }
        let eaten = self.sides[side].mon().item;
        self.sides[side].mon_mut().item = "";
        self.sides[side].mon_mut().last_item = eaten;
        match ripe {
            item::Ripe::Heal(flat) => {
                let mon = self.sides[side].mon_mut();
                let amount = flat.min(mon.max_hp - mon.hp);
                if amount > 0 {
                    mon.hp += amount;
                    events.push(Event::Healed {
                        side: side as u8 + 1,
                        amount,
                    });
                }
            }
            item::Ripe::HealEighth => {
                let mon = self.sides[side].mon_mut();
                let amount = ((mon.max_hp / 8).max(1)).min(mon.max_hp - mon.hp);
                if amount > 0 {
                    mon.hp += amount;
                    events.push(Event::Healed {
                        side: side as u8 + 1,
                        amount,
                    });
                }
            }
            item::Ripe::Boost(boost) => {
                self.sides[side].mon_mut().apply_boost(boost, 1);
                events.push(Event::Boosted {
                    side: side as u8 + 1,
                    boost,
                    delta: 1,
                });
            }
            // The sim samples the stats that are not already maxed, and a
            // pinned sample takes the first — which is Attack.
            item::Ripe::StarfBoost => {
                let order = [Boost::Atk, Boost::Def, Boost::SpAtk, Boost::SpDef, Boost::Spe];
                let pick = order.iter().copied().find(|b| {
                    let i = match b {
                        Boost::Atk => Stat::Atk,
                        Boost::Def => Stat::Def,
                        Boost::SpAtk => Stat::SpAtk,
                        Boost::SpDef => Stat::SpDef,
                        _ => Stat::Spe,
                    };
                    self.sides[side].mon().stage(i) < 6
                });
                if let Some(boost) = pick {
                    self.sides[side].mon_mut().apply_boost(boost, 2);
                    events.push(Event::Boosted {
                        side: side as u8 + 1,
                        boost,
                        delta: 2,
                    });
                }
            }
            item::Ripe::FocusEnergy => {
                self.sides[side].mon_mut().focused = true;
            }
            item::Ripe::None => {}
        }
    }

    /// Announce and replace one side's active if it just fainted.
    pub(super) fn announce_faint(&mut self, side: usize, events: &mut Vec<Event>) {
        if self.sides[side].mon().fainted() {
            if let Some((i, orig)) = self.sides[side].mon_mut().mimic_backup.take() {
                self.sides[side].mon_mut().moves[i as usize] = orig;
            }
            if let Some(orig) = self.sides[side].mon_mut().transform_backup.take() {
                self.sides[side].mon_mut().moves = orig;
            }
            // A borrowed ability goes back with everything else the sim
            // clears when a mon goes down.
            if let Some(born_with) = self.sides[side].mon_mut().ability_backup.take() {
                self.sides[side].mon_mut().ability = born_with;
            }
            if let Some((stats, types)) = self.sides[side].mon_mut().transform_stats.take() {
                let mon = self.sides[side].mon_mut();
                mon.atk = stats[0];
                mon.def = stats[1];
                mon.spa = stats[2];
                mon.spd = stats[3];
                mon.spe = stats[4];
                mon.type_override = types;
            }
            // A fainted trapper/gazer releases its victim.
            self.sides[1 - side].mon_mut().trapped_n = 0;
            self.sides[1 - side].mon_mut().mean_looked = false;
            events.push(Event::Fainted {
                side: side as u8 + 1,
            });
        }
    }

    /// Everything the sim's `clearVolatile` drops when a mon leaves the
    /// field, applied to `side`'s current active. Shared by the voluntary
    /// switch and by Roar and Whirlwind dragging one off.
    pub(super) fn switch_out_reset(&mut self, side: usize) {
            // The trapper/gazer leaving the field releases its
            // victim; a sport leaves with its hummer (handled by
            // the outgoing mon's own field reset below).
            self.sides[1 - side].mon_mut().trapped_n = 0;
            self.sides[1 - side].mon_mut().mean_looked = false;
            let out = self.sides[side].mon_mut();
            if ability::cures_on_switch_out(&out.bearer()) && !out.fainted() {
                out.status = None;
                out.sleep_n = 0;
            }
            out.flash_fire = false;
            if let Some(born_with) = out.ability_backup.take() {
                out.ability = born_with;
            }
            out.choice_locked = None;
            out.active_turns = 0;
            out.toxic_n = 0;
            out.confusion_n = 0;
            out.identified = false;
            out.sure_hit = 0;
            out.charged_elec = false;
            out.grudged = false;
            out.tormented = false;
            out.torment_fresh = false;
            out.raging = false;
            out.fury_n = 0;
            out.fury_fresh = false;
            out.last_used = None;
            out.last_used_id = None;
            out.last_move_used_id = None;
            out.last_hit_by = None;
            out.last_hit_by_slot = None;
            out.last_missed = false;
            if let Some((i, orig)) = out.mimic_backup.take() {
                out.moves[i as usize] = orig;
            }
            if let Some(orig) = out.transform_backup.take() {
                out.moves = orig;
            }
            if let Some((stats, types)) = out.transform_stats.take() {
                out.atk = stats[0];
                out.def = stats[1];
                out.spa = stats[2];
                out.spd = stats[3];
                out.spe = stats[4];
                out.type_override = types;
            }
            out.bide = None;
            out.rolling = None;
            out.curled = false;
            out.encore_n = 0;
            out.encore_fresh = false;
            out.disabled_slot = None;
            out.disable_n = 0;
            out.disable_fresh = false;
            out.disable_skip_tick = false;
            out.imprisoning = false;
            out.imprison_fresh = false;
            out.type_override = None;
            out.cursed = false;
            out.ingrained = false;
            out.stall_counter = 0;
            out.protected = false;
            out.magic_coat = false;
            out.snatching = false;
            out.attracted_by = None;
            out.enduring = false;
            out.taunt_n = 0;
            out.nightmared = false;
            out.stockpile_n = 0;
            out.yawn_n = 0;
            out.perish_n = 0;
            out.destiny = false;
            out.mean_looked = false;
            // The sim's clearVolatile also wipes every stat stage and
            // the accuracy/evasion pair, drops any lock the mon was
            // under, and zeroes its action count — which is why Fake
            // Out works again on a mon that left and came back.
            out.stages = Default::default();
            out.acc_stage = 0;
            out.eva_stage = 0;
            out.flinched = false;
            out.focusing = false;
            out.uproar_ending = false;
            out.locked_move = None;
            out.acted = false;
            out.sport = None;
            out.sub_hp = 0;
            out.focused = false;
            out.minimized = false;
            out.seeded = false;
            out.trapped_n = 0;
            out.charging = None;
            out.charge_fresh = false;
            out.charge_fresh = false;
            out.rolling_fresh = false;
            out.rampage = None;
            out.must_recharge = false;
            out.recharge_fresh = false;
    }

    /// Send in replacements for whoever is down. The sim asks for these only
    /// AFTER the residual phase (`fieldEvent("Residual")`, then
    /// `checkFainted`), so a mon that faints mid-turn stays in its slot for
    /// the rest of the turn: the other side's move finds no target, and the
    /// residuals tick against an empty slot rather than against the
    /// replacement. The loop repeats because Spikes can drop the incoming
    /// mon too, and the sim just asks again.
    pub(super) fn replace_fainted(&mut self, events: &mut Vec<Event>) {
        // A decided battle asks for nothing: the sim's checkWin runs inside
        // faintMessages, ahead of checkFainted, so the loser's last mon is
        // simply left lying where it fell — status and all.
        if self.over() {
            return;
        }
        // Both replacements are PLACED before either is greeted. The sim's
        // `switchIn` puts the mon on the field and queues a separate
        // `runSwitch` action, and only those queued actions — sorted by Speed
        // — start the abilities. So when two mons go down together, the
        // Intimidate on one of the replacements cows the other replacement,
        // which is already standing there. Greeting each one as it landed had
        // the first arrival staring at an empty slot.
        let mut guard = 0;
        loop {
            guard += 1;
            if guard > 8 {
                break;
            }
            let mut arrived = [false; 2];
            for side in 0..2 {
                if self.deferred_switch[side].is_some() {
                    continue; // its own switch is still coming
                }
                if !self.sides[side].mon().fainted() {
                    continue;
                }
                let Some(next) = self.sides[side].first_healthy() else {
                    continue;
                };
                self.sides[side].mon_mut().status = None;
                let slot_item = self.hand_slot_item_over(side);
                self.sides[side].reorder_for_switch(next);
                self.sides[side].active = next;
                self.sides[side].mon_mut().last_item = slot_item;
                events.push(Event::Switched {
                    side: side as u8 + 1,
                    party_index: next,
                });
                arrived[side] = true;
            }
            if !arrived[0] && !arrived[1] {
                break;
            }
            let first = self.faster_side(true);
            for side in [first, 1 - first] {
                if arrived[side] {
                    self.switch_in_greet(side, events);
                    self.end_of_action();
                }
            }
        }
    }

    /// What greets a mon as it comes in. Gen 3 hands back the sleep it spent
    /// on Snore or Sleep Talk right before retreating, so a sleeper can
    /// attack, switch out and come back no closer to waking. Then Spikes
    /// bite a grounded arrival: an eighth, a sixth, a quarter for one, two,
    /// three layers. Flying types float over.
    pub(super) fn switch_in_greet(&mut self, side: usize, events: &mut Vec<Event>) {
        // `runSwitch` fires the SwitchIn event BEFORE it starts the arriving
        // mon's ability, so a White Herb on the field answers whatever was
        // already there and NOT the Intimidate that is about to land: the
        // drop stands until some move ends or the residual phase comes round.
        self.white_herb(side);
        self.white_herb(1 - side);
        self.greet(side, true, events);
    }

    pub(super) fn greet(&mut self, side: usize, tidy: bool, events: &mut Vec<Event>) {
        // Truant counts the turn it arrives on, unless the battle has not
        // started: the sim keys that off its own turn counter.
        let turn = self.turn;
        self.sides[side].mon_mut().loafing = turn > 0;
        {
            let mon = self.sides[side].mon_mut();
            if mon.status == Some(Status::Sleep) {
                mon.sleep_n += mon.sleep_skipped;
            }
            mon.sleep_skipped = 0;
        }
        // The layers bite first, and a mon they kill greets nothing.
        self.entry_hazards(side, events);
        if self.sides[side].mon().fainted() {
            return;
        }
        // What this mon walks in WITH is what greets the field. Trace copies
        // at the same moment, but an ability handed over in this era is never
        // started — the sim gates that on gen > 3 — so a traced Intimidate
        // cows nobody and a traced Drizzle brings no rain.
        let own = self.sides[side].mon().ability;
        let foe_ability = self.sides[1 - side].mon().ability;
        if ability::traces(&self.sides[side].mon().bearer())
            && !self.sides[1 - side].mon().fainted()
        {
            self.set_ability(side, foe_ability);
        }
        let greeter = ability::Bearer {
            ability: own,
            ..self.sides[side].mon().bearer()
        };
        // A weather ability lays its sky down on arrival. Gen 3 gives it no
        // clock at all: it holds until something else sets the weather.
        if let Some(sky) = ability::weather_on_entry(&greeter) {
            let weather = match sky {
                "rain" => Weather::Rain,
                "sun" => Weather::Sun,
                _ => Weather::Sandstorm,
            };
            // A weather ability re-laying the sky it already found is NOT a
            // no-op in this era. `setWeather` bails early on a repeat only
            // when the source is an ability AND the duration is already
            // endless; a five-turn sandstorm from the MOVE is a duration it
            // will happily overwrite with its own endless one. So a Tyranitar
            // walking into someone else's sandstorm makes it permanent.
            let already_endless = self.weather == Some(weather) && self.weather_n == u8::MAX;
            if !already_endless {
                let changed = self.weather != Some(weather);
                self.weather = Some(weather);
                self.weather_n = u8::MAX;
                if changed {
                    events.push(Event::WeatherStarted { weather });
                    for w in 0..2 {
                        self.forecast(w);
                    }
                }
            }
        }
        // The arrival's own item `onStart`, which `runSwitch` fires after the
        // ability has started: this is the White Herb answering an Intimidate
        // that landed a moment ago on the other side of the field, and the
        // one that catches a negative boost carried in on a Baton Pass. It
        // sits in `greet` rather than in the switch wrapper because the
        // battle's OPENING goes through here too, where both mons greet each
        // other in turn.
        self.white_herb(side);
        // Forecast comes in at `onSwitchInPriority: -2`, under every other
        // arrival handler, so it reads a sky the newcomer may have just laid
        // down itself. This sits in `greet` rather than in the switch wrapper
        // because the battle's OPENING goes through here too.
        self.forecast(side);
        // Intimidate cows whatever is standing across the field — but not
        // through a substitute, and in this era not at all if the only
        // target has one up.
        if ability::intimidates(&greeter)
            && !self.sides[1 - side].mon().fainted()
            && self.sides[1 - side].mon().sub_hp == 0
            && !ability::blocks_drop(&self.sides[1 - side].mon().bearer(), ability::Drop::Attack)
            && self.sides[1 - side].mist_n == 0
        {
            self.sides[1 - side].mon_mut().apply_boost(Boost::Atk, -1);
            events.push(Event::Boosted {
                side: (1 - side) as u8 + 1,
                boost: Boost::Atk,
                delta: -1,
            });
        }
        if tidy {
            self.ability_update(side);
        }
    }

    /// The hazards the arrival walks onto, which gen 3 runs BEFORE its ability
    /// and item are started: `runSwitch` is `runEvent('EntryHazard')`, then
    /// `runEvent('SwitchIn')`, then `if (!pokemon.hp) return false`, and only
    /// then the two `singleEvent('Start', ...)` calls. So a mon whose Trace
    /// borrows a Levitate still eats the Spikes it stepped on — the ability it
    /// arrives with is the one that decides — and a mon the layers KILL never
    /// gets to greet the field at all: no Intimidate, no Trace, no weather, no
    /// item.
    pub(super) fn entry_hazards(&mut self, side: usize, events: &mut Vec<Event>) {
        let layers = self.sides[side].spikes;
        if layers == 0 {
            return;
        }
        let mon = self.sides[side].mon();
        let (t1, t2) = mon.types();
        // Spikes only bite what stands on the ground, and Levitate is the
        // other way to be off it.
        if t1 == Type::Flying || t2 == Type::Flying || mon.ability == "levitate" {
            return;
        }
        let max = mon.max_hp;
        let amount = match layers {
            1 => max / 8,
            2 => max / 6,
            _ => max / 4,
        }
        .max(1);
        let mon = self.sides[side].mon_mut();
        let amount = amount.min(mon.hp);
        mon.hp -= amount;
        events.push(Event::SpikesDamage {
            side: side as u8 + 1,
            amount,
        });
        self.announce_faint(side, events);
    }

    /// Announce whoever is down — target first, then the user (recoil can
    /// faint it too). Replacements are NOT sent in here: the sim only asks
    /// for one at an action boundary, so the empty slot has to survive to
    /// the end of the current move.
    pub(super) fn resolve_faints(&mut self, side: usize, foe: usize, events: &mut Vec<Event>) {
        // Destiny Bond: KO the bonded target with a move, go down with it.
        if self.sides[foe].mon().fainted() && self.sides[foe].mon().destiny {
            self.sides[side].mon_mut().hp = 0;
        }
        // Grudge: the killing move's PP drains to nothing — unless the mon
        // that used it is already dead by then. `onFaint` opens with
        // `if (!source || source.fainted || ...) return`, and a move with
        // `selfdestruct: "always"` queues ITS OWN user's faint before the hit
        // lands, so by the time the Grudge holder's Faint event runs the
        // attacker is gone and takes its PP with it. Recoil is different: the
        // target is dequeued first there, so a Double-Edge that kills its
        // user still loses its PP.
        if self.sides[foe].mon().fainted()
            && self.sides[foe].mon().grudged
            && !self.self_destructed
        {
            if let Some(slot_i) = self.sides[side].mon().last_used {
                if let Some(ms) = self.sides[side].mon_mut().moves.get_mut(slot_i as usize) {
                    ms.pp = 0;
                }
            }
        }
        for who in [foe, side] {
            self.announce_faint(who, events);
        }
    }
}
