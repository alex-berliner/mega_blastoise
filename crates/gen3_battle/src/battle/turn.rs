//! The turn driver: choice resolution order and the whole-turn `step_with`,
//! end-of-turn phase included.

extern crate alloc;

use alloc::vec::Vec;

use crate::ability;
use crate::data::{
    Boost, SideCondition, Status, Weather,
};
use crate::types::Type;

use super::*;

impl Battle {
    /// Resolve one turn.
    ///
    /// Order is switches first, then moves by Speed with the faster mon going
    /// first and ties broken by the RNG, which is how Gen 3 does it.
    ///
    /// Unsupported so far, and silently absent rather than wrongly modelled:
    /// abilities, held items, weather, move priority, secondary effects, and
    /// multi-turn moves.
    pub fn step(&mut self, choices: [Choice; 2]) -> Vec<Event> {
        self.step_with(choices, &TurnScript::default())
    }

    /// [`Battle::step`], with any scripted seats forced down their scripted
    /// branch instead of rolling. The play path always passes an empty script;
    /// the parity tests pass the same script they gave the reference sim.
    pub fn step_with(&mut self, choices: [Choice; 2], script: &TurnScript) -> Vec<Event> {
        let mut events = Vec::new();
        // The sim sorts this turn's actions as the choices come in, which is
        // before the turn's first update(). A Cheri Berry eaten a moment
        // later cures the paralysis but does not reorder the turn, so the
        // speeds are read here and held.
        let scripted_now = script.seats.iter().any(|s| s.is_some());
        // The sim flips the turn's Quick Claw coin at the tail of `nextTurn`,
        // which is before any choice is read — so it is settled here, ahead of
        // the speed order it decides.
        self.turn_seats = script.seats;
        self.claw_this_turn = if scripted_now {
            script.claw
        } else {
            self.rng.below(5) == 0
        };
        self.speed_first = Some(self.faster_side(scripted_now));
        for side in 0..2 {
            self.ability_update(side);
            if !self.sides[side].mon().fainted() {
                let mon = self.sides[side].mon_mut();
                mon.active_turns = mon.active_turns.saturating_add(1);
            }
        }
        self.turn += 1;
        self.acted_this_turn = [false; 2];
        self.dragged = [false; 2];
        self.taken_physical = [0; 2];
        self.taken_special = [0; 2];
        self.hit_this_turn = [false; 2];

        for side in 0..2 {
            self.pp0_at_choice[side] = match choices[side] {
                Choice::Move(i) => self.sides[side]
                    .mon()
                    .moves
                    .get(i)
                    .is_some_and(|m| m.pp == 0),
                _ => false,
            };
            // The Imprison seal is read HERE, before anyone switches, and
            // held for the turn. The sim recomputes its disable flags once in
            // `nextTurn` and then commits the choice against them, so an
            // imprisoner that leaves the field partway through the turn does
            // not hand its victim's moves back until the next one.
            let foe = 1 - side;
            self.sealed_at_choice[side] = match choices[side] {
                Choice::Move(i) => {
                    let sealed_id = self.sides[side]
                        .mon()
                        .moves
                        .get(i)
                        .map(|m| m.entry.id);
                    self.sides[foe].mon().imprisoning
                        && sealed_id.is_some_and(|id| {
                            self.sides[foe].mon().moves.iter().any(|m| m.entry.id == id)
                        })
                }
                _ => false,
            };
        }
        // A seat using Focus Punch tightens its focus before anything else
        // happens this turn (the sim's before-turn charge step); while
        // focusing, the flinch volatile is refused outright.
        //
        // The charge belongs to the move that is QUEUED, not to the slot
        // that was picked: the sim throws a locked mon's choice away whole
        // (`chooseMove` pushes the locked move and returns), so a Rollout
        // or a recharge never tightens a focus it will not punch with, and
        // with no volatile there, the flinch lands on it instead.
        for side in 0..2 {
            if let Choice::Move(i) = choices[side] {
                let mon = self.sides[side].mon();
                let focusing = self.acting_move_id(side, i) == Some("focuspunch");
                if focusing && !mon.must_recharge && !mon.fainted() {
                    self.sides[side].mon_mut().focusing = true;
                }
            }
        }

        // Pursuit fires at a mon on its way out, before the switch happens
        // and at double power. The user's own action is spent doing it (the
        // sim cancels the queued move), and if the strike lands a KO the
        // chosen switch is not cancelled — this era re-queues it for the end
        // of the turn, so the slot simply stays empty until then.
        // Whether each side MAY switch is settled once, before anything
        // moves. The sim decides it when it builds the turn's request, so
        // two mons can trade places even though each would hold the other
        // in place once it arrived — re-asking mid-turn let the first
        // switch-in's Shadow Tag cancel the second side's answer.
        let may_switch = [self.can_switch(0), self.can_switch(1)];
        let mut pursued = [false; 2];
        self.deferred_switch = [None; 2];
        for side in 0..2 {
            let Choice::Switch(idx) = choices[side] else {
                continue;
            };
            if !may_switch[side] || self.sides[side].mon().fainted() {
                continue;
            }
            let foe = 1 - side;
            let Choice::Move(mi) = choices[foe] else {
                continue;
            };
            // The chosen slot only counts if the mon is free to use it: one
            // locked into a charge, a rampage, a Rollout, a Bide or a
            // recharge is swinging that instead, and the sim never offers
            // Pursuit in its request to begin with.
            let locked = {
                let m = self.sides[foe].mon();
                m.must_recharge
                    || m.charging.is_some()
                    || m.rampage.is_some()
                    || m.rolling.is_some()
                    || m.bide.is_some()
            };
            // Nor does it count when the request would have handed the mon
            // Struggle. The sim builds its queue out of `getMoves`, which
            // greys out the last move of a mon under Torment, a Disabled
            // slot, a Taunted status move, a move the imprisoning foe knows
            // and every move but a Choice lock, and answers a mon with no
            // usable move at all with Struggle. `beforeTurnMove` and its
            // Pursuit volatile hang off the action as queued, so a mon that
            // was going to Struggle never arms one and there is nothing to
            // intercept; it takes its own turn after the switch instead.
            let is_pursuit = !locked
                && !self.struggles_at_choice(foe, mi)
                && self.sides[foe]
                    .mon()
                    .moves
                    .get(mi)
                    .is_some_and(|m| m.entry.id == "pursuit");
            // The interception has its own refusal guard, and it turns away
            // a loafing Truant user exactly as it turns away a frozen or a
            // sleeping one. The guard returns BEFORE `queue.cancelMove`, so
            // the pursuiter keeps its own queued action and takes its normal
            // turn after the switch — where Truant stops it for nothing, no
            // damage and no PP.
            let able = !self.sides[foe].mon().fainted()
                && !matches!(
                    self.sides[foe].mon().status,
                    Some(Status::Freeze | Status::Sleep)
                )
                && !(self.sides[foe].mon().loafing
                    && ability::truant(&self.sides[foe].mon().bearer()));
            if !is_pursuit || !able {
                continue;
            }
            self.pursuing = true;
            let landed = self.use_move(foe, mi, script.seats[foe], &mut events);
            self.run_pending_call(script, &mut events);
            // The interception answers no `AfterMove`: the sim fires it from
            // the switch-out hook through `useMove`, below `runMove`, so
            // Charge's cleanup never runs for it. Gen 3's Pursuit writes the
            // band's lock by hand at the interception itself, right where the
            // strike is (`if (this.actions.useMove(...) && source.getItem()
            // .isChoice) source.addVolatile("choicelock")`), so the lock
            // still goes on. (Ours reads the strike as "reached the move",
            // so a blow the sim would call a miss still locks; nothing in
            // this era's Pursuit reaches that branch but a blow it absorbs.)
            if landed {
                self.apply_choice_lock(foe, Some("pursuit"));
            }
            self.settle_choice_lock(foe, false);
            // No White Herb here: the sim fires an interception from the
            // switch-out hook through `useMove`, the caller BELOW `runMove`,
            // and only `runMove` runs `AfterMove`. The arrival that follows
            // it is a switch-in, which answers one anyway.
            self.end_of_action();
            self.pursuing = false;
            pursued[foe] = true;
            if self.sides[side].mon().fainted() {
                self.deferred_switch[side] = Some(idx);
            }
        }

        // Switches resolve before any move, in side order. Leaving the field
        // resets a Toxic count: the poison stays, the clock starts over.
        // Switches resolve in SPEED order — the speed of the mon leaving —
        // and each arrival greets the field the moment it lands, before the
        // other side has moved. That is what decides who an Intimidate cows
        // and what a Trace finds standing opposite: on a double switch the
        // slower side's newcomer is greeted by a field that has already
        // changed, and the faster side's by one that has not.
        let switch_first = self.faster_side(scripted_now);
        for side in [switch_first, 1 - switch_first] {
            if self.deferred_switch[side].is_some() {
                continue;
            }
            if let Choice::Switch(idx) = choices[side] {
                if !may_switch[side] {
                    // Held in place, or locked into a move of its own:
                    // switching is refused and the turn is forfeit.
                    continue;
                }
                if idx < self.sides[side].party.len() && !self.sides[side].party[idx].fainted() {
                    self.switch_out_reset(side);
                    let slot_item = self.hand_slot_item_over(side);
                    self.sides[side].reorder_for_switch(idx);
                    self.sides[side].active = idx;
                    self.sides[side].mon_mut().last_item = slot_item;
                    events.push(Event::Switched {
                        side: side as u8 + 1,
                        party_index: idx,
                    });
                    self.switch_in_greet(side, &mut events);
                    self.end_of_action();
                }
            }
            // `faintMessages` runs cancelAction over every mon still standing
            // the moment anything goes down, and in gen 3 that takes SWITCHES
            // as well as moves. A newcomer that walked onto Spikes and died
            // there cancels the other side's switch before it happens.
            if (0..2).any(|s| self.sides[s].mon().fainted()) {
                break;
            }
        }

        // Whether anything is lying down as the move phase opens. This has to
        // be read BEFORE the replacement is swapped in, or the empty slot is
        // already filled and the cancellation never happens.
        let already_down = (0..2).any(|s| self.sides[s].mon().fainted());
        // A switch-in that dropped to Spikes is replaced before anyone moves.
        self.replace_fainted(&mut events);

        // Then moves: priority bracket first, Speed inside a bracket.
        let scripted = script.seats.iter().any(|s| s.is_some());
        let first = self.first_mover(&choices, scripted);
        // Going down cancels that side's queued action outright, and the
        // replacement does not inherit it — so the cancellation is recorded
        // BEFORE anyone is swapped in, while the slot is still empty.
        let mut cancelled = [false; 2];
        for side in 0..2 {
            cancelled[side] = pursued[side] || already_down;
        }
        let rest = TurnRest::Moves {
            from: 0,
            first: first as u8,
            choices,
            cancelled,
        };
        self.finish_turn(rest, *script, &mut events);
        events
    }

    /// The rest of a turn, from `rest` on. A replacement the player has to
    /// choose STOPS it there: in this era the sim checks for faints at every
    /// action boundary (`checkFainted` when the next action is a move or the
    /// residual) and asks for the switch on the spot, and `commitChoices`
    /// queues the answer AHEAD of what was left of the turn. So the turn is
    /// parked, `send_out` picks it back up, and the pick is on the field for
    /// the residual phase exactly as the lowest-slot default's is.
    pub(super) fn finish_turn(
        &mut self,
        mut rest: TurnRest,
        script: TurnScript,
        events: &mut Vec<Event>,
    ) {
        loop {
            if self.awaiting_replacement.iter().any(|&w| w) {
                self.parked_turn = Some((rest, script));
                return;
            }
            match rest {
                TurnRest::Moves {
                    from,
                    first,
                    choices,
                    cancelled,
                } => {
                    rest = self.move_phase(
                        from as usize,
                        first as usize,
                        choices,
                        cancelled,
                        &script,
                        events,
                    );
                }
                TurnRest::DeferredSwitches => {
                    self.deferred_switches(events);
                    rest = TurnRest::Residual;
                }
                TurnRest::Residual => {
                    self.residual_phase(&script, events);
                    rest = TurnRest::Tidy;
                }
                TurnRest::Tidy => {
                    self.tidy_turn(events);
                    return;
                }
            }
        }
    }

    /// The move phase, from the mover at `from` in the turn's order on.
    /// Returns where the turn goes next: on to the deferred switches, or back
    /// here for the second mover once the player has answered.
    fn move_phase(
        &mut self,
        from: usize,
        first: usize,
        choices: [Choice; 2],
        mut cancelled: [bool; 2],
        script: &TurnScript,
        events: &mut Vec<Event>,
    ) -> TurnRest {
        // A pass target that walked onto Spikes and went down there throws
        // the rest of the turn away like any other faint.
        if (0..2).any(|s| self.sides[s].mon().fainted()) {
            cancelled = [true; 2];
        }
        for (k, side) in [first, 1 - first].into_iter().enumerate().skip(from) {
            if self.over() {
                break;
            }
            if !cancelled[side] {
                if let Choice::Move(index) = choices[side] {
                    self.acted_this_turn[side] = true;
                    // Whoever is left in the order still has an action; the
                    // second mover has nobody behind it.
                    let foe = 1 - side;
                    self.will_act =
                        side == first && !cancelled[foe] && matches!(choices[foe], Choice::Move(_));
                    let moved = self.use_move(side, index, script.seats[side], events);
                    self.run_pending_call(script, events);
                    // The band's lock and Charge's cleanup both hang off
                    // `AfterMove`, so they answer an action only if it got
                    // that far: a frozen, sleeping or fully-paralysed mon,
                    // or a slot drained dry since the choice, leaves both
                    // standing. An intercepting Pursuit answers neither.
                    self.settle_choice_lock(side, moved);
                    // `onAnyAfterMove`: a White Herb on either seat undoes
                    // what the move just did to it, without waiting for the
                    // end of the turn. The sim runs that event out of
                    // `runMove`, so an action a gate turned away before the
                    // move (a frozen mon, a full paralysis, a slot drained
                    // dry since the choice) never answers one.
                    if moved {
                        self.white_herb(side);
                        self.white_herb(1 - side);
                    }
                    self.end_of_action();
                }
            }
            // ANY faint stops the rest of the turn dead in this era. The
            // sim's faintMessages runs `cancelAction` over every active mon
            // when `gen <= 3 && singles`, not just over the one that went
            // down — so the survivor's queued move is thrown away too. A
            // one-mon battle could never show this: the faint ended it.
            if (0..2).any(|s| self.sides[s].mon().fainted()) {
                cancelled = [true; 2];
            }
            // A mon dragged off by Roar or Whirlwind takes its action with
            // it; whoever the drag brought in does not get to use it.
            for s in 0..2 {
                cancelled[s] |= self.dragged[s];
            }
            // The sim checks for faints at every action boundary in this
            // era (`gen <= 3` in checkFainted's guard), so a replacement is
            // already on the field when the residual phase runs.
            self.replace_fainted(events);
            // A question for the player (a Baton Pass target, or a faint's
            // replacement) is answered before the next action: the sim makes
            // the switch request at the end of `runAction` and queues the
            // answer as an instaswitch, ahead of the move still to come.
            if k == 0 && self.awaiting_replacement.iter().any(|&w| w) {
                return TurnRest::Moves {
                    from: 1,
                    first: first as u8,
                    choices,
                    cancelled,
                };
            }
        }
        TurnRest::DeferredSwitches
    }

    fn deferred_switches(&mut self, events: &mut Vec<Event>) {
        // The switch a Pursuit KO pushed back takes its turn now, once the
        // moves are done and BEFORE the residuals — the sim re-queues it at
        // priority -101, so the mon it brings in is on the field to take its
        // own poison tick.
        for side in 0..2 {
            if let Some(idx) = self.deferred_switch[side].take() {
                if idx < self.sides[side].party.len() && !self.sides[side].party[idx].fainted() {
                    self.sides[side].mon_mut().status = None;
                    let slot_item = self.hand_slot_item_over(side);
                    self.sides[side].reorder_for_switch(idx);
                    self.sides[side].active = idx;
                    self.sides[side].mon_mut().last_item = slot_item;
                    events.push(Event::Switched {
                        side: side as u8 + 1,
                        party_index: idx,
                    });
                    self.switch_in_greet(side, events);
                }
            }
        }
        self.replace_fainted(events);
    }

    fn residual_phase(&mut self, script: &TurnScript, events: &mut Vec<Event>) {
        let scripted = script.seats.iter().any(|s| s.is_some());
        // The whole end-of-turn phase is skipped once the battle is decided
        // — but a wipe DURING the phase does not cut it short (the sim
        // finishes both weather chips even when the first one ends it).
        if !self.over() {
            // Weather chips first — the games' upkeep slot: sandstorm hits
            // anything not Rock/Ground/Steel, hail anything not Ice, a
            // sixteenth each, faster side first.
            // Wish arrives at the end of the turn after it was made — at the
            // games' order 4, BEFORE the weather chips and the status ticks
            // (the sim logs the heal ahead of the hail) — healing whoever is
            // active then.
            let mut wish_present = [false; 2];
            for side in 0..2 {
                if self.sides[side].wish_n > 0 {
                    wish_present[side] = true;
                    self.sides[side].wish_n -= 1;
                    if self.sides[side].wish_n == 0 {
                        // Half of whoever CATCHES it, not half of whoever
                        // made it: `target.baseMaxhp / 2` in the sim's onEnd.
                        // Gen 5 moved it to the wisher; this era did not.
                        let want = self.sides[side].mon().max_hp / 2;
                        if !self.sides[side].mon().fainted() {
                            self.heal(side, want, events);
                        }
                    }
                }
            }

            // The clock runs down first: the sim's field residual decrements the
            // weather's duration and clears it before `onWeather` would chip, so
            // a five-turn sandstorm lands FOUR ticks, not five. Nothing under a
            // three-turn fuzz could ever have noticed.
            if let Some(weather) = self.weather {
                self.weather_n = self.weather_n.saturating_sub(1);
                if self.weather_n == 0 {
                    self.weather = None;
                    events.push(Event::WeatherEnded { weather });
                    for w in 0..2 {
                        self.forecast(w);
                    }
                }
            }

            if matches!(self.effective_weather(), Some(Weather::Sandstorm | Weather::Hail)) {
                let sand = self.effective_weather() == Some(Weather::Sandstorm);
                let first = self.faster_side(scripted);
                for side in [first, 1 - first] {
                    let mon = self.sides[side].mon();
                    if mon.fainted() {
                        continue;
                    }
                    let (t1, t2) = mon.types();
                    let immune = if sand {
                        [t1, t2]
                            .iter()
                            .any(|t| matches!(t, Type::Rock | Type::Ground | Type::Steel))
                            || ability::immune_to_sandstorm(&mon.bearer())
                    } else {
                        t1 == Type::Ice || t2 == Type::Ice
                    };
                    if immune {
                        continue;
                    }
                    // Underground or underwater shelters from the weather;
                    // mid-Fly/Bounce does not.
                    if matches!(mon.semi_invulnerable(), Some("dig" | "dive")) {
                        continue;
                    }
                    let amount = (self.sides[side].mon().max_hp / 16).max(1);
                    let mon = self.sides[side].mon_mut();
                    let amount = amount.min(mon.hp);
                    mon.hp -= amount;
                    events.push(Event::WeatherDamage {
                        side: side as u8 + 1,
                        amount,
                    });
                    self.announce_faint(side, events);
                }
            }

            // The sim gathers every residual handler on the field into ONE
            // list and hands it to `speedSort`. Running that sort for real,
            // rather than guessing its shape, is the only way to get the ties
            // right: `speedSort` is a SELECTION sort, and a selection sort is
            // not stable. When it finds its minimum behind a tied pair it
            // swaps that minimum forward, and the element it displaces lands
            // where the minimum was — at the BACK of the pair. So a third
            // handler with a lower key can reverse two tied ones just by
            // being there. Speeds are read once, up front, exactly as the sim
            // reads them: `case 'residual'` calls `updateSpeed()` before the
            // field event, so a Salac eaten mid-phase does not reshuffle it.
            let speeds = [self.turn_speed(0), self.turn_speed(1)];
            // The list holds only the handlers that EXIST, because presence
            // is what displacement runs on. An earlier cut pushed every
            // bucket for both sides and let absent ones no-op — which sorts
            // to the same order for distinct Speeds, but at a TIE the
            // symmetric phantoms cancel the selection sort's displacement
            // and every tied pair came out side-1-first. The sim's list for
            // a Transformed pair holding one Wiki Berry has THREE entries,
            // and it is the berry's presence behind the two tied Leech Seeds
            // that flings the first seed to the back.
            //
            // Insertion order mirrors `findPokemonEventHandlers`: per side,
            // the status handler, then the volatiles, then the ability, then
            // the item — and after each side's mon, its side conditions,
            // which is where a pending Wish sits. The Wish entry is a marker:
            // its EFFECT already ran in the wish phase above (order 7 sorts
            // ahead of everything here), but its presence in the list still
            // displaces ties, so it sorts along and runs nothing. Fainted
            // mons' handlers are gathered too — the sim collects them and
            // skips them at run time, and they still occupy slots.
            const WISH_MARK: usize = usize::MAX;
            let mut plan: Vec<(usize, usize)> = Vec::new();
            for s in 0..2 {
                let mon = &self.sides[s].party[self.sides[s].active];
                let ability = mon.ability;
                // Status: only brn/psn/tox carry a residual handler.
                if matches!(
                    mon.status,
                    Some(Status::Burn | Status::Poison | Status::Toxic)
                ) {
                    plan.push((4, s));
                }
                // Volatiles with a residual callback or a bare duration.
                // The sim pools one mon's volatiles in the order they were
                // ADDED: `findPokemonEventHandlers` walks
                // `pokemon.volatiles` by insertion order, so a mon seeded
                // before it was rooted hands the pool its Leech Seed ahead
                // of its Ingrain. Two of one mon's volatiles that tie in
                // `comparePriority` are broken by nothing else, and the
                // selection sort moves tied pairs about, so that order
                // decides which way they come out. `add_volatile` stamped
                // each one with the clock it landed on; that is the order
                // here.
                let mut vol: [(usize, u16); VOL_COUNT] = [(0, 0); VOL_COUNT];
                let mut nv = 0;
                if mon.ingrained {
                    vol[nv] = (0, mon.vol_stamp[VOL_INGRAIN]);
                    nv += 1;
                }
                if mon.seeded {
                    vol[nv] = (3, mon.vol_stamp[VOL_LEECH_SEED]);
                    nv += 1;
                }
                if mon.nightmared {
                    vol[nv] = (5, mon.vol_stamp[VOL_NIGHTMARE]);
                    nv += 1;
                }
                if mon.cursed {
                    vol[nv] = (6, mon.vol_stamp[VOL_CURSE]);
                    nv += 1;
                }
                // A stale bind has a handler here too: the sim's volatile
                // outlives its user and deletes itself in `onResidual`, so
                // it is still one of the gathered handlers on the turn its
                // clock ran out under something else (a Rapid Spin or a
                // Substitute zeroes the clock, and the flag has to be spent
                // here or it eats the NEXT bind's first chip).
                if mon.trapped_n > 0 || mon.trap_stale {
                    vol[nv] = (7, mon.vol_stamp[VOL_BIND]);
                    nv += 1;
                }
                if mon.bide.is_some() {
                    vol[nv] = (13, mon.vol_stamp[VOL_BIDE]);
                    nv += 1;
                }
                if mon.must_recharge {
                    vol[nv] = (8, mon.vol_stamp[VOL_RECHARGE]);
                    nv += 1;
                }
                if mon.rampage.is_some() {
                    let bucket = if mon.locked_move == Some("uproar") { 9 } else { 11 };
                    vol[nv] = (bucket, mon.vol_stamp[VOL_RAMPAGE]);
                    nv += 1;
                }
                if mon.yawn_n > 0 {
                    vol[nv] = (10, mon.vol_stamp[VOL_YAWN]);
                    nv += 1;
                }
                vol[..nv].sort_by_key(|&(_, stamp)| stamp);
                for &(bucket, _) in &vol[..nv] {
                    plan.push((bucket, s));
                }
                // Abilities.
                if matches!(ability, "raindish" | "shedskin" | "speedboost") {
                    plan.push((1, s));
                }
                if ability == "truant" {
                    plan.push((12, s));
                }
                // The item: presence is answered by the item module itself,
                // beside the effects — the hand-copied list this replaced
                // was one berry short.
                if crate::item::has_residual(mon.item) {
                    plan.push((2, s));
                }
                // This side's conditions, gathered after its mon. The sim
                // gathers the WHOLE list before running anything, so a wish
                // that resolved in the phase above was still present at
                // gather time — hence the flag captured before that phase.
                if wish_present[s] || self.sides[s].wish_n > 0 {
                    plan.push((WISH_MARK, s));
                }
            }
            speed_sort(&mut plan, |&(b, s)| {
                if b == WISH_MARK {
                    // Wish: order 7, ahead of every order-10 handler.
                    (7, 0, -(speeds[s] as i32), 0)
                } else {
                    residual_key(b, speeds[s])
                }
            });
            for (b, s) in plan {
                if self.over() {
                    break;
                }
                if b == WISH_MARK {
                    continue;
                }
                self.residual_bucket(b, s, scripted, events);
            }

            // The White Herb's own residual, at order 29 — after every tick
            // above it, and the last thing either side does.
            for side in 0..2 {
                self.white_herb(side);
            }

            // A Future Sight lands at the end of its third turn, computed from
            // the launcher's snapshot against the target now standing.
            //
            // Two of them land in the same residual are sorted by the SPEED OF
            // THE MON IN THE SLOT EACH ONE AIMS AT, not by seat: the sim hangs
            // the condition on the target's slot and finds it through that
            // slot's mon, so `resolvePriority` hands the handler that mon's
            // `getActionSpeed` and the residual's `speedSort` runs on it. The
            // faster target's delayed hit lands first, which is what decides
            // the battle when only one of the two knocks anything out.
            let first = self.faster_side(scripted);
            for side in [first, 1 - first] {
                if self.over() {
                    break;
                }
                if let Some((n, dealt, id)) = self.sides[side].incoming {
                    if n > 1 {
                        self.sides[side].incoming = Some((n - 1, dealt, id));
                    } else {
                        self.sides[side].incoming = None;
                        let mon = self.sides[side].mon();
                        if !mon.fainted() {
                            // A taken aim reaches the mon the launcher
                            // locked on, whatever it is doing: the sim's
                            // lockon condition answers the Invulnerability
                            // event the landing runs through, and then the
                            // accuracy one that follows it.
                            let sure = self.sides[1 - side].mon().sure_hit > 0
                                && self.sides[1 - side].mon().sure_hit_on as usize
                                    == self.sides[side].active;
                            // A target mid Fly/Dig/Bounce/Dive when the hit
                            // arrives dodges it like any other attack.
                            if !sure && mon.semi_invulnerable().is_some() {
                                continue;
                            }
                            // Only ACCURACY waits for the landing — 90 for
                            // Future Sight, 85 for Doom Desire — off the
                            // launcher's seat script for that turn. A miss
                            // simply drops the delayed hit. The damage itself
                            // was locked in at launch; the sim even strips the
                            // target's Endure before it lands.
                            let landed = sure
                                || match script.seats[1 - side] {
                                    Some(sc) => sc.hit,
                                    None => {
                                        self.rng.below(100)
                                            < if id == "doomdesire" { 85 } else { 90 }
                                    }
                                };
                            if !landed {
                                continue;
                            }
                            if dealt > 0 {
                                // The band is the one thing that can still cut
                                // the landing blow back to a single point. Endure
                                // is not: the sim's `futuremove.onEnd` strips it
                                // off the target before the move is resolved.
                                let banded = self.band_saves(side);
                                let mon = self.sides[side].mon_mut();
                                let hit_sub = mon.sub_hp > 0;
                                if hit_sub {
                                    let amount = dealt.min(mon.sub_hp);
                                    mon.sub_hp -= amount;
                                    events.push(Event::SubDamage {
                                        side: side as u8 + 1,
                                        amount,
                                    });
                                    if self.sides[side].mon().sub_hp == 0 {
                                        events.push(Event::SubBroke {
                                            side: side as u8 + 1,
                                        });
                                    }
                                } else {
                                    let cap = if banded {
                                        mon.hp.saturating_sub(1)
                                    } else {
                                        mon.hp
                                    };
                                    let amount = dealt.min(cap);
                                    mon.hp -= amount;
                                    events.push(Event::Damage {
                                        side: side as u8 + 1,
                                        amount,
                                        effectiveness: 100,
                                        crit: false,
                                    });
                                    // It also goes into the attacked-by book,
                                    // which the sim writes at the end of the
                                    // same trySpreadMoveHit. That matters
                                    // because Doom Desire and Future Sight
                                    // are both on Mirror Move's refusal list:
                                    // a delayed hit landing is what a Mirror
                                    // Move aimed afterwards finds, and it
                                    // fails on it.
                                    let who = self.sides[1 - side].active;
                                    let mon = self.sides[side].mon_mut();
                                    mon.last_hit_by = Some(id);
                                    mon.last_hit_by_slot = Some(who);
                                    // A delayed hit is a real hit: the sim
                                    // resolves it through trySpreadMoveHit,
                                    // so the Hit event runs and everything
                                    // that answers being struck answers this
                                    // too. A raging target's Attack climbs,
                                    // and a Bide banks it.
                                    if let Some((stored, left)) = self.sides[side].mon().bide {
                                        self.sides[side].mon_mut().bide =
                                            Some((stored.saturating_add(amount), left));
                                    }
                                    if self.sides[side].mon().raging
                                        && !self.sides[side].mon().fainted()
                                    {
                                        self.boost(side, Boost::Atk, 1, events);
                                    }
                                    self.announce_faint(side, events);
                                }
                            }
                        }
                    }
                }
            }

            // The perish count falls; at zero the song collects. Both songs
            // sit at residual order 12, so Speed decides which counts down
            // first — and the count is a `duration` handler, which the sim
            // runs down its OWN branch: `handler.end()` with no faintMessages
            // after it. A perish KO therefore only queues the faint, the
            // battle is not declared over inside the phase, and the other
            // side's song still counts. Two mons on zero go down together.
            // …but none of that runs at all when an EARLIER residual ended
            // the battle: `fieldEvent` returns at `if (this.ended)` after the
            // handler that decided it, and the counts below it never tick. A
            // poison KO of the last mon leaves the other side's song
            // unfinished and its singer alive.
            let perish_first = self.faster_side(scripted);
            // Decided BEFORE the counts, not by them: an earlier residual
            // ending the battle skips both songs — but a perish KO itself
            // only queues the faint (the duration branch runs no
            // faintMessages), so the second song still counts and two mons
            // on zero go down together.
            let decided_before_perish = self.over();
            for side in [perish_first, 1 - perish_first] {
                if decided_before_perish {
                    break;
                }
                if self.sides[side].mon().perish_n > 0 && !self.sides[side].mon().fainted() {
                    self.sides[side].mon_mut().perish_n -= 1;
                    let n = self.sides[side].mon().perish_n;
                    events.push(Event::PerishCount {
                        side: side as u8 + 1,
                        n,
                    });
                    if n == 0 {
                        self.sides[side].mon_mut().hp = 0;
                        self.announce_faint(side, events);
                    }
                }
            }

            // Team conditions run out: five end-of-turn ticks including the
            // turn they went up.
            for side in 0..2 {
                for cond in [
                    SideCondition::Reflect,
                    SideCondition::LightScreen,
                    SideCondition::Safeguard,
                    SideCondition::Mist,
                ] {
                    let n = self.sides[side].condition_n(cond);
                    if *n > 0 {
                        *n -= 1;
                        if *n == 0 {
                            events.push(Event::SideEnded {
                                side: side as u8 + 1,
                                condition: cond,
                            });
                        }
                    }
                }
            }

            // Taunt, Encore and Disable wear off on their own short clocks.
            for side in 0..2 {
                let mon = self.sides[side].mon_mut();
                if mon.taunt_n > 0 {
                    mon.taunt_n -= 1;
                }
                if mon.encore_n > 0 {
                    mon.encore_n -= 1;
                    // Encore also ends the moment the move it is forcing runs
                    // out of PP, which its `onResidual` checks straight after
                    // the duration tick. The clock and the PP are two separate
                    // ends: the sim takes the duration branch first and only
                    // asks about PP when that branch did not fire.
                    let spent = mon
                        .encored_slot
                        .and_then(|i| mon.moves.get(i as usize))
                        .is_none_or(|m| m.pp == 0);
                    if mon.encore_n > 0 && spent {
                        mon.encore_n = 0;
                        mon.encore_fresh = false;
                        mon.encored_slot = None;
                    }
                }
                if mon.disable_n > 0 {
                    mon.disable_fresh = false;
                    if mon.disable_skip_tick {
                        mon.disable_skip_tick = false;
                        continue;
                    }
                    mon.disable_n -= 1;
                    if mon.disable_n == 0 {
                        mon.disabled_slot = None;
                    }
                }
            }
        }

        // The residual phase is one action as far as the queue is concerned:
        // whoever it knocked out is replaced when it finishes, not between
        // its individual ticks.
        self.replace_fainted(events);
    }

    fn tidy_turn(&mut self, events: &mut Vec<Event>) {
        // `nextTurn` walks each mon's attacked-by book and SPLICES OUT every
        // entry whose attacker is no longer on the field. It is a destructive
        // purge, not a filter applied at read time, so a mon that hit you and
        // then switched out leaves no record — and coming back in does not
        // restore it, because the entry is already gone. Mirror Move is the
        // only reader, and this is why it fails against an attacker that took
        // a lap on the bench, even one that returned to the same party slot.
        for side in 0..2 {
            let foe = 1 - side;
            if self.sides[side].mon().last_hit_by_slot != Some(self.sides[foe].active) {
                let mon = self.sides[side].mon_mut();
                mon.last_hit_by = None;
                mon.last_hit_by_slot = None;
            }
        }

        // A flinch lasts exactly the turn it landed in — as do Protect's
        // shield and Endure's brace.
        for side in 0..2 {
            let mon = self.sides[side].mon_mut();
            mon.flinched = false;
            mon.protected = false;
            mon.magic_coat = false;
            mon.snatching = false;
            mon.enduring = false;
            mon.torment_fresh = false;
            mon.encore_fresh = false;
            mon.imprison_fresh = false;
            mon.uproar_ending = false;
            // Fury Cutter's ramp is a volatile with a two-turn clock that
            // only a landed Fury Cutter refreshes. So a turn that did not
            // land one is the turn it runs out, and it does not matter why:
            // a miss, a Truant loaf, a flinch and a full paralysis all end it
            // the same way, at the end of the turn rather than on the spot.
            if mon.fury_n > 0 {
                if mon.fury_fresh {
                    mon.fury_fresh = false;
                } else {
                    mon.fury_n = 0;
                }
            }
            // A rolling lock that did not swing this turn is over.
            if mon.rolling.is_some() {
                if mon.rolling_fresh {
                    mon.rolling_fresh = false;
                } else {
                    mon.rolling = None;
                    mon.locked_move = None;
                }
            }
            // A charge only gets the one turn to come down.
            if mon.charging.is_some() {
                if mon.charge_fresh {
                    mon.charge_fresh = false;
                } else {
                    mon.charging = None;
                    mon.charge_fresh = false;
                    mon.locked_move = None;
                }
            }
            mon.focusing = false;
            mon.sure_hit = mon.sure_hit.saturating_sub(1);
        }

        if let Some(win) = self.winner() {
            events.push(Event::Win { side: win });
        }
    }

    pub(super) fn first_mover(&mut self, choices: &[Choice; 2], scripted: bool) -> usize {
        let prio = |side: usize| match choices[side] {
            Choice::Move(i) => {
                if self.struggles_at_choice(side, i) {
                    0
                } else {
                    // A locked mon swings the LOCK, whatever the player
                    // picked, so the bracket is the lock's. Getting this
                    // from the chosen slot instead hands a mon mid-Ice-Ball
                    // the +3 of an Endure it will never use.
                    let mon = self.sides[side].mon();
                    let locked = mon
                        .locked_move
                        .filter(|_| {
                            mon.rampage.is_some()
                                || mon.rolling.is_some()
                                || mon.bide.is_some()
                                || mon.charging.is_some()
                        })
                        .and_then(crate::data::move_by_id);
                    match locked {
                        Some(e) => e.priority,
                        None if mon.must_recharge => 0,
                        None => mon.moves.get(i).map(|s| s.entry.priority).unwrap_or(0),
                    }
                }
            }
            Choice::Switch(_) => 0,
        };
        let (p0, p1) = (prio(0), prio(1));
        match p0.cmp(&p1) {
            core::cmp::Ordering::Greater => 0,
            core::cmp::Ordering::Less => 1,
            core::cmp::Ordering::Equal => match self.speed_first {
                // The order was settled as the turn opened; a berry eaten
                // since then cures the paralysis without reshuffling it.
                Some(first) => first,
                None => self.faster_side(scripted),
            },
        }
    }

    /// Which side moves first this turn: higher Speed (paralysis included),
    /// RNG on a tie in play. Under a script a tie goes to player 1, matching
    /// the reference sim with its tie-shuffle pinned to insertion order.
    pub(super) fn faster_side(&mut self, scripted: bool) -> usize {
        let s0 = self.turn_speed(0);
        let s1 = self.turn_speed(1);
        match s0.cmp(&s1) {
            core::cmp::Ordering::Greater => 0,
            core::cmp::Ordering::Less => 1,
            core::cmp::Ordering::Equal if scripted => 0,
            core::cmp::Ordering::Equal => self.rng.below(2) as usize,
        }
    }

    /// Who moves first given the chosen moves: higher priority bracket, then
    /// [`Battle::faster_side`] within it. A switch resolves before moves and
    /// takes no bracket.
    /// True when this seat's chosen move can only come out as Struggle —
    /// decided at CHOICE time, so turn order uses Struggle's priority 0
    /// instead of the dead move's (the sim's request step offers only
    /// Struggle, so the queued action never carries the old priority).
    pub(super) fn struggles_at_choice(&self, side: usize, i: usize) -> bool {
        let mon = self.sides[side].mon();
        let locked = mon.charging.is_some()
            || mon.rampage.is_some()
            || mon.bide.is_some()
            || mon.rolling.is_some();
        if locked {
            return false;
        }
        let Some(slot) = mon.moves.get(i) else {
            return false;
        };
        let status_movish = !slot.entry.damaging;
        let foe_mon = self.sides[1 - side].mon();
        slot.pp == 0
            // A Choice lock greys out every other slot at REQUEST time, which
            // is where the sim decides Struggle — so the action it queues
            // carries Struggle's own priority of zero, not the priority of
            // the move the player reached for. A Roar picked through a lock
            // does not move last; it Struggles at nought.
            || mon.choice_locked.is_some_and(|id| id != slot.entry.id)
            || mon.disabled_slot == Some(i as u8)
            || (mon.tormented && mon.last_used_id == Some(slot.entry.id))
            || (mon.taunt_n == 1 && status_movish)
            || (foe_mon.imprisoning && foe_mon.moves.iter().any(|m| m.entry.id == slot.entry.id))
    }

    /// Which move a mon is actually about to swing, whatever the player sent.
    /// `Side.chooseMove` throws the choice away and queues `getLockedMove()`
    /// in its place, and `runMove` applies an Encore's override above that —
    /// both of them ABOVE `runEvent('BeforeMove')`. So every gate that reads
    /// the move (the sleep gate asking `sleepUsable`, the freeze gate asking
    /// whether this one thaws its user, Disable naming the move it shut off)
    /// is answered about the lock and never about the slot that was picked.
    /// This mirrors the override that runs for real further down `use_move`,
    /// minus its bookkeeping.
    ///
    /// An id, not a slot, because a lock names a MOVE the mon need not know:
    /// `twoturnmove`'s `onLockMove` hands back the move that charged, so a
    /// Mirror Move that copied Fly releases Fly on a Steelix that never
    /// learnt it. Reading such a lock off the moveset finds nothing and falls
    /// back to the slot that was picked, which is the CALLER, so the gates
    /// below would answer about Mirror Move where the sim answers about Fly.
    pub(super) fn acting_move_id(&self, side: usize, index: usize) -> Option<&'static str> {
        let mon = self.sides[side].mon();
        let index = match (mon.encore_n > 0, mon.encored_slot) {
            (true, Some(i)) => i as usize,
            _ => index,
        };
        let releasing = mon.charging.is_some()
            || mon.rampage.is_some()
            || mon.bide.is_some()
            || mon.rolling.is_some();
        let index = match mon.charging {
            Some(i) => i as usize,
            None => match mon.rampage {
                Some((i, _)) => i as usize,
                None => index,
            },
        };
        if releasing {
            if let Some(id) = mon.locked_move {
                return Some(id);
            }
        }
        mon.moves.get(index).map(|m| m.entry.id)
    }

    pub fn over(&self) -> bool {
        self.sides[0].defeated() || self.sides[1].defeated()
    }

    pub(super) fn winner(&self) -> Option<u8> {
        match (self.sides[0].defeated(), self.sides[1].defeated()) {
            (true, true) => Some(0),
            (true, false) => Some(2),
            (false, true) => Some(1),
            _ => None,
        }
    }
}
