# Carry-over from the RP2354B rounds

The 5-board order is now the ESP32-S3-WROOM-1-N16R8 board (Alex, 2026-10-28;
`hardware/ref_design/docs/esp32s3_board_proposal.md` on main). The RP2354B register is frozen at
tag `vv-rp2354b-final` (c8ac9dd). This file keeps what its rounds found that applies to any board
that shares the cell, the charger, the case, the panel, the buttons, the audio or the connectors,
or that is built by the same scripts at JLC. When the register is re-seeded for the new board,
each row becomes a requirement and a check (or a reason it no longer applies), and its number is
recomputed from the new design. check_trace does not read this file.

Each row: the finding, its number on the RP2354B board (71737ae), the check that found it, and
what the new design must satisfy. The IDs are the frozen register's.

## The cell and its pocket

| # | Finding | Number at 71737ae | Found by | The new design must |
|---|---|---|---|---|
| C1 | The tub's pocket is the worst-case cell's own size: no fit | pocket 52.00 x 36.00 mm for a 52 x 36 cell: 0.00 mm spare against 0.40 (DEF-62) | VER-181 | Make the pocket 0.4 mm longer and wider than the worst-case cell (0.1 mm print, 0.1 mm fit a side), with 0.5 mm of FR4 gap on every face and 1.5 mm at the lead end after tolerances: a 55.2 x 38.2 mm hole for a 52 x 36 cell, or a cell of 51.6 x 35.6 mm or less by its spec sheet (REQ-BAT-08) |
| C2 | The cell was held off the edge by tape, not by the pocket | lead end 0.25 mm against 1.5 mm, -0.15 mm after tolerances (DEF-58, DEF-53) | VER-181 | Hold the cell off every edge with the pocket's walls alone, wherever it lies in the pocket |
| C3 | The cell's size, plug and PCM are not known from a spec sheet | no spec sheet on disk; cell.json unwritten (DEF-11, DEF-78) | VER-37, VER-184 | Buy cells whose spec sheet gives the maximum size with the PCM, the plug (family and pitch), the charge limits and the PCM's trip points, and record them in analysis/cell.json before copper is set |
| C4 | The charge current was set for another cell | R5 2.4k gave 460-500 mA, about 0.7 C for 700 mAh; 242-295 mA after the fix (DEF-11) | VER-37, VER-160 | Set ICHG from the chosen cell's own limit, at the resistor's and the charger's tolerance |
| C5 | Room for a swollen cell | 0.80 mm under the panel at worst case (passes; DEF-51, DEF-65) | VER-182 | Keep free height over a worst-case cell under the real ceiling of the case, read from case.scad |
| C6 | A cell cut off at 3.50 V reaches its PCM's under-voltage trip too soon | 0.8 months from the cutoff at worst case against 2 months (DEF-69) | VER-103 | Size the off current, the cutoff voltage and the PCM's UVP together so a cut-off cell lasts the shelf period REQ-BAT-10 sets |
| C7 | The cell connector and the speaker connector could take each other's plug | J2 PicoBlade 1.25 mm, J4 JST SH 1.0 mm since 71737ae: keyed apart by footprint (F-R2) | VER-184, VER-202 | Key the cell's header and every other 2-pin header apart by family or pitch |

## The charger and the power path

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

## The panel

| # | Finding | Number at 71737ae | Found by | The new design must |
|---|---|---|---|---|
| D1 | The panel's supply is above its rating | VCI and VDDI up to 3.366 V (the LDO's +2%) against the module's 3.3 V (DEF-61) | VER-198, VER-34 | Keep the panel's supply inside the module's own table at the regulator's tolerance, or Alex accepts it in writing |
| D2 | The backlight's worst case was over the panel's limit | 122.4 mA, then 117.7 mA after R61 3.74k, against 120 mA (DEF-39) | VER-137 | Keep the backlight's worst-case current under the panel's maximum with every tolerance and the op-amp's offset from its datasheet |
| D3 | The case used panel numbers that were not the drawing's | 54.50 x 83.00 x 2.45 mm against the drawing's 54.56 x 82.84 (DEF-59) | VER-152 | Take the panel's outline, neck, border and thickness from its drawing on disk; mark every other number ESTIMATE |
| D4 | Seven case estimates set copper and none is measured | the FPC's body, tail, tip range, slack and fpc_low, ss_pitch, ss_mem (DEF-78) | VER-152 | Measure each ESTIMATE that sets a part's place before the copper is frozen, or keep the whole range clear |
| D5 | The firmware's clock limits for the panel are written nowhere | panel read 6.25 MHz or less (trc 150 ns); a write closes at 18.75 MHz with 3.3 ns to spare (DEF-76) | VER-199 | Write the panel bus's read and write clock limits, and the shift registers' clock and load widths, in pinmap.md |
| D6 | A full frame is slow on the bus | 184 ms at 20 MHz SPI, 197 ms at 18.75 MHz (DF-180) | VER-43 | Meet REQ-DSP-11's full-frame and text-update times on the new bus (the proposal's 8-bit bus), measured on a real panel |

## The buttons

| # | Finding | Number at 71737ae | Found by | The new design must |
|---|---|---|---|---|
| B1 | The pull-ups cut the pill margin | 10k gave about 3.7 kohm; 47k gives about 17 kohm against REQ-BTN-01's 15 kohm (DEF-49) | VER-45, VER-33 | Read a pill pressed up to 15 kohm at the input's VIL and the pull-up's tolerance |
| B2 | The power latch's Start margin | sure on to 9.1 kohm at VSYS 3.42 V after R15 back to 100k (DEF-02) | VER-33 | Keep the latch's turn-on margin over both Start contacts at the lowest VSYS |
| B3 | The ground pour comes inside the comb's keep-out | 2.875 mm from every comb's centre against REQ-BTN-04's 2.9 mm (DEF-74) | VER-132 | Keep every pour, via and foreign copper 2.9 mm from a contact's centre (a rule area in build_pcb.py) |
| B4 | The Start/Select domes, pills and caps were not in the BOM | bom.md bought none (DEF-42) | VER-167 | Buy every contact class the case and board use |

## The audio

| # | Finding | Number at 71737ae | Found by | The new design must |
|---|---|---|---|---|
| A1 | The speaker's plug does not fit its header | the GSPK151035PN's 1.25 mm plug against J4's 1.0 mm SH; the SH refit not bought (DEF-64) | VER-202 | Match the speaker's plug to its header, or buy the refit leads with the speakers |
| A2 | The amplifier's states rest on a missing datasheet | U7's shutdown level and current from u7.json, fm8002a.pdf not on disk | VER-196, VER-124, VER-200 | Hold the amplifier off at power-up with a resistor, and check its shutdown polarity, level and heat from its datasheet on disk |

## The case and the connectors

| # | Finding | Number at 71737ae | Found by | The new design must |
|---|---|---|---|---|
| K1 | The case screws engage too little thread | M2 x 6: 3.2 mm in the side pilots against 4-5 mm (DEF-31) | VER-147, VER-190 | Engage 4 mm or more of thread in printed pilots, per pilot |
| K2 | A connector's mouth faces the tub's wall | J5 2 mm from the wall: the SH plug crosses it by 4.02 mm (DEF-77) | VER-165 | Leave every connector's plug and cable box clear of the walls with the board in the tub, or say which are used with the board out |
| K3 | The ESD array may be placed turned 180 degrees by JLC | a reversed U1 shorts 3V3 (DEF-14) | VER-28 | Check every polarised part on JLC's placement preview before ordering |
| K4 | Connector CPL rows were off their pads | J1 1.50 mm, J2 and J4 0.95 mm, J3 0.23 mm at be437f8; none over 0.10 mm at 71737ae (DEF-33) | VER-176, VER-28 | Write each part's pads' centre in the CPL and confirm the origins on JLC's preview |
| K5 | The fast nets cross gaps in the ground under them | the USB pair 38.7 mm with no ground under it, D+ and D- 3.6 mm apart; SR_CLK 6.9 mm against 2 mm (DEF-75) | VER-21, VER-108 | Route the USB pair side by side on one layer over unbroken ground, and keep every fast net over ground |

## Any board built by these scripts at JLC

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

## RP2354B-only, left out of the table

These apply to the RP2354B core and do not carry: the 27 ohm USB series resistors (DEF-08); the
E9 errata pull-down on PWR_HOLD and the errata firmware rules (DEF-10, DEF-71, VER-16); the
RP2354B and L2 stock (DEF-29); firmware for the RP2350 (DEF-34, VER-149, VER-151); U10, the
buffer for GPIO47's pad leakage (DEF-63's part, DF-182); the copied core's +1V1 capacitor and its
overlay on Raspberry Pi's minimal design (DEF-61's second half, VER-23, VER-197's values); the GPIO
mux and PWM slices (VER-15); the SPI bus choice itself (DEC-13; D6 keeps its frame-time rule).

Last updated: 2026-10-28 04:14 EDT
