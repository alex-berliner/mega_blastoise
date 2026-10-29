# Artifact audit of the V&V register, 2026-10-29

Read-only audit of `hardware/vv` on main (a2269e6) for leftovers from the RP2354B, ESP32-S3 and F1C200s re-targets that are not requirements of the RP2350B board with one external GD25Q128. Nothing under `hardware/vv/` was changed except this file. Spec used: `docs/static/DESIGN.md`, `docs/static/next_spin_requirements.md` and Alex's decisions of 2026-10-29.

## Summary

- Live rows audited: 383 (requirements 131, checks 195, DFMEA 55, open defects 2). Live means Status not DROPPED, RETIRED, REJECTED (checks: also not CUT; defects: also not CLOSED). Every live check and DFMEA row is board `rp2350b` or `both`.
- Verdicts: KEEP 345, REWORD 36, DROP 0, ASK 2.
- Answer to Alex's suspicion: yes, but small. The 0bc51d6 re-seed already dropped whole ESP32-S3 and F1C200s rows. What remains is leftover wording and numbers inside live rows: 36 rows need a REWORD, 2 need a decision from Alex, and no whole live row is a DROP (the dead clauses inside rows are listed as REWORD with the corrected text).
- Main leftovers by board: F1C200s (Linux, GBA, microSD slot, FUSB303B, VBUS switch, high-speed USB) in REQ-BAT-12, -PWR-02, -PWR-05, -DSP-11, -THM-04, -EMC-02, -MEC-08, -MEC-11, -TST-02, -TST-07; ESP32-S3 (WROOM module, PSRAM, U.FL antenna, TLV75733P, DM3AT slot, SW24 power key, third 74HC165, LEDC) in REQ-FAB-10, REQ-PWR-04/-09/-11, REQ-BTN-02 and VER-12, -19, -28, -29, -34, -41, -110, -112, -125, -161, -193, -198; RP2354B internal-flash design: only REQ-PRD-08 (the BOOTSEL-button recovery, 2 of 2) and none naming the 2 MB inside flash or its 2.97 V.
- Why nothing caught them: VER-188 (stale_names.py) compares live rows with design.py at the baseline, and README says that baseline is still the ESP32-S3 board (`- Baseline board: esp32s3`, main at fd66be9). So U4 the module, J8, SW24, U11 and the TLV75733P all pass as 'present in design.py'. They will fail only once the RP2350B design.py replaces the baseline.
- Parts check: of the LCSC numbers cited in live rows, only these are in neither the RP2354B board's design.py at `vv-rp2354b-final` nor C42415655, C2758105, C2067262: C114218 (DM3AT microSD slot: REQ-FAB-10, VER-41, DF-238), C3013946 (WROOM-1U module: VER-41), C485517 (TLV75733P: VER-41, DF-238), C2759975 (TPS850 power key: VER-41), C4943394 (Kinghelm flex antenna: VER-34). C530539 (the GSPK151035PN speaker, REQ-AUD-06, VER-128) is in the RP2354B board's bom.md but not its design.py because it is a bought-in part, and the speaker stays: KEEP. No live row names the RP2354B's internal flash as the current flash.
- RP-era rows: no live row was last written at or before `vv-rp2354b-final` (every row was rewritten by the ESP32-S3 re-seed at 39cc07f, or later). Note `vv-rp2354b-final` is not an ancestor of main (it is a sibling branch from 71737ae, which main's ESP32-S3 line left at 39cc07f), so the ancestor test gives zero RP rows. I therefore also report 'same text at the RP tag' (Statement, or title and procedure, or failure mode identical to the tag's): 138 live rows. Those are the RP-era rows below.

### Rows by era (last write of the Statement / title+procedure / failure mode, `git merge-base --is-ancestor` against the three tags)

| Era | Requirements | Checks | DFMEA | Defects | Total |
|---|---|---|---|---|---|
| RP | 0 | 0 | 0 | 0 | 0 |
| ESP32 | 68 | 82 | 0 | 2 | 152 |
| F1C | 28 | 50 | 27 | 0 | 105 |
| NEW | 35 | 63 | 28 | 0 | 126 |
| all | 131 | 195 | 55 | 2 | 383 |

### Verdicts by era

| Era | KEEP | REWORD | DROP | ASK |
|---|---|---|---|---|
| RP | 0 | 0 | 0 | 0 |
| ESP32 | 141 | 10 | 0 | 1 |
| F1C | 88 | 17 | 0 | 0 |
| NEW | 116 | 9 | 0 | 1 |

### RP-era sample (15 of the 138 rows with the same text at the RP tag, random.seed(20261029))

Sample: VER-188, VER-128, REQ-FAB-06, VER-49, VER-132, VER-06, REQ-MEC-05, VER-111, REQ-PWR-07, VER-02, REQ-TST-05, REQ-BTN-04, REQ-FAB-08, VER-171, REQ-AUD-06.

Result: 13 hold for the RP2350B board as written (nothing in them is flash-, boot- or latch-specific; VER-128 and REQ-AUD-06 name the speaker, which stays). REQ-PWR-07 holds but its budget lacks the TPS3421 (the ASK below). VER-188 holds but is the check that cannot see the leftovers (above). Outside the sample, the same-text list does hold one stale row: REQ-PRD-08 (REWORD). Nothing in the 138 mentions the internal 2 MB flash, the 2.97 V flash minimum or hold-to-latch.

### Sources check

34 live requirements keep a Rationale or Source that the register itself marks 'Was: on the ESP32-S3 board (tag vv-esp32s3-final), to redo for the RP2350B board'. Their statements bind and are real RP2350B requirements as written (the board keeps the BQ24073, the cell, the panel, the case and the contacts), so they are KEEP, but their rationale numbers still come from the ESP32-S3 board's documents and are due for the redo VER-103, VER-138, VER-144 and VER-235 already plan. Not in the REWORD table because the register already flags them; list under KEEP below. No live row's Source rests on the F1C200s's rails or the ESP32-S3's 90 to 120 mA budget or 10.7 mm case as a current number: REQ-PRD-17 cites the 10.7 mm only as 'Was'.

## REWORD, DROP and ASK rows

No row is a whole-row DROP. Where a clause belongs to a dropped board it is deleted inside the REWORD text.

| ID | Era | Evidence (term, part or source) | Verdict | Corrected text or question |
|---|---|---|---|---|
| REQ-AUD-03 | ESP32 (same text at the RP tag) | Rationale: 'since 39cc07f the 1.5k filter gives 9.7 kHz nominal ... VER-100: FAIL' is the ESP32-S3 board's PWM filter; the RP2354B tag's two-stage RC was -3 dB at about 14 kHz, and VER-100 leaves the source (PWM or I2S) open | REWORD (rationale) | Rationale: drop the 39cc07f filter numbers; state the chosen source's filter at its corners (VER-100). Statement unchanged. |
| REQ-BAT-12 | F1C | Statement: 'Linux running a GBA game'; Rationale/Source: the ESP32-S3's ADC1 on IO1, +-50 mV after Espressif calibration, esp32-s3.txt:3804, Wi-Fi load | REWORD | Statement: '... at the highest load the firmware draws (the battler with sound playing, backlight at full)'. Rationale and Source: redo from the RP2350's ADC (GPIO40 to 47) behind the RP2354B board's U10 MCP6001 buffer and R17/R18, as VER-138 does; delete the ESP32 ADC figures. |
| REQ-BTN-02 | ESP32 (same text at the RP tag) | Rationale: 'the power key SW24 carries VSYS on one pad' (ESP32-S3 board) | REWORD (rationale) | Rationale: 'the power slider's contact carries a rail on one pad, so leakage across it turns the device on (REQ-PWR-11); on the others it reads as a press'. |
| REQ-DSP-06 | ESP32 (same text at the RP tag) | Rationale: BL_PWM on 'IO9's LEDC' (ESP32-S3 peripheral); the RP2354B/RP2350B backlight reference is GPIO28 PWM6 A | REWORD (rationale) | Rationale: replace 'IO9's LEDC' by 'the RP2350B's PWM slice on BL_PWM'; re-run the committed simulation with the board's R61. Statement unchanged. |
| REQ-DSP-11 | F1C | Statement: 'so a GBA frame (240 x 160, scaled) reaches the panel'; Rationale: PSRAM holds the 307,200 B frame, esp32s3_board_proposal.md (the F1C200s/ESP32-S3 frame store). The RP2350B streams from SRAM over PIO | REWORD | Statement: 'A full-screen 320 x 480 RGB565 redraw over the 8-bit 8080 bus shall take 16.7 ms or less (60 fps).' Rationale: 25 MHz moves a frame in 12.3 ms of bus time; no frame store beyond the 520 KB SRAM (VER-224). |
| REQ-EMC-02 | F1C | Statement: 'its USB-C high-speed lines in use'; Rationale: the device transmits (Wi-Fi), WROOM-1U modular grant and antenna type. The RP2350B USB is full speed and the board has no radio | REWORD | Statement: '... at a test house, its USB-C full-speed lines in use; the board has no radio.' Rationale: delete the modular-grant text (the intentional-radiator clause); an unintentional radiator needs the Part 15 Subpart B declaration only. |
| REQ-FAB-10 | NEW | Statement: 'the RP2350B (C42415655), the GD25Q128 (C2758105) and the DM3AT (C114218) shall never be substituted'; C114218 is the Hirose DM3AT microSD slot (J8) of the ESP32-S3 and F1C200s boards; Rationale: PSRAM and 1.8 V module substitution (DF-238) | REWORD | Statement: '... the RP2350B (C42415655), the GD25Q128 (C2758105) and the TPS3421 (C2067262) shall never be substituted.' Rationale: replace the module/PSRAM sentence by the flash-size and hard-reset-delay variants (DF-238, DF-262). |
| REQ-MEC-08 | F1C | Statement lists 'the microSD opening' among the allowed through-openings (F1C200s and ESP32-S3 boards' DM3AT slot) | REWORD | Statement: '... other than the listed ones (USB-C, the power slider's slot, cap holes, speaker grille, screw holes)'. |
| REQ-MEC-11 | F1C | Statement: 'the microSD slot's body shall clear its opening in the case by 0.2 mm a side'; Rationale: SD slot 13.85 mm in a 14.0 mm opening at 39cc07f (VER-165, DF-233) | REWORD | Statement: delete the microSD clause: 'Every connector's mouth (USB-C with a mated plug, the cell's plug and wires, the panel's FPC lead-in, the speaker's plug and its leads) shall be clear of parts taller than 1 mm and of every case wall but its own opening.' Rationale: drop the SD sentence. |
| REQ-PRD-04.B | NEW | Rationale 'the charger is not chosen yet (the BQ24074 of ref_design is the candidate)'; the RP2354B board's charger is the BQ24073 (C15220) and REQ-BAT-01/-03/-13, VER-109, VER-133 all cite the BQ24073 | REWORD | Rationale: 'the charger is the RP2354B board's BQ24073 (C15220), kept on the RP2350B board'. Statement unchanged. |
| REQ-PRD-08 | ESP32 (same text at the RP tag) | statement says 'enter the USB boot loader ... 2 out of 2', the RP2354B board's BOOTSEL-button path; contradicts its own allocation REQ-PRD-08.B (hard reset, Select at power-on, failed-update fall back, 3 of 3); Source validation_plan #182, #202 is the old BOOT button plan | REWORD | Statement: 'A unit with blank, corrupt, failed-update or hung firmware shall be recovered with the case closed, 3 out of 3 tries each, and shall take a debugger with the case open.' Source: DESIGN.md:63 and REQ-PRD-08.B. |
| REQ-PWR-02 | F1C | Statement lists 'the FUSB303B's, the VBUS switch's' pins; both belonged to the F1C200s board (host/device role controller and VBUS-out switch). The console is a device only with no VBUS out | REWORD | Statement: '... every pin fed from VBUS (the charger's, the ESD array's, the CC pull-downs') shall stay 0.3 V or more under its absolute maximum.' |
| REQ-PWR-04 | F1C | Rationale and Source rest on the WROOM module's VDD33 3.0-3.6 V (esp32-s3-wroom-1.txt:1596), the TLV75733P (tlv757p.txt) and Wi-Fi; U3 on the RP2354B board is the RT9013-33GB (C47773) | REWORD | Statement unchanged (3.30 V +-3%). Rationale and Source: the RP2350's IOVDD range, the panel's VCI 3.3 V maximum (er-tft035ips-6.txt:747-748) and the RT9013-33GB's accuracy; re-derive in VER-235. |
| REQ-PWR-05 | F1C | Statement: 'Linux running a GBA game'; Rationale: module VDD33 minimum 3.0 V and the TLV75733P's dropout giving 2.976 V (ESP32-S3 numbers) | REWORD | Statement: '... at full load (backlight 100%, sound playing, the battler running) every rail shall stay inside its range (REQ-PWR-20) and the device shall not reset.' Rationale: the cell less the BQ24073's FET and the RT9013's dropout against the RP2350's and the panel's minimum (VER-235). |
| REQ-PWR-07 | ESP32 (same text at the RP tag) | The 30 uA / 60 uA budget (26.8 uA at 4.2 V, VER-103) was worked out for the board without a TPS3421. A push-button controller on the slider's line must be powered while the unit is off to see the slide | ASK | Question for Alex: is the TPS3421 fed from VSYS/cell (always on, so its quiescent current counts against 30 uA and REQ-PRD-05's 2 years) or from 3V3 (off when off, so the slider needs another path to turn the unit on)? Rows to update with the answer: REQ-PWR-07, REQ-PRD-05, VER-65, VER-103, VER-146. |
| REQ-PWR-09 | F1C | Rationale/Source cite IO21, R42 and esp32-s3.txt:3748 (the ESP32-S3's power-hold pin); the RP2350B's hold pin is PWR_HOLD (GPIO44) with the pull-down of DEF-10 / RP2350-E9 | REWORD (rationale) | Rationale: 'PWR_HOLD resets as an input; its external pull-down holds it low (DEF-10, RP2350-E9, VER-243)'. Source: design.py PWR_HOLD pull-down; pinmap.md Power control. Statement unchanged. |
| REQ-PWR-11 | F1C | Rationale/Source rest on SW24 (the ESP32-S3 board's power key, TPS850 tact switch) and R13/R15/SW24 in design.py | REWORD (rationale) | Rationale: 'the power slider's contact (or seat 1's Start, if it is the power key) carries a rail on one pad; sweat or flux can bridge it (DF-242)'. Source: VER-239's output once the switch is chosen. Statement unchanged. |
| REQ-THM-04 | F1C | Statement: 'Charging while playing GBA under Linux'; Rationale: 3.31 W at VER-31's Wi-Fi load, U3 0.43 W, 'the module 0.74 W' (ESP32-S3 module); the 2.0 W limit was set for that load | REWORD | Statement: 'Charging while playing (5.5 V source, 3.0 V cell, backlight and audio at full scale) shall put 2.0 W or less inside the case ...'. Rationale: redo from VER-144's table for the BQ24073, RT9013 and the RP2350B; 2.0 W stays Claude's proposal until VER-144 gives the real sum. |
| REQ-TST-02 | F1C | Statement lists 'the card, the USB-C roles'; no SD card, and the USB-C has one role (device) | REWORD | Statement: '... an audio tone, the flash's JEDEC ID and its 16 MB readback, the USB-C device enumeration, the cell reading and the charger's states ...' (as VER-76 and VER-242 already say). |
| REQ-TST-07 | F1C | Statement counts 'the microSD slot' among the connectors with a mating budget | REWORD | Statement: 'No shipped unit shall have any connector (the cell's, the panel's, the speaker's) mated more than 50% of its rated cycles, counted per unit.' |
| VER-05 | NEW | procedure names 'paper_checks.py backlight, the ESP32-S3 board's two-sink model, is the template' and the 'IO9' PWM | REWORD | Procedure: name the RP2354B board's MCP6002 sink circuit (tag vv-rp2354b-final) as the circuit, and GPIO28 PWM6 A as the drive; keep the pass limits. |
| VER-12 | NEW | pass limit: 'PWR_EN <= 5.5 V at every instant (the TLV75733P's EN recommended maximum, tlv757p.txt:208)'; U3 is the RT9013-33GB (C47773) | REWORD | Pass limit: 'PWR_EN <= the RT9013's EN rating less 0.3 V' with the RT9013 datasheet line; the same 5.5 V sits in VER-71's limit by reference ('as VER-12'). |
| VER-19 | NEW | procedure places 'the power key SW24 at case.scad's pwr_pos' (ESP32-S3 board's power key) | REWORD | Procedure: 'the power slider's switch (or seat 1's Start if it is the power key, REQ-PWR-21) at case.scad's pwr_pos'. |
| VER-28 | ESP32 | procedure and pass limit list U4 'the module: its pin 1 and its U.FL end', U11, J8 and SW24: the WROOM-1U, the third 74HC165, the DM3AT slot and the ESP32-S3 power key | REWORD | Procedure and pass limit: U4 is the RP2350B (QFN-80, pin 1 dot); add the GD25Q128 and the TPS3421 to the polarised list; delete J8 and SW24; keep U11 only if the pin plan (VER-232) keeps a third 74HC165 (see the ASK on VER-199). |
| VER-29 | ESP32 | rotation table lists 'the WROOM-1U module', 'the DM3AT' and 'the TPS850' footprints (ESP32-S3 board parts) | REWORD | Procedure: footprint list becomes QFN-80 (RP2350B), the GD25Q128's package, the TPS3421's package, SOT-23, SOIC-8, TSSOP-16, SOD-323, FH12, PicoBlade, JST SH. |
| VER-34 | NEW | needs: 'the flex antenna's datasheet (Kinghelm C4943394 ...)' and 'u3.json rewritten for the TLV75733P' (ESP32-S3 board's antenna and LDO); C4943394 is in no RP2354B BOM | REWORD | Procedure and needs: drop the antenna datasheet and the TLV75733P note; list the RT9013-33GB (C47773), the RP2350B, the GD25Q128 and the TPS3421 datasheets (VER-233). |
| VER-41 | ESP32 | procedure: 'the parts new at 39cc07f first (the module C3013946, the TLV75733P C485517, the DM3AT C114218, the TPS850 C2759975)'; pass limit 'U4, U3, J8 and SW24 are the designed parts'. All four LCSC numbers are in no RP2354B BOM | REWORD | Procedure: 'the parts new on this board first (the RP2350B C42415655, the GD25Q128 C2758105, the TPS3421 C2067262)'. Pass limit: 'U4 (RP2350B), U5 (flash), U3 and the power switch are the designed parts'. |
| VER-110 | F1C | procedure: 'the module's MAC, flash and PSRAM IDs (VER-242) ... mating counts for J2, J3, J4 and J8' | REWORD | Procedure: 'the RP2350B's chip ID, the flash's JEDEC ID (VER-242) ... mating counts for J2, J3 and J4'. |
| VER-112 | ESP32 | procedure heights: 'U4 3.2 +-0.15 (esp32-s3-wroom-1.txt:3130)', U11, 'SW24 1.4 (rocpu_tps850.txt:33)', 'J8 1.68 (hirose_dm3 ...)' | REWORD | Procedure: replace U4, SW24 and J8 by the RP2350B's QFN-80, the GD25Q128's and the TPS3421's drawn heights; U11 only if kept. |
| VER-125 | F1C | procedure lists 'U4 (the WROOM-1U's land pattern, Figure 11-2: 40 x 1.5 x 0.9 pads ...)', U11, U3 as 'the TLV757P's DBV0005A', J8 and SW24 (rocpu_tps850.pdf) | REWORD | Procedure: U4 is the RP2350B QFN-80 against Raspberry Pi's footprint (as VER-254), add the GD25Q128 and the TPS3421, U3 is the RT9013's SOT-23-5; delete the WROOM, J8 and SW24 lands. |
| VER-161 | ESP32 | procedure: 'design.ESP32 through the module's pin table'; 'R42' and 'pinmap.md's RESET button' examples (ESP32-S3 board) | REWORD | Procedure: read the RP2350B's pin file (VER-149) instead of design.ESP32; examples from the RP2350B design. |
| VER-193 | NEW | pass limit: 'the module's air at or under its 65 C ambient' (the WROOM-1U module's rating) | REWORD | Pass limit: 'every part at or under its maximum less 20 C and the RP2350B's case temperature under its datasheet limit (REQ-SOC-07)'; delete the module clause. |
| VER-198 | NEW | title and procedure list 'U11, ... J8's bus' (the third 74HC165 and the DM3AT's SD bus) | REWORD | Title and procedure: 'U2, U3, U4 (RP2350B), U5 (GD25Q128), the TPS3421, U8, U9, the panel'; delete J8's bus; U11 only if kept. |
| VER-199 | NEW | pass limit and title cover 'the button scan' and 'SR_CLK' (the 74HC165 chain clock). The RP2354B board chained 2 x 74HC165 for 16 buttons; the ESP32-S3 board chained 3 for 24 bits. The RP2350B has 48 GPIOs (REQ-PRD-01.B says 'directly, or a chain') | ASK | Question for Alex: should the 20 contacts and the slider be read on RP2350B GPIOs directly (no 74HC165, no scan clock, REQ-BTN-10 becomes a GPIO poll) or through the RP2354B board's 2 chained 74HC165 plus direct Start/Select? VER-232 (pin plan) decides it; until then VER-198, VER-199, VER-28, VER-112, VER-125 carry U8/U9/U11 by assumption. |
| DF-230 | F1C | prevention: 'the DS Lite combs' footprint and keep-out as on the ESP32-S3 board'; the combs are the RP2354B board's too | REWORD (wording) | Prevention: 'the DS Lite combs' footprint and keep-out as on ref_design (the same combs on the RP2354B and ESP32-S3 boards)'. |
| DF-233 | F1C | item and failure mode: 'Connector mouths and the card's opening ... the card does not seat' | REWORD | Item: 'Connector mouths'. Failure mode: 'a mouth too tight in the case'. Effect: 'a plug does not seat'. Delete the card. |
| DF-238 | NEW | failure mode: 'the DM3AT C114218' is in the substitution list (microSD slot of the ESP32-S3/F1C200s boards) | REWORD | Failure mode: '(the RP2350B C42415655, the GD25Q128 C2758105 for a smaller or another flash, the TPS3421 C2067262 for another delay variant)'. |
| DEF-47 | ESP32 | open defect; Status text says 'the register now describes 71737ae' and 'Gate ORDER'; README says the baseline is main's ESP32-S3 design (fd66be9) and the register targets the RP2350B | REWORD | Status: 'OPEN: the register describes the RP2350B board, but design.py on main is still the ESP32-S3 board's (README, Baseline board: esp32s3), so the drift check cannot yet compare them; FIXED again when scripts/check.sh runs check_trace.py (PLAN.md step 0.3) and the RP2350B design replaces the baseline.' |

## KEEP rows

KEEP (345): REQ-ASM-01, REQ-ASM-01.B, REQ-ASM-02, REQ-AUD-01, REQ-AUD-04, REQ-AUD-05, REQ-AUD-06, REQ-BAT-01, REQ-BAT-02, REQ-BAT-02.B, REQ-BAT-03, REQ-BAT-04, REQ-BAT-05, REQ-BAT-05.B, REQ-BAT-06, REQ-BAT-07, REQ-BAT-08, REQ-BAT-09, REQ-BAT-10, REQ-BAT-11, REQ-BAT-13, REQ-BAT-14, REQ-BAT-15, REQ-BAT-16, REQ-BTN-01, REQ-BTN-03, REQ-BTN-04, REQ-BTN-05, REQ-BTN-06, REQ-BTN-07, REQ-BTN-08, REQ-BTN-10, REQ-DSP-01, REQ-DSP-02, REQ-DSP-03, REQ-DSP-04, REQ-DSP-05, REQ-DSP-07, REQ-DSP-09, REQ-DSP-10, REQ-EMC-01, REQ-FAB-01, REQ-FAB-02, REQ-FAB-03, REQ-FAB-04, REQ-FAB-05, REQ-FAB-06, REQ-FAB-07, REQ-FAB-07.B, REQ-FAB-08, REQ-FAB-09, REQ-FAB-11, REQ-FAB-12, REQ-MEC-01, REQ-MEC-02, REQ-MEC-03, REQ-MEC-04, REQ-MEC-05, REQ-MEC-06, REQ-MEC-07, REQ-MEC-09, REQ-MEC-10, REQ-MEC-12, REQ-PRD-01, REQ-PRD-01.B, REQ-PRD-02, REQ-PRD-02.B, REQ-PRD-03, REQ-PRD-03.B, REQ-PRD-04, REQ-PRD-05, REQ-PRD-07, REQ-PRD-08.B, REQ-PRD-09, REQ-PRD-10, REQ-PRD-11, REQ-PRD-14, REQ-PRD-15, REQ-PRD-17, REQ-PRD-18, REQ-PRD-19, REQ-PWR-01, REQ-PWR-10, REQ-PWR-12, REQ-PWR-13, REQ-PWR-14, REQ-PWR-16, REQ-PWR-17, REQ-PWR-20, REQ-PWR-21, REQ-SOC-01, REQ-SOC-02, REQ-SOC-03, REQ-SOC-04, REQ-SOC-05, REQ-SOC-06, REQ-SOC-07, REQ-SOC-08, REQ-SOC-09, REQ-SOC-10, REQ-THM-01, REQ-THM-02, REQ-THM-03, REQ-TST-01, REQ-TST-03, REQ-TST-04, REQ-TST-05, REQ-TST-06, REQ-TST-08, REQ-USB-01, REQ-USB-02, VER-01, VER-02, VER-03, VER-04, VER-06, VER-07, VER-08, VER-13, VER-14, VER-17, VER-18, VER-20, VER-21, VER-22, VER-26, VER-27, VER-30, VER-31, VER-32, VER-35, VER-36, VER-37, VER-38, VER-39, VER-40, VER-42, VER-44, VER-45, VER-46, VER-48, VER-49, VER-50, VER-51, VER-52, VER-53, VER-54, VER-56, VER-60, VER-61, VER-62, VER-63, VER-64, VER-65, VER-66, VER-67, VER-68, VER-69, VER-70, VER-71, VER-74, VER-76, VER-80, VER-81, VER-82, VER-83, VER-85, VER-86, VER-87, VER-88, VER-89, VER-90, VER-91, VER-92, VER-93, VER-94, VER-95, VER-96, VER-97, VER-98, VER-99, VER-100, VER-101, VER-102, VER-103, VER-104, VER-105, VER-106, VER-108, VER-109, VER-111, VER-113, VER-114, VER-116, VER-117, VER-118, VER-126, VER-128, VER-129, VER-130, VER-131, VER-132, VER-133, VER-134, VER-135, VER-136, VER-137, VER-138, VER-139, VER-140, VER-141, VER-142, VER-143, VER-144, VER-145, VER-146, VER-147, VER-148, VER-149, VER-152, VER-153, VER-154, VER-155, VER-156, VER-157, VER-158, VER-159, VER-160, VER-162, VER-164, VER-165, VER-166, VER-167, VER-168, VER-169, VER-170, VER-171, VER-172, VER-173, VER-174, VER-176, VER-177, VER-178, VER-180, VER-181, VER-182, VER-184, VER-186, VER-188, VER-189, VER-190, VER-192, VER-195, VER-196, VER-200, VER-201, VER-202, VER-218, VER-219, VER-220, VER-223, VER-224, VER-227, VER-228, VER-230, VER-231, VER-232, VER-233, VER-235, VER-236, VER-237, VER-238, VER-239, VER-240, VER-241, VER-242, VER-243, VER-244, VER-245, VER-248, VER-249, VER-251, VER-252, VER-253, VER-254, VER-255, VER-256, VER-257, VER-258, VER-259, VER-260, VER-261, DF-205, DF-206, DF-210, DF-211, DF-212, DF-213, DF-214, DF-215, DF-216, DF-217, DF-218, DF-219, DF-220, DF-221, DF-222, DF-223, DF-224, DF-225, DF-226, DF-227, DF-228, DF-229, DF-231, DF-232, DF-234, DF-235, DF-236, DF-237, DF-239, DF-240, DF-241, DF-242, DF-246, DF-247, DF-248, DF-249, DF-250, DF-251, DF-252, DF-253, DF-254, DF-255, DF-256, DF-257, DF-258, DF-259, DF-260, DF-261, DF-262, DF-263, DF-264, DF-265, DEF-44.

Of these, 34 are KEEP with a rationale still to redo (marked 'Was: ... ESP32-S3 board' by the register): REQ-AUD-05, REQ-AUD-06, REQ-BAT-01, REQ-BAT-03, REQ-BAT-08, REQ-BAT-09, REQ-BAT-10, REQ-BAT-13, REQ-BAT-14, REQ-BTN-01, REQ-BTN-08, REQ-DSP-01, REQ-DSP-02, REQ-DSP-03, REQ-DSP-05, REQ-DSP-07, REQ-FAB-01, REQ-FAB-02, REQ-FAB-04, REQ-FAB-09, REQ-MEC-09, REQ-MEC-10, REQ-MEC-12, REQ-PRD-05, REQ-PRD-10, REQ-PWR-01, REQ-PWR-10, REQ-PWR-12, REQ-PWR-13, REQ-PWR-16, REQ-PWR-17, REQ-THM-02, REQ-THM-03, REQ-TST-06.

Rows that mention a dropped board only as history and so stay KEEP: REQ-PRD-01 and REQ-TST-08 (prod_board notes), REQ-PRD-09 (the microSD card it used to count), REQ-PRD-14, -15, -SOC-01, -SOC-02, -USB-01, -USB-02 ('It was the F1C200s board's' notes), REQ-PRD-19 and VER-259, VER-260 ('Linux' as a host operating system for the USB drive), VER-189 ('Was:' list), DF-239, DF-241, DF-247 to DF-251, DF-214, DF-217, DF-219 ('Was:' lineage), DF-257, DF-258 (RP2354B copied core), DEF-44, and every 'stripboard' row (the Pico 2 prototype used by the chip gate).

## Out of scope, noted

`bridges.csv` (pad pairs of the baseline design: U11, U8, U9, the module's pins) and `grid.md` / `gates.md` / `carryover.md` (66, 15, 14 lines naming ESP32-S3, F1C200s, Linux, microSD, DM3AT, SW24 or J8) were not audited row by row; bridges.csv is generated from the baseline design.py and will be regenerated when the RP2350B design.py lands.

## Scripts

Run from the repo root with `python3 -I`. `era.py` writes `/tmp/audit/eras.json`; `scan.py` needs it and writes `scan.json`; `scan2.py` is the supplementary scan; `parts.py` checks LCSC numbers. The scripts are kept in `/tmp/audit/` and reproduced here.

### era.py: era of each row

```python
#!/usr/bin/env python3
"""Era of each live register row: walk the file's history (oldest first), record the
commit where the era-defining fields last changed, classify it by tag ancestry."""
import csv, io, re, subprocess, sys, json
R = "/home/alex/Code/mega_blastoise_vv"
def git(*a):
    return subprocess.run(["git", "-C", R, *a], capture_output=True, text=True, check=True).stdout
TAGS = [("RP", "vv-rp2354b-final"), ("ESP32", "vv-esp32s3-final"), ("F1C", "vv-f1c200s-final")]
_cache = {}
def era(sha):
    if sha in _cache: return _cache[sha]
    e = "NEW"
    for name, tag in TAGS:
        if subprocess.run(["git","-C",R,"merge-base","--is-ancestor",sha,tag]).returncode == 0:
            e = name; break
    _cache[sha] = e; return e
def commits(path):
    return git("log", "--reverse", "--no-merges", "--topo-order", "--format=%H", "main", "--", path).split()
def blob(sha, path):
    r = subprocess.run(["git","-C",R,"show",f"{sha}:{path}"], capture_output=True, text=True)
    return r.stdout if r.returncode == 0 else ""
def csv_rows(text, idcol, keys):
    out = {}
    for r in csv.DictReader(io.StringIO(text)):
        out[r[idcol]] = (tuple(r[k] for k in keys), r)
    return out
def md_blocks(text, hdr, field):
    out = {}
    for m in re.finditer(r"^### (" + hdr + r")[: ](.*?)(?=^### |\Z)", text, re.S | re.M):
        body = m.group(0); i = m.group(1)
        f = re.search(r"^- " + field + r": (.*(?:\n(?!- |\n|###).*)*)", body, re.M) if field else None
        out[i] = ((m.group(2).split("\n")[0] + (f.group(1) if f else "")), body)
    return out
def track(path, parse):
    last = {}; cur = {}
    for sha in commits(path):
        new = parse(blob(sha, path))
        for i, (key, row) in new.items():
            if i not in cur or cur[i] != key:
                last[i] = sha
            cur[i] = key
        for i in list(cur):
            if i not in new: del cur[i]
    return last
def main():
    P = "hardware/vv/"
    res = {}
    res["REQ"] = track(P+"requirements.md", lambda t: md_blocks(t, r"REQ-[A-Z]+-\d+(?:\.[A-Z])?", "Statement"))
    res["VER"] = track(P+"verification.csv", lambda t: csv_rows(t, "id", ["title","procedure"]))
    res["DF"]  = track(P+"dfmea.csv", lambda t: csv_rows(t, "id", ["failure_mode","item"]))
    res["DEF"] = track(P+"defects.md", lambda t: md_blocks(t, r"DEF-\d+", "Symptom"))
    out = {k: {i: [s, era(s)] for i, s in v.items()} for k, v in res.items()}
    json.dump(out, open("/tmp/audit/eras.json","w"))
    from collections import Counter
    for k, v in out.items(): print(k, Counter(e for _, e in v.values()))
main()
```

### scan.py: term scan (the brief's 47 terms) and live-row filter

```python
#!/usr/bin/env python3
"""Term scan, part scan and era table over the LIVE register rows (HEAD of main)."""
import csv, io, json, re, subprocess, sys
R = "/home/alex/Code/mega_blastoise_vv"; P = R + "/hardware/vv/"
def git(*a): return subprocess.run(["git","-C",R,*a],capture_output=True,text=True,check=True).stdout
TERMS = ["ESP32","S3","WROOM","PSRAM","LCD_CAM","LEDC","strapping","IO0","F1C","Allwinner","V3s","Linux","gpSP",
 "GBA ROM","emulator","microSD","SD card","SDMMC","FEL","FUSB","OTG","host mode","VBUS out","boost","Wi-Fi","WiFi",
 "antenna","BLE","RP2354","2 MB","internal flash","ST7796","stripboard","prod_board","4-layer","island","cellcut",
 "buck-boost","ME6211","AP3156","TLV75733P","J7 BOOT","DDR","90 ohm","high speed"]
# word-ish match for short tokens so 'S3' does not hit 'RS3', 'FEL' not 'FELT'
def rx(t): return re.compile(r"(?<![A-Za-z0-9_])" + re.escape(t) + r"(?![A-Za-z0-9_])" if len(t) <= 4 or t in ("boost",) else re.escape(t), re.I)
RX = {t: rx(t) for t in TERMS}
DEAD = ("DROPPED","RETIRED","REJECTED","CUT")
eras = json.load(open("/tmp/audit/eras.json"))
def rows():
    t = open(P+"requirements.md").read()
    for m in re.finditer(r"^### (REQ-[A-Z]+-\d+(?:\.[A-Z])?):(.*?)(?=^### |\Z)", t, re.S|re.M):
        st = re.search(r"^- Status: (\S+)", m.group(0), re.M)
        if st and st.group(1).strip(" .,;") in DEAD: continue
        yield "REQ", m.group(1), m.group(0)
    for r in csv.DictReader(open(P+"verification.csv")):
        if r.get("last_result","").split()[:1] and r["last_result"].split()[0] in DEAD: continue
        if re.search(r"\b(DROPPED|RETIRED|REJECTED)\b", r["title"]+r["last_result"][:20]): continue
        yield "VER", r["id"], " | ".join(f"{k}: {v}" for k,v in r.items() if v)
    for r in csv.DictReader(open(P+"dfmea.csv")):
        if r["status"].split()[:1] and r["status"].split()[0] in DEAD: continue
        yield "DF", r["id"], " | ".join(f"{k}: {v}" for k,v in r.items() if v)
    t = open(P+"defects.md").read()
    for m in re.finditer(r"^### (DEF-\d+):(.*?)(?=^### |\Z)", t, re.S|re.M):
        st = re.search(r"^- Status: (\S+)", m.group(0), re.M)
        s = st.group(1).strip(" .,;:") if st else ""
        if s in DEAD or s == "CLOSED": continue
        yield "DEF", m.group(1), m.group(0)
if __name__ == "__main__":
    live = list(rows())
    out = {}
    for kind, i, text in live:
        hits = sorted({t for t, r in RX.items() if r.search(text)})
        parts = sorted(set(re.findall(r"\bC\d{4,8}\b", text)))
        out[i] = {"kind":kind,"era":eras[kind][i][1] if i in eras[kind] else "?","hits":hits,"parts":parts}
    json.dump(out, open("/tmp/audit/scan.json","w"), indent=1)
    from collections import Counter
    print(len(live), Counter(v["kind"] for v in out.values()))
    print("era", Counter(v["era"] for v in out.values()))
    print("flagged", sum(1 for v in out.values() if v["hits"]))
    tc = Counter(t for v in out.values() for t in v["hits"]); print(tc)
```

### scan2.py: supplementary terms found while reading

```python
#!/usr/bin/env python3
"""Supplementary scan: terms beyond the brief that the reading turned up."""
import sys,json,re
sys.path.insert(0,"/tmp/audit"); import scan
EXTRA=["2.97","BOOTSEL","BOOT button","hold-to-latch","latch","GBA","FireRed","SW24","U11","J8","DM3AT","TPS850","IO21","IO1","IO9","R42","74HC165","SR_CLK","SR_DATA","SR_PL","U.FL","module","the card","SPI","UART","USB CDC","TLV757","MCP6001","pinmap.md"]
def rxx(t): return re.compile(r"(?<![A-Za-z0-9_])"+re.escape(t)+r"(?![A-Za-z0-9_])",re.I)
R={t:rxx(t) for t in EXTRA}
out={}
for k,i,t in scan.rows():
    h=[x for x,r in R.items() if r.search(t)]
    if h: out[i]=h
json.dump(out,open("/tmp/audit/scan2.json","w"))
from collections import Counter
print(Counter(x for v in out.values() for x in v))
for term in sys.argv[1:]:
    print(term,[i for i,v in out.items() if term in v])
```

### parts.py: LCSC part check against the RP2354B BOM

```python
#!/usr/bin/env python3
import json, re, subprocess
R="/home/alex/Code/mega_blastoise_vv"
def g(*a): return subprocess.run(["git","-C",R,*a],capture_output=True,text=True).stdout
tag="vv-rp2354b-final"
pat=re.compile(r"\bC\d{4,8}\b")
des=set(pat.findall(g("show",f"{tag}:hardware/ref_design/board/design.py")))
bom=set(pat.findall(g("show",f"{tag}:hardware/ref_design/docs/bom.md")))
allowed=des|{"C42415655","C2758105","C2067262"}
d=json.load(open("/tmp/audit/scan.json"))
print("design.py parts",len(des),"bom.md extra",sorted(bom-des))
bad={}
for i,v in d.items():
    for p in v["parts"]:
        if p not in allowed: bad.setdefault(p,[]).append(i)
for p,ids in sorted(bad.items()): print(p, "in bom.md only" if p in bom else "NOT in BOM", ids)
```

Term hits across live rows (brief's list): ESP32 80, S3 77, stripboard 15, Linux 9, WROOM 7, microSD 6, Wi-Fi 5, 2 MB 5, TLV75733P 5, RP2354 4, SD card 3, PSRAM 3, antenna 3, prod_board 2, OTG 1, 90 ohm 1, LEDC 1, host mode 1, ME6211 1. 115 rows hit at least one term; the rest of the table above comes from reading ESP32 and F1C era rows without a hit and from the supplementary scan.

Last updated: 2026-10-29 12:21:49 (audit run on main at a2269e6).
