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
says: the RP2354B, TP4056, MCP6002 (U6) and FM8002A pin-1 dots, the RT9013 (U3), SRV05-4 (U1), BAT54C,
AO3401A, the AO3400As (Q5, Q6) and the 2N7002s, the diodes (D2, D3, D5, D6: the band is the cathode), and J3,
the panel's connector (pin 1 at its left end, its mouth towards seat 1).
JLC fixes rotation offsets on request; the CPL carries KiCad's angles. U1 needs one: JLC's
footprint for C13612 is KiCad's SOT-23-6 turned 180 degrees, and placed unturned its GND pin
lands on the +3V3 pad. U3's (C47773) is the same SOT-23-5 footprint JLC used for the ME6211.

Not assembled by JLC (no LCSC number): the 20 membrane contacts (bare gold), test pads,
locating holes and fiducials. J3, the panel's ZIF connector, and J4, the speaker's header, are
assembled.

## 2. Order and make the rest

The list, with the listings found so far: [bom.md](bom.md), "You". In short: the
ER-TFT035IPS-6 panel (buydisplay.com), a 403450 cell with a Molex PicoBlade (MX1.25) plug, INGHAi's GSPK151035PN speaker (its
leads end in a 1.25 mm plug), two sets each of DS
Lite silicone and caps, tape and foam, and the printed case (`case/tub.stl` and `case/top.stl`, the one-piece top).

Before the first board order, check the parts against the design (the approved
fit check): one panel against its drawing, the cell's size with its protection board
and its plug's polarity, the speaker's plug in a PicoBlade header, and the silicone and caps in a
printed top. The fit kit (`case/fitkit/`) has the prints for it: a board mock, dummies
of the loose parts and quick coupons of the case, with a test order in its README. The
numbers to measure are in [design_review.md](design_review.md).

**Cell polarity varies by vendor.** Pin 1 of J2 (silkscreen `+`) must be the red
lead. A reversed cell destroys the charger. If a batch arrives reversed, lift each
crimp's latch with a needle, pull the two pins out of the plug and swap them.

Printing: PLA, every part as exported: the tub back down without supports; the top face down, with supports whose interface layers
(the ones that touch it) are a second filament, Bambu Support For PLA, so the ends' faces, the slopes and the side rounds peel off
clean (two filaments and a prime tower: `case/print_package_README.md` has the plates and the slots). FDM holes come out small:
drill the 1.2 mm pinholes to size. Pull the supports off the top before anything else goes in.

## 3. Hand work on the board

Nothing is soldered at assembly (Alex, 2026-10-20): the cell and the speaker both plug in.

1. Look over the chips with exposed pads (TP4056, RP2354B): their pads must have
   wetted. X-ray is not needed for a prototype; a lifted corner pin shows under a loupe.
2. Leave the empty footprints empty unless bring-up asks for them. C42 and C43 are 1206 (the
   largest common size, 1.6 mm between pads, fine with an ordinary iron). JP1 and JP2 come
   bridged; don't cut them.

## 4. First power, before the case

Follow [bringup.md](bringup.md) with the board on the bench: USB first, then the cell.

## 5. Put it together

It goes together upside down (2026-10-21, `case/SHELL.md`; the cell in the board and the speaker on its top since 2026-10-22; the
one-piece top since 2026-10-25): the top lies face down on its screen face, everything goes into it from behind, and the tub
closes over it all. Every part goes in straight down.

1. **The top**, supports pulled off, face down on a flat surface: it rests on its screen face (the band across the case, the
   panel's window in it); the two ends stand 1.85 mm off the surface, so the caps that stand in them clear it.
2. **The speaker:** peel its front ring's liner and press the speaker face down into its pocket in the right bezel, over the
   grille: the pocket centres it. Stick the tape strips round the panel's pocket (leave the liner on).
3. **Caps** into the top's ends from behind. Each ABXY cap's flange tab drops into a slot and ribs fill the notches in its flange,
   so each cap fits only its own hole, one way round: turn the caps to match. Seen from that seat's player (from the front): A's
   tab (right cap) points down-left, B's (bottom) up-left, X's (top) down-right, Y's (left) up-right; from behind it is mirrored,
   so go by the slots. Then the D-pad and Start/Select, then the **silicone** over them: the walls on the ends' underside locate
   each pad.
4. **The panel:** peel the tape liner and lay the panel glass down into the pocket; the frame centres it.
5. **The cell** into the pocket in the tub's floor on a strip of double-sided tape, pushed against the pocket's right wall (towards
   the USB-C side, where TH1 reads it), its leads at the left end.
6. **Plug in, the board on edge** at seat 1's end, its face towards the panel: flip J3's latch up, slide the FPC's contacts into J3
   (contacts down) until it stops, square, and press the latch shut; the cell's plug into J2, at the corner of the board's hole,
   with the tub held beside it.
7. **The board**, face down towards the bosses (four in the top beside the glass, one at each end), the FPC folding under the panel
   at the seat-1 edge (don't crease it). Just before it lands, the speaker's plug into J4, by the right edge below the speaker,
   its mouth towards the glass (its red lead is +, nearest J4's pin 1; the amplifier is bridge-tied, so a reversed speaker only
   flips its phase): the speaker stays in the top and its 35 mm leads reach J4 only with the board nearly in place. Lay the
   leads round the screw boss between the speaker and J4, on the glass's side, then the board down onto the bosses.
8. **The tub** over it all: the cell drops into the board's hole, the crush ribs on the tub's walls centre the board, its tongue
   slides inside the top's skirt all round, its keys into the pockets in the top's sides and its tab rises into the notch under
   the USB-C (the board's tab in its slot). Press it down all round until the seam is an even line: the grip ribs hold it, and
   the case now holds together on its own.
9. **Six M2 x 6** from the back, up through the tub's bosses and the board into the top's bosses, snug, not tight (they cut their
   own thread in the plastic). They clamp the board between the bosses and hold the case shut, as on a DS Lite.
10. Opening it again: the six screws out, lift the tub off (the cell comes with it, on its leads), lift the board a little and
    unplug J4 first (the speaker stays in the top), then lift the board out.

The power button is seat 1's Start button: hold it for about a second to turn the device on.
