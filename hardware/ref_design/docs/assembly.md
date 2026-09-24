# Building one

## 1. Order the board (JLCPCB)

Files are in `board/fab/` (regenerate with `python3 board/tools/fab.py`).

| Setting | Value | Why |
|---|---|---|
| Gerbers | `battler_gerbers.zip` | copper x2, mask, paste, silk, outline, drills |
| Layers / thickness | 2 / 1.6 mm | top: parts, most routing, a 3V3 pour over the chip; bottom: GND pour |
| Surface finish | **ENIG** | the silicone pills press straight on the contact combs: they must be gold, not HASL |
| Min via hole | **0.3 mm** (0.5 mm pads, 0.6 mm on power), tented | JLC's standard: 0.25 mm adds a fee, a 4-wire test and TG155 material (about $40 an order) |
| Min track / space | 0.15 / 0.15 mm (0.1 mm in the RP2354B pin field) | |
| Assembly | Economic PCBA, top side only, `battler_bom_jlc.csv` + `battler_cpl_jlc.csv` | 90 placements, 36 part numbers |

In JLC's placement preview, check every polarised part turned the way the silkscreen
says: the RP2354B, TP4056, AP3156 and FM8002A pin-1 dots, the ME6211, SRV05-4, BAT54C,
AO3401A and the 2N7002s, and the diodes (D2, D3, D5, D6: the band is the cathode).
JLC fixes rotation offsets on request; the CPL carries KiCad's angles.

Not assembled by JLC (no LCSC number): the panel's landing J3 (JLC's paste leaves its
pads tinned, which is what the hand joint wants), the 16 membrane contacts (bare gold),
the speaker pads, test pads, locating holes and fiducials.

## 2. Order and make the rest

The list, with the listings found so far: [bom.md](bom.md), "You". In short: the
QD3525 panel, a 503450 cell with a JST PH plug, a 20 mm speaker, two sets each of DS
Lite silicone and caps, tape and foam, and the printed case (`case/tub.stl`,
`case/lid.stl`).

Before the first board order, check the parts against the design (the approved
fit check): one panel against its drawing, the cell's size with its protection board
and its plug's polarity, the speaker's thickness, and the silicone and caps in a
printed lid. The numbers to measure are in [design_review.md](design_review.md).

**Cell polarity varies by vendor.** Pin 1 of J2 (silkscreen `+`) must be the red
lead. A reversed cell destroys the charger. If a batch arrives reversed, lift each
crimp's latch with a needle, pull the two pins out of the plug and swap them.

Printing: PETG (or PLA), the lid face down. The lid is 0.6 mm thick over the battery
connector: 100% infill there. FDM holes come out small: drill the 1.2 mm pinholes to
size. The snap fit is untested: print one tub and lid first and tune `snap_barb` and
`snap_t` in `case/case.scad` if the lid is too loose or too tight.

## 3. Hand work on the board

1. Speaker: solder its leads to J4 on the back of the board, `+` to the pad marked `+`.
   Keep the leads short; the speaker sits on the tub floor right below.
2. Look over the chips with exposed pads (TP4056, AP3156, RP2354B): their pads must have
   wetted. X-ray is not needed for a prototype; a lifted corner pin shows under a loupe.
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

1. Lid, face down on the bench: stick the tape strips on the window ledge (leave the
   liner on).
2. Tub: foam pad and cell into the cell pocket, cable towards seat 1; speaker into its
   ring on the floor, face down.
3. Board into the tub over them, its six holes over the six pins, resting on the bosses
   and the four ribs under the button clusters, USB-C in its wall opening. Feed the
   cell's plug up through the slot next to J2 and plug it in (J2's mouth faces the slot).
4. Panel: fold it over onto the board, face up, its tail looping under it at the seat-1
   edge (don't crease the tail). It rests loosely on the parts under it for now.
5. Silicone pieces onto the contacts, caps onto the silicone.
6. Peel the tape liner and lower the lid straight down until all six hooks click into
   the tub wall: its bosses take the pins and clamp the board, its locating frame centres
   the panel and the tape takes it, the caps come up through their holes and the
   retainers trap their flanges. From here on the panel hangs from the lid, 2.4 mm clear
   of the board.
7. Opening it again: work a thin blade down between the lid and the tub wall at a hook
   and ease that hook inwards; go round the six.

The power button is seat 1's Y button: hold it for about a second to turn the device on.
