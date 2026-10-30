# Number sources, batch 2 (REQ-DSP, REQ-AUD, REQ-BTN, REQ-MEC, REQ-THM), 2026-10-30

Queue issue #89, batch 2. The `- Numbers:` field (format and rules: README.md, 'Where a requirement's numbers come from'; first batch: analysis/number_sources_2026-10-30.md) is now on every live row of REQ-DSP, REQ-AUD, REQ-BTN, REQ-MEC and REQ-THM. No Statement was changed (`git diff` shows only added lines). Source order followed Alex's policy of 2026-10-30: the part's datasheet, then a standard, then Alex's spec, then arithmetic, last judgment (or count).

Every `datasheet:` quote was read in the `pdftotext -layout` text and is checked on its line by tools/check_trace.py. The one IEC 62368-1 text on hand is edition 1 (2010), Alex's local copy; it is not in the repository, and only the quoted value of each `standard:` tag is written here. Each `standard:` tag names it as `IEC 62368-1:2010 (ed. 1, superseded; current is ed. 4, 2023)`.

## 1. Counts

41 live rows (REQ-DSP-01..07, 09..11; AUD-01, 03..07; BTN-01..08, 10; MEC-01..12; THM-01..04), 128 numbers in 120 entries (plus 2 `in:` inputs, both judgment). DSP-08 and -12, AUD-02 and BTN-09 are DROPPED and carry no field.

| Tag | Numbers | Entries |
|---|---|---|
| datasheet | 22 | 17 |
| standard | 4 | 4 |
| spec | 1 | 1 |
| derived | 14 | 13 |
| judgment | 72 | 70 |
| count | 15 | 15 |

`standard` is used for the first time: THM-01's 48 C (Table 42) and the 25 C room ambient of IEC 62368-1 B.1.7 (THM-01, THM-03, AUD-04). `judgment` is again the largest tag; the mechanics rows (MEC) and buttons rows (BTN) have no datasheet or standard behind them, only the case drawing and the membrane notes.

## 2. The `judgment:` numbers

Each has the full reason, with what looser or tighter would cost, in its row's Numbers field. The reason below is the first clause of it.

| Row | Value | Reason |
|---|---|---|
| REQ-DSP-04 | 0.5-3 mm | Rationale "too short cannot mate, too long bows under the panel", no source gives the fold slack |
| REQ-DSP-06 | 5 us | Rationale "a design choice (Claude's)", the panel publishes no pulse rating |
| REQ-DSP-07 | 5% | Claude's brightness-sag limit (O-21), no source on disk sets it |
| REQ-DSP-07 | 10% | Claude's limit for the worst LED in the newer datasheet's Vf range |
| REQ-DSP-07 | 3.4 V | the Statement names the newer datasheet's maximum, that datasheet is not on disk (unread) and the panel datasheet on disk gives 3.2 V maximum (er-tft035ips-6.txt:713) |
| REQ-DSP-09 | 5 mA | 4% of the panel's 120 mA maximum (er-tft035ips-6.txt:714), a design choice (Rationale) |
| REQ-DSP-09 | 10 us | a design choice (Rationale), a flash this short is not expected to be visible (Rationale) |
| REQ-DSP-10 | 60 Hz | the frame rate the battler's display loop targets (Claude's reading, Alex to confirm per the Rationale), the ILI9488 datasheet's own frame-rate example is 60.76 Hz (ili9488.txt:10841) |
| REQ-DSP-10 | +-10% | 54 to 66 Hz, wide enough to read a TE pulse on a scope, narrow enough to tell a missed pulse (Rationale) |
| REQ-DSP-11 | 60 fps | Claude's estimate, Alex sets it (DEC-13, Source) |
| REQ-AUD-01 | 1 kHz | the usual single-tone test frequency for an amplifier's output power, and the FM8002A datasheet's power table uses it (fm8002a.txt:126 "f=1KHZ") |
| REQ-AUD-01 | 0.35 W | Claude's floor under the designed 0.4-0.5 W on the cell (Rationale) |
| REQ-AUD-01 | 8 ohm | the speaker's impedance as bom.md records it (INGHAi GSPK151035PN), its datasheet is not on disk (unread) |
| REQ-AUD-01 | 3.8 V | a cell near its middle (nominal 3.7 V, full 4.2 V), the Rationale says the amplifier clips below a 3.7 V cell |
| REQ-AUD-03 | -3 dB | the half-power corner is the convention for a bandwidth |
| REQ-AUD-03 | 10 kHz | Rationale "a 15 mm speaker reproduces little above 10 kHz" and it is under the 16 kHz Nyquist of the 32 kHz mixer (REQ-AUD-07) |
| REQ-AUD-03 | 50 Hz | the low end of what a 15 mm speaker plays (Rationale) |
| REQ-AUD-04 | 10 min | the test that heats the part to steady state (Rationale) |
| REQ-AUD-04 | 1 kHz | the usual single-tone test frequency, the same tone as REQ-AUD-01 |
| REQ-AUD-05 | 10 uA | the datasheet gives only a typical power-down leakage IOFF of 4.2 uA (fm8002a.txt:123 "4.2 uA") and no maximum, 10 uA is about 2.4 times the typical |
| REQ-AUD-05 | 50 ms | Claude's limit for the amplifier waking before the first sound, the datasheet gives a wake time TD of 100 ms typical (fm8002a.txt:133 "100 ms", as the Rationale says) which is twice this Statement, so on paper the Statement is not |
| REQ-AUD-05 | 10 mV | Claude's figure for a pop too small to hear on an 8 ohm speaker (no source on disk) |
| REQ-AUD-06 | 0.5 W | the INGHAi GSPK151035PN's rating as bom.md records it, the speaker's datasheet is not on disk (unread) |
| REQ-AUD-06 | 8 ohm | the same speaker's impedance, as bom.md records it (datasheet unread) |
| REQ-AUD-07 | 40 dB | Claude's engineering number, kept by Alex's decision of 2026-10-29 (Source) |
| REQ-AUD-07 | 1 kHz | the usual single-tone reference level, the same frequency REQ-AUD-01 uses |
| REQ-BTN-01 | 1 kohm | Claude's limit for a carbon pill on ENIG (Rationale), much below the 47k pull-up so the line reads clean |
| REQ-BTN-01 | 15 kohm | Claude's limit (Rationale, DF-230): the pad voltage with a 15 kohm pill against a 44.65k pull-up is 0.83 V, but the RP2350's VIL maximum at IOVDD 3.3 V is 0.8 V (rp2350.txt:86760 "IOVDD=3.3 V -0.3 0.8"), which allows a pill of onl |
| REQ-BTN-02 | 100 Mohm | 4.2 V over 100 Mohm leaks 42 nA, about 2100 times less than the 47k pull-up draws (Rationale) |
| REQ-BTN-02 | 24 h | Claude's choice of one day, the humidity itself is not stated in the Statement and no standard on disk sets it |
| REQ-BTN-03 | 0.20 mm | the comb's measured gap, above JLC's minimum spacing (the JLC page is not on disk), board/membrane/README.md:34 records 0.200 mm |
| REQ-BTN-03 | 1.60 mm | the smallest contact spot that bridges the nets anywhere on the comb (board/membrane/README.md:35 "1.60 mm across"), it leaves 0.95 mm of registration against the 3.5 mm pill |
| REQ-BTN-03 | 3.5 mm | the low end of the DS Lite pill's 3.5-4 mm range inferred from the measured comb (board/membrane/README.md:35 and :116), the pill itself is not measured on disk |
| REQ-BTN-03 | 4.8 mm | the mask opening of the comb pattern (board/membrane/README.md:37 "one 4.80 mm opening"), a design choice that uncovers all comb copper |
| REQ-BTN-04 | in: 0.5 mm margin | the margin outside the mask opening where no copper may lift the pill (Rationale) |
| REQ-BTN-04 | 0.2 mm | the width of a contact's own stubs, the same 0.20 mm line the comb is drawn with (board/membrane/README.md:34) |
| REQ-BTN-05 | 30% | Claude's limit relative to a genuine DS Lite (Rationale: a relative limit avoids an absolute number) |
| REQ-BTN-05 | 0.2 mm | Claude's limit relative to a genuine DS Lite |
| REQ-BTN-06 | in: 0.05 mm margin | what is left between the 0.95 mm the geometry allows and the Statement |
| REQ-BTN-07 | 100,000 presses | Claude's estimate for a toy used daily for a few years (Rationale), 100,000 over 3 years is about 91 presses a day |
| REQ-BTN-08 | 4 mm | the dome diameter of the part chosen (ref_design PLAN.md:26 "4 mm metal snap domes"), its drawing dome_600-415s-260.pdf has no text to quote |
| REQ-BTN-08 | 600-415S-260 | the HYP part Claude chose (PLAN.md row 4), a drawing with no text |
| REQ-BTN-08 | 0.1 mm | the placement error a dome must tolerate (Rationale: a design choice) |
| REQ-BTN-08 | 0.05 mm | the plunger clearance above the dome apex (case.scad assert) |
| REQ-BTN-10 | 2 ms | Claude's proposal (Source), five reads inside the shortest press |
| REQ-BTN-10 | 10 ms | a press of this length or more must not be missed, longer than a tactile contact's bounce and shorter than a deliberate tap |
| REQ-MEC-01 | 0.2 mm | the clearance margin on a drawn or measured height (Rationale: a design choice) |
| REQ-MEC-02 | 0.2 mm | the margin at worst-case tolerance after the cell's swelling (Rationale: a design choice) |
| REQ-MEC-05 | 20 open/close cycles | covers a few services and the bring-up opens per unit (Rationale) |
| REQ-MEC-06 | 1 mm | the cell movement a pocket and tape may allow after the drop (Rationale: a design choice) |
| REQ-MEC-09 | 4.0 mm | two screw diameters of the M2, the bottom of the 2 to 2.5 x d rule for self-tapping screws in printed plastic (Claude's analysis O-52, no source on disk) |
| REQ-MEC-09 | 0.5 mm | the gap between the tip and the pilot's end (Rationale: a design choice) |
| REQ-MEC-09 | 3.0 mm | 1.5 x d for the end pilots, which hold 3.30 mm at most (Rationale) |
| REQ-MEC-09 | 0.1 mm | the end pilots stop 0.6 mm under the face and hold 3.30 mm at most, so the tip stops 0.1 mm short (Rationale) |
| REQ-MEC-10 | 100 presses | Claude's figure for two players pressing the shared screen (Rationale: a design choice, O-59) |
| REQ-MEC-10 | 20 N | a firm finger press on the glass centre, IEC 62368-1:2010 (ed. 1, superseded) Annex T.2 and T.3 use 10 N and 30 N for fittings and enclosures, not for a glass press |
| REQ-MEC-10 | 24 h | Claude's soak for the tape's bond at temperature |
| REQ-MEC-10 | 50 C | above the 40 C ambient of REQ-THM-04 |
| REQ-MEC-11 | 1 mm | the height clear at each connector mouth (Rationale: a design choice, F-28) |
| REQ-MEC-12 | the tub's 6.0 mm boss | the boss diameter the tub is drawn with (case.scad:388 tub_boss_d 6.0), a design value |
| REQ-MEC-12 | 1 mm | the margin outside the boss (Rationale: a design choice, O-43) |
| REQ-MEC-12 | 6.0 mm or more | the larger part is longer so its pad sits further from its centre (Rationale) |
| REQ-MEC-12 | 0603 | the EIA size class at which the larger limit starts (Rationale) |
| REQ-THM-01 | 43 C | the target, Claude's reading that a handheld held for hours should meet a lower limit than the 48 C of the standard's edition 1 text (Rationale: 43 C beyond 8 h, to confirm, a later edition's figure that is not on disk (unread)) |
| REQ-THM-01 | 100% | the worst load of the display (Rationale) |
| REQ-THM-02 | 5 C | the difference between the thermistor and the cell's hotter point (Rationale: a design choice) |
| REQ-THM-03 | 35 C | 10 C of rise on the 25 C ambient and 15 C under the 50 C top of the BQ24073's 103AT-2 window (bq24074.txt:1968 "0°C to 50°C") |
| REQ-THM-04 | 3.0 V | the BQ24073's pre-charge to fast-charge threshold is 3 V typical (bq24074.txt:776 "2.9 3 3.1 V"), a cell just above it takes the full charge current at the lowest voltage which is the most the charger dissipates |
| REQ-THM-04 | 2.0 W | Claude's proposal until VER-144 gives the real sum (Rationale) |
| REQ-THM-04 | 40 C | a hot pocket or a car on a sunny day, B.1.7 of IEC 62368-1:2010 lets the maker specify the maximum ambient |

## 3. The `count:` numbers

| Row | Value | Reason |
|---|---|---|
| REQ-DSP-02 | 10 | ten frames, no standard sets it, fewer cannot show an intermittent corrupt pixel, more costs bench time |
| REQ-DSP-02 | 0 corrupted pixels | none allowed, zero by definition and not a sample |
| REQ-BTN-03 | 4-ring | the comb pattern has four rings (board/membrane/README.md:283 "4 rings"), a design choice and not a sample |
| REQ-MEC-01 | 0 parts | no part on a guessed height, zero by definition and not a sample |
| REQ-MEC-03 | 8 directions | the D-pad has four arms and four diagonals, eight positions to rock through, a design fact and not a sample |
| REQ-MEC-03 | 0 contact | none allowed, zero by definition and not a sample |
| REQ-MEC-04 | 1 orientation | keyed to a single way, unique by definition |
| REQ-MEC-04 | 1 hole | each cap fits one hole, unique by definition |
| REQ-MEC-04 | 1 rotation | each cap fits one rotation, unique by definition |
| REQ-MEC-05 | 0 cracked keys or grip ribs | none allowed, zero by definition and not a sample |
| REQ-MEC-05 | 0 stripped bosses | none allowed, zero by definition and not a sample |
| REQ-MEC-05 | 0 torn cell leads | none allowed, zero by definition and not a sample |
| REQ-MEC-07 | 5 different USB-C cables | a sample of the overmolds sold (Rationale), no standard sets it, fewer cannot show a spread of overmolds, more costs cables bought |
| REQ-MEC-08 | 0 through-openings | none besides the listed ones, zero by definition and not a sample |
| REQ-THM-03 | 0 temperature pauses | none allowed, zero by definition and not a sample |

## 4. Wrong cites found

Every `<file>.txt:<line>` cite and page cite in the 41 rows was read against the datasheet's `pdftotext -layout` text, and the IEC cite against edition 1's text.

- REQ-DSP-01, Rationale: `er-tft035ips-6.txt:718` for the 8-bit 8080 bus is "LED Life Time". Source: `er-tft035ips-6.txt:709-722` is the backlight table (section 4.4), not the interface. The IM strap table is at `er-tft035ips-6.txt:669-682` and the mode IM2-0 = 011 at `:678` ("0 1 1 8080 MCU 8-bit Bus Interface"). The three numbers are tagged to :678.
- REQ-THM-01, Source: "IEC 62368-1 cl. 9.3, Table 38". In edition 1 the touch temperature limits are clause 9.2.5, Table 42 (clause 9.3 is protection against thermal energy sources; Table 38 is the sizes of protective earthing conductors). The 48 C is tagged to Table 42 ("held in normal use (>1 min)": 48 C for metal, glass, plastic and wood alike).
- REQ-THM-01, Rationale: "TS1 limit for plastic held 10 min to 8 h is 48 C (43 C beyond 8 h)". Edition 1 has no 10 min to 8 h bands and no 43 C figure: its bands are >1 min, 10 s to 1 min, 1 s to 10 s and <1 s. The 43 C is tagged `judgment` (a later edition's figure, not on disk, `unread`).
- REQ-DSP-07, Rationale: "its Vf sits near its 3.2 V typical (rated at 20 mA)". The panel datasheet gives Vf 3.0 V typical and 3.2 V maximum, at If = 120 mA (`er-tft035ips-6.txt:713`), not at 20 mA.
- REQ-DSP-07, Statement: "the newer datasheet's maximum" (3.4 V) names a datasheet that is not on disk. The 3.4 V is tagged `judgment` and says `unread`.

Cites that hold: `ili9488.txt:16159` (twc 40 ns), `er-tft035ips-6.txt:713-715` and `:714-715` (backlight Vf and current), Hirose FH12 p.2 (`hirose_fh12.txt:51`, t = 0.3 +-0.05) and p.11 (`:705`, `:744`, B +-0.07 and 25.5 for 50 contacts), `fm8002a.txt:110` (TOPR -40 to 85 C), `bq24074.txt:824-827` (VHOT 300 mV, VCOLD 2100 mV), `bq24074.txt:1968` (103AT-2 window 0 to 50 C).

## 5. Numbers where a source disagrees with the Statement

None of these were changed (a Statement is Alex's to edit); each is said in the number's own entry.

| Row | Statement | Source says |
|---|---|---|
| REQ-BTN-01 | pill up to 15 kohm still reads pressed | RP2350 VIL maximum at IOVDD 3.3 V is 0.8 V (`rp2350.txt:86760`). A 15 kohm pill against a 44.65k pull-up puts 0.83 V on the pad, above 0.8 V. The pill limit the datasheet guarantees is 0.8 x 44.65 / (3.3 - 0.8) = 14.3 kohm. The Rationale defers the VIL reading to VER-233 and VER-232. |
| REQ-AUD-05 | full level within 50 ms | FM8002A wake time TD is 100 ms typical (`fm8002a.txt:133`), as the Rationale itself says. The datasheet gives IOFF = 4.2 uA typical and no maximum, so the 10 uA has no datasheet limit. |
| REQ-DSP-06 | VSYS up to 4.4 V | BQ24073 VO(REG) is 4.3 / 4.4 / 4.5 V (`bq24074.txt:729`): 4.4 V is the typical, 4.5 V the maximum. |
| REQ-DSP-04 | FPC 25.5 +-0.07 mm wide | The connector asks for B +-0.07 (`hirose_fh12.txt:705`), but the panel's own drawing gives 25.5 +-0.15 (`er-tft035ips-6.txt:280`), twice as loose, so a panel inside its drawing can fail this Statement. The thickness holds: 0.3 +-0.03 is inside 0.30 +-0.05. |

## 6. Checks

`python3 hardware/vv/tools/check_trace.py`: the Numbers check reports no fault in the 41 rows and no untagged number left in the five sections. It prints one structure error that was there before this batch and is not in these files: `README.md: design baseline differs from HEAD at hardware/ref_design/tools/build.py, case.scad, export.py (DEF-47)`. `tools/seed_faults.py` reports 153 seeded faults, 109 caught, RESULT: FAIL, the same counts as on main (c34e653) without this batch: the baseline error above stops the runs of the faults that need a clean check_trace, so it is not caused by the Numbers field. Both need the baseline updated (DEF-47) by whoever owns README.md.

Last updated: 2026-10-30 13:43 EDT
