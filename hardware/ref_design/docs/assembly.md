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
| Assembly | Economic PCBA, top side only, `battler_bom_jlc.csv` + `battler_cpl_jlc.csv` | 117 placements, 49 part numbers |

In JLC's placement preview, check every polarised part turned the way the silkscreen
says: the RP2354B, TP4056, MCP6002 (U6) and FM8002A pin-1 dots, the ME6211, SRV05-4, BAT54C,
AO3401A, the AO3400As (Q5, Q6) and the 2N7002s, the diodes (D2, D3, D5, D6: the band is the cathode), and J3,
the panel's connector (pin 1 at its left end, its mouth towards seat 1).
JLC fixes rotation offsets on request; the CPL carries KiCad's angles.

Not assembled by JLC (no LCSC number): the 20 membrane contacts (bare gold), test pads,
locating holes and fiducials. J3, the panel's ZIF connector, and J4, the speaker's header, are
assembled.

## 2. Order and make the rest

The list, with the listings found so far: [bom.md](bom.md), "You". In short: the
ER-TFT035IPS-6 panel (buydisplay.com), a 403450 cell with a Molex PicoBlade (MX1.25) plug, INGHAi's GSPK151035PN speaker (its
leads end in a 1.25 mm plug), two sets each of DS
Lite silicone and caps, tape and foam, and the printed case (`case/tub.stl`,
`case/lid.stl`).

Before the first board order, check the parts against the design (the approved
fit check): one panel against its drawing, the cell's size with its protection board
and its plug's polarity, the speaker's plug in a PicoBlade header, and the silicone and caps in a
printed lid. The fit kit (`case/fitkit/`) has the prints for it: a board mock, dummies
of the loose parts and quick coupons of the case, with a test order in its README. The
numbers to measure are in [design_review.md](design_review.md).

**Cell polarity varies by vendor.** Pin 1 of J2 (silkscreen `+`) must be the red
lead. A reversed cell destroys the charger. If a batch arrives reversed, lift each
crimp's latch with a needle, pull the two pins out of the plug and swap them.

Printing: PETG for the lid (its snap hooks bend across its layers; PLA hooks snapped on the
first print), the lid face down. The lid is 1.1 mm thick over the battery connector: 100%
infill there. FDM holes come out small: drill the 1.2 mm pinholes to size. Tune `snap_barb`
and the hook thickness in `case/case.scad` if the lid is too loose or too tight.

## 3. Hand work on the board

Nothing is soldered at assembly (Alex, 2026-10-20): the cell and the speaker both plug in.

1. Look over the chips with exposed pads (TP4056, RP2354B): their pads must have
   wetted. X-ray is not needed for a prototype; a lifted corner pin shows under a loupe.
2. Leave the empty footprints empty unless bring-up asks for them. C41-C43 are 1206 (the
   largest common size, 1.6 mm between pads, fine with an ordinary iron). R48 is left off on
   purpose: it is the revision strap's bit 0. JP1 and JP2 come bridged; don't cut them.

## 4. First power, before the case

Follow [bringup.md](bringup.md) with the board on the bench: USB first, then the cell.

## 5. Put it together

1. Lid, face down on the bench: stick the tape strips on the window ledge (leave the
   liner on).
2. Into the tub: the cell in its pocket on the floor, lengthwise, its wires at seat 1's end,
   a 0.45 mm foam pad on top. The speaker face down on its pad beside the left wall, between
   the cell and seat 2's end: peel its foam ring's liner and press it down, its solder tab
   towards the wall and its leads towards the middle, so the ring seals round the chamber
   that opens through the wall.
3. Push the cell's plug up through the board's hole (5.45 x 4.4 mm, 5.5 mm in front of J2's
   mouth, between seat 1's clusters at the screen's edge: the whole plug passes), and the
   speaker's plug up through the same-sized hole at seat 2's end (in front of J4), then the
   board into the tub: its tab under the USB-C drops into the slot in the wall (it goes in one
   way only), pressed down past the crush ribs that centre it onto the floor's support ribs and
   bosses. Lay the plug flat, wires towards the hole, and push it into J2: its wires run
   straight from its tail down through the hole, with no fold. The speaker's plug goes into
   J4 the same way (its red lead is +, nearest J4's pin 1; the amplifier is bridge-tied, so a
   reversed speaker only flips its phase). The cell's plug can't reach J4.
4. Panel: lay it face down beyond the board's seat-1 edge, flip J3's lid up, slide the
   FPC's contacts into J3 (contacts down) until it stops, square, and press the lid shut.
   Then fold the panel over onto the board, face up, its FPC looping under it at the
   seat-1 edge (don't crease it). It rests loosely on the parts under it for now.
   A cracked panel comes out the same way: lid up, FPC out.
5. Silicone pieces onto the contacts, caps onto the silicone. Each ABXY cap's flange tab
   drops into a slot in the lid, and ribs in the lid fill the notches in its flange, so each
   cap fits only its own hole, one way round: turn the caps to match. Seen from that seat's player:
   A's tab (right cap) points down-left, B's (bottom) up-left, X's (top) down-right, Y's
   (left) up-right. The slots are 30 degrees wide, so close is enough; if the lid won't
   seat, a cap is turned too far.
6. Peel the tape liner and lower the lid straight down until all four hooks click into
   the tub wall and its lip seats in the wall's step: its locating frame centres the
   panel and the tape takes it, the caps come up through their holes and the retainers
   trap their flanges. From here on the panel hangs from the lid, its top in the lid's pocket.
7. Turn it over and drive the six M2 x 8 screws up through the tub's bosses and the
   board into the lid's bosses, snug, not tight (they cut their own thread in the plastic).
   They clamp the board between the bosses and hold the case shut, as on a DS Lite.
8. Opening it again: the six screws out, then a thin blade down between the lid and the
   tub wall at a hook to ease it inwards; go round the six.

The power button is seat 1's Start button: hold it for about a second to turn the device on.
