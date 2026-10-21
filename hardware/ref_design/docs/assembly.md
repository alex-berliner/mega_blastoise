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
ER-TFT024IPS-3 panel (buydisplay.com; the board is still laid out for the 3.5 in ER-TFT035IPS-6 until it is re-placed for the 133 mm case), a 403450 cell with a Molex PicoBlade (MX1.25) plug, INGHAi's GSPK151035PN speaker (its
leads end in a 1.25 mm plug), two sets each of DS
Lite silicone and caps, tape and foam, and the printed case (`case/tub.stl`,
`case/top.stl` and two of `case/deck.stl`).

Before the first board order, check the parts against the design (the approved
fit check): one panel against its drawing, the cell's size with its protection board
and its plug's polarity, the speaker's plug in a PicoBlade header, and the silicone and caps in a
printed deck. The fit kit (`case/fitkit/`) has the prints for it: a board mock, dummies
of the loose parts and quick coupons of the case, with a test order in its README. The
numbers to measure are in [design_review.md](design_review.md).

**Cell polarity varies by vendor.** Pin 1 of J2 (silkscreen `+`) must be the red
lead. A reversed cell destroys the charger. If a batch arrives reversed, lift each
crimp's latch with a needle, pull the two pins out of the plug and swap them.

Printing: PLA, every part as exported and without supports: the tub back down, the top piece
and the two decks face down. FDM holes come out small: drill the 1.2 mm pinholes to size.

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

It goes together upside down (2026-10-21, `case/SHELL.md`): everything is laid into the top piece
face down, and the tub closes over it.

1. **The board, face up.** Push the cell's plug up through the board's hole (5.45 x 4.4 mm, in
   front of J2's mouth, between seat 1's clusters at the screen's edge: the whole plug passes), lay
   it flat, wires towards the hole, and push it into J2: its wires run straight from its tail down
   through the hole, with no fold. The speaker's plug goes up through the same-sized hole at seat
   2's end into J4 the same way (its red lead is +, nearest J4's pin 1; the amplifier is
   bridge-tied, so a reversed speaker only flips its phase). The cell's plug can't reach J4.
2. Turn the board over. Stick the cell on its back at its silkscreen outline with the 0.45 mm foam
   pad, its wires at seat 1's end, and the speaker by its back with thin double-sided tape at its
   outline beside the left edge, face (and foam ring) away from the board, its leads towards the
   middle. Peel the speaker ring's liner later, just before the tub goes on.
3. **The top piece**, face down on a cloth. Stick the tape strips round the panel's pocket (leave
   the liner on).
4. **The decks:** lower each into its end of the top piece from behind, face down. It drops onto
   its wedges along the step and round the spine and sits flat.
5. **Caps** into the decks from behind. Each ABXY cap's flange tab drops into a slot and ribs fill
   the notches in its flange, so each cap fits only its own hole, one way round: turn the caps to
   match. Seen from that seat's player (from the front): A's tab (right cap) points down-left, B's
   (bottom) up-left, X's (top) down-right, Y's (left) up-right; from behind it is mirrored, so go by
   the slots. Then the D-pad and Start/Select, then the **silicone** over them: the walls on the
   deck's back locate each pad.
6. **The panel:** peel the tape liner and lay the panel glass down into the pocket; the frame
   centres it. Hold the board on edge at seat 1's end, its face towards the panel, flip J3's latch
   up, slide the FPC's contacts into J3 (contacts down) until it stops, square, and press the latch
   shut. A cracked panel comes out the same way.
7. **The board**, face down onto the bosses (four on the top piece, one on each deck), the FPC
   folding under the panel at the seat-1 edge (don't crease it). The crush ribs on the side walls and
   the decks' end walls centre it; its tab under the USB-C sits in the notch.
8. Peel the speaker ring's liner. **The tub** over it all: its tongue slides inside the skirts all
   round, its tab rises into the notch under the USB-C, and the speaker's ring lands on its pad round
   the chamber.
9. **Six M2 x 8** from the back, up through the tub's bosses and the board into the top piece's and
   the decks' bosses, snug, not tight (they cut their own thread in the plastic). They clamp the
   board between the bosses and hold the case shut, as on a DS Lite.
10. Opening it again: the six screws out, lift the tub off, lift the board out (the cell and the
    speaker come with it).

The power button is seat 1's Start button: hold it for about a second to turn the device on.
