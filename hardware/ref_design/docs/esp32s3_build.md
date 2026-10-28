# ESP32-S3 board: what the build changed (2026-10-28)

Branch ref-design-esp32s3. The RP2354B core, its crystal, inductor and 1V1 parts are gone; the board carries an
ESP32-S3-WROOM-1U-N16R8 (LCSC C3013946) with its antenna off the board. Everything below is generated or checked by the scripts in
`board/tools`; the pin map is `docs/pinmap.md`, its differences from the proposal `docs/esp32s3_pinmap_diff.md`, the V&V carry-over rows
`carryover_status.md` (in the session's scratchpad).

## Board
- Routed 2-layer board: DRC 0 errors, 0 unconnected, 0 parity issues (check_pcb PASS), check_fit PASS; 170 footprints, 2262 tracks (6152 mm), 646 vias. Tracks and vias 0.518 mm from the routed edge or more.
- C10 (22 uF, 0603 C59461, 6.3 V X5R) and C11 sit in the strip beside the module: +3V3 pads 4.28 and 3.40 mm from U4 pin 2.
- Module at KiCad (73.9, 117.1) turned 90 degrees: its U.FL side to the left wall. Body x 64.05 .. 83.25, y 108.1 .. 126.1, 3.2 mm tall.
  `check_fit.BODY_LOCAL` carries its body (KiCad's library has no WROOM-1U 3D model).
- Antenna: the flex antenna (Kinghelm C4943394 or an equal within the module's FCC grant) lies on the left wall above the module, x 64.5 .. 71,
  y 62.1 .. 107.9, on a 40 mm U.FL cable. The board has a copper-free rule area on both layers, x board edge .. 71.4, y 62.6 .. 107.0 (not a
  Kinghelm number: the maker's datasheet was not retrievable; the guidelines' Fig 26 reading is in the antenna report). H2's boss touches the mock
  plate's end by 0.4 mm. The cable and the antenna are fitted by hand (JLC does not do cables).
- microSD: Hirose DM3AT-SF-PEJM5 (C114218, 1.68 mm), mouth to the left wall at y 128 .. 142 (case opening sd_y), back at x 78.02 under the panel's
  FPC zone edge (78.05). Footprint `battler:microSD_HC_Hirose_DM3AT-SF-PEJM5`: KiCad's with the two front shell tabs shortened to 7.75 (their
  land is not in Hirose's drawing: unverified). The case's left wall is 1.5 mm further out (board 73.9 x 151.48, case 78.9 x 156.48).
- Left wall growth: 1.5 not 1.3 because the shell tabs' copper needs 0.31 to the board edge with the slot's back held at 78.05.
- U2 uses TI's RGT0016C land pattern (0.26 pad to exposed pad).
- Cell hole 38.2 x 55.2 (the pocket 52.4 x 36.4).
- Power: a dedicated button SW24 (ROCPU TPS850 J14K, C2759975, top actuated, 1.4 mm) at KiCad (103.25, 162.75) under seat 1's deck, between U8 and the
  Start/Select frame (H6's boss takes the centre line). The cap is printed (case.scad pwr_cap; pin in a 3.0 hole, flange in an underside recess, top 0.5
  under the face inside a 6 mm pocket, so it cannot be pressed lying face down). Both Starts are plain dome buttons to GND now; Q7, D8, R62 are gone;
  P1 Start is bit 22 of the chain, bit 16 is the power button (docs/esp32s3_pinmap_diff.md).
- BOOT and RESET buttons and their pinholes are not on this board (the left bezel is the module, antenna and slot; the right has no room);
  J7 (3V3, EN, BOOT, GND pads on the back) stays. A question for Alex (scratchpad esp32_build/report.md).
- H4 moved from (67.5, 140) to (67, 151), a deck screw with the end screws' pilot; the cell hole is 38.2 x 55.2 (extra 0.4 on the left and at seat 1's end).
- The power key's face pocket is a 51 degree cone, 5.0 wide at the face (the top prints face down: a straight pocket was a 6.9 mm bridge).

## Tools changed
- `tools/place_new.py`: nearest legal spot for the new parts; `placement_new.json` is its result, applied by placement.py.
- `tools/fp_esp32.py`, `fp_sd.py`, `fp_u2.py`, `fp_pogo.py`: footprints written into lib/battler.pretty.
- build_pcb.py: no core copy (CORE is empty), the antenna's rule area, four GND vias in the module's ground pad, the left edge out.
- route.py: no RP2354B 3V3 pour or stubs, the comb keep-out 2.9 mm across flats, the grown outline.
- check_pcb.py fails on every non-silk violation; silk.py has the JLC marker.
- case: `grow_l`, `sd_cut`, H4 deck pin (case.scad); check_wall skips the opening.

## Not verified
JLC Basic/Extended and stock of the new parts; Hirose's land for the front tabs; the antenna's RF (no simulation, no keep-out number from its maker);
the FCC grant's portable-use rules for an external antenna; BOM, CPL and polarity against JLC's preview; bring-up and assembly text (bringup.md,
assembly.md, bom.md still describe the RP2354B board).
