# Defects and corrective actions

Every defect found in the design, the docs, the parts supply or the checks themselves.
The rule: **no defect closes without a new or changed check** that would have caught it,
and that check passes on fresh evidence. A defect whose design is fixed but whose check
does not exist yet is FIXED, not CLOSED. `check_trace.py` refuses a CLOSED defect with no
check named, and the DVT and PVT gates refuse any defect that is not CLOSED.

Fields: Found (where, phase, which check or person), Symptom, Root cause, Why missed (why
the checks before it did not catch it), Check added or changed (VER ids), Updated (REQ and
DF rows), Gate (the first gate it must be fixed by: R4 counts it there and at every later
gate, not before: O-R5), Status (OPEN, FIXED, CLOSED or ACCEPTED, then a note), and optionally
Board (ref_design when absent). A CLOSED defect's check must pass on fresh evidence, or check_trace fails.

ACCEPTED (2026-10-28) is the one way to move a defect to a later gate without fixing it. It needs
Alex's signed line `| A-NNN | DF-ACCEPT:DEF-nn | ACCEPTED | <the gate it moves to> | <why> |` in
releases/APPROVALS.md; an ACCEPTED Status without that signed line fails check_trace. Before the
gate it moved to, R4 lists the defect instead of failing on it, and every gate criterion (G-ORD-22
too) lets through as ACCEPTED the checks this defect names under 'Check added or changed', and only
those. From that gate on it counts as OPEN again. It does not verify a requirement (R2) or an H row
(R3): Alex defers the requirement or accepts the row on its own line. Leave the Gate field as it was.

Per-unit findings on built boards (a bridge, a tombstone, a reversed cell) are not design
defects: they go in hardware/vv/evidence/findings.csv (VER-110). A finding seen on 2 units
becomes a DEF here.

Seeded 2026-10-24 from the git log under `hardware/ref_design` (this branch, and three
commits on other branches where named) and from `docs/design_review.md` ("Found and fixed");
re-seeded 2026-10-25 for 684a609 and 2026-10-26 for be437f8. A defect whose part or function left the design is RETIRED
with the commit that removed it: it keeps its ID and history, and the DVT gate counts it as
closed. RETIRED is not a way to close a defect whose part is still on the board.

### DEF-01: USB ESD array's reference on VBUS back-fed VBUS
- Found: independent model audit, round 1, design phase before any order (4495595, 2026-09-25)
- Symptom: with no cable in, the MCU's D+ pull-up fed VBUS through U1's D+ steering diode to about 2.7 V; PWR_EN stayed up (the device could not turn off) and Q2 was left half on.
- Root cause: U1's REF pin tied to VBUS without tracing the paths back from the MCU's pins into an unpowered rail.
- Why missed: netlist, ERC and DRC check connectivity, not what flows where; no back-power path table existed (validation_plan #79).
- Check added or changed: VER-159 (check_backpower.py over design.nets(), seeded with this very fault: U1 pin 5 back on VBUS must fail it)
- Updated: REQ-PWR-07, DF-98
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was CLOSED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-223 on the RP2350B board.

### DEF-02: R15 cut to 10k removed the Start contact's pill margin
- Found: model audit, round 2, design phase (d049b35, 2026-09-25)
- Symptom: round 1 lowered R15 (PWR_EN pull-down) from 100k to 10k against D5 leakage; that cut the pill resistance Q1 could still read by about five times.
- Root cause: a fix for one failure mode made without re-deriving the latch's other margins.
- Why missed: the latch's margins exist only in prose; no state table or script recomputes them on a change.
- Check added or changed: VER-33 (latch state table: the largest series resistance that still turns on) and VER-211 (1 kohm in series with the power key on the real board)
- Updated: REQ-PWR-08, REQ-PWR-11, DF-27
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was FIXED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-230 (a change to one part re-derives the contacts' margins) and DF-242 (the power key) on the RP2350B board.

### DEF-03: The unplug hand-over took tens of ms, not microseconds
- Found: model audit, round 3, design phase (5095356, 2026-09-25)
- Symptom: C1 held VBUS up after an unplug and Q2's gate could only fall as fast as VBUS: Q2's body diode carried the system for tens of ms at the cell less 0.5 V. design_review.md said microseconds.
- Root cause: the hand-over was reasoned, not simulated, and the bleed path (R38 10k) was too weak.
- Why missed: no transient simulation of the power path at the time.
- Check added or changed: the ODE simulation behind design_review.md (not committed: VER-11); VER-70 on the real board.
- Updated: REQ-PWR-05, REQ-PWR-06, DF-24
- Gate: ORDER
- Status: RETIRED in 2d1124d: Q2, D7 and R38's old role left with the TP4056's load sharing; the BQ24073 joins OUT to BAT with no hand-over gap. VER-11 retired with it; VER-70 measures the unplug

### DEF-04: 3V3 fell under the flash's 2.97 V on most unplugs
- Found: Fable and Opus debate, design phase (e565907, 2026-09-26)
- Symptom: Q2's body diode drops about 0.6 V at 0.25 A; with the cell under about 3.7 V, 3V3 fell under QSPI_IOVDD's 2.97 V minimum on every unplug.
- Root cause: the body diode was counted on to carry the gap.
- Why missed: as DEF-03.
- Check added or changed: VER-11 (not committed), VER-70 (not run).
- Updated: REQ-PWR-05, DF-15, DF-34
- Gate: ORDER
- Status: RETIRED in 2d1124d: as DEF-03; 3V3 through the unplug is now REQ-PWR-05 (3.135 V, VER-70)

### DEF-05: Hot plug rang VSYS over the ME6211's 6.5 V
- Found: Fable and Opus debate, design phase (e565907)
- Symptom: a 2 m cable into a board with no cell peaked VBUS at 7.1 V and VSYS at 6.7 V (ME6211 maximum 6.5 V).
- Root cause: a ceramic input capacitor on a live USB plug-in; the TP4056 datasheet's own series resistor was missing.
- Why missed: no hot-plug simulation before the debate.
- Check added or changed: the hot-plug ODE (not committed: VER-12); VER-71 on the real board.
- Updated: REQ-PWR-02, DF-39
- Gate: ORDER
- Status: RETIRED in 2d1124d: VSYS is the BQ24073's regulated OUT (4.3-4.5 V), no longer VBUS through D5; the ME6211 left in 1fd0956. The plug-in ring now meets U2's IN (28 V absolute) and the enables: VER-12, VER-71

### DEF-06: VSYS rang over the FM8002A's 6.0 V, and bring-up still checks 6.5 V
- Found: validation checklist, design phase (4c4d846, 2026-10-05); the stale bring-up limit found while seeding this register (2026-10-24)
- Symptom: with no cell, VSYS peaked about 6.6 V against the FM8002A's 6.0 V maximum. R47 (0.1 ohm) brought it to about 5.7 V. bringup.md step 5.6 still passes VSYS up to 6.5 V, the ME6211's limit, not U7's.
- Root cause: the hot-plug limit was set from one part (U3) on a rail that feeds a lower-rated part (U7); the procedure was not updated with the fix.
- Why missed: no list of every part on each rail with its lowest absolute maximum; procedures are prose.
- Check added or changed: VER-71's pass limit is now VSYS <= 5.7 V and VBUS <= 7.7 V (REQ-PWR-02 from REQ-PWR-14's transient rule, F-8); VER-31 (ratings per rail) and VER-36's lowest-maximum-per-rail table (not built); VER-161 checks bringup.md's limits against that table.
- Updated: REQ-PWR-02, REQ-PWR-14, DF-41
- Gate: ORDER
- Status: RETIRED in 2d1124d: VSYS no longer follows VBUS (as DEF-05), so U7 sees at most 4.5 V; bringup.md 5.6's text is DEF-21's

### DEF-07: J6 TX and RX swapped against the Debug Probe
- Found: datasheet pin review (check_pins.py, pin_review/j6.json), design phase (4c4d846, 2026-10-05)
- Symptom: GPIO20 (UART1 TX) was on J6 pin 1, where the Debug Probe's TX also drives: contention, no link.
- Root cause: the connector spec names pins from the target's side; design.py named them from the probe's.
- Why missed: ERC, netlist and DRC trust design.py's pin numbers.
- Check added or changed: VER-02 (check_pins.py fails when a reviewed pin's net changes).
- Updated: REQ-MCU-04, DF-63
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was CLOSED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-251 (the debug UART's direction in the pin plan) on the RP2350B board.

### DEF-08: USB series resistors were 22 ohm; the RP2350 requires 27
- Found: datasheet pin review (check_pins.py, pin_review/u4.json pins 66 and 67), design phase (4c4d846)
- Symptom: R21/R22 were 22 ohm; the RP2350 datasheet (Table 2) requires 27 ohm.
- Root cause: a Basic part chosen for cost, reasoned as "fine for full speed" without the datasheet.
- Why missed: values were never checked against a datasheet requirement before the pin review.
- Check added or changed: VER-02 (passives.json checks each resistor's value and LCSC number). The stale comments are DEF-21's.
- Updated: REQ-MCU-02
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was CLOSED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-261 (the USB pair's parts from the chip's own datasheet) on the RP2350B board.

### DEF-09: The ME6211's CE could rise above its VIN at a live plug-in
- Found: datasheet pin review (pin_review/u3.json), design phase (4c4d846)
- Symptom: PWR_EN rises from VBUS through R14 within microseconds while VSYS lags behind R47/D5 into C9: CE could exceed VIN + 0.3 V, its absolute maximum.
- Root cause: two paths from VBUS with different time constants into one part's pins.
- Why missed: no transient check of pin-to-pin limits.
- Check added or changed: VER-72 (scope PWR_EN - VSYS at plug-in; not run).
- Updated: REQ-PWR-03, DF-30
- Gate: ORDER
- Status: RETIRED in 1fd0956: the RT9013's EN is 6 V absolute whatever VIN is; the enable's limit at a faulty source is DEF-50

### DEF-10: PWR_HOLD had no external pull-down (RP2350-E9)
- Found: validation plan (#178), design phase (2026-10-05; fixed in 4c4d846)
- Symptom: after a crash the released PWR_HOLD pad can sit near 2 V (E9 on A2 silicon); through D2 that kept the ME6211 on, so a crashed unit stayed on and drained its cell.
- Root cause: the reset state of a GPIO assumed, not read from the errata.
- Why missed: no errata table; the pin review found E9 only for the ADC pins.
- Check added or changed: VER-02 checks R42's value; VER-77 (release on real silicon; not built); VER-16 (errata table; not built).
- Updated: REQ-PWR-09, REQ-MCU-07, DF-25
- Gate: ORDER
- Status: RETIRED in 39cc07f: erratum E9 left with the RP2350; R42 stays and holds IO21 low after a reset (VER-33, VER-77)

### DEF-11: Charge current too high for the 403450
- Found: validation plan (#28, #128), design phase (fixed in 4c4d846, 2026-10-05)
- Symptom: R5 2.4k set 460-500 mA, about 0.7 C for a 700 mAh cell.
- Root cause: R5 kept from the 503450 (1000 mAh) when the cell changed.
- Why missed: no requirement tied the charge current to the cell; the cell's spec sheet was never obtained.
- Check added or changed: VER-37 (cell spec sheet; not run); VER-160 (cell.json and an assert that recomputes the limit when the cell or R5 changes; not built: it would have failed at 950afcf and 226bbb6); VER-66 (charge current per unit; not run).
- Updated: REQ-BAT-02, DF-07
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was FIXED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-216 on the RP2350B board.

### DEF-12: AP3156 backlight charge pump out of stock
- Found: stock check before ordering (b7a6238, 2026-10-07)
- Symptom: the AP3156 (C443652) had 0 stock at JLC and LCSC.
- Root cause: a single-source Extended part with low stock and no stock watch.
- Why missed: stock was last checked on a July 2026 snapshot.
- Check added or changed: VER-41 (order-day stock and pre-orders; not run); VER-14 (LCSC identity and stock class; not built).
- Updated: REQ-FAB-10, DF-69
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was FIXED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-238 on the RP2350B board.

### DEF-13: ProTek SRV05-4 out of stock
- Found: stock check (eea0ee8, 2026-10-07)
- Symptom: JLC had 2 of C85364 left.
- Root cause: as DEF-12.
- Why missed: as DEF-12.
- Check added or changed: VER-41; VER-14.
- Updated: REQ-FAB-10, DF-69
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was FIXED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-238 on the RP2350B board.

### DEF-14: U1's JLC footprint may be turned 180 degrees from KiCad's
- Found: on another branch, 1fd0956 (2026-10-22): "JLC's footprint for C13612 is KiCad's SOT-23-6 turned 180 degrees". This branch's eea0ee8 (2026-10-07) says "the same JLC library footprint ... so the CPL rotation is unchanged". The review (F-19) reads both as true: eea0ee8 means unchanged against the ProTek C85364 it replaced, which shared JLC's footprint, and JLC's SOT-23-6 zero differs from KiCad's by 180 degrees. So the CPL with KiCad's angle places U1 reversed unless corrected.
- Symptom: if 1fd0956 is right, a CPL with KiCad's angle places U1 reversed: 3V3 on its ground reference, its TVS forward biased, 3V3 shorted.
- Root cause: rotation offsets between KiCad and JLC footprints are checked by eye in JLC's preview, and only on order day.
- Why missed: there is no rotation table and no dry run has been done.
- Check added or changed: VER-29 (rotation table from the kicad-jlcpcb-tools plugin's corrections, applied to fab.py's CPL; not built); VER-28 (dry run with screenshots; not run).
- Updated: REQ-FAB-07.R, DF-67, DF-68 (U1 is Semtech's C13612 again since 1fd0956; assembly.md says JLC's footprint is KiCad's turned 180 degrees)
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-234 on the RP2350B board.

### DEF-15: First PLA print: hooks snapped, lid tight, pins bound
- Found: first full print of the case (88eec7c, 2026-10-05)
- Symptom: the 1.9 mm FDM pins bound and snapped; the snap hooks snapped; the lid was tight at the deck steps and felt flimsy.
- Root cause: FDM layer strength and fit tolerances not designed for; no coupon before the full print.
- Why missed: check_fit checks heights, not snap strain or print tolerance.
- Check added or changed: none that runs; VER-48 (current shell with real parts) and VER-92 (open/close cycles) not run.
- Updated: REQ-MEC-05, DF-89
- Gate: CASE
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was FIXED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-249 (the printed case's features) on the RP2350B board.

### DEF-16: D-pad opening too tight for Alex's AliExpress D-pad
- Found: fit print with real caps (7606442, 2026-10-19)
- Symptom: the D-pad bound in its opening.
- Root cause: cap sizes from open models of genuine parts; the aftermarket D-pad is bigger.
- Why missed: no measured cap dimensions per lot.
- Check added or changed: the fit kit's tune_openings coupon; VER-126 (incoming caps per lot; not run).
- Updated: REQ-MEC-03
- Gate: CASE
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was FIXED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-249 (the printed case's features) on the RP2350B board.

### DEF-17: Deck insert would not go on over the silicone
- Found: insert print (73c3686, 2026-10-19)
- Symptom: round each cluster the deck came down as a solid disc to 0.35 mm over the board, and the flanges stopped 1.75 mm up.
- Root cause: the silicone's height estimated, the shell's rib structure not modelled.
- Why missed: no clash check between silicone, caps and shell.
- Check added or changed: VER-07 (check_buttons.py, added 269dc87, shown to fail on a cap that does not fit its slot).
- Updated: REQ-MEC-03
- Gate: CASE
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was FIXED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-249 (the printed case's features) on the RP2350B board.

### DEF-18: Start/Select pad's square wells hit round flange pockets
- Found: silicone pad model (86e81e2, 2026-10-19)
- Symptom: the pad's square wells collided with the round pockets in the lid.
- Root cause: pocket shape drawn before the pad was modelled.
- Why missed: as DEF-17.
- Check added or changed: VER-07 (caps_silicone and silicone_shell checks).
- Updated: REQ-MEC-03
- Gate: CASE
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was FIXED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-249 (the printed case's features) on the RP2350B board.

### DEF-19: Holes through the lid
- Found: Alex, on the live 3D view (177123c, 2026-10-19)
- Symptom: the panel pocket kept the frame's 6 mm FPC notch, which the step's S-curve cut open; the six 6 mm screw pilots came through the screen face.
- Root cause: features sized before the screen face dropped to 5.8 mm over the board.
- Why missed: no check for unintended openings.
- Check added or changed: none on this branch; VER-113 (openings check; not built). Another branch has check_manifold.py. The same class (the stack moved and a fastener was not recomputed) is DEF-31.
- Updated: REQ-MEC-08, DF-90
- Gate: CASE
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was FIXED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-249 (the printed case's features) on the RP2350B board.

### DEF-20: J6's mouth faced J5's back
- Found: model audit, round 1 (4495595)
- Symptom: J6's mouth faced J5's back 5 mm away: no room for the cable.
- Root cause: connector access not checked in placement.
- Why missed: no connector-access keep-out.
- Check added or changed: VER-165 (connector mouth keep-outs in check_fit; not built).
- Updated: REQ-MCU-04, REQ-MEC-11
- Gate: ORDER
- Status: RETIRED in 86bf694: J6 left; connector access for J1, J2, J4 and J5 is VER-165 (REQ-MEC-11)

### DEF-21: Docs disagree with the design
- Found: while seeding this register (2026-10-24, VER-39); some of these are fixed on other branches (67e3d13), not here
- Symptom: re-checked at be437f8 (2026-10-26, VER-39). Still true or new: (a) bringup.md step 0 expects one body-diode drop from Q2 and TP1 above about 1 kohm; Q2 is gone and R38 is 10k (VER-53 and VER-52 hold the new limits). (b) bringup.md lists TP7 as LS_GATE; it is CHG_TEMP, the BQ24073's TS node. (c) bringup.md steps 1, 2.3, 5.1-5.6 describe the TP4056: CHG_DONE_N, 333-364 mA by R5, the TEMP pin, D7 at the unplug, VBUS under 8 V for the TP4056; step 4a puts TH1 left of J3. (d) bringup.md 6.2-6.3 still call seat 1's Y the power button (both Starts since 2d1124d). (e) assembly.md's polarised list (line 17) and section 3 (line 55) name the TP4056; 'JP1 and JP2 come bridged' (five jumpers). (f) bom.md: TP4056 at $0.17 among the priciest parts (line 33), 'the Start/Select strip isn't used' and no domes or Start/Select caps (DEF-42). (g) hardware/ref_design/README.md's first paragraph says the cell lies under the board (it sits in the board's hole since 2026-10-21). (h) pin_review/panel.json and u6.json still name the ME6211 as 3V3's source. (i) docs/pinmap.md calibrates the VBAT gain at the end of a charge (DEF-40, DF-117). (j) assembly.md step 10 lifts the tub with the cell still plugged into J2 (DF-169). (l) bom.md:55 still buys an adhesive foam pad for the cell. (m) pin_review/u4.json's E9 note (line 17) says PWR_HOLD is GPIO2, CHG_OFF GPIO19, AMP_EN GPIO11 and BL_PWM GPIO44 (GPIO44, GPIO43, GPIO34 and GPIO28). New at be437f8: (n) bringup.md's display steps 3.1-3.5 are the 2.4 in panel's: the ST7789V's init, RDDID 0x85 0x85 0x52, IM3-0 = 1110, COLMOD 0x55, 154 kB frames, 70 mA backlight at 0.077 V and a 29% minimum duty (the ILI9488 needs ID4 0x94 0x88, IM2-0 high, COLMOD 0x66, 461 kB frames; 113 mA at 0.124 V; 18%). (o) assembly.md says the speaker sits on the board's top (line 67, it is sunk in a hole since 9cff0d3) and J4's mouth faces the glass (line 88; placement.py turns it towards the speaker). (p) placement.py's comments: the docstring and SPEAKER's block put the speaker on the board's top, CUTOUTS' block says the speaker and its amplifier went and gives strips of 7.5 and 8.9 mm (9.2 each at be437f8); design.py:152 ('half-close Q2') and :455 (the speaker on the board's top) (F's objection). (q) hardware/ref_design/PLAN.md's table: the speaker on the board's top, the case 77.4 x 132.7 x 9.55 mm (156.64 x 9.6 at be437f8), the cell between seat 2's deck and J3's range. (r) docs/design_review.md's charger and power sections still describe the TP4056 and the ME6211. (s) the panel's datasheet: pin_review/panel.json cites a Wayback fetch of 2025-01-09 into pin_review/ds/, while case.scad (lines 31-32) says the datasheet could not be downloaded on 2026-10-26. Neither is so: pin_review/ds/ does not exist in the main checkout, but hardware/ref_design/datasheets/er-tft035ips-6.pdf and its .txt do (REV 1.0 of Dec-15-2022, untracked), and their outline differs from case.scad's (DEF-59). panel.json's '2.35 +-0.1' thickness is not in that copy. panel.json's fpc_orientation still puts J3 at (100, 120.89); the board has it at (100.0, 117.59) (pcbnew at be437f8) (F-R7). New at 71737ae (re-read 2026-10-27): (t) assembly.md and bringup.md do not yet say the speaker's leads are cut and given a JST SH plug for J4 (DF-183, DEF-64), nor that the cell lies along the board in a 37.8 x 54.8 mm hole with its leads out of the seat-2 end to J2 in the right strip; (u) bom.md quotes the panel at 54.50 x 83.00 x 2.30 mm, a third size beside case.scad's 54.56 x 82.84 x 2.45 and the drawing's 54.56 x 82.84 x 2.15 +-0.15, and its notes still say the active area and thickness are the old design's (F-S3); (v) bom.md still says 'the Start/Select strip isn't used' (VER-167). (w) assembly.md step 5 tapes the cell 'pushed against the pocket's right wall' (line 81); since 71737ae the cell lies along the board and the pocket's walls hold it. (x) assembly.md:14 says 117 placements and 49 part numbers; at 71737ae fab.py writes 120 CPL rows and 48 BOM lines, as design.py gives (VER-130, check_bom_cpl.py, 2026-10-27). (n)'s backlight numbers are now 108 mA nominal at 0.119 V (R61 3.74k). Re-run 2026-10-27 at 71737ae (VER-39): 21 of 22 items still true, only (d) is fixed. Items (a)-(r) of 2026-10-24 that concerned parts that left went with them; (k), bom.md's board size, is NOT fixed (corrected 2026-10-27, O-R3): its order line asks for 72.4 x 151.64 mm, and at 71737ae the board is 72.4 x 151.48 mm, 73.4 mm wide across J1's 1.0 mm tab (Edge.Cuts x 63.8 to 136.2, the tab to 137.2, y 24.26 to 175.74); that line is the number typed into the order, so it is DEF-79's, at Gate ORDER. New at 71737ae (O-R3, O-R4, O-R5, F-R4): (y) case/SHELL.md's current section (2026-10-26, 'this section wins over the numbers below') gives the case as 77.4 x 156.64, the board as 72.4 x 151.64 with a 54 x 37 cell hole and the panel as 54.50 x 83.00, while at 71737ae the case is 156.48 mm long, the board 151.48 mm, the cell hole 37.8 x 54.8 mm (the cell turned 90 degrees) and the panel's outline 54.56 x 82.84. (z) hardware/ref_design/case/CELLCUT_FOR_CASE.md@9975484's top section (2026-10-27, 'Turned 90 degrees') is current for the hole and the pocket, but the board and case lengths sit in its superseded section (72.4 x 151.64 and 77.4 x 156.64 x 9.6). (aa) hardware/ref_design/README.md's case and board lines (31, 35) still give 156.64 and 151.64. (ab) design.py's backlight comment says 0.124 V at 100% duty, 56 mA a pin and 113 mA for the six LEDs, and its R52/R53 comment says 2.2 ohm x 56 mA = 0.124 V, above R61's own line (0.119 V, 117.7 mA worst case). (ac) board/sim/backlight_circuit.py's docstring says GPIO44's PWM through 100k into 3.9k and its rbot is 3.9e3, and board/sim/backlight.py's K is 3.3 x 3.9 / 103.9 and its docstring's filter 100k / 3.9k: the simulation models R61 3.9k, not 71737ae's 3.74k on GPIO28 (DEF-45, VER-05). (ad) bom.md's feeder-loading row says 12 extended part numbers because 'J4 reuses J2's part number', with a $36.84 setup and a $53.35 total per board; J4 is a JST SH SM02B-SRSS-TB (C160402) of its own since 71737ae, so the count and the price are wrong (F-R4, DEF-79). The design session rewrites (y)-(ad); the register asks and checks, it does not edit them.
- Root cause: docs repeat numbers from design.py and case.scad by hand; nothing compares them.
- Why missed: no doc check; reviews read the docs as truth.
- Check added or changed: VER-39 (stale-doc sweep, now also design.py's and placement.py's comments and the case documents case/*.md: FAIL 2026-10-26 at be437f8, the list above; its scope is hardware/ref_design/docs/*.md, hardware/ref_design/case/*.md, hardware/ref_design/README.md, hardware/ref_design/PLAN.md, design.py and placement.py); VER-161 (check_claims.py: the numbers, GPIOs and lists compared with design.py, case.scad and pin_review; not built) is the real fix; VER-130 (check_bom_cpl.py) fails on (x) until assembly.md agrees. For (s): the PDF is on disk, so VER-152 reads the FPC neck, body, tail and tip range off its outline drawing now and cites the lines (tools/datasheet_hashes.py records the file's sha256); the ESTIMATE stays in case.scad until the case pass, and the real panel's measurement stays the closing check. bringup.md matching the design is now also a precondition of first power (G-EVT-02, round 2's objection).
- Updated: REQ-TST-01, DF-102, DF-169
- Gate: CASE
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-239 on the RP2350B board.

### DEF-22: The validation plan's statuses went stale
- Found: task brief and this register (2026-10-24)
- Symptom: since 2026-10-07 R42, Q4/R43/R44, JP1-JP5, TP6-TP16, C41-C43 and the revision strap landed in design.py while validation_plan.md still marked those items "no".
- Root cause: statuses kept by hand in a 218-item prose list.
- Why missed: nothing reads the statuses back.
- Check added or changed: vp_map.csv holds the statuses with evidence as a one-time mapping; validation_plan.md is archived and check_trace no longer compares the two (O-11), so editing the old plan can no longer break the register. No check proves a status right; the live statuses are the VER rows.
- Updated: none
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was FIXED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-239 (the docs and the register describe the board) on the RP2350B board.

### DEF-23: JST PH J2 too tall for the lowered lid
- Found: print preparation (f231998, 2026-10-19)
- Symptom: the 5.5 mm JST PH needed a 0.5 mm pad on the lowered lid, which lifted the screen face off the bed when printed face down.
- Root cause: connector height chosen before the lid came down.
- Why missed: check_fit allowed J2 into a lid pocket by rule.
- Check added or changed: VER-06 (check_fit, J2 pocket rule)
- Updated: REQ-MEC-01
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was CLOSED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-229 (part heights against the case) on the RP2350B board.

### DEF-24: Audio numbers in the docs were wrong
- Found: review panel of seven model families, checked by Fable (7768a97, 2026-09-25)
- Symptom: the docs claimed two 16 kHz poles (really -3 dB at 6.6 kHz loaded), 1 W full scale (really about 0.55 W on USB) and ignored the FM8002A's 100 ms wake.
- Root cause: numbers from unloaded filter formulas and the amplifier's headline rating.
- Why missed: no audio simulation committed.
- Check added or changed: VER-100 (audio chain simulation; not in the repo); VER-64 on the real board.
- Updated: REQ-AUD-01, REQ-AUD-03, REQ-AUD-05
- Gate: CASE
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was FIXED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-248 (the audio chain's numbers from a script) on the RP2350B board.

### DEF-25: Lid window too tight for the panel's float and tolerance
- Found: model audits, rounds 1-3 (4495595, d049b35, 5095356)
- Symptom: the panel's locating frame had 0.4 mm clearance and the window 0.5 mm margin, less than the panel's +-0.4 mm float, its +-0.3 mm active-area tolerance and print error.
- Root cause: nominal fits without a tolerance stack.
- Why missed: no XY or Z stack-up analysis.
- Check added or changed: check_fit learned the lid's panel frame (d049b35): VER-06 (stale); VER-20 (stack; not built).
- Updated: REQ-MEC-02, REQ-PRD-06
- Gate: CASE
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was FIXED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-249 (the panel's window against its float) on the RP2350B board.

### DEF-26: The datasheet gate passes with open questions in it
- Found: while seeding this register (2026-10-24)
- Symptom: check_pins.py passes while the pin reviews hold findings it never reads: panel.json's VCI and VDDI (ok false: 3V3 up to 3.366 V against 3.3 V), u4.json pin 58 (GPIO47 above IOVDD when off, 'WAIVED' by the design session in the review file, not by Alex), the missing second 4.7 uF on +1V1 (pin 10), and ratings text that still names parts that left (u6.json: ME6211). The BQ24073's TS tolerances, VBAT(REG), pre-charge and termination limits are not in u2.json at all, though VER-32, VER-134 and VER-138 need them. The second review (O-R20, 2026-10-25) adds: a waiver passes whoever wrote it (u4.json pin 58 by the design session); a review's mpn and lcsc are never compared with design.py, so an IC swapped on the same nets passes (every identity matches today, by script); entries with ok null (j1's stubs) are never read; reviews of removed parts (d5, q2, r47, j6, and the C41/R48 entries in passives.json) still sit in pin_review and are to move out (outside hardware/vv).
- Root cause: check_pins.py fails only on a pin with ok false; it never reads the ratings, and "OPEN QUESTION" text in an ok true entry is invisible to it.
- Why missed: the gate was written to catch net changes, not unresolved findings.
- Check added or changed: VER-36 (read both rating schemas; fail on ok false or null without a waiver, on a waiver that cites no A-NNN line, on an IC's or connector's lcsc or mpn that differs from design.py, on a review of a ref absent from design.py, on open-question text, on a passive rated under 1.25 x its net; not built). Its acceptance test is the failure list it generates from the current files, never a typed list.
- Updated: REQ-FAB-02, REQ-PWR-14, REQ-MCU-09, DF-94, DF-95, DF-96
- Gate: EVT
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-211 on the RP2350B board.

### DEF-27: check_fit guesses part heights silently
- Found: validation plan #96 (2026-10-05); still true at 76955d2
- Symptom: KNOWN_HEIGHT is empty, PACKAGE_HEIGHT is a guess table, and an unknown package gets 1.5 mm without a warning.
- Root cause: the fit check was written for speed with box models.
- Why missed: nobody seeded a fault to see whether check_fit can fail on a tall part.
- Check added or changed: VER-112 (heights from drawings; unknown package fails; not built, at CASE); VER-22 (seeded faults; not built).
- Updated: REQ-MEC-01, DF-91
- Gate: CASE
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-229 on the RP2350B board.

### DEF-28: Design analyses are not in the repo
- Found: while seeding this register (2026-10-24)
- Symptom: the hot-plug analysis at U2's IN, the copper-resistance solves and the audio filter run are quoted in design_review.md and validation_plan.md, but no script for them is committed; the backlight sim that is committed runs for the 3.5in panel (DEF-45). Since 2026-10-25 the pre-order numbers this register quotes (inrush, backlight worst case, cutoff, screws, MLCC keep-out, TH1, CPL, drill files, VBUS fault reach, BOM coverage, holes, swelling) come from hardware/vv/tools/design_numbers.py.
- Root cause: analyses run in scratch sessions; only their conclusions were kept.
- Why missed: no rule that a quoted number needs its script in the repo.
- Check added or changed: VER-12, VER-13, VER-100 carry the missing ones as NOT_BUILT; VER-99, VER-114, VER-131, VER-137, VER-138, VER-142, VER-147, VER-164, VER-167, VER-176, VER-177 and VER-20 (in part) now run design_numbers.py through run_checks.py; the README's evidence rule: an analysis or test PASS needs a saved output under evidence/ whose header names its commit, tools and result.
- Updated: REQ-PWR-02, REQ-PWR-05, REQ-AUD-03
- Gate: EVT
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-222 (the hot-plug analysis as a script) and DF-248 (the audio filter's) on the RP2350B board.

### DEF-29: RP2354B and L2 out of stock at JLC and LCSC
- Found: BOM audit on another branch (67e3d13, 2026-10-21): "RP2354B and L2 out at JLC and LCSC, U1 3 left, J5/J6 2 left, U3 724". This branch's bom.md still shows 1665 RP2354B in stock (2026-10-07).
- Symptom: the order cannot be assembled as designed; JLC may offer substitutes.
- Root cause: low-stock single-source parts; the core inductor must be the Abracon part the RP2350 guide names.
- Why missed: as DEF-12.
- Check added or changed: VER-41 (order-day stock, pre-orders, no substitution; not run).
- Updated: REQ-FAB-10, DF-69, DF-93
- Gate: ORDER
- Status: RETIRED in 39cc07f: the RP2354B and L2 left; the new parts' stock and class are DEF-81's

### DEF-30: Which design is the order candidate is not recorded
- Found: while seeding this register (2026-10-24)
- Symptom: the board to order is hardware/ref_design at 71737ae (main, 2026-10-27: the board spin 40281b8 merged as bfe9609, then a SOURCES.md fix), per the 2026-10-27 brief; this register was re-seeded for it and README's design baseline names its files. No release record names it yet; if a further design pass is needed (DEC-27's hole, DEC-28's marker), the order candidate is the commit after it, named by DEC-35 and the release record, and the baseline moves to it (O-S3). Was: be437f8 on ref-design-35in.
- Root cause: no release record names the commit being ordered.
- Why missed: no configuration control before this register.
- Check added or changed: VER-42 (release record: tag, commit, fab zip hash; not run); check_trace.py's rule R7 fails any gate when HEAD's design files differ from README's design baseline ("gate evaluated against a design it does not describe", F-1); evidence goes stale when HEAD moves past changes it covers, or its commit is not an ancestor of HEAD.
- Updated: REQ-FAB-08, DF-76
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-237 (the order candidate named in a release record) on the RP2350B board.

### DEF-31: The case screws engage 1.4 mm of the lid's pilots
- Found: two-model review (O-52), 2026-10-24, from case.scad
- Symptom: since the case got 3.8 mm thinner (2026-10-22) the screws are M2 x 6 from a 1 mm counterbore: the tip at z 7.0, 3.2 mm of thread above the board (1.6 x d) in the side pilots under the screen band (5.00 mm deep) and the end pilots (3.30 mm). Self-tapping screws in printed plastic want 4-5 mm where the pilot allows it. bom.md buys M2 x 6 since 03df0f2.
- Root cause: the stack changed and the fastener and pilot were not recomputed (DEF-19's class).
- Why missed: no assert ties the screw length to the stack; bom.md's "M2 x 8" is prose.
- Check added or changed: VER-147 (design_numbers.py screws: FAIL at 3.2 mm); VER-92 counts stripped bosses after 20 cycles; VER-190 (the case's architecture, 2026-10-27) maps REQ-MEC-09 to screw_l and fails on the same 3.2 mm.
- Updated: REQ-MEC-09, DF-119
- Gate: CASE
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-232 on the RP2350B board.

### DEF-32: The drill file mixes plated and unplated holes
- Found: two-model review (O-14), 2026-10-24, from a scratch export with fab.py's own kicad-cli arguments
- Symptom: fab.py still exports one Excellon file (kicad-cli's default, MixedPlating) holding the 0.30 mm vias, J1's plated slots, and the unplated holes: J1's two 0.65 mm pegs, SW20-SW23's four 0.5 mm holes and H1-H6 at 2.2 mm. JLC's KiCad guide asks for separate files; a CAM that plates by file shrinks the pegs and J1 rides up off its pads.
- Root cause: kicad-cli's default drill export; nobody read the drill file.
- Why missed: VER-27 (the one look at the gerbers and drills) was gated nowhere (O-1).
- Check added or changed: VER-131 (design_numbers.py drill: FAIL until fab.py passes --excellon-separate-th; then parse both files); VER-114 (every hole >= 0.30 mm: PASS).
- Updated: REQ-FAB-04, DF-127
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was CLOSED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-250 (the drill files split) on the RP2350B board.

### DEF-33: The CPL uses footprint origins, and J1's and J2's are off their pads
- Found: two-model review (O-16), 2026-10-24, measured with pcbnew
- Symptom: re-measured at be437f8 with pcbnew (design_numbers.py cpl): every assembled footprint's origin sits within 0.10 mm of its pads' centre except J1 (1.50 mm), J2 and J4 (0.95 mm, the same PicoBlade footprint) and J3 (0.23 mm). fab.py writes the origin; JLC places by its own library footprint's origin, which for a connector is usually the body's centre, as J1's own origin is (its F.Fab body is centred on it; the pads are not, because the rear shell legs sit behind the SMD row). So the 1.50, 0.95, 0.95 and 0.23 mm are the offsets between origin and pads' centre; whether JLC's placement ends up off the pads is unknown until the placement preview (O-R7). Writing the pads' centroid into the CPL now could create the offset instead of removing it, so no CPL row moves before the preview.
- Root cause: footprints whose origin is not the centroid, and a CPL that assumes it is.
- Why missed: no centroid check; the dry run has not been done.
- Check added or changed: VER-176 (origin against the pads' centre: FAIL; J1-J4 pass only through a JLC_FIXED entry naming the placement preview screenshot and the offset JLC's library applies, which may be zero); VER-29 (the rotation table, at DVT; not built); VER-28 screenshots J1-J4 and is the reference.
- Updated: REQ-FAB-07.R, DF-128
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was FIXED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-234 on the RP2350B board.

### DEF-34: No firmware can run on the ordered board
- Found: two-model review (O-56, F-44), 2026-10-24
- Symptom: the fw crate targets the RP2350A (rp235xa, GPIO0-29), 4096K of flash and the ST7796; the only ref-board firmware (fw-ref-board a01f6fb) predates the routing levers (86bf694): it holds PWR_HOLD on GPIO2 (P2_START at 684a609) and drives SR_PL on GPIO28 (BL_PWM); every pin number is typed by hand. pinmap.md is generated from design.py (board/tools/pinmap.py), but no firmware file is.
- Root cause: firmware written against another branch's design, with no generated pin file.
- Why missed: the generated pin definitions (vp_map #56) were scheduled for EVT, so the first build for the real chip would be the day the boards arrive.
- Check added or changed: VER-149 (the generated pin file against design.py) and VER-215 (the ESP32-S3 build)
- Updated: REQ-MCU-10, REQ-MCU-11, DF-121, DF-122
- Gate: EVT
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-251 (a pin file generated from design.py) on the RP2350B board.

### DEF-35: The trace gate could pass on evidence that did not show what it claimed
- Board: both
- Found: two-model review (O-1, O-2, O-3, O-4, O-5, O-26, F-2, F-3, F-5, F-13), 2026-10-24
- Symptom: eight ORDER checks (VER-13, VER-19, VER-21, VER-22, VER-27, VER-34, VER-126, VER-127) were named by no order criterion, requirement or H row, so the order gate could pass without them. An H row was "covered" by any one passing check in its detection text, even one that cannot see its failure (DF-70 by VER-02, DF-26 by VER-104). An acceptance for EVT held for ever. PASS was typed by hand, twice resting on a commit message (VER-06, VER-07). A covers regex kept its quotes and so never matched (VER-43's "(PANEL|fpc|J3)"), and an unquoted one over-matched (VER-37's R5 on R52-R61); loop-made parts (R52/R53) never staled VER-05. A check on external state (stock) never went stale. DF-87 reached L with an occurrence of 1.
- Root cause: the gate checked that IDs existed, not that the evidence could show what was claimed.
- Why missed: the seeded-fault run that tested check_trace was never committed (DEF-28's rule).
- Check added or changed: VER-129 (tools/seed_faults.py: each of these faults seeded into a copy of the register must fail check_trace).
- Updated: README rules; gates.md G-ORD-22; dfmea.csv 'detects'; verification.csv valid_days
- Gate: ORDER
- Status: CLOSED (VER-129 PASS 2026-10-30 at f0d7373: 156 of 156 seeded faults caught, after the policy checks' edits to check_trace.py and seed_faults.py)

### DEF-36: check_pcb.py passes every DRC warning that is not silk
- Found: two-model review (O-12, F-27), 2026-10-24
- Symptom: battler.kicad_pro sets hole_to_hole, holes_co_located, copper_sliver, isolated_copper, connection_width, padstack, lib_footprint_mismatch, lib_footprint_issues, footprint_symbol_field_mismatch, net_conflict, missing_footprint, extra_footprint and duplicate_footprints to warning; check_pcb.py fails only on errors, while VER-04's pass limit says 'silkscreen warnings only'. At be437f8 its three 'cosmetic' silk_edge_clearance warnings are the board's name, BOARD_REV and build text cut away by the cell's hole (DEF-57): a silk warning can hide a defect too.
- Root cause: the script's rule is weaker than the limit it is credited with.
- Why missed: no seeded fault for check_pcb.
- Check added or changed: VER-04 (fail on any type not in COSMETIC, and silk_edge_clearance out of COSMETIC), VER-22 (a seeded hole_to_hole fault; DVT); not built. VER-18 now reads the silk texts against the outline itself (FAIL). At ORDER, VER-04's evidence lists every DRC warning type and count, read by hand (F-R2(e)).
- Updated: vp_map #19
- Gate: EVT
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was FIXED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-250 (DRC with every warning counted) on the RP2350B board.

### DEF-37: TH1 sits at the cell's edge, and four bare pads face the pouch
- Found: two-model review (O-30, F-36), 2026-10-24, from placement.py and case.scad
- Symptom: since the cell sits in the board's hole (2026-10-21) TH1 is on the board's top at (127.8, 79.0) since the 3.5 in re-placement, 0.80 mm right of the hole's right edge, facing the cell's end across the routed FR4 edge and an air gap of 0 to 0.5 mm when the case pushes a worst-case cell right (up to 6 mm for a short cell not pushed home); its GND pad ties it to the pour. The bare back-side pads no longer face the cell (none within 1 mm of the hole: VER-164).
- Root cause: the cell moved under the board (2026-10-09) and nothing checks placement against the cell's rectangle.
- Why missed: check_fit never reads positions.json's cell and skips test points on its back-side pass.
- Check added or changed: VER-164 (design_numbers.py th1: the geometry, PASS); VER-87 (two thermocouples: the lag; not built).
- Updated: REQ-THM-02, REQ-BAT-08, DF-04
- Gate: CASE
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-218 on the RP2350B board.

### DEF-38: C9 sits 0.9 mm from a screw boss's edge
- Found: two-model review (O-43), 2026-10-24, measured with pcbnew
- Symptom: C9 (22 uF on VSYS, the S9 flex-crack row) at (131.2, 131.4) is about 0.9 mm from the edge of the boss that clamps H3 between two printed bosses; no other MLCC is within 8 mm of a hole. The screws go in after the cell is plugged in, so a crack shorts the cell through Q2 inside the closed case.
- Root cause: no keep-out between MLCCs and the case's clamping bosses.
- Why missed: DF-38's detection (VER-52) runs before the board is screwed in.
- Check added or changed: VER-142 (MLCC keep-out; not built); VER-92 (C9 at 40x and TP2 after 20 screw cycles).
- Updated: DF-38
- Gate: ORDER
- Status: RETIRED in 2d1124d: C9 left with the TP4056's load sharing; the class (an MLCC by a clamping boss) is VER-142 (PASS: the nearest is C14, 4.40 mm from H4) and REQ-MEC-12

### DEF-39: Backlight at 100% exceeds the panel's 120 mA at worst-case tolerance (again since 79de1f7)
- Found: two-model review (F-7, O-21), 2026-10-24, by hand
- Symptom: on the 3.5in panel (until 53e21c8), 3V3 +2%, R61 +1% and R60 -1%, U6's +4.5 mV offset and R52/R53 -1% gave about 61.2 mA a pin, 122.5 mA against 120 mA. The 2.4 in panel (53e21c8 to 79de1f7) took it to 78.0 mA against 80 mA and the defect was closed. 79de1f7 brought back the ER-TFT035IPS-6 with R61 3.9k, and the same stack again: 61.2 mA a pin, 122.4 mA against 120 mA (20.4 mA an LED on average); the design comment and pin_review/panel.json say '120 mA worst case' because they leave out the resistors' 1%.
- Root cause: the requirement and the sim used nominal parts; then the panel change brought back a value chosen for nominal.
- Why missed: no tolerance sweep at first (the sim hard-codes its values: DEF-45); in 79de1f7 the pin review re-derived the worst case by hand without the resistors' tolerance. VER-137 would have failed on that commit had check_trace run on design commits (DEF-47, PLAN.md step 0.3).
- Check added or changed: VER-137 (design_numbers.py backlight: the tolerance sum from design.py's values, now with the six-LED panel's 120 mA and the firmware's highest duty read from pinmap.md: FAIL 2026-10-26, 122.4 mA; it prints the duty cap, 97.9%, and the R61 that would pass, 3.74k).
- Updated: REQ-DSP-05, REQ-DSP-07, REQ-DSP-12, DF-130
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was CLOSED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-228 on the RP2350B board.

### DEF-40: After calibration, the 3.50 V cutoff may never fire
- Found: two-model review (O-49, O-23, O-60), 2026-10-24, by hand
- Symptom: the ADC's reference is 3V3 and pinmap.md sets the gain at the end of a charge taking the cell as 4.20 V (the BQ24073's CV is +-1%): in regulation the cutoff fires at a true 3.465-3.535 V; in dropout the reading rises as the cell falls, and with 3V3 +2%, the RT9013's 120 mV maximum dropout at 150 mA and a -1% CV its lowest reading is 3.521 V, so the cutoff never fires. With no cell the BQ24073's battery detection toggles CHG, so a gain from a false end-of-charge edge can also be stored. GPIO47 also reads through R17/R18's 235 kohm, so every 100 nA of pad leakage is 47 mV of VBAT: the RP2350's input leakage over temperature alone can exceed REQ-BAT-12's window (O-R15; design_numbers.py cutoff now counts it).
- Root cause: a ratio measurement treated as absolute; a calibration point chosen for convenience.
- Why missed: VER-67's limit applies above 3.6 V and its sweep runs with USB in, which keeps 3V3 regulated.
- Check added or changed: VER-138 (design_numbers.py cutoff: FAIL), VER-139 (bench sweep without USB), VER-155 (bench calibration at 3.50 V)
- Updated: REQ-BAT-12, REQ-TST-06, DF-116, DF-117
- Gate: EVT
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-225 on the RP2350B board.

### DEF-41: Assembly works next to a live cell
- Found: two-model review (O-29, F-37), 2026-10-24, from assembly.md
- Symptom: on 2026-10-24 assembly.md plugged the cell before the FPC went into J3 and soldered the speaker with the cell in the tub. Since 7da6a90 and the island's assembly (2026-10-23) nothing is soldered and the FPC is latched before the cell is plugged (section 5 step 6). Still open in the same class: J4 is plugged with the cell in, and the case is opened with the cell still on J2 (DF-169, DF-170).
- Root cause: the procedure was written in the order the parts stack, not the order that is safe.
- Why missed: no DFMEA row for hand work near the cell.
- Check added or changed: VER-91 (records the step order: speaker on the bare board, iron cold, FPC latched, cell last; not run).
- Updated: DF-102, DF-112
- Gate: CASE
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was FIXED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-239 (assembly.md's order: the panel's tail latched before the cell is plugged) on the RP2350B board.

### DEF-42: The Start/Select pills and caps are not in the BOM
- Found: two-model review (F-40), 2026-10-24
- Symptom: since 2d1124d SW20-SW23 (Start and Select; SW21 and SW23 power on) are 4 mm metal snap domes (HYP 600-415S-260) under Kapton dots and the DS Lite Start/Select strip, but bom.md says 'the Start/Select strip isn't used' and buys no domes, Kapton dots or Start/Select caps.
- Root cause: the BOM was not updated with the design.
- Why missed: nothing checks that every contact class has a bought pill and cap.
- Check added or changed: VER-167 (design_numbers.py bom: FAIL); VER-126 (incoming silicone, caps and domes).
- Updated: DF-167, DF-168
- Gate: CASE
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-230 (every contact's parts bought) on the RP2350B board.

### DEF-43: Plug-in inrush is over USB 2.0's 50 uC
- Found: two-model review (F-24), 2026-10-24, by arithmetic
- Symptom: at 76955d2, C1 10 uF x 5 V = 50 uC alone, and with no cell D5 charged the VSYS capacitors at plug-in (46.8 uF x 4.6 V = 215 uC): 265 uC against USB 2.0's 50 uC, carried as a written deviation in REQ-PWR-13. Since 2d1124d D5 is gone and VSYS sits behind the BQ24073's input switch: VBUS carries C1 alone, 10 uF and 50 uC at 5.0 V.
- Root cause: bulk capacitance behind a diode, not a soft-start switch.
- Why missed: VER-99 was carried as NOT_BUILT behind the hot-plug ODE, though a calculator settles it.
- Check added or changed: VER-99 (design_numbers.py inrush: the VBUS capacitance and the parts between VBUS and VSYS, changed for the BQ24073).
- Updated: REQ-PWR-13
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was CLOSED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-224 on the RP2350B board.

### DEF-44: Requirements that failed by construction
- Board: both
- Found: two-model review (O-22, F-8, F-9, F-11, F-12, O-31), 2026-10-24
- Symptom: REQ-PWR-14's 80%-of-absolute-maximum rule failed every 3.3 V part; REQ-PWR-02's 5.8 V broke REQ-PWR-14's own transient rule; REQ-PWR-01 asked a TP4056 board to charge to done at 4.40 V; REQ-PRD-07 asked for normal-use touch limits under a single fault; REQ-FAB-03's 0.13 mm ring failed every via on the board; REQ-AUD-01's 0.35 W and 4.7 Vpp disagreed.
- Root cause: numbers written from memory or from one part, never checked against the design they govern.
- Why missed: requirements are prose, and none was approved yet.
- Check added or changed: none (requirements wording); VER-161 will check the numbers they quote.
- Updated: REQ-PWR-14, REQ-PWR-02, REQ-PWR-01, REQ-PWR-16, REQ-PRD-07, REQ-THM-01, REQ-FAB-03, REQ-AUD-01
- Gate: ORDER
- Status: FIXED (reworded; all still PROPOSED)

### DEF-45: The backlight sim cannot follow design.py, and its evidence missed loop-made parts
- Found: two-model review (O-5), 2026-10-24
- Symptom: board/sim/backlight_circuit.py hard-codes rs = 2.2, rg = 330, rf = 10k, cf = 1n, rtop = 100k, rbot = 3.9k and gate_pd = 100k, and its docstring says BL_REF comes from GPIO44 (GPIO28). Its values were wrong for the 2.4 in panel (R61 2.4k) from 53e21c8 and are right again since 79de1f7 (R61 3.9k, 56 mA a pin), by coincidence: nothing ties them to design.py.
- Root cause: a sim that copies values, and line-regex freshness.
- Why missed: nobody changed a value to see whether the evidence went stale.
- Check added or changed: VER-05 (covers part:R52-R61, C48-C51, Q5, Q6, U6; backlight.py to assert its defaults against design.PARTS; re-run needed; at EVT since 2026-10-26: the loop's parts are BOM values and the bench sees ringing on unit 1). Since 2026-10-28 VER-05 is paper_checks.py backlight, which reads every value from design.py (and is shown failing with C48/C49 seeded to 10 pF); board/sim/backlight.py still hard-codes R61 3.9k and is no longer any check's evidence.
- Updated: REQ-DSP-05, DF-44, REQ-DSP-09
- Gate: EVT
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was CLOSED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-228 (the backlight's worst case from design.py) on the RP2350B board.

### DEF-46: Bring-up's cell emulation cannot hold 3.50 V with USB in
- Found: two-model review (O-19), 2026-10-24
- Symptom: VER-70 (the unplug at a 3.50 V 'cell' from the bench supply) and VER-67 (a 3.0-4.2 V sweep) both have USB connected; the charger pushes its 242-295 mA into the supply, most supplies cannot sink, so the 'cell' climbs toward 4.2 V and the 3.50 V corner is never tested.
- Root cause: a bench supply treated as a cell.
- Why missed: the procedure was never run.
- Check added or changed: VER-70 and VER-67 revised (CHG_OFF held high, or 10 ohm 5 W across the supply; VBAT recorded at the trigger instant).
- Updated: REQ-PWR-05, DF-34
- Gate: EVT
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was FIXED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-226 on the RP2350B board.

### DEF-47: The register drifted off the design it was meant to describe
- Board: both
- Found: the brief of 2026-10-25 and check_trace.py after vv-plan was rebased onto 684a609: 13 structure errors (covers naming parts that no longer exist: C9, D5, D7, Q2, R6, R39, R40, R47), while the content still described 76955d2. On 2026-10-26 the rebase onto be437f8 failed the default mode at once (7 errors: the baseline and the CLOSED defects' stale checks), as the fix intended.
- Symptom: the register was seeded on 76955d2 (TP4056, ME6211, the 3.5in ILI9488 panel) on 2026-10-24, two days after the design had moved on in other branches (BQ24073 2026-10-22, RT9013 2026-10-22, ER-TFT024IPS-3 2026-10-21, the cell in a hole, 74HC165s, domes). Its order gate, decisions and H rows were about parts that would not be on the ordered board, and the first run's plan estimated half a day to re-seed.
- Root cause: the register was seeded from the checkout at hand, not from a named order candidate; its design baseline covered only design.py, placement.py and case.scad, and a mismatch was reported in the default mode but failed only at a gate.
- Why missed: nothing runs check_trace.py on a design commit (it is not in scripts/check.sh); the default mode printed 'baseline differs' as a report line and exited 0; a rebase that changes the design under the register raised nothing until someone ran the tool.
- Check added or changed: check_trace.py's structure check now fails when README.md's design baseline differs from HEAD or does not name every design path (design.py, placement.py, lib/, battler.kicad_pcb, last_route.ses, build_pcb.py, route.py, silk.py, fab.py, case.scad); VER-129 seeds both faults and passes. Still to do outside hardware/vv: a hardware step in scripts/check.sh that runs check_trace.py (PLAN.md step 0.3).
- Updated: README (baseline at 684a609), every file of the register
- Gate: ORDER
- Status: OPEN (2026-10-29, artifact audit: the register describes the RP2350B board, but design.py on main is still the dropped ESP32-S3 board's (README, Baseline board: esp32s3), so the drift check cannot yet compare them, and VER-188 (stale_names.py) passes leftovers of the dropped boards because it compares against that file: it cannot catch them until the RP2350B design.py replaces the baseline (PLAN.md step 3, NM-05). It is FIXED again when scripts/check.sh runs `python3 hardware/vv/tools/check_trace.py`, a one-line step outside hardware/vv that is Alex's (PLAN.md step 0.3), and the RP2350B design replaces the baseline. If Alex would rather not hold the order for it, its Gate moves to CASE in a register edit his next GATE@ line freezes)

### DEF-48: The charge window's hot edge is 47 C and nothing on the board can move it
- Found: re-seeding for 684a609 (2026-10-25), from pin_review/u2.json pin 1
- Symptom: TH1 (B3950 10k) against the BQ24073's fixed TS thresholds (0.30 V hot, 2.1 V cold at 75 uA) gives 3.5-47.2 C nominal, and 1.4-51.5 C at SLUS810N's limits with TH1's 1% (O-R3). The hot edge is 6.5 C over a typical 45 C cell limit. No resistor network fixes it: a parallel resistor that brings the hot edge to 45 C drags the cold edge to -11.6 C, a series one makes the hot edge hotter (O-R6), so the first run's two DNP footprints could not do what they promised.
- Root cause: the TP4056's window divider (R39, R40) left with it, and the BQ24073's window was taken as given.
- Why missed: REQ-BAT-03 was written for the TP4056; nothing computes the window from design.py (VER-32 not built).
- Check added or changed: VER-32 (design_numbers.py tempwin, the window at SLUS810N's TS limits: FAIL, 1.4-51.5 C at the worst corner); VER-135 (per unit, now with a finger-warm step: TH1 is an NTC).
- Updated: REQ-BAT-03, DF-147, DF-146
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-217 on the RP2350B board.

### DEF-49: The shift registers cut the pill margin from 10 kohm to about 3.7 kohm
- Found: re-seeding for 684a609 (2026-10-25)
- Symptom: REQ-BTN-01 asked each input to read pressed up to 10 kohm of pill resistance, a margin set against the RP2350's 50k internal pull-up. Since 2d1124d the 16 D-pad and A/B/X/Y contacts feed 74HC165 inputs pulled up by 10k arrays; with a VIL of about 0.94 V at 3.3 V a pill reads pressed only under about 3.7 kohm.
- Root cause: the pull-up value was chosen for the shift registers without re-deriving the contact's margin (DEF-02's class: a change made without re-deriving the other margins).
- Why missed: the margin lives in prose; no script computes it from design.py.
- Check added or changed: VER-45 (pill resistance on a bare board) and VER-56; REQ-BTN-01 restated at 3.3 kohm pending PLAN.md decision DEC-24 (RN1-RN4 at 47k, BOM only, restores about 18 kohm).
- Updated: REQ-BTN-01, DF-164
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-230 on the RP2350B board.

### DEF-50: A faulty USB source reaches the regulator's enable ahead of the charger's OVP
- Found: re-seeding for 684a609 (2026-10-25), design_numbers.py vbus
- Symptom: the BQ24073 blocks VBUS from VSYS above 6.6 V (28 V absolute), but PWR_EN is fed from VBUS through R14 and D3 and takes U3's EN (6 V absolute) over its rating above VBUS 6.9 V; EN1 through R43 passes its 7 V above 9.5 V.
- Root cause: the latch's USB wake path and EN1's pull-up were kept from the TP4056 design, where no part had OVP, so nothing flagged them when U2 gained it.
- Why missed: no table of which pins VBUS reaches and at what level.
- Check added or changed: VER-177 (design_numbers.py vbus: FAIL at 6.9 V).
- Updated: REQ-PWR-17, DF-101
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-222 on the RP2350B board.

### DEF-51: A worst-case cell had 0.30 mm to swell under the deck
- Found: re-seeding for 684a609 (2026-10-25), design_numbers.py swell
- Symptom: at 684a609 a 4.5 mm cell's top stood 1.6 mm over the board (cell_top, set to the decks' limit) under a deck whose underside was 1.9 mm over it: 0.30 mm against REQ-BAT-07's 0.4 mm. Since 9cff0d3 (be437f8's placement) the cell lies under the glass's seat-2 half and no button end reaches it: the panel's back is the lowest thing over it, 0.80 mm free.
- Root cause: cell_top was set to what the decks allow, not to the room the cell needs to swell.
- Why missed: the Z stack has no script (VER-20 was not built).
- Check added or changed: VER-182 (design_numbers.py swell, rewritten 2026-10-26 to read what case.scad puts over the tub's pocket instead of assuming a deck: PASS, 0.80 mm).
- Updated: REQ-BAT-07, DF-84
- Gate: CASE
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was CLOSED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-215 on the RP2350B board.

### DEF-52: The trace gate and the fab checks still passed on words, typed results and unsigned approvals
- Board: both
- Found: two-model review of the re-seeded register (O-R1 to O-R5, O-R8, O-R11, O-R12, O-R13, O-R21, F-R1, F-R3, F-R6, F-R13), 2026-10-25
- Symptom: VER-176 and VER-131 passed on the words 'centroid' or '--excellon-separate-th' anywhere in fab.py, comments included, and read no fab output. Uncommitted edits to the register and its evidence opened the gate (VER-138 FAIL to PASS in a scratch clone). An inspection or demonstration passed on any repository file. An approval counted from any commit without a Claude trailer, though every commit has the same author and merges never carry one. R4 at the order gate counted case and docs defects. Occurrence 3 ('proven on similar hardware') sat on 71 rows of parts never built, keeping five severity 9-10 rows at M. At DVT and PVT an EVT inspection still counted. prod_board's gate read no DFMEA row. A commit to the order branch after the baseline raised nothing on vv-plan, and the schematics, the project file and case/export.py were not design paths. A change record's re-run list was hand-picked.
- Root cause: rules written for the faults the first review named, each checked one way only.
- Why missed: seed_faults.py seeded the faults known on 2026-10-24; none of these.
- Check added or changed: VER-129 (new seeded faults for each: a comment in fab.py, an uncommitted register edit, uncommitted evidence, an inspection on a design file, an unsigned and a merged approval line, a CASE defect at the order gate, an O=3 row with no 'Similar:', an EVT first article at the PVT gate, a live row citing a RETIRED id, a design commit on the order branch, a change record that omits a re-run, TP11 moved next to the hole). The drill and CPL checks (design_numbers.py drill and cpl) now run fab.py's own export into a temp dir and read what it writes; they fail on the board's real output, which DEF-32 and DEF-33 hold, not this defect.
- Updated: README rules and baseline; check_trace.py; design_numbers.py; dfmea.csv occurrences; gates.md
- Gate: ORDER
- Status: CLOSED (VER-129 PASS 2026-10-30 at f0d7373: 156 of 156 seeded faults caught, after the policy checks' edits to check_trace.py and seed_faults.py)

### DEF-53: A worst-case cell sits 0.25 mm from the routed FR4 edge on every side
- Found: two-model review (O-R7), 2026-10-25, from case.scad and assembly.md step 5; restated on every side by round 2's sign-off objections (O, F), 2026-10-26
- Symptom: the tub's pocket is 0.25 mm inside the board hole's outline on each side, so a worst-case 52 x 36 cell against any wall of it (assembly.md pushes it to the right one) is 0.25 mm from the FR4 edge: -0.15 mm after JLC's routing (about +-0.2 mm), the print's and the screws' tolerances, at the pushed end and on both long sides; its lead end is 1.35 mm against 1.5. REQ-BAT-08 asks 0.5 mm all round. Widening one end of the hole changes nothing while the pocket follows the hole (seeded in VER-129).
- Root cause: the pocket was sized for the cell's fit, not against REQ-BAT-08; the worst-case cell is a listing's 50 +-2 x 34 +-2, not a spec sheet's.
- Why missed: no check computed the gap; the first fix proposed (widen the right end) was checked against the pushed end only.
- Check added or changed: VER-181 (design_numbers.py cellgap: every side, and what would pass: FAIL).
- Updated: REQ-BAT-08, DF-85 (now H)
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was CLOSED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-214 on the RP2350B board.

### DEF-54: Rows still written for 76955d2 after the re-seed
- Found: two-model review (O-R9, O-R10, F-R2, F-R4, F-R5, F-R7, F-R9, F-R10, F-R11, F-R14, F-R16), 2026-10-25
- Symptom: VER-77 scoped GPIO2 for PWR_HOLD (GPIO44; GPIO2 is P2_START); VER-130 listed J4 as unassembled (a JLC-placed PicoBlade since 7da6a90) and typed 49 lines and 117 placements (44 and 118); VER-166 and REQ-FAB-11 used the old 70.5 x 151.34 mm board; VER-89 inspected an ESOP; VER-91 soldered the speaker; VER-36 and VER-159 named TP4056 and ME6211 entries; VER-148 named a RETIRED row; DF-45 kept 'GPIO44' for BL_PWM; the latch checks covered only seat 1's Start; units.csv counted J6 matings.
- Root cause: the re-seed rewrote rows by part; rows that quote a GPIO, a count or a dimension were read by eye.
- Why missed: the claims script (VER-161) was not built and sat at the case gate; no rule refused a live row that cites a RETIRED one.
- Check added or changed: VER-161 (check_claims.py, moved to ORDER; not built); check_trace's structure rule against a live row citing a RETIRED id outside 'Was:' (seeded in VER-129).
- Updated: VER-28, VER-29, VER-33, VER-36, VER-52, VER-68, VER-77, VER-78, VER-79, VER-89, VER-90, VER-91, VER-130, VER-148, VER-159, VER-166; REQ-ASM-01, REQ-FAB-11, REQ-PWR-08, REQ-PWR-11; DF-45, DF-144
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was FIXED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-239 (the register's rows re-read at a re-seed) on the RP2350B board.

### DEF-55: JLC's order-number marker is not on the board
- Found: two-model review (O-R19), 2026-10-25: `grep -c JLCJLC` on battler.kicad_pcb and silk.py gives 0 and 0
- Symptom: REQ-FAB-05 asks for 'JLCJLCJLCJLC' on the back silk so JLC prints its number there; it is not in the design (still absent at 71737ae: `grep -c JLCJLC` on battler.kicad_pcb and silk.py gives 0 and 0), so JLC places it anywhere (DF-125: on a comb). Since 40281b8 silk.py moves the board's name and BOARD_REV text out of the cell's hole onto clear spots (DEF-57 CLOSED); the marker was not added with them.
- Root cause: REQ-FAB-05 was written as if the marker existed.
- Why missed: no check read the silk for it.
- Check added or changed: VER-132 (k) (the marker on B.Silkscreen, or JLC's paid removal option in the release record; built 2026-10-27, FAIL at 71737ae: absent among the 20 back-silk texts).
- Updated: REQ-FAB-05, DF-125
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was CLOSED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-250 (JLC's order-number marker on the back silk) on the RP2350B board.

### DEF-56: Copper within 0.20 mm of the cell's and the speaker's routed edges
- Found: re-seeding for be437f8 (2026-10-26), building VER-18 (design_numbers.py edge)
- Symptom: the GND pours on both layers come 0.20 mm from the cell's hole (54.0 x 37.0 mm) and the speaker's hole (11.0 x 16.0 mm, new in 9cff0d3); J2's mounting pad is 0.275 mm from the board's left edge; J2's courtyard runs 0.22 mm and J4's 0.12 mm past the outline. REQ-FAB-03 asks 0.30 mm of copper to a routed edge (JLC's figure as this register quotes it; VER-17 confirms it on the dated page). The speaker's hole also leaves a 1.00 mm FR4 bar to the right edge (DF-178).
- Root cause: the board's rules allow 0.2 mm (battler.kicad_pro min_copper_edge_clearance), and the PicoBlades were placed at the edges to clear the glass.
- Why missed: KiCad's DRC checks its own 0.2 mm; VER-18 (the edge check) was NOT_BUILT.
- Check added or changed: VER-18 (design_numbers.py edge: copper, courtyards and silk against every routed edge; since O-R2 each distance printed beside its worst case); VER-17 (JLC's dated page decides which of the two the rule applies to).
- Updated: REQ-FAB-03, DF-179, DF-178
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-236 on the RP2350B board.

### DEF-57: The board's name, BOARD_REV and build text sit in the cell's hole
- Found: re-seeding for be437f8 (2026-10-26), building VER-18 (design_numbers.py edge reads the silk texts)
- Symptom: silk.py writes 'TWO-PLAYER HANDHELD  REF-A', 'REV 1' (design.BOARD_REV) and '2 LAYERS  1.6 MM  ENIG' at (100, 57.6), (100, 60.6) and (100, 63.2) on the back, 'between seat 2's clusters' as the old layout had it; since 9cff0d3 the cell's hole runs y 57.0-94.0 there, so all three are routed away. 'BOOT' and 'RST' are written at (-31.0, -32.5) and (-32.0, -31.6), case coordinates used as KiCad's: off the board. 'SPEAKER' and '15x10 8R' sit in the speaker's hole. The FAI reads BOARD_REV from the silk (VER-109), and a BOM-only change bumps it (README, R6): with no printed revision the boards cannot be told apart.
- Root cause: fixed text positions in silk.py that the re-placement did not move; check_pcb.py treats silk_edge_clearance as cosmetic (DEF-36).
- Why missed: KiCad's DRC reported three silk_edge_clearance warnings and check_pcb passed them; silk.py's own clash list is printed, not failed.
- Check added or changed: VER-18 (every silk text wholly on the board; FAIL); VER-109 (the FAI reads BOARD_REV).
- Updated: REQ-FAB-05, DF-125
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was FIXED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-236 on the RP2350B board.

### DEF-58: The cell's lead end is held by a piece of tape, not by the pocket
- Found: two-model review (O-R8), 2026-10-26, from case.scad and assembly.md step 5
- Symptom: assembly.md step 5 tapes the cell pushed right in the tub's pocket. Nothing in the design holds it there: an unpushed cell rests against the pocket's left wall, where its lead end has only the pocket's fit (0.25 mm today, -0.15 mm after JLC's routing, the print's and the screws' tolerances) against REQ-BAT-08's 1.5 mm. DEC-27(a)'s pocket (0.9 mm inside the hole) would give 0.5 mm there, still short.
- Root cause: the pocket is sized for the cell's fit, and the pushed position is an assembly instruction rather than a geometry.
- Why missed: cellgap computed the pushed end only, because the drawn model shows the cell pushed.
- Check added or changed: VER-181 (design_numbers.py cellgap, now printing and failing on the unpushed lead end as well: FAIL).
- Updated: REQ-BAT-08, DF-04, DF-85
- Gate: CASE
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was CLOSED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-214 on the RP2350B board.

### DEF-59: case.scad's panel numbers are not the drawing's
- Found: two-model review (O-R6), 2026-10-26, from the datasheet downloaded 2026-10-26 to the main checkout (untracked)
- Symptom: the ER-TFT035IPS-6 datasheet (REV 1.0 of Dec-15-2022; its outline drawing drawn Dec-20-22, revision A of Feb-22-2022, er-tft035ips-6.txt:42, :204, :324) gives the outline 54.56 x 82.84 +-0.2 mm, the thickness 2.1 to 2.15 +-0.15 mm and the FPC neck 4.47 mm; case.scad uses 54.50 x 83.00 x 2.45 mm and a 4.59 mm neck, and cites a "2.35 +-0.1 (table 2.3)" that this copy does not hold (the table says 2.1, line 175; the side view 2.15 +-0.15, line 218). The fold makes the outline an estimate either way, so the numbers set J3's place and the 1.05 mm limit under the FPC's body.
- Root cause: the numbers were scaled from the old 3.5 in panel's drawing when the datasheet could not be downloaded; nothing compared them when it could.
- Why missed: no check compares case.scad's panel numbers with the datasheet; the ESTIMATE ledger (VER-152) is not built.
- Check added or changed: VER-152 (the ESTIMATE ledger: the panel's outline, active area, thickness and FPC neck, body, tail and tip range each need a measured value and an evidence file; not built); VER-136 (the real FPC's reach and seating at J3).
- Updated: REQ-MEC-11, DF-61, DF-123
- Gate: CASE
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was FIXED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-229 on the RP2350B board.

### DEF-60: A GND via was moved by hand in the board file
- Found: two-model review (O-R10), 2026-10-26, from the git log at 62b01f2
- Symptom: commit 62b01f2 moved one GND via by hand in battler.kicad_pcb. README.md says the KiCad files are never written by hand, and no DEF or DF row covers it. VER-143's release run runs fab.py twice on the committed board file and never rebuilds it, so the committed copper and the rebuilt copper are never compared: every step-1 change (DEC-28's silk.py, DEC-32's pours) rebuilds and brings the DRC error back.
- Root cause: the hand edit was made to get a DRC error out of the way, outside the scripts that generate the board.
- Why missed: no check compares the committed board with the one the scripts build.
- Check added or changed: VER-143 (rebuild from design.py at the tag, re-run the DRC and diff the rebuilt copper against the committed file with the dated lines stripped; not built).
- Updated: DF-179
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was FIXED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-237 on the RP2350B board.

### DEF-61: Two pin-review findings that set the BOM are unanswered
- Found: two-model review (F-R2(d)), 2026-10-26, split from DEF-26
- Symptom: the panel's VCI and VDDI take 3V3, up to 3.366 V (the RT9013's +2%), against the module's 3.3 V maximum (er-tft035ips-6.txt:747-748; its absolute maximum is 4.6 V, line 734, and the ILI9488's own VCI range runs to 3.6 V); and the copied core has one 4.7 uF on +1V1 where Raspberry Pi's minimal design may have two (u4.json pin 10). Both are answered by a BOM change or a written acceptance, so they belong before the order, while DEF-26's tooling does not.
- Root cause: the pin reviews recorded the findings and nothing made anyone answer them.
- Why missed: check_pins.py does not read the ratings (DEF-26).
- Check added or changed: VER-34 (the datasheet audit quotes the panel's table and answers VCI and VDDI in writing); VER-23 (the overlay against RP-010329 answers the +1V1 capacitor in writing); VER-198 (2026-10-27: the application-circuit script reads er-tft035ips-6.txt:747 and fails on the 3.366 V until Alex accepts it or the supply changes).
- Updated: REQ-PWR-14, DF-96
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-227 on the RP2350B board.

### DEF-62: The cell's pocket has no fit for a worst-case cell
- Found: round 3's sign-off (O-S1), 2026-10-26, against DEC-27's geometry; confirmed at 71737ae on 2026-10-27 by design_numbers.py cellgap
- Symptom: case.scad at 71737ae draws the tub's pocket 52.0 x 36.0 mm (the 37.8 x 54.8 mm hole less 0.9 mm on the long sides and the seat-1 end and 1.9 mm at the lead end), exactly the worst-case 52 x 36 cell. A pocket printed 0.1 mm small (the same print tolerance the gap's 0.40 mm stack counts) does not take a cell at its spec maximum, and a pouch pressed into a tight pocket is stressed. Every FR4 gap is met only because the pocket is the cell's size.
- Root cause: the pocket was sized so the gap stack passes with no margin, and the fit was never a number in REQ-BAT-08.
- Why missed: cellgap checked the gaps, not the fit; O-S1 raised it at sign-off.
- Check added or changed: VER-181 (design_numbers.py cellgap now checks the fit: the pocket 0.4 mm longer and wider than the cell, 0.1 mm print and 0.1 mm fit a side; FAIL at 71737ae, 0.00 mm spare; seeded in VER-129 with a 51.7 mm cell, and its 51.6 x 35.6 mm control passes); REQ-BAT-08's fit clause (PROPOSED).
- Updated: REQ-BAT-08, DF-85, DF-04
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was CLOSED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-214 on the RP2350B board.

### DEF-63: U10 went on the board without its datasheet
- Found: re-seeding for 71737ae (2026-10-27), from pin_review/u10.json
- Symptom: the board spin added U10 (MCP6001T-I/OT, C116490) to buffer VBAT/2 for GPIO47 (DEF-40). Its datasheet is not in hardware/ref_design/datasheets: u10.json says LCSC's download gave an HTML page, so the pin table is KiCad's MCP6L91T-EOT symbol and the ratings are the LCSC listing's typical figures, marked UNVERIFIED. A wrong pinout would misread VBAT on all five boards; the input's limits with VDD at 0 V (the device off, VIN+ at VBAT/2 behind 235 kohm, about 6.8 uA into +3V3) are not read; and the waiver on u4.json pin 58 was rewritten for U10 by the design session, not by Alex (DEC-29).
- Root cause: a part added in a board spin with the datasheet download failing, and nothing stopping the spin on it.
- Why missed: check_pins.py does not demand a datasheet file on disk (DEF-26); VER-34, which would, is not run.
- Check added or changed: VER-34 (the audit now names U10, TH1's TCTR0805 and J4's SH header first, and replaces design_numbers.py's stand-ins for U10's bias, output resistance and drift); VER-138 prints the stand-ins it uses; VER-159 lists U10's input as a back-power path.
- Updated: DF-182, DF-98, DF-138, REQ-BAT-12, REQ-PWR-07
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was FIXED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-211 on the RP2350B board.

### DEF-64: The speaker's plug no longer fits J4, and its replacement is not bought
- Found: re-seeding for 71737ae (2026-10-27), from bom.md's speaker line
- Symptom: J4 is a JST SH SM02B-SRSS-TB since 71737ae (DF-170), but the GSPK151035PN comes with a 1.25 mm plug. bom.md says its leads need a 1.0 mm SH plug, SHR-02V-S, fitted by hand, not yet ordered. Fitting it means cutting the plug off and crimping SSH-003T-P0.2 contacts (a fine crimp that wants its own tool) or buying pre-crimped SH leads and joining them to the speaker's, by hand, on every unit; assembly.md has no such step, and the speaker's 35 mm leads must still reach J4 from its hole.
- Root cause: J4 changed to key it apart from the cell's plug, and the speaker is sold with one plug only.
- Why missed: the change was made in the board spin; the register's speaker check (VER-128) still named the PicoBlade sample.
- Check added or changed: VER-128 (the SH plug latched in an SM02B-SRSS-TB sample, the cell's plug refused by it; its needs list the SH parts and a crimper); DF-183; VER-202 (design_numbers.py spkplug, F-R2: the speaker's plug and leads against J4 and J2 from bom.md's row and the routed board; FAIL while the row says the SH plug is not yet ordered).
- Updated: DF-183, DF-170, DF-118, REQ-MEC-11
- Gate: CASE
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-231 on the RP2350B board.

### DEF-65: The swelling check used the old decks' line, not the one-piece top's ceiling
- Found: re-seeding for 71737ae (2026-10-27): design_numbers.py swell failed at 0.30 mm when the cell's pocket reached case y 46.1
- Symptom: VER-182 put the ends' 1.9 mm ceiling (z_deck_under) over everything past joint_y - joint_w (43.49), the decks' line before the one-piece top (be437f8). With the rotated hole the pocket's seat-2 end reaches y 46.1 and the check read 0.30 mm free against 0.40. case.scad's own ceiling (uni_ceil) falls to z_deck_under only at the slope's foot, y 48.07; between the panel's frame and the foot the slope's inside stands 3.40 mm over the board, 1.80 mm over the cell.
- Root cause: the check copied the case's geometry instead of reading its ceiling function, and the case changed under it.
- Why missed: at be437f8 the pocket stopped short of 43.49, so the stale line never mattered.
- Check added or changed: VER-182 (design_numbers.py swell now evaluates case.scad's degree trigonometry and its uni_lim and uni_foot, and prints the slope's ceiling over the cell).
- Updated: REQ-BAT-07
- Gate: CASE
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was CLOSED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-215 on the RP2350B board.

### DEF-66: J2's polarity mark overlaps its label, and the PANEL label sits on U4's pads
- Found: re-seeding for 71737ae (2026-10-27), reading VER-04's six DRC warnings by hand (DEF-36)
- Symptom: kicad-cli's DRC at 71737ae reports silk_overlap between J2's '+' (132.2, 73.8) and its 'BATT' label (131.6, 74.3) on F.Silkscreen, and silk_over_copper for the 'PANEL' label (80.7, 115.7) over U4's pads 45, 46 and 47 (LCD_MISO, LCD_DC, LCD_SCK_MCU). The '+' is what REQ-BAT-05.R's per-cell polarity check reads (red lead on J2 pin 1, silk '+'); JLC clips silk off the pads (the gerbers subtract the mask), so 'PANEL' prints in part. The two silk_edge_clearance warnings are J1's outline over its tab (by design).
- Root cause: silk.py places labels by a clash list it prints and does not fail (DEF-57's cause), and check_pcb.py passes every silk warning as cosmetic (DEF-36).
- Why missed: VER-18 checks that each text is on the board, not that it is legible.
- Check added or changed: VER-04 (its evidence lists the six warnings; this re-seed read them); VER-132 (the fab content check: every silk text clear of other silk and of pads; not built).
- Updated: REQ-BAT-05.R, DF-125
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-250 (no silk on a pad) on the RP2350B board.

### DEF-67: Three of U4's supply pins have no capacitor near them, and the far DVDD pin has no 4.7 uF
- Found: VER-197 (paper_checks.py core), built 2026-10-27 for the coverage grid's class 4 / mcu cell, at 71737ae
- Symptom: on the routed board the nearest capacitor to DVDD pin 51 is C25 at 7.2 mm; C22, the 100 nF the schematic gives pin 51 (sch_sheets.py SERVES), sits about 15 mm away. IOVDD pins 41 and 50 have no capacitor within 5 mm (7.4 and 6.7 mm); their own C17 and C15 sit about 10 mm away. DVDD pin 32, the one furthest from the regulator, has C21 (100 nF, 3.0 mm) and no 4.7 uF: rp2350.txt:32665-32675 asks 4.7 uF there, and 6.3.8.1 recommends a second 4.7 uF on the core supply (DF-94). The other IOVDD, ADC_AVDD, USB_OTP_VDD and QSPI_IOVDD pins each have one within 3.6 mm.
- Root cause: placement.py places C13-C20, C22 and C23 by hand ('the screw, J2 and the folded FPC', placement.py:226-230); C15, C17 and C22 landed in a cluster near (83, 127), outside the core, and nothing measures a decoupler's distance to its pin.
- Why missed: DRC and the netlist check connectivity, not distance; VER-23 (the overlay against RP-010329) is not built and its files are not on disk.
- Check added or changed: VER-204 (the module's decoupling; Was: VER-197)
- Updated: DF-94, REQ-MCU-09
- Gate: ORDER
- Status: RETIRED in 39cc07f: the RP2354B's supply pins left; the module's 22 uF far from its pin is DEF-82

### DEF-68: The cell's current runs through a 0.15 mm track, and VSYS reaches U3 through 229 mohm
- Found: VER-13 (paper_checks.py copper), built 2026-10-27, at 71737ae
- Symptom: CELL_P, from J2 pin 1 to JP5, is 13.5 mm of 0.15 mm track: 44 mohm, and at the row's 0.75 A it carries 124% of IPC-2221's 0.60 A for a 10 C rise. JP5 to U2's BAT is 52 mohm of 0.30 mm track. U2's OUT reaches U3's input through 229 mohm (about 116 mm of 0.25 mm), JP3 through 87 mohm. Copper alone drops 115 mV from the cell to the worst VSYS branch against the 30 mV limit, and 62.5 mV from VBUS against 50 mV. At the realistic peak on the cell (about 0.35 A) CELL_P is within its 10 C chart and drops about 15 mV; the drop to U3 adds to its dropout near the cutoff (VER-138).
- Root cause: build_pcb.py's PWR net class (0.3 mm, build_pcb.py:56-58) lists VSYS, VBAT and VBUS but not /USB_Charger/CELL_P or /Audio/AMP_VDD (it still lists /USB_Charger/VBUS_D5, a net that left the design), so both route at the default 0.15 mm (build_pcb.py:62); VSYS's long run to the left strip is 0.25-0.30 mm.
- Why missed: no copper resistance check existed (VER-13 was NOT_BUILT); DRC only checks minimum widths.
- Check added or changed: VER-13 (built; --seed narrows CELL_P to 0.05 mm); VER-201 (design_numbers.py netclass, O-R8: build_pcb.py's class lists against the board's nets and the nets that carry 0.25 A or more; FAIL on VBUS_D5's stale entry, CELL_P and AMP_VDD)
- Updated: REQ-BAT-12, DF-138
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-221 on the RP2350B board.

### DEF-69: A cell cut off at 3.50 V can reach its PCM's under-voltage trip in under a month
- Found: VER-103 (paper_checks.py shelf), built 2026-10-27, at 71737ae
- Symptom: with the device off the board draws about 27 uA at 25 C (U2 6.5, R17 into U10's clamp 7.9, U7 shut down 8.4 as twice its typical, U3 1.5, three FETs' leakage 3: stand-ins where the datasheets are not on disk) and 40 uA at 45 C. With the cell's stand-ins (700 mAh, 5-10% of it below 3.50 V, a PCM's 3-6 uA, 1-3% a month of self-discharge) the cell reaches the UVP 2.5 months after the cutoff at nominal but 0.8 months at the worst case, and 15.5 months from full against 24. The board's drain is about half the loss, the cell's own self-discharge the other half.
- Root cause: REQ-BAT-10's 2-month and 2-year numbers were set before the budget was added up; U10's unpowered input (R17) and U7's shutdown on the live VSYS each add 4-8 uA.
- Why missed: VER-103 was NOT_BUILT.
- Check added or changed: VER-103 (built; --seed stiffens the divider and must fail)
- Updated: REQ-BAT-10, REQ-PWR-07
- Gate: EVT
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-223 on the RP2350B board.

### DEF-70: C24, the core regulator's input capacitor, is under the RP2350's 4.7 uF minimum at its own tolerance
- Found: VER-30 (paper_checks.py mlccbias), built 2026-10-27, at 71737ae
- Symptom: C24 is CL05A475MP5NRNC, 4.7 uF +-20% (the M in its part number): 3.76 uF at its low tolerance before any DC bias, against VREG_VIN's 'at least 4.7 uF' (rp2350.txt:33598). C25, the same part as the regulator's output, meets 'COUT 4.7 uF +-20%' (33603) only before DC bias: at -20% it has nothing left for the loss at 1.1 V. Raspberry Pi's minimal design, which the core copies, uses a 4.7 uF 0402 in both places (VER-23 confirms, its files not on disk).
- Root cause: the copied core's values were kept without reading the datasheet's limits against the parts' tolerance.
- Why missed: VER-30 (MLCC derating) was NOT_BUILT; VER-23 is not built.
- Check added or changed: VER-30
- Updated: DF-94
- Gate: ORDER
- Status: RETIRED in 39cc07f: C24 left with the RP2354B's core regulator; a capacitor short at its DC bias at 39cc07f is DEF-83 (C3)

### DEF-71: Three firmware rules the RP2350's errata need are not in pinmap.md
- Found: VER-16 (analysis/errata_rp2350.md) and VER-196 (paper_checks.py ampstate), 2026-10-27, at 71737ae
- Symptom: on an A2 part, E9 can hold AMP_EN near 2.2 V behind R37's 100k if firmware leaves the pin an input with its input buffer on, which turns Q3 on and the amplifier with it; a watchdog reset resets the pads only when PSM WDSEL includes RESETS (all 0 at reset, rp2350.txt:36739-36791), which the SDK's watchdog_enable() sets; E16 can corrupt OTP reads on A2 if OTP is used. pinmap.md's Power control section names none of these.
- Root cause: the errata table (VER-16) was not written, so their firmware consequences were never collected.
- Why missed: VER-16 was NOT_BUILT; VER-196 had not run.
- Check added or changed: VER-16, VER-196 (both now name the rules); VER-39's doc sweep reads pinmap.md for them
- Updated: DF-94, REQ-PWR-09
- Gate: EVT
- Status: RETIRED in 39cc07f: the RP2350's errata left; the ESP32-S3's are VER-214's

### DEF-72: U2's footprint puts the cell's pins closer to the GND pad than TI's land pattern
- Found: VER-125 (check_land.py), built 2026-10-27, at 71737ae
- Symptom: U2 uses KiCad's VQFN-16-1EP_3x3mm_P0.5mm_EP1.68x1.68mm: its pads are 0.82 x 0.25 mm at a 2.92 mm span, so each pad's inner end sits 0.21 mm from the 1.68 mm exposed pad. TI's land pattern for the RGT0016C (bq24074.txt:3061-3100) has 0.60 x 0.24 mm pads at a 2.80 mm span: 0.26 mm. On U2 the exposed pad is GND and pins 2 and 3 are BAT, so a bridge there shorts the cell on the board (DF-148, severity 10). The count, pitch, pin 1 and the exposed pad match.
- Root cause: design.py takes KiCad's generic IPC-7351 VQFN footprint, whose pads run 0.22 mm longer, inward as well as out, than the maker's example; nobody compared it with TI's page.
- Why missed: VER-125 was NOT_BUILT; DRC passes 0.21 mm.
- Check added or changed: VER-125 (built; --seed shrinks the exposed pad and must fail)
- Updated: DF-148
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was FIXED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-220 on the RP2350B board.

### DEF-73: VBUS on J1's CC or data pins overstresses the CC parts above 3.9 V and the whole 3V3 domain above 4 V
- Found: VER-195 (circuit_checks.py usbfault), built 2026-10-27, at 71737ae
- Symptom: with a broken source's voltage on J1's CC or data pins (a CC or data line shorted to VBUS in a cable or charger; a solder bridge on J1's row puts VBUS on CC2 at B4-B5 and A9-B5, bridges.csv): GPIO46 and GPIO45 (CC1_SENSE, CC2_SENSE, not 5 V tolerant, rp2350.txt:100894-100900) pass IOVDD + 0.5 V (101043) from about 3.9 V behind R3 and R4's 10k, 1.6 mA into each clamp at 20 V; R1 and R2 (5.1k 0402, 62.5 mW) pass their rating at 17.9 V (78 mW at 20 V); on D+ or D- U1's steering diodes push the fault into +3V3 from about 4.0 V, and the RT9013 cannot sink, so the RP2350, the panel and everything on 3V3 go over 3.63 V. The pad's diode and U1's onset are stand-ins (the SRV05-4's datasheet is not on disk).
- Root cause: the CC sense taps and the data lines were sized for USB 2.0's 5 V world; nothing bounds a 20 V fault on them. VER-177 (DEF-50) covered VBUS itself, not the pins beside it.
- Why missed: VER-195 was NOT_BUILT.
- Check added or changed: VER-195 (built; FAIL at 71737ae, so no seed is needed to show it can fail)
- Updated: DF-101, DF-51
- Gate: EVT
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-222 on the RP2350B board.

### DEF-74: The F.Cu GND pour comes 2.875 mm from every comb's centre, inside REQ-BTN-04's 2.9 mm
- Found: VER-132 (layout_checks.py fab, item (b)), built 2026-10-27, at 71737ae
- Symptom: on all 16 membrane contacts (SW1-SW16) the top-side GND pour's nearest edge is 2.875 mm from the contact's centre (measured on pcbnew's filled zone), 0.025 mm inside the 2.9 mm REQ-BTN-04 keeps clear of everything but the contact's own stubs. No via, no foreign track and no paste or silk is inside; the 4.8 mm mask openings are whole. The pour is flat copper under mask, outside the 3.5 mm pill's 1.75 mm radius, so the pill is not lifted (the reason for the 2.9 mm was a tented via).
- Root cause: the pour's clearance round the comb's pads gives 2.875 mm; nothing in build_pcb.py states the 2.9 mm as a keep-out.
- Why missed: VER-132 was NOT_BUILT; VER-162 checks the contacts' own copper, not the pour round them.
- Check added or changed: VER-132 (built; its seed puts paste in SW7's circle and must fail)
- Updated: REQ-BTN-04
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was CLOSED there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-230 on the RP2350B board.

### DEF-75: The fast nets cross gaps in the ground under them, and D+ and D- differ by 3.6 mm
- Found: VER-21 (layout_checks.py ret) and VER-108 (layout_checks.py usbpair), built 2026-10-27, at 71737ae
- Symptom: sampled every 0.1 mm against the GND fill on the other layer, the longest runs with no ground under them are USB_DP and USB_DM 38.7 mm (the pair runs stacked, D+ on B.Cu over D- on F.Cu, along the cell's hole from (80.65, 107.3), so each sees the other and no ground), SR_CLK 6.9 mm, SR_PL 6.0, AUDIO_PWM 5.5, LCD_SCK 3.4, LCD_MOSI 3.1, SR_DATA 2.5, LCD_SCK_MCU 2.25 and LCD_MOSI_MCU 2.15 mm, against VER-21's 2 mm. The USB pair's stubs pass (0.98 mm at most against 3.5) but D+ is 70.9 mm and D- 67.2 mm, 3.64 mm apart against VER-108's 2 mm. For a sense of size: full-speed USB's bit is 83 ns and its edges 4-20 ns, so 3.64 mm (about 24 ps) and a 39 mm run (well under a tenth of an edge's length) do not break the link on their own; the gaps raise emissions and crosstalk, which EVT's near-field scan (VER-96) and the SR_CLK test (VER-179) measure.
- Root cause: on two layers with the cell's hole and the panel's strip taking room, the router kept no ground under these nets, and nothing checked it; the 2 mm limits are generic, written before the route.
- Why missed: VER-21 and VER-108 were NOT_BUILT.
- Check added or changed: VER-21, VER-108 (built; their seeds drop the ground fill and lengthen a stub, and must fail)
- Updated: DF-51, DF-184
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-261 on the RP2350B board.

### DEF-76: pinmap.md states no clock limits for the panel's SPI or the buttons' shift registers
- Found: VER-199 (circuit_checks.py timing), built 2026-10-27, at 71737ae
- Symptom: the hardware closes its timing: the panel's SPI write at 18.75 MHz (clk_peri / 8) has 3.3 ns over the ILI9488's 50 ns write cycle (ili9488.txt:16258; at 20 MHz it would have none), 19.2 ns of SCL high and low against 10 ns, and 16.6 ns of set-up against 10 ns; SR_CLK's edge is 16.9 ns. But the limits the firmware must keep are written nowhere: a panel read (LCD_MISO) needs 6.25 MHz or less (trc 150 ns, 16262), the write 18.75 MHz or less (never 20 MHz), and SR_CLK 5.44 MHz or less with SR_PL low 80 ns and high 80 ns before the first edge (the 74HC165's 2 V figures, read as the bound at 3.3 V: sn74hc165.txt:371-377, 472, 488).
- Root cause: pinmap.md describes the pins, not the buses' timing; the clocks were left to the firmware.
- Why missed: VER-199 is new (2026-10-27), the paper check class 12 lacked for the mcu.
- Check added or changed: VER-199 (built; its seed puts 1k in LCD_SCK and must fail)
- Updated: REQ-DSP-11, DF-180
- Gate: EVT
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-206 (the panel bus's clock limits) on the RP2350B board.

### DEF-77: J5's mouth faces the side wall 2 mm away: no SWD cable with the board in the tub
- Found: VER-165 (case_checks.py mouths), built 2026-10-27, at 71737ae
- Symptom: J5 (the SWD JST SH, rotated -90 at (67.9, 105.0)) opens towards the board's left edge: its body ends at x 65.28 and the tub's inner wall stands at 63.30, so a 6 mm SH plug and cable box crosses the wall by 4.02 mm. The plug fits only with the board out of the tub. J2's and J4's plug boxes (case.scad's j2_plug, j4_plug) are clear of parts over 1 mm and inside the walls; no pinhole is inside a boss (the nearest 6.33 mm).
- Root cause: J5 was placed for routing with its mouth towards the nearest edge; REQ-MEC-11 asks for a clear mouth, and no check read it.
- Why missed: VER-165 was NOT_BUILT.
- Check added or changed: VER-165 (built; its seed makes a mouth box 8 mm deep and must fail)
- Updated: REQ-MEC-11
- Gate: CASE
- Status: RETIRED in 39cc07f: J5 left with SWD

### DEF-78: Seven case ESTIMATEs and the cell's size set copper and none is measured
- Found: VER-152 (case_checks.py estimates), built 2026-10-27, at 71737ae
- Symptom: nudged through openscad -D, seven ESTIMATE-marked values move the board's inputs in positions.json: the panel FPC's body, tail, tip range and slack (fpc_body, fpc_tail, tip_range, fpc_slack: J3's place, fpc_zones, j3_zone), fpc_low (the height allowed under the FPC: today the tallest part there is C38 at 0.50 mm), ss_pitch (SW20-SW23's places and the ribs' cut-outs) and ss_mem (the Start/Select pieces). The cell's 52 x 36 x 4.5 sets the hole and waits for VER-37's spec sheet. None has a measurement or a spec-sheet value in hardware/vv/evidence. Eight other ESTIMATEs (the caps, the D-pad's peg, the domes' skirt, ss_plunger_gap, the FPC's neck) move nothing the board reads: they are the case gate's.
- Root cause: the panel's FPC numbers are the old design's, scaled from the drawing's views (case.scad:47); the Start/Select strip has not been measured.
- Why missed: VER-152 was NOT_BUILT.
- Check added or changed: VER-152 (built; its seed adds an unmeasured copper-setting estimate and must fail)
- Updated: DF-61
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-229 on the RP2350B board.

### DEF-79: bom.md's order line and its cost table are not the board being ordered
- Found: the review's O-R3 and F-R4 (2026-10-27), reading hardware/ref_design/docs/bom.md at 71737ae
- Symptom: bom.md's bare-board row asks for "2 layers, 72.4 x 151.64 mm (72.5 x 151.74 over the outline)". At 71737ae the board is 72.4 x 151.48 mm, 73.4 mm wide across J1's 1.0 mm tab (Edge.Cuts x 63.8 to 136.2, the tab to 137.2, y 24.26 to 175.74). JLC cuts the board from the Gerbers, so the typed size sets the quote and JLC's check against the files, not the board; the case's pocket is board_l plus 2 x board_clear (0.5 mm), so the fit is not at stake. bom.md's feeder-loading row says "12 extended part numbers (... J4 reuses J2's part number)", $36.84, and a $53.35 total a board: since 71737ae J4 is the JST SH SM02B-SRSS-TB (C160402), its own BOM line, so the count, the feeder cost and the total are not this BOM's (48 BOM lines and 120 placements, VER-130). The TP4056 still sits among the priciest parts (DEF-21 (f)).
- Root cause: bom.md's order and factory blocks are typed by hand and were not re-derived when the panel change shortened the board or when J4 changed.
- Why missed: no check reads the order line against Edge.Cuts; VER-39's sweep did not read the cost table's part list.
- Check added or changed: VER-39 (its scope now names bom.md's order line, counts and cost rows, and case/*.md; to re-run); VER-130 (the BOM and CPL counts against design.py); VER-117 (the re-quote replaces the snapshot)
- Updated: REQ-FAB-11, REQ-PRD-09
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-239 on the RP2350B board.

### DEF-80: The flex antenna sits within 15 mm of the panel, the cell and a screw
- Found: VER-205 (layout_checks.py antenna), built 2026-10-28, at 39cc07f
- Symptom: the antenna's plate (x 64.5-71, y 62.1-107.9, docs/living/esp32s3_build.md) is 1.3 mm from the panel's glass outline, 10.6 mm from the cell's pocket, and H2's M2 screw boss lies over its top end; the guidelines give 15 mm in all directions for the module's own PCB antenna (esp32-s3-hw-design-guidelines.txt:1299), and the flex antenna's own keep-out is not on disk. The board's rule area is clean: 0 copper inside it on both layers.
- Root cause: the WROOM-1U's antenna was put in the only free strip of the left bezel, between the board's edge and the panel, with no maker's figure to place it by.
- Why missed: no check read the antenna until this re-seed.
- Check added or changed: VER-205 (built; its seed puts a pour inside the keep-out and must fail); VER-207 (range in the closed case, unit 1)
- Updated: REQ-RF-01, REQ-RF-02, REQ-RF-03, DF-192
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries to no DF row: the RP2350B board has no radio (DESIGN.md:77, no Wi-Fi).

### DEF-81: The new parts' JLC class and stock were never checked
- Found: docs/living/esp32s3_build.md 'Not verified' and docs/living/esp32s3_pinmap_diff.md, read at this re-seed (2026-10-28)
- Symptom: the module (C3013946), the TLV75733P (C485517), the DM3AT (C114218) and the TPS850 (C2759975) went on the board with no check of JLC's class (Basic, Extended), stock or minimum order; the board session's parts database was not reachable.
- Root cause: parts chosen offline.
- Why missed: VER-41 runs on order day only.
- Check added or changed: VER-41 (order-day stock and class, the four first); VER-14 (LCSC identity); VER-186 (check_lcsc.py over Konnect's 2026-07-13 snapshot, 2026-10-28: the four are in JLC's database as designed, all Extended, stock 18078, 30038, 9158 and 2979 against 33 for 30 boards; a snapshot, so VER-41 still decides on order day)
- Updated: REQ-FAB-10, DF-69
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-238 on the RP2350B board.

### DEF-82: The module's 22 uF sits 23.6 mm from its 3V3 pin
- Found: VER-204 (paper_checks.py module), built 2026-10-28, at 39cc07f
- Symptom: C10 (22 uF, the module's bulk in Espressif's peripheral schematic, esp32-s3-wroom-1.txt:2875) has its pad 23.6 mm from pin 2; C11 (100 nF) is 3.4 mm away. The build note said about 10 mm.
- Root cause: C10 placed above the module's top row where place_new.py found room.
- Why missed: no check read the module's decoupling before this re-seed.
- Check added or changed: VER-204 (built; its seed makes C11 1 uF and must fail); VER-208 (3V3 at C11 in TX bursts, unit 1)
- Updated: REQ-MCU-09, DF-187
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-210 on the RP2350B board.

### DEF-83: C3, U2's BAT capacitor, is under its 4.7 uF minimum at 4.2 V
- Found: VER-30 (paper_checks.py mlccbias), rewritten 2026-10-28 with Samsung's DC-bias curves on disk
- Symptom: C3 (CL10A106KP8NNNC, 10 uF 10 V X5R 0603) is 9.0 uF at its -10% and loses 65.4% at 4.2 V (samsung_cl10a106kp8nnnc_dcbias.csv): 3.11 uF against SLUS810N's 'Bypass BAT to VSS with a 4.7-uF to 47-uF ceramic capacitor' (bq24074.txt:501). It was the same part at 71737ae; the curve was not on disk then.
- Root cause: a 10 V 0603 X5R chosen by its nominal value.
- Why missed: VER-30 was BLOCKED on the curves.
- Check added or changed: VER-30 (now reads the curves; BLOCKED only on C10's, CL21A226MAQNNNE)
- Updated: DF-136
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-235 on the RP2350B board.

### DEF-84: U4's thermal pad is 3.9 mm square against the module's 3.7
- Found: VER-125 (check_land.py), rewritten 2026-10-28 with the module's land pattern
- Symptom: fp_esp32.py draws the WROOM-1U's GND pad 3.90 x 3.90 mm; Figure 11-2's text gives 3.7 (esp32-s3-wroom-1.txt:3422-3471). The 40 pads (1.5 x 0.9 at 1.27, the columns 17.5 apart) match.
- Root cause: the footprint written by script without the land pattern's numbers read off the drawing.
- Why missed: no land-pattern check for the module.
- Check added or changed: VER-125 (a 'module' land row; its seed is U2's); VER-27 (2026-10-28: the pad's paste, nine 0.9 mm windows, prints 47.9% of the 3.90 mm pad against VER-27's 50-80%; it would be 53% of a 3.7 mm pad; Espressif asks only for 'the correct amount', esp32-s3-wroom-1.txt:2972-2974)
- Updated: DF-185
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-220 on the RP2350B board.

### DEF-85: No antenna or U.FL cable is on any buy list, and the case has no place for it
- Found: VER-167 (design_numbers.py bom), extended 2026-10-28, at 39cc07f
- Symptom: the WROOM-1U has a U.FL connector and no antenna (esp32-s3-wroom-1.txt:100); bom.md names neither the flex antenna (Kinghelm C4943394 in the build note) nor a cable, and case.scad has no feature that holds it on the left wall.
- Root cause: the antenna is a hand-fitted part outside JLC's BOM.
- Why missed: the BOM check read contact classes only.
- Check added or changed: VER-167 (now asks for the antenna and the cable); VER-207
- Updated: REQ-RF-02, DF-197
- Gate: CASE
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-239 (bom.md names every hand-fitted part) on the RP2350B board.

### DEF-86: 3V3 falls to 2.976 V at the module in a Wi-Fi burst at the 3.50 V cutoff
- Found: VER-204 with VER-13's copper solver, 2026-10-28, at 39cc07f
- Symptom: at VBAT 3.50 V and a TX burst (3V3 0.62 A, the system 1.24 A): U2's battery FET 124 mV (bq24074.txt:724), VSYS's 0.24 mm track from U2's OUT to U3's IN 137 mV (221 mohm), U3's dropout 263 mV (tlv757p.txt:298 scaled): 2.976 V against VDD33's 3.0 V minimum (esp32-s3-wroom-1.txt:1596).
- Root cause: U3 sits across the board from U2, and the Wi-Fi peak was not in the 3V3 budget the copper was routed for.
- Why missed: no check combined the burst, the copper and the dropout.
- Check added or changed: VER-204 (built); VER-13; VER-208 on unit 1
- Updated: REQ-PWR-05, REQ-MCU-09, DF-187
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-210 on the RP2350B board.

### DEF-87: Wi-Fi's heat takes the module's air over its 65 C and U3 over its junction limit on paper
- Found: VER-144, VER-193 and VER-200 (thermal.py), rewritten 2026-10-28 at 39cc07f
- Symptom: with Wi-Fi listening, the CPU at 240 MHz and the SD card writing (VER-31's 362 mA average on 3V3), the worst mode puts 3.31 W inside the case: its air 62.6 C at 40 C outside; the module's air 84.9 C on a 30 C/W stand-in against the N16R8's 65 C ambient (esp32-s3-wroom-1.txt:1598); U3 at 0.434 W reaches 163 C on JEDEC's 231.1 C/W (106 C on TI's EVM 100.8) against 105 C; the cell's air 47.6 C charging while playing at 25 C against 45 C; U7 144.7 C on a 160 C/W stand-in against 130 C. All on stand-ins for h, the module's theta and U7's; the 2.0 W budget of REQ-THM-04 is 3.31 W here.
- Root cause: Wi-Fi and a 240 MHz CPU added to a closed handheld sized for the RP2354B's heat.
- Why missed: the heat table had the RP2354B's 150 mA budget until this re-seed.
- Check added or changed: VER-144, VER-193, VER-200 (rewritten); VER-86 on unit 2
- Updated: REQ-THM-04, REQ-MCU-13, DF-31, DF-193
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-213 on the RP2350B board.

### DEF-88: pin_review/u3.json describes the RT9013, not the TLV75733P on the board
- Found: VER-36 (circuit_checks.py pinreview) at 39cc07f, 2026-10-28
- Symptom: u3.json names RT9013-33GB, C47773 and the RT9013's ratings (EN 1.6 V, 400 mV dropout at 500 mA) while design.py's U3 is TLV75733PDBVR, C485517. VER-02 passes because check_pins.py compares each pin's net, and the two parts share the SOT-23-5 pinout (IN 1, GND 2, EN 3, NC 4, OUT 5: tlv757p.txt:114). Seven review files describe parts that left.
- Root cause: the review file kept when the part changed.
- Why missed: VER-02 reads nets, not identity; VER-36 read identity but was not run at 39cc07f.
- Check added or changed: VER-36 (its identity rule fires; its A-line count fixed)
- Updated: REQ-FAB-02, DEF-26
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-211 on the RP2350B board.

### DEF-89: The microSD slot's body has 0.075 mm a side in the case's opening
- Found: VER-165 (case_checks.py mouths), extended 2026-10-28, at 39cc07f
- Symptom: J8's body (its 3D model) spans y 128.075-141.925; the left wall's opening (case.scad sd_y) is y 128.0-142.0: 0.075 mm a side against the print's 0.1 mm and the board's 0.1 mm float on its screws. Its top, 1.67 mm, is clear of the opening's 2.60 mm.
- Root cause: the opening sized to the slot's 13.85 mm body plus 0.15 mm.
- Why missed: the mouths check read J2, J4 and J5 only.
- Check added or changed: VER-165
- Updated: REQ-STO-03, REQ-MEC-11, DF-195
- Gate: CASE
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-233 on the RP2350B board.

### DEF-90: J8 reaches 0.175 mm into the folded FPC's zone, 1.675 mm tall against fpc_low's 1.05
- Found: VER-20 (case_checks.py stack) at 39cc07f, 2026-10-28
- Symptom: J8's extent ends at x 78.225; the folded FPC's zone starts at x 78.05 (positions.json fpc_zones); under the FPC the parts must stay under fpc_low (1.05 mm, an ESTIMATE) less 0.2 mm, and J8 is 1.675 mm tall. The build note put the slot's back at 78.02, 0.03 mm short of the zone; its 3D model reaches further.
- Root cause: the slot's back placed against an estimated zone with its footprint's outline, not its 3D body.
- Why missed: the stack check had not run on 39cc07f.
- Check added or changed: VER-20; VER-152 (fpc_low is an ESTIMATE)
- Updated: DF-123
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-229 on the RP2350B board.

### DEF-91: A microSD card plugged in while running can dip 3V3 under 3.0 V
- Found: VER-206 (paper_checks.py sdinsert), built 2026-10-28, at 39cc07f
- Symptom: SD_VDD is +3V3 through JP6 with no load switch: the rail's 21.4 uF (at 3.35 V on Samsung's curves, half value where none) shares its charge with the card's own capacitance; a card over 1.8 uF takes 3V3 under 3.0 V, a 10 uF card (stand-in) to 2.21 V, for about 38 us until U3's 1.2 A limit refills it.
- Root cause: the card's supply straight off the module's rail.
- Why missed: new at 39cc07f.
- Check added or changed: VER-206 (built; its seed takes the bulk away); VER-209 (three cards on the scope)
- Updated: REQ-STO-02, DF-194
- Gate: EVT
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries to no DF row: the microSD card whose insertion dipped 3V3 left the design on 2026-10-29.

### DEF-92: J7's 3V3 pad back-feeds U3 when a jig powers it
- Found: VER-159 (check_backpower.py), its jig state added 2026-10-28
- Symptom: design.py's note says to power the board through J7's 3V3 'with the cell out and nothing on USB'; then U3's OUT sits over its IN + 0.3 V, over the TLV75733P's absolute maximum (tlv757p.txt:180), and its body diode feeds VSYS (1137-1138).
- Root cause: the pogo pads kept from the RP2354B board without the regulator's reverse rule.
- Why missed: the path table had no jig state.
- Check added or changed: VER-159 (the jig state, listed with DF-189); VER-212 (the recovery procedure)
- Updated: REQ-PWR-19, DF-189
- Gate: EVT
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-241 on the RP2350B board.

### DEF-93: The audio filter's bandwidth is 8.2 kHz at the worst corner against 10 kHz
- Found: VER-100 (paper_checks.py audio) at 39cc07f, 2026-10-28
- Symptom: R32 and R33 went to 1.5k for the LEDC's 312.5 kHz carrier: 44.5 dB down nominal, 42.6 dB at the worst corner (pass), but the -3 dB point is 9.7 kHz nominal and 8.2 kHz at the worst corner against REQ-AUD-03's 10 kHz. design.py's comment says 6.7 kHz (board/sim/audio_filter.py's model).
- Root cause: carrier rejection traded against bandwidth with the requirement unchanged.
- Why missed: VER-100 had not run on 39cc07f.
- Check added or changed: VER-100
- Updated: REQ-AUD-03
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-248 (the audio filter's bandwidth at its corners) on the RP2350B board.

### DEF-94: USB's series resistors are 11.5 mm from the module
- Found: VER-191 (paper_checks.py usb) at 39cc07f, 2026-10-28
- Symptom: R21 and R22 (22 ohm) sit 11.5 mm in a straight line and 18.2 and 20.9 mm of track from pins 14 and 13, against 'place them close to the chip' (esp32-s3-hw-design-guidelines.txt:875, 1307). The design moved them there in e20cfa6 because the pair could not reach pads under the pins.
- Root cause: routing room under the module.
- Why missed: new at 39cc07f.
- Check added or changed: VER-191
- Updated: REQ-MCU-02, DF-198
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-261 (the USB pair's resistors at the chip) on the RP2350B board.

### DEF-95: J2 carries up to 1.24 A against a 1 A catalogue rating
- Found: VER-31 (circuit_checks.py ratings) at 39cc07f, 2026-10-28
- Symptom: a Wi-Fi TX burst (3V3 0.62 A) with the backlight at 117.7 mA and the amplifier's 0.5 A peak draws 1.24 A through J2, the PicoBlade, against its 1 A (a catalogue stand-in: molex_53261.pdf has no text layer) and the register's 80% rule. The cell (700 mAh) sees about 1.8 C.
- Root cause: Wi-Fi's peak current on a connector sized for the RP2354B's load.
- Why missed: the ratings table had no Wi-Fi load.
- Check added or changed: VER-31
- Updated: DF-118
- Gate: EVT
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-221 on the RP2350B board.

### DEF-96: BL_PWM low leaves the backlight glowing on a unit whose U6 offset is positive
- Found: VER-05 (paper_checks.py backlight) at 39cc07f, 2026-10-28
- Symptom: with 3V3 on and BL_PWM low, BL_REF is 0 V and each sink regulates its 2.2 ohm sense to U6's input offset. The MCP6002's offset is up to +4.5 mV at VCM = VSS (mcp6001.txt:135), so a unit can hold 2.07 mA a sink, 4.13 mA in all (0.69 mA an LED): a visible glow, and 4.13 mA more in the charging screen's sleep. pinmap.md says BL_PWM 'low = off' and turns the backlight off that way while charging. It is under VER-05's 5 mA limit, so the check passes.
- Root cause: a single-supply sink with no offset trim, and a 'off' that relies on a zero reference.
- Why missed: the ngspice run modelled the offset only at 5% duty and up; nothing looked at duty 0.
- Check added or changed: VER-05 (prints the off-state current); VER-62 records the current through JP2 with BL_PWM low on unit 1
- Updated: VER-62
- Gate: EVT
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-228 on the RP2350B board.

### DEF-97: The smallest 4 mm dome's rim sits 0.171 mm inside the ring pad's inner edge
- Found: VER-178 (check_dome.py) at 39cc07f, 2026-10-28, the first run on the dome's drawing
- Symptom: the HYP 600-415S-260 drawing gives D 4.0 +0/-0.05 mm (dome_600-415s-260.pdf page 1, row 600-4***-***). Dome_4mm_Round's ring runs r 1.704 to 2.300 mm as KiCad fills it. At 0.1 mm of placement error the smallest dome's rim (r 1.975) is 0.171 mm inside the ring's inner edge, against VER-178's and REQ-BTN-08's 0.2 mm; the outer side has 0.200 mm. At the hand placement's 0.3 mm the near side of the rim is 0.029 mm off the ring (the far side stays on it, so the dome still makes contact). The apex, the gap, the vent, the pads' layers and the Kapton dot pass.
- Root cause: the ring was centred on the nominal 4.0 mm rim (r 2.0), with no allowance for the drawing's -0.05 mm.
- Why missed: the drawing was not on disk until 2026-10-28.
- Check added or changed: VER-178 (built: check_dome.py)
- Updated: DF-168
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-230 on the RP2350B board.

### DEF-98: design.py names Samsung as the maker of the four Fenghua X7R capacitors
- Found: VER-186 (check_lcsc.py) at 39cc07f, 2026-10-28, against Konnect's JLC snapshot of 2026-07-13
- Symptom: C37, C38 (4.7n, 0402B472K500NT, C1538) and C48, C49 (1n, 0402B102K500NT, C1523) carry the maker 'Samsung' in design.py, so in the schematic's and the board's fields; JLC's record and the MPN say FH (Guangdong Fenghua). design.py's C() helper names the maker 'FH' only for 0402CG parts. The BOM fab.py writes for JLC has no maker column and JLC matches on the LCSC number, so the order gets the right part; a person reading the schematic or buying by hand would not.
- Root cause: a maker chosen by an MPN prefix rule that covers only one Fenghua series.
- Why missed: no check compared the maker field with the part's record.
- Check added or changed: VER-186 (built)
- Updated: none
- Gate: EVT
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-238 (each part's identity against its record) on the RP2350B board.

### DEF-99: The PLA case over U2 runs past PLA's heat deflection when charging while playing
- Found: VER-192 (thermal.py casemat) at 39cc07f, 2026-10-28, its first run
- Symptom: every printed part is Bambu PLA Basic (print_package_README.md:18). VER-193's model puts the case's inner surface at 62.6 C on average and 73.4 C over U2 at 40 C outside in the hottest mode (charging while playing, the backlight full, Wi-Fi on), and 47.6 C and 58.4 C at 25 C outside. PLA's heat deflection at 0.45 MPa is about 55-60 C (a stand-in: the maker's sheet is not on disk). VER-192's limit wants 88.4 C at 40 C; even the room-temperature spot over U2 (58.4 C) is above the stand-in's 57 C, so the lid can creep over U2 and the panel.
- Root cause: the case's material was chosen for printing (supports, the one-piece top); the Wi-Fi board's heat (3.31 W inside, DEF-87) was never set against it.
- Why missed: VER-192 was NOT_BUILT; VER-193 printed the inner surface without a limit.
- Check added or changed: VER-192 (built: thermal.py casemat)
- Updated: DF-202
- Gate: CASE
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-213 on the RP2350B board.

### DEF-100: U2's exposed pad gets no paste, and its four paste windows sit between U2's pins
- Found: VER-27 (layout_checks.py gerbers) at 39cc07f, 2026-10-28, its first run
- Symptom: in the F.Paste gerber fab.py writes, U2's exposed pad 17 (1.68 x 1.68 mm, GND) has no paste at all (0%, against the 85% of TI's own stencil example, bq24074.txt:3180), and four 0.24 x 0.60 mm paste windows lie on U2's pin rows where no pad is: at (124.58, 85.60) between pins 15 (TD, GND) and 16 (ISET), at (125.42, 85.60) between 14 (TMR, open) and 13 (IN, VBUS), at (124.58, 88.40) between 6 (EN1) and 5 (EN2, GND), at (125.42, 88.40) between 7 (PGOOD) and 8 (VSS). Each window overlaps one neighbour's window by 0.07 mm and leaves 0.09 mm of stencil to the other's. Paste on the mask between 0.5 mm pins makes bridges: ISET to GND stops all charging (bq24074.txt:709, the start-up short check), EN1 to EN2 gives USB100 (a 100 mA input), PGOOD to VSS a stuck status, TMR to IN undefined timers. The unsoldered exposed pad loses TI's 44.5 C/W (it assumes the pad soldered), so U2's 0.80 W runs it into thermal regulation or past VER-144's limit.
- Root cause: board/tools/fp_u2.py builds battler:VQFN-16-1EP_3x3mm_RGT0016C from KiCad's VQFN-16 and moves every pad but '17' onto TI's pin rows; KiCad's footprint carries the exposed pad's paste as four unnumbered paste-only pads (0.68 mm at +-0.42, +-0.42), so the script resized them to 0.24 x 0.60 and moved them to y +-1.4 with the pins.
- Why missed: VER-125 (check_land.py) compares copper, not paste; VER-132 reads paste only over the button contacts; VER-27, which reads the exposed pads' paste, was not built (gerbv by eye) and only advisory.
- Check added or changed: VER-27 (built: layout_checks.py gerbers; binding at G-ORD-09 since 2026-10-28)
- Updated: DF-08, DF-149
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-219 on the RP2350B board.

### DEF-101: At its drawing's 3.35 mm the module breaks the glass's 0.2 mm clearance; check_fit checks it at the nominal 3.2
- Found: VER-112 (case_checks.py heights) at 39cc07f, 2026-10-28, its first run
- Symptom: the ESP32-S3-WROOM-1U is 3.2 +-0.15 mm tall (esp32-s3-wroom-1.txt:3130, Figure 10-2), so up to 3.35 mm. check_fit checks it at BODY_LOCAL's 3.2 (case/check_fit.py), under case.scad's glass_limit of 3.30 (glass_gap 3.5 less 0.2). At 3.35 mm check_fit's own rule fails: 0.15 mm to the panel's back against REQ-MEC-01's 0.2, before the board's and the print's tolerances (VER-20). check_fit also takes the three SN74HC165s at their 3D model's 1.09 mm against the drawing's 1.2 MAX (sn74hc165.txt:1809); at 1.2 mm they still clear.
- Root cause: check_fit's stand-in body for the module took the drawing's nominal height, not its maximum; the comment at case.scad:222 counts 0.1 mm of room over a 3.2 mm module.
- Why missed: VER-112 was NOT_BUILT; check_fit takes a model's or a stand-in's height as true.
- Check added or changed: VER-112 (built: case_checks.py heights)
- Updated: DF-91, REQ-MEC-01
- Gate: CASE
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-229 (heights at the drawing's maximum) on the RP2350B board.

### DEF-102: The panel's FPC tail is 25.5 +-0.15 mm wide against the FH12-50S's recommended 25.5 +-0.07
- Found: VER-219 (case_checks.py fpc) at 39cc07f, 2026-10-28, its first run
- Symptom: the ER-TFT035IPS-6 drawing gives its tail 25.5 +-0.15 mm wide (er-tft035ips-6.txt:285); Hirose recommends B = 25.5 +-0.07 mm for a 50-contact FPC (hirose_fh12.txt:744, 705). A tail at the panel's upper limit (25.65 mm) is 0.08 mm wider than the FH12's recommended maximum, and over the connector's 25.57 mm D column (hirose_fh12.txt:250, read as its FPC opening). The thickness (0.3 +-0.03 against 0.3 +-0.05), contact width and contact span sit inside Hirose's bands.
- Root cause: the panel's maker holds its tail's outline looser than Hirose's recommendation; nothing compared the two drawings.
- Why missed: VER-44 (calipers on a real tail) needs the panel in hand; no paper check read both drawings.
- Check added or changed: VER-219 (built: case_checks.py fpc)
- Updated: REQ-DSP-04
- Gate: ORDER
- Board: esp32s3
- Status: DROPPED with esp32s3 (tag vv-esp32s3-final): the ESP32-S3 board is dropped; it was OPEN there, neither fixed nor closed on the RP2350B board. Its lesson carries as DF-206 on the RP2350B board.

### DEF-105: 54 live requirements cite no primary Source (a datasheet line, a standard's clause or Alex's decision)
- Board: rp2350b
- Found: VER-189 (the per-subsystem requirements review) on 2026-10-29: VER-189's pass limit wants 0 live requirements without a primary Source; 54 of the 133 live rows (REQ-PRD rows aside, which Alex signed) cite only the archived validation_plan (#n), the design's own files (design.py, case.scad, placement.py, bringup.md, design_review.md, assembly.md), or Claude's estimate, analysis or review.
- Symptom: the numbers on those rows are the design's own or Claude's: REQ-BAT-05.B, -07, -08, -10, -14, -15; REQ-PWR-09, -10, -14; REQ-DSP-06, -09, -10; REQ-AUD-03, -04, -06; REQ-BTN-02 to -08; REQ-MEC-01 to -12; REQ-THM-02 to -04; REQ-FAB-01, -02, -05 to -10; REQ-ASM-01, -02; REQ-TST-01 to -03, -05 to -08. Nobody outside the register has set them, so a check can pass them against a number nobody owns.
- Root cause: the rows were seeded from the validation_plan's methods and the design files and carried across four boards; requirements.md's own Source definition allows 'validation_plan method (#n)', which VER-189's pass limit does not.
- Why missed: no check read each row's Source against the pass limit's definition until VER-189 ran.
- Check added or changed: VER-189 (re-run once each row cites a datasheet line, a standard's clause or Alex's number, or Alex signs the row)
- Updated: REQ-BAT-05.B, REQ-BAT-07, REQ-BAT-08, REQ-BAT-10, REQ-BAT-14, REQ-BAT-15, REQ-PWR-09, REQ-PWR-10, REQ-PWR-14, REQ-DSP-06, REQ-DSP-09, REQ-DSP-10, REQ-AUD-03, REQ-AUD-04, REQ-AUD-06, REQ-BTN-02, REQ-BTN-03, REQ-BTN-04, REQ-BTN-05, REQ-BTN-06, REQ-BTN-07, REQ-BTN-08, REQ-MEC-01, REQ-MEC-02, REQ-MEC-03, REQ-MEC-04, REQ-MEC-05, REQ-MEC-06, REQ-MEC-07, REQ-MEC-08, REQ-MEC-09, REQ-MEC-10, REQ-MEC-11, REQ-MEC-12, REQ-THM-02, REQ-THM-03, REQ-THM-04, REQ-FAB-01, REQ-FAB-02, REQ-FAB-05, REQ-FAB-06, REQ-FAB-07, REQ-FAB-08, REQ-FAB-09, REQ-FAB-10, REQ-ASM-01, REQ-ASM-02, REQ-TST-01, REQ-TST-02, REQ-TST-03, REQ-TST-05, REQ-TST-06, REQ-TST-07, REQ-TST-08
- Gate: ORDER
- Status: OPEN (2026-10-29, after the Source rewrite on vv-189-fix): 2 of the 54 now cite a datasheet line (REQ-DSP-06: er-tft035ips-6.pdf p.11; REQ-AUD-04: fm8002a.pdf p.3); the other 52 are design choices whose Source reads "Alex's decision (to sign)", with the derivation and arithmetic in each Rationale, and stay PROPOSED. What remains is Alex's signed line on those 52 (with REQ-BAT-16, REQ-SOC-03, REQ-AUD-07 and REQ-FAB-13): the table is analysis/to_sign_2026-10-29.md. VER-189 is re-run after he signs; no row is APPROVED by this fix.

### DEF-106: Live requirements whose number has no tolerance
- Board: rp2350b
- Found: VER-189 on 2026-10-29: REQ-SOC-04 and REQ-PRD-08.B say 'about 8 s' for the hard reset; REQ-SOC-03 gives the crystal's frequency (12 MHz) but no start-up time or frequency tolerance; REQ-BAT-16 says 'about 140 mA'.
- Symptom: a check cannot pass or fail 'about'; VER-258 had to read 'about 8 s' as 4 s to 12 s on Claude's own reading.
- Root cause: DESIGN.md (Alex) says 'about 8 seconds' and the rows copied it.
- Why missed: no check read each statement for a value and a tolerance.
- Check added or changed: VER-189 (re-run once the tolerances are set); VER-258 (reads the datasheet's delay)
- Updated: REQ-SOC-04 (4 s to 12 s proposed, Claude's reading), REQ-SOC-03, REQ-BAT-16, REQ-PRD-08.B (signed: Alex's to change)
- Gate: ORDER
- Status: OPEN (2026-10-29, on vv-189-fix; the tolerances are set, what remains is Alex's signature on them and the crystal's frequency tolerance): the hard reset is 6 s to 10 s (Alex, 2026-10-29) in REQ-SOC-04's Statement and VER-258's pass limit, and REQ-PRD-08.B's Notes read 'about 8 s' that way (its signed Statement is untouched); REQ-BAT-16 is 140 mA +-10% (126 to 154 mA); REQ-SOC-03 has a start-up limit of 10 ms from 3V3 valid (a design choice, neither the RP2350 nor the crystal datasheet is on disk; also VER-237's pass limit), to sign. It moves to FIXED once Alex signs REQ-SOC-04, REQ-SOC-03 and REQ-BAT-16 and the crystal's frequency tolerance is set (the part is chosen at NM-04), and closes when VER-189 is re-run on fresh evidence.
