# Defects and corrective actions

Every defect found in the design, the docs, the parts supply or the checks themselves.
The rule: **no defect closes without a new or changed check** that would have caught it,
and that check passes on fresh evidence. A defect whose design is fixed but whose check
does not exist yet is FIXED, not CLOSED. `check_trace.py` refuses a CLOSED defect with no
check named, and the DVT and PVT gates refuse any defect that is not CLOSED.

Fields: Found (where, phase, which check or person), Symptom, Root cause, Why missed (why
the checks before it did not catch it), Check added or changed (VER ids), Updated (REQ and
DF rows), Status (OPEN, FIXED or CLOSED, then a note).

Seeded 2026-10-24 from the git log under `hardware/ref_design` (this branch, and three
commits on other branches where named) and from `docs/design_review.md` ("Found and fixed").

### DEF-01: USB ESD array's reference on VBUS back-fed VBUS
- Found: independent model audit, round 1, design phase before any order (4495595, 2026-09-25)
- Symptom: with no cable in, the MCU's D+ pull-up fed VBUS through U1's D+ steering diode to about 2.7 V; PWR_EN stayed up (the device could not turn off) and Q2 was left half on.
- Root cause: U1's REF pin tied to VBUS without tracing the paths back from the MCU's pins into an unpowered rail.
- Why missed: netlist, ERC and DRC check connectivity, not what flows where; no back-power path table existed (validation_plan #79).
- Check added or changed: none yet; VER-127 (back-power path table and an off-state test with a probe attached) is the check that would catch it.
- Updated: REQ-PWR-07, DF-98
- Status: FIXED (REF on 3V3); VER-127 not built

### DEF-02: R15 cut to 10k removed the Start contact's pill margin
- Found: model audit, round 2, design phase (d049b35, 2026-09-25)
- Symptom: round 1 lowered R15 (PWR_EN pull-down) from 100k to 10k against D5 leakage; that cut the pill resistance Q1 could still read by about five times.
- Root cause: a fix for one failure mode made without re-deriving the latch's other margins.
- Why missed: the latch's margins exist only in prose; no state table or script recomputes them on a change.
- Check added or changed: none yet; VER-33 (latch state table) and VER-78 (1 kohm in series on the real board).
- Updated: REQ-PWR-08, REQ-PWR-11, DF-27
- Status: FIXED (R15 back to 100k, R38 bleeder added); VER-33 and VER-78 not built

### DEF-03: The unplug hand-over took tens of ms, not microseconds
- Found: model audit, round 3, design phase (5095356, 2026-09-25)
- Symptom: C1 held VBUS up after an unplug and Q2's gate could only fall as fast as VBUS: Q2's body diode carried the system for tens of ms at the cell less 0.5 V. design_review.md said microseconds.
- Root cause: the hand-over was reasoned, not simulated, and the bleed path (R38 10k) was too weak.
- Why missed: no transient simulation of the power path at the time.
- Check added or changed: the ODE simulation behind design_review.md (not committed: VER-11); VER-70 on the real board.
- Updated: REQ-PWR-05, REQ-PWR-06, DF-24
- Status: FIXED (R38 10k to 1k); VER-11 not in the repo, VER-70 not run

### DEF-04: 3V3 fell under the flash's 2.97 V on most unplugs
- Found: Fable and Opus debate, design phase (e565907, 2026-09-26)
- Symptom: Q2's body diode drops about 0.6 V at 0.25 A; with the cell under about 3.7 V, 3V3 fell under QSPI_IOVDD's 2.97 V minimum on every unplug.
- Root cause: the body diode was counted on to carry the gap.
- Why missed: as DEF-03.
- Check added or changed: VER-11 (not committed), VER-70 (not run).
- Updated: REQ-PWR-05, DF-15, DF-34
- Status: FIXED (D7 B5819W across Q2); the 3.135 V question on VREG_AVDD and USB_OTP_VDD is still open (DF-34)

### DEF-05: Hot plug rang VSYS over the ME6211's 6.5 V
- Found: Fable and Opus debate, design phase (e565907)
- Symptom: a 2 m cable into a board with no cell peaked VBUS at 7.1 V and VSYS at 6.7 V (ME6211 maximum 6.5 V).
- Root cause: a ceramic input capacitor on a live USB plug-in; the TP4056 datasheet's own series resistor was missing.
- Why missed: no hot-plug simulation before the debate.
- Check added or changed: the hot-plug ODE (not committed: VER-12); VER-71 on the real board.
- Updated: REQ-PWR-02, DF-39
- Status: FIXED (R41 2.2 ohm in series with C1); VER-12 not in the repo

### DEF-06: VSYS rang over the FM8002A's 6.0 V, and bring-up still checks 6.5 V
- Found: validation checklist, design phase (4c4d846, 2026-10-05); the stale bring-up limit found while seeding this register (2026-10-24)
- Symptom: with no cell, VSYS peaked about 6.6 V against the FM8002A's 6.0 V maximum. R47 (0.1 ohm) brought it to about 5.7 V. bringup.md step 5.6 still passes VSYS up to 6.5 V, the ME6211's limit, not U7's.
- Root cause: the hot-plug limit was set from one part (U3) on a rail that feeds a lower-rated part (U7); the procedure was not updated with the fix.
- Why missed: no list of every part on each rail with its lowest absolute maximum; procedures are prose.
- Check added or changed: VER-71's pass limit is VSYS <= 5.8 V; VER-31 (ratings per rail) not built.
- Updated: REQ-PWR-02, REQ-PWR-14, DF-41
- Status: FIXED (R47); bringup.md 5.6 still says 6.5 V (also in DEF-21)

### DEF-07: J6 TX and RX swapped against the Debug Probe
- Found: datasheet pin review (check_pins.py, pin_review/j6.json), design phase (4c4d846, 2026-10-05)
- Symptom: GPIO20 (UART1 TX) was on J6 pin 1, where the Debug Probe's TX also drives: contention, no link.
- Root cause: the connector spec names pins from the target's side; design.py named them from the probe's.
- Why missed: ERC, netlist and DRC trust design.py's pin numbers.
- Check added or changed: VER-02 (check_pins.py fails when a reviewed pin's net changes).
- Updated: REQ-MCU-04, DF-63
- Status: CLOSED (VER-02 PASS 2026-10-24). Note: j6.json's issue text still describes the old mismatch (DEF-21).

### DEF-08: USB series resistors were 22 ohm; the RP2350 requires 27
- Found: datasheet pin review (check_pins.py, pin_review/u4.json pins 66 and 67), design phase (4c4d846)
- Symptom: R21/R22 were 22 ohm; the RP2350 datasheet (Table 2) requires 27 ohm.
- Root cause: a Basic part chosen for cost, reasoned as "fine for full speed" without the datasheet.
- Why missed: values were never checked against a datasheet requirement before the pin review.
- Check added or changed: VER-02 (passives.json checks each resistor's value and LCSC number).
- Updated: REQ-MCU-02
- Status: CLOSED (VER-02 PASS 2026-10-24). Note: design_review.md, pinmap.md and design.py's comment above R21 still say 22 ohm (DEF-21).

### DEF-09: The ME6211's CE could rise above its VIN at a live plug-in
- Found: datasheet pin review (pin_review/u3.json), design phase (4c4d846)
- Symptom: PWR_EN rises from VBUS through R14 within microseconds while VSYS lags behind R47/D5 into C9: CE could exceed VIN + 0.3 V, its absolute maximum.
- Root cause: two paths from VBUS with different time constants into one part's pins.
- Why missed: no transient check of pin-to-pin limits.
- Check added or changed: VER-72 (scope PWR_EN - VSYS at plug-in; not run).
- Updated: REQ-PWR-03, DF-30
- Status: FIXED (C47 100 nF slows PWR_EN to about 1 ms); VER-72 not run

### DEF-10: PWR_HOLD had no external pull-down (RP2350-E9)
- Found: validation plan (#178), design phase (2026-10-05; fixed in 4c4d846)
- Symptom: after a crash the released PWR_HOLD pad can sit near 2 V (E9 on A2 silicon); through D2 that kept the ME6211 on, so a crashed unit stayed on and drained its cell.
- Root cause: the reset state of a GPIO assumed, not read from the errata.
- Why missed: no errata table; the pin review found E9 only for the ADC pins.
- Check added or changed: VER-02 checks R42's value; VER-77 (release on real silicon; not built); VER-16 (errata table; not built).
- Updated: REQ-PWR-09, REQ-MCU-07, DF-25
- Status: FIXED (R42 4.7k); VER-77 and VER-16 not built

### DEF-11: Charge current too high for the 403450
- Found: validation plan (#28, #128), design phase (fixed in 4c4d846, 2026-10-05)
- Symptom: R5 2.4k set 460-500 mA, about 0.7 C for a 700 mAh cell.
- Root cause: R5 kept from the 503450 (1000 mAh) when the cell changed.
- Why missed: no requirement tied the charge current to the cell; the cell's spec sheet was never obtained.
- Check added or changed: VER-37 (cell spec sheet; not run); VER-66 (charge current per unit; not run).
- Updated: REQ-BAT-02, DF-07
- Status: FIXED (R5 3.3k, 333-364 mA); the cell's own limit is still unknown

### DEF-12: AP3156 backlight charge pump out of stock
- Found: stock check before ordering (b7a6238, 2026-10-07)
- Symptom: the AP3156 (C443652) had 0 stock at JLC and LCSC.
- Root cause: a single-source Extended part with low stock and no stock watch.
- Why missed: stock was last checked on a July 2026 snapshot.
- Check added or changed: VER-41 (order-day stock and pre-orders; not run); VER-14 (LCSC identity and stock class; not built).
- Updated: REQ-FAB-10, DF-69
- Status: FIXED (two discrete current sinks, all Basic or Preferred, simulated first: VER-05)

### DEF-13: ProTek SRV05-4 out of stock
- Found: stock check (eea0ee8, 2026-10-07)
- Symptom: JLC had 2 of C85364 left.
- Root cause: as DEF-12.
- Why missed: as DEF-12.
- Check added or changed: VER-41; VER-14.
- Updated: REQ-FAB-10, DF-69
- Status: FIXED (Semtech SRV05-4.TCT, C13612)

### DEF-14: U1's JLC footprint may be turned 180 degrees from KiCad's
- Found: on another branch, 1fd0956 (2026-10-22): "JLC's footprint for C13612 is KiCad's SOT-23-6 turned 180 degrees". This branch's eea0ee8 (2026-10-07) says the opposite: "the same JLC library footprint ... so the CPL rotation is unchanged".
- Symptom: if 1fd0956 is right, a CPL with KiCad's angle places U1 reversed: 3V3 on its ground reference, its TVS forward biased, 3V3 shorted.
- Root cause: rotation offsets between KiCad and JLC footprints are checked by eye in JLC's preview, and only on order day.
- Why missed: there is no rotation table and no dry run has been done.
- Check added or changed: VER-29 (rotation table; not built); VER-28 (dry run with screenshots; not run).
- Updated: REQ-FAB-07, DF-67, DF-68
- Status: OPEN (the two records disagree; settle it before the order)

### DEF-15: First PLA print: hooks snapped, lid tight, pins bound
- Found: first full print of the case (88eec7c, 2026-10-05)
- Symptom: the 1.9 mm FDM pins bound and snapped; the snap hooks snapped; the lid was tight at the deck steps and felt flimsy.
- Root cause: FDM layer strength and fit tolerances not designed for; no coupon before the full print.
- Why missed: check_fit checks heights, not snap strain or print tolerance.
- Check added or changed: none that runs; VER-48 (current shell with real parts) and VER-92 (open/close cycles) not run.
- Updated: REQ-MEC-05, DF-89
- Status: FIXED (pins removed, hooks tapered, 0.3 mm at the steps, lid 2.0 mm, PETG recommended)

### DEF-16: D-pad opening too tight for Alex's AliExpress D-pad
- Found: fit print with real caps (7606442, 2026-10-19)
- Symptom: the D-pad bound in its opening.
- Root cause: cap sizes from open models of genuine parts; the aftermarket D-pad is bigger.
- Why missed: no measured cap dimensions per lot.
- Check added or changed: the fit kit's tune_openings coupon; VER-126 (incoming caps per lot; not run).
- Updated: REQ-MEC-03
- Status: FIXED (gap 0.5 to 0.8 mm)

### DEF-17: Deck insert would not go on over the silicone
- Found: insert print (73c3686, 2026-10-19)
- Symptom: round each cluster the deck came down as a solid disc to 0.35 mm over the board, and the flanges stopped 1.75 mm up.
- Root cause: the silicone's height estimated, the shell's rib structure not modelled.
- Why missed: no clash check between silicone, caps and shell.
- Check added or changed: VER-07 (check_buttons.py, added 269dc87, shown to fail on a cap that does not fit its slot).
- Updated: REQ-MEC-03
- Status: CLOSED (VER-07 PASS at 269dc87, fresh)

### DEF-18: Start/Select pad's square wells hit round flange pockets
- Found: silicone pad model (86e81e2, 2026-10-19)
- Symptom: the pad's square wells collided with the round pockets in the lid.
- Root cause: pocket shape drawn before the pad was modelled.
- Why missed: as DEF-17.
- Check added or changed: VER-07 (caps_silicone and silicone_shell checks).
- Updated: REQ-MEC-03
- Status: CLOSED (VER-07 PASS at 269dc87)

### DEF-19: Holes through the lid
- Found: Alex, on the live 3D view (177123c, 2026-10-19)
- Symptom: the panel pocket kept the frame's 6 mm FPC notch, which the step's S-curve cut open; the six 6 mm screw pilots came through the screen face.
- Root cause: features sized before the screen face dropped to 5.8 mm over the board.
- Why missed: no check for unintended openings.
- Check added or changed: none on this branch; VER-113 (openings check; not built). Another branch has check_manifold.py.
- Updated: REQ-MEC-08, DF-90
- Status: FIXED (pilots stop 0.8 mm under the face; notch only in the frame)

### DEF-20: J6's mouth faced J5's back
- Found: model audit, round 1 (4495595)
- Symptom: J6's mouth faced J5's back 5 mm away: no room for the cable.
- Root cause: connector access not checked in placement.
- Why missed: no connector-access keep-out.
- Check added or changed: none.
- Updated: REQ-MCU-04
- Status: FIXED (J6 turned 180 degrees); no check

### DEF-21: Docs disagree with the design
- Found: while seeding this register (2026-10-24, VER-39); some of these are fixed on other branches (67e3d13), not here
- Symptom: (a) bringup.md 6.2-6.3 and pinmap.md "Power control" call seat 1's Y the power button (Start since 2026-10-19). (b) design_review.md, pinmap.md and design.py's comment above R21/R22 say 22 ohm; the parts are 27 ohm. (c) bringup.md step 0 expects TP1 above about 1 kohm (R38 1k in parallel with the TEMP divider and the D5/D6 paths reads below that) and one body-diode drop from TP3 to TP2 (D7 in parallel reads 0.15-0.35 V). (d) bringup.md 5.6 passes VSYS to 6.5 V (U7's limit is 6.0 V). (e) README says the caps stand 1.3 mm proud; case.scad has cap_proud = 1.85. (f) assembly.md's polarised-part list leaves out D7. (g) assembly.md 3.3 says "JP1 and JP2 come bridged"; there are five jumpers. (h) pin_review/panel.json pin 10 names a "C44 DNP spare" that design.py does not have; j6.json and u4.json (pins 66, 67) still describe the pre-fix mismatches. (i) design_review.md "Checks run" says 157 parts and 124 nets; today's netlist has 173 parts and 127 nets (VER-01). (j) PLAN.md says the TP4056 runs at 500 mA (it is 333-364 mA). (k) The cell plug's hole: PLAN.md 5.5 x 4.5 mm, design_review.md a 5 x 2.3 mm slot, assembly.md 7.2 x 5.7 mm. (l) datasheets/SOURCES.md points me6211.pdf at C235316, not the placed C82942.
- Root cause: docs repeat numbers from design.py and case.scad by hand; nothing compares them.
- Why missed: no doc check; reviews read the docs as truth.
- Check added or changed: VER-39 (stale-doc sweep, FAIL 2026-10-24); a script that pulls the numbers from design.py and case.scad is the real fix (not built).
- Updated: REQ-TST-01, DF-14, DF-41
- Status: OPEN

### DEF-22: The validation plan's statuses went stale
- Found: task brief and this register (2026-10-24)
- Symptom: since 2026-10-07 R42, Q4/R43/R44, JP1-JP5, TP6-TP16, C41-C43 and the revision strap landed in design.py while validation_plan.md still marked those items "no".
- Root cause: statuses kept by hand in a 218-item prose list.
- Why missed: nothing reads the statuses back.
- Check added or changed: vp_map.csv now holds the statuses with evidence; check_trace.py fails when vp_map disagrees with validation_plan.md's titles or stated statuses. It cannot tell that a status is wrong.
- Updated: none
- Status: FIXED (statuses moved to vp_map.csv); no check proves a status right

### DEF-23: JST PH J2 too tall for the lowered lid
- Found: print preparation (f231998, 2026-10-19)
- Symptom: the 5.5 mm JST PH needed a 0.5 mm pad on the lowered lid, which lifted the screen face off the bed when printed face down.
- Root cause: connector height chosen before the lid came down.
- Why missed: check_fit allowed J2 into a lid pocket by rule.
- Check added or changed: VER-06 (check_fit, J2 pocket rule); now stale.
- Updated: REQ-MEC-01
- Status: FIXED (Molex PicoBlade, 3.4 mm); VER-06 stale

### DEF-24: Audio numbers in the docs were wrong
- Found: review panel of seven model families, checked by Fable (7768a97, 2026-09-25)
- Symptom: the docs claimed two 16 kHz poles (really -3 dB at 6.6 kHz loaded), 1 W full scale (really about 0.55 W on USB) and ignored the FM8002A's 100 ms wake.
- Root cause: numbers from unloaded filter formulas and the amplifier's headline rating.
- Why missed: no audio simulation committed.
- Check added or changed: VER-100 (audio chain simulation; not in the repo); VER-64 on the real board.
- Updated: REQ-AUD-01, REQ-AUD-03, REQ-AUD-05
- Status: FIXED (docs corrected; C37/C38 to 4.7 nF and 8-bit PWM in ad291a8)

### DEF-25: Lid window too tight for the panel's float and tolerance
- Found: model audits, rounds 1-3 (4495595, d049b35, 5095356)
- Symptom: the panel's locating frame had 0.4 mm clearance and the window 0.5 mm margin, less than the panel's +-0.4 mm float, its +-0.3 mm active-area tolerance and print error.
- Root cause: nominal fits without a tolerance stack.
- Why missed: no XY or Z stack-up analysis.
- Check added or changed: check_fit learned the lid's panel frame (d049b35): VER-06 (stale); VER-20 (stack; not built).
- Updated: REQ-MEC-02, REQ-PRD-06
- Status: FIXED (frame 0.8 mm, window 1.0 mm)

### DEF-26: The datasheet gate passes with open questions in it
- Found: while seeding this register (2026-10-24)
- Symptom: check_pins.py passes while the pin reviews hold open questions and unknown ratings: VREG_AVDD and USB_OTP_VDD's 3.135 V minimum against the 2.97 V unplug target (u4.json pins 61, 68), the missing second 4.7 uF on +1V1 (pin 10), the unknown 3V3 budget and CE transient (u3.json), D5's surge (d5.json), R47's surge (r47.json), the TP4056's BAT absolute maximum (u2.json).
- Root cause: check_pins.py fails only on a pin with ok false; it never reads the ratings, and "OPEN QUESTION" text in an ok true entry is invisible to it.
- Why missed: the gate was written to catch net changes, not unresolved findings.
- Check added or changed: VER-36 (fail on ok null ratings and open-question text; not built).
- Updated: REQ-FAB-02, REQ-PWR-05, REQ-PWR-15, REQ-MCU-09, DF-31, DF-34, DF-94
- Status: OPEN

### DEF-27: check_fit guesses part heights silently
- Found: validation plan #96 (2026-10-05); still true at 76955d2
- Symptom: KNOWN_HEIGHT is empty, PACKAGE_HEIGHT is a guess table, and an unknown package gets 1.5 mm without a warning.
- Root cause: the fit check was written for speed with box models.
- Why missed: nobody seeded a fault to see whether check_fit can fail on a tall part.
- Check added or changed: VER-112 (heights from drawings; unknown package fails; not built); VER-22 (seeded faults; not built).
- Updated: REQ-MEC-01, DF-91
- Status: OPEN

### DEF-28: Design analyses are not in the repo
- Found: while seeding this register (2026-10-24)
- Symptom: the unplug and hot-plug ODEs, the copper-resistance solves and the audio filter run are quoted in design_review.md and validation_plan.md, but no script for them is committed, so nobody can re-run them after a change.
- Root cause: analyses run in scratch sessions; only their conclusions were kept.
- Why missed: no rule that a quoted number needs its script in the repo.
- Check added or changed: VER-11, VER-12, VER-13, VER-100 carry them as NOT_BUILT; the README's evidence rule.
- Updated: REQ-PWR-02, REQ-PWR-05, REQ-AUD-03
- Status: OPEN

### DEF-29: RP2354B and L2 out of stock at JLC and LCSC
- Found: BOM audit on another branch (67e3d13, 2026-10-21): "RP2354B and L2 out at JLC and LCSC, U1 3 left, J5/J6 2 left, U3 724". This branch's bom.md still shows 1665 RP2354B in stock (2026-10-07).
- Symptom: the order cannot be assembled as designed; JLC may offer substitutes.
- Root cause: low-stock single-source parts; the core inductor must be the Abracon part the RP2350 guide names.
- Why missed: as DEF-12.
- Check added or changed: VER-41 (order-day stock, pre-orders, no substitution; not run).
- Updated: REQ-FAB-10, DF-69, DF-93
- Status: OPEN (pre-orders pending per Alex's notes)

### DEF-30: Which design is the order candidate is not recorded
- Found: while seeding this register (2026-10-24)
- Symptom: this branch (vv-plan at 76955d2, the same as local main) is REF-A with the TP4056, ME6211 and the 3.5in ER-TFT035IPS-6. origin/main (56933a5) has moved on: BQ24073 charger, RT9013 LDO, 2.4in ER-TFT024IPS-3 panel, 74HC165 button shift registers, a plug-in speaker. Every check and requirement here describes 76955d2.
- Root cause: no release record names the commit being ordered.
- Why missed: no configuration control before this register.
- Check added or changed: VER-42 (release record: tag, commit, fab zip hash; not run); check_trace.py marks evidence stale when HEAD moves past changes it covers, and refuses evidence from a commit that is not an ancestor of HEAD.
- Updated: REQ-FAB-08, DF-76
- Status: OPEN (Alex to name the order candidate)
