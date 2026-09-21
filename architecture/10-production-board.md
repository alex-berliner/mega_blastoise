# Production Board

Target design for a board that can be batch produced and sold, replacing the
hand-wired `hardware/stripboard_v2/`. Same device as
[09-single-screen.md](./09-single-screen.md); this page only covers what changes
when the thing stops being a one-off.

Decisions here were made on 2026-09-17. Where a number was measured from a
source file it says where; where something is an estimate it says so.

---

## 1. What changes and what does not

**Changes:** every physical part. Buttons, MCU packaging, power chain, display
mounting, plus audio hardware that has never existed on any revision.

**Does not change:** the battle engine, the core/platform split, or the parity
rule. New hardware gets the same treatment as the old: all display logic in
core, all input semantics in core, platforms do raw IO only. Audio is new and
follows the same rule, which means the mixer lives in core and the platforms
only push samples, so the web build sounds like the device.

**The constraints of the stripboard are gone.** No 2.54 mm grid, no 24x56 hole
outline, no 9 mm cap limit, no ~30 mm cluster spacing. Re-decide ergonomics from
scratch rather than porting that layout.

---

## 2. Hardware target

| Item | Decision | Notes |
|---|---|---|
| Buttons | **DS Lite silicone membranes** on bare PCB pads | 15.0 mm face-button diamond, half the SNES footprint. One D-pad piece + one ABXY piece per seat covers a seat's 8 inputs exactly. See §3 |
| MCU | **Bare RP2350B** (QFN-80, 48 GPIO) | Not the A: §4 shows the pin budget needs more than 30. Fab house solders it; 0.4 mm pitch is not hand-solderable |
| Core circuit | Copy Raspberry Pi's RP2350B Minimal KiCad design | Their regulator layout is mandatory, including the polarity-marked Abracon inductor. ABM8-272-T3 crystal, W25Q128JVS 16 MB flash |
| Charger | Power-path IC (BQ25185 class), **not** TP4056 | TP4056 has no power path: a running device and the cell share charge current and termination gets unreliable |
| Cell | Protected LiPo with an NTC, on JST-PH | Carriers require UN38.3 test docs from the supplier |
| 3V3 rail | Buck-boost | Uses the cell down to ~3.0 V; the stripboard's Schottky drop is gone |
| Power switch | Soft button, hold to toggle | Also delivers the hold-to-reset behaviour the reset-button TODO wanted |
| Display | Bare **IPS** panel on FPC, behind a cover window | IPS still mandatory: two seats view one flat panel from opposite oblique angles |
| Backlight | Own driver, PWM | The old module hid this behind a BLK pin |
| Audio | MAX98357A + 20-28 mm speaker in a sealed chamber | New. §5 |
| Radio | **None** | No Wi-Fi, no Bluetooth. Keeps FCC to the unintentional-radiator path |
| USB | One USB-C, charge and data | The stripboard had two ports (charger USB-C + Pico micro-USB) |

---

## 3. Buttons and ergonomics

Per seat: D-pad (4) + A + B + INFO + MENU = 8 inputs. A SNES D-pad silicone
gives the first four and a SNES ABXY silicone gives the other four, so two
molded pieces per seat with nothing left over. SNES replacement membranes and
button caps are cheap and widely stocked.

Geometry measured from the SNES OEM controller replacement PCB
(`snes-controller-main-pcb.kicad_pcb`, CC BY-SA 4.0, see
`hardware/membrane/README.md` for the licence caveat):

| Cluster | Spacing |
|---|---|
| D-pad | 17.75 mm between opposite directions |
| ABXY diamond | 27.0 mm wide, 21.0 mm tall |
| Contact pad | 5 mm interdigitated, 0.3 mm fingers, 0.3 mm gaps, 23 fingers per net |
| D-pad centre to ABXY centre | 82.0 mm |

**The compact alternative is DS Lite**: a **15.0 x 15.0 mm** face-button diamond
against the SNES 27.0 x 21.0 mm, measured from a membrane breakout board's
gerbers and corroborated by an unrelated project aligned to the real console
(`hardware/membrane/README.md`). Its contacts are 4.52 mm concentric rings
rather than combs, which tolerates a rotated pill better. DS Lite membranes are
in production as repair spares, and Adafruit's cuttable elastomer keypad sits on
the same 15 mm grid as a second source.

**Decided 2026-09-21: DS Lite, on the gerber numbers as measured.** They are an
operational assumption, not calipered, and cheap to revise: the pads are
generated, so a corrected pitch is a re-run rather than a redraw.

**Open: cluster spacing.** Board envelope, panel fixed:

| Donor and spacing | Board |
|---|---|
| SNES at 82 mm, as the controller does it | 132 x 158 mm |
| SNES at 45 mm, pieces nearly touching | 86 x 158 mm |
| DS Lite at 50 mm | 91 x 158 mm |
| DS Lite at 40 mm | 81 x 158 mm |

Height barely moves, because the panel sets it. Width is the only variable, and
the panel's own 55.5 mm puts a floor near 72 mm. SNES at full controller spacing
wraps a 132 mm slab around a 55.5 mm panel, which is out of proportion; DS Lite
parts sit near the panel's own width. Compare the stripboard at 61 x 142 mm.
Silicone piece widths in that table are estimates, and they set the floor on
spacing, so that floor is the one number still worth calipering.

**Take the dual footprint idea** from that same project: its
`SNES_BTN_soft+mech` pad accepts either a membrane or a tact switch. Using that
shape keeps the membrane decision reversible in copper.

---

## 4. Pin budget

RP2350B has 48 GPIO, eight of them ADC-capable.

| Function | Pins |
|---|---|
| Buttons, 8 per seat, direct to GPIO | 16 |
| Display: CS, SCK, MOSI, RST, DC, BL | 6 |
| Audio: I2S BCLK, LRCLK, DIN | 3 |
| Battery sense (ADC pin) | 1 |
| Charger status | 1 |
| Soft power button | 1 |
| Status / error LED | 1 |
| UART0 TX, debug | 1 |
| **Total** | **30** |

No matrix and no diodes: 16 switches straight to GPIO with internal pull-ups,
active low, so cross-player ghosting cannot happen. 18 pins spare.

Firmware: `embassy-rp` already has an `rp235xb` feature beside the `rp235xa` one
in use, so the port is that flag plus a new pin map.

---

## 5. Audio

Never existed on any revision, so there is nothing to port.

- **Chain:** RP2350B I2S (embassy-rp ships `pio_programs/i2s.rs`) to MAX98357A
  to speaker. The amp runs off the battery rail.
- **Bar to clear:** GBA quality, which is 8-bit samples plus tone channels
  through a small mono speaker. The electronics beat that comfortably; the
  speaker and the case decide the result.
- **Speaker:** 20-28 mm, 8 ohm, ~1 W, with a sealed back chamber and a grille
  designed into the case. Open-backed into the case interior sounds thin.
- **Mixer lives in core.** Parity rule, same as the renderer.

---

## 6. Before it can be sold

- **Rename.** "Mega Blastoise" is a Pokemon. That includes the USB manufacturer
  string in `mega_blastoise_fw/src/subsystems/usb_v2.rs:157`, which every host
  sees on plug-in.
- **Real USB IDs.** `usb_v2.rs:156` uses the placeholder `0xc0de:0xcafe`.
  Raspberry Pi allocates free PIDs under its VID for products built on its
  silicon (`github.com/raspberrypi/usb-pid`).
- **No preloaded firmware.** Ships blank; the user downloads. Keeps the sprites
  and names off the product.
- **Certification.** FCC Part 15B in the US, CE in the EU. No radio keeps this
  on the cheap path. Market to adults; a children's product triggers CPSIA
  testing in the US.
- **Battery paperwork.** UN38.3 test summary from the cell supplier.

---

## 7. Open questions

1. Cluster spacing (§3). Needs a real membrane in hand to fix the floor.
2. Exact IPS panel part, with a datasheet and a stable part number. The
   Amazon-sourced ST7796 module is not a production part.
3. Layer count. Raspberry Pi's reference core is 2-layer; four may be easier
   once buttons, audio and display routing are on the board.
4. Enclosure process: 3D print for prototypes, injection moulding at volume.
   The `cad/` subproject moves onto the critical path because the case is what
   holds the membranes against the board.
5. Whether rev A carries an RP2350B module footprint as a hedge, or goes
   straight to the bare chip. Current lean: bare chip, with one Solder Party
   RP2350 Stamp XL bought separately for firmware bring-up while boards are at
   the fab.
