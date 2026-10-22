# The shell for the 2.4 in panel (2026-10-21), with the cell in the board (2026-10-22)

The case's redesign after Alex's notes on his last print (items 10, 13, 14 and 15) and the change
to the EastRising ER-TFT024IPS-3, then reworked for the board with the cell in a hole in it and the
speaker on its top (row 19, `CELLCUT_FOR_CASE.md`, which supersedes `BOARD_OUTLINE.md` where they
differ). Everything here is in `case.scad`; this file says why.

## In one paragraph

Four printed parts: a **top piece** (the screen face, the curved steps down to the decks, a spine
at each end, and the sides), two **decks** (one part,
printed twice: the button faces at each end), and the **tub** (the back, with a pocket for the
cell). The top piece and the decks all curve over the sides and meet the tub flush at one seam,
4.2 mm up the walls, where the back's round ends. Each deck seats into the top piece **from behind**
on a 45 degree wedge, reaches out to the case's outline and curves over the end and the sides
itself, and has its own screw. The board, with the cell in a hole in it, lies inside the tub's
walls. Everything is built **upside down**: the speaker into its pocket, caps and silicone into the
decks, the decks into the face-down top piece, the panel into its pocket, the board face down (the
speaker's plug into J4 first), the tub (the cell stuck in its pocket) over it, six screws from the
back. The speaker stays in the top piece, so J4 is unplugged before the board and the top piece come
apart. Every part prints without
supports in its own pose: the tub back down, the top piece and the decks face down.

| | cb51826 | 2026-10-21 | Now (2026-10-22) |
|---|---|---|---|
| Outside | 75.4 x 156.2 mm | 75.4 x 132.7 mm | **77.4 x 132.7 mm** (a DS Lite is 73.9 x 133.0) |
| Thickness | 13.4 at the screen, 11.5 at the decks | 13.35, 11.5 | **9.55 at the screen, 7.7 at the decks** |
| Board | 70.4 x 151.24 | 70.4 x 127.7 | 72.4 x 127.7 (r 5.5), a 54 x 37 hole for the cell |
| Cell | under the board | under the board | in the board's hole, its bottom in a 0.6 pocket in the tub's floor |
| Speaker | floor grille | floor chamber, side outlet | on the board's top in the right bezel, a grille in the face |
| Parts | tub, lid, two deck inserts | tub, top piece, two decks | the same |
| Held by | six screws, four snap hooks | six M2 x 8: four into the top piece, one into each deck | six M2 x 6, the same way (the side ones into the top piece beside the glass, since the side screws came back inward) |
| Crush ribs | on the lid | on the top piece's and the decks' walls | on the tub's walls (the board is inside them) |

![outside, from the front](images/outside_front.png)
![outside, from the back](images/outside_back.png)
![outside, from the side (USB-C)](images/outside_side.png)

(The images are rendered from the parts of this commit; see "Images" at the end.)

The side view has no board in it, so the USB-C opening shows the far wall's inside and reads as
a recess; it is a through-opening (checked 2026-10-22): J1's mouth (8.94 x 3.20, its face 0.19 under
the surface) sits centred in it, 0.25 clear in y and 0.29 / 0.25 in z, and a mated plug's overmold
(the shortest the USB-IF allows) stands 0.16 off the wall. case.scad asserts the opening against
the port and the plug, and check_fit.py checks it against J1's 3D model.

## The board with the cell in it (2026-10-22)

`CELLCUT_FOR_CASE.md` (final) has the board's side. What the case does with it:

- **2 mm wider.** The board is 72.4 (x +-36.2), so `case_w` is 77.4, 1 mm more on each side. The
  clusters, Start/Select and the debug pinholes stay where they were (`edge_margin_x` 5 is the
  case's side to a silicone piece, `edge_margin` 4 still its end), so `positions.json`'s buttons
  don't move. The USB-C opening, the receptacle's face (x 38.55) and the board's tab and its slot
  go 1 mm out with the right wall, with the stepped tab seams and the `usb_gap` opening as before.
- **The stack.** The worst-case cell (4.5) stands at most `cell_top` 1.6 over the board, so its
  bottom is 2.9 under the top copper, 1.3 under the board's back. The tub's floor is pocketed
  `cell_pocket` 0.6 under it (0.9 of floor left there), so the board rests on 0.7 ribs
  (`board_lift`): floor 1.5 + 0.7 + board 1.6 + 5.75 = **9.55 at the screen**, 7.7 at the decks
  (the notes' figure for the pocketed floor; 10.15 with a flat one).
- **The cell's pocket** is the hole's outline 0.25 inside (r 0.75 corners). The cell lies against
  its right wall (x 25.05), so TH1, at the hole's right edge, reads it across about 1 mm.
- **Floor ribs under the strips.** The ring of ribs under the board's edge and the 12 mm grid stay,
  clear of the hole by 0.5; new ribs run along each strip (the left one at x -31.2 under SW18, SW19
  and J5/J6, the right one at x 29.6 under the power latch) and across the right one at y 14 (by
  TH1). They keep off the four Start/Select domes' vents, J1's legs and the test pads on the back
  (`rib_free`, which check_fit reads).
- **The board is inside the tub's walls.** Its top (3.8) is under the seam (4.2), so the top pieces'
  skirts no longer reach it: the **crush ribs moved to the tub's walls** (sides at y +-28, ends at
  x +-22), their lead-in at the top, where the board meets them as the tub goes on. The tub's
  cavity follows the back's round only 0.2 in (`tub_round_in`), so the board's edge keeps 0.3 to
  it. The skirts' cut over the tongue is now only the wall's inner half: the decks' plates come
  down to 5.7, under the tongue's cut, and a cut across the whole inside took 0.1 off them and left
  the pads' walls hanging.
- **The side screws** are the board's, inside the top piece (Alex, 2026-10-22: no shoulders): H1
  and H2 at (+-32.5, 29), at the seat-2 ends of the strips beside the cell's hole; H3 at (25.9,
  -21.95) in the right bezel, between the speaker and J4's plug; H4 at (-33.0, -15.2) in the left
  bezel, between J2 and the LDO. The top piece's 5.0 bosses run from the board to the ceiling (the
  pilot stops 0.8 under the screen face). Each keeps 0.3 or more from the panel's frame (H3: 0.54),
  1.2 from the side wall's inside, 0.72 from the step's top edge and 0.3 from the joint's band where
  the decks' wedges start (H1, H2: y 31.5 against 31.8), and H3 0.52 from the speaker's pocket, 0.36
  from J4 and 0.30 from its mated plug (asserts). The **tub's bosses** under the board are only
  `board_lift` (0.7) tall; 6.0 wide at H1, H3 and the ends, **5.0 at H2 and H4** (`tub_boss_d`): a
  6.0 boss would come within 0.8 of the cell's hole at H2 and 0.2 of the board's edge at H4. The 5.0
  ones leave 1.3 to the hole at H2 and 0.7 inside the board's edge at H4, and 1.3 of plastic round
  the screw's 2.4 clearance hole (the head's 4.4 counterbore is in the floor under them, 1.0 from the
  tub's bottom chamfer at H4). The shoulders, the decks' notches and their checks went.
- **Shorter screws:** M2 x 6. An 8 would run 0.9 past the decks' pilots now; a 6's tip stops 0.1
  short of the decks' (which end 0.6 under the face) and 1.75 short of the top piece's.
- **The speaker** lies on the board's top in the right bezel (centre 28.7, -11.4 since the side
  screws came back inward, its seat-2 end 0.1 short of the cell's hole; 10 x 15 x 3.5, 3.8 worst
  case). The top piece's underside has a 0.5 pocket over it, its outline plus 0.5 (the
  board's keep-out), so 0.45 over a worst-case speaker; the face over it is 1.5. The **grille** is
  three 1.0 slots along the racetrack, 1.8 apart, inside the outline less 1.5 (over the cone):
  12, 10.8 and 10.8 long, about 33 mm2 open. The rim round them is solid, for the front foam ring
  to seal on. Face down they are plain holes in the plate on the bed: nothing to bridge
  (`overhangs.py`: no supports). The bezel next to the screen bay keeps its full 2.0 from the panel
  frame (x 22.86) to the pocket (x 23.2), 1.5 from there to the grille (x 26.4); the grille's slots run
  y -5.4 to -17.4. A 0.3 pocket over J4 (31.9, -27.0) and its mated plug, each grown 0.25, gives
  the plug room; it goes round H3's boss and stops at the joint's band. The floor chamber, the
  side outlet and the pad went.
- **The speaker's leads** (35 mm, to a 1.25 mm plug) run on the board from the speaker to the back
  of J4's mated plug (x 24.75, its mouth faces the glass), round H3's boss, which stands between
  them. With the leads leaving the speaker's seat-1 end (turn it so they do), the shortest path that
  keeps a 0.8 lead off the boss, the speaker and J4 is 14.0 mm (1.5 straight out of the plug, then
  up the 0.65 gap between H3's boss and U7's end, partly over U7's end under the panel frame, 1.0 of
  room there, and over the boss's top, 1.4 below the speaker). If they can't ride over U7, round U7
  under the panel it is 27.4. Plugging in takes about 10 more (the plug's 4.0 travel out of the
  mouth, and a few mm to hold it): 24 or 37 mm against 35. Leads leaving a long side need 17.1
  (left) to 25.0 (right), the far end 28.1, each over U7's end. So J4 is plugged with the board just
  off its place: with the leads in, J4 can be at most about 20 mm from where it lands.
- **J2** (the cell's plug, 3.4) at the hole's seat-1 corner and its mated plug (3.2, over the left
  strip) sit under the left bezel's 3.75; **J5/J6** (2.9) face the left edge. Their plugs go in
  with the tub off; no slot in the wall (debug is case-open, and a slot weakens the wall and lets
  light in).
- **Start/Select (row 4)** close on 4.0 mm metal snap domes (0.31 high at worst with the Kapton
  dot): `ss_metal_dome`, `ss_plunger_gap` and two asserts: the dome fits the pad's well (4.6 >=
  4.3), and the pad's plunger rests 0.05 over the dome's apex. The plunger's rest height in a real
  strip is unmeasured; 0.36 (the least that clears) is assumed, so `ss_lift` stays 0.35. If a strip
  measures g < 0.36, raise `ss_lift` by 0.36 - g: at most 0.15 with today's lip, up to 0.45 with a
  0.3 Start/Select chamfer. The model's pads now show each metal dome and the plunger over it.
- **The spines** keep their length; seat 1's covers U6 (1.75, over the decks' 1.6) and seat 2's only
  U9 (1.2), so they could be shorter (not done: the decks and positions.json stay as they are).

![across the cell](images/section_cell.png)
![across the speaker and its grille](images/section_speaker.png)
![along the speaker and J4](images/section_speaker_along.png)
![across H2: the slimmed tub boss by the cell's hole](images/section_h2.png)
![across H4: the slimmed tub boss by the board's edge](images/section_h4.png)
![along H3: between the speaker and J4](images/section_h3.png)

## The smooth rounds (2026-10-22)

The top piece's and the decks' rounds and the step were stacks of flat 0.1 mm slabs, as the tub's
was until 4df3f96. They are now solids lofted through the profile every 0.05: the decks' outline as
the tub's; the top piece as boxes lofted up the step (the screen block, the spines) and the side
walls swept along y under the step's height, so the side round still follows the step down to the
deck. The chamfers are exactly `overhang`: every downward face is
45.0000 degrees from vertical or steeper (166 mm2 of the top piece's and 73 mm2 of a deck's at
45). Sliced on the P1S (0.2 mm, Bambu Studio's command line), the outer wall now lies on the true
surface at every layer (0.000 mm off) and steps out 0.2, 0.2, 0.199, 0.18, 0.15, 0.126 ... up each
round; the slabs had sat up to 0.05 off it on the decks' round and 0.025 on the step. (Here the
slabs' faces happened to fall 0.01 off the P1S's planes, so they printed without the tub's 0.4
risers; the lofts no longer depend on that.) The loft also runs its sides' edges the other way
round from its caps': 4df3f96's ran them the same way, which OpenSCAD's Manifold backend rejected
("PolySet -> Manifold conversion failed") and quietly repaired.

## The length and the width

The end bands are as they were (the clusters, Start/Select and the end screw sit where they sat
relative to the case's end): `band` = `edge_margin` 4 + `piece_d` 26 + `piece_to_glass` 5 = 35 mm a
side, so `case_l` is 132.7. The width is the board's 72.4 plus the walls and their clearance, 77.4,
2 mm more than the clusters set (`edge_margin_x` 5 at the sides); `glass_side` follows, 16.6 mm
from the bay to each long edge: room for USB-C and the speaker on the right, the core and the debug
parts on the left.

## Item 10: the top pieces curve over the sides and meet the tub flush

- **The seam** (`z_seam` = `back_r` = 4.2) is where the tub's back round ends and the walls go
  straight up. Below it is the tub: the floor, the round (4.2, ending in a 45 degree chamfer at the
  bed) and nothing else. Above it the top piece (in the middle) and the decks (at the ends) form all
  of the walls and roll into their faces through `face_r` = 3.0: an arc out of the wall that turns
  into a chamfer at `overhang` where the arc would go flatter in the print pose (the same profile
  as the back's round, `edge_in()`).
- **The joint at the seam:** the tub's wall carries a **tongue**, its inner half 1.5 mm up past the
  seam (`tongue_h`); the top pieces' skirts come down outside it, their outer half to the seam. The
  outsides are flush; a skirt can't move inwards; there is no straight path through the seam for
  light or dust; and it is the top pieces that wrap over the tub ("or go over it"), not the tub that
  forms a lip. The skirts stop `seam_gap` 0.1 short of the tub so the screws clamp the board, not
  the seam.
- **Width:** 77.4 since 2026-10-22 (the board's 72.4); the walls stay 2.0.
- **USB-C:** with the seam below the port, the opening's lower half is now a **tab of the tub** that
  rises from the seam into a notch in the top piece's skirt (the old lid tongue, upside down), with
  the stepped seams that stopped light showing through. The board's tab sits in a slot in that tab.
- **Snap hooks: dropped.** They held the lid on the tub while the case was turned over for the
  screws. Built upside down, nothing has to hold anything while the screws go in (gravity does), and
  the PLA hooks snapped on the first print. The six screws hold the case closed, as before.

## Item 13: the step down to the decks is a curve

The step is 1.85 mm tall (the screen face 5.75 over the board, the decks 3.90). It belongs to the
top piece, which prints face down, so every point of it faces down in the print pose, and nowhere
may it slope flatter than `overhang` (45 degrees, a guess until the `6_tests` ladder). Under that rule a
straight 45 degree slope is already the gentlest possible line; a curve can only be steeper
somewhere. So the step is the same round as every other outside edge, which reads as one shape
(the screen block's edges all roll over alike): `step_r` = 3.0, an arc that reaches the deck steep
(22 degrees off vertical) and turns, near the screen face, into a 45 degree chamfer where it would
otherwise go flatter. Its run is 1.53 mm plus a 0.6 mm flat at the deck's level (`joint_land`, so the
top piece's edge at the joint is square, not a knife edge). The seam with the deck lies in the
crease at its foot.

When Alex's ladder says a printer can do, say, 60 degrees, `overhang = 60` lets the arc run much
further up before the chamfer starts (and makes every other round rounder too); nothing else
changes. A concave cove at the deck's edge, the softest shape for a thumb, is impossible in any
face-down print: it would be a downward face at the deck's height with nothing under it.

## Item 14: no more cards slotted in from the front

The inserts dropped in from the front between the tub's walls and tucked under a lip on the tub's
end wall; they wobbled at the sides and the lip felt fragile. Now each deck:

- **reaches the case's outline** and curves over the end and the sides itself, down to the seam,
  where its skirt sits on the tub's tongue like the top piece's: supported all round its outer edge
  by the tub, no lip;
- **seats into the top piece from behind:** along the step and round the spine its edge goes 0.8
  straight down from the face (`deck_land`) and then out 1.2 at 45 degrees under a matching undercut
  in the top piece (`wedge_w`, `deck_gap` clearance, 0.25 until `6_tests`). It drops in from behind, centres itself, and can't
  come out of the front. The wedges on the spine's two sides and its end oppose the ones along the
  main step, so pushed home it can't move in x or y either;
- **has its own screw:** the end screw now goes through the board into a boss on the deck (the end
  bosses moved off the spine into the deck, just past the spine's end), pulling it down onto the
  board and its wedges up against the top piece. No anchor ring round a column any more, so the
  0.15 ring that was too tight is gone with it.

One part fits both ends (the end bosses are each other turned round, as before).

## Item 15: built upside down

1. **Top piece face down** on a clean cloth. **Speaker** face down into its pocket over the grille,
   its front foam ring stuck to the pocket's ceiling (the pocket centres it; standing on the board,
   0.45 to 0.75 under the ceiling, its front would be open to its back).
2. **Decks:** each face down into its end of the top piece from behind; it drops onto its wedges
   and sits flat.
3. **Caps** into the decks from behind (A/B/X/Y tab into its slot, notches onto their ribs; the
   D-pad; Start/Select), then the **silicone** on them: the walls on the deck's back locate each pad.
   The Start/Select metal domes and their Kapton dots are on the board already.
4. **Panel** glass down into the top piece's pocket on its tape; the frame centres it.
5. **The cell** into the tub's floor pocket on a strip of double-sided tape, against the pocket's
   right wall (towards TH1).
6. **Board on edge** at seat 1's end, face towards the panel: the FPC into J3 and its latch, the
   cell's plug into J2 (the tub held beside it).
7. **Board** face down towards the bosses (four in the top piece, one on each deck), the FPC
   folding under the panel. Just before it lands, the **speaker's plug into J4** (its mouth faces the
   glass; the leads reach about 20 mm, so J4 must be near its place), the leads laid round H3's
   boss on the panel's side; then the board down onto the bosses.
8. **Tub** over it all: the cell drops into the board's hole, the crush ribs on the tub's walls
   centre the board, the tongue slides inside the skirts, the tab into the USB-C notch.
9. **Six M2 x 6** from the back, snug.

Opening it is the same backwards: screws out, lift the tub off (the cell comes with it, on its
leads), lift the board only a little and **unplug J4 first**: the speaker stays stuck in the top
piece (Alex's choice), on 35 mm leads. Then the board comes away, and everything else lies where it
was. The same goes for anyone opening it face up: unplug J4 before the top piece comes off.

## The other parts of the design

- **How the screen is located and held:** as before, its top in a pocket in the top piece's
  underside under a 0.8 skin, centred by a 1.1 frame (0.4 a side), held by double-sided tape. The
  window is the panel's visual area (38.72 x 50.96, the active area plus 1.0). The panel's back is
  2.4 over the board (J3 under it), 0.8 over a worst-case cell.
- **J3's whole range:** the folded FPC's tip lands 22.5 to 25 mm in from the seat-1 glass edge
  (`tip_range`, an estimate), so J3's origin is 20.3 to 22.8 in. The case keeps the FPC's whole
  footprint, out to 25.5 in, and J3's courtyard over the whole range clear (`fpc_zones`, `j3_zone`).
- **How the board is located:** crush ribs on the tub's side and end walls, 0.05 short of the
  board, with a lead-in at their upper end; the screws line it up. It is clamped between the tub's
  bosses and the top pieces' bosses, and rests on the tub's floor ribs.
- **Screws:** side bosses at (+-32.5, 29), (25.9, -21.95) and (-33.0, -15.2) in the top piece;
  end bosses at (+-3.0, +-50.75), on the decks.
- **Debug pinholes:** BOOTSEL and RESET at (-31.2, 8.6 and 1.6); J5 and J6 at y 15.2 and 22.7.

## Print orientation and supports

| Part | Pose | Supports (`overhangs.py`) |
|---|---|---|
| tub | back down, as exported | none |
| top piece | face down | none |
| deck (print two) | face down | none |
| board mock, USB-C board coupon | as before | yes, as before (the USB-C stand-in) |
| speaker dummy | flat | none since 2026-10-22 (a plain racetrack, face up) |

![section across seat 1](images/section_seat.png)
![section along the D-pad's line](images/section_length_dpad.png)
![section across the screen](images/section_screen.png)
![the step and the deck's joint](images/section_joint_zoom.png)
![the seam](images/section_seam_zoom.png)

## Row 9: the tub's bottom edge (2026-10-21)

Alex saw a vertical lip on the outside of the tub, all round its bottom edge, on the print of the
10-20 kit. That tub's mesh has none (its only one was the speaker pad, row 6) and neither kit added
a brim or support round it. But its round was built in flat 0.1 steps whose faces lay exactly on
the P1S's slicing planes, and Bambu Studio took the step above or below as it liked: the chamfer
printed 0.35, 0.1, 0.3, 0.1 out per layer, pairs of layers standing straight at the bed. The
round is now one smooth solid lofted through the outline every 0.1 (`tub_outside()`), so each
layer steps out 0.2 on the P1S and the U1, from the wall into the back within `overhang`. If the
next print still shows a lip, it is the first layer's squish: the kits keep Bambu's and Alex's
0.15 elephant-foot compensation.

## Best guesses until `6_tests`

Alex won't print `6_tests` soon, so these are committed as best guesses (2026-10-21). Each is marked
"GUESS" in `case.scad`; when the plate is printed, check each against its coupon and move it.

| Parameter | Guess | Checked on `6_tests` by | Which way to move it |
|---|---|---|---|
| `cap_hole_d` | 8.0 | `tune_openings`: holes 7.6 / 7.8 / 8.2 beside the keyed 8.0 cluster | the smallest hole a real A/B/X/Y cap drops through and presses without catching; smaller if 8.0 rattles |
| `dpad_gap` | 0.65 | `tune_openings`: D-pad openings 0.55 / 0.65 / 0.75 | the tightest opening the real D-pad rocks freely in every direction |
| `overhang` | 45 | `test_overhangs`: fins 45 to 75 degrees from vertical, a convex R6 round (on the P1S and the U1) | up to the steepest fin that prints clean on both printers; every round and the step to the decks get rounder with it |
| `slide_fit` (`deck_gap`, `seam_fit`) | 0.25 a side | `test_rings`: rings 0.2 / 0.3 / 0.4 / 0.5 a side over the 5.0 `test_column` | the smallest ring that slides on the column by hand without force or rocking (0.15 was too tight on the old insert's ring) |

`slide_fit` is the one parameter for a printed part sliding on a printed part: the tub's tongue
inside the skirts (`seam_fit`) and each deck's wedge in the top piece's undercut (`deck_gap`) take
it. Raising it from 0.15 changed no position the board reads (`positions.json` is the same): the
deck's wedge now stops 0.25 short of the undercut (it reaches 0.95 under the top piece, was 1.05),
the top piece's wall behind the undercut is as thick as before (`step_wall_t` 1.35, which used to be
written 1.2 + `deck_gap`), and the tub's tongue is 0.75 thick (was 0.85).

## Alternatives considered

- **Decks as wells in the top piece** (the top piece's rim at screen height all round each end,
  the deck dropped into it from behind): the stiffest, every screw in the top piece, no seam round
  the decks' outer edges. Rejected: a 1.9 mm bumper round the thumbs' area, higher than the caps'
  tops, and Alex asked for "the top pieces" to curve over the sides.
- **A one-piece top with the decks in it:** no deck seams, but face down the decks print 1.85 mm
  off the bed (supports, the peel-away face the inserts were made to avoid); face up its insides are
  all ceilings. Rejected.
- **Printing the top piece face up** (the outside all top surfaces, a smooth curve for free):
  the bezels' undersides become 13 mm ceilings. Rejected.
- **The deck carrying the curve's foot:** face down, anything of the deck above its face is below
  the bed. Not possible.
- **No spine** (a straight step across, a simpler rectangular deck): since 2026-10-22 J2 and J4 are
  in the bezels and the spines only cover U6 and U9, so it is the board's call now: U6 (1.75) under
  seat 1's spine would have to go under the screen or drop to 1.6.
- **The side screws' bosses on the decks** (2026-10-22): the simplest, but the top piece would have
  no screw, and the decks' wedges only hold it from moving in. **A tab of the top piece at the
  deck's height** round each screw: a 5.6 mm ledge 1.85 off the bed, face down. So: shoulders,
  until the board brought the side screws back inside the top piece (2026-10-22) and they went.
- **A D-shaped or wall-merged tub boss at H2 and H4** (2026-10-22): a round 5.0 boss already keeps
  1.3 from the cell's hole and 0.7 inside the board's edge, and it is only 0.7 tall; a D or a boss
  run into the wall would gain nothing there.
- **A slot in the left wall for the debug cables** (J5, J6): not done, debug is case-open.
- **Keeping the hooks:** nothing needs them when the case is built upside down, and they snapped in
  PLA.
- **The seam higher** (at the port's centre, no USB-C tab): then the tub forms most of the walls and
  the board isn't inside the top piece's skirt when it goes in. **Seam at the board:** the same
  tab and a thin tub wall above the round. The round's end is the natural line.
- **The tub's skin over the top's tongue** (the old way round): the tub would form the outer lip,
  which item 10 rules out.

## Open questions

1. **The overhang limit** (`overhang`, 45, a guess): the `6_tests` ladder on both printers. A higher limit
   makes the step and every edge rounder in one line.
2. **The deck's wedge and its clearance** (`deck_gap` = `slide_fit` 0.25, a guess; `wedge_w` 1.2): the `0_buttons_end`
   coupons (top piece end + deck) show whether a deck drops in, centres and sits without rocking, and
   whether the seam in the step's crease looks right.
3. **The seam** (`seam_fit` = `slide_fit` 0.25, a guess; `seam_gap` 0.1, `tongue_h` 1.5): the seam coupons.
4. **J3's range** (`tip_range` 22.5 to 25) is an estimate; a real panel's folded FPC settles it.
5. **The speaker in its pocket:** stuck by its front ring to the pocket's ceiling, its back stands
   0.45 (worst case) to 0.75 off the board. If Alex would rather tape it to the board (as the board
   session drew it), its front then needs a foam ring 0.5 to 0.8 thick to seal on the grille's rim.
6. **J4 and the speaker's leads:** the speaker stays in the top piece, so J4 must be plugged with
   the board just off its place (the leads reach about 20 mm past the closed path) and unplugged
   before the parts come apart. Worth trying on the fit kit with a real speaker: whether the leads
   lie flat round H3's boss and over U7's end, and whether the plug can be reached with the FPC in.
   The speaker's lead exit is unknown here (assumed at its seat-1 end).
7. The screw columns: no column passes through another printed part any more, so the `6_tests`
   rings only matter if one ever does again.

## Images

Rendered from the parts of this commit (`case/images/`; the session scratchpad has the sources):
the outside views with OpenSCAD from the assembled STLs, the sections with matplotlib from
`trimesh` cuts of the same parts (the cell nominal, the speaker 3.5 tall).
