# Membrane button contact pads

Reference footprints for silicone membrane buttons (rubber dome + carbon pill
pressing on bare PCB pads), for the production board that replaces
`hardware/stripboard_v2/`. Buttons are aftermarket Nintendo replacement
membranes, so the PCB pads must match what those pills were molded for.

## What is here

`ButtonPads.pretty/` is vendored from
<https://github.com/nataliethenerd/KicadConductivePads>, released under the
Unlicense (public domain, see `UPSTREAM_LICENSE`). Sizing is based on Game Boy
Color / Game Boy Pocket pads.

| Footprint | Outer extent | Use |
| --- | --- | --- |
| `conductive pad` | 4.95 x 4.94 mm round, interlocking fingers | D-pad directions, A, B |
| `STSL` | 7.10 x 3.40 mm oblong | Start / Select style keys (INFO, MENU) |

Both define `F.Mask` openings, so the contact area stays bare copper.

## The production footprint: `DSL_button_4ring`

Generated, not hand-drawn, by `tools/gen_dsl_button.py` into the project library
(`hardware/prod_board/prod_board/prod.pretty/`). Re-run it to change a
dimension; do not edit the output.

    python3 tools/gen_dsl_button.py <path>/prod.pretty
    python3 tools/check_contact_fp.py <path>/prod.pretty/DSL_button_4ring.kicad_mod

| Property | Value |
|---|---|
| Rings | 0.720 / 1.260 / 1.720 / 2.180 mm, the DS Lite's measured radii |
| Conductor | 0.1524 mm, the DS Lite's measured width |
| Min copper gap between nets | **0.200 mm**, checked from the saved file |
| Smallest contact spot that bridges the nets anywhere | **1.60 mm across**; a DS Lite pill is 3.5-4 mm |
| Pads | 1 = signal, 2 = common. Each is one connected custom pad |
| Mask | one 4.80 mm opening; pads carry no mask or paste of their own |
| BOM / pick-and-place | **excluded from both**: it is copper, nothing to buy or place |

**Structure: two stepped-spiral combs, 180 degrees apart.** Each net runs arc,
radial step, arc, step, arc, step, arc, alternating halves of the pad as it
goes out. Neither net ever crosses the other. It differs from the original,
which ties its arcs together with bus bars through the middle and fills the
centre with a split disc (see below); ours joins them with radial steps and
leaves a 1.29 mm hole in the middle, which is where the worst-case contact spot
sits. Same principle, our own artwork, so nothing of theirs is copied.

**The gap between nets is wider on the inner rings** (29.3 degrees at the
inside, 9.3 at the outside) because it has to be the same 0.20 mm of copper at
every radius. The tightest spot is not the chord between the two arc ends: it is
the perpendicular from one net's arc end onto the other net's radial step, just
inside the ring, so the gap angle is asin((clearance + width) / r). A first
version used the chord and came out at 0.1965 mm; the checker caught it.

**Gotcha for anyone scripting KiCad pads:** `PAD.AddPrimitiveShape` keeps the
pointer it is given without copying it. Set `shape.thisown = False` first, or
Python frees the shape and its memory gets reused, and the pad silently saves
whatever landed there. The first run saved the mask circle as the pad's copper.

## Four face buttons (ABXY)

The SNES OEM controller replacement PCB is the best documented source, and it is
a KiCad project rather than gerbers:
<https://codeberg.org/taylor-madeak/oshw-snes-controller-pcb> (CC BY-SA 4.0,
OSHWA certified US002604). It is a drop-in board for original SNES controllers,
so it carries the real pad geometry for the OEM membranes.

Its footprint library `hardware/libraries/Nintendo.pretty` has:

- `SNES_BTN_5mm_0.3_0.3`: 5 mm interdigitated pattern, 0.3 mm fingers with
  0.3 mm gaps, 23 fingers per net.
- `SNES_BTN_soft+mech_5mm_0.3_0.3`: the same pad plus tactile switch holes, so
  one board takes either a membrane or a tact switch. Worth copying the idea for
  the first production board.

Measured placements from `snes-controller-main-pcb.kicad_pcb` (these are facts
about the OEM layout, not copied artwork):

| Cluster | Spacing |
| --- | --- |
| D-pad | 17.75 mm between opposite directions, so 8.9 mm from centre per arm |
| ABXY diamond | 27.0 mm left to right (Y to A), 21.0 mm top to bottom (X to B) |
| Start / Select | 16.1 mm apart |
| D-pad centre to ABXY centre | 82.0 mm |

That last row is the one to keep in mind: the stripboard put a player's two
clusters ~30 mm apart, which Alex already called cramped. A real controller is
82 mm.

### DS Lite, the compact option

Measured 2026-09-17 from the gerbers of the "DS Lite Membrane PCB" sold by
Ampersand on Tindie (<https://www.tindie.com/products/ampersand/ds-lite-membrane-pcb/>),
a 31 x 34 mm two-layer breakout. EAGLE 9.3.1 export dated 2022-02-28. **The
board is reversible: top copper carries the D-pad, bottom copper the ABXY
diamond**, which is why the two sides measure differently. The product page says
"do whatever you like with them" but ships no licence file, so treat these as
dimensions (facts, freely usable) and redraw the copper rather than copying
their artwork into a board we sell.

| Property | DS Lite | SNES |
|---|---|---|
| Face-button diamond | **15.000 x 15.000 mm** (adjacent buttons 10.607) | 27.0 x 21.0 mm |
| D-pad | 14.000 mm on this board; 13.5 mm per an independent project | 17.75 mm |
| Contact pattern | 4.52 mm dia | 5 mm |
| Pattern style | 4 concentric bands, 0.1524 mm conductors, 0.381 / 0.305 / 0.305 gaps | interdigitated comb, 0.3/0.3 mm |

Both spacings were read from the solder mask bar flashes: bottom layer at
(8.0,15.5) (23.0,15.5) (15.5,8.0) (15.5,23.0) gives exactly 15.000 mm, top layer
at (8.5,15.5) (22.5,15.5) (15.5,8.5) (15.5,22.5) gives 14.000 mm. The 15.000 mm
ABXY figure is corroborated by [facelesstech/ds_lite_clicky_buttons](https://github.com/facelesstech/ds_lite_clicky_buttons)
(15.011 x 15.001 mm), an unrelated project aligned to the real console, so treat
**ABXY 15.0 mm as a property of the console** and the D-pad number as
breakout-specific until calipers say otherwise.

The 4.52 mm pattern implies a carbon pill around 3.5-4.0 mm, which matches the
keypad design guide rule that the electrode should be 1.25x the pill or at least
1 mm larger than it.

### The real pattern: two interleaved combs, not alternating rings

Corrected 2026-09-21 after a connectivity analysis of the copper (an earlier
pass binned radii and wrongly called it four concentric rings alternating
nets). Each ring is **cut into two arcs on different nets**, and the two nets
form identical combs 180 degrees apart. Walking outward from the centre:

```
   45 deg : COMMON   SIGNAL   COMMON   SIGNAL
  135 deg : COMMON   SIGNAL   COMMON   SIGNAL
  225 deg : SIGNAL   COMMON   SIGNAL   COMMON
  315 deg : SIGNAL   COMMON   SIGNAL   COMMON
```

Measured arcs, angles CCW from +X, conductor 0.1524 throughout:

| Radius | COMMON arc | SIGNAL arc |
|---|---|---|
| 0.720 | 10.2 to 169.8 | 189.6 to 350.4 |
| 1.260 | 208.5 to 335.6 | 28.9 to 155.0 |
| 1.720 | 4.3 to 144.4 | 184.0 to 324.6 |
| 2.180 | 195.8 to 346.2 | 16.1 to 165.9 |

**How the original ties each net together (corrected 2026-09-23).** Tracing
connectivity across the whole board, not a crop, shows it does not use radial
steps. Two horizontal bus bars run through the middle, one per net, and each
ring's half-arcs hang off them; the centre is a disc split into two halves, one
per net. The short 4-7 degree arcs that cross every ring are where those bars
pass through. An earlier note here called them radial jumpers; that was an
inference from a crop and it was wrong.

**Why this beats alternating full rings.** Each net is one continuously
connected comb, so neither net has to cross the other, and no ring needs a
break for a spoke to pass. It also means each pad is a single connected shape,
which is what KiCad wants for a custom pad. A pill landing anywhere still
crosses alternating nets on its way out from the centre.

**Verified island count:** the bottom copper layer is exactly 5 separate pieces
of metal, one large pour (the common net) and four button signals.

Also worth copying:
- **The common net is the ground pour itself**, so each button costs one signal
  trace and nothing else.

Each button position also carries a small SMD footprint (one 5.8 x 1.4 mm pad
plus two 1.0 x 0.6 mm pads at +/-3 mm), which looks like an alternative switch
option in the same spot.

**Working assumption (2026-09-21): these gerber numbers are correct.** Alex's
call: build to 15.0 mm ABXY, 14.0 mm D-pad and 4.52 mm pads now, and revisit if
calipers on a real membrane later disagree. Nothing downstream should wait on
that measurement.

**Verified: the mask opens the whole pad.** A rendering of the bottom copper with
the bottom mask overlaid shows a full circular opening over each 4.52 mm ring
stack, plus the bar. A research pass claimed the mask exposed only the
1.4 x 5.8 mm bar and left the rings covered; that is wrong, and the render is
the evidence. Regenerate it with `tools/gerb.py` if this ever needs
re-checking.

**Second source at the same pitch.** [Adafruit 1611](https://www.adafruit.com/product/1611)
is a silicone elastomer keypad on a 15 mm grid, explicitly cuttable, around
$0.25 per button, and Adafruit publishes its mating footprint (0.400 mm traces,
0.380 mm gaps, annulus 4.81-10.58 mm dia) in the Adafruit_Trellis EAGLE library.
A cut-down 2x2 from that sheet is a drop-in if DS Lite spares ever dry up, which
is worth designing toward given the whole plan rests on aftermarket parts.

**Metal snap domes remain the fallback** if silicone disappoints, and unlike
loose silicone domes they have catalogue part numbers at MOQ 1 with published
mating pad geometry: Keystone 5133TR (8.40 mm dia, 220 g, five 3.0 mm pads on an
8.0 mm bolt circle), Snaptron F06180 (6.00 mm, 180 g, 5M cycles). Retention is
off the shelf too, via Snaptron Peel-N-Place or Keystone's 5200 adhesive spacer
sheet, so domes can sit at any spacing at least 3 mm apart. Note that Snaptron's
published double-sided pad table is internally inconsistent for the 4-16 mm
rows; use Keystone's drawing instead.

**Custom silicone tooling, for comparison:** $800-4,000 and 3-5 weeks, paying
off somewhere around 200-500 units. Avoiding tooling is the premise of using
aftermarket parts, but at real volume the saving is smaller than it looks.

For the GBA route there is no OEM four-button pad. Freeplay's GBA-shell boards
get X/Y by fitting a **second A/B silicone pad** beside the first, which is the
cheap way to reach four face buttons without leaving GBA parts.

## Cross-checking against a real console

Open hardware recreations of Game Boy boards carry the OEM pad geometry at true
scale, which beats measuring a donor board by hand:

- Game Boy Advance: <https://github.com/MouseBiteLabs/Game-Boy-Enhance>
  (gerbers only, CC BY-SA 4.0). Import the gerbers into KiCad to read exact
  shapes.
- Game Boy Pocket / Color: <https://github.com/MouseBiteLabs/Game-Boy-Pocket-Color>
  (CC BY-SA 4.0).
- Game Boy Advance SP: <https://github.com/vtx22/AGB-PCB> is **CC BY-NC-SA**.
  Non-commercial only, so do not copy from it into a board we intend to sell.

CC BY-SA is share-alike: copying that copper artwork into our board can oblige
us to publish our board files under the same licence. Use the public-domain
footprints above as the working source and treat the recreations as a
dimensional sanity check only.

## Other donors surveyed

Licence matters here because the product gets sold. Ranked by usefulness:

| Project | Files | Licence | Membranes | Geometry |
|---|---|---|---|---|
| [Ampersand DS Lite breakout](https://www.tindie.com/products/ampersand/ds-lite-membrane-pcb/) | gerbers | none stated, page says "do whatever you like with them" | DS Lite | ABXY 15.0, pad 4.52 |
| [facelesstech/ds_lite_clicky_buttons](https://github.com/facelesstech/ds_lite_clicky_buttons) | KiCad | **CC BY-NC-SA, noncommercial** | DS Lite, but metal domes | dimensional reference only |
| [HHL-ProCon-Parts](https://github.com/HandHeldLegend/HHL-ProCon-Parts) | KiCad | GPL-3.0, commercial OK | Switch Pro | ABXY 23.8 x 20.4, double spiral |
| [KicadConductivePads](https://github.com/nataliethenerd/KicadConductivePads) | footprints | **Unlicense**, cleanest of the lot | GBC / GB Pocket-ish | pad only, vendored here |
| [game-and-watch-hardware](https://github.com/Upcycle-Electronics/game-and-watch-hardware) | KiCad | MIT | Game & Watch, A/B only | round contact 4.8 x 5.0 |
| [OpenTendo-AGB](https://github.com/Redherring32/OpenTendo-AGB) | KiCad | CC BY-SA | GBA | 2 buttons, A to B ~13.7 |

Noncommercial, so unusable for a product whatever else they offer: the
facelesstech board above, `progcc-4-hw`, `Retro-Lite-CM5`,
`nataliethenerd/AGB_ReverseEngineer`. `HHL-KiCad-Lib` states no licence at all,
which means all rights reserved.

**No open design files exist at all** for 3DS, New 3DS, 3DS XL, DSi, PSP, PS
Vita, Joy-Con, Wii Classic Controller or Wii U. ODROID-GO publishes only a
schematic PDF; PyGamer and PyBadge publish EAGLE but use tact switches.

Also checked and rejected, so nobody searches them twice: `hackzone/cz20-badge`
(9.0 mm pad, no licence stated, no four-button cluster) and `jtgans/RetroPad`
(12 mm pad, no licence stated, a crude two-bar contact rather than rings or a
comb). Both are roughly twice the pad diameter we want.

## Loose single silicone domes

Viable but poorly supported. Available from Chinese marketplaces in maybe 8-10
stock sizes at roughly $0.17-0.22 each with the pill included, and absent from
every Western distributor. **Actuation force is not published for any of them**,
so choosing one means ordering samples, and with no carrier sheet each dome
needs its own pocket in the case. That capture work is the real price of
arbitrary spacing.

Correction to an earlier note in this project: Freeplay's "rubber nipple"
buttons are ordinary 12 x 12 mm tact switches with a silicone cap, not
conductive domes. Freeplay's actual conductive-rubber advice is to cut up the
GBA's own A/B pad.

## Surface finish: prefer carbon ink

Rubber keypad design guides want **30-50 microinches of hard gold over nickel**
on contact areas. JLCPCB's ENIG is **1-2 microinches** of immersion gold, an
order of magnitude thinner, so a bare-ENIG contact can drift upward in
resistance over the product's life. Hard gold that thick is a specialist finish
and not cheap.

**Carbon ink screen printed over the copper sidesteps the whole question.** It
cannot oxidise, it is rated for over a million presses, and it costs cents per
board. It is what remotes and OEM controllers use. Never HASL or OSP.

**But carbon ink and the 4-ring pattern are incompatible.** Carbon is screen
printed, so it resolves far coarser than etched copper: Eurocircuits specifies
0.30 mm lines and 0.40 mm gaps for carbon contacts. `DSL_button_4ring` has
0.15 mm conductors and gaps down to 0.20 mm, so printed carbon would bridge the
two nets. **The finish and the ring count are one decision:**

| Pattern | Finish it needs |
|---|---|
| 4 rings (`DSL_button_4ring`) | ENIG or hard gold; accept that thin ENIG gold may drift over the product's life |
| ~2 rings, 0.3 mm lines, 0.4+ mm gaps | carbon ink |

The 2-ring learning footprint (`DSL_button`: 0.20 mm lines, radii 1.1 and 2.0)
is already close to carbon-compatible; at 0.30 mm lines it would be. **Open:
pick one.** The test coupon should carry both, one ENIG and one carbon, pressed
against real DS Lite membranes.

## Fab requirements for these pads

- ENIG or hard gold for the 4-ring pattern; carbon ink only for a coarse
  pattern (see above). Never HASL.
- No solder mask, silkscreen, or vias inside the contact area.
- Nothing else under the membrane footprint, so the silicone seats flat and its
  moulded air channels stay clear.
- A carbon pill reads roughly 20-100 ohm fresh, a few hundred worn, which is
  solidly low against the RP2350's internal pull-ups.

## Before committing the main board

Order a small test coupon carrying several pad variants (finger count, pattern
diameter, with and without carbon ink) plus a header, and press the actual
aftermarket membranes onto it.
