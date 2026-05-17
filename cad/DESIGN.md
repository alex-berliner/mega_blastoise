# Mega Blastoise — Enclosure CAD Subproject

3D-print-ready case design for the physical Mega Blastoise board.

See also: [DESIGN.md](../DESIGN.md) (gameplay), [ELECTRONICS.md](../ELECTRONICS.md) (wiring/BOM).

---

## Goal of this subproject

Produce a **3D-printable, two-part clamshell enclosure** that houses every
component in the Mega Blastoise electronics design and is **as small as a phone
or smaller**, while staying playable by two strangers sitting across it at a
convention.

Concretely, "done" means:

1. A parametric **OpenSCAD** model in this directory (`board.scad`) that exports
   two STLs — `bottom-tub.stl` and `top-faceplate.stl` — that slice and print
   on a consumer FDM printer (≤180 mm bed) with **no supports**.
2. Every component from [ELECTRONICS.md](../ELECTRONICS.md) has a defined home:
   a pocket, standoff, cutout, or window — with print-tolerance clearances.
3. Footprint ≤ a large phone (target **≤ 70 × 160 mm**), thickness ≤ ~16 mm.
4. Serviceable: opens with screws, components are removable, USB-C charge port
   reachable without opening.
5. The chosen toolchain needs **no CAD skill from the user** — the model is
   text I edit; the user only installs OpenSCAD and presses F6 to export.

Non-goals (v1): final aesthetic fit-and-finish, snap-fit lids, gasketing,
in-mold lettering beyond simple embossed button labels, designing the PCB
itself (this is an enclosure, components are hand-wired / module-based).

---

## Design decisions (confirmed)

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Modeling tool | **OpenSCAD** (code-based, parametric) | Lives in the repo, version-controlled, editable by Claude, zero CAD learning curve. |
| Part substitutions | **Allowed**, documented as deviations | Stock Pico + 2000 mAh LiPo are too large for "phone-sized". |
| Construction | **Two-part clamshell** (bottom tub + top faceplate, screwed) | Prints flat, no supports, serviceable, convention-robust. |

---

## Proposed BOM deviations from ELECTRONICS.md

These shrink the board substantially. They are **proposals** — not yet folded
into ELECTRONICS.md. Once confirmed, they should be added there as resolved
open questions.

| ELECTRONICS.md part | Footprint | Proposed substitute | Footprint | Why |
|---------------------|-----------|---------------------|-----------|-----|
| Raspberry Pi Pico | 51 × 21 mm | **RP2040-Zero** (Waveshare) | 23.5 × 18 mm | Same RP2040, same GPIO needs, ~28 mm shorter. USB-C onboard for flashing. |
| LiPo 2000 mAh | ~60 × 50 × 7 mm | **~1200 mAh slim cell** (e.g. 553450) | ~50 × 34 × 6 mm | Halves the largest footprint. Trade: ~2.5–3 h gameplay vs ~5 h (still a full demo session, recharge between). |
| WS2812B strip @ 144 LED/m | 12 LEDs ≈ 83 mm | **WS2812B-2020** at ~4 mm pitch | 12 LEDs ≈ 48 mm | The HP-bar width was setting board width. Denser/smaller LEDs shrink it ~35 mm. |
| TP4056 / MT3608 / slide switch | discrete | keep discrete, **bare mini modules** | TP4056 ~26×17, MT3608 ~17×17 | No functional change; just spec the smallest module variants. |

GPIO map in ELECTRONICS.md is unaffected — RP2040-Zero exposes all the GP pins
the firmware uses (button matrix, 2× I²C, PIO LED, PWM buzzer).

---

## What the board looks like

Portrait slab. The two players sit on the **opposite short edges**, board flat
on the table between them. Each player's controls are mirrored 180° so the
layout reads right-side-up from their own seat. A center divider strip splits
the two halves.

```
            ┌─────────────────────────────┐  ← P2's edge (top)
            │   (1) [  P2 OLED  ] (2)      │   move btns at OLED corners
            │   (3) [  0.96"    ] (4)      │
            │      [ S1 ][ S2 ][ S3 ]      │   P2 switch row (under OLED)
            │        ◉  ● ● ●              │   P2: HP LED + 3 party
            │ ─ ─ ─ ─  ▓ buzzer ▓ ─ ─ ─    │   centre divider + shared grille
            │        ◉  ● ● ●              │   P1: HP LED + 3 party
            │      [ S1 ][ S2 ][ S3 ]      │   P1 switch row
            │   (1) [  P1 OLED  ] (2)      │
            │   (3) [  0.96"    ] (4)      │   move btns at OLED corners
            └─────────────────────────────┘  ← P1's edge (bottom)
                  ↑ USB-C charge port on one long side edge,
                    slide power switch adjacent (web-mirrored layout)
```

### Top faceplate (the face players see and touch)

- **2× OLED windows** — rectangular cut-throughs sized to the SSD1306 glass
  active area, with a recessed ledge inside so each module sits flush from
  behind and the bezel hides the PCB edge.
- **14 button holes** — round clearance holes for 12 mm tactile-button
  actuators (or printed keycaps on 6 mm switches — chosen via parameter).
  Embossed labels beside each: `1 2 3 4` for moves, `1 2 3` for party.
- **2× LED diffuser slots** — long thin windows over each player's LED bar,
  backed by a thin print-in-place light diffuser wall (or a slot for a strip
  of frosted acrylic / translucent print).
- **Center divider** — slightly raised rib so the two play areas read as
  separate; doubles as a finger-stop and hides the LED strip seam.
- **Buzzer grille** — a small array of holes on one edge over the piezo.
- **Screw bosses** — countersunk holes at the four corners + mid-sides.

### Bottom tub (holds the electronics, hidden underneath)

- **MCU pocket** — recessed seat + nub locators for the RP2040-Zero, oriented
  so its USB-C aligns with the edge cutout.
- **Battery bay** — a walled pocket sized to the slim LiPo with a retention
  lip / foam-tape floor; routed away from button posts.
- **Power-circuit shelf** — flat areas with low standoffs / tape pads for the
  TP4056 and MT3608 mini modules.
- **Edge cutouts** — USB-C pass-through (TP4056 charge port) on one long
  edge; SPDT slide-switch slot adjacent so it's flick-able from outside.
- **Cable channels** — shallow troughs guiding the LED strip across the
  center, I²C runs to each OLED, and the button-matrix harness.
- **Standoffs / screw posts** — mate with the faceplate's countersinks;
  heat-set insert bosses (M2 or M2.5) or self-tapping post option (parameter).

### Estimated dimensions (parametric — these are the starting defaults)

| Axis | Driver | Default |
|------|--------|---------|
| Width (short, across player) | OLED 27.3 + flanking move btns + side margins | **71.3 mm** |
| Length (long, player-to-player) | 2× (OLED + switch row + small LED row + gaps) + centre | **132.6 mm** |
| Thickness | LiPo 6 mm + wiring + faceplate + button protrusion | **13.5 mm** |

Result: **71.3 × 132.6 × 13.5 mm** — smaller than a phone (shortened once
the wide 8-LED HP bar collapsed to a single RGB HP LED). See
[PARTS.md](./PARTS.md) for the derivation from pinned part dimensions.

Every number above will be a named variable at the top of `board.scad`
(`oled_w`, `btn_pitch`, `wall_t`, `led_pitch`, `clearance`, …) so the whole
case re-derives when one part changes.

---

## Ergonomic note / tension to flag

"As small as possible" fights "two adults comfortably press 7 buttons each
facing each other." At 66 mm wide a 4-button move row at ~12 mm pitch fits,
but it is tight. Mitigations baked into the plan: tactile buttons over the
narrowest practical pitch, a raised center divider as a hand-stop, and the
button-pitch / switch-size left as parameters so we can loosen it after a
test print without redesigning. Calling this out now so it's a conscious
trade, not a surprise at print time.

---

## Open questions before modeling

1. ~~Exact module dimensions~~ — **resolved**: canonical parts + dims pinned
   in [PARTS.md](./PARTS.md). Four "high-verify" modules still need
   caliper-check once bought; MT3608 thickness conflict flagged there.
2. **Button choice** — 12 mm tactile (robust, bigger) vs 6 mm + printed
   keycaps (smaller, more print work). Default: 12 mm.
2b. ~~Button mounting~~ — **resolved: stripboard** on tub standoffs.
   Copper strips double as matrix row lines; jumpers for columns.
   `btn_pitch` now grid-locked to a 2.54 mm multiple — default 15.24 mm
   (6 holes) for pressability. Raises board width to ~72 mm. See PARTS.md.
3. ~~LED diffusion~~ — **resolved: option 1, integral & simple.** Faceplate
   has a thin recessed floor (~1 mm, `led_diffuser_t`) spanning each LED
   slot — part of the faceplate, no rebate, no separate insert, no mode
   parameter. Switching to acrylic/open later = faceplate reprint (accepted;
   it's cheap and test-printed first). Print faceplate in translucent/natural
   filament for even glow (material choice, not a CAD change).
4. **Fastener** — heat-set inserts (clean, reusable) vs self-tapping into
   printed posts (zero extra parts). Default: heat-set M2.5.
5. **Printer bed size** — 158 mm length must fit the user's printer; confirm
   bed ≥ ~165 mm, else split parts or shrink further.

---

## Status

- ✅ `cad/board.scad` written — parametric `bottom_tub()` + `top_faceplate()`
  + `assembly()`, with `echo`/`assert` self-checks and `cut=`/`check=` modes.
- ✅ Rev 2 — **web-mirrored layout**: 4 move buttons at OLED corners, 3-switch
  row under the OLED, single RGB HP LED + 3 party LEDs, centred shared buzzer.
- ✅ Validated via the METHODOLOGY render+assert loop: asserts pass, both
  parts single watertight solids (admesh: 1 part each), dims confirmed
  **71.3 × 132.6 × 13.5 mm**.
- ✅ STLs: `cad/bottom-tub.stl`, `cad/top-faceplate.stl`.
- ✅ Renders: `cad/renders/contact-sheet.png`, `cad/renders/assembly-hero.png`.
- ⚠️ **Cross-crate debt:** single-HP-LED diverges from ELECTRONICS.md /
  firmware / web (was 8-LED HP bar). Per the web-fidelity rule those must be
  updated to match (8→1, lose LED-by-LED drain). Not yet propagated.

## Next steps

1. **Propagate the single-HP-LED decision** to ELECTRONICS.md, firmware, web.
2. Caliper the four high-verify parts once bought; update PARTS.md → params.
3. Confirm printer bed (param `bed`, default 220×220) and the B-list defaults.
4. Optional polish: embossed move/switch labels, edge fillets.
5. Test-print the faceplate first (cheap; validates layout + OLED fit).
