# Defects and corrective actions

Every defect found in the design, the docs, the parts supply or the checks themselves.
The rule: **no defect closes without a new or changed check** that would have caught it,
and that check passes on fresh evidence. A defect whose design is fixed but whose check
does not exist yet is FIXED, not CLOSED. `check_trace.py` refuses a CLOSED defect with no
check named, and the DVT and PVT gates refuse any defect that is not CLOSED.

Fields: Found (where, phase, which check or person), Symptom, Root cause, Why missed (why
the checks before it did not catch it), Check added or changed (VER ids), Updated (REQ and
DF rows), Status (OPEN, FIXED or CLOSED, then a note), and optionally Board (ref_design when
absent). A CLOSED defect's check must pass on fresh evidence, or check_trace fails.

Per-unit findings on built boards (a bridge, a tombstone, a reversed cell) are not design
defects: they go in hardware/vv/evidence/findings.csv (VER-110). A finding seen on 2 units
becomes a DEF here.

Seeded 2026-10-24 from the git log under `hardware/ref_design` (this branch, and three
commits on other branches where named) and from `docs/design_review.md` ("Found and fixed").

### DEF-01: USB ESD array's reference on VBUS back-fed VBUS
- Found: independent model audit, round 1, design phase before any order (4495595, 2026-09-25)
- Symptom: with no cable in, the MCU's D+ pull-up fed VBUS through U1's D+ steering diode to about 2.7 V; PWR_EN stayed up (the device could not turn off) and Q2 was left half on.
- Root cause: U1's REF pin tied to VBUS without tracing the paths back from the MCU's pins into an unpowered rail.
- Why missed: netlist, ERC and DRC check connectivity, not what flows where; no back-power path table existed (validation_plan #79).
- Check added or changed: VER-159 (check_backpower.py over design.nets(), at the order gate, seeded with this very fault: U1 pin 5 back on VBUS must fail it); VER-127 (the off test with the Debug Probe attached, unit 1).
- Updated: REQ-PWR-07, DF-98
- Status: FIXED (REF on 3V3); VER-159 not built

### DEF-02: R15 cut to 10k removed the Start contact's pill margin
- Found: model audit, round 2, design phase (d049b35, 2026-09-25)
- Symptom: round 1 lowered R15 (PWR_EN pull-down) from 100k to 10k against D5 leakage; that cut the pill resistance Q1 could still read by about five times.
- Root cause: a fix for one failure mode made without re-deriving the latch's other margins.
- Why missed: the latch's margins exist only in prose; no state table or script recomputes them on a change.
- Check added or changed: none yet; VER-33 (latch state table) and VER-78 (10 kohm in series on the real board, the margin REQ-BTN-01 allows; 1 kohm proved nothing: F-10).
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
- Check added or changed: VER-71's pass limit is now VSYS <= 5.7 V and VBUS <= 7.7 V (REQ-PWR-02 from REQ-PWR-14's transient rule, F-8); VER-31 (ratings per rail) and VER-36's lowest-maximum-per-rail table (not built); VER-161 checks bringup.md's limits against that table.
- Updated: REQ-PWR-02, REQ-PWR-14, DF-41
- Status: FIXED (R47); bringup.md 5.6 still says 6.5 V and must say 5.7 V (also in DEF-21)

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
- Check added or changed: VER-02 (passives.json checks each resistor's value and LCSC number). The stale comments are DEF-21's.
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
- Check added or changed: VER-37 (cell spec sheet; not run); VER-160 (cell.json and an assert that recomputes the limit when the cell or R5 changes; not built: it would have failed at 950afcf and 226bbb6); VER-66 (charge current per unit; not run).
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
- Found: on another branch, 1fd0956 (2026-10-22): "JLC's footprint for C13612 is KiCad's SOT-23-6 turned 180 degrees". This branch's eea0ee8 (2026-10-07) says "the same JLC library footprint ... so the CPL rotation is unchanged". The review (F-19) reads both as true: eea0ee8 means unchanged against the ProTek C85364 it replaced, which shared JLC's footprint, and JLC's SOT-23-6 zero differs from KiCad's by 180 degrees. So the CPL with KiCad's angle places U1 reversed unless corrected.
- Symptom: if 1fd0956 is right, a CPL with KiCad's angle places U1 reversed: 3V3 on its ground reference, its TVS forward biased, 3V3 shorted.
- Root cause: rotation offsets between KiCad and JLC footprints are checked by eye in JLC's preview, and only on order day.
- Why missed: there is no rotation table and no dry run has been done.
- Check added or changed: VER-29 (rotation table from the kicad-jlcpcb-tools plugin's corrections, applied to fab.py's CPL; not built); VER-28 (dry run with screenshots; not run).
- Updated: REQ-FAB-07.R, DF-67, DF-68
- Status: OPEN (FIXED when VER-29's table corrects the CPL; CLOSED on VER-28's screenshot of U1)

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
- Status: FIXED (it was CLOSED on VER-07's PASS at 269dc87, which rests on a commit message; it closes again when VER-07 passes with saved evidence)

### DEF-18: Start/Select pad's square wells hit round flange pockets
- Found: silicone pad model (86e81e2, 2026-10-19)
- Symptom: the pad's square wells collided with the round pockets in the lid.
- Root cause: pocket shape drawn before the pad was modelled.
- Why missed: as DEF-17.
- Check added or changed: VER-07 (caps_silicone and silicone_shell checks).
- Updated: REQ-MEC-03
- Status: FIXED (as DEF-17: closes again when VER-07 passes with saved evidence)

### DEF-19: Holes through the lid
- Found: Alex, on the live 3D view (177123c, 2026-10-19)
- Symptom: the panel pocket kept the frame's 6 mm FPC notch, which the step's S-curve cut open; the six 6 mm screw pilots came through the screen face.
- Root cause: features sized before the screen face dropped to 5.8 mm over the board.
- Why missed: no check for unintended openings.
- Check added or changed: none on this branch; VER-113 (openings check; not built). Another branch has check_manifold.py. The same class (the stack moved and a fastener was not recomputed) is DEF-31.
- Updated: REQ-MEC-08, DF-90
- Status: FIXED (pilots stop 0.8 mm under the face; notch only in the frame)

### DEF-20: J6's mouth faced J5's back
- Found: model audit, round 1 (4495595)
- Symptom: J6's mouth faced J5's back 5 mm away: no room for the cable.
- Root cause: connector access not checked in placement.
- Why missed: no connector-access keep-out.
- Check added or changed: VER-165 (connector mouth keep-outs in check_fit; not built).
- Updated: REQ-MCU-04, REQ-MEC-11
- Status: FIXED (J6 turned 180 degrees); VER-165 not built

### DEF-21: Docs disagree with the design
- Found: while seeding this register (2026-10-24, VER-39); some of these are fixed on other branches (67e3d13), not here
- Symptom: (a) bringup.md 6.2-6.3 and pinmap.md "Power control" call seat 1's Y the power button (Start since 2026-10-19). (b) design_review.md, pinmap.md and design.py's comment above R21/R22 say 22 ohm; the parts are 27 ohm. (c) bringup.md step 0 expects TP1 above about 1 kohm (R38 1k in parallel with the TEMP divider and the D5/D6 paths reads below that) and one body-diode drop from TP3 to TP2 (D7 in parallel reads 0.15-0.35 V). (d) bringup.md 5.6 passes VSYS to 6.5 V (U7's limit is 6.0 V). (e) README says the caps stand 1.3 mm proud; case.scad has cap_proud = 1.85. (f) assembly.md's polarised-part list leaves out D7. (g) assembly.md 3.3 says "JP1 and JP2 come bridged"; there are five jumpers. (h) pin_review/panel.json pin 10 names a "C44 DNP spare" that design.py does not have; j6.json and u4.json (pins 66, 67) still describe the pre-fix mismatches. (i) design_review.md "Checks run" says 157 parts and 124 nets; today's netlist has 173 parts and 127 nets (VER-01). (j) PLAN.md says the TP4056 runs at 500 mA (it is 333-364 mA). (k) The cell plug's hole: PLAN.md 5.5 x 4.5 mm, design_review.md a 5 x 2.3 mm slot, assembly.md 7.2 x 5.7 mm. (l) datasheets/SOURCES.md points me6211.pdf at C235316, not the placed C82942. Found in the two-model review (2026-10-24): (m) design.py lines 349-350 still say "22 ohm ... fine for full-speed USB" above R21/R22 = 27 (part of b). (n) bom.md buys M2 x 8 screws (DEF-31). (o) bom.md says the Start/Select strip is not used (DEF-42). (p) pinmap.md calibrates the VBAT gain on the first CHG_DONE_N edge (DEF-40). (q) assembly.md mates the cell before the FPC and solders the speaker with the cell in the tub (DEF-41). (r) bringup.md step 1 has no no-cell charge-path screen (VER-133) and step 5's cell-emulation steps cannot hold 3.50 V with USB in (DEF-46).
- Root cause: docs repeat numbers from design.py and case.scad by hand; nothing compares them.
- Why missed: no doc check; reviews read the docs as truth.
- Check added or changed: VER-39 (stale-doc sweep, FAIL 2026-10-24); VER-161 (check_claims.py: the numbers and lists compared with design.py, case.scad and pin_review; not built) is the real fix.
- Updated: REQ-TST-01, DF-14, DF-41
- Status: OPEN

### DEF-22: The validation plan's statuses went stale
- Found: task brief and this register (2026-10-24)
- Symptom: since 2026-10-07 R42, Q4/R43/R44, JP1-JP5, TP6-TP16, C41-C43 and the revision strap landed in design.py while validation_plan.md still marked those items "no".
- Root cause: statuses kept by hand in a 218-item prose list.
- Why missed: nothing reads the statuses back.
- Check added or changed: vp_map.csv holds the statuses with evidence as a one-time mapping; validation_plan.md is archived and check_trace no longer compares the two (O-11), so editing the old plan can no longer break the register. No check proves a status right; the live statuses are the VER rows.
- Updated: none
- Status: FIXED (plan archived; statuses in vp_map.csv and verification.csv)

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
- Check added or changed: VER-36 (read both rating schemas, what/limit and param/ds; fail on ok false or null without a waiver, on open-question text, on a passive rated under 1.25 x its net; not built). Its acceptance test: it must fail today on 9 entries, panel.json's VCI and VDDI (ok false) among them.
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
- Check added or changed: VER-11, VER-12, VER-13, VER-100 carry them as NOT_BUILT; the README's evidence rule, now enforced: a method A PASS needs a saved output under evidence/ whose header names its commit, tools and result. The backlight sim that is in the repo hard-codes its values (DEF-45).
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
- Check added or changed: VER-42 (release record: tag, commit, fab zip hash; not run); check_trace.py's rule R7 fails any gate when HEAD's design files differ from README's design baseline ("gate evaluated against a design it does not describe", F-1); evidence goes stale when HEAD moves past changes it covers, or its commit is not an ancestor of HEAD.
- Updated: REQ-FAB-08, DF-76
- Status: OPEN (Alex to name the order candidate)

### DEF-31: The case screws engage 1.4 mm of the lid's pilots
- Found: two-model review (O-52), 2026-10-24, from case.scad
- Symptom: the counterbore puts the head's face at z = 1.0 mm and the board top is at 7.6 mm, so an M2 x 8 ends at 9.0 mm: about 3.5 turns in the lid's 1.6 mm pilot out of the 5.0 mm it offers. Self-tapping screws in printed PETG want 4-5 mm.
- Root cause: the stack changed and the fastener and pilot were not recomputed (DEF-19's class).
- Why missed: no assert ties the screw length to the stack; bom.md's "M2 x 8" is prose.
- Check added or changed: VER-147 (screw asserts; not built); VER-92 counts stripped bosses after 20 cycles at the chosen length.
- Updated: REQ-MEC-09, DF-119
- Status: OPEN (M2 x 11 engages 4.4 mm and ends 0.6 mm short of the pilot's end)

### DEF-32: The drill file mixes plated and unplated holes
- Found: two-model review (O-14), 2026-10-24, from a scratch export with fab.py's own kicad-cli arguments
- Symptom: one battler.drl, 'TF.FileFunction,MixedPlating', holds the 0.30 mm vias and pad holes, J1's 0.6 mm plated slots, J1's two 0.65 mm NPTH pegs and the 2.2 mm H1-H6 together. JLC's KiCad guide asks for separate files; a CAM that plates by file shrinks the pegs to about 0.55 mm and J1 rides up off its pads.
- Root cause: kicad-cli's default drill export; nobody read the drill file.
- Why missed: VER-27 (the one look at the gerbers and drills) was gated nowhere (O-1).
- Check added or changed: VER-131 (--excellon-separate-th and a parse of both files; not built); G-ORD-22 now gates every ORDER check.
- Updated: REQ-FAB-04, DF-127
- Status: OPEN

### DEF-33: The CPL uses footprint origins, and J1's and J2's are off their pads
- Found: two-model review (O-16), 2026-10-24, measured with pcbnew
- Symptom: every assembled footprint's origin sits at its pads' centre except J1 (TYPE-C-31-M-12, 1.46 mm along its depth) and J2 (PicoBlade, 0.60 mm). fab.py writes the origin; JLC places at its own footprint's centroid.
- Root cause: footprints whose origin is not the centroid, and a CPL that assumes it is.
- Why missed: no centroid check; the dry run has not been done.
- Check added or changed: VER-29 (the centroid table beside the rotation table; not built); VER-28 screenshots J1 and J2.
- Updated: REQ-FAB-07.R, DF-128
- Status: OPEN

### DEF-34: No firmware can run on the ordered board
- Found: two-model review (O-56, F-44), 2026-10-24
- Symptom: the fw crate targets the RP2350A (rp235xa, GPIO0-29), 4096K of flash and the ST7796; the only ref-board firmware (fw-ref-board a01f6fb) uses origin/main's pin map: the charger on GPIO41/42 (P1_B and P1_X here), a 74HC165 on GPIO28-30 (P1's cross combs here), no CHG_OFF. Every pin number is typed by hand.
- Root cause: firmware written against another branch's design, with no generated pin file.
- Why missed: the generated pin definitions (vp_map #56) were scheduled for EVT, so the first build for the real chip would be the day the boards arrive.
- Check added or changed: VER-149 (generated pin file and a host test), VER-150 (an rp235xb build at 2048K), VER-151 (flash budget); all at the order gate (G-ORD-23); not built.
- Updated: REQ-MCU-10, REQ-MCU-11, DF-121, DF-122
- Status: OPEN

### DEF-35: The trace gate could pass on evidence that did not show what it claimed
- Board: both
- Found: two-model review (O-1, O-2, O-3, O-4, O-5, O-26, F-2, F-3, F-5, F-13), 2026-10-24
- Symptom: eight ORDER checks (VER-13, VER-19, VER-21, VER-22, VER-27, VER-34, VER-126, VER-127) were named by no order criterion, requirement or H row, so the order gate could pass without them. An H row was "covered" by any one passing check in its detection text, even one that cannot see its failure (DF-70 by VER-02, DF-26 by VER-104). An acceptance for EVT held for ever. PASS was typed by hand, twice resting on a commit message (VER-06, VER-07). A covers regex kept its quotes and so never matched (VER-43's "(PANEL|fpc|J3)"), and an unquoted one over-matched (VER-37's R5 on R52-R61); loop-made parts (R52/R53) never staled VER-05. A check on external state (stock) never went stale. DF-87 reached L with an occurrence of 1.
- Root cause: the gate checked that IDs existed, not that the evidence could show what was claimed.
- Why missed: the seeded-fault run that tested check_trace was never committed (DEF-28's rule).
- Check added or changed: VER-129 (tools/seed_faults.py: each of these faults seeded into a copy of the register must fail check_trace).
- Updated: README rules; gates.md G-ORD-22; dfmea.csv 'detects'; verification.csv valid_days
- Status: CLOSED (check_trace.py rewritten; VER-129 PASS 2026-10-24 at e7a26e8: 29 seeded faults caught)

### DEF-36: check_pcb.py passes every DRC warning that is not silk
- Found: two-model review (O-12, F-27), 2026-10-24
- Symptom: battler.kicad_pro sets hole_to_hole, holes_co_located, copper_sliver, isolated_copper, connection_width, padstack, lib_footprint_mismatch, lib_footprint_issues, footprint_symbol_field_mismatch, net_conflict, missing_footprint, extra_footprint and duplicate_footprints to warning; check_pcb.py fails only on errors, while VER-04's pass limit says "silkscreen warnings only". Today's evidence happens to list silk warnings only.
- Root cause: the script's rule is weaker than the limit it is credited with.
- Why missed: no seeded fault for check_pcb.
- Check added or changed: VER-04 (fail on any type not in COSMETIC), VER-22 (a seeded hole_to_hole fault); not built.
- Updated: vp_map #19
- Status: OPEN

### DEF-37: TH1 sits at the cell's edge, and four bare pads face the pouch
- Found: two-model review (O-30, F-36), 2026-10-24, from placement.py and case.scad
- Symptom: TH1 is at (85.2, 124.0), 2.2 mm inside the cell's long edge and about 15 mm from its centre (100, 117), on the GND pour U2's EP vias heat, behind 1.6 mm of FR4 and a 0.45 mm foam pad that alone is about twice the board's thermal resistance. DF-04 said "TH1 over the cell's centre". TP11-TP14 are bare 1.0 mm pads on the back inside the cell's rectangle.
- Root cause: the cell moved under the board (2026-10-09) and nothing checks placement against the cell's rectangle.
- Why missed: check_fit never reads positions.json's cell and skips test points on its back-side pass.
- Check added or changed: VER-164 (cell rectangle rule and a TH1 lag estimate; not built); VER-87 (two thermocouples, centre and edge).
- Updated: REQ-THM-02, REQ-BAT-08, DF-04
- Status: OPEN (copper decision before the order: a B.Cu patch under TH1 with vias, or a silicone thermal pad for the foam)

### DEF-38: C9 sits 0.9 mm from a screw boss's edge
- Found: two-model review (O-43), 2026-10-24, measured with pcbnew
- Symptom: C9 (22 uF on VSYS, the S9 flex-crack row) at (131.2, 131.4) is about 0.9 mm from the edge of the boss that clamps H3 between two printed bosses; no other MLCC is within 8 mm of a hole. The screws go in after the cell is plugged in, so a crack shorts the cell through Q2 inside the closed case.
- Root cause: no keep-out between MLCCs and the case's clamping bosses.
- Why missed: DF-38's detection (VER-52) runs before the board is screwed in.
- Check added or changed: VER-142 (MLCC keep-out; not built); VER-92 (C9 at 40x and TP2 after 20 screw cycles).
- Updated: DF-38
- Status: OPEN

### DEF-39: Backlight at 100% exceeds the panel's 120 mA at worst-case tolerance
- Found: two-model review (F-7, O-21), 2026-10-24, by hand
- Symptom: 3V3 +2%, R61 +1% and R60 -1%, U6's +4.5 mV offset and R52/R53 -1% give about 61.2 mA a pin, 122.5 mA in total. REQ-DSP-05's "+-5%" is below the parts' spread.
- Root cause: the requirement and the sim used nominal parts.
- Why missed: no tolerance sweep; the sim hard-codes its values (DEF-45).
- Check added or changed: VER-137 (backlight_tol.py; not built); VER-62 at the firmware's maximum duty.
- Updated: REQ-DSP-05, REQ-DSP-07, REQ-DSP-12, DF-130
- Status: OPEN (fix with no copper: a 95% duty cap in firmware, or R61 3.6k)

### DEF-40: After calibration, the 3.50 V cutoff may never fire
- Found: two-model review (O-49, O-23, O-60), 2026-10-24, by hand
- Symptom: the ADC's reference is 3V3. On a unit with a +2% LDO, a -1% CV and 0.14 V of dropout, the calibrated reading rises from 3.54 V at the knee to 3.57 V at a true 3.0 V: the cutoff never fires, and 3V3 falls under QSPI_IOVDD's 2.97 V at about a 3.11 V cell. The gain is also calibrated on the first CHG_DONE_N edge, which flickers with no cell, so a gain off by about 4% can be stored. The cutoff has no tolerance, though the unplug margin depends on it to 50 mV.
- Root cause: a ratio measurement treated as absolute; a calibration point chosen for convenience.
- Why missed: VER-67's limit applies above 3.6 V and its sweep runs with USB in, which keeps 3V3 regulated.
- Check added or changed: VER-138 (cutoff analysis), VER-139 (bench sweep without USB), VER-155 (bench calibration at 3.50 V), VER-156 (no gain from a no-cell edge); not built.
- Updated: REQ-BAT-12, REQ-TST-06, DF-116, DF-117
- Status: OPEN (decide before the order: a hardware reference needs copper)

### DEF-41: Assembly works next to a live cell
- Found: two-model review (O-29, F-37), 2026-10-24, from assembly.md
- Symptom: section 5 plugs the cell (step 3) before the FPC goes into J3 (step 4), so VSYS is live on J3 pin 50 through Q2 and JP2 while the FPC is pushed in; step 2 solders the speaker's leads to J4 with the cell already in its pocket, 20 mm away.
- Root cause: the procedure was written in the order the parts stack, not the order that is safe.
- Why missed: no DFMEA row for hand work near the cell.
- Check added or changed: VER-91 (records the step order: speaker on the bare board, iron cold, FPC latched, cell last; not run).
- Updated: DF-102, DF-112
- Status: OPEN (assembly.md to change)

### DEF-42: The Start/Select pills and caps are not in the BOM
- Found: two-model review (F-40), 2026-10-24
- Symptom: since 2026-10-19 design.py puts SW20-SW23 (SW21 is the power button) under the Start/Select membrane, but bom.md says "the Start/Select strip isn't used" and buys only cross and A/B/X/Y caps; DSLITE.md has no Start/Select numbers.
- Root cause: the BOM was not updated with the design.
- Why missed: nothing checks that every contact class has a bought pill and cap.
- Check added or changed: VER-167 (bom_coverage.py; not built).
- Updated: DF-124
- Status: OPEN

### DEF-43: Plug-in inrush is over USB 2.0's 50 uC
- Found: two-model review (F-24), 2026-10-24, by arithmetic
- Symptom: C1 10 uF x 5 V = 50 uC alone; with no cell, D5 charges the VSYS capacitors at plug-in (C2 4.7 uF, C5 10 uF, C9 22 uF, C34 10 uF, C35 0.1 uF: 46.8 uF x 4.6 V = 215 uC). R47's 0.1 ohm does not slow it.
- Root cause: bulk capacitance behind a diode, not a soft-start switch.
- Why missed: VER-99 was carried as NOT_BUILT behind the hot-plug ODE, though a calculator settles it.
- Check added or changed: VER-99 (the arithmetic, now an analysis that can be re-run from design.py).
- Updated: REQ-PWR-13
- Status: OPEN (the deviation is written into REQ-PWR-13; it closes when Alex approves that requirement and VER-99 passes)

### DEF-44: Requirements that failed by construction
- Board: both
- Found: two-model review (O-22, F-8, F-9, F-11, F-12, O-31), 2026-10-24
- Symptom: REQ-PWR-14's 80%-of-absolute-maximum rule failed every 3.3 V part; REQ-PWR-02's 5.8 V broke REQ-PWR-14's own transient rule; REQ-PWR-01 asked a TP4056 board to charge to done at 4.40 V; REQ-PRD-07 asked for normal-use touch limits under a single fault; REQ-FAB-03's 0.13 mm ring failed every via on the board; REQ-AUD-01's 0.35 W and 4.7 Vpp disagreed.
- Root cause: numbers written from memory or from one part, never checked against the design they govern.
- Why missed: requirements are prose, and none was approved yet.
- Check added or changed: none (requirements wording); VER-161 will check the numbers they quote.
- Updated: REQ-PWR-14, REQ-PWR-02, REQ-PWR-01, REQ-PWR-16, REQ-PRD-07, REQ-THM-01, REQ-FAB-03, REQ-AUD-01
- Status: FIXED (reworded; all still PROPOSED)

### DEF-45: The backlight sim cannot follow design.py, and its evidence missed loop-made parts
- Found: two-model review (O-5), 2026-10-24
- Symptom: board/sim/backlight_circuit.py hard-codes rs = 2.2, rg = 330, rf = 10k, cf = 1n, rtop = 100k, rbot = 3.9k and gate_pd = 100k. VER-05's covers regex matched design.py lines 428-431 and 437-439 but not 432-436, where R52-R59 and C48/C49 are made in a loop: R52/R53 at 1.8 ohm (69 mA a pin, 138 mA in total) would have left VER-05 fresh.
- Root cause: a sim that copies values, and line-regex freshness.
- Why missed: nobody changed a value to see whether the evidence went stale.
- Check added or changed: VER-05 (now covers part:R52-R61, C48-C51, Q5, Q6, U6; backlight.py to assert its defaults against design.PARTS; re-run needed).
- Updated: REQ-DSP-05, DF-44
- Status: OPEN

### DEF-46: Bring-up's cell emulation cannot hold 3.50 V with USB in
- Found: two-model review (O-19), 2026-10-24
- Symptom: VER-70 (the unplug at a 3.50 V 'cell' from the bench supply) and VER-67 (a 3.0-4.2 V sweep) both have USB connected; the TP4056 pushes 333-364 mA into the supply, most supplies cannot sink, so the 'cell' climbs toward 4.2 V and the 3.50 V corner is never tested.
- Root cause: a bench supply treated as a cell.
- Why missed: the procedure was never run.
- Check added or changed: VER-70 and VER-67 revised (CHG_OFF held high, or 10 ohm 5 W across the supply; VBAT recorded at the trigger instant).
- Updated: REQ-PWR-05, DF-34
- Status: FIXED (procedures revised; bringup.md to follow); closes when VER-70 passes
