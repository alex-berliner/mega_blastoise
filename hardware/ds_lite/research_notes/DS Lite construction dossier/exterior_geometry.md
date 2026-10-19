# DS Lite (USG-001) exterior geometry, bottom half focus

Scope: Nintendo DS Lite, model USG-001 only. The original DS (NTR-001), DSi and DSi XL are named only where a source mixes them in.

Most numbers here come from one place: a 3D scan of a third-party (aftermarket) DS Lite shell. Wesk posted it on BitBuilt, and this research pass measured it. Read the frame and label notes under Question 1 before using any coordinate.

## 1. Official dimensions and weight; bottom half vs top half thickness; is the bottom half uniform?

### Takeaway
Nintendo gives 133.0 (W) x 73.9 (D) x 21.5 (closed thickness) mm and about 218 g. Nintendo publishes no thickness for either half.

From the scan, the bottom half is about 13.5 mm thick at its rim and the top half about 7.9 mm. The sum, 21.4 mm, is within 0.1 mm of the official figure. The bottom half is not uniform:
- the frame around the touchscreen rises about 1.8 to 2.3 mm above the button "wings";
- the hinge strip between the knuckles sits about 1.5 mm lower than the wings;
- the left hinge knuckle rises to about 21.2 mm, nearly the full closed thickness.

### Cited Findings

#### Frame and labels used in this whole document
- **Frame S (scan frame):**
  - +X points right as seen from the front with the face up. +Y points away from the user, toward the hinge. +Z points out of the face; Z = 0 is the lowest point of the back face.
  - X/Y origin: the centre of the back shell's plan bounding box. In Frame S the outer walls are:
    - left/right: x = -66.4 / +66.4 at mid-height (+/-66.0 near z = 4, +/-66.4 at the lip top);
    - front: y = -36.5 at mid-height, -36.8 at the lip top;
    - back wall of the hinge section: y = +37.05.
- **Labels:**
  - "measured (scan)": measured in this pass from Wesk's scan of an aftermarket shell. Scan resolution is about 0.1 mm. Aftermarket parts may differ slightly from Nintendo OEM parts.
  - "derived": arithmetic shown.
  - "official": Nintendo.
  - "estimated": judgement.
- **Scan used:** "DS Lite Scan (3rd Party)" by Wesk, BitBuilt staff, posted 2024-02-13 ([BitBuilt post](https://bitbuilt.net/forums/posts/65514/)).
  - Five meshes, about 2.5 M triangles each, in mm (the scale matches Nintendo's outline to within 0.3 %). The archive names them Bottom Cover (back shell), Button Cover (face plate), LCD Cover, Lid Cover and Battery Cover.
  - Download: [DS Lite.rar](https://BitBuilt.net/wesk/3D%20Scans/DS%20Lite.rar). The post says "provided as reference only" and states no licence.
- **Method:**
  - Each mesh was aligned with its largest flat face on Z, then rotated in plane to the minimum-area rectangle.
  - The face plate needed a 180 degree turn about Z to put the D-pad on the left. The back shell needed a flip (180 degrees about Y).
  - The flip was confirmed by matching an asymmetric screw hole at (+51, +22) in the back shell to a boss on the face plate.

#### Official and press figures
- **Official size:** 縦 73.9 x 横 133.0 x 厚さ 21.5 mm (closed), about 218 g including the battery and stylus. Label: official ([Nintendo JP DS Lite spec page](https://www.nintendo.co.jp/ds/spec/)).
- **Same page:**
  - stylus 87.5 mm long, 4.9 mm diameter (official);
  - screen 256 x 192 dots at 0.24 mm dot pitch (official).
  - It also lists the original DS (NTR-001) in a separate column. Do not mix the two up.
- **Wikipedia:**
  - 133 x 73.9 x 21.5 mm and 218 g for the DS Lite;
  - 148.7 x 84.7 x 28.9 mm and 275 g for the original DS NTR-001;
  - no inline citation for the dimensions ([Wikipedia, Nintendo DS Lite](https://en.wikipedia.org/wiki/Nintendo_DS_Lite)).
- **Conflicting press reports:**
  - HEXUS reportedly gives 215 g and swaps the depth and height labels ("73.9mm high and 21.5mm deep"). Source: search-result summary, page not fetched ([HEXUS](https://HEXUS.net/qaem4)).
  - SlashGear gives 73.99 mm, probably a typo for 73.9. Source: search-result summary ([SlashGear](https://www.slashgear.com/nintendo-ds-lite-the-light-way-of-gaming-13252)).
  - dimensions.com agrees with 133 mm, 21.5 mm and 218 g. Source: search-result summary ([dimensions.com](https://www.dimensions.com/element/nintendo-ds-lite)).
- **Thinner than the original:** the DS Lite is 21.5 mm closed vs 28.9 mm for the original DS ([Nintendo JP spec page](https://www.nintendo.co.jp/ds/spec/)).

#### Measured thicknesses (scan; source: [BitBuilt scan](https://bitbuilt.net/forums/posts/65514/))
- **Bottom half at the rim:**
  - The back-shell lip top is at z = 13.48 to 13.71 at five sections (sides, front, both ends).
  - The back face under those points is at z = 0.07 to 0.24, so the local rim thickness is 13.3 to 13.6 mm (measured).
- **Face plate "wings"** (around the D-pad and ABXY): top surface at z = 13.1 to 13.55. That is 0.1 to 0.3 mm below the back-shell lip top (measured, after the stacking below).
- **Raised screen frame:** top at z = 15.06 (left and right of the window), 15.17 (front of the window) and 15.63 (hinge side of the window). That is 1.8 to 2.3 mm above the wings (measured).
- **Hinge strip** between the knuckles (where the top half's barrel lies): top at z = 11.7 to 12.0 (measured).
- **Left hinge knuckle:** top at z = 21.16 to 21.23 (measured).
- **Top half, at the side edges:** about 7.9 mm.
  - The lid's outer face is 8.15 above its rim at y = 0.
  - The LCD cover's outer rim sits flush with the lid's lip.
  - The lid's outer face is flat to within 0.45 mm across the part (z max 7.80 to 8.25 on a 20 mm grid).
- **How the two bottom-half shells stack:**
  - The face plate's flange (underside z = 7.38 to 7.47 in its own frame) sits on the ledge inside the back shell's wall (z = 12.1 to 12.36).
  - That gives a Z offset of 12.2 - 7.43 = +4.77 mm.
  - The face plate edge then sits just inside the 0.6 to 0.8 mm back-shell lip.
  - In plan, the face plate also needs +0.8 mm in Y relative to the back shell's bbox centre. Two checks agree: the front lip fit (face plate edge at y = -36.8 must move to the lip's inner face at -36.0) and screw boss to hole offsets (+0.79 and +1.02 mm). All face plate coordinates below include this +0.8 mm shift (derived).

### Inferences
- Bottom half 13.5 + top half 7.9 = 21.4 mm, against the official 21.5 mm. That supports the stacking above to within about 0.1 to 0.2 mm (derived).
- The bottom half is about 63 % of the closed thickness (13.5 / 21.5, derived).
- The two halves nest in relief. The top half's inner face is concave: its rim stands about 2.1 mm proud of its centre. The LCD cover's outer surface is at about 2.35 to 2.5 in the lid frame vs 0.27 at its rim (measured).
  - A NintendoWorldReport commenter independently described the top half as "raised around the edges, making it concave" ([NWR](https://nintendoworldreport.com/news/11047/ds-lite-details)).
  - The bottom half's raised screen frame (1.8 to 2.3 mm) therefore drops into the top half's recess when closed. That is why the closed thickness equals rim + rim, not rim + rim + frame.
- The knuckle top at about 21.2 mm sits roughly flush with the closed lid top at about 21.4 mm (derived).
- For an enclosure that should "feel like" a DS Lite bottom half, the governing numbers are:
  - 13.5 mm body;
  - about 15.2 to 15.6 mm over the raised screen frame;
  - 21.2 mm only at the hinge knuckle.

### Gaps
- No official per-half thickness was found from Nintendo or any teardown.
- The scan is of an aftermarket shell. The face plate wings vary by 0.3 to 0.45 mm front to back on the right side but not on the left. That could be warp, scan error, or a real slope; a genuine DS Lite should be measured with calipers to settle it.

## 2. Plan-view outline of the bottom half: width, depth, corner radii, and relation to the top half

### Takeaway
The bottom half is a 132.8 to 132.9 x 73.6 to 73.9 mm rounded rectangle:
- straight sides, straight front and back;
- front corners with a plan radius of about 4.3 to 4.4 mm;
- the back edge carries a 92.4 mm wide hinge section in the middle, with a 20.4 x 9.4 mm notch at each back corner where L and R sit.

Closed, the top half is flush with it in width to within about 0.1 mm.

### Cited Findings
All entries are measured (scan) from the [BitBuilt scan](https://bitbuilt.net/forums/posts/65514/) unless marked.

- **Back shell width:**
  - x = -66.30 to -66.40 (left) and +66.20 to +66.50 (right) at mid-height, so 132.6 to 132.9 mm;
  - +/-66.6 at the lip top (133.2 mm);
  - official 133.0 ([Nintendo JP](https://www.nintendo.co.jp/ds/spec/)).
- **Back shell depth:**
  - front wall y = -36.5 (mid-height) to -36.8 (lip top);
  - back wall of the hinge section y = +37.04 to +37.10;
  - so 73.6 to 73.9 mm; official 73.9.
- **Sides are straight in plan:** across y = -30 to +26 the x extent varies by less than 0.2 mm.
- **Front corners in plan:**
  - front-left r = 4.38 mm and front-right r = 4.42 mm (fit rms 0.06 mm) in the z = 1.5 to 4 slab;
  - front-left r = 4.29 mm (rms 0.08) in the z = 5 to 9 slab;
  - the fit failed near the lip (z 10 to 13), probably because of noise in the scan.
- **Hinge section of the back shell:** x = -46.1 to +46.3 (92.4 mm wide), running back to y = +37.05. Its back wall top is at z = 9.5 to 9.9.
- **L and R corner notches:**
  - the back shell is absent for |x| = 46.2 to 66.6 (20.4 mm) from y = +27.7 to the back (about 9.4 mm);
  - at mid-height (z 5 to 9) the pocket runs further forward, to y = +23.6 to +25.4.
- **Face plate outline:** top-face perimeter x = -65.8 to +65.9 (131.7 mm), inset about 0.5 to 0.8 mm inside the back-shell lip on each side. The back corners are cut by the knuckle (left) and by the separate LED hinge piece (right).
- **Top half (lid) outline:**
  - x = -66.43 to +66.49 (about 132.9 mm), flush with the bottom half's 132.8 to 133.2 mm to within about 0.1 mm;
  - the lid's free-end corners fit r = 3.84 to 4.01 mm in plan (rms 0.11 to 0.17).
- **Shoulder buttons:** "do not protrude from the system casing as much as on the original DS ... cannot be seen when looking directly at the front of the system" ([NWR](https://nintendoworldreport.com/news/11047/ds-lite-details)).

### Inferences
- Plan corner r (about 4.3 to 4.4) and the section edge radius (about 4.2, see Question 3) are nearly equal. The front corners are therefore close to a uniform 4.2 to 4.4 mm blend in every direction. Modelling them as a rounded box with r = 4.3 at the back face is a good approximation (estimated).
- The top half's free-end corner (about 3.9 to 4.0 mm) is slightly tighter than the bottom half's (about 4.3 to 4.4 mm). It could be a real difference or aftermarket/scan noise; with one scan it is unresolved (estimated).

### Gaps
- The corner radius right at the lip (the top of the bottom half) could not be fitted reliably.
- No OEM drawing was found to confirm any plan radius.

## 3. Edge profile in section, parting line, and how the two bottom-half shells meet

### Takeaway
**Back shell (the tub):**
- flat back;
- about 4.2 mm radius blend into a nearly vertical side wall;
- the wall flares outward about 0.35 to 0.4 mm on the way up (about 2 degrees of draft);
- a thin 0.6 to 0.8 mm lip at the top.

**Face plate:** drops inside the lip and rests on a ledge about 1.5 mm below the lip top. Its top is 0.1 to 0.3 mm below the lip, with a small (about 0.5 mm) rounded edge.

**The only visible seam on the outside** runs around the top face, 0.6 to 0.8 mm in from the outer edge. There is no seam on the side walls of the bottom half.

### Cited Findings
All entries are measured (scan) from the [BitBuilt scan](https://bitbuilt.net/forums/posts/65514/).

- **Lower edge (back face to wall) radius:**
  - 4.23 mm at the right side (y = -22, fit rms 0.008 mm);
  - 4.18 mm at the front (x = +56, rms 0.016 mm).
  - Fits at other sections were spoiled by nearby features: the battery recess on the left, the power switch slot at y = +10.
- **Side wall draft:**
  - the outer wall is at x = 66.0 at z = 4.0 to 6.6, 66.2 at z = 6.8 to 10.6, and 66.4 at z = 10.7 to 13.7 (right side, y = -22);
  - so it flares about 0.4 mm over about 9.7 mm, roughly 2.4 degrees, wider at the top;
  - the front wall shows the same, the arc ending at y = -36.3 and the wall top at -36.6.
- **Wall and plate thickness:**
  - back-shell side wall about 1.5 mm (x 64.9 to 66.4 at mid-height);
  - back plate 1.27 mm (z 0.17 to 1.44 at y = -22).
- **Back shell lip and ledge:**
  - side (y = -22): lip x = 65.7 to 66.4 (0.6 to 0.8 mm thick); internal ledge at z = 12.17 to 12.29 (x 64.8 to 65.6); lip top z = 13.35 to 13.71;
  - front (x = +56): lip y = -36.8 to -36.0; ledge z = 12.10 to 12.36; lip top 13.40 to 13.56.
- **Face plate edge:**
  - plate 1.33 mm thick in the wings (z 7.29 to 8.62 in its own frame);
  - flange about 1.5 mm wide (x 64.4 to 65.9) resting on the ledge;
  - outer top edge rounded from x = 65.8 at z = 12.84 to 65.17 at z = 13.34 (assembled), roughly r = 0.5 to 0.6 mm (estimated from the points);
  - a downward skirt 1.3 mm wide sits 2.3 to 3 mm in from the outer edge and goes down 3.8 to 4.4 mm (inner locating rib, not visible outside).
- **Previous project note:** the face plate is "1.4-1.5 mm" and is "nested in a stepped wall". Source: from the same Wesk scan via an earlier pass (`/home/alex/Code/mega_blastoise_dslite/hardware/ref_design/case/DSLITE.md`). This pass measures 1.33 mm in the wing at y = -22 and about 1.0 to 1.2 mm at the window edge.
- **Top half edge, for context:**
  - lid top surface 8.15 at x = 61, falling to about 7.0 at x = 64.6, about 5.4 at x = 65.8, and down the wall at x = 66.1 to 66.4;
  - so a large top-edge round of about 4 to 5 mm (estimated from section points);
  - lid lip and ledge mirror the bottom half: ledge at about 2.35, lip to about 0.25 (lid frame).
- **Assembly:**
  - the shells close with 7 screws from the back (iFixit list in Question 5);
  - "Insert a spudger edge between the lower and upper case near the top right corner... continue along the back";
  - "Some units are sealed with adhesive"
  - ([iFixit Lower Case Replacement](https://www.ifixit.com/Guide/Nintendo+DS+Lite+Lower+Case+Replacement/4636)). On iFixit, "lower case" means the back shell and "upper case" the face plate.

### Inferences
- **Section model for OpenSCAD** (estimated, from the measurements above):
  - flat back at z = 0;
  - R4.2 blend into the wall;
  - wall from x = 66.0 (z = 4.2) to 66.4 (z = 13.7);
  - lip top at 13.6 to 13.7 with a small (about 0.3 mm) round;
  - face top at 13.3 to 13.5, set in by 0.7 mm, with an R0.5 edge.
- The bottom half has no draft-angle seam or visible chamfer on its sides. The silhouette is the back shell alone, so a 3D-printed two-part case can copy the scheme exactly: a tub with a thin lip, and a lid nested inside.

### Gaps
- There is no section through a genuine OEM shell; draft and lip numbers may differ slightly from an aftermarket mould.
- The lip-top radius and the face plate's edge radius are under 1 mm and close to the scan's resolution.

## 4. Layout of the bottom half's top (inner) face

### Takeaway
All coordinates are Frame S (mm), with the face plate shifted +0.8 in Y. All entries are measured (scan) from the [BitBuilt scan](https://bitbuilt.net/forums/posts/65514/) unless marked.

- **Touchscreen window:** 69.1 x 53.7 mm with about 1 mm corner radii, centred at (0.05, -1.6), inside a raised frame.
- **D-pad cross opening:** 18.75 x 18.80 mm, centred at (-51.36, +3.92), 15.0 mm in from the left wall.
- **ABXY:** four 7.5 mm holes in a diamond centred at (+51.10, +6.67), 15.3 mm in from the right wall. The hole pitch is 15.35 mm, which conflicts with the 15.0 mm contact pitch in other sources.
- **Start/Select:** two 3.5 x 3.6 mm holes below the ABXY.
- **LED windows:** two small slits at the right end of the hinge strip.

### Cited Findings

#### Window and frame
| Feature | Frame S coordinates (mm) | Size (mm) | From edges (mm) |
|---|---|---|---|
| Touchscreen window (narrowest opening) | x -34.5 to +34.6, y -28.5 to +25.2; centroid (0.05, -1.58) | 69.1 x 53.7; corner r 0.86 to 1.02 (fits rms 0.02 to 0.06) | 31.8 to 31.9 from each side wall; 8.0 to 8.3 from the front wall; 11.85 from the back wall |
| Window lip at the opening | z 9.42 to 10.41 own frame (front), 9.25 to 10.29 (side) | lip about 1.0 mm thick | |
| Raised frame, side ramps | rises from the wing level at x = +/-40 to the top at x = +/-35.2 | about 1.7 mm rise over about 4.8 mm | |
| Raised frame, front ramp | rises from y = -33.2 to y = -29.0 (both surfaces of the plate rise, so it is a formed step, not a thickening) | about 1.8 mm over about 4.2 mm | |
| Raised frame, hinge side | top at z = 15.6 for y = +25 to +27, then drops about 3.5 mm to the hinge strip (z about 12.0) by y = +31 | | |

- **Derived from the official 0.24 mm dot pitch** ([Nintendo JP](https://www.nintendo.co.jp/ds/spec/)):
  - active area 256 x 0.24 = 61.44 mm by 192 x 0.24 = 46.08 mm;
  - diagonal 76.8 mm, which is 3.02 in.
  - The window is therefore the active area plus (69.1 - 61.44)/2 = 3.8 mm horizontally and (53.7 - 46.08)/2 = 3.8 mm vertically on each side.

#### Buttons and other features
| Feature | Centre, Frame S (mm) | Size (mm) | From edges (mm) |
|---|---|---|---|
| D-pad cross opening | (-51.36, +3.92) | 18.75 (x) by 18.80 (y) overall; arm width 6.60 to 6.80 | 15.0 from the left wall; 40.4 to 40.7 from the front wall |
| A (right) | (+58.77, +6.66) | dia 7.49 | 7.6 from the right wall |
| B (bottom) | (+51.09, -1.01) | dia 7.48 | |
| X (top) | (+51.12, +14.35) | dia 7.48 | |
| Y (left) | (+43.42, +6.67) | dia 7.50 | |
| ABXY diamond centre | (+51.10, +6.67) | Y to A 15.35, B to X 15.36 | 15.3 from the right wall; 43.2 to 43.5 from the front wall |
| Upper small round button (Start, label assignment not verified) | (+41.51, -18.06) | 3.45 x 3.60 | 24.9 from the right wall; 18.4 to 18.7 from the front wall |
| Lower small round button (Select, label assignment not verified) | (+41.50, -25.77) | 3.50 x 3.60 | 10.7 to 11.0 from the front wall |
| LED slit 1 (in the hinge strip, right end) | (+57.24, +31.97) | 0.8 x 2.0 | |
| LED slit 2 | (+59.51, +32.73) | 0.9 x 5.3 | |

- **D-pad centre to ABXY centre:** 102.46 mm across and 2.75 mm vertically, ABXY higher (derived).
- **Earlier pass, same Wesk scan:** "face holes are 7.6 mm for A/B/X/Y and 18.9 mm across the D-pad's cross" (`/home/alex/Code/mega_blastoise_dslite/hardware/ref_design/case/DSLITE.md`). This pass measures 7.48 to 7.50 mm and 18.75 to 18.80 mm at the narrowest through-opening.
- **Conflict on ABXY pitch:**
  - 15.000 mm (Ampersand DS Lite membrane breakout gerbers) and 15.011 x 15.001 mm (facelesstech clicky buttons project), both recorded in `/home/alex/Code/mega_blastoise_dslite/hardware/membrane/README.md`;
  - 15.35 mm between hole centres in this aftermarket face plate (measured, scan);
  - the scan's overall scale matches Nintendo's outline to within 0.3 %, so this is not a scan-scale error.
- **D-pad cap fit:** cap arms 5.8 to 5.9 mm (DSLITE.md) against a 6.6 to 6.8 mm opening, so about 0.4 mm clearance per side (derived).
- **Start/Select relocated:** Wikipedia's caption says Start and Select were "relocated" on the Lite compared with the original DS ([Wikipedia](https://en.wikipedia.org/wiki/Nintendo_DS_Lite)).

### Inferences
- With a 7.5 mm hole and a 7.0 mm cap, a 0.35 mm pitch error puts each cap only about 0.18 mm off-centre. An aftermarket shell with 15.35 mm holes would still work over a 15.0 mm membrane. For the enclosure, 15.0 is the safer figure for the contacts; the 15.35 hole pitch is an aftermarket-shell fact (estimated).
- The touchscreen top surface probably sits about 1.0 mm below the frame top, under the 1.0 mm lip: z about 14.1 to 14.6, which is 0.7 to 1.0 mm above the wings (estimated).
- **Visual layout, front view:**
  - left wing: the D-pad alone;
  - centre: the raised window frame;
  - right wing: the ABXY diamond with Start and Select stacked 7.7 mm apart below it, both on x = +41.5. That is directly under the Y button's column, 9.6 mm left of the diamond centre (derived).

### Gaps
- **Microphone hole:** not located. No through-hole over 1 mm² was found on the face plate's hinge strip; the scan may not resolve it.
- **Start/Select labels:** which hole is Start and which is Select was not confirmed from a source.
- **LED hinge piece:** the LEDs' exact exterior windows are in the separate "LED hinge" part (Question 5), which was not scanned.

## 5. Side, front and back edge features

### Takeaway
- **Front edge, left to right:** volume slider slot, then the 62.6 mm wide Slot-2 (GBA) opening, then the headphone/mic jack.
- **Right side:** the stylus entry (a 6 mm arched notch about 31 mm from the front) and, further back, the 12.2 mm power-switch slot.
- **Left side:** plain.
- **Back edge, in the 92.4 mm hinge section:** the 34.2 mm Slot-1 opening in the middle, the AC adapter port to its left (as seen from the front), and two wrist-strap holes to its right.
- **L/R:** fill 20.4 mm wide notches at both back corners.
- **Hinge knuckles:** the bottom half carries a 16.6 mm long, about 9.2 mm deep knuckle on the left, and a separate "LED hinge" knuckle on the right. The top half's 100 mm barrel sits between them.

### Cited Findings
All coordinates are Frame S (mm); the z values are assembled heights above the back face.

#### Front edge
- **Volume slot:**
  - x -51.8 to -39.55 (12.25 wide), z 5.65 to 8.9 (3.25 tall); centre 20.7 mm from the left wall. Measured (scan).
  - Wikipedia's caption places the GBA slot, the volume slider and the headphone/microphone jack on the front ([Wikipedia](https://en.wikipedia.org/wiki/Nintendo_DS_Lite)).
- **Slot-2 (GBA) opening:**
  - x -31.4 to +31.2 (62.6 wide); open from the back plate (z about 1.7) up to z = 9.6 to 9.8; above it a 3.8 to 4.0 mm band of front wall up to the lip. Measured (scan).
  - Under the slot, the back plate is cut back to y = -33.7 (it reaches -36.2 elsewhere at that height).
  - A shallow finger notch, x about -12 to +12, reaches y = -29.27 at x = 0: 4.45 mm deep, an arc of radius about (12² + 4.45²)/(2 x 4.45) = 18.4 mm (derived). Measured (scan).
- **Slot-2 dust cover:**
  - "The GBA cartridge slot does indeed have a removable cover to keep out dust ... can be removed with your thumb" ([NWR](https://nintendoworldreport.com/news/11047/ds-lite-details));
  - "You remove a hollow cap first... you can put this cap back into place" ([Techgage](https://techgage.com/article/nintendo_ds_lite/2/)).
  - A GBA game protrudes "about 1 cm" (NWR commenter quoting a Japanese site, low confidence) ([NWR](https://nintendoworldreport.com/news/11047/ds-lite-details)).
- **Headphone/microphone jack:**
  - round opening dia 4.82 mm, centred x = +48.56, z = 6.81: 17.8 mm from the right wall. Measured (scan).
  - It is part of a combined opening x +39.9 to +51.0, z 4.5 to 9.3 (the left part rectangular, x 40 to 47, z 4.75 to 8.2). Measured (scan).
  - Techgage: "To the bottom right of the unit is a spot for headphones and an external microphone" ([Techgage](https://techgage.com/article/nintendo_ds_lite/2/)). The official spec lists one ステレオヘッドホン・マイク接続端子 (combined jack) ([Nintendo JP](https://www.nintendo.co.jp/ds/spec/)).

#### Right side
- **Power switch slot:** y +6.2 to +18.4 (12.2 long), z 6.3 to 9.5 (3.2 tall). Measured (scan).
  - "The power switch is located on the right side of the system, above the stylus"; "a spring-loaded slider switch" ([NWR](https://nintendoworldreport.com/news/11047/ds-lite-details)).
  - Wikipedia lists a "Power flick-switch" ([Wikipedia](https://en.wikipedia.org/wiki/Nintendo_DS_Lite)).
- **Stylus entry:**
  - an arched notch in the right wall, y -8.5 to -2.5 (6.0 wide at the base, 5.0 wide at z = 5.3), arch top at z = 7.3. Measured (scan).
  - It is open to the back: a 10.2 x 5.4 mm notch into the back face (x +54.4 to +64.6, y -8.1 to -2.7). Measured (scan).
  - "The right side of the unit houses the stylus, which is not visable until you pull it out" ([Techgage](https://techgage.com/article/nintendo_ds_lite/2/)).
  - The stylus has "a rounded back end, which makes it fit flush with the system casing when stored" ([NWR](https://nintendoworldreport.com/news/11047/ds-lite-details)).
  - Stylus 87.5 mm long, 4.9 mm diameter ([Nintendo JP](https://www.nintendo.co.jp/ds/spec/)).
  - Wikipedia: "1 cm longer and 2 mm thicker" than the original DS stylus ([Wikipedia](https://en.wikipedia.org/wiki/Nintendo_DS_Lite)).
- **Back-shell floor along the probable stylus lane:**
  - y -11 to -2 is flat (z 2.0 to 2.2 inside) from x = -20 to +40;
  - ribs at y about -12.5 (z 3.8 to 4.0) and y about -1 (z 5.1 at x = -20). Measured (scan).

#### Left side and back edge
- **Left side:** no openings in the left wall. Measured (scan).
- **Slot-1 (DS card):**
  - opening in the back wall x -17.1 to +17.1 (34.2 wide), z about 2.0 to 7.4. Measured (scan).
  - The bottom lip (z 1.6 to 2.5) is present only near the ends; a finger notch in the back plate reaches y = +32.07 at x = 0, from the edge at about +34.95 (2.9 mm deep). Measured (scan).
  - "The slot on the top is for your DS games" ([Techgage](https://techgage.com/article/nintendo_ds_lite/2/)).
- **AC adapter port:**
  - notch in the back wall x -31.0 to -22.3 (8.7 wide), z 5.9 up to the wall top at 9.8. Its top is closed by the face plate's hinge strip, whose underside is at about z 8.7. Measured (scan).
  - "the connector beside it is for the power adapter" ([Techgage](https://techgage.com/article/nintendo_ds_lite/2/)).
  - Its external port "appears to be identical to that of the Game Boy Micro" ([NWR](https://nintendoworldreport.com/news/11047/ds-lite-details)).
- **Wrist-strap holes:**
  - two windows in the back wall at x +22.4 to +25.0 and +28.3 to +31.0 (about 2.6 x 2.1 mm each), z 5.6 to 7.8. Measured (scan).
  - "The notches to the left of the game cartridge spot is for a wrist strap" (left as seen from behind) ([Techgage](https://techgage.com/article/nintendo_ds_lite/2/)).
  - Nintendo: thread the strap's loop through the attachment holes on the back ([Nintendo support, via search summary](https://en-americas-support.nintendo.com/app/answers/detail/a_id/4094/~/how-to-attach-the-wrist-strap)).

#### L/R and hinge
- **L and R pockets:**
  - |x| = 46.2 to 66.6 (20.4 wide), from the floor edge at y = +27.7 to the back at y = +37.1 (9.4 deep);
  - at mid-height the pocket runs forward to y about +23.7;
  - on the left it is open from z = 0 up to the knuckle's underside at z = 8.6. Measured (scan).
  - The buttons are "removed by pulling out the metal pegs that secure them" ([iFixit Shoulder Buttons guide, via search summary](https://www.ifixit.com/Guide/Nintendo+DS+Lite+Shoulder+Buttons+Replacement/122138)).
- **Left hinge knuckle (part of the face plate):**
  - x -66.6 to -50.0 (16.6 long); y +28.2 to +37.4; top at z = 21.2;
  - in section, a vertical-walled block whose top is an arc of r = 4.43 to 4.50, centred y = +32.8, z = 16.7 (fit rms 0.17 to 0.22);
  - its back face is vertical from z = 8.6 to 16.7;
  - the outer end (-X) is a closed rounded dome, about r 3.5 at the top corner as seen from behind (estimated from the render);
  - the inner end (+X) is an open bore that takes the top half's barrel;
  - about 1.2 mm wall. Measured (scan).
- **Top half barrel (part of the LCD cover):** x -50.0 to +50.0 (100 mm), centred on the console, with the barrel top band at y about +30.8 to +33.5 (LCD cover's own frame). Measured (scan).
- **Right knuckle:** this aftermarket face plate has no tall right knuckle. Its hinge strip ends at x about +55 to +66 in a low pocket with locating posts and the two LED slits (Question 4).
  - A BitBuilt user lists "the LED hinge" as a separately modelled DS Lite part ([BitBuilt thread](https://bitbuilt.net/forums/threads/looking-for-a-full-3d-model-of-a-nintendo-ds-lite-shell.6131/latest)).
- **Hinge strip (cradle)** between the knuckles: top at z = 11.7 to 12.0, under the top half's barrel. Measured (scan).

#### Screws, feet and battery cover
- **Screws:**
  - case: two 4.3 mm gold Phillips, one 3.9 mm black tri-point, one 3.3 mm silver Phillips, three 5.5 mm silver tri-point;
  - battery cover: one captive Phillips;
  - two rubber feet must be pried off first (they hide screws)
  - ([iFixit Lower Case Replacement](https://www.ifixit.com/Guide/Nintendo+DS+Lite+Lower+Case+Replacement/4636)).
  - Earlier pass: seven screws into face bosses, two through the board (`DSLITE.md`).
- **Holes through the back plate** (measured, scan):
  - (+34.92, +31.73) and (-34.89, +31.73), dia about 5.5 (counterbores, in the hinge section);
  - (+59.26, -29.21) dia 2.7;
  - (+51.16, +22.66) dia 2.3;
  - inside the battery recess: (-55.23, -29.48) dia 1.7, (-58.01, +17.58) dia 1.3, and (-47.64, -12.46) dia 3.7.
- **Face plate bosses** that match: (-34.80, +31.74) against hole (-34.89, +31.73); (+52.04, +22.44) against hole (+51.16, +22.66). Measured (scan).
- **Battery recess, outer face:** x -61.12 to -24.02 (37.1 mm) by y -31.62 to +23.52 (55.1 mm); its ledge is at z about 2.65 to 2.9. Measured (scan).
- **Battery cover:**
  - scan bounding box 57.15 x 37.04 x 3.03 mm (measured);
  - four small latch/tab slots along the recess's right and front edges at (-36.2, -29.3), (-32.9, -29.3), (-28.8, -24.8), (-28.3, -19.2), plus a 4.6 x 12 mm slot at x -28.7 to -24.1, y +3.1 to +15.1 (measured).
  - "the spot for a 5.2v rechargable battery" is on the back ([Techgage](https://techgage.com/article/nintendo_ds_lite/2/)).
  - 1000 mAh ([Wikipedia](https://en.wikipedia.org/wiki/Nintendo_DS_Lite)).

### Inferences
- **The stylus lies along the X axis,** entering on the right at y about -5.5. If its 87.5 mm length runs in from x = +66.4, its tip reaches x about -21.1. That clears the battery recess, which starts at x = -24.0 (derived).
  - A GBA cartridge inserted about 1 cm proud would end inside at y about -11.8. The 35 mm cart length is assumed, not sourced (estimated). That leaves the lane at y -8.5 to -2.5 free.
  - This placement is an inference from the openings and the floor lane. No source says it outright.
- **Right knuckle size:** with the 100 mm barrel centred and the left knuckle 16.6 mm long, the separate right "LED hinge" knuckle should span x about +50.0 to +66.6. By symmetry it is probably the same height and profile as the left one (derived plus estimated).
- **Rubber feet:** the two dia 5.5 counterbores at (+/-34.9, +31.7) are probably the rubber-foot seats over two screws (estimated).
- **Battery cover:** the cover is 2 mm longer than the recess opening (57.15 vs 55.1), consistent with tabs that tuck under the shell at each end. Its 3.0 mm thickness on a ledge about 2.7 to 2.9 mm deep suggests it sits flush with the back (estimated).

### Gaps
- **L and R buttons:** no dimensions found. The scan contains no L/R buttons, so only the pocket envelope is known (20.4 x 9.4 x about 8.6 mm). A parts listing searched lumps DS Lite with DSi and DSi XL L/R buttons ([eBay listing title](https://www.ebay.de/itm/357413566229)), so take aftermarket L/R part sizes with care.
- **LED hinge piece:** not scanned and no dimensions found.
- **Volume and power slider caps:** not scanned. How far they stand proud of the wall is unknown.
- **Slot-2 dust cover:** not scanned. Its exterior face is assumed flush with the front wall (estimated).
- **Screw assignment:** which back-plate hole takes which of iFixit's seven screws was not established.

## 6. Underside (back face)

### Takeaway
The back face is a single flat plane, flat within about 0.6 mm in this scan, with:
- a recessed 37.1 x 55.1 mm battery door on the left half (behind the D-pad);
- two shallow finger notches, at the Slot-2 and Slot-1 edges;
- the stylus notch at the right edge;
- two foot counterbores in the hinge section;
- a few screw holes.

There is no deliberate curvature in this aftermarket shell; whether the OEM back is also flat was not checked against a genuine unit.

### Cited Findings
- **Flatness:** back face height on a 20 mm grid ranges from z = 0.03 to 0.64, with the edges (0.03 to 0.2) lower than the centre (0.4 to 0.64). Measured (scan) from the [BitBuilt scan](https://bitbuilt.net/forums/posts/65514/).
- **Battery recess:** on the left half from the front's point of view, x -61.1 to -24.0. The flip of the back shell was confirmed by the asymmetric screw hole at (+51.2, +22.7) matching a face plate boss. Measured (scan).
- **Label:** iFixit says the model number USG-001 is on "the label on the back cover" ([iFixit device page](https://www.ifixit.com/Device/Nintendo_DS_Lite)).
- **Rubber feet:** two, pried off to reach screws ([iFixit Lower Case Replacement](https://www.ifixit.com/Guide/Nintendo+DS+Lite+Lower+Case+Replacement/4636)).
- **Notches, openings and holes:** see Question 5 for the Slot-2 finger notch (24 mm wide, 4.45 deep), the Slot-1 notch (about 2.9 deep), the stylus notch (10.2 x 5.4) and the hole list. Measured (scan).
- **Recessed panel:** a shallow panel in the back face spans x about -19 to +19 and y about -25 to -18 (about 38 x 7 mm). It is recessed about 0.25 to 0.3 mm: z 0.63 to 0.67 inside vs 0.32 to 0.45 around it at y = -21 and x = 0. Measured (scan) from the [BitBuilt scan](https://bitbuilt.net/forums/posts/65514/).

### Inferences
- The 0.4 to 0.6 mm rise in the middle is most likely warp in the aftermarket part or a scan artefact. The lid (top half outer) shows a similar 0.45 mm spread. The design intent is probably flat (estimated).
- The shallow recessed panel near the front of the back face (see Cited Findings) is probably the seat for the back label (estimated).

### Gaps
- Label recess: its edges were read only at 2 mm and 0.5 mm steps, so the outline is approximate.
- Rubber foot height and diameter: not found.

## 7. Dimensioned drawings, patents, models and source quality

### Takeaway
No Nintendo dimensioned drawing or service manual for the DS Lite exterior was found. The only measurable 3D source found is Wesk's scan of an aftermarket shell. It is good, and internally consistent with Nintendo's outline and closed thickness to within about 0.1 to 0.3 mm. Treat its sub-millimetre numbers as aftermarket-shell numbers.

The US design patent first suspected (D556759) is the Game Boy Micro, not the DS Lite.

### Cited Findings
- **Wesk's scan:**
  - five meshes (back shell, face plate, top-half inner and outer, battery cover);
  - about 2.5 M triangles each;
  - may contain "Blobs" where light could not reach;
  - "provided as reference only"
  - ([BitBuilt post](https://bitbuilt.net/forums/posts/65514/); archive at [DS Lite.rar](https://BitBuilt.net/wesk/3D%20Scans/DS%20Lite.rar), 296.7 MB, RAR5).
- **Scan consistency checks** (derived):
  - outline 132.8 to 133.2 x 73.6 to 73.9 mm vs official 133.0 x 73.9;
  - bottom half 13.5 + top half 7.9 = 21.4 mm vs official 21.5.
- **USPTO D556759** "Hand-held electronic game machine" (Nintendo, filed 2005-11-14, JP priority 2005-05-13, inventors Kawanobe and Sakiyama) ([Google Patents](https://patents.google.com/patent/USD556759); [PDF](https://patentimages.storage.googleapis.com/d7/22/31/5bf4ec9207fa93/USD556759.pdf)).
  - It shows a single-screen, non-folding device with the D-pad and two round buttons beside the screen, which is the Game Boy Micro. Search tools surfaced it for DS Lite queries; **it is not the DS Lite**.
  - Its relatives D540396 and D532829 are "Cover for hand-held electronic game machine" (faceplates) ([D540396](https://patents.google.com/patent/USD540396); [D532829](https://patents.google.com/patent/USD532829)).
- **Other 3D sources:**
  - A BitBuilt user reported finding only partial DS Lite models ("the buttons, the LED hinge, and in rare cases the bottom of the shell"). A thread titled "DS Lite Shell Fusion 360 Model" exists, contents not seen ([BitBuilt thread](https://bitbuilt.net/forums/threads/looking-for-a-full-3d-model-of-a-nintendo-ds-lite-shell.6131/latest)).
  - Models already used in the earlier cap study: facelesstech's DS Lite front plate (Printables 205921), Pi Tin (github.com/jackw01/pi-tin), jusbot caps (Printables 1178557), MacroSOS (Printables 986494) (`/home/alex/Code/mega_blastoise_dslite/hardware/ref_design/case/DSLITE.md`). None was re-measured for the exterior in this pass.
- **Conflations to watch:**
  - Wikipedia's spec table gives the DS Lite display as "83 mm (3.25 in)". The official 0.24 mm pitch at 256 x 192 gives 76.8 mm (3.02 in) (derived from [Nintendo JP](https://www.nintendo.co.jp/ds/spec/)). The 3.25 in figure is probably another model's (a DSi conflation is suspected but not verified) ([Wikipedia](https://en.wikipedia.org/wiki/Nintendo_DS_Lite)).
  - The Nintendo JP spec page lists the original DS (NTR-001) beside the DS Lite.
  - Aftermarket L/R button listings group DS Lite with DSi/DSi XL ([eBay listing title](https://www.ebay.de/itm/357413566229)).
  - iFixit's device page has no exterior dimensions ([iFixit](https://www.ifixit.com/Device/Nintendo_DS_Lite)).

### Inferences
- **To rebuild the measurements:**
  1. Download the archive into an empty directory and extract it with libarchive. The system 7z lacks the RAR5 codec.
  2. Align each mesh as described under Question 1.
  3. Stack the face plate on the back shell at (dx, dy, dz) = (0, +0.8, +4.77).
  - All numbers above came from plane sections, plan occupancy maps (0.05 to 0.1 mm grid) and least-squares circle fits on the aligned meshes. This pass's working scripts were in the session scratchpad and are not kept in the repo.
- **Highest-value check against a real DS Lite** (estimated): with calipers, measure
  - the rim thickness (13.5);
  - the window frame rise (about 2);
  - the ABXY hole pitch (15.35 vs 15.0);
  - the plan corner radius (4.3);
  - the lower edge radius (4.2).
  These five settle almost every aftermarket-vs-OEM question in this note.

### Gaps
- **DS Lite design registration:** the actual US design patent or JP 意匠登録 for the DS Lite was not identified. The JP registrations behind D556759 (2005-013667/8/9) belong to the Game Boy Micro family.
- **Service documentation:** no Nintendo service manual or dimensioned drawing was found.
- **OEM scan:** no scan or caliper study of a genuine OEM shell was found.
