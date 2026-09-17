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
a 31 x 34 mm two-layer breakout that accepts DS Lite ABXY membranes. EAGLE
9.3.1 export dated 2022-02-28. **No licence ships with the download**, so treat
these as dimensions (facts, freely usable) and redraw the copper rather than
copying their artwork into a board we sell.

| Property | DS Lite | SNES |
|---|---|---|
| Face-button diamond | **14.0 x 14.0 mm** | 27.0 x 21.0 mm |
| Contact pattern | ~4.4 mm dia | 5 mm |
| Pattern style | concentric rings, 0.15 mm traces, ~0.35 mm gaps | interdigitated comb, 0.3/0.3 mm |

Two things worth copying from it:

- **Concentric rings beat a comb** for rotation tolerance. A pill bridges
  adjacent rings whichever way it lands; a comb has one orientation where an
  off-centre pill can sit mostly on one net.
- **The common net is the ground pour itself**, so each button costs one signal
  trace and nothing else.

Each button position also carries a small SMD footprint (one 5.8 x 1.4 mm pad
plus two 1.0 x 0.6 mm pads at +/-3 mm), which looks like an alternative switch
option in the same spot.

**Unresolved, 14 vs 15 mm.** The 14.00 mm above is measured straight out of that
breakout's gerbers and is exact for that board. A research pass reported the DS
Lite diamond as 15.000 mm, source not yet pinned down; it may be the console
itself rather than one person's breakout of it. Settle it with calipers on a
real membrane before committing copper.

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

## Surface finish: prefer carbon ink

Rubber keypad design guides want **30-50 microinches of hard gold over nickel**
on contact areas. JLCPCB's ENIG is **1-2 microinches** of immersion gold, an
order of magnitude thinner, so a bare-ENIG contact can drift upward in
resistance over the product's life. Hard gold that thick is a specialist finish
and not cheap.

**Carbon ink screen printed over the copper sidesteps the whole question.** It
cannot oxidise, it is rated for over a million presses, and it costs cents per
board. It is what remotes and OEM controllers use. That makes it the default
here, with ENIG as the fallback if a fab will not print carbon. Never HASL or
OSP.

## Fab requirements for these pads

- Carbon ink over copper preferred; ENIG acceptable. Never HASL.
- No solder mask, silkscreen, or vias inside the contact area.
- Nothing else under the membrane footprint, so the silicone seats flat and its
  moulded air channels stay clear.
- A carbon pill reads roughly 20-100 ohm fresh, a few hundred worn, which is
  solidly low against the RP2350's internal pull-ups.

## Before committing the main board

Order a small test coupon carrying several pad variants (finger count, pattern
diameter, with and without carbon ink) plus a header, and press the actual
aftermarket membranes onto it.
