# Pinned Parts — dimensions for the CAD model

Canonical parts + datasheet/nominal dimensions feeding `cad/board.scad`.
Resolves open question #1 in [DESIGN.md](./DESIGN.md).

**Clone reality:** these modules are made by many vendors with ±1–3 mm
variation. Strategy: model nominal dims + the listed clearance, leave each as
a named parameter, and **caliper-verify the parts you actually buy** before
the final print. The "Verify" column flags which ones bite hardest if wrong.

All mm. L = long axis of part, W = short, H = tallest point above its seat.

| # | Part | Canonical pick | L | W | H | Clearance to add | Verify | Notes |
|---|------|----------------|---|---|---|------------------|--------|-------|
| 1 | MCU | Waveshare **RP2040-Zero** | 23.5 | 18.0 | ~4.5 | +0.4 mm/side pocket | med | USB-C overhangs one short edge ~1.5 mm → align to edge cutout. Castellated + 2 through-hole rows. |
| 2 | Display ×2 | **0.96" SSD1306 I²C** OLED (4-pin) | 27.3 | 27.3 | ~4.1 | +0.5 mm/side; window = active +1 mm | **high** | Active glass 21.74 × 10.86 mm. Some clones 26×26 — measure. Glass-out, seat on internal ledge, header to inside. |
| 3 | Power module | **MH-CD42** (IP5306): USB-C charge + 5 V/2 A boost + protection, all-in-one | 34.0 | 22.0 | ~7.0 | +0.4 mm/side | **high** | Replaces separate TP4056+MT3608. ~7 mm lies flat in the 9 mm cavity → **no bump-out**. USB-C to edge cutout. Clones vary — caliper. |
| 5 | Battery | **LiPo 553450** ~1200 mAh | 50.0 | 34.0 | 5.5 | +1 mm L/W, +0.8 mm H (swell) | **high** | Single biggest footprint. Pocket ~51 × 35 × 6.3. Pad floor with foam tape. |
| 6 | Power switch | **SS-12D00** SPDT slide | 8.7 | 3.6 | 3.5 | slot + actuator slot | med | Tiny. Actuator protrudes through a side-edge slot, flick from outside. |
| 7 | Buzzer | 12 mm passive piezo | Ø12 | — | 9.0 | +0.5 mm radial | low | Round pocket; grille holes in faceplate above it. |
| 8 | Buttons ×14 | 12 mm 4-leg tactile (THT) | 12.0 | 12.0 | ~7.3 | actuator hole Ø4.0 | med | Soldered to stripboard (#12). `btn_pitch` grid-locked: default 15.24 mm (6 × 2.54). |
| 12 | Button substrate | Stripboard ~1.6 mm, 2.54 mm grid | cut to fit | cut to fit | 1.6 | standoff posts in tub | low | One piece per player's 7-button cluster. Strips = matrix rows; jumper columns. |
| 13 | LED diffuser | Integral faceplate skin (~1 mm) | per-window | — | 1.0 | part of faceplate, no separate part | low | Blind round windows, translucent front skin. Print faceplate in translucent filament. |
| 9 | LEDs ×8 | **5 mm through-hole WS2812B** (1 HP + 3 party per player) | Ø5 | — | ~8 | blind window pocket | low | **HP = single RGB LED**, colour = health. Stripboard-friendly (matches #12). ~$5/5-pack; prototype + final. ⚠ cross-crate change — see note. |
| 10 | Passives | 330 Ω R, 470 µF + 100 nF caps | — | — | ≤12 | lump in wiring budget | low | 470 µF electrolytic ~Ø8 × 12 mm — lay sideways in a cable channel. |
| 11 | Fasteners | M2.5 heat-set inserts + screws | — | — | — | boss OD 5, bore Ø3.6 | — | Inserts in tub posts; countersinks in faceplate. |

---

## ✅ Resolved: integrated power module (was MT3608 thickness conflict)

The original TP4056 + MT3608 split put a 14 mm-tall boost module in a 9 mm
cavity, forcing an ugly underside bump-out. **Resolved** by swapping both
for a single **MH-CD42 (IP5306)** all-in-one: USB-C charge + 5 V boost +
protection in one ~34 × 22 × **7 mm** board. 7 mm < the 9 mm cavity, so it
lies flat — **bump-out eliminated**, tub is now a clean simple clamshell.

The `boost_bumpout` parameter and blister code remain in `board.scad`
(default `false`, auto-disabled since the module now fits) as a fallback if
a taller module is ever substituted.

> ⚠ **BOM deviation, logged not propagated:** ELECTRONICS.md still specs
> TP4056 + MT3608 separately. Per the exploratory-mode rule, this is *not*
> back-propagated to ELECTRONICS.md / firmware until the design is finalized.
> Functionally equivalent (charge + 5 V boost + protection); the IP5306's
> auto-boost / double-tap-enable behaviour differs from a plain slide switch
> — revisit power-on UX when propagating.

---

## Recomputed board envelope (from pinned dims)

| Axis | Drivers | Result |
|------|---------|--------|
| Width | OLED 27.3 + 2× flanking move btn + 2×7 margin | **71.3 mm** |
| Length | 2 × (margin 6 + OLED 27.3 + switch row 12 + small LED row + gaps) + centre 12 | **132.6 mm** |
| Thickness | LiPo 6.3 + wiring + faceplate 2.5 + button travel ≈ **13.5 mm** |

Updated envelope: **71.3 × 132.6 × 13.5 mm** — smaller than a phone;
shortened after the wide LED bar collapsed to a single HP LED, and now a
clean flat clamshell (no bump-out) thanks to the integrated power module.

> ⚠ **Cross-crate change:** single-HP-LED + web-style button layout deviates
> from ELECTRONICS.md (was 24 WS2812B, 8-LED HP bar). Per the web-fidelity
> rule, ELECTRONICS.md / firmware / web must be updated to match (8→1 HP
> LED, lose LED-by-LED drain). Tracked as a project decision, not yet
> propagated to those crates.

---

## Parameter name map (→ `board.scad`)

Each part drives named variables so the case re-derives on any change:

```
rp2040    = [23.5, 18.0, 4.5];     oled       = [27.3, 27.3, 4.1];
oled_active = [21.74, 10.86];      pmod       = [34.0, 22.0, 7.0];
lipo      = [50.0, 34.0, 5.5];     // pmod = MH-CD42 (charge+boost+protect)
slide_sw  = [8.7, 3.6, 3.5];       buzzer_d   = 12.0;  buzzer_h = 9.0;
btn       = 12.0;  btn_pitch = 15.24;  move_gap = 3.0;  // switch row + flank
hp_led_d  = 5.0;   party_led_d = 3.0;  party_led_n = 3;  party_led_pitch = 6.0;
strip_grid = 2.54;                      // stripboard, btn_pitch = N*grid
led_diffuser_t = 1.0;  // translucent front skin over each blind LED window
clearance = 0.4;   wall_t = 2.0;   faceplate_t = 2.5;
boost_bumpout = true;              insert_boss_od = 5.0;
bed = [220, 220];   // user's printer bed — assert board fits
```

---

## Still needs the user

- **Confirm / order the exact SKUs** for the "high"-verify rows (OLED #2,
  power module #3, LiPo #5) and caliper the received parts → update table.
- **Printer bed size** — set `bed` so the fit assertion is real (132.6 mm
  length must clear the bed; default 220×220).
- **Power-on UX** — IP5306 auto-boosts / double-tap-enables; decide whether
  the SPDT slide switch stays (in series on 5 V out) when propagating.

Sources: [Waveshare RP2040-Zero](https://www.waveshare.com/rp2040-zero.htm),
[RP2040-Zero datasheet PDF](https://files.waveshare.com/upload/4/4c/RP2040_Zero.pdf),
[MH-CD42 / IP5306 (done.land)](https://done.land/components/power/powersupplies/battery/chargers/charge-discharge/ip5306/mh-cd42/),
[IP5306 module (done.land)](https://done.land/components/power/powersupplies/battery/chargers/charge-discharge/ip5306/),
[0.96" SSD1306 OLED (displaymodule)](https://www.displaymodule.com/products/0-96-inch-oled-graphic-display-128x64-with-i2c),
[SSD1306 OLED datasheet (Mouser)](https://www.mouser.com/datasheet/2/1398/Soldered_333099-3395096.pdf).
