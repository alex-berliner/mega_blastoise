> **Archived (superseded).** The reference design replaced this page's MAX98357A with PWM audio into an FM8002A, its buck-boost with an ME6211 LDO, and dropped the cover lens (2026-09-24).
> The current design lives in [DESIGN.md](../../../DESIGN.md) and [hardware/ref_design/](../../../hardware/ref_design/README.md).

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
| MCU | **Bare RP2350B** (QFN-80, 48 GPIO) | Settled 2026-09-21 after checking modules, see §8. Not the A: §4 needs more than 30 pins. Fab house solders it; 0.4 mm pitch is not hand-solderable |
| Core circuit | Copy Raspberry Pi's RP2350B Minimal KiCad design | Their regulator layout is mandatory, including the polarity-marked Abracon inductor. ABM8-272-T3 crystal, W25Q128JVS 16 MB flash |
| Charger | Power-path IC (BQ25185 class), **not** TP4056 | TP4056 has no power path: a running device and the cell share charge current and termination gets unreliable |
| Cell | Protected LiPo with an NTC, on JST-PH | Carriers require UN38.3 test docs from the supplier |
| 3V3 rail | Buck-boost | Uses the cell down to ~3.0 V; the stripboard's Schottky drop is gone |
| Power switch | Soft button, hold to toggle | Also delivers the hold-to-reset behaviour the reset-button TODO wanted |
| Display | Bare **IPS** panel on FPC, 3.5 in 320x480 class, behind an **anti-glare** cover lens | IPS still mandatory: two seats view one flat panel from opposite oblique angles. Shortlist and the driver-IC trap in §9 |
| Backlight | Topology **depends on the panel's LED string**, PWM dimming | Series string: one boost constant-current driver. Parallel bank at ~3.2 V: a boost cannot regulate inside the battery window, so it needs a fixed ~4.7 V rail plus a current sink. §9 |
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
  string in `crates/mega_blastoise_fw/src/subsystems/usb_v2.rs:157`, which every host
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
2. Which panel from the §9 shortlist.
3. Layer count. Raspberry Pi's reference core is 2-layer; four may be easier
   once buttons, audio and display routing are on the board.
4. Enclosure process: 3D print for prototypes, injection moulding at volume.
   The case work (once the v1 `cad/` subproject, now at tag `v1-archive`) moves onto the critical path because the case is what
   holds the membranes against the board.
5. Resolved, see §8.

---

## 8. Why the bare chip, not a module

Checked 2026-09-21, because a prefab module the fab could solder would have
removed the core-layout risk entirely. **It does not exist.** JLCPCB's library
carries the bare RP2350A and RP2350B, plus the Pico family (Pico, Pico W,
Pico 2 W) as LCC-43 parts they will place on the cheap tier with a fixture.
Every RP2350B module is a small-vendor part LCSC does not stock:

| Module | Why not |
|---|---|
| Solder Party RP2350 Stamp XL, $11 | right shape, not an LCSC part, consignment only |
| Waveshare Core2350B, $5-8 | 2.54 mm through-hole pins, and USB exits on a 0.5 mm FPC, so it is a plug-in board not an SMT part |
| Olimex PICO2-XL, WeAct | not LCSC parts |
| Pimoroni PGA2350 | pin grid array, not placeable |

JLC does not accept customer-supplied parts outside their consignment
programme, so "the house solders it" only applies to their own library.

The Pico family would be placeable, but it is RP2350A: 26 exposed GPIO against
the 30 in §4, which forces an MCP23017 expander, drops flash to 4 MB, and
leaves our single USB-C needing the module's underside test pads to reach the
data lines. The bare chip avoids all three, costs about $1.06, and puts the only
delicate part of the layout on Raspberry Pi's reference, which their guide
requires us to copy anyway.

---

## 9. Panel

Surveyed 2026-09-21. **The 3.5 inch 320x480 IPS class is the only size in range
with datasheet-backed, distributor-stocked IPS supply.** 3.2 inch 240x320 IPS
barely exists as a purchasable part, so dropping resolution to save SRAM would
cost us the supply chain.

Nearly every candidate shares the same glass: **active area 48.96 x 73.44 mm,
outline about 54.5 x 83 mm**. Lay the board and case out to that and several
sources stay interchangeable.

### Two findings that change the design

**ILI9488 over 3-wire SPI cannot do RGB565.** It offers 3 bits per pixel or 18,
and RGB565 exists only on its 4-line SPI and parallel interfaces. **ST7796S does
do RGB565 over SPI.** So an ILI9488 panel that exposes only 3-line SPI wants a
PIO 8080 parallel driver instead: 12 pins rather than 6, which the budget in §4
still absorbs (36 of 48), and about 81 fps against 15.

**The framebuffer question is moot.** ST7796S, ILI9488 and ST7789V all carry
full-resolution GRAM on chip, so the panel holds and refreshes the image itself
and we push dirty rectangles into it. That already matches what the firmware
does with 16x16 dirty tiles. A 320x480 RGB565 framebuffer would be 300 KiB of
the RP2350B's 520 KB; we do not need one. If one is ever wanted,
the B package can map QSPI PSRAM on XIP CS1.

### Shortlist

| Panel | Price | Driver / interface | Notes |
|---|---|---|---|
| **Displaytech DT035CTFT-IPS-SHB** | $33.73 @1, Digi-Key stock | ST7796UI, **4-wire SPI RGB565** | Best engineering fit: existing SPI plumbing works as-is. 1000 nits. IPS 80/80/80/80 verified. Against: priciest, single source, odd **39-pin** tail whose matching ZIF is unconfirmed |
| **Winstar WF35UTYAIDNN0 / Raystar RFI350U-AYW-DNN** | ~$15 @1 | ILI9488, **3-line SPI only** | Best production position: two makers, one identical part, 40-pin 0.5 mm standard tail, **anti-glare polariser included**. Costs a PIO 8080 driver and an awkward backlight |
| 4D Systems 4DLCD-35-320-480 | $26 @100 | ILI9488, MODE pin selects 4-wire SPI | Best documentation of the lot, but **320 nits is too dim** behind a cover lens |
| EastRising ER-TFT035IPS-6 | maybe $12-20 | ILI9488, 4-wire SPI | Probably cheapest; **nothing verified**, their site blocks automated fetches. Worth an email |

Rejected for being TN, which one seat would see inverted: Crystalfontz
CFAF320480C7-035TN, Raystar RFJ320D, Midas MDT0320ASH, and the cheap LCSC
HS35B01A whose own datasheet says "12 o'clock" with viewing angles "TBD".

### Backlight

- **Series string** (Displaytech 3S2P, 11.4-13.4 V at 60 mA): one boost
  constant-current driver. TPS61161 class, about $0.26 at 100.
- **Parallel bank** (Winstar/Raystar, 3.0-3.4 V at 160 mA): the LED forward
  voltage sits *inside* the 3.0-4.2 V battery window, so a boost cannot
  regulate and the 3V3 rail has no headroom. Either ask the vendor for a
  series-string variant, which they routinely build, or run a fixed ~4.7 V
  boost into a constant-current sink.
- Full brightness costs 0.37-0.74 W depending on panel; indoors at 30-50% PWM
  budget 0.15-0.35 W. This dominates battery life.

### Cover lens

**Anti-glare (matte), not anti-reflective, and an air gap rather than optical
bonding.** A flat face-up panel reflects ceiling lights into both players' eyes
symmetrically; AR still leaves a sharp legible mirror image, while AG diffuses
it. AG is also cheaper, and the Winstar/Raystar glass already has it. Optical
bonding is not worth it at our volume.

A good air gap means all of: AG lens, a **printed black border** hiding the
panel's inactive edge and the FPC transition, a **foam perimeter gasket** for
dust and light piping, and the smallest gap that avoids Newton's rings when
someone presses on it. Mount with a perimeter ring of 3M 9495LE, never a
full-face pad. Prototype in non-glare cast acrylic; move to chemically
strengthened glass for production, since acrylic scratches on a shared surface
people touch.
