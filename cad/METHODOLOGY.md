# CAD Iteration Methodology (OpenSCAD)

How Claude iterates on `board.scad` with a real visual + assertion feedback
loop, so most design errors are caught without a test print.

Toolchain: OpenSCAD, code-based. Model is `cad/board.scad` (text, edited
directly). Requires `openscad` installed (`sudo apt install openscad`).

---

## The loop

1. **Edit** `board.scad` (parametric — named vars at top, no magic numbers).
2. **Render a fixed contact sheet** of standard views, headless:
   ```bash
   openscad -o /tmp/mb_iso.png      --camera=0,0,0,60,0,135,520 --imgsize=1200,1200 board.scad
   openscad -o /tmp/mb_top.png      --camera=0,0,0,0,0,0,420     --imgsize=1200,1200 -D 'part="faceplate"' board.scad
   openscad -o /tmp/mb_tub.png      --camera=0,0,0,0,0,0,420     --imgsize=1200,1200 -D 'part="tub"' board.scad
   openscad -o /tmp/mb_xsec_w.png   --camera=0,0,0,90,0,0,420    --imgsize=1200,1200 -D 'cut="width"' board.scad
   openscad -o /tmp/mb_xsec_l.png   --camera=0,0,0,90,0,90,520   --imgsize=1200,1200 -D 'cut="length"' board.scad
   ```
3. **Read every PNG** — actually look. Check: cutouts land on their part,
   nothing intersects, walls present, pockets hollow, parts mate.
4. **Read the build log** — `echo()` lines and any `assert()` failure.
5. **Fix, repeat.** Change one parameter group at a time; re-render; compare.

The cross-section views (`cut=`) are mandatory each iteration — internal
pockets, wall thickness, and clearances are invisible from outside.

---

## Self-checking harness (build into `board.scad`)

Catch errors numerically so a bad parameter aborts the render loudly:

- **`echo()` every derived dimension**: final board W×L×H, wall thickness,
  each pocket size, clearance gaps. These print to the build log; read them.
- **`assert()` invariants**, e.g.:
  - `assert(led_bar_w + 2*side_margin <= board_w)`
  - button rows do not overlap the OLED window
  - screw bosses do not intersect the battery bay or button posts
  - `assert(board_l <= bed_l && board_w <= bed_w)` — fits the printer bed
  - interior depth ≥ tallest component + clearance
- **STL sanity** after export: bounding box (fits bed?), manifold check,
  "needs supports?" reasoning from geometry (no steep overhangs / bridges).

Self-test convention: a `-D 'check=true'` mode that only runs the asserts +
echoes (fast, no geometry) for a quick gate before full renders.

---

## What this catches vs. what needs the user

| Claude verifies solo | Only a test print verifies |
|----------------------|----------------------------|
| Shape, placement, clearances, dimensions | Real module physically slots in & holds |
| Fits print bed, manifold, support-free | Tolerance feel (too tight / rattles) |
| Internal pockets via cross-section renders | Button-press comfort for two people |
| Parametric assertions hold | User's printer warping/bridging |
| Datasheet dims modeled correctly | Whether bought part matches datasheet |

Residual user dependency is **physical reality**, not visual correctness.
Mitigation: pin exact part numbers before modeling (model real mm), and
print the cheap **faceplate first** to validate button pitch + OLED fit
before committing filament to the full tub.

---

## Workflow order

1. Pin exact part numbers → real datasheet mm into parameter block.
2. Write/iterate `board.scad` with the loop above until renders + asserts
   are clean.
3. Export `top-faceplate.stl` + `bottom-tub.stl` via CLI.
4. User test-prints faceplate only; reports fit; tune parameters.
5. Full print; tune; done.

Every dimension is a named variable at the top of `board.scad` so the whole
case re-derives when one part changes — never hardcode a measurement inline.
