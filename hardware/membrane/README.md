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

## Fab requirements for these pads

- ENIG finish, or carbon ink screen printed over the copper. Never HASL.
- No solder mask, silkscreen, or vias inside the contact area.
- Nothing else under the membrane footprint, so the silicone seats flat and its
  moulded air channels stay clear.
- A carbon pill reads roughly 20-100 ohm fresh, a few hundred worn, which is
  solidly low against the RP2350's internal pull-ups.

## Before committing the main board

Order a small test coupon carrying several pad variants (finger count, pattern
diameter, with and without carbon ink) plus a header, and press the actual
aftermarket membranes onto it.
