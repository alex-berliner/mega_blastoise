# Mega Blastoise — Electronics Design

Working document for physical board wiring, power, GPIO assignment, and physical layout. Scope is a convention-portable demo: 3v3 battles, preset teams, no removable physical pieces.

---

## Component List

| Part | Qty | Purpose | Interface |
|------|-----|---------|-----------|
| Raspberry Pi Pico (RP2040) | 1 | Main MCU | — |
| WS2812B NeoPixel strip | 24 LEDs | HP bars, party status, status effect, animations | PIO (one wire) |
| 128×64 monochrome OLED (SSD1306) | 2 | Per-player display: sprites, moves, tooltips | I²C |
| 12 mm tactile button | 14 | 4 move + 3 party per player | Button matrix |
| Piezo buzzer | 1 | Audio cues | PWM |
| LiPo battery (2000 mAh) | 1 | Power | — |
| TP4056 charging module | 1 | USB-C charging | — |
| MT3608 boost converter | 1 | LiPo → 5 V | — |
| Slide switch (SPDT) | 1 | Power on/off | — |

Approximate parts cost: **~$40** (excluding enclosure and connectors).

---

## GPIO Map

The RP2040 Pico has 27 usable GPIOs. With NFC and 7-segment displays cut, the new map is comfortable — 7 spare pins after everything is wired.

```
GP0   spare
GP1   spare
GP2   spare
GP3   spare
GP4   spare
GP5   spare
GP6   Button matrix ROW 0  (P1 moves)
GP7   Button matrix ROW 1  (P1 party)
GP8   Button matrix ROW 2  (P2 moves)
GP9   Button matrix ROW 3  (P2 party)
GP10  Button matrix COL 0
GP11  Button matrix COL 1
GP12  Button matrix COL 2
GP13  Button matrix COL 3
GP14  spare
GP15  spare
GP16  I2C0 SDA  →  P1 OLED (SSD1306, addr 0x3C)
GP17  I2C0 SCL  →  P1 OLED
GP18  I2C1 SDA  →  P2 OLED (SSD1306, addr 0x3C)
GP19  I2C1 SCL  →  P2 OLED
GP20  NeoPixels (PIO one-wire)
GP21  Buzzer (PWM)
GP22  spare
GP23  (SMPS mode — avoid)
GP24  (VBUS sense — avoid)
GP25  (onboard LED — usable for boot diagnostics)
GP26  spare / ADC0
GP27  spare / ADC1
GP28  spare / ADC2
GP29  spare / ADC3 / VSYS sense
```

The two OLEDs are on separate I²C buses (each at default address 0x3C) to avoid having to swap one to 0x3D and to let either display update without blocking the other.

---

## Button Matrix

4 rows × 4 columns = 16 positions; 14 used.

| Row | Player | Function | Active cols |
|-----|--------|----------|-------------|
| 0 | P1 | Move buttons 1–4 | cols 0–3 |
| 1 | P1 | Party slots 1–3 | cols 0–2 |
| 2 | P2 | Move buttons 1–4 | cols 0–3 |
| 3 | P2 | Party slots 1–3 | cols 0–2 |

Cols 3 in party rows (1 and 3) are unused matrix positions.

### Wiring

- **Row pins (GP6–9):** outputs, one LOW at a time during scan; others HIGH or high-Z.
- **Col pins (GP10–13):** inputs with internal pull-ups.
- Detection: drive row LOW → read columns → LOW column = button pressed at that (row, col).
- Scan all 4 rows in sequence at ~1 ms — fast enough that no press is missed.
- **Hold-to-inspect:** firmware distinguishes short press (~50 ms) from hold (~200 ms+) on move buttons during the move-pick state.

### Physical placement

Each player has a pair of button rows on their side of the board:

```
[ P1 party: 3 buttons ]
[ P1 moves: 4 buttons ]
       [board center]
[ P2 moves: 4 buttons ]
[ P2 party: 3 buttons ]
```

Button labels are silkscreened on the board face — "1 / 2 / 3 / 4" for moves, "1 / 2 / 3" for party slots.

---

## LED Strip

24 WS2812B LEDs total, single chain driven by GP20 via PIO. Daisy-chained continuously across the board — physical layout can split into two visual segments while remaining one logical strip.

```
[ P1: LEDs 0–11 ]    ←——————→    [ P2: LEDs 12–23 ]
```

### Per-player assignment (12 LEDs each)

| LEDs | Count | Function |
|------|-------|----------|
| 0–7 | 8 | HP bar — active Pokémon's health |
| 8–10 | 3 | Party status — one LED per team slot |
| 11 | 1 | Status effect indicator |

**HP bar:** 8 LEDs = 12.5% per LED. Color shifts green → yellow → red as HP drops. Drains LED-by-LED on damage with a buzzer hit.

**Party slots (3 each):** dim green = alive on bench; bright white = currently active; off = fainted. Pulses white during forced-switch prompts.

**Status effect:** yellow = paralyzed, orange-red = burned, cyan = frozen, purple = poisoned, slow green pulse = asleep. Off = no status. When a status lands, the HP bar briefly tints the same color as the status LED to teach players what the status does.

---

## OLED Displays (per player)

Two 128×64 monochrome SSD1306 OLEDs, one per player station. I²C interface, 4 wires each (VCC, GND, SDA, SCL). Drive at 400 kHz fast mode — fast enough for sprite swaps and tooltip updates.

### What each OLED shows

| State | Display content |
|-------|-----------------|
| Attract | Alternates "PLAY" / "ME", chased by the buzzer chime |
| Mystery draft | Each Pokémon revealed one at a time: large sprite + name |
| Battle ready | Active Pokémon portrait centered |
| Move pick | Active Pokémon sprite (small) + numbered move list (4 lines) |
| Hold-to-inspect | Tooltip: move name, type, power, accuracy, secondary effect |
| Status applied | Sprite + small "PAR" / "BRN" / "PSN" / etc. tag in corner |
| Faint | Grayed sprite + "FAINTED" |
| Pick replacement | Two small sprites + HP for remaining Pokémon |
| Tactical timeout (opponent) | Your remaining team's sprites + HP |
| Win / lose | Stat recap (damage, crits, turns) → "WINNER!" / "GG!" |

Sprite assets are pre-rendered 1-bit bitmaps baked into flash. Roster of ~20 Pokémon × 1 sprite each = ~20 KB of sprite data, well within the 2 MB flash budget.

### Why per-player, not center-shared

A single shared display means one player reads it upside-down and team-private info (your moves, your tooltips) is exposed to your opponent. Two cheap OLEDs solve both problems and add convention drama: each player has their own little screen telling them what to do.

---

## Buzzer

Single piezo buzzer driven by PWM on GP21. Polyphonic-feeling melodies achieved via fast frequency switching (no DAC needed). Used for:

- Attract-mode chime (every 30 s when idle)
- Pokémon-reveal beep during draft
- Ready-up chord at battle start
- Move hit sounds
- Super-effective "ding-ding" double-beep
- Critical hit sting
- Status-applied zap
- Faint descending three-note tone
- Win victory jingle (4 notes)

Mounted on the underside or behind a small grille. Volume: tactile-button-loud — clear in a quiet room, audible but not piercing in a convention hall.

---

## Power

### Battery design (untethered)

LiPo + boost converter so the board runs without a tether for a full convention day:

```
LiPo 2000 mAh
  ├─ TP4056 charge controller   (USB-C input on board edge for charging)
  └─ MT3608 boost → 5 V         (regulated up from LiPo voltage)
       └─ slide switch (on/off)
            ├─ Pico VSYS
            └─ NeoPixel VCC

Pico 3V3 out → OLED VCC (×2), button pull-ups (internal)

GND: all components share one common ground rail
```

The boost is non-negotiable — WS2812B minimum VDD is 3.5 V, and a discharging LiPo drops to 3.0 V at empty. Without the boost, the LEDs misbehave well before the battery is actually flat.

### Current budget (approximate)

| Consumer | Peak current |
|----------|-------------|
| Pico (RP2040 + 3V3 reg) | ~100 mA |
| NeoPixels (24 × ~20 mA moderate brightness) | ~480 mA |
| OLEDs (×2) | ~40 mA |
| Buzzer (PWM, brief) | ~30 mA |
| **Total peak** | **~650 mA @ 5 V** |

At average gameplay brightness, draw is closer to ~300 mA. From a 2000 mAh LiPo at ~85% boost efficiency: **~5 hours of active gameplay**, more in attract mode.

### Charging

USB-C port on the board edge wired to the TP4056. Plug in overnight, ready for the next day. The slide switch isolates the load while charging — boost circuits draw quiescent current even when the Pico is "off."

---

## Physical Layout & Dimensions

Portrait orientation, players on opposite short edges facing each other.

```
┌─────────────────────────────────────────┐  ← top edge
│  [ P2 OLED (128×64) ]                   │
│  [ P2 party: 3 buttons ]                │
│  [ P2 moves:  4 buttons ]               │
│                                          │
│  ┌──── P2 LEDs: 8 HP + 3 party + status ─┐│
│  │ █ █ █ █ █ █ █ █  ●  ●  ●   ● status  ││
│  └────────────────────────────────────────┘│
│                                            │
│  ┌──── P1 LEDs: 8 HP + 3 party + status ─┐│
│  │ █ █ █ █ █ █ █ █  ●  ●  ●   ● status  ││
│  └────────────────────────────────────────┘│
│                                            │
│  [ P1 moves:  4 buttons ]                  │
│  [ P1 party: 3 buttons ]                   │
│  [ P1 OLED (128×64) ]                      │
└────────────────────────────────────────────┘  ← bottom edge

(Buzzer mounted under the board, grille on one side. Pico + battery + power circuit
 mounted on underside in a recessed cavity.)
```

### Dimensions

LED strip density: **144 LEDs/m** keeps each player's 12-LED segment to ~85 mm — a compact strip that fits comfortably across the board.

| Dimension | Calculation | Value |
|-----------|-------------|-------|
| Width | LED strip (~85 mm) + side margins | **~150 mm** |
| Height (top to bottom) | OLED (15 mm) + buttons (~40 mm) + LED row (12 mm) + center gap (10 mm) + LED row (12 mm) + buttons (~40 mm) + OLED (15 mm) + margins (20 mm) | **~165 mm** |

**Final board size: roughly 150 mm × 165 mm.** Smaller than a paperback book — easily portable, fits in a tote bag with room to spare. Light enough to hand-carry around the convention.

Pico, battery, charge module, boost, and switch all mount on the underside in a 5–8 mm recess. Total board thickness: ~12 mm including buttons protruding from the top face.

---

## Open Questions

1. **OLED orientation** — landscape (128 wide, 64 tall) or portrait? Landscape gives more room for the move list (4 lines fit naturally). Confirm sprite proportions.
2. **Enclosure** — single piece of laser-cut acrylic (cheapest, fastest), 3D-printed shell (more rugged), or hand-finished wood (prettiest)? Needs to survive a day of convention handling.
3. **Charging UX** — solid LED while charging, blinking when full? Use the Pico's onboard LED (GP25) for this.
4. **Attract-mode buzzer** — how often is too often? 30 s feels right but might annoy your booth neighbors. Add a volume potentiometer? Or rely on the slide switch.
5. **NeoPixel data resistor** — 330 Ω series resistor on GP20 line, recommended.
6. **Decoupling caps** — 470 µF across NeoPixel 5 V at strip entry, 100 nF on each OLED VCC. Worth designing in from the start.
