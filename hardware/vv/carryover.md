# Carry-over from the RP2354B and ESP32-S3 rounds

**2026-10-28: the register targets the F1C200s board.** The ESP32-S3 board (39cc07f) and the RP2354B
board (71737ae) are both dropped; their registers are at tags `vv-esp32s3-final` and `vv-rp2354b-final`.
This file keeps what their rounds found that applies to any board that shares the cell, the charger, the
case, the panel, the buttons, the audio or the connectors, or that is built by the same scripts at JLC.
Each group now carries as the F1C200s board's DFMEA rows (dfmea.csv) and the checks they name, all
NOT_RUN or NOT_BUILT until an F1C200s design exists: the cell and its pocket (C rows) as DF-214, DF-215,
DF-216 and DF-231; the charger and the power path (P rows) as DF-217 to DF-226 and DF-213 (heat); the
panel (D rows) as DF-227 to DF-229; the buttons (B rows) as DF-230; the case (K rows) as DF-232 to
DF-234; the fab outputs (F rows) as DF-235 to DF-239; ESD as DF-240. The dropped defects whose lessons
no row carried (F's audit, 2026-10-28) carry as DF-248 (the audio chain), DF-249 (the printed case's
features), DF-250 (the fab files and DRC) and DF-251 (the firmware's pin file); each dropped defect in
defects.md names its row. The verdicts in the tables below
are the ESP32-S3 board's at 39cc07f, history now; check_trace does not read this file.

Each row: the finding, its number on the RP2354B board (71737ae), the check that found it, and
what the new design must satisfy. The IDs are the frozen register's.

## At 39cc07f: each row's requirement, check and verdict (2026-10-28)

Re-seed of 2026-10-28. Each verdict is this register's own number from the evidence of that date
(`evidence/2026-10-28_*`), not the board session's status file: FIXED (the check passes on the new
board), STILL FAILING (it fails, with the number), NEEDS ALEX (a decision or a part only he can
give), N/A (the reason the row no longer applies). Every row has a requirement and a check.

| # | Requirement | Check | Verdict at 39cc07f |
|---|---|---|---|
| C1 | REQ-BAT-08 | VER-181 | FIXED: the hole 55.2 x 38.2 mm, the pocket 52.4 x 36.4, 0.40 mm longer and wider than the worst-case 52 x 36 cell against 0.40 |
| C2 | REQ-BAT-08 | VER-181 | FIXED: the pocket's walls 0.90 mm inside the hole on three faces and 1.90 at the lead end: 0.50 and 1.50 mm after the 0.40 mm of tolerances, wherever the cell lies (recomputed: the board session had not) |
| C3 | REQ-BAT-06, REQ-BAT-02 | VER-37, VER-184 | NEEDS ALEX: no spec sheet on disk, analysis/cell.json unwritten; VER-184 BLOCKED on it |
| C4 | REQ-BAT-02.R | VER-37, VER-160 | NEEDS ALEX: R5 3.3k gives 242-295 mA (KISET 797-975 A-ohm, bq24074.txt:785, R5 1%); the cell's own limit waits for its spec sheet |
| C5 | REQ-BAT-07 | VER-182 | FIXED: 1.87 mm free over a worst-case cell (the slope's inside 3.47 mm over the board limits it; the board session's 1.9 mm is the panel's back) against 0.40 mm of swelling |
| C6 | REQ-BAT-10, REQ-PWR-07 | VER-103 | STILL FAILING: the off budget 26.8 uA at 25 C (U3's 1 uA shutdown, tlv757p.txt:265), 0.8 months from the 3.50 V cutoff to the PCM's UVP at the worst case and 2.5 nominal, against 2 months (stand-ins for the cell) |
| C7 | REQ-BAT-05.R | VER-184, VER-202 | FIXED on the board: J2 PicoBlade 1.25 mm, J4 JST SH 1.0 mm, another family and pitch; the plugs themselves are C3's and A1's |
| P1 | REQ-BAT-03 | VER-32 | STILL FAILING: 3.9-48.2 C at the worst corner against 45 C (TH1 10k B4460, its resistance and B +-1% from its part number, tctr0805.txt:260); B 5130 needed, none stocked |
| P2 | REQ-THM-02 | VER-164 | STILL FAILING: TH1 1.05 mm from the hole's edge and 1.95 mm or more from the cell's face against 1.0 mm |
| P3 | REQ-FAB-02 | VER-125 | FIXED for U2: TI's RGT0016C land, 0.26 mm pad to exposed pad, equal to TI's; VER-125 still fails on U4's thermal pad, 3.90 against 3.7 mm (DEF-84). The copper is fixed, the paste is not: fp_u2.py moved the exposed pad's four paste windows onto the pin rows, so the pad prints 0% and the windows sit between pins (VER-27 FAIL, DEF-100, 2026-10-28) |
| P4 | REQ-PWR-14, REQ-FAB-03 | VER-201, VER-13 | STILL FAILING: the PWR class now names only real nets and holds every net of 0.25 A or more at 0.3 mm (1.00 A by IPC-2221; VSYS 0.98 A, VER-201 PASS), but VSYS reaches U3 through 0.24 mm track, 221 mohm (137 mV at 0.62 A), and the cell to VSYS copper is 178.8 mV against 30 mV (VER-13 FAIL) |
| P5 | REQ-PWR-17 | VER-177 | STILL FAILING: U3's EN (6.0 V absolute, tlv757p.txt:179) over its rating from VBUS 9.1 V, U2's EN1 from 12.4 V, against 20 V; the accepted value is Alex's (DEC-17) |
| P6 | REQ-PWR-17 | VER-195 | STILL FAILING: IO2 and IO4 pass VDD + 0.3 V from about 4.0 V behind R3 and R4, R1 and R2 reach 62.5 mW at 17.9 V, the 3V3 domain through U1 from about 3.9 V; a clamp is a part choice (DEC-17) |
| P7 | REQ-PWR-07 | VER-159 | FIXED: the path table passes in six states, 36 paths in 9 table lines, U1's REF on 3V3; it now lists J7's jig on 3V3 back-feeding U3 (DF-189, DEF-92) |
| P8 | REQ-PWR-13 | VER-99 | FIXED: 47.3 uC at 5.25 V with C1 at +20% integrated along its own DC-bias curve (samsung_cl10a106ma8nrnc_dcbias.csv) against 50 uC (recomputed: the board session took 52.5 uC at nominal) |
| P9 | REQ-BAT-12, REQ-TST-06 | VER-138 | STILL FAILING: the dropout knee is gone (the ESP32-S3's ADC has its own reference), but its error is +-50 mV at the pin, +-100 mV of VBAT (esp32-s3.txt:3804), and a gain at charge-done keeps the CV's 3.465-3.535 V; the per-unit point at 3.50 V (VER-155) fixes it |
| P10 | REQ-THM-04 | VER-144, VER-193 | STILL FAILING: 3.31 W inside the case in the worst mode with Wi-Fi on against 2.0 W; U3 163 C on JEDEC's 231.1 C/W (106 C on the EVM's 100.8) against 105 C; the module's air 84.9 C against 65 C; the cell 47.6 C against 45 C (at VER-144's 5.25 V and 3.4 V corners, TI's, not the requirement's 5.5 V and 3.0 V) |
| P11 | REQ-PWR-01 | VER-67, VER-70 | STILL FAILING: VER-67's and VER-70's procedures sink the charge current (CHG_OFF high or 10 ohm 5 W), but bringup.md, which the bench follows, still describes the RP2354B board (DEF-21, DEF-46) |
| D1 | REQ-PWR-04 | VER-198, VER-34 | STILL FAILING: up to 3.333 V (the TLV75733P's +1% to TJ 85 C, tlv757p.txt:252; 3.350 V at +1.5%, 255) against the panel's 3.3 V (er-tft035ips-6.txt:747-748); down from 3.366 V; proposed: Alex accepts (DEC-09) |
| D2 | REQ-DSP-05 | VER-137 | FIXED: 117.2 mA worst case (3V3 +1.5%) against 120 mA |
| D3 | REQ-MEC-01 | VER-152 | FIXED: the panel's outline 54.56 x 82.84, neck 4.47 and border from the drawing; the rest marked ESTIMATE |
| D4 | REQ-MEC-01 | VER-152, VER-20 | STILL FAILING: seven ESTIMATEs set copper with no measurement (VER-152 FAIL); J8 now reaches 0.175 mm into the folded FPC's zone at 1.675 mm tall against fpc_low's 1.05 less 0.2 (VER-20 FAIL, DEF-90) |
| D5 | REQ-DSP-02 | VER-199 | STILL FAILING: pinmap.md states the 8080 bus's 25 MHz and 40 ns; it states no SR_CLK rate or SR_PL widths |
| D6 | REQ-DSP-11 | VER-43, VER-199 | NEEDS ALEX: 12.3 ms of bus time a frame in RGB565 at 25 MHz on paper (VER-199), 0 ns spare on twc; the panel and an ESP32-S3 dev board to buy for VER-43 |
| B1 | REQ-BTN-01 | VER-45, VER-33 | FIXED on paper: against a 47k -5% pull-up and the 74HC165's VIL at 3.3 V (0.94 V, interpolated from TI's 0.5 V at 2 V and 1.35 V at 4.5 V) a pill reads pressed up to 17.8 kohm against 15 (18.5 kohm on the Starts' and Selects' 1% R63, R86-R88); VER-45 measures it on a bare board |
| B2 | REQ-PWR-08 | VER-33 | FIXED: SW24 at its 500 mOhm plus 1 kohm at VSYS 3.0 V: Q1's gate 2.88 V, 0.38 V over 2.5 V, EN 2.69 V against VIH 1.0 V; sure on to 5.0 kohm (recomputed: the board session had not) |
| B3 | REQ-BTN-04 | VER-132 | FIXED: the GND pour 2.900 mm from every comb's centre against 2.9 |
| B4 | REQ-BTN-08 | VER-167 | STILL FAILING: bom.md buys no 600-415S-260 domes and no Start/Select caps |
| A1 | REQ-AUD-06 | VER-202 | STILL FAILING: the speaker's 1.25 mm plug against J4's 1.0 mm SH; its refit not bought |
| A2 | REQ-AUD-05, REQ-AUD-04 | VER-196, VER-200 | STILL FAILING on the heat half: shutdown high is off (fm8002a.txt:75-76), 4.2 uA (123), Q3's gate 0.10 V (VER-196 PASS); fm8002a.pdf prints no thermal resistance, and on a SOIC-8's 160 C/W stand-in U7 reaches 144.7 C against 130 (VER-200 FAIL) |
| K1 | REQ-MEC-09 | VER-147 | STILL FAILING: M2 x 6 engages 3.20 mm in the side pilots H1-H3 against 4.0 (M2 x 7 or 8 pass); H4's deck pilot 3.20 against 3.0 passes |
| K2 | REQ-MEC-11 | VER-165 | STILL FAILING: J2's and J4's mouth boxes are clear (J5 left), but J8's body has 0.075 mm a side in the SD opening against 0.2 mm (DEF-89) |
| K3 | REQ-FAB-07.R | VER-28 | NEEDS ALEX: the dry run's preview of every polarised part (U4 and SW24 new) |
| K4 | REQ-FAB-07.R | VER-176, VER-28 | FIXED in the CPL: every row within 0.10 mm of its pads' centre (VER-176 PASS); JLC's preview of J1-J4, J8, U4 and SW24 still decides (VER-28) |
| K5 | REQ-MCU-02, REQ-EMC-02 | VER-21, VER-108 | STILL FAILING: the longest runs over no ground are SD_D3 14.22 mm, SR_CLK 13.94, AUDIO_PWM 13.22, USB_DP_MCU 8.64 against 2 mm; D+ 83.65 and D- 87.25 mm, 3.60 mm apart against 2.0 |
| F1 | REQ-MCU-09 | VER-204 | STILL FAILING: C10 (22 uF) 23.6 mm from the module's 3V3 pin against 5 mm; C11 (100 nF) 3.4 mm |
| F2 | REQ-PWR-14 | VER-30 | STILL FAILING: C3 (BAT) 3.11 uF at 4.2 V on its Samsung curve against 4.7 uF (bq24074.txt:501); C10's curve (CL21A226MAQNNNE) not on disk; C24 left |
| F3 | REQ-FAB-03 | VER-18, VER-17 | STILL FAILING: 0.308 mm in the data, 0.108 mm after the 0.2 mm outline tolerance against 0.30 mm; JLC's dated page (VER-17) decides which the rule is |
| F4 | REQ-FAB-05 | VER-132 | FIXED: 'JLCJLCJLCJLC' on B.Silkscreen |
| F5 | REQ-FAB-01 | VER-04, VER-18 | STILL FAILING: 2 silk_overlap and 4 silk_edge_clearance warnings (cosmetic: VER-04 passes); no text in a hole or on a pad (VER-18 PASS) |
| F6 | REQ-FAB-01 | VER-04 | FIXED: check_pcb.py fails every violation type but five cosmetic silk and text types |
| F7 | REQ-FAB-04 | VER-131 | FIXED: battler-PTH.drl 677 holes and 4 slots, battler-NPTH.drl 12 holes, every one of the 689 in its right file |
| F8 | REQ-FAB-08 | VER-143 | STILL FAILING: two --reuse rebuilds compared: not done (VER-143 NOT_BUILT) |
| F9 | REQ-FAB-10 | VER-41 | NEEDS ALEX: order-day stock; the module, the TLV75733P, the DM3AT and the TPS850 were never checked against JLC (DEF-81) |
| F10 | REQ-FAB-02 | VER-34, VER-36 | STILL FAILING: 20 datasheets reached the disk; still missing the flex antenna's, C10's DC-bias curve and the ESP32-S3 errata; pin_review/u3.json describes the RT9013 (DEF-88) |
| F11 | REQ-ASM-01.R | VER-39, VER-130 | STILL FAILING: bringup.md, assembly.md and bom.md describe the RP2354B board; assembly.md counts 117 placements and 49 part numbers against design.py's 114 and 45 (VER-130 FAIL) |

Tally: FIXED 16 (C1, C2, C5, C7, P3, P7, P8, D2, D3, B1, B2, B3, K4, F4, F6, F7), STILL FAILING 25,
NEEDS ALEX 5 (C3, C4, D6, K3, F9; P5 and D1 also wait on a value only he sets), N/A 0. The left-out RP2354B rows stay left out: the 27 ohm resistors (22 ohm now, Espressif's), E9,
the RP2354B and L2 stock, the RP2350 firmware, the copied core, the GPIO mux and PWM slices are
RETIRED with 39cc07f; U10 stays on the board, its datasheet now on disk (DEF-63 FIXED); the SPI
bus left for the 8080 bus (D6's rule kept).

## The rows as found at 71737ae

### The cell and its pocket

| # | Finding | Number at 71737ae | Found by | The new design must |
|---|---|---|---|---|
| C1 | The tub's pocket is the worst-case cell's own size: no fit | pocket 52.00 x 36.00 mm for a 52 x 36 cell: 0.00 mm spare against 0.40 (DEF-62) | VER-181 | Make the pocket 0.4 mm longer and wider than the worst-case cell (0.1 mm print, 0.1 mm fit a side), with 0.5 mm of FR4 gap on every face and 1.5 mm at the lead end after tolerances: a 55.2 x 38.2 mm hole for a 52 x 36 cell, or a cell of 51.6 x 35.6 mm or less by its spec sheet (REQ-BAT-08) |
| C2 | The cell was held off the edge by tape, not by the pocket | lead end 0.25 mm against 1.5 mm, -0.15 mm after tolerances (DEF-58, DEF-53) | VER-181 | Hold the cell off every edge with the pocket's walls alone, wherever it lies in the pocket |
| C3 | The cell's size, plug and PCM are not known from a spec sheet | no spec sheet on disk; cell.json unwritten (DEF-11, DEF-78) | VER-37, VER-184 | Buy cells whose spec sheet gives the maximum size with the PCM, the plug (family and pitch), the charge limits and the PCM's trip points, and record them in analysis/cell.json before copper is set |
| C4 | The charge current was set for another cell | R5 2.4k gave 460-500 mA, about 0.7 C for 700 mAh; 242-295 mA after the fix (DEF-11) | VER-37, VER-160 | Set ICHG from the chosen cell's own limit, at the resistor's and the charger's tolerance |
| C5 | Room for a swollen cell | 0.80 mm under the panel at worst case (passes; DEF-51, DEF-65) | VER-182 | Keep free height over a worst-case cell under the real ceiling of the case, read from case.scad |
| C6 | A cell cut off at 3.50 V reaches its PCM's under-voltage trip too soon | 0.8 months from the cutoff at worst case against 2 months (DEF-69) | VER-103 | Size the off current, the cutoff voltage and the PCM's UVP together so a cut-off cell lasts the shelf period REQ-BAT-10 sets |
| C7 | The cell connector and the speaker connector could take each other's plug | J2 PicoBlade 1.25 mm, J4 JST SH 1.0 mm since 71737ae: keyed apart by footprint (F-R2) | VER-184, VER-202 | Key the cell's header and every other 2-pin header apart by family or pitch |

### The charger and the power path

| # | Finding | Number at 71737ae | Found by | The new design must |
|---|---|---|---|---|
| P1 | The charge window's hot edge is over the cell's limit | TH1 B4460 10k: 3.9-48.2 C at the worst corner against 45 C; it needs B 5130 or more, and no stocked 10k NTC has it; no resistor on TS fixes it (DEF-48) | VER-32 | Reach a hot edge of 45 C or less at the charger's TS limits and TH1's tolerance: an NTC with the B the window needs, a cell with its own NTC lead, or a requirement restated to the cell's own range |
| P2 | TH1 is too far from the cell to read it | 1.95 mm or more from the cell's face, against 1.0 mm (DEF-37) | VER-164 | Put the thermistor within 1.0 mm of the cell, or couple it with a part the case holds |
| P3 | U2's footprint puts the cell's pins closer to the ground pad than TI's land pattern | 0.21 mm pad to exposed pad against TI's 0.26 mm (RGT0016C); a bridge shorts the cell (DEF-72, DF-148 severity 10) | VER-125 | Use the maker's land pattern for every QFN, checked against its drawing on disk |
| P4 | Power nets are in no net class | CELL_P 13.5 mm of 0.15 mm track: 124% of IPC-2221's 0.60 A at 0.75 A; VSYS to U3 229 mohm; 115 mV from the cell against 30 mV; the PWR class names VBUS_D5, no net of the board (DEF-68) | VER-13, VER-201 | Put every net that carries 0.25 A or more in a class whose width meets IPC-2221 at its current, and name only nets the board has |
| P5 | VBUS overstresses enable pins | U3's EN (6 V absolute) over its rating from VBUS 9.1 V, U2's EN1 from 12.4 V, against a 20 V faulty source (DEF-50) | VER-177 | Keep every pin fed from VBUS inside its absolute maximum up to the fault voltage Alex sets in REQ-PWR-17, or clamp it |
| P6 | VBUS on the CC or data pins overstresses the CC parts and the 3V3 domain | CC parts above 3.9 V, the 3V3 domain above 4 V, R1 and R2 at 17.9 V (DEF-73) | VER-195 | State what a CC or data line at VBUS may do, and protect the pins that are not 5 V tolerant (the MCU's included) to that level |
| P7 | The ESD array's REF on VBUS back-fed VBUS from D+ | about 2.7 V on VBUS with no cable; the device could not turn off (DEF-01) | VER-159 | Pass a back-power table: in each power state no live net feeds a part whose supply is off, unless the table accepts it |
| P8 | Plug-in inrush has no margin | 10 uF: 50 uC at 5.0 V, 52.5 uC at 5.25 V against USB 2.0's 50 uC, at nominal value (DEF-43) | VER-99 | Keep VBUS capacitance inside USB 2.0's 10 uF and 50 uC at the worst-case voltage and the part's tolerance |
| P9 | The cutoff cannot fire in dropout, and its calibration point is off | lowest reading 3.521 V with the LDO in dropout; CV calibration 3.465-3.535 V (DEF-40) | VER-138 | Compute the cutoff's error with the ADC reference, the LDO's dropout and the charger's CV tolerance, and calibrate it on the bench (REQ-TST-06) |
| P10 | Heat while charging and playing is over the budget | 2.16 W charging while playing against REQ-THM-04's 2.0 W; computed at 5.25 V and 3.4 V, not the requirement's 5.5 V and 3.0 V; U2 at 44.5 C/W (JEDEC) and failing above 62 C/W | VER-144, VER-193 | Run the heat budget at the requirement's corners with the 2-layer board's own thermal resistance, and test the total against the budget |
| P11 | Bring-up could not hold a cell voltage with USB in | the charger pushed 242-295 mA into a supply that cannot sink (DEF-46) | VER-70, VER-67 | Write bring-up steps that emulate the cell with a load that sinks the charge current |

### The panel

| # | Finding | Number at 71737ae | Found by | The new design must |
|---|---|---|---|---|
| D1 | The panel's supply is above its rating | VCI and VDDI up to 3.366 V (the LDO's +2%) against the module's 3.3 V (DEF-61) | VER-198, VER-34 | Keep the panel's supply inside the module's own table at the regulator's tolerance, or Alex accepts it in writing |
| D2 | The backlight's worst case was over the panel's limit | 122.4 mA, then 117.7 mA after R61 3.74k, against 120 mA (DEF-39) | VER-137 | Keep the backlight's worst-case current under the panel's maximum with every tolerance and the op-amp's offset from its datasheet |
| D3 | The case used panel numbers that were not the drawing's | 54.50 x 83.00 x 2.45 mm against the drawing's 54.56 x 82.84 (DEF-59) | VER-152 | Take the panel's outline, neck, border and thickness from its drawing on disk; mark every other number ESTIMATE |
| D4 | Seven case estimates set copper and none is measured | the FPC's body, tail, tip range, slack and fpc_low, ss_pitch, ss_mem (DEF-78) | VER-152 | Measure each ESTIMATE that sets a part's place before the copper is frozen, or keep the whole range clear |
| D5 | The firmware's clock limits for the panel are written nowhere | panel read 6.25 MHz or less (trc 150 ns); a write closes at 18.75 MHz with 3.3 ns to spare (DEF-76) | VER-199 | Write the panel bus's read and write clock limits, and the shift registers' clock and load widths, in pinmap.md |
| D6 | A full frame is slow on the bus | 184 ms at 20 MHz SPI, 197 ms at 18.75 MHz (DF-180) | VER-43 | Meet REQ-DSP-11's full-frame and text-update times on the new bus (the proposal's 8-bit bus), measured on a real panel |

### The buttons

| # | Finding | Number at 71737ae | Found by | The new design must |
|---|---|---|---|---|
| B1 | The pull-ups cut the pill margin | 10k gave about 3.7 kohm; 47k gives about 17 kohm against REQ-BTN-01's 15 kohm (DEF-49) | VER-45, VER-33 | Read a pill pressed up to 15 kohm at the input's VIL and the pull-up's tolerance |
| B2 | The power latch's Start margin | sure on to 9.1 kohm at VSYS 3.42 V after R15 back to 100k (DEF-02) | VER-33 | Keep the latch's turn-on margin over both Start contacts at the lowest VSYS |
| B3 | The ground pour comes inside the comb's keep-out | 2.875 mm from every comb's centre against REQ-BTN-04's 2.9 mm (DEF-74) | VER-132 | Keep every pour, via and foreign copper 2.9 mm from a contact's centre (a rule area in build_pcb.py) |
| B4 | The Start/Select domes, pills and caps were not in the BOM | bom.md bought none (DEF-42) | VER-167 | Buy every contact class the case and board use |

### The audio

| # | Finding | Number at 71737ae | Found by | The new design must |
|---|---|---|---|---|
| A1 | The speaker's plug does not fit its header | the GSPK151035PN's 1.25 mm plug against J4's 1.0 mm SH; the SH refit not bought (DEF-64) | VER-202 | Match the speaker's plug to its header, or buy the refit leads with the speakers |
| A2 | The amplifier's states rest on a missing datasheet | U7's shutdown level and current from u7.json, fm8002a.pdf not on disk | VER-196, VER-124, VER-200 | Hold the amplifier off at power-up with a resistor, and check its shutdown polarity, level and heat from its datasheet on disk |

### The case and the connectors

| # | Finding | Number at 71737ae | Found by | The new design must |
|---|---|---|---|---|
| K1 | The case screws engage too little thread | M2 x 6: 3.2 mm in the side pilots against 4-5 mm (DEF-31) | VER-147, VER-190 | Engage 4 mm or more of thread in printed pilots, per pilot |
| K2 | A connector's mouth faces the tub's wall | J5 2 mm from the wall: the SH plug crosses it by 4.02 mm (DEF-77) | VER-165 | Leave every connector's plug and cable box clear of the walls with the board in the tub, or say which are used with the board out |
| K3 | The ESD array may be placed turned 180 degrees by JLC | a reversed U1 shorts 3V3 (DEF-14) | VER-28 | Check every polarised part on JLC's placement preview before ordering |
| K4 | Connector CPL rows were off their pads | J1 1.50 mm, J2 and J4 0.95 mm, J3 0.23 mm at be437f8; none over 0.10 mm at 71737ae (DEF-33) | VER-176, VER-28 | Write each part's pads' centre in the CPL and confirm the origins on JLC's preview |
| K5 | The fast nets cross gaps in the ground under them | the USB pair 38.7 mm with no ground under it, D+ and D- 3.6 mm apart; SR_CLK 6.9 mm against 2 mm (DEF-75) | VER-21, VER-108 | Route the USB pair side by side on one layer over unbroken ground, and keep every fast net over ground |

### Any board built by these scripts at JLC

| # | Finding | Number at 71737ae | Found by | The new design must |
|---|---|---|---|---|
| F1 | Decoupling capacitors are far from their supply pins | three supply pins with no capacitor within 5 mm (7.2, 7.4 and 6.7 mm); the far core pin with no bulk capacitor (DEF-67) | VER-197 | Put each supply pin's capacitor within the MCU maker's distance (5 mm or less), with the bulk value the maker's guide asks at each pin group |
| F2 | A capacitor is under its minimum at its own tolerance and DC bias | C24 4.7 uF -20% is 3.76 uF against the 4.7 uF minimum, before DC bias (DEF-70) | VER-30 | Meet every capacitance minimum at the part's tolerance and its DC-bias curve at the working voltage |
| F3 | Copper sits close to the routed edges | 0.308 mm in the data, 0.108 mm with the 0.2 mm outline tolerance, against 0.30 mm (DEF-56) | VER-18, VER-17 | Keep copper 0.30 mm from every routed edge after JLC's outline tolerance, per JLC's dated page |
| F4 | JLC's order-number marker is not on the board | no 'JLCJLCJLCJLC' on the silk (DEF-55) | VER-132 | Put the marker on a clear back-silk spot, off the combs |
| F5 | Silk overlaps silk and copper | J2's '+' over 'BATT'; 'PANEL' on U4's pads (DEF-66, DEF-57) | VER-04, VER-18 | Pass DRC with no silk overlap and no text in a hole or on a pad |
| F6 | check_pcb.py passes DRC warnings that are not silk | 13 rule types set to warning are never failed (DEF-36) | VER-04 | Fail on every DRC warning type except a named cosmetic list |
| F7 | One drill file mixed plated and unplated holes | 628 holes in one MixedPlating file at be437f8; fixed at 71737ae (DEF-32) | VER-131 | Export PTH and NPTH drill files apart |
| F8 | A via was moved by hand in the board file | 62b01f2 (DEF-60) | VER-143 | Generate the board whole from the scripts, and show two --reuse rebuilds give the same copper |
| F9 | Parts went out of stock | the AP3156 backlight pump and ProTek's SRV05-4 (DEF-12, DEF-13) | VER-41 | Check every JLC part's stock on order day |
| F10 | Parts went on the board without their datasheets | 4, then 15 parts with no datasheet on disk (U10's pin table from KiCad's symbol, DEF-63); 9 PASSes rested on them (2026-10-28) | VER-34 | Put every part's datasheet in hardware/ref_design/datasheets before the part is placed, and name it in each check's 'sources' |
| F11 | The docs disagreed with the design | 21 of 22 doc items stale; bom.md's order line 151.64 mm against 151.48 (DEF-21, DEF-79) | VER-39 | Keep bringup.md, assembly.md and bom.md's order line generated from or checked against the design before first power |

Rows: 7 cell, 11 power, 6 panel, 4 buttons, 2 audio, 5 case and connectors, 11 board process: 46.

### RP2354B-only, left out of the table

These apply to the RP2354B core and do not carry: the 27 ohm USB series resistors (DEF-08); the
E9 errata pull-down on PWR_HOLD and the errata firmware rules (DEF-10, DEF-71, VER-16); the
RP2354B and L2 stock (DEF-29); firmware for the RP2350 (DEF-34, VER-149, VER-151); U10, the
buffer for GPIO47's pad leakage (DEF-63's part, DF-182); the copied core's +1V1 capacitor and its
overlay on Raspberry Pi's minimal design (DEF-61's second half, VER-23, VER-197's values); the GPIO
mux and PWM slices (VER-15); the SPI bus choice itself (DEC-13; D6 keeps its frame-time rule).

Last updated: 2026-10-28 12:40 EDT
