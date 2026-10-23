# The island's coupons: 45 against 60 degrees

One small plate to choose between the two versions of the island case: the 60 degree one (the main
line, `ref-design-island`) and the 45 degree one (`ref-design-island-45`). They are the same design;
only the steepest downward face each assumes the printer prints clean differs, and with it every
round, the island's edge and the laps between the arms. Each piece here is cut from the real part at
that angle, in the pose the real part prints in (the island and the decks face down), with a tab on
one of its cut ends carrying its angle in raised figures: **45** or **60**. Nothing needs supports.

<!-- P1S -->

`P1S/island_coupons.3mf` is sliced for the Bambu Lab P1S (open and send); `U1/island_coupons.3mf`
is a Snapmaker Orca project for the U1 (open, slice). `stl/` has every piece. Print it on the printer
(or both) that will print the case, at 0.20 mm, as the case kits are.

## The pieces (each twice: 45 and 60)

| Piece | What it is | Pairs with |
|---|---|---|
| `corner_island` | seat 1's right corner of the island (the screen block): its edge profile round the corner, the wedge under its foot, its inside's corner | `corner_deck` |
| `corner_deck` | seat 1's deck round that corner: an L, the right arm beside the island and the core's lip across the step | `corner_island` |
| `meet_seat1` | seat 1's left arm's end, with its lap (the tongue that goes behind the other arm's skirt) | `meet_seat2` |
| `meet_seat2` | seat 2's left arm's end, the skirt the lap goes behind | `meet_seat1` |
| `ear_island` | the seat-1 end of the island's ear (over the USB-C), with its lap, and the island's side beside it | `ear_arm` |
| `ear_arm` | seat 1's right arm's end, which the ear's lap goes behind | `ear_island` |

Only pair a 45 piece with a 45 piece and a 60 with a 60.

## What differs

| | 45 | 60 |
|---|---|---|
| The island's top edge (the step's profile, all round it) | a 45 degree bevel 0.88 tall and 0.88 wide from the screen face, then the r 3.0 arc down to the 0.35 straight foot: 1.36 from the foot to the flat face | a 60 degree bevel only 0.40 tall (0.70 wide), then the same arc: 1.79 from the foot to the flat face |
| The arms' and the ear's outside round (r 3.0) | a 0.88 x 0.88 bevel at the face, the arc down the wall; the face's flat ends 1.76 in from the outline | a 0.40 tall bevel, the arc down the wall; the face's flat ends 2.20 in |
| The laps (where the left arms meet, at the ear's ends) | 1.9 long behind the other skirt (2.05 past its own end), their underside rising at 45 | 3.0 long (3.15), their underside at 60 |
| The island's wedge under the decks' lips | 45 (the same) | 45 (the same) |

## Putting them together (on the bench, in your hand)

The case is built face down: the decks first, then the island into them from behind. The pieces go
together the same way, and since a deck's face stands 1.85 above the island's when they are face
down, hold the deck's piece in your fingers rather than on the bench.
1. **The corner.** `corner_deck` face down (as it printed) in your fingers by its arm. Drop
   `corner_island` into the corner from behind, face down, straight down: its wedge settles onto the
   deck's lip along both edges and round the corner. Turn the pair over and look at it face up.
2. **The left arms.** `meet_seat2` face down on the bench, then `meet_seat1` face down beside it,
   straight down, its lap going down behind `meet_seat2`'s skirt, the two arms' ends 0.15 apart.
3. **The ear.** `ear_arm` face down in your fingers, then `ear_island` face down into it from behind,
   straight down: the ear's lap goes behind the arm's skirt. Turn it over.

The tabs are on the far ends; they don't get in the way.

## What to look for

- **Does each 60 piece print as clean as its 45 twin?** Face down, the bevels at the face are
  what leaves the bed, and the laps' undersides overhang. Look at the island's top edge round the
  corner, the arms' outside round near the face, and the underside of each lap: drooping, loose
  strands or a rough band at the 60 pieces' edges that the 45 pieces don't have means the printer
  doesn't print 60 clean; take the 45 case.
- **Do they go together the same way?** Each pair should drop together without force: the corner's
  island seats on the deck's lip with an even 0.15 line at its foot and doesn't rock; the left arms lie
  flush outside, the lap behind the skirt, no light through the joint; the ear's end sits flush with
  the arm. A 60 lap that droops can catch on the other skirt; a 60 bevel that sags can make the
  island's foot line uneven.
- **Which looks right?** Face up, side by side: the 60 corner is rounder (a short bevel and a long
  arc, the island's flat face 0.44 narrower each side); the 45 one has a visible bevel along its top
  edge and on the arms' outside.

If both print clean and go together, it is a choice of look: the 60 line is the rounder one. If the 60
pieces don't print clean, the 45 case prints on any printer.
