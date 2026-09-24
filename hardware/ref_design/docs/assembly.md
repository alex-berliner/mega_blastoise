# Building one

## 1. Order the board (JLCPCB)

Files are in `board/fab/` (regenerate with `python3 board/tools/fab.py`).

| Setting | Value | Why |
|---|---|---|
| Gerbers | `battler_gerbers.zip` | copper x4, mask, paste, silk, outline, drills |
| Layers / thickness | 4 / 1.6 mm, JLC04161H-7628 stack-up | In1 = GND plane, In2 = 3V3 plane |
| Surface finish | **ENIG** | the silicone pills press straight on the contact combs: they must be gold, not HASL |
| Via | 0.25 mm drill / 0.5 mm pad (0.3 / 0.6 on power), tented | copied core uses 0.25 mm |
| Min track / space | 0.15 / 0.15 mm (0.1 mm in the RP2350B pin field) | |
| Assembly | top side only, `battler_bom_jlc.csv` + `battler_cpl_jlc.csv` | 91 placements, 40 part numbers |

In JLC's placement preview, check every polarised part turned the way the silkscreen
says: the RP2350B, BQ24074, TPS63001, AP3156 and MAX98357A pin-1 dots, the flash,
USBLC6, BAT54C, 2N7002, the diodes (D2, D3) and LEDs (D1, D4). JLC fixes rotation offsets
on request; the CPL carries KiCad's angles.

Not assembled by JLC (no LCSC number): the panel's landing J3 (JLC's paste leaves its pads
tinned, which is what the hand joint wants), the 16 membrane contacts (bare gold), the
speaker pads, test pads, mounting holes and fiducials.

## 2. Order and make the rest

| Item | Qty | Notes |
|---|---|---|
| QD3525 3.5in 320x480 IPS panel (ST7796U) | 1 | bare panel, 48-contact solder tail; check the part number on the tail against the maker's drawing |
| Li-ion cell 103450 (34 x 50 x 10 mm) with protection board and JST PH 2.0 plug | 1 | **polarity varies by vendor**: pin 1 (silkscreen `+`) must be the positive lead |
| Speaker, 20 mm round, 8 ohm, 1 W, <= 4.5 mm thick, wire leads | 1 | |
| DS Lite silicone: D-pad piece + ABXY piece | 2 of each | aftermarket replacements |
| DS Lite caps: D-pad cross + A, B, X, Y | 2 sets | |
| Case: `case/tub.stl`, `case/lid.stl`, `case/pwr_cap.stl` | 1 each | print them yourself in PETG (or PLA), the lid face down. The lid is only 0.6 mm thick over the battery connector: print it at 100% infill and check that spot. FDM holes come out small: drill the 1.2 mm pinholes and 1.7 mm screw pilots to size |
| Cover lens, 1.0 mm clear acrylic, `case/lens.dxf` | 1 | laser cut, glued into the lid's top recess |
| Double-sided tape for the panel (e.g. 3M 9495LE), 1 mm strips | ~30 cm | panel to the lid's window ledge |
| Light pipe, 2.0 mm clear acrylic rod, 5.5 mm long | 2 | over the charge and error LEDs |
| M2 x 16 self-tapping screws (pan head) | 6 | from below into the lid bosses |
| Foam pad, 0.5 mm, 30 x 45 mm | 1 | between the cell and the board |

## 3. Hand work on the board

1. Speaker: solder its leads to J4 on the back of the board, `+` to the pad marked `+`.
   Keep the leads short; the speaker sits on the tub floor right below.
2. Look over the charger and regulator corners (BQ24074, TPS63001, AP3156, MAX98357A):
   their exposed pads must have wetted. X-ray is not needed for a prototype; a lifted
   corner pin shows under a loupe.
3. Panel tail to J3. Lay the panel face down on the bench beyond the board's seat-1 edge,
   its tail reaching back flat over J3, contacts down. Line the tail's edges up with the
   silk ticks either side of J3, and its contact 1 with J3's pin-1 bar (left end seen
   from above, seat 1 nearest you). Tape the panel down. Flux the row, tack the two end
   contacts, check the alignment, then drag a fine chisel tip along the row (the pads are
   already tinned) or use a hot-bar tool. Check for bridges under a loupe and clean the
   flux off. The panel stays face down until the case goes together.

## 4. First power, before the case

Follow [bringup.md](bringup.md) with the board on the bench: USB first, then the cell.

## 5. Put it together

1. Lid, face down on the bench: glue the lens into the top recess, push the light pipes
   into their holes, stick the tape strips on the window ledge (leave the liner on).
2. Tub: foam pad and cell into the cell pocket, cable towards seat 1; speaker into its ring
   on the floor, face down; power cap into its wall slot.
3. Board into the tub over them, resting on the six bosses and the four ribs under the
   button clusters, USB-C in its wall opening. Feed the cell's plug up through the slot
   next to J2 and plug it in (J2's mouth faces the slot).
4. Panel: fold it over onto the board, face up, its tail looping under it at the seat-1
   edge (don't crease the tail). It rests loosely on the parts under it for now.
5. Silicone pieces onto the contacts, caps onto the silicone.
6. Peel the tape liner and lower the lid straight down: its locating frame centres the
   panel and the tape takes it, the caps come up through their holes and the retainers
   trap their flanges. From here on the panel hangs from the lid, 2.4 mm clear of the board.
7. Six M2 x 16 screws from below. Snug, not tight: the bosses are plastic.
