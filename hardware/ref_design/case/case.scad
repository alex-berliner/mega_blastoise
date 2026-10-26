// Reference design case: the single source of truth for every position the
// board depends on (outline, contacts, screw holes, connectors, height limits).
//
//   part="layout"   coloured top view
//   part="board"    board outline with its cable slots, 2D (DXF for Edge.Cuts)
//   part="3d"       assembly; explode=<mm> lifts the top
//   part="tub", "top"   printable parts (STL), each in its print pose (the top face down, on support filament)
//   part="top_up"   the top face up, as it sits on the case (its skirts' tips on the bed)
//
// The shell (2026-10-21, case/SHELL.md; the one-piece top since 2026-10-25): a tub, the back, and one top, the whole face
// and sides. The top is a band at the screen's height across the case's full width, its two ends the DS Lite's lower
// screen frame in section (a sharp top edge and one concave arc down to the ends), and the ends beyond it at the buttons'
// height (the genuine caps stand 1.41 proud of them), all curving over the sides down to a seam low on the back's round
// where the tub meets it. It prints face down, the slopes and rounds on Bambu Support For PLA interface (the second
// filament), so its outside keeps to no overhang limit. Built upside down: caps and silicone into the top from behind, the
// speaker into its pocket, the panel, the board, the tub over it all, six screws from the back.
//
// Frame: origin at the case centre, +Y towards seat 2 (seat 1 at -Y), z = 0 at
// the bottom of the tub. KiCad frame in the echoes: centre (100,100), Y down.
// case/export.py writes the echo block's JSON line to positions.json for the board scripts.
//
// ESTIMATE marks a number that needs a real part and calipers before ordering.

part = "layout";
explode = 0;
$fn = 72;

// ---------------- bought parts ----------------
// EastRising ER-TFT035IPS-6 (ILI9488, 320 x 480 IPS, "No Touch"): the 3.5 in panel, back since 2026-10-26 (it was the 2.4 in
// ER-TFT024IPS-3 from 2026-10-21; the case was 23.5 mm shorter). Its numbers are the old 3.5 in design's (5d0ef99^), from the
// datasheet's outline drawing as the old reviewer read it (the datasheet is behind Cloudflare and the Wayback copy was down on
// 2026-10-26, so none of it is re-checked here): ESTIMATE until a real panel and calipers say
panel_w = 54.50;  panel_l = 83.00;  panel_t = 2.45;     // outline with the FPC folded; the drawing's side view says
                                                        // 2.35 +-0.1 (its table 2.3): the thickest
tape_t = 0.15;                                          // double-sided tape between the top piece's skin and the panel
panel_fit = 0.8;   // the top piece's locating frame is this much larger: the panel is +-0.2, FDM openings print small
frame_wall = 1.1;  frame_h = 1.0;                       // that frame's walls, hanging from the top piece
aa_w = 48.96;     aa_l = 73.44;                         // active area (the old design's; ESTIMATE for this exact part)
aa_to_top = 2.5;  aa_to_fpc = panel_l - aa_l - aa_to_top;   // active area to the glass edges (centred across)
aa_off = (aa_to_fpc - aa_to_top) / 2;                   // active area sits 2.28 mm towards seat 2
window_margin = 1.0;                                    // bezel opening past the active area: covers the panel's float
                                                        // in its frame, its active-area tolerance and a printed
                                                        // opening's error; at most 0.5 mm of the panel's black border shows
// Its FPC leaves the seat-1 edge, folds under the panel and plugs into a 50-pin ZIF connector (J3, Hirose FH12-50S, bottom
// contact). Seen from above, x from the panel's centre, lengths along the FPC from that edge: a 4.59 mm neck (35.6 wide,
// centre +3.6) wraps the edge, then a 14.41 mm body (47.9 wide, centre +2.5) lies under the panel, then an 11.7 mm tail (25.5
// wide, centred) ends in the 3.5 mm contact strip. ESTIMATE (the old design's, scaled from the drawing's views, not dimensioned)
fpc_neck = [3.6, 35.6, 4.59];   fpc_body = [2.5, 47.9, 14.41];   fpc_tail = [0, 25.5, 11.7];
fpc_x = fpc_tail[0];  fpc_w = fpc_neck[1];              // J3 and the tail are centred; the fold passes through the frame as wide as the neck
fpc_l = fpc_body[2] + fpc_tail[2];
// Folded, the tip lands somewhere in tip_range in from the seat-1 glass edge (the fold isn't drawn: the old design's 26.11, 1.2
// either way). J3 isn't fixed until a real panel is measured, so the board keeps the whole range clear
tip_range = [fpc_l - 1.2, fpc_l + 1.2];                 // ESTIMATE
tip_in = (tip_range[0] + tip_range[1]) / 2;
fpc_low = 1.05;                                         // parts this tall (over the board) may lie under the FPC's body: it lies on the glass's back
fpc_neck_keep = 5.1;                                    // nothing this far in from the glass edge: the neck wraps it (ESTIMATE, the old drawing's 4.59)
fpc_slack = 1.0;                                        // the connector 1 mm short of the tail's tip: a
                                                        // long FPC bows, a short one can't reach (ESTIMATE)
conn_rear = 1.2;  conn_front = 4.4;                     // FH12 footprint origin to its back and to its mouth
j3_w = 29.1;                                            // FH12-50S, across its pins (3D model)
// The panel's bay: the case and board layout are built around it, so a panel that
// fits the bay (with its tape and a 0.25 mm gap each side) moves nothing else.
bay_w = panel_w + 1.5;  bay_l = panel_l + 3.24;

// The cell sits in a hole in the board (row 19; Alex, 2026-10-21: the case was too thick with it
// under the board), in the board's plane, across the board between seat 2's deck and J3's range:
// its top at most cell_top over the board, its bottom in a pocket in the tub's floor. The hole is
// the board's (placement.py CUTOUTS, case/CELLCUT_FOR_CASE.md): 0.5 round a worst-case cell, 1.0
// more at its left end, where its leads come out and run to J2 at the hole's seat-1 corner. It is
// pushed to the hole's right end, where the charger's thermistor TH1 reads it through the gap.
// With the 3.5 in panel the hole lies under the glass's seat-2 half (2026-10-26): the folded FPC takes the seat-1 half
cell = [52, 36, 4.5];                                   // 403450, worst case: 50 +-2 by 34 +-2 by 4.0 +-0.5
cell_nom = [50, 34, 4.0];                               // and nominal (the dummies and pictures)
cell_hole = [[-27.0, 6.0], [27.0, 43.0]];  cell_hole_r = 1.0;   // the board's hole: corners, corner radius
cell_top = 1.6;                                         // its top over the top copper, worst case: the decks'
                                                        // band (deck_limit) allows 1.6 where its seat-2 edge reaches
cell_pocket = 0.6;                                      // the tub's floor pocketed under it: the case 0.6 thinner
cell_pocket_fit = 0.25;                                 // the pocket inside the hole's outline, each side
// The speaker (Alex, 2026-10-22): INGHAi's GSPK151035PN, a 15 x 10 mm racetrack 3.5 mm thick with
// its adhesive front foam ring, whose 35 mm leads end in a 1.25 mm 2-pin plug for J4. Since the 3.5 in panel
// (2026-10-26) it lies in a hole in the board like the cell, its back on a pocket in the tub's floor, face up: beside the
// 54.5 mm glass only 8 mm of board is left before the wall, and a speaker lying on the board stands 3.8 over it, 1.6
// over the glass's limit. Sunk, its top is spk_over over the board, so it can lie half under the glass; its cone fires up
// through a grille in the right ear's flat top (the ear: the raised block that runs out to the wall over J1, J4 and the
// speaker's grille). The board keeps parts out of the hole's outline plus spk_keep (SPEAKER in placement.py)
spk = [10, 15, 3.5];                                    // across (x), along (y, the racetrack's length), height
spk_h_max = 3.8;                                        // its top over its seat, worst case (tolerance and its tape)
spk_pos = [29.7, -24.5];                                // its centre and its cone's (placement.py SPEAKER)
spk_keep = 0.5;                                         // the board's hole is this much bigger than its outline
spk_pocket_floor = 0.6;                                 // the tub's floor pocketed under it, as under the cell
spk_hole_r = 1.0;                                       // the hole's corner radius, over the racetrack's: a rounded rectangle
grille_in = 1.5;                                        // the grille stays this far inside the speaker's outline:
                                                        // the rim round it is solid, for its front ring to seal on
grille_slot = [1.0, 1.4];                               // the grille's slots along the racetrack: width, pitch
grille_x = [28.6, 34.0];                                // and in x between these: outside the panel's pocket, on the ear's flat top
// J4 (the speaker's plug, PicoBlade like J2, 3.4 tall) in the right strip, its mouth along the strip towards the speaker: a
// PicoBlade is 7.65 across and 5.2 deep, and its mated plug stands 4 mm out from the mouth. With the mouth towards the
// glass (the old layout) the pair would be 9.2 long across a strip that is 8.5 wide beside the glass
j4_body = [[27.775, -10.8], [35.425, -5.6]];               // its model's box (centre 31.8, -7.1: CELLCUT_FOR_CASE.md)
j4_plug = [[28.3, -14.8], [34.9, -10.8]];                // the mated plug's (about), towards the speaker
j4_h = 3.4;

abxy_pitch = 15.0;  dpad_pitch = 14.0;                  // DS Lite (gerbers)
piece_d = 26;                                           // the layout's envelope for a silicone piece (Alex's pads,
                                                        // 2026-10-20: A/B/X/Y 26.5 across its lobes, D-pad 24;
                                                        // pad2d below has their real outlines)
// Genuine DS Lite caps, measured from open models of them and of shells built for them
// (jackw01/pi-tin, CERN-OHL-W; Printables 1178557, 205921, 986494): see case/DSLITE.md
cap_abxy_d = 7.0;  cap_flange_d = 8.8;  cap_h = 3.56;  // ABXY: body, flange, flange bottom to top
cap_tab_r = 5.6;  cap_tab_w = 2.0;                      // each flange's keying tab, out from the cap's centre
dpad_arm_l = 17.8;  dpad_arm_w = 5.9;  dpad_h = 3.34;   // D-pad cross
dpad_flange_r = 6.5;                                    // its flange fills the corners between the arms
flange_t = 0.55;                                        // both caps' flanges
// The openings, lips and the walls under them follow Alex's DS Lite faceplate (front.stl, measured
// 2026-10-21, case/DSLITE.md), except where a print has tuned a clearance
cap_hole_d = 8.0;  pocket_d = 9.2;                      // deck: hole (the faceplate's 8.00; was 7.6), flange pocket
                                                        // (faceplate 9.18). GUESS (Alex, 2026-10-21: best guess until
                                                        // 6_tests): the 8.0 hole; tune_openings (7.6/7.8/8.2) settles it
cap_chamfer = 0.6;                                      // 45 degree chamfer round each hole at the face (faceplate)
lip_t = 1.6;                                            // the face down to the A/B/X/Y flanges' tops (faceplate 1.60;
                                                        // Pi Tin 1.65; was 1.16)
dpad_gap = 0.60;          // D-pad opening past the cross each side: 0.5 (the faceplate's 18.80 x 6.90 opening) was
                          // too tight for Alex's AliExpress D-pad in the first print, 0.8 too loose in the next
                          // (Alex, 2026-10-21), so halfway: 0.65. 2026-10-26 (Alex: "tighten the wells on the dpad a very
                          // small amount"): 0.60, 0.05 tighter each side (tune: tune_openings, 0.55 / 0.65 / 0.75).
                          // GUESS (Alex, 2026-10-21: best guess until 6_tests)
dpad_chamfer = 0.8;       // 45 degree chamfer round the D-pad's opening (faceplate)
dpad_pocket_gap = 0.5;    // its flange's pocket past the flange (0.35 bound with the cross; the faceplate's is 0.25)
dpad_lift = 0.35;         // raises (or, negative, lowers) the D-pad's retaining lip against the A/B/X/Y lips: the
                          // D-pad sat at the wrong height in the first print (tune: tune_stack_dpad_*). The faceplate
                          // lifts it 0.4 (a 1.2 lip), which puts its flange 2.10 over the board, as here
// tab slot in the pocket, a wedge from the cap's centre, slot_off degrees counter-clockwise of the tab (seen from
// the front): it takes a tab at our 45 degrees from the flat or at the faceplate's 52 (Alex, 2026-10-21: the
// faceplate's slots sit 7 degrees off ours, as facelesstech's do), and the cap turns a few degrees at most
slot_r = 5.95;  slot_a = 35;  slot_off = 2.5;
// tab directions, degrees clockwise from the seat's right, seen from the front: a pinwheel,
// each 45 degrees clockwise from the flange's flat (which faces the diamond's centre)
abxy_tabs = [["A", [1, 0], 135], ["B", [0, -1], 225], ["X", [0, 1], 45], ["Y", [-1, 0], 315]];
// Notches in each flange's edge (same angle convention), from case/DSLITE.md: Alex's caps have them
// and the first print's lid had nothing to match. A rib in the flange's pocket fills each one, so
// every cap goes in only one way and only in its own hole. notch_keys = false drops the ribs if a
// real cap disagrees (check with tune_openings first)
abxy_notches = [["A", [225]], ["B", [0, 180]], ["X", [225]], ["Y", [90, 180]]];
notch_keys = true;
notch_w = 0.8;  notch_reach = 0.45;                     // rib width, and how far it reaches in over the flange
dome_h = 1.75;                                          // board to the flange's underside at rest: DS Lite shells
                                                        // stop the flanges 2.3 mm over the board (scan of an
                                                        // aftermarket shell, Game Boy Macro and Macro F CAD all
                                                        // agree, 2026-10-19); 1.2 left the silicone no room. The
                                                        // faceplate gives 1.70 if its walls stand on the 0.65 web


// ---------------- layout ----------------
case_w = 77.4;            // the board's 72.4 (2026-10-22, Alex: up to 2 mm wider for its strips beside the
                          // cell's hole) and the walls: 2 mm wider than the 75.4 the clusters set, the clusters
                          // where they were (edge_margin_x)
glass_side = (case_w - bay_w) / 2;                      // panel bay edge to long case edge (10.7: J1, J2 and J4 beside
                                                        // the glass, and the island's slope)
edge_margin = 4;          // case end to silicone piece
edge_margin_x = edge_margin + 1;                        // and case side to silicone piece, 1 more since the case
                                                        // grew 2 mm wider round the same clusters (2026-10-22)
piece_to_glass = 5.2;     // silicone piece to glass edge (5 until the 3.5 in panel's 2.45 thick glass lengthened the slope's run to 5.85)
wall = 2.0;  board_clear = 0.5;
corner_r = 4.3;           // plan corners: a DS Lite's 4.3 to 4.4 (8 felt round and toy-like, 2026-10-19)
board_r = 5.5;            // the board's own corners (its KiCad outline): no longer follow the case's
// The flattest a face may slope downwards in its print pose, in degrees from vertical, for the parts that print without
// supports: the tub (the back's round and chamfer) and the top's skirts (back_levels). 60 on this branch
// (ref-design-cellcut-60, 2026-10-23, a variant until the overhang test plates (overhang_kit.zip) show both printers print
// a 60 degree face clean; case/SHELL.md). overhangs.py reads it from here. The top itself prints on support filament and
// does not keep to it: its side rounds are true arcs and its slopes are the DS Lite's
overhang = 60;
// The tub's outside edge rounds over as far as `overhang` allows: an arc into the wall, and where the arc would go
// flatter than `overhang` in the print pose, a tangent chamfer at `overhang` (the tub prints back down; the top's skirts
// follow the same round down to the seam). The top's own rounds are true arcs: it prints on support filament
back_r = 4.2;             // the back rolls into the walls through back_r, as a DS Lite's does (4.2 mm)
face_r = 2.8;             // the top's faces roll into the walls (Alex, 2026-10-21, item 10: the top curves over the sides, the
                          // tub no longer forms the outer lip), as a true quarter arc since the one-piece top (2026-10-25);
                          // 2.8 since 2026-10-24 (Alex)
support_pitch = 12;       // the floor's support ribs under the board
// The seam: the top meets the tub flush, low on the back's round. The tub's wall carries a tongue, the inner half of the
// wall standing up past the seam, and the top's skirt comes down outside it (its outer half to the seam): they sit flush
// with the tub, can't move inwards, and the seam has no straight path into the case. Six screws lock it all (the snap
// hooks went, 2026-10-21: PLA hooks snapped). The tongue carries the joints (Alex, 2026-10-22: the parts "need to flow
// better and interlock" and must hold together before the screws): tabs (keys) into pockets in the top, and grip ribs
// (case/SHELL.md, "The joints")
// The seam came down onto the back's round (Alex, 2026-10-22, after the joints: the top's walls over most of the round,
// to a much lower ledge on the tub, the tub keeping its tongue): it was at the round's end (back_r, 4.2). The skirts
// follow the round down to it, their inner faces still straight down the tongue, so they slide straight on. 2.55 is as
// low as the skirts' tips stay skirt_t thick with the tongue where it is (the round comes 0.30 in at their tips, 2.65;
// at a 2.5 seam it would be 0.32 and the tips 0.68); beside the board's edge there is 0.25 to spare (asserted below)
z_seam = 2.55;
// Printed part sliding on printed part, each side: the tub's tongue inside the skirts, the tabs in their pockets.
// GUESS (Alex, 2026-10-21: best guess until 6_tests): 0.25; test_rings (0.2 to 0.5) settles it
slide_fit = 0.25;
seam_fit = slide_fit;                                   // the tongue's clearance to the skirts (and the tabs')
deck_tongue_top = 6.6;    // the tongue's top round the top's ends (6.7 until 2026-10-25, see core_lead), 1.1 under their face: there the
                          // tongue leans in as the face's round comes in, and its tip is chamfered. Along the side walls, under
                          // the band, it is lower: arm_tongue_top, below. It rises from the seam: 4.05 tall since 2026-10-25
                          // (4.15 since the seam came down; 2.5 over the seam at 4.2; 1.5 before "The joints")
deck_tongue_h = deck_tongue_top - z_seam;
skirt_t = 0.7;            // the skirts' outer half is never thinner than this, outside the tongue's groove
tongue_t = wall / 2 - seam_fit;                         // the tongue's thickness (0.75; its tip tapers)
tongue_lead = 0.3;        // the tip's outer edge chamfered 45 degrees, a lead-in into the skirts
core_lead = 0.45;         // the same chamfer on the ends' tongue (2026-10-25): the round's skirt over it is thin (wall under the top's
                          // side round, SHELL.md), so the tongue's corner gives it room; the tip stays 0.3 thick (tip_min 0.25)
seam_gap = 0.1;                                         // the skirts stop this short of the tub at the seam (and
                                                        // the grooves this far over the tongue's top), so the
                                                        // screws clamp the board, not the seam

// ---------------- stack (z) ----------------
floor_t = 1.5;  pcb_t = 1.6;  top_t = 2.0;               // top 2.0, not 1.5: the first print felt flimsy
spk_seat = floor_t - spk_pocket_floor;                  // the speaker's seat, z (0.9)
// the board sits this high over the tub's floor, on ribs: a worst-case cell in its hole, its top
// cell_top over the board, its bottom in the floor's pocket (0.7: the case is 9.55 at the screen)
board_lift = cell[2] - cell_top - pcb_t - cell_pocket;
glass_gap = 2.4;          // panel back above the board: J3 (2.0 mm) under the panel keeps 0.4 mm against
                          // the connector's and a printed top's tolerances (Alex, 2026-09-26)
// The panel's top sits in a pocket in the top piece's underside, under a skin face_t thick over its
// black border, so the screen face is only face_t above the glass instead of top_t (2026-10-19)
face_t = 0.8;
bay_t = glass_gap + panel_t + tape_t + face_t - top_t;  // board top to the top piece's underside (outside the panel)
// The FPC's lengths were fitted to a panel 2.4 mm above the board; a higher panel spends the
// extra drop on the way down to J3, so J3 moves that much closer to the edge (the safe side:
// a long FPC bows, a short one can't reach)
function conn_in_at(tip) = tip - fpc_slack - conn_rear - (glass_gap - 2.4);   // connector origin, in from the edge
conn_in = conn_in_at(tip_in);
z_board = floor_t + board_lift;                         // board bottom
z_top_cu = z_board + pcb_t;                             // board top: every height limit counts from here
z_glass = z_top_cu + glass_gap;
case_h = z_glass + panel_t + tape_t + face_t;
z_ceil = case_h - top_t;                                // the top piece's underside (the panel's pocket goes higher)
retainer_z = z_top_cu + dome_h + flange_t;              // the lip the flanges rest against
// Under the deck, as in a DS Lite shell (measured from a scan of an aftermarket shell and the Game
// Boy Macro faceplates, 2026-10-19): no solid block, just ribs that rest on each silicone pad's
// flat web and locate it. A/B/X/Y: a ring round each flange pocket (the rings merge) and a walled
// pocket for the pad's raised centre square; D-pad: a four-lobed ring round its pad (its cap's peg
// through the pad's centre hole does the rest); Start/Select: a square frame round each cap.
// Elsewhere the deck's underside clears the pads. The flange pockets run from the ribs' bottom up
// to the lip. Alex's pads (2026-10-20) have a 0.65 web and a rim 0.4 wide and 1.8 tall round
// their outer edge, as the DS Lite's shell expects: its clover wall (1.8 thick) stands inside the
// rim. So the A/B/X/Y rings stay inside the rim, the D-pad's ring stands just outside its pad,
// and the deck's underside lifts over each pad (rim_relief) so nothing presses on a rim
// The faceplate (2026-10-21) builds it the same way: its walls all end 3.2 under the face, which
// is on the web if the board's top is 3.85 under it. Its A/B/X/Y clover is one wall, round lobes
// joined by straight runs with 1.0 fillets between them; its D-pad wall is a round-ended lobe per
// arm, inside the rim too; its Start/Select frame is one rounded rectangle round two square cells
sil_rib_gap = 0.8;                                      // the ribs' bottom over the board: room for the web (the
                                                        // faceplate's stand on it: 0.65)
rim_relief = 0.15;                                      // the deck's underside over a pad: the rim's top, plus this
rim_clear = 0.1;                                        // every wall stays this far inside a pad's rim
retainer_bot = z_top_cu + sil_rib_gap;
pocket_h = retainer_z - retainer_bot;
abxy_ring = 5.25;                                       // the A/B/X/Y clover's lobe radius, round each hole: Alex's
                                                        // shell's clover is 10.45 across a lobe, inside his pad's
                                                        // rim (the faceplate's 5.525 would sit on it)
abxy_fillet = 1.0;                                      // the clover's fillets between the lobes (faceplate)
abxy_square = [5.15, 0.7, dome_h + flange_t];          // centre square's pocket: inside, wall, roof over the board
                                                        // (faceplate: 5.10, 0.70, at the flanges' lip: 2.25)
dpad_wall = [4.8, 6.8, 0.65];                           // D-pad wall, a lobe per arm: outer radius, its centre out
                                                        // from the cross's, thickness (faceplate 4.8, 6.8, 0.625),
                                                        // trimmed to stay inside the pad's rim
ss_frame_w = 0.6;  ss_frame_r = 1.0;                    // the Start/Select frame's wall and corners (faceplate)
ss_cell = 5.8;            // each square cell in it, round the pad's well (faceplate 5.70, here + 0.1 for a print)
ss_cell_r = 0.3;
cap_top = retainer_z + cap_h - flange_t;                // at rest
// Stepped ends: the screen face is set by the panel stack, which leaves genuine caps below it. So
// each end drops to a button deck where the caps stand as on a DS Lite (the band across the case stays at the screen's
// height; the strip between the clusters, the spine, is under the band's end), and Start and Select and the end screw sit
// in the deck
cap_proud = cap_h - flange_t - lip_t;                   // above the deck: 1.41, Pi Tin's 1.3 (1.85 until 2026-10-21,
                                                        // over a 1.16 lip from scans of aftermarket shells)
deck_face = cap_top - cap_proud;                        // 3.90 over the board (the faceplate's face is 3.85 over it)
z_deck_under = deck_face - top_t;                       // the deck's underside
// The ends (Alex, 2026-10-21, items 14 and 15): each end of the top (both clusters and Start/Select) is at deck_face, 1.85
// under the screen face, so the genuine caps stand cap_proud proud of it. Since 2026-10-25 the top is one part: a band at
// the screen's height across the whole width and an end at deck_face beyond it (case/SHELL.md; the island and the two
// decks it replaced are in "Rejected" there)
spine_w = 13.5;                                         // positions.json's decks zones (unchanged since 2026-10-22) leave this
                                                        // strip between the clusters out: the board's tall parts under the
                                                        // band's end (check_fit.py's TOP zones)
step_d = case_h - deck_face;                            // the screen face over the ends (1.85)
joint_y = bay_l / 2 + 3.0;                              // the old step's foot: clear of the panel's frame and FPC fold
joint_w = 2.55;                                         // and its wall's width; the board's decks zone (positions.json) starts this far inside it
// the tongue's top along the side walls: seam_gap under the ends' underside, 3.03 over the seam (0.02 more: the groove's top
// off the ends' underside's plane)
arm_tongue_top = z_deck_under - seam_gap - 0.02;
// Start and Select (2026-10-19): each seat's DS Lite Start/Select membrane (it comes in the same
// pad kit: one piece, two pills) between its clusters, near the case's end, Start on the
// player's right. Seat 1's Start is the power button. ESTIMATE: membrane, pill pitch and caps
ss_pitch = 7.5;  ss_mem = [14, 6];                      // pill to pill (the faceplate's 7.85), membrane outline
// The faceplate's openings (2026-10-21): a 3.80 hole (4.5 caps can't pass it) with a 0.6 chamfer, a
// 5.35 flange pocket 1.0 tall over each cell, a 1.2 lip: the flanges sit 0.4 over the A/B/X/Y ones
ss_hole_d = 3.8;  ss_pocket = [5.35, 1.0];              // hole; flange pocket: diameter, height
ss_lift = 0.35;                                         // flange lip over the A/B/X/Y lips, as dpad_lift (faceplate 0.4)
ss_lip_z = retainer_z + ss_lift;                        // the Start/Select flanges' lip
ss_cap_d = 3.2;  ss_flange_d = 5.0;  ss_cap_h = 3.0;    // cap body, flange, flange bottom to top (ESTIMATES: the body
                                                        // and flange from the faceplate's openings; were 4.5 and 6.0)

// ---------------- derived ----------------
cluster_spacing = case_w - piece_d - 2 * edge_margin_x;
band = edge_margin + piece_d + piece_to_glass;
case_l = bay_l + 2 * band;
inset = wall + board_clear;
board_w = case_w - 2 * inset;
board_l = case_l - 2 * inset;
seat_y = case_l / 2 - edge_margin - piece_d / 2;
ss_y = seat_y + 9.1;                                    // Start/Select's centre line

function rot180(p) = [-p[0], -p[1]];
function kicad(p) = [100 + p[0], 100 - p[1]];
dpad1 = [-cluster_spacing / 2, -seat_y];
abxy1 = [ cluster_spacing / 2, -seat_y];
dpad2 = rot180(dpad1);  abxy2 = rot180(abxy1);
clusters = [dpad1, abxy1, dpad2, abxy2];
d = dpad_pitch / 2;  a = abxy_pitch / 2;
seat1 = [
  ["SW1", "P1 up",    dpad1 + [0,  d]],
  ["SW2", "P1 down",  dpad1 + [0, -d]],
  ["SW3", "P1 left",  dpad1 + [-d, 0]],
  ["SW4", "P1 right", dpad1 + [ d, 0]],
  ["SW5", "P1 A",     abxy1 + [ a, 0]],
  ["SW6", "P1 B",     abxy1 + [0, -a]],
  ["SW7", "P1 X",     abxy1 + [0,  a]],
  ["SW8", "P1 Y",     abxy1 + [-a, 0]],
];
seat2_names = ["P2 up", "P2 down", "P2 left", "P2 right", "P2 A", "P2 B", "P2 X", "P2 Y"];
buttons = concat(seat1, [for (i = [0:7]) [str("SW", i + 9), seat2_names[i], rot180(seat1[i][2])]]);
// Start/Select pills: seat 1 faces +Y, so its right is +X; seat 2's are seat 1's turned round
ss_buttons = [["SW20", "P1 select", [-ss_pitch / 2, -ss_y]], ["SW21", "P1 start", [ss_pitch / 2, -ss_y]],
              ["SW22", "P2 select", [ss_pitch / 2, ss_y]], ["SW23", "P2 start", [-ss_pitch / 2, ss_y]]];
ss_centres = [[0, -ss_y], [0, ss_y]];

// Six M2 screws, as the DS Lite closes: from the back through the tub's bosses and the board's holes,
// clamping the board between the tub's bosses and the top's (the first print's pins through the
// board's holes snapped, so they went; crush ribs centre the board and the screws line it up). Four
// go into the top's bosses beside the glass, one into each end's boss between its clusters.
// The board goes in one way only: turned round, J1's receptacle hits the wall away from the USB-C
// opening, and the end bosses are off the centre line
// The side screws are the board's (placement.py SIDE_SCREWS; Alex, 2026-10-22: |y| <= 29, inside the
// top, no shoulders): H1 and H2 at the seat-2 ends of the strips beside the cell's hole, H3 in
// the right bezel between the speaker and J4, H4 in the left bezel between J2 and the LDO
// (2026-10-26, the 3.5 in panel: all four at the glass's corners, x +-32.5 (beyond the panel's frame, 28.75) and y +-40, under the
// island's side slope and the deck beside it, where the ends' ceiling holds their bosses)
side_pins = [[32.5, 40.0], [-32.5, 40.0], [32.5, -40.0], [-32.5, -40.0]];
pin_key = 3.0;            // the end bosses sit off the centre line (seat 1's board hole is there)
pin_out = 1.4;            // and this much further out than the clusters
// seat 2's end boss is seat 1's turned round (2026-10-20)
pins = concat(side_pins, [[pin_key, seat_y + pin_out], [-pin_key, -seat_y - pin_out]]);
boss_d = 5.0;  hole_d = 2.2;
// The tub's bosses under the board (0.7 tall, board_lift) are 6.0, except at H2 and H4, where 6.0
// would come within 0.8 of the cell's hole (H2) and 0.2 of the board's edge (H4): there they are
// boss_d, 1.3 round the screw's 2.4 clearance (the head's counterbore is in the floor under them)
tub_boss_d = [6.0, boss_d, 6.0, boss_d, 6.0, 6.0];
// M2 x 6 self-tapping, wafer head (4 mm, 1 mm): M2 x 8 until the case got 3.8 mm thinner (2026-10-22),
// when an 8 would have run past the ends' pilots
screw_cb = [4.4, 1.0];  screw_clear = 2.4;  screw_pilot = 1.6;  screw_l = 6;
deck_engage = deck_face - z_top_cu - 0.6;               // the end screws' pilots stop 0.6 under the ends' face (the screw's tip 0.1 short)
side_engage = min(6.0, case_h - z_top_cu - 0.8);        // the side screws' pilots stop 0.8 under the band's face

// edge parts, all on the right (+X) long edge. J1 slid 2 mm up its wall (towards seat 2) when the side
// screws came back inward (2026-10-22, CELLCUT_FOR_CASE.md): its opening, tab, notch and slot follow usb_y
usb_y = 1.5; usb_w = 8.94;  usb_h = 3.25;               // TYPE-C-31-M-12 shell
usb_r = 1.0;                                            // its corner radius: a rounded rectangle with 1.2 mm flats
                                                        // on its ends, not a stadium (HRO's 3D model, 2026-10-20)
// The receptacle's face sits usb_recess under the case's surface, so a plug's overmold comes up to
// the case (a mated plug's stands about 0.25 off it: plug_len_min) and the opening only has to clear
// the receptacle's shell: the shell's shape (Alex,
// 2026-10-20: the stadium it used to be was "a different kind of oval"), usb_gap round it (Alex's fit
// test: the overmold-sized opening left a big gap round a port sunk 1.15 mm in the wall). To reach the
// surface the board grows a tab under the receptacle (its face stays 1.35 past the board's edge, as
// its footprint expects), which sits in a slot in the wall. The opening is split at the port's
// centre: the upper half in the top's wall, the lower half in a tab of the tub that rises from
// the seam into a notch in the skirt (no roof over a hole, in either print pose). The seams round the
// tab step halfway through the wall (usb_step), so they don't run straight through it (Alex,
// 2026-10-20: light showed through them). The tab stays with the seam under the port (2026-10-22):
// the receptacle stands 0.85 into the skirt, and the board goes onto the face-down top straight
// down, so whatever lies under the receptacle's mouth in the wall can't be the top's
usb_recess = 0.15;  usb_gap = 0.25;
usb_hole = [usb_w + 2 * usb_gap, usb_h + 2 * usb_gap, usb_r + usb_gap];   // width, height, corner radius
usb_step = 1.0;                                         // the step in the tab's seams, inside the wall's outer half
usb_notch_w = 18.0;  usb_fit = 0.2;    // the notch in the top's skirt, and the tub's tab's clearance in it
usb_front = case_w / 2 - usb_recess;                    // receptacle face
usb_tab = usb_front - 1.35 - board_w / 2;              // the board's tab under it, past the board's edge
usb_tab_w = 12.0;  usb_tab_fit = 0.3;
// J1 stands in the right wall, its top 0.65 under the ends' face and the opening's top 0.4 under it, so no side at the
// ends' height could hold it; the band's side is at the screen's height there. The keys, grip ribs and the board
// stay clear of the tab's notch along the wall (usb_zone)
usb_zone = [usb_y - usb_notch_w / 2 - usb_fit, usb_y + usb_notch_w / 2 + usb_fit];
// a USB-C plug (USB-IF Type-C compliance document rev 1.2): its shell 6.65 +-0.10 long from the
// overmold's front, the overmold 12.35 x 6.5 at most. Mated, its tip is usb_mate inside the
// receptacle's face (the receptacle's shell length, 6.20 in the Type-C spec)
plug_len_min = 6.55;  usb_mate = 6.20;
// left long edge: debug. SW18 BOOTSEL and SW19 RESET under pinholes in the face: since 2026-10-24 (the routing
// levers, CELLCUT_FOR_CASE.md) out of the left strip into the left bezel below H4, one above the other, each
// lying across the bezel (TS-1088, its 3D model 4.8 x 2.9 x 1.95: pin_sw its half size in the plan, pin_sw_h
// its height). A floor rib in the tub under them (pin_rib) carries a pin's press into the floor
pin_pos = [[-31.0, -30.2], [-31.0, -33.85]];
pin_sw = [2.4, 1.45];  pin_sw_h = 1.95;
pin_guide_d = 3.0;  pin_guide_z = 2.4;                  // the guides round the pinholes, down to this over the board
// J5 (SWD), JST SH, its mouth towards the left edge (the board's, 2026-10-22): plugged with the case open (the
// tub off), as the plug stands past the board's edge; no slot in the wall. J6 (UART) went on 2026-10-24
swd_pos = [-32.1, -5.0];
debug_h = 2.9;
// J2, the cell's plug (PicoBlade 53261, 3.4 tall), at the cell hole's seat-1 corner in the left
// bezel, its mouth into the hole's corner: the mated plug stands 3 mm out over the left strip and
// the hole's corner (placement.py), its leads down beside the cell's left end
j2_pos = [-31.7, 2.56];  j2_h = 3.4;
// the panel frame's notches where a tall connector (3.4, over the frame's limit) lies beside the glass: [side, y0, y1]
frame_notches = [[1, -15.3, -5.1], [-1, -0.5, 9.7]];
j2_plug = [[-35.4, 5.2], [-28.0, 9.2]];               // the mated plug's outline, corners
// the spine (between each seat's clusters) keeps its length for positions.json's decks zones: seat 1's covers U6 (the
// backlight's op-amp, 1.75 tall, at y -36.4, over the ends' 1.6; the top has a pocket for it, u6_box) and seat 2's only
// U9 (1.2)
spine_end = 43.695 + joint_w;                           // the spine's end, its inside to 43.7

// ---------------- checks ----------------
// the distance from point p to the rectangle r = [[x0, y0], [x1, y1]] (0 inside it)
function rect_dist(p, r) = norm([max(0, min(r[0][0], r[1][0]) - p[0], p[0] - max(r[0][0], r[1][0])),
                                 max(0, min(r[0][1], r[1][1]) - p[1], p[1] - max(r[0][1], r[1][1]))]);
assert(cluster_spacing >= piece_d + 2, "a seat's two silicone pieces would touch");
assert(seat_y - piece_d / 2 >= bay_l / 2 + piece_to_glass - 1e-6, "cluster runs into the glass");
assert(cluster_spacing / 2 + piece_d / 2 <= board_w / 2, "silicone piece hangs off the board");
assert(glass_side >= 9.7, "no room for the tall connectors beside the glass");
// the cell in its hole: on the board, under the panel and the ends, and its pocket in the floor
assert(cell_hole[0][0] >= -board_w / 2 + 7 && cell_hole[1][0] <= board_w / 2 - 7, "the strips beside the cell's hole are too narrow");
assert(cell_hole[1][0] - cell_hole[0][0] >= cell[0] + 1.0 && cell_hole[1][1] - cell_hole[0][1] >= cell[1] + 1.0,
       "a worst-case cell doesn't fit its hole");
// (the fit kit's tuning slices render other silicone heights, which move the ends' face: tune skips the
// checks of the ends against the cell and the screws)
tune = false;
assert(tune || cell_top <= glass_gap - 0.4 && cell_top <= z_deck_under - z_top_cu - 0.3 + 1e-6, "the cell's top runs into the panel or the ends");
assert(cell[2] - cell_top >= pcb_t + 0.5, "the cell doesn't reach through the board to the floor");
assert(floor_t - cell_pocket >= 0.8, "the floor under the cell is too thin");
assert(board_lift >= 0.7 - 1e-6, "the board rests on ribs too low to print");
// the side screws' bosses (the tub's under the board, the top's on it) on the board, clear of
// the cell's hole and the board's edge
for (i = [0:3]) let(s = pins[i], b = max(tub_boss_d[i], boss_d)) {
  assert(rect_dist(s, cell_hole) >= b / 2 + 1.0, "a side screw's boss runs into the cell's hole");
  assert(abs(s[0]) + b / 2 <= board_w / 2 - 0.5, "a side screw's boss runs past the board's edge");
}
// the speaker in the right strip, sunk in its hole in the board like the cell: the hole (the speaker's outline plus spk_keep)
// leaves the board's edge a rim of 0.9, is a web 1.0 clear of the cell's hole and of J4's plug, and the speaker's top, on its seat in the
// floor's pocket, stays under the glass's and the ends' limits; its grille's slots lie inside its outline less grille_in, outside the panel's
// frame, on the island's side slope (or the ear's flat top)
spk_top_over = spk_seat + spk_h_max - z_top_cu;         // its top over the board's top, worst case (0.9)
assert(spk_pos[0] + spk[0] / 2 + spk_keep <= board_w / 2 - 0.9, "the speaker's hole leaves the board's edge too thin a rim");
assert(spk_pos[1] + spk[1] / 2 + spk_keep <= cell_hole[0][1] - 1.0 || spk_pos[0] - spk[0] / 2 - spk_keep >= cell_hole[1][0] + 1.0, "the speaker's hole runs into the cell's");
assert(spk_pos[1] + spk[1] / 2 + spk_keep + 1.0 <= j4_plug[0][1] || spk_pos[1] - spk[1] / 2 - spk_keep - 1.0 >= j4_plug[1][1], "the speaker's hole runs into J4's plug");
assert(floor_t - spk_pocket_floor >= 0.8, "the floor under the speaker is too thin");
assert(spk_top_over <= glass_gap - 0.4 && spk_top_over <= z_deck_under - z_top_cu - 0.3 + 1e-6, "the speaker's top runs into the panel or the ends");
assert(spk_pos[0] - spk[0] / 2 - spk_keep >= -board_w / 2 + 0.9 && grille_x[0] >= (panel_w + panel_fit) / 2 + 0.9, "the grille runs into the screen's bezel");
assert(grille_x[1] <= case_w / 2 - face_r - 0.5, "the grille runs into the side round");
assert(j4_body[0][0] >= (panel_w + panel_fit) / 2 + 0.1 && j4_body[1][0] <= board_w / 2 - 0.5, "J4 isn't in the strip beside the glass");
// USB-C: the opening centred on the receptacle's mouth (it stands on the board's top), usb_gap round
// it, cut through the whole wall from inside the tub's round to past the outside, the receptacle's
// face in the wall, and a mated plug's overmold off the wall (check_fit.py checks the same against
// J1's 3D model, 2026-10-22)
assert(abs(usb_zc - (z_top_cu + usb_h / 2)) <= 0.05 && usb_hole[0] >= usb_w + 0.3 && usb_hole[1] >= usb_h + 0.3,
       "the USB-C opening doesn't clear the receptacle's mouth");
assert(usb_open_x[0] <= in_x - tub_round_in - 0.5 && usb_open_x[1] >= case_w / 2 + 0.5, "the USB-C opening isn't cut through the wall");
assert(usb_front > in_x && usb_front <= case_w / 2, "the USB-C receptacle's face isn't in the wall");
assert(usb_recess <= plug_len_min - usb_mate - 0.1, "a mated USB-C plug's overmold runs into the wall");
// the opening's lower half is the tub's tab's, which rises from the seam (under the opening it is 0.8
// tall or more), and its upper half the top's: the receptacle (on the board, face down in the top)
// goes into the notch from behind, past nothing of the top
assert(usb_zc - usb_hole[1] / 2 - z_seam >= 0.8 && usb_zc - usb_step - 0.1 >= usb_zc - usb_hole[1] / 2 + 0.3,
       "the tub's USB-C tab under the opening is too thin");
assert(usb_front - (case_w / 2 - wall / 2) > 0 && z_seam < z_top_cu, "the USB-C notch isn't needed: drop the tab");
// J2 (under the left ear) and J5 (under the slope and the deck's pocket): x, y over the board's top, their model's boxes
assert(j2_plug[0][0] >= -board_w / 2 && j2_plug[1][0] <= -(panel_w + panel_fit) / 2 - 0.15, "J2's plug isn't beside the glass");
assert(j2_plug[0][1] >= j2_pos[1] + 2.6 - 0.5 && abs(j2_plug[1][0] - cell_hole[0][0]) <= 1.0, "J2's plug isn't at the hole's corner");
// the debug switches (2026-10-24) in the left bezel below H4: their guides over them, the two side by side, clear
// of J5, J2's plug, the cell's hole and the panel's frame (H4 and the other screws' bosses: below the side screws'
// asserts). Along the left wall (the key at y -21, the grip ribs, the band's end): in the joints' asserts
function pin_rect(p) = [p - pin_sw, p + pin_sw];
assert(pin_guide_z >= pin_sw_h + 0.3 && z_ceil - z_top_cu - pin_guide_z >= 1.0, "a pinhole's guide lands on its switch or is too short");
assert(pin_pos[0][0] == pin_pos[1][0] && abs(pin_pos[0][1] - pin_pos[1][1]) >= 2 * pin_sw[1] + 0.3 && abs(pin_pos[0][1] - pin_pos[1][1]) >= pin_guide_d + 0.6,
       "the debug switches or their guides run into each other");
for (p = pin_pos) {
  assert(abs(p[1] - swd_pos[1]) >= 3.5 + pin_sw[1] + 0.3, "a debug switch runs into J5");
  assert(rect_dist(p, j2_plug) >= pin_sw[0] + 0.3 && rect_dist(p, j2_plug) >= pin_sw[1] + 0.3, "a debug switch runs into J2's plug");
  assert(rect_dist(p, cell_hole) >= max(pin_sw) + 1.0, "a debug switch runs into the cell's hole");
}
// the side screws in the top, beside the glass: clear of the panel's frame, inside the side walls, under the whole
// screen face (not the slope), clear of the band's end (the ends' plate runs on under it) and of the speaker and J4 round H3
frame_rect = [[-(panel_w + panel_fit) / 2 - frame_wall, -(panel_l + panel_fit) / 2 - frame_wall],
              [(panel_w + panel_fit) / 2 + frame_wall, (panel_l + panel_fit) / 2 + frame_wall]];
function racetrack_dist(p, c, w, l) = norm(p - [c[0], max(c[1] - (l - w) / 2, min(c[1] + (l - w) / 2, p[1]))]) - w / 2;
for (s = side_pins) {
  assert(rect_dist(s, frame_rect) >= boss_d / 2 + 0.3, "a side boss runs into the panel frame");
  assert(abs(s[0]) + boss_d / 2 <= in_x - 1.0, "a side boss runs into the side wall");
  assert(abs(s[1]) + boss_d / 2 + 0.3 <= joint_y - joint_w + 1e-6, "a side boss runs into the band's end");
  assert(racetrack_dist(s, spk_pos, spk[0] + 2 * spk_keep, spk[1] + 2 * spk_keep) >= boss_d / 2 + 0.3, "a side boss runs into the speaker's pocket");
  assert(rect_dist(s, j4_body) >= boss_d / 2 + 0.3 && rect_dist(s, j4_plug) >= boss_d / 2 + 0.3 - 1e-6, "a side boss runs into J4 or its plug");
}
for (p = pin_pos) for (i = [0:5]) let(q = pins[i], b = max(tub_boss_d[i], boss_d)) {
  assert(norm(p - q) >= boss_d / 2 + 2.4 + 0.5, "a pinhole switch (SW18, SW19) runs into a screw boss");
  assert(rect_dist(q, pin_rect(p)) >= b / 2 + 0.5 && norm(p - q) >= b / 2 + pin_guide_d / 2 + 0.5, "a debug switch or its guide runs into a screw boss");
}
for (p = pin_pos) assert(rect_dist(p, frame_rect) >= pin_guide_d / 2 + 0.5, "a pinhole's guide runs into the panel's frame");
// the screws: each tip short of its pilot's end
assert(tune || screw_cb[1] + screw_l <= z_top_cu + side_engage + 1e-6 && screw_cb[1] + screw_l <= z_top_cu + deck_engage + 1e-6, "the screws are longer than their pilots");
assert(z_top_cu + side_engage <= case_h - 0.8 + 1e-6 && z_top_cu + deck_engage <= deck_face - 0.6 + 1e-6, "a pilot runs too near a face");
assert(panel_w + panel_fit <= bay_w && panel_l + panel_fit <= bay_l, "panel doesn't fit its bay");
assert(glass_gap >= 1.0, "no room under the panel");
assert(face_t >= 0.6 && face_t <= top_t, "face_t out of range");
assert(retainer_z < z_ceil, "button retainer above the top");
// the overhang limit and the rounds (case/SHELL.md): every round fits its wall
assert(overhang >= 30 && overhang <= 80, "overhang out of range");
// the board lies inside the tub's walls (the cell went into the board, 2026-10-22): its crush ribs
// reach past its top, its walls keep board_clear - tub_round_in off its edge
assert(z_top_cu + crush_reach <= z_seam + deck_tongue_h && z_board - 0.3 >= floor_t, "the tub's crush ribs miss the board's edge");
assert(tub_round_in <= board_clear - 0.3, "the tub's wall comes within 0.3 of the board");
// the screws' counterbores in the tub's floor keep a wall to the bottom edge's chamfer at the first layer's
// middle (0.1 up): at 60 degrees the chamfer comes 0.6 further in (H4's by the left wall: 0.60, was 1.14)
for (s = pins) assert(min(case_w / 2 - abs(s[0]), case_l / 2 - abs(s[1])) - back_in(0.1) - screw_cb[0] / 2 >= 0.5,
                      "a screw's counterbore comes within 0.5 of the tub's bottom edge");
// the seam on the back's round (2026-10-22): the skirts' tips (at z_bot, where the round has come in
// furthest over the skirts) skirt_t thick outside their grooves, at the screen's height and the ends'; and
// beside the board's edge, at every height the skirts cover, the skirt, its fit, the tongue and 0.25
// to the board inside the round (the round comes in furthest at the skirts' tips)
assert(z_seam <= back_r && z_seam >= back_r * (1 - sin(overhang)), "the seam isn't on the back's round");
for (H = [case_h, deck_face]) assert(groove_in(z_bot, H) - back_in(z_bot) >= skirt_t - 1e-6, "a skirt's tip is too thin");
assert(wall + board_clear - back_in(z_bot) >= skirt_t + seam_fit + tongue_t + 0.25, "no room beside the board's edge for a skirt and the tongue");
// The joints (2026-10-22, case/SHELL.md "The joints"; tune skips them: its slices move the ends' face). The tongue: as tall
// as Alex asked (at least 2.5, about 3), its tip whole, 0.2 or more from the board's edge (it passes the edge as the tub goes
// on), the groove over it leaving the skirts skirt_t (by groove_in) and the ends' plate at least 0.8
function inner_max(H, z0, zt) = max([for (l = tongue_levels(H, z0, zt)) l[2]]);
function tip_min(H, z0, zt) = min([for (l = tongue_levels(H, z0, zt)) l[2] - l[1]]);
if (!tune) {
  assert(deck_tongue_h >= 2.5 && z_arm_tongue - z_seam >= 2.5, "the tongue is shorter than the 2.5 Alex asked for");
  assert(tip_min(deck_face, z_seam, z_deck_tongue) >= 0.25 && tip_min(deck_face, z_seam, z_arm_tongue) >= 0.4 && tip_min(deck_face, z_seam, z_key) >= 0.25,
         "the tongue's (or a tab's) tip is too thin");
  assert(tongue_in(z_deck_tongue - 0.4, deck_face, z_deck_tongue) - tongue_out(z_deck_tongue - 0.4, deck_face, z_deck_tongue) >= 0.6,
         "the tongue is too thin 0.4 under its tip");
  assert(max(inner_max(deck_face, z_seam, z_deck_tongue), inner_max(deck_face, z_seam, z_arm_tongue), inner_max(deck_face, z_seam, z_key))
         <= wall + board_clear - 0.2, "the tongue comes within 0.2 of the board's edge");
  assert(deck_face - (z_deck_tongue + seam_gap) >= 0.8, "the ends' plate over the tongue is too thin");
  assert(z_deck_tongue > z_arm_tongue && deck_face - face_r < z_deck_tongue, "the tongue's heights are the wrong way round");
  // the step in the tongue (tongue_y) lies under the slope, where the cavity's ceiling is higher than the taller tongue
  assert(uni_lim(tongue_y - tongue_y_fit) >= z_deck_tongue + seam_gap + 0.3, "the taller tongue runs into the slope's ceiling");
  // the keys: one in each side wall's band, on the side walls' height of tongue, clear of the USB-C notch, the grip ribs
  // and the side screws' bosses; the plate over its pocket 0.8. The left keys are the right ones turned round
  assert(deck_face - (z_key + seam_gap) >= 0.8, "the plate over a key's pocket is too thin");
  assert(key_lead < key_w / 2 && key_lead <= key_h, "a key's lead-in is out of range");
  for (k = keys) assert(len([for (q = keys) if (q[0] == -k[0] && q[1] == -k[1]) q]) == 1, "the keys aren't each other turned round");
  for (g = grips) assert(len([for (q = grips) if (q[0] == -g[0] && q[1] == -g[1]) q]) == 1, "the grip ribs aren't each other turned round");
  for (k = keys) {
    assert(abs(k[1]) + key_band <= tongue_y - tongue_y_fit - 1.0, "a key runs into the tongue's step");
    assert(k[0] < 0 || k[1] - key_band >= usb_zone[1] + 1.0 || k[1] + key_band <= usb_zone[0] - 1.0, "a key is at the USB-C notch");
    for (g = grips) if (g[0] == k[0]) assert(abs(g[1] - k[1]) >= key_w / 2 + seam_fit + grip_w / 2 + 0.5, "a key runs into a grip rib");
    for (s = side_pins) assert(abs(s[0]) + boss_d / 2 <= case_w / 2 - key_pocket_in - 0.3 || abs(s[1] - k[1]) >= key_band + boss_d / 2, "a key's pocket runs into a side screw's boss");
  }
  // the grip ribs: on the tongue's straight part, 0.05 short of the skirt, each on its height of tongue, clear of the
  // USB-C notch
  for (g = grips) assert(grip_top(g[2]) - z_seam >= 1.5, "a grip rib is too short");
  for (g = grips) let(w = len(g) > 3 ? g[3] : grip_w) {
    assert(g[2] == z_arm_tongue ? abs(g[1]) + w / 2 <= tongue_y - tongue_y_fit - 0.3 : abs(g[1]) - w / 2 >= tongue_y + tongue_y_fit + 0.3,
           "a grip rib is on the wrong height of tongue");
    assert(g[0] < 0 || g[1] + w / 2 <= usb_zone[0] - 0.5 || g[1] - w / 2 >= usb_zone[1] + 0.5, "a grip rib is at the USB-C notch");
  }
  assert(grip_reach > 0 && grip_reach < seam_fit, "a grip rib reaches the skirt");
  // the debug switches and their pinholes' guides, 0.3 (the guides 0.5) in from the side wall's cavity, clear of the key at
  // y -21 (its pocket in to key_pocket_in), the grip ribs and the band's end
  for (p = pin_pos) {
    for (k = keys) if (k[0] * p[0] > 0) assert(abs(k[1] - p[1]) >= key_band + pin_sw[1] + 0.5 || abs(p[0]) + pin_sw[0] <= case_w / 2 - key_pocket_in - 1.0,
                                               "a debug switch runs into a key's pocket");
    for (g = grips) if (g[0] * p[0] > 0) assert(abs(p[0]) + pin_guide_d / 2 <= in_x - 2.0, "a pinhole's guide runs into a grip rib");
    assert(abs(p[1]) + max(pin_sw[1], pin_guide_d / 2) <= joint_y - joint_w - 3.0, "a debug switch runs into the band's end");
  }
  assert((boss_d - screw_pilot) / 2 >= 1.2, "a pilot's wall is too thin");
  // the slope's foot clears the ends' button walls on the pads, their end bosses and Start/Select's frames
  for (c = clusters) assert(abs(c[1]) - (c == dpad1 || c == dpad2 ? dpad_wall[0] + dpad_wall[1] : a + abxy_ring) >= uni_foot - 0.05 + 0.3,
                            "the slope runs into the walls on a pad");
  assert(seat_y + pin_out - boss_d / 2 >= uni_foot + 0.3, "the slope runs into an end boss");
  assert(ss_y - (ss_cell + 2 * ss_frame_w) / 2 >= uni_foot + 0.5, "the slope runs into Start/Select's frame");
}
// The button stack's design rules (check_buttons.py checks the geometry itself: the pads clear
// the shell, every cap sits and presses without touching it, at rest the caps only touch the domes)
assert(sil_rib_gap >= sil_web_t, "the ribs crush the silicone pads' web");
assert(retainer_bot > z_top_cu + sil_web_t, "the top reaches down onto the pads");
assert(cap_proud - dome_travel >= 0.4, "a pressed cap sinks to the deck: a thumb catches its edge");
let (r = (cap_hole_d - cap_abxy_d) / 2) assert(r >= 0.15 && r <= 0.5, str("A/B/X/Y hole clearance ", r, " a side: sticks or rattles"));
assert(dpad_gap >= 0.3 && dpad_gap <= 1.0, "D-pad opening clearance out of range");
let (r = (ss_hole_d - ss_cap_d) / 2) assert(r >= 0.15 && r <= 0.5, str("Start/Select hole clearance ", r, " a side"));
assert(pocket_d >= cap_flange_d + 0.3, "A/B/X/Y flanges bind in their pockets");
assert(ss_pocket[0] >= ss_flange_d + 0.3, "Start/Select flanges bind in their pockets");
assert(ss_cell >= ss_well[0] + 0.2, "Start/Select cells too small for the pad's well");
assert(ss_pocket[0] <= ss_cell, "a Start/Select flange pocket wider than its cell: face down, it overhangs");
assert(ss_lip_z - ss_pocket[1] >= z_top_cu + ss_well[2] + 0.1, "the Start/Select cells' roof lands on the pad's wells");
// row 4: each Start/Select metal dome fits its pad's well, and the plunger rests clear of its apex
assert(ss_well[0] - 2 * ss_well[1] >= ss_metal_dome[0] + 0.3, "the metal dome doesn't fit the pad's well");
assert(ss_plunger_gap + (ss_lift - 0.35) >= ss_metal_dome[1] + 0.05 - 1e-9, "the Start/Select plunger rests on its metal dome");
// the lips over the flanges (each a 45 degree chamfer and a straight part)
assert(lip_t >= cap_chamfer + 0.8 && deck_face - (retainer_z + dpad_lift) >= dpad_chamfer + 0.3
       && deck_face - ss_lip_z >= cap_chamfer + 0.5, "a lip over the flanges is too thin");
assert(abxy_ring >= pocket_d / 2 + 0.6, "A/B/X/Y clover too thin to print");
assert(abxy_ring <= abxy_lobe_r - sil_rim[0] - rim_clear, "the A/B/X/Y clover lands on the pad's rim");
assert(abxy_fillet > 0 && abxy_fillet <= abxy_ring - pocket_d / 2 + 0.6, "the clover's fillet is out of range");
assert(dpad_wall[2] >= 0.6, "the D-pad's wall is too thin to print");
assert(dpad_wall[1] + dpad_wall[0] - dpad_wall[2] >= dpad_arm_l / 2 + dpad_gap + 0.5, "the D-pad's wall is in its opening");
assert(z_top_cu + sil_rim[1] + rim_relief <= deck_face - 1.2, "the deck over the pads' rims is too thin");
assert(dpad_peg_d <= dpad_hole_d - 0.2 + 1e-6, "the D-pad's peg doesn't pass through its pad's hole");
assert(abxy_square[0] >= abxy_boss[0] + 0.1 && abxy_square[2] >= abxy_boss[1] + 0.05 && abxy_square[2] <= retainer_z - z_top_cu + 1e-6,
       "the A/B/X/Y pad's centre square doesn't fit its pocket");
assert(dpad_peg_gap >= 0, "the D-pad's peg holds its flange off the lip");
assert(z_deck_under > retainer_bot + 0.5, "button deck lower than the retainers");
assert(joint_y - joint_w > (panel_l + panel_fit) / 2 + frame_wall + 0.3, "the band's end (positions.json's decks zones) runs into the panel frame");
for (c = clusters) assert(abs(c[1]) - piece_d / 2 >= joint_y + 0.5 || abs(c[0]) - piece_d / 2 >= spine_w / 2 + 0.5,
                          "a silicone piece runs under the band");
// the openings and their chamfers lie on the ends' face, clear of the slope's foot (tune: the fit kit's slices render other
// silicone heights, which move the ends' face and with it the slope)
for (b = buttons) assert(tune || abs(b[2][1]) - cap_hole_d / 2 - cap_chamfer >= uni_foot + 1.0, "a cap's opening runs into the slope");
for (c = [dpad1, dpad2]) assert(tune || abs(c[1]) - dpad_arm_l / 2 - dpad_gap - dpad_chamfer >= uni_foot + 1.0, "the D-pad's opening runs into the slope");
for (s = pins) for (c = clusters) assert(norm(s - c) >= piece_d / 2 + boss_d / 2, "screw boss under a silicone piece");
assert(seat_y + pin_out - boss_d / 2 >= spine_end + 0.5, "an end boss runs under the spine");
assert(seat_y + pin_out + boss_d / 2 + 0.3 <= ss_y - (ss_cell + 2 * ss_frame_w) / 2, "an end boss runs into Start/Select's frame");
assert(pins[4] == rot180(pins[5]), "the end bosses aren't each other turned round");
assert(ss_y - ss_mem[1] / 2 - 0.75 >= spine_end, "Start/Select under the spine");
assert(ss_y + ss_mem[1] / 2 + 1 <= board_l / 2, "Start/Select off the board's end");
for (c = clusters) assert(norm([ss_mem[0] / 2, abs(abs(c[1]) - ss_y) < ss_mem[1] / 2 ? 0 : abs(abs(c[1]) - ss_y) - ss_mem[1] / 2] - [abs(c[0]), 0]) >= piece_d / 2 + 0.5, "Start/Select runs into a silicone piece");

// ---------------- echo: everything the board script needs ----------------
function jnum(v) = str(round(v * 1000) / 1000);
function jpt(p) = str("[", jnum(kicad(p)[0]), ", ", jnum(kicad(p)[1]), "]");
function join(l, i = 0) = i >= len(l) ? "" : str(i ? ", " : "", l[i], join(l, i + 1));
function fpc_y(in) = -(panel_l / 2 - in);               // y of a point `in` from the seat-1 glass edge
echo(str("CASE ", case_w, " x ", case_l, " x ", case_h, " mm   board ", board_w, " x ", board_l, " r ", board_r));
echo(str("JSON {",
  "\"case\": {\"w\": ", jnum(case_w), ", \"l\": ", jnum(case_l), ", \"h\": ", jnum(case_h), ", \"deck_h\": ", jnum(deck_face),
    ", \"seam\": ", jnum(z_seam), ", \"z_board\": ", jnum(z_board), "}, ",
  "\"board\": {\"w\": ", jnum(board_w), ", \"l\": ", jnum(board_l), ", \"r\": ", jnum(board_r), ", \"t\": ", pcb_t, "}, ",
  "\"glass\": [", jpt([-(panel_w + panel_fit) / 2, (panel_l + panel_fit) / 2]), ", ", jpt([(panel_w + panel_fit) / 2, -(panel_l + panel_fit) / 2]), "], ",
  "\"fpc_low\": ", fpc_low, ", \"fpc_neck_y\": ", jnum(kicad([0, fpc_y(fpc_neck_keep)])[1]), ", ",
  "\"glass_limit\": ", glass_gap - 0.2, ", ",
  "\"bezel_limit\": ", jnum(z_ceil - z_top_cu - 0.3), ", ",
  "\"retainer_limit\": ", jnum(retainer_bot - z_top_cu - 0.3), ", ",
  "\"fpc_zones\": [[", jpt([fpc_tail[0] - fpc_tail[1] / 2 - 0.5, fpc_y(tip_range[1] + 0.5)]), ", ",
    jpt([fpc_tail[0] + fpc_tail[1] / 2 + 0.5, fpc_y(fpc_body[2])]), "], [",
    jpt([fpc_body[0] - fpc_body[1] / 2 - 0.5, fpc_y(fpc_body[2])]), ", ", jpt([fpc_body[0] + fpc_body[1] / 2 + 0.5, fpc_y(-1.5)]), "]], ",
  "\"fpc_conn\": {\"x\": ", jnum(kicad([fpc_x, 0])[0]), ", \"y\": ", jnum(kicad([0, fpc_y(conn_in)])[1]),
    ", \"y_range\": [", jnum(kicad([0, fpc_y(conn_in_at(tip_range[0]))])[1]), ", ", jnum(kicad([0, fpc_y(conn_in_at(tip_range[1]))])[1]), "]}, ",
  "\"j3_zone\": [", jpt([fpc_x - j3_w / 2 - 0.5, fpc_y(conn_in_at(tip_range[1]) + 2.0 + 0.5)]), ", ",
    jpt([fpc_x + j3_w / 2 + 0.5, fpc_y(conn_in_at(tip_range[0]) - conn_front - 0.5)]), "], ",
  "\"pieces\": [", join([for (c = clusters) jpt(c)]), "], \"piece_d\": ", piece_d, ", ",
  "\"ss_buttons\": {", join([for (b = ss_buttons) str("\"", b[0], "\": ", jpt(b[2]))]), "}, ",
  "\"ss_pieces\": [", join([for (c = ss_centres) str("[", jpt(c - ss_mem / 2), ", ", jpt(c + ss_mem / 2), "]")]), "], ",
  "\"buttons\": {", join([for (b = buttons) str("\"", b[0], "\": ", jpt(b[2]))]), "}, ",
  "\"pins\": [", join([for (s = pins) jpt(s)]), "], \"boss_d\": ", boss_d, ", \"hole_d\": ", hole_d, ", ",
  "\"usb\": {\"y\": ", jnum(kicad([0, usb_y])[1]), ", \"front_x\": ", jnum(kicad([usb_front, 0])[0]),
    ", \"tab\": [", jpt([board_w / 2, usb_y + usb_tab_w / 2]), ", ", jpt([board_w / 2 + usb_tab, usb_y - usb_tab_w / 2]), "]}, ",
  "\"rib_free\": [", join([for (r = rib_free) str("[", jpt([r[0], r[3]]), ", ", jpt([r[2], r[1]]), "]")]), "], ",
  "\"pinholes\": [", jpt(pin_pos[0]), ", ", jpt(pin_pos[1]), "], ",
  "\"swd\": ", jpt(swd_pos), ", ",
  "\"cell_hole\": [", jpt([cell_hole[0][0], cell_hole[1][1]]), ", ", jpt([cell_hole[1][0], cell_hole[0][1]]), "], \"cell_top\": ", cell_top, ", ",
  "\"back_limit\": ", jnum(board_lift - 0.2), ", ",
  "\"speaker\": {\"c\": ", jpt(spk_pos), ", \"size\": [", spk[0], ", ", spk[1], "], \"keep\": ", spk_keep,
    ", \"top_over\": ", jnum(spk_top_over), ", \"seat\": ", jnum(spk_seat), "}, ",
  "\"crush\": [", join([for (c = crush) jpt(c)]), "], ",
  "\"frame\": {\"outer\": [", jpt([-(panel_w + panel_fit) / 2 - frame_wall, (panel_l + panel_fit) / 2 + frame_wall]), ", ",
    jpt([(panel_w + panel_fit) / 2 + frame_wall, -(panel_l + panel_fit) / 2 - frame_wall]), "], \"inner\": [",
    jpt([-(panel_w + panel_fit) / 2, (panel_l + panel_fit) / 2]), ", ", jpt([(panel_w + panel_fit) / 2, -(panel_l + panel_fit) / 2]),
    "], \"limit\": ", jnum(z_ceil - z_top_cu - frame_h - 0.3), ", \"notches\": [",
    join([for (n = frame_notches) str("[", jpt([n[0] > 0 ? 27 : -31, n[2]]), ", ", jpt([n[0] > 0 ? 31 : -27, n[1]]), "]")]), "]}, ",
  "\"decks\": [", join(concat([for (sy = [-1, 1], sx = [-1, 1]) str("[", jpt([sx * (spine_w / 2 - joint_w), sy * (joint_y - joint_w)]), ", ",
    jpt([sx * case_w / 2, sy * case_l / 2]), "]")], [for (sy = [-1, 1]) str("[", jpt([-spine_w / 2, sy * (spine_end - joint_w)]), ", ",
    jpt([spine_w / 2, sy * case_l / 2]), "]")])), "], \"deck_limit\": ", jnum(z_deck_under - z_top_cu - 0.3),
  "}"));
// the USB-C opening for check_fit.py (KiCad x, y; z over the board's top), not in positions.json
echo(str("USBC {\"hole_y\": [", jnum(kicad([0, usb_y + usb_hole[0] / 2])[1]), ", ", jnum(kicad([0, usb_y - usb_hole[0] / 2])[1]),
  "], \"hole_z\": [", jnum(usb_zc - usb_hole[1] / 2 - z_top_cu), ", ", jnum(usb_zc + usb_hole[1] / 2 - z_top_cu),
  "], \"wall_x\": [", jnum(kicad([in_x, 0])[0]), ", ", jnum(kicad([case_w / 2, 0])[0]),
  "], \"plug_len_min\": ", plug_len_min, ", \"mate\": ", usb_mate, "}"));

// the one-piece top's lower ceilings for check_fit.py (KiCad corners; the limit over the board's top), not in positions.json:
// the strip between the ends' clusters that positions.json's decks zones leave out (the band's end, which has the ends'
// ceiling there), less U6's pocket in it
function jrect(r) = str("[", jpt([r[0][0], r[1][1]]), ", ", jpt([r[1][0], r[0][1]]), "]");
top_strips = [for (sy = [-1, 1]) [[-(spine_w / 2 - joint_w), sy * (joint_y - joint_w)], [spine_w / 2 - joint_w, sy * (spine_end - joint_w)]]];
echo(str("TOP {\"limit\": ", jnum(z_deck_under - z_top_cu - 0.3), ", \"strips\": [", join([for (z = top_strips) jrect(z)]),
  "], \"pocket_limit\": ", jnum(deck_face - uni_pocket_skin - z_top_cu - 0.3), ", \"pockets\": [", join([for (p = uni_pockets) jrect(p)]), "]}"));
echo(str("ISLAND {\"edge\": [", uni_edge_x, ", ", uni_edge, "], \"run\": ", uni_run, ", \"r\": ", uni_r, ", \"skin\": ", uni_skin, ", \"phi\": ", uni_lim_phi,
  ", \"deck_face\": ", deck_face, ", \"z_top_cu\": ", z_top_cu, ", \"z_ceil\": ", z_ceil, ", \"z_deck_under\": ", z_deck_under, ", \"ear_xa\": ", uni_ear_xa,
  ", \"ears\": [", join([for (e = uni_ears) str("[", e[0], ", ", e[1], ", ", e[2], "]")]), "], \"pocket_skin\": ", uni_pocket_skin, ", \"case_w\": ", case_w, "}"));
for (b = buttons) echo(str(b[0], "  ", b[1], "   KiCad (", kicad(b[2])[0], ", ", kicad(b[2])[1], ")"));
echo(str("CAPS flange top ", retainer_z - z_top_cu, " above the board, cap top ", cap_top - z_top_cu, ", deck face ",
         deck_face - z_top_cu, ", screen face ", case_h - z_top_cu, ": ends ", deck_face, " mm thick, a ", step_d, " mm step"));
echo(str("STACK floor ", floor_t, " + ribs ", board_lift, " + PCB ", pcb_t, " + under the panel ", glass_gap,
         " + panel ", panel_t, " + tape ", tape_t, " + top over the panel ", face_t, " = ", case_h, " mm; top's underside ",
         bay_t, " over the board; seam ", z_seam, " over the tub's bottom"));

// ---------------- 2D ----------------
module rrect(w, l, r) offset(r = r) square([w - 2 * r, l - 2 * r], center = true);
module at(p, z = 0) translate([p[0], p[1], z]) children();
module ring(t = 0.35) difference() { offset(r = t) children(); children(); }
// The walls that stand on the pads' webs, at a cluster's centre (the faceplate's shapes, 2026-10-21),
// each kept rim_clear inside its pad's rim. The A/B/X/Y clover's outline: a lobe round each hole, the
// lobes joined by straight runs (each lobe's tangent), filleted where the runs meet
module inside_rim2d(c) offset(delta = -(sil_rim[0] + rim_clear)) pad2d(c);
module abxy_clover2d() intersection() {
  offset(r = -abxy_fillet) offset(r = abxy_fillet) union() {
    square([2 * a, 2 * abxy_ring], center = true);  square([2 * abxy_ring, 2 * a], center = true);
    for (t = [0:90:270]) rotate(t) translate([a, 0]) circle(r = abxy_ring, $fn = 64);
  }
  inside_rim2d(abxy1);
}
// the D-pad's wall: per arm a lobe round-ended at dpad_wall[1] out, chamfered at 45 degrees where
// two lobes meet (outer outline; the wall is dpad_wall[2] thick inside it)
module dpad_wall_outer2d() intersection() {
  union() for (t = [0:90:270]) rotate(t) {
    hull() { translate([dpad_wall[1], 0]) circle(r = dpad_wall[0], $fn = 64); square([0.01, 2 * dpad_wall[0]], center = true); }
    polygon([[dpad_wall[0], dpad_wall[0]], [dpad_wall[0], dpad_wall[0] + 0.95], [dpad_wall[0] + 0.95, dpad_wall[0]]]);
  }
  inside_rim2d(dpad1);
}
module dpad_wall2d() difference() { dpad_wall_outer2d(); offset(delta = -dpad_wall[2]) dpad_wall_outer2d(); }
// Start/Select: one frame round both cells, at the seat's Start/Select centre (along x)
module ss_frame2d() difference() {
  rrect(ss_pitch + ss_cell + 2 * ss_frame_w, ss_cell + 2 * ss_frame_w, ss_frame_r);
  for (s = [-1, 1]) translate([s * ss_pitch / 2, 0]) rrect(ss_cell, ss_cell, ss_cell_r);
}
// a silicone pad's outline, at its cluster's centre: a lobe round each dome on the buttons' axes
// measured (Alex, 2026-10-20): across the lobes A/B/X/Y 26.5 and D-pad 24, notch to notch 17.5 and 15.6
abxy_lobe_r = 5.75;  abxy_waist_r = 8.75;  dpad_lobe_r = 5.0;  dpad_waist_r = 7.8;
ss_pad = [15.8, 8.0];
module pad2d(c) let(dp = c == dpad1 || c == dpad2) union() {
  circle(r = dp ? dpad_waist_r : abxy_waist_r, $fn = 64);
  for (t = [0:90:270]) rotate(t) translate([(dp ? dpad_pitch : abxy_pitch) / 2, 0])
    circle(r = dp ? dpad_lobe_r : abxy_lobe_r, $fn = 48);
}
module cross(w, l) union() { square([l, w], center = true); square([w, l], center = true); }
module window() translate([0, aa_off]) square([aa_w + 2 * window_margin, aa_l + 2 * window_margin], center = true);
// The top's plan at the screen's height (the layout view): the band across the case, to its slope's foot
module top_region2d() square([4 * case_w, 2 * uni_foot], center = true);
module outline2d() rrect(case_w, case_l, corner_r);
module inner2d(grow = 0) offset(delta = -wall + grow) outline2d();
// how far in from an edge's wall line a round is at depth dd under the face it rolls into: an arc of
// radius r into the wall, and where the arc would be flatter than `overhang` from vertical, its
// tangent chamfer at `overhang` (as the part prints, the chamfer is the part nearest the bed)
// (ov: another limit than `overhang`, for the overhang kit's first-layer coupon, which stays the 45 tub's)
function edge_in(dd, r, ov = overhang) = let(s = sin(ov), c = cos(ov))
  dd >= r ? 0 : dd >= r * (1 - s) ? r - sqrt(r * r - (r - dd) * (r - dd)) : r * (1 - c) + (r * (1 - s) - dd) * tan(ov);
// how far in from the case's outline the back's round (and chamfer) has come at height z
function back_in(z, ov = overhang) = edge_in(z, back_r, ov);
// a rectangle from its corners [[x0, y0], [x1, y1]], its corners rounded r (grow: out from it)
module corners2d(c, r = 0, grow = 0) translate(c[0] - [grow, grow]) offset(r = r + grow) offset(delta = -(r + grow))
  square(c[1] - c[0] + 2 * [grow, grow]);
module cell_hole2d(grow = 0) corners2d(cell_hole, cell_hole_r, grow);
module board_outline() difference() {
  union() {
    rrect(board_w, board_l, board_r);
    if (usb_tab > 0) translate([board_w / 2 - 1, usb_y - usb_tab_w / 2]) square([usb_tab + 1, usb_tab_w]);
  }
  cell_hole2d();
  spk2d(spk_keep);
}
if (part == "board") board_outline();

if (part == "layout") {
  color("#34343d") linear_extrude(0.05) outline2d();
  color("#4a4a56") translate([0, 0, 0.07]) linear_extrude(0.02) intersection() { outline2d(); top_region2d(); }
  color("#2e7d4f") translate([0, 0, 0.1]) linear_extrude(0.05) board_outline();
  color([0.85, 0.35, 0.35, 0.5]) translate([0, 0, 0.2]) linear_extrude(0.05) cell2d(cell);
  color([0.9, 0.7, 0.2, 0.6]) translate([0, 0, 0.2]) linear_extrude(0.05) spk2d();
  color("#101014") translate([0, 0, 0.3]) linear_extrude(0.05) square([panel_w, panel_l], center = true);
  color("#3b6fd1") translate([0, 0, 0.4]) linear_extrude(0.05) window();
  color("#c9a227") translate([0, 0, 0.45]) linear_extrude(0.05)
    translate([fpc_x, fpc_y(tip_range[1] / 2)]) square([fpc_w, tip_range[1]], center = true);
  for (c = clusters) color([0.85, 0.85, 0.9, 0.45]) at(c, 0.5) linear_extrude(0.05) circle(d = piece_d);
  for (c = ss_centres) color([0.85, 0.85, 0.9, 0.45]) at(c, 0.5) linear_extrude(0.05) rrect(ss_mem[0], ss_mem[1], 2);
  for (b = concat(buttons, ss_buttons)) color("#f2c94c") at(b[2], 0.6) linear_extrude(0.05) circle(d = 4.8);
  for (s = pins) color("white") at(s, 0.7) linear_extrude(0.05) ring(0.4) circle(d = boss_d);
  color("#f08a3c") at([usb_front - 3.65, usb_y], 0.8) linear_extrude(0.05) square([7.3, usb_w], center = true);
  color("#f08a3c") at(j2_pos, 0.8) linear_extrude(0.05) square([9.55, 6.9], center = true);
  color("#f08a3c") translate([0, 0, 0.8]) linear_extrude(0.05) for (r = [j4_body, j4_plug]) corners2d(r);
  for (p = pin_pos) color("red") at(p, 0.9) linear_extrude(0.05) circle(d = 1.2);
}

// ---------------- 3D ----------------
in_x = case_w / 2 - wall;  in_y = case_l / 2 - wall;
// Crush ribs centre the board: on the tub's side walls (x) and end walls (y), [x, y, direction of the
// wall], each with a 45 degree lead-in at its upper end, where the board meets it as the tub goes on.
// The board lies inside the tub's walls since the cell went into it (2026-10-22): until then the
// ribs were on the top's walls (the board was over the seam)
crush = [[in_x, 28, 0], [in_x, -28, 0], [-in_x, 28, 180], [-in_x, -28, 180],
         [22, in_y, 90], [-22, in_y, 90], [22, -in_y, -90], [-22, -in_y, -90]];
crush_reach = board_clear - 0.05;   // how far a rib stands off the wall: 0.05 mm short of the board
module crush_rib(c) at(c) rotate(c[2]) rotate([90, 0, 0]) linear_extrude(1.2, center = true)
  polygon([[0.05, floor_t - 0.01], [-crush_reach, floor_t - 0.01], [-crush_reach, z_top_cu], [-0.01, z_top_cu + crush_reach], [0.05, z_top_cu + crush_reach]]);
// the tub's cavity follows the back's round only this far in (the board's edge, inside the walls,
// keeps board_clear - tub_round_in to them)
tub_round_in = 0.2;

// ---- the joints (2026-10-22, case/SHELL.md "The joints") ----
eps = 0.01;               // how far a solid overlaps one it joins (the 0.01s elsewhere in this file)
// The tongue and the skirts' groove over it, as insets from the case's outline at height z, along a
// wall whose face is at H (deck_face: the ends' faces, the band's sides too, as the groove follows the round from the ends'
// height) under a tongue whose top is zt. The groove's outer side (the skirt's inside) is the
// wall's middle until the face's round comes within skirt_t of it, then follows the round in; the
// tongue keeps seam_fit inside it, tongue_t thick, its tip chamfered: its outer edge (tongue_lead, a
// lead-in into the skirt) and its inner edge at 45 degrees from the wall's inside (the tongue passes
// the board's edge as the tub goes on: the chamfer leads the board in, and the tongue never comes
// nearer it than the wall does). The groove's inner side keeps seam_fit inside the tongue (it matters
// in the ends' plate, where the part is solid there)
function groove_in(z, H) = max(wall / 2, edge_in(H - z, face_r) + skirt_t);
function lead_of(zt) = zt == z_deck_tongue ? core_lead : tongue_lead;
function tongue_out(z, H, zt) = max(groove_in(z, H) + seam_fit, wall / 2 + seam_fit + max(0, z - (zt - lead_of(zt))));
function tongue_in(z, H, zt) = min(groove_in(z, H) + seam_fit + tongue_t, wall + (zt - z));
function groove_inner(z, H, zt) = tongue_in(min(z, zt), H, zt) + seam_fit;
// [z, outer inset, inner inset] every 0.05 from z0 up to zt: the tongue's, and the groove's (to zt + seam_gap)
function tongue_levels(H, z0, zt) = [for (k = [0:ceil((zt - z0) / 0.05 - 1e-6)]) let(z = min(zt, z0 + 0.05 * k))
  [z, tongue_out(z, H, zt), tongue_in(z, H, zt)]];
function groove_levels(H, zt, z0 = z_seam - 0.987) = [for (k = [0:ceil((zt + seam_gap - z0) / 0.05 - 1e-6)])
  let(z = min(zt + seam_gap, z0 + 0.05 * k)) [z, groove_in(z, H), groove_inner(z, H, zt)]];
// (the grooves' levels start off the parts' planes: a level on the top's underside left
// non-manifold edges)
// a ring round the case between two insets, lofted through [z, outer, inner] levels (the inner loft runs
// a little past both ends, so no face of the two coincides); shrink: both faces that much into the ring,
// for a solid that overlaps another's faces (eps: touching solids leave non-manifold edges)
module ring_loft(levels, shrink = 0) let(n = len(levels) - 1) difference() {
  rr_loft([for (l = levels) [l[0], l[1] + shrink]]);
  rr_loft(concat([[levels[0][0] - 0.01, levels[0][2] - shrink]], [for (l = levels) [l[0], l[2] - shrink]], [[levels[n][0] + 0.01, levels[n][2] - shrink]]));
}
// The tongue is deck_tongue_h tall round the top's ends and only arm_tongue_top along the side walls, under the band
// (the tub is as it was for the island and the decks, 2026-10-23). It steps at tongue_y, 1.2 past the old joint across the
// step (the island's wedge's reach), just inside the slope's foot (uni_foot 36.2). The skirts' grooves overlap that step
// tongue_y_fit each way, as the tub can shift along the case by its keys' fit
tongue_y_fit = seam_fit + 0.1;
tongue_y = joint_y + 1.2;
module yband(y0, y1, z0 = -1, z1 = 20) translate([-2 * case_w, y0, z0]) cube([4 * case_w, y1 - y0, z1 - z0]);
module y_in(y) yband(-y, y);                            // |y| <= y
module y_out(y) { yband(y, case_l); yband(-case_l, -y); }   // |y| >= y
z_deck_tongue = z_seam + deck_tongue_h;  z_arm_tongue = arm_tongue_top;
// Where it is the wall's inner half (all round up to z_arm_straight, round the cores up to z_tongue_straight) it is
// cut from the plan's own offsets, as the USB-C tab and the crush ribs that meet it are; the leaning and
// chamfered parts are lofts, eps inside it where they overlap
z_tongue_straight = max([for (k = [0:800]) let(z = z_seam + 0.01 * k)
  if (z <= z_deck_tongue && tongue_out(z, deck_face, z_deck_tongue) <= wall / 2 + seam_fit + 1e-6) z]);
z_arm_straight = z_arm_tongue - tongue_lead - 0.05;
module tongue_ring2d() difference() { inner2d(wall / 2 - seam_fit); inner2d(); }
module tongue() {
  translate([0, 0, z_seam - eps]) linear_extrude(z_arm_straight - z_seam + eps) tongue_ring2d();
  // under the round's end, where the board's edge is, the tongue reaches in to the cavity's wall below
  // the seam (tub_round_in), 0.3 off the board, so the tub's top 1.65 isn't only the tongue's 0.75
  if (z_seam < back_r) translate([0, 0, z_seam - eps]) linear_extrude(back_r - z_seam + eps)
    difference() { inner2d(wall / 2 - seam_fit); inner2d(-tub_round_in); }
  intersection() { ring_loft(tongue_levels(deck_face, z_arm_straight - 0.13, z_arm_tongue), eps); y_in(tongue_y + eps); }
  intersection() {
    union() {
      translate([0, 0, z_arm_straight - 0.13]) linear_extrude(z_tongue_straight - z_arm_straight + 0.13) tongue_ring2d();
      ring_loft(tongue_levels(deck_face, z_tongue_straight - 0.13, z_deck_tongue), eps);
    }
    y_out(tongue_y);
  }
}
// The keys: tabs that carry the tongue key_h higher, key_w along the wall, into pockets in the top's side walls, two on each
// side: they locate the top along the case and hold its skirt out on the tongue. [side, y]; clear of the USB-C notch and
// the grip ribs
keys = [[1, 21], [1, -18], [-1, -21], [-1, 18]];   // the left ones the right ones turned round (2026-10-24)
key_w = 5.0;  key_h = 1.1;  key_lead = 0.5;
z_key = z_arm_tongue + key_h;
module key_span(k, w, lead = 0, z0 = z_seam - 1, z1 = case_h) translate([k[0] > 0 ? 0 : -case_w, k[1], 0]) rotate([90, 0, 90])
  linear_extrude(case_w) polygon([[-w / 2, z0], [w / 2, z0], [w / 2, z1 - lead], [w / 2 - lead, z1], [-w / 2 + lead, z1], [-w / 2, z1 - lead]]);
module key_tab(k) intersection() { ring_loft(tongue_levels(deck_face, z_seam - 0.01, z_key), eps); key_span(k, key_w, key_lead, z1 = z_key); }
// its pocket in the top's wall: from the skirt's inside in to key_pocket_in (into the cavity), up to seam_gap over the key
key_pocket_in = wall + 0.1;
module key_pocket(k) intersection() {
  ring_loft([for (l = groove_levels(deck_face, z_key)) [l[0], l[1], key_pocket_in]]);
  key_span(k, key_w + 2 * seam_fit);
}
key_band = key_w / 2 + seam_fit + 0.3;                  // the clear band either side of a key
// Grip ribs on the tongue's outside, standing grip_reach off it: 0.05 short of the skirt, as the board's
// crush ribs are of the board, so a printed skirt (an inside face, which prints small) grips them. They
// hold each part on the tub until the screws go in and centre it across the case, so the parts' sides
// line up. Two on each side of each end, two on each side wall; the left wall's the right one's turned round.
// [side, y, the tongue's top there, width]
grip_reach = seam_fit - 0.05;  grip_w = 1.0;                 // GUESS (2026-10-22): the first print of these joints
                                                        // settles it (case/SHELL.md, "Best guesses")
grips = concat([for (sx = [-1, 1], sy = [-1, 1], y = [54, 66]) [sx, sy * y, z_deck_tongue]],      // (40, 52 until the 3.5 in panel moved the tongue's step out to 47.3)
               [for (sx = [-1, 1], sy = [-1, 1], y = [26, 38]) [sx, sy * y, z_arm_tongue]],
               [[1, -13.25, z_arm_tongue], [1, 17.2, z_arm_tongue], [-1, 13.25, z_arm_tongue], [-1, -17.2, z_arm_tongue]]);
// a rib runs up the tongue to 0.1 short of where its outside starts to lean in, its top a 45 degree lead-in
function grip_top(zt) = max([for (k = [0:800]) let(z = z_seam + 0.01 * k) if (z <= zt && tongue_out(z, deck_face, zt) <= wall / 2 + seam_fit + 1e-6) z]);
module grip(g) let(sx = g[0], w = len(g) > 3 ? g[3] : grip_w, z1 = grip_top(g[2]) - 0.1)
  translate([sx * (case_w / 2 - wall / 2 - seam_fit), g[1], 0]) scale([sx, 1, 1]) rotate([90, 0, 0]) linear_extrude(w, center = true)
    polygon([[-0.1, z_seam - 0.3], [grip_reach, z_seam - 0.3], [grip_reach, z1 - grip_reach - 0.05], [-0.05, z1], [-0.1, z1]]);



ss_vent = 1.25;           // each Start/Select dome's 0.5 mm vent, this far towards seat 1 from its centre
// what the floor's ribs keep clear of under the board: [x0, y0, x1, y1]
rib_free = concat([
  [cell_hole[0][0] - 0.5, cell_hole[0][1] - 0.5, cell_hole[1][0] + 0.5, cell_hole[1][1] + 0.5],   // the cell's hole
  [spk_pos[0] - spk[0] / 2 - spk_keep - 0.5, spk_pos[1] - spk[1] / 2 - spk_keep - 0.5,
   spk_pos[0] + spk[0] / 2 + spk_keep + 0.5, spk_pos[1] + spk[1] / 2 + spk_keep + 0.5],               // the speaker's
  [usb_front - 8.5, usb_y - usb_tab_w / 2, board_w / 2 + usb_tab, usb_y + usb_tab_w / 2],         // J1's shell legs
  [-10, 50.0, 10, 53.0], [-12.2, -1.0, -9.0, 2.0], [-9.3, -12.5, 1.3, -9.5], [-24.8, -22.8, -21.6, -19.6],  // test pads
  [-8.0, -50.5, 8.0, -46.5],                                                                      // J7's pogo pads
], [for (b = ss_buttons) [b[2][0] - 0.75, b[2][1] - ss_vent - 0.75, b[2][0] + 0.75, b[2][1] - ss_vent + 0.75]]);   // dome vents
module rib_free2d() for (r = rib_free) translate([r[0], r[1]]) square([r[2] - r[0], r[3] - r[1]]);
// Ribs under the strips beside the cell's hole, the board's weakest part (2026-10-22 review), along
// their length: the left one under J5, where a cable is pushed in (until 2026-10-24 also under the debug
// switches, which left the strip, and J6, which went); the right one under TH1 and J1's end (the power latch
// left it on 2026-10-24), with one across it at y 14. The floor's grid and its ring round the board's edge cross them
th1_y = 21.0;                                           // TH1 (placement.py), at the cell hole's right edge
strip_ribs = [[[-31.2 - 0.6, cell_hole[0][1]], [-31.2 + 0.6, cell_hole[1][1]]],
              [[29.6 - 0.6, cell_hole[0][1]], [29.6 + 0.6, cell_hole[1][1]]],
              [[cell_hole[1][0], th1_y - 0.6], [board_w / 2, th1_y + 0.6]]];
// and one under the debug switches (2026-10-24) in the left bezel, along under both plungers and into H4's boss:
// a pin's press goes into the floor, not into bending the board
// (2026-10-26: H4 stands at the glass's corner now, y -40, below both switches: the rib runs from under the upper one down into H4's boss)
pin_rib_y1 = max(pin_pos[0][1], pin_pos[1][1]) + pin_sw[1] + 0.5;
pin_rib = [[pin_pos[0][0] - 0.6, side_pins[3][1] + tub_boss_d[3] / 2 - 0.5], [pin_pos[0][0] + 0.6, pin_rib_y1]];
assert(side_pins[3][1] < min(pin_pos[0][1], pin_pos[1][1]) && abs(side_pins[3][0] - pin_pos[0][0]) + 0.6 <= tub_boss_d[3] / 2 - 0.3
       && pin_rib[1][1] >= max(pin_pos[0][1], pin_pos[1][1]) + pin_sw[1] + 0.3, "the debug switches' floor rib doesn't run under both into H4's boss");
for (r = rib_free) assert(r[2] <= pin_rib[0][0] || r[0] >= pin_rib[1][0] || r[3] <= pin_rib[0][1] || r[1] >= pin_rib[1][1],
                          "the debug switches' floor rib is cut by a keep-out");
// the speaker's outline in the plan, a racetrack along y (grow: out from it), and its grille: slots
// along it, inside the outline less grille_in
module spk2d(grow = 0) at(spk_pos) hull() for (s = [-1, 1]) translate([0, s * (spk[1] - spk[0]) / 2]) circle(d = spk[0] + 2 * grow);
module grille2d() offset(r = grille_slot[0] / 2, $fn = 24) intersection() {
  translate([grille_x[0] + grille_slot[0] / 2, -case_l]) square([grille_x[1] - grille_x[0] - grille_slot[0], 2 * case_l]);
  spk2d(-grille_in - grille_slot[0] / 2);
  at(spk_pos) for (i = [-3:3]) translate([i * grille_slot[1], 0]) square([0.002, 2 * spk[1]], center = true);
}
// the cell's outline in its hole, pushed to the hole's right end and centred along it (size: worst or nominal)
cell_x1 = cell_hole[1][0] - cell_pocket_fit;            // its right end, against the pocket's wall
module cell2d(size) translate([cell_x1 - size[0], (cell_hole[0][1] + cell_hole[1][1] - size[1]) / 2]) square([size[0], size[1]]);

// ---- the tub (the back): printed back down, as exported ----
// its outside: the back rolls into the walls (in 0.1 mm layers up the round), up to the seam
// Smooth, not in steps: one solid lofted through the outline every 0.1 up the round (the outline
// inset back_in(z), a rounded rectangle whose corners shrink with it), so the slicer meets the real
// surface. Stepped 0.1 layers put their flat faces exactly on the P1S's slicing planes (0.1, 0.3,
// 0.5 ...), and it took the step above or below as it liked: the chamfer at the bed printed 0.35,
// 0.1, 0.3, 0.1 out per layer, a staircase of 0.4 risers (row 9, Alex: "a vertical lip" at the
// bottom edge)
back_n = round(back_r / 0.1);                           // the loft's levels: every 0.1 up the round
corner_n = 18;                                          // points on each corner's quarter arc
function rr_pts(d, z) = let(hx = case_w / 2 - corner_r, hy = case_l / 2 - corner_r, r = corner_r - d)
  [for (q = [0:3]) let(cx = (q == 0 || q == 3) ? hx : -hx, cy = q < 2 ? hy : -hy)
     for (i = [0:corner_n]) let(a = 90 * q + 90 * i / corner_n) [cx + r * cos(a), cy + r * sin(a), z]];
// a solid lofted through rings of points (each ring convex, the same count, counter-clockwise about
// the axis they are stacked along). Its sides run each edge the other way round from the caps (the
// tub's loft of 4df3f96 ran them the same way, which OpenSCAD's Manifold backend rejected,
// "PolySet -> Manifold conversion failed", and repaired on a slower path)
module loft(rings) let(m = len(rings[0]), n = len(rings) - 1) polyhedron(
  points = [for (r = rings) each r],
  faces = concat(
    [for (i = [1:m - 2]) [0, i, i + 1]],                                  // the first ring's cap
    [for (i = [1:m - 2]) [n * m, n * m + i + 1, n * m + i]],              // the last's
    [for (k = [0:n - 1], i = [0:m - 1]) let(a = k * m + i, b = k * m + (i + 1) % m) each [[b, a, a + m], [b, a + m, b + m]]]));
// a solid lofted through rings that needn't be convex (its caps one face each): the top's slopes
module loft_nc(rings) let(m = len(rings[0]), n = len(rings) - 1) polyhedron(
  points = [for (r = rings) each r],
  faces = concat([[for (i = [0:m - 1]) i]], [[for (i = [m - 1:-1:0]) n * m + i]],
                 [for (k = [0:n - 1], i = [0:m - 1]) let(a = k * m + i, b = k * m + (i + 1) % m) each [[b, a, a + m], [b, a + m, b + m]]]));
// a solid lofted through the case's outline at each level [z, inset], from the first level to the last
module rr_loft(levels) loft([for (l = levels) rr_pts(l[1], l[0])]);
// [z, inset] up the back's round every 0.1 from z0 to z1 (both ends included); grow: that much more in
function back_levels(z0, z1, grow = 0, ov = overhang) = let(k0 = floor(z0 / (back_r / back_n) + 1e-6) + 1, k1 = ceil(z1 / (back_r / back_n) - 1e-6) - 1)
  [for (z = concat([z0], [for (k = [k0:1:k1]) k * back_r / back_n], [z1])) [z, back_in(z, ov) + grow]];
// the case's outside up the back's round to h (and straight up past its end), shrink: that much inside it
// (one loft: a straight prism on its top left non-manifold edges where the USB-C tab met the joint)
module tub_outside(h = z_seam, shrink = 0, ov = overhang) rr_loft(concat(back_levels(0, min(h, back_r), shrink, ov), h > back_r ? [[h, shrink]] : []));
module tub() difference() {
  union() {
    difference() {
      tub_outside();
      // the cavity: tub_round_in inside the walls up the round (the board's edge is in it, since
      // 2026-10-22), the wall's inside above it
      translate([0, 0, floor_t]) linear_extrude(z_seam - floor_t + 0.01) inner2d(-tub_round_in);
    }
    // the tongue: the wall's inner half, up past the seam inside the skirts, its keys (tabs into the top's
    // side walls) and its grip ribs
    tongue();
    for (k = keys) key_tab(k);
    for (g = grips) grip(g);
    // the tab that carries the USB-C opening's lower half, up from the seam into the top's notch,
    // its outside the round's (0.01 in from it: its foot overlaps the wall under the seam, and an outer
    // face flush with the wall's would meet it exactly along the seam)
    intersection() { usb_notch(usb_fit, 0.1); tub_outside(case_h, 0.01); }
    // bosses up to the board (the screws come up through them)
    // (turned half a facet: unturned, the floor rib at x 24.4 runs exactly through two of H3's corners)
    for (i = [0:5]) at(pins[i]) rotate(180 / $fn) cylinder(d = tub_boss_d[i], h = z_board);
    // ribs under each silicone piece carry the button presses into the floor
    for (c = clusters) at(c) difference() {
      cylinder(d = piece_d - 4, h = z_board);
      translate([0, 0, -1]) cylinder(d = piece_d - 6, h = z_board + 2);
    }
    // the board rests on ribs board_lift high over the floor: a ring round its edge, a grid and the
    // strips' ribs, clear of the cell's hole, the test pads on the board's back, J1's legs and the
    // domes' vents
    translate([0, 0, floor_t - 0.01]) linear_extrude(board_lift + 0.01) difference() {
      intersection() {
        offset(delta = -1.5) rrect(board_w, board_l, board_r);
        union() {
          difference() { offset(delta = -1.5) rrect(board_w, board_l, board_r); offset(delta = -2.7) rrect(board_w, board_l, board_r); }
          for (x = [-board_w / 2:support_pitch:board_w / 2]) translate([x - 0.6, -board_l / 2]) square([1.2, board_l]);
          for (y = [-board_l / 2:support_pitch:board_l / 2]) translate([-board_w / 2, y - 0.6]) square([board_w, 1.2]);
          for (r = strip_ribs) corners2d(r);
          corners2d(pin_rib);
        }
      }
      rib_free2d();
    }
    for (c = crush) crush_rib(c);
  }
  // screws: a counterbore for the head from below, then clearance up through the boss
  for (s = pins) at(s, -1) { cylinder(d = screw_cb[0], h = 1 + screw_cb[1]); cylinder(d = screw_clear, h = z_board + 2); }
  // the cell's pocket in the floor, inside its hole's outline
  translate([0, 0, floor_t - cell_pocket]) linear_extrude(cell_pocket + 0.01) cell_hole2d(-cell_pocket_fit);
  // and the speaker's, inside its hole's outline (the speaker lies on it, face up)
  translate([0, 0, spk_seat]) linear_extrude(spk_pocket_floor + 0.01) spk2d(spk_keep - cell_pocket_fit);
  // USB-C: the opening's lower half, in the tab; over the tab the tongue stops either side of the
  // top's skirt, which comes down to the opening's centre there (usb_fit along the wall)
  usb_opening();
  translate([in_x - 1, usb_y - usb_notch_w / 2 - usb_fit, usb_zc - 0.11]) cube([wall + 1, usb_notch_w + 2 * usb_fit, case_h]);
  // the slot the board's tab drops into
  if (usb_tab > 0) translate([in_x - tub_round_in - 0.01, usb_y - usb_tab_w / 2 - usb_tab_fit, z_board - 0.3])
    cube([usb_tab - board_clear + tub_round_in + usb_tab_fit + 0.01, usb_tab_w + 2 * usb_tab_fit, case_h]);
}

usb_zc = z_top_cu + usb_h / 2;   // the port's centre
usb_open_x = [in_x - 1, in_x + wall + 1];   // the opening's cut, through the whole wall
// the notch in the top's skirt under the port (fit 0: as cut from the top) or the tub's
// tab in it (fit, lift: its clearance each side and above): full width in the wall's inner half,
// usb_step narrower and lower in its outer half, so the top frames the tab outside (the tab
// starts 0.01 under the seam, in the tub's wall, and 0.01 outside the wall's inside, in the tongue: a solid
// only touching it leaves non-manifold edges)
module usb_notch(fit = 0, lift = 0) let(o = fit == 0 ? 0.01 : -0.01, w = usb_notch_w - 2 * fit, dn = fit == 0 ? 1 : 0.01) {
  translate([in_x - o, usb_y - w / 2, z_seam - dn]) cube([wall / 2 + (fit == 0 ? 2 * o : o), w, usb_zc - lift - z_seam + dn]);
  translate([in_x + wall / 2, usb_y - w / 2 + usb_step, z_seam - dn])
    cube([wall / 2 + dn, w - 2 * usb_step, usb_zc - usb_step - lift - z_seam + dn]);
}
module usb_opening() translate([usb_open_x[0], usb_y, usb_zc]) rotate([0, 90, 0])
  linear_extrude(usb_open_x[1] - usb_open_x[0]) rrect(usb_hole[1], usb_hole[0], usb_hole[2]);

// every ABXY cap: [centre, tab direction in OpenSCAD's counter-clockwise degrees]. Seat 2's
// cluster is seat 1's turned 180 degrees, caps and all
function abxy_keys() = [for (s = [[abxy1, 0], [abxy2, 180]]) for (i = [0:3]) let(t = abxy_tabs[i])
  [s[0] + (s[1] == 0 ? t[1] * a : -t[1] * a), s[1] - t[2], [for (n = abxy_notches[i][1]) t[2] - n]]];
// one A/B/X/Y cap's deck cut, in its own frame (tab along +x), from the pocket's floor up: the hole,
// the flange's pocket less its notch ribs, the tab's slot
// face: the deck's face over the cut's origin, where the 45 degree chamfer opens out (face down it
// prints as a 45 degree overhang, no support)
cut_face = deck_face - retainer_bot + 0.01;
// (its round cuts are turned half a facet: unturned, corners of the hole and the pocket lie exactly
// on the tab slot's sides, at -15 and 20 degrees, and leave non-manifold edges)
module abxy_cut(h, notches, hole_d = cap_hole_d, face = cut_face) {
  rotate(180 / $fn) cylinder(d = hole_d, h = h);
  translate([0, 0, face - cap_chamfer]) rotate(180 / $fn) cylinder(d1 = hole_d, d2 = hole_d + 2 * cap_chamfer + 0.02, h = cap_chamfer + 0.01);
  difference() {
    rotate(180 / $fn) cylinder(d = pocket_d, h = pocket_h + 0.01);
    if (notch_keys) for (n = notches) rotate(n) translate([cap_flange_d / 2 - notch_reach, -notch_w / 2, -1])
      cube([pocket_d / 2 - cap_flange_d / 2 + notch_reach + 0.05, notch_w, pocket_h + 2]);
  }
  // (0.01 over the pocket's roof: a roof in the same plane leaves non-manifold edges)
  linear_extrude(pocket_h + 0.02) polygon([[0, 0], for (i = [0:6]) let(t = slot_off - slot_a / 2 + slot_a * i / 6) slot_r * [cos(t), sin(t)]]);
}
module dpad_cut(h, gap = dpad_gap, face = cut_face) {
  translate([0, 0, dpad_lift]) linear_extrude(h) offset(r = gap) cross(dpad_arm_w, dpad_arm_l);
  // the chamfer, an arm at a time (each convex, so a hull): the cross grown by dpad_chamfer at the face
  for (t = [0, 90]) rotate(t) hull() {
    translate([0, 0, face - dpad_chamfer]) linear_extrude(0.01) offset(r = gap) square([dpad_arm_l, dpad_arm_w], center = true);
    translate([0, 0, face]) linear_extrude(0.02) offset(r = gap + dpad_chamfer) square([dpad_arm_l, dpad_arm_w], center = true);
  }
  linear_extrude(pocket_h + dpad_lift + 0.01) offset(r = dpad_pocket_gap) dpad_flange();
}
// one Start/Select cap's cut: the cell for the pad's well, the flange's pocket over it, the hole and its chamfer
module ss_cut(h, face = cut_face) {
  top = ss_lip_z - retainer_bot + 0.01;                 // the pocket's roof: the flange's lip
  linear_extrude(top - ss_pocket[1]) rrect(ss_cell, ss_cell, ss_cell_r);
  translate([0, 0, top - ss_pocket[1] - 0.01]) cylinder(d = ss_pocket[0], h = ss_pocket[1] + 0.01);
  cylinder(d = ss_hole_d, h = h);
  translate([0, 0, face - cap_chamfer]) cylinder(d1 = ss_hole_d, d2 = ss_hole_d + 2 * cap_chamfer + 0.02, h = cap_chamfer + 0.01);
}
module dpad_flange() union() { circle(r = dpad_flange_r, $fn = 64); cross(dpad_arm_w, dpad_arm_l); }

// ---- the top (the one piece: the band across the case at the screen's height, the ends); printed face down ----
// Its outside is smooth solids, not 0.1 mm slabs (2026-10-22): slabs put their flat faces on the slicer's planes, and
// it printed their steps unevenly (row 9 on the tub, 4df3f96). Every face here is a ruled surface between levels every
// 0.05 down its slope and the rounds, so the slicer meets the real surface
z_bot = z_seam + seam_gap;                              // the skirts' bottom
// Under the round's end the skirt follows the back's round in, down to its tip at z_bot (2026-10-22)
skirt_levels = back_levels(z_bot, back_r);              // [z, inset] up the round, z_bot to back_r
// what the top carries inside it: the USB-C notch's roof, the side screws' bosses, the pinhole guides, the panel's frame
module top_inner() {
  // over the USB-C notch the skirt's inner half stays, from the opening's centre up: the tongue
  // stops either side of it, so it frames the opening's upper half and keys the top along the wall
  translate([in_x - eps, usb_y - usb_notch_w / 2, usb_zc - eps]) cube([wall / 2 + 2 * eps, usb_notch_w, z_arm_tongue + seam_gap + 0.1 - usb_zc + eps]);
  // bosses down to the board top: with the tub's bosses they clamp it
  for (s = side_pins) at(s, z_top_cu) cylinder(d = boss_d, h = z_ceil - z_top_cu + 0.01);
  // pinhole guides reach down towards the switches
  for (p = pin_pos) at(p, z_top_cu + pin_guide_z) cylinder(d = pin_guide_d, h = z_ceil - z_top_cu - pin_guide_z + 0.01);
  // panel locating frame (its opening is the panel's pocket, cut below: one cut, so no two faces
  // coincide there)
  translate([0, 0, z_ceil - frame_h]) linear_extrude(frame_h + 0.01) difference() {
    square([panel_w + panel_fit + 2 * frame_wall, panel_l + panel_fit + 2 * frame_wall], center = true);
    translate([fpc_x, -panel_l / 2]) square([fpc_w + 4, 6], center = true);   // the FPC's fold passes here
    for (n = frame_notches) translate([n[0] > 0 ? 27 : -31, n[1]]) square([4, n[2] - n[1]]);   // the tall connectors' notches
  }
}
// and what is cut from it
module top_cuts() {
  // USB-C: the opening's upper half, and below it the notch the tub's tab fills
  usb_opening();
  usb_notch(0);
  // the panel's frame and its pocket: its top face_t under the screen face
  translate([0, 0, z_ceil - frame_h - 0.01]) linear_extrude(frame_h + top_t - face_t + 0.01)
    square([panel_w + panel_fit, panel_l + panel_fit], center = true);
  // viewing window: the panel's own glass shows through, no lens
  translate([0, 0, z_ceil - 1]) linear_extrude(top_t + 2) window();
  // pilot holes for the screws
  for (s = side_pins) at(s, z_top_cu - 0.01) cylinder(d = screw_pilot, h = side_engage);
  // pinholes
  for (p = pin_pos) at(p, 0) cylinder(d = 1.2, h = case_h + 1);
  // the speaker's grille: slots through the face over its cone (face down they are plain holes in the plate on the bed); the
  // rim round them stays solid for its front ring to seal on
  translate([0, 0, z_ceil]) linear_extrude(top_t + 1) grille2d();
}
// ---- an end (sy: -1 seat 1's, 1 seat 2's): what the top carries under the ends' plate, and its openings ----
// Seat 2's end is seat 1's turned round (the end bosses, the clusters and Start/Select all are)
function end_pin(sy) = sy < 0 ? pins[5] : pins[4];
module end_inner(sy) {
  // the walls that rest on the silicone pads (the pockets are cut below)
  // (the A/B/X/Y clover is solid between its pockets, where the faceplate leaves it open round
  // pinwheel ears: here the tab slots are cut into it and key the caps with the notch ribs)
  for (c = [abxy1, abxy2]) if (c[1] * sy > 0) at(c, retainer_bot) linear_extrude(z_deck_under - retainer_bot + 0.01) union() {
    abxy_clover2d();
    square(abxy_square[0] + 2 * abxy_square[1], center = true);
  }
  for (c = [dpad1, dpad2]) if (c[1] * sy > 0) at(c, retainer_bot) linear_extrude(z_deck_under - retainer_bot + 0.01) dpad_wall2d();
  for (c = ss_centres) if (c[1] * sy > 0) at(c, retainer_bot) linear_extrude(z_deck_under - retainer_bot + 0.01) ss_frame2d();
  // its boss, down to the board: the end screw pulls the top onto the board
  at(end_pin(sy), z_top_cu) cylinder(d = boss_d, h = z_deck_under - z_top_cu + 0.01);
}
// and its buttons' openings, the pads' reliefs and its pilot
module end_cuts(sy) {
  for (k = abxy_keys()) if (k[0][1] * sy > 0) at(k[0], retainer_bot - 0.01) rotate(k[1]) abxy_cut(case_h, k[2]);
  for (c = [dpad1, dpad2]) if (c[1] * sy > 0) at(c, retainer_bot - 0.01) dpad_cut(case_h);
  for (b = ss_buttons) if (b[2][1] * sy > 0) at(b[2], retainer_bot - 0.01) ss_cut(case_h);
  // over each pad, the deck's underside up past the rim, round the walls
  for (c = [abxy1, abxy2]) if (c[1] * sy > 0) at(c, z_top_cu) linear_extrude(sil_rim[1] + rim_relief) difference() {
    offset(r = rim_relief) pad2d(c);
    abxy_clover2d();
    square(abxy_square[0] + 2 * abxy_square[1], center = true);
  }
  for (c = [dpad1, dpad2]) if (c[1] * sy > 0) at(c, z_top_cu) linear_extrude(sil_rim[1] + rim_relief) difference() {
    offset(r = rim_relief) pad2d(c);
    dpad_wall2d();
  }
  for (c = [abxy1, abxy2]) if (c[1] * sy > 0) at(c, retainer_bot - 0.01)  // the A/B/X/Y pad's centre square
    linear_extrude(z_top_cu + abxy_square[2] - retainer_bot + 0.01) square(abxy_square[0], center = true);
  at(end_pin(sy), z_top_cu - 0.01) cylinder(d = screw_pilot, h = deck_engage);
}


// ---- the top: one part (case/SHELL.md; the one-piece mock-up of 2026-10-24, the design since 2026-10-25) ----
// Alex: "we're gonna go uni body for the top and print with pla support material", and on 2026-10-25, after the viewer:
// "make unibody main". One part instead of the island and the two decks (SHELL.md, "Rejected"): no wedge, lips, laps,
// hooks or build bars; it goes on the tub over the tongue and the keys. Printed face down with Bambu Support For PLA as the
// support interface, so its outside need not keep to `overhang`: the side rounds are true arcs.
// The raised screen block is a band across the whole width (2026-10-25, Alex, after an x-ray showing that J1, J2,
// J4, J5 and the speaker force a block out to the sides: "have the island extend fully to the left and right but try
// to minimize its extension on the top and bottom"): at the screen's height from the left side wall to the right, so
// the sides at the screen's height are the case's side profile (the round of face_r) and beyond the band the ends'.
// Its two ends are the only edges, and each is the DS Lite's lower screen frame in section ("make an exact copy of
// this curve structure and adapt it to our purpose"), measured from nintendo_ds_bottom_base.stl: a sharp top edge, and
// from it ONE concave arc of radius 8.0 (rms 0.002 mm over its facets, on the right side, the left and the top)
// that starts at 36 degrees off the flat and is tangent to the deck 4.70 out, 1.528 down (a 0.07 bead on the edge
// is left out: it is under half a layer, and face down it would lie below the bed). Scaled uniformly to our step
// (uni_k = step_d / 1.528 = 1.21): R 9.69, 5.69 out. The DS's plan corners (the frame is an island there, 31 from
// the side wall) are the slope's offset with a radius that grows with the height, 0 at the foot to 1.45 at the top
// edge; ours has no plan corners, the band runs out to the side walls, where the DS's own frame has nothing to copy
// (its base is cut flat at the deck's level, the slope's foot at the cut): the section is carried out over the side
// round unchanged (each slice of the step is the ends' side section lifted by the profile's height), so the same
// curve shows in the side view and the step runs straight out and is cut by the case's side at both heights
uni_ds_r = 8.0;  uni_ds_angle = 36;                      // the DS Lite's slope: its arc's radius, and its angle at the edge
uni_ds_h = uni_ds_r * (1 - cos(uni_ds_angle));           // 1.528 (measured 1.523)
uni_k = step_d / uni_ds_h;                               // the scale to ours (1.21)
uni_r = uni_ds_r * uni_k;                                // the arc's radius here (9.69)
uni_run = uni_r * sin(uni_ds_angle);                     // the slope's run, edge to foot (5.69)
uni_skin = 0.7;           // the inside keeps this far under the slope (and under the ends' face over a pocket: below)
uni_pocket_skin = 0.75;   // the ends' face over a pocket in their underside (uni_limit's slab: 0.05 under uni_skin)
// What sets the ends: the panel's pocket (half its length 30.13): the top edge is 0.4 outside it, as the last pass
// had it, which keeps the pocket's upper corner (0.8 under the face) 0.88 from the slope. J4, the FPC's fold and the
// rest of the board's tall parts no longer reach it: the slope is gentle, so the inside's ceiling is still z_ceil
// until 2.0 past the edge, the foot is uni_run further out
uni_edge = (panel_l + panel_fit) / 2 + 0.4;              // the top edge, from the middle (30.53)
uni_foot = uni_edge + uni_run;                           // 36.22
uni_s_n = 24;             // the slope's slices, every 1.5 degrees of the arc
uni_arc_n = 36;           // the side rounds' levels (true quarter arcs, every 2.5 degrees)
uni_x_r = case_w / 2 - face_r;                           // where the side rounds start (35.9)
// the case's outside up to a face at H: the skirt down the back's round, the wall, a true quarter arc of face_r
module uni_shell(H) rr_loft(concat(skirt_levels, [for (k = [0:uni_arc_n]) let(a = 90 * k / uni_arc_n) [H - face_r + face_r * sin(a), face_r * (1 - cos(a))]]));
// the slope's height over the ends' face at t out from the top edge (the DS's arc, scaled), and its slices'
// angles: from the edge (uni_ds_angle) to the foot (0)
function uni_h(t) = t <= 0 ? step_d : t >= uni_run ? 0 : uni_r * (1 - cos(asin(sin(uni_ds_angle) - t / uni_r)));
// the ends' section (the side rounds and the flat between) lifted by h, out past the outline for the case's shell
// to cut: each slice of the step
function uni_section(h) = let(cz = deck_face - face_r + h, zb = deck_face - face_r - 1, n = 18)
  concat([[-(uni_x_r + face_r + 0.5), zb]],
         [for (k = [0:n]) let(a = 90 - 90 * k / n) [-(uni_x_r + face_r * sin(a)), cz + face_r * cos(a)]],
         [for (k = [0:n]) let(a = 90 * k / n) [uni_x_r + face_r * sin(a), cz + face_r * cos(a)]],
         [[uni_x_r + face_r + 0.5, zb]]);
// (inner: the section of the inside's limit, uni_isection, instead of the outside's)
module uni_slope(inner = false) loft_nc([for (k = [-1:uni_s_n]) let(ph = uni_ds_angle * (1 - max(k, 0) / uni_s_n),
                                                                     t = k < 0 ? -0.02 : uni_r * (sin(uni_ds_angle) - sin(ph)),
                                                                     h = k < 0 ? step_d : uni_h(t))
  [for (p = inner ? uni_isection(h) : uni_section(h)) [p[0], uni_edge + t, p[1]]]]);
module uni_outer() {
  uni_shell(deck_face);
  intersection() {
    uni_shell(case_h);
    union() {
      translate([-case_w, -uni_edge, z_bot - 1]) cube([2 * case_w, 2 * uni_edge, case_h - z_bot + 2]);
      uni_slope();
      mirror([0, 1, 0]) uni_slope();
    }
  }
}
// the inside's limit under the slope: its arc at radius uni_r + uni_skin about the same centre (uni_foot from the
// middle, uni_r over the ends' face), to where it meets the face's offset (uni_skin under it). Its levels,
// [z, how far in from the foot]: straight down below the arc, the arc, the face's offset
uni_lim_phi = acos((deck_face + uni_r - case_h + uni_skin) / (uni_r + uni_skin));    // where the arc meets the face's offset
uni_limit_levels = concat([[z_seam - 2, 0]],
  [for (k = [0:uni_s_n]) let(p = uni_lim_phi * k / uni_s_n) [deck_face + uni_r - (uni_r + uni_skin) * cos(p), (uni_r + uni_skin) * sin(p)]],
  [[case_h - uni_skin, (uni_r + uni_skin) * sin(uni_lim_phi)]]);
function uni_rect(inset, z) = let(y = uni_foot - inset, x = case_w) [[-x, -y, z], [x, -y, z], [x, y, z], [-x, y, z]];
// the inside's ceiling at y from the middle, under the band (the limit's; z_ceil is the other)
function uni_lim(y) = let(d = uni_foot - abs(y), rs = uni_r + uni_skin, s = d / rs)
  d <= 0 ? deck_face - uni_skin : s >= sin(uni_lim_phi) ? case_h - uni_skin : deck_face + uni_r - rs * sqrt(1 - s * s);
// the distance from a point [y from the middle, z] in the section to the band's outside (the face, the slope's arc)
function uni_dist(p) = let(y = abs(p[0]))
  min(min([for (k = [0:200]) let(t = uni_run * k / 200) norm([uni_edge + t - y, deck_face + uni_h(t) - p[1]])]), y < uni_edge ? case_h - p[1] : 99);
// under the band the inside stays uni_skin under the slope and the face; elsewhere under the ends' face uni_pocket_skin
module uni_limit() {
  translate([-case_w, -case_l, z_seam - 2]) cube([2 * case_w, 2 * case_l, deck_face - uni_pocket_skin - z_seam + 2]);
  loft_nc([for (l = uni_limit_levels) uni_rect(l[1], l[0])]);
}
// The inside under the side rounds (2026-10-25): the ends' round (face_r) keeps uni_skin of plastic square to the
// surface, so the inside's upper outer corner (the hollow's wall at the inner outline, 2.0 in, and its ceiling) was
// 0.56 to 0.59 under it where the slope's ceiling meets the round (y 33 to 35, 0.58). The inside is cut by the round's
// offset: the same arcs' centres, radius face_r - uni_skin, lifted with the slope as the outside is. The limit is a
// slab of those sections: the band's, the slope's two sides, and the ends' beyond the foot. It reaches down
// past the seam so it cuts nothing but the inside's top
uni_in_r = face_r - uni_skin - 0.01;   // (0.01: the outside's chords, every 5 degrees, lie 0.003 inside its arcs)
function uni_isection(h) = let(cz = deck_face - face_r + h, zb = z_seam - 2, n = 18, x = uni_x_r + uni_in_r + 0.5)
  concat([[-x, zb]],
         [for (k = [0:n]) let(a = 90 - 90 * k / n) [-(uni_x_r + uni_in_r * sin(a)), cz + uni_in_r * cos(a)]],
         [for (k = [0:n]) let(a = 90 * k / n) [uni_x_r + uni_in_r * sin(a), cz + uni_in_r * cos(a)]],
         [[x, zb]]);
module uni_in_limit() {
  rotate([90, 0, 0]) linear_extrude(2 * uni_edge, center = true) polygon(uni_isection(step_d));
  translate([0, 0.003, 0]) uni_slope(inner = true);      // (0.003 off uni_limit's loft, which ends at the same foot)
  mirror([0, 1, 0]) translate([0, 0.003, 0]) uni_slope(inner = true);
  for (m = [0, 1]) mirror([0, m, 0]) translate([0, case_l, 0]) rotate([90, 0, 0]) linear_extrude(case_l - uni_foot + 0.01) polygon(uni_isection(0));
}
// The skirts' groove round the ends (2026-10-25): its upper outer corner stood 0.47 under the round (the skirt is
// skirt_t thick horizontally, and the round is 43 degrees there). The tub's tongue there has the chamfer core_lead (0.45)
// at its tip and is 0.1 lower, and the groove's outer wall is filled to follow that chamfer 0.1 off it (normal to it:
// uni_tip_clear), as a 45 degree ramp from where it leaves the groove's wall (0.25 off the tongue) up to the groove's
// ceiling, which keeps seam_gap over the tongue's top. Along the whole ring, so the side walls and the ends alike
uni_tip_clear = 0.1;
uni_ramp_h = uni_tip_clear * sqrt(2);                    // that clearance measured across the case, on a 45 degree face
function uni_groove_out(z, zt) = max(groove_in(z, deck_face), wall / 2 + seam_fit - uni_ramp_h + (z - (zt - core_lead)));
function uni_groove_levels(zt) = [for (l = groove_levels(deck_face, zt)) [l[0], uni_groove_out(l[0], zt), l[2]]];
// the skirts' groove over the tongue: round the ends (the filled groove above) and along the side walls under the band
module uni_skirt_groove() {
  intersection() { ring_loft(uni_groove_levels(z_deck_tongue)); y_out(tongue_y - tongue_y_fit); }
  intersection() { ring_loft(groove_levels(deck_face, z_arm_tongue)); y_in(tongue_y + tongue_y_fit); }
}
// its checks: the filled wall keeps its skin (uni_groove_skin, square to the round) at every level, and keeps the
// tongue's tip uni_tip_clear off (its chamfer: uni_ramp_h across, and its top's corner by distance) and, where the ramp isn't there, the groove's
// own seam_fit
uni_groove_skin = 0.65;
if (!tune) for (l = uni_groove_levels(z_deck_tongue)) {
  assert(uni_rskin(case_w / 2 - l[1], l[0], deck_face) >= uni_groove_skin, "the skirts' groove is too near the side round");
  assert(let(g = tongue_out(min(l[0], z_deck_tongue), deck_face, z_deck_tongue) - l[1])
         l[0] <= z_deck_tongue ? g >= uni_ramp_h - 1e-6 : norm([g, l[0] - z_deck_tongue]) >= uni_tip_clear - 1e-6,
         "the filled groove runs into the tongue's chamfer");
  assert(l[0] > z_deck_tongue - core_lead - uni_ramp_h || tongue_out(l[0], deck_face, z_deck_tongue) - l[1] >= seam_fit - 1e-6, "the groove's fit is gone below the chamfer");
}
// the distance from a point [x from the middle, z] in the section where the surface's face is at H, to the side round
// (and to the flat face above the round's start)
function uni_rskin(x, z, H) = x <= uni_x_r ? H - z : z >= H - face_r ? face_r - norm([x - uni_x_r, z - (H - face_r)]) : case_w / 2 - x;
// the pocket in the ends' underside over U6 (1.75 tall: 0.15 under the ends' underside without it), up to
// uni_pocket_skin under their face, 0.5 round it (seat 1's end: U6 is at y -39.4 .. -33.4, in the spine between the clusters)
u6_pos = [-3.2, -55.0];                                  // U6 beside U8 in seat 1's spine (placement.py)
u6_box = [u6_pos + [-2.45, -3.0], u6_pos + [2.45, 3.0]];
uni_pockets = [[[u6_box[0][0] - 0.5, u6_box[0][1] - 0.5], [u6_box[1][0] + 0.5, u6_box[1][1] + 0.5]]];
module uni_cavity() intersection() {
  union() {
    translate([0, 0, z_seam - 1]) linear_extrude(z_deck_under - z_seam + 1) inner2d();     // under the ends' face
    intersection() {
      union() {
        translate([0, 0, z_seam - 1]) linear_extrude(z_ceil - z_seam + 1) intersection() {   // under the band (0.05 in: off its foot)
          inner2d(); square([2 * case_w, 2 * (uni_foot - 0.05)], center = true);
        }
        for (p = uni_pockets) translate([p[0][0], p[0][1], z_seam - 1]) cube([p[1][0] - p[0][0], p[1][1] - p[0][1], deck_face - z_seam + 1]);
      }
      uni_limit();
    }
  }
  uni_in_limit();
}

// the band is the whole width: no side edges, no ears (the island of 2026-10-26 with the slope on all four sides was reverted the same day)
j5_box = [[-34.6, -7.5], [-29.7, -2.5]];                // J5's model box (placement.py), mouth to the left wall
uni_edge_x = case_w;  uni_ears = [];  uni_ear_xa = 0;  uni_corner_r = 0;
// the inside's ceiling at p = [x, y]: z_ceil under the band, the limit's arc under the slope, the ends' underside beyond the foot
function uni_ceil(p) = abs(p[1]) >= uni_foot ? z_deck_under : min(z_ceil, uni_lim(p[1]));
// the least ceiling over a rectangle's corners, edge midpoints and centre
function uni_ceil_box(r) = min([for (u = [0, 0.5, 1], v = [0, 0.5, 1]) uni_ceil([r[0][0] + u * (r[1][0] - r[0][0]), r[0][1] + v * (r[1][1] - r[0][1])])]);
module top() difference() {
  union() {
    difference() {
      uni_outer();
      uni_cavity();
      // the groove the tub's tongue slides up, all round, and the keys' pockets
      uni_skirt_groove();
      for (k = keys) key_pocket(k);
    }
    top_inner();
    for (sy = [-1, 1]) end_inner(sy);
  }
  top_cuts();
  for (sy = [-1, 1]) end_cuts(sy);
}
// its checks: the pocket's upper corner and the side screws' pilots 0.7 from the outside, the board's tall parts (their
// 3D models' boxes, check_fit.py) 0.3 under the inside's limit (positions.json's own margin under a ceiling), the
// grille in the band's flat face, the side screws under it, U6 in its pocket, the buttons' wells clear of the foot
if (!tune) {   // (the fit kit's tuning slices move the ends' face, and with it the slope: they skip these)
  assert(abs(uni_h(0) - step_d) < 1e-6 && abs(uni_h(uni_run)) < 1e-6, "the slope's profile");
  assert(uni_dist([(panel_l + panel_fit) / 2, case_h - face_t]) >= uni_skin, "the skin over the panel's pocket's edge is too thin");
  assert(uni_ceil_box(j4_body) - (z_top_cu + j4_h) >= 0.3 && uni_ceil_box(j4_plug) - (z_top_cu + j4_h - 0.2) >= 0.3, "J4 or its plug isn't under the band");
  assert(uni_ceil_box([[j2_pos[0] - 3.85, j2_pos[1] - 2.6], [j2_pos[0] + 3.85, j2_pos[1] + 2.6]]) - (z_top_cu + j2_h) >= 0.3
         && uni_ceil_box(j2_plug) - (z_top_cu + j2_h - 0.2) >= 0.3, "J2 or its plug isn't under the band");
  assert(uni_ceil_box([[usb_front - 8.0, usb_y - 4.5], [usb_front, usb_y + 5.0]]) - (z_top_cu + usb_h) >= 0.3, "J1 isn't under the band");
  assert(uni_ceil_box(j5_box) - (z_top_cu + debug_h) >= 0.3, "J5 isn't under the band");
  assert(min(z_ceil, uni_lim(fpc_y(-1.5) - 0.3)) - (z_glass + 0.5) >= 0.3, "the FPC's fold isn't under the band");   // the fold's outer reach, 1.5 past the glass
  assert(spk_top_over >= 0 && z_ceil - (z_top_cu + spk_top_over) >= 0.5, "the speaker's top is too near the ceiling under its grille");
  assert(grille_x[1] <= uni_x_r - 0.1 && max([for (y = [spk_pos[1] - spk[1] / 2, spk_pos[1] + spk[1] / 2]) abs(y)]) <= uni_edge - 1, "the speaker's grille isn't in the band's flat face");
  for (s = side_pins) {
    assert(abs(s[0]) + screw_pilot / 2 + 1.0 <= uni_x_r && uni_dist([abs(s[1]) + screw_pilot / 2, case_h - 0.8]) >= uni_skin, "a side screw's pilot is too near the band's outside");
  }
  assert(deck_face - uni_pocket_skin - (z_top_cu + 1.75) >= 0.3 && uni_pocket_skin >= uni_skin, "no room for U6 in its pocket");
  // the inside's upper outer corner (the hollow's wall at the inner outline, up to its ceiling or the round's offset,
  // whichever is lower) keeps uni_skin square to the side round, along the slope and under the band
  for (k = [0:uni_s_n]) let(t = uni_run * k / uni_s_n, h = uni_h(t), H = deck_face + h, xw = case_w / 2 - wall,
      zc = min(min(z_ceil, uni_lim(uni_edge + t)), H - face_r + sqrt(max(0, uni_in_r * uni_in_r - pow(xw - uni_x_r, 2)))))
    assert(uni_rskin(xw, zc, H) >= uni_skin - 1e-3, "the inside's corner is too near the side round");
}

module board3d() color("#2e7d4f") translate([0, 0, z_board]) linear_extrude(pcb_t) board_outline();
// the cell in its hole on the pocket's floor, and the speaker on the board (nominal sizes)
module cell3d() translate([0, 0, floor_t - cell_pocket]) linear_extrude(cell_nom[2]) offset(r = 1) offset(delta = -1) cell2d(cell_nom);
module speaker3d() translate([0, 0, spk_seat]) linear_extrude(spk[2]) spk2d();

// button caps, for the assembly view only (they are bought DS Lite parts)
// The caps at rest (flanges up against their lips; the D-pad's and Start/Select's lips are lifted).
// Pressed (press = true): A/B/X/Y and Start/Select straight down dome_travel; the D-pad tilted toward dpad_dir (degrees, seat frame), rocking on its
// flange's far edge until the pressed side has gone down dome_travel over its dome(s)
module caps(press = false, dpad_dir = 0, which = "all") {    // which: "all", "abxy", "ss" or "dpad"
  z0 = retainer_z - flange_t;
  dz = press ? -dome_travel : 0;
  if (which == "all" || which == "abxy") for (k = abxy_keys()) at(k[0], z0 + dz) rotate(k[1]) abxy_cap(k[2]);
  if (which == "all" || which == "ss") for (b = ss_buttons) at(b[2], z0 + ss_lift + dz) ss_cap();
  if (which == "all" || which == "dpad") for (c = [dpad1, dpad2]) at(c, z0 + dpad_lift) rotate(c == dpad2 ? 180 : 0) dpad_tilt(press ? dpad_dir : undef) dpad_cap();
}
module abxy_cap(notches) {
  difference() {                                        // the flange, less its keying notches
    cylinder(d = cap_flange_d, h = flange_t, $fn = 48);
    for (n = notches) rotate(n) translate([cap_flange_d / 2 - notch_reach - 0.05, -notch_w / 2 - 0.05, -1]) cube([2, notch_w + 0.1, 3]);
  }
  translate([0, -cap_tab_w / 2, 0]) cube([cap_tab_r, cap_tab_w, flange_t]);
  cylinder(d = cap_abxy_d, h = cap_h, $fn = 48);
  translate([0, 0, -cap_stem[1]]) cylinder(d = cap_stem[0], h = cap_stem[1] + 0.01, $fn = 32);   // its stem
}
module ss_cap() { cylinder(d = ss_flange_d, h = flange_t, $fn = 48); cylinder(d = ss_cap_d, h = ss_cap_h, $fn = 48); }
// the D-pad, its flange's underside at z 0; its pivot peg stops dpad_peg_gap over the board
// (ESTIMATE: dpad_peg_d, dpad_peg_gap)
module dpad_cap() {
  linear_extrude(flange_t) dpad_flange();
  linear_extrude(dpad_h) cross(dpad_arm_w, dpad_arm_l);
  translate([0, 0, z_top_cu + dpad_peg_gap - (retainer_z - flange_t + dpad_lift)])
    cylinder(d = dpad_peg_d, h = retainer_z - flange_t + dpad_lift - z_top_cu - dpad_peg_gap + 0.01, $fn = 32);
}
// a cardinal press rocks on the far arm's tip, a diagonal on the flange's far rim
module dpad_tilt(dir) if (is_undef(dir)) children(); else
  let(diag = dir % 90 != 0, back = diag ? dpad_flange_r : dpad_arm_l / 2,
      reach = diag ? d * cos(45) : d, a = atan(dome_travel / (back + reach)), u = [cos(dir), sin(dir)])
    translate([-back * u[0], -back * u[1], flange_t]) rotate(a = a, v = [-u[1], u[0], 0])
      translate([back * u[0], back * u[1], -flange_t]) children();
// The silicone pads, for pictures and fit checks only (nothing is printed from them). Alex measured
// his A/B/X/Y and D-pad pads (2026-10-20): outlines, web, rim, plungers, the centre square, the
// D-pad's hole and its collar's height, the domes' free height and the carbon nub. Still ESTIMATES:
// the D-pad collar's width, the dome skirts, the Start/Select pad and its wells, and how far a cap
// travels before its pill closes on the board (a DS Lite's stroke is about twice an original DS's
// tact switches: 0.5 to 1 mm, dossier section 2)
dome_travel = 1.0;
dpad_peg_gap = 0.2;                                     // the D-pad's peg over the board at rest
dpad_peg_d = 2.2;                                       // ESTIMATE: through the pad's 2.4 hole (Alex's pads,
                                                        // 2026-10-20; was 2.4, under Pi Tin's 2.5 hole)
sil_web_t = 0.65;  plunger_d = 4.0;                    // measured: the pad's web; each dome's flat top
dome_base_d = 6.0;                                      // ESTIMATE: each dome's skirt (HHL photo)
sil_rim = [0.4, 1.8];                                   // measured: the rim round each pad's outer edge: width, height
dpad_hole_d = 2.4;                                      // measured: the D-pad pad's centre hole, through its collar
dpad_collar = [3.3, 1.7];                               // measured: that collar's outside at its top (its base may be
                                                        // wider), and its height
abxy_boss = [4.9, 1.7];                                 // measured: A/B/X/Y centre square: side, top over the board
nub_d = 1.5;                                            // measured: each dome's carbon nub
// measured: each dome's free height, table to its top: D-pad 2.3, A/B/X/Y 2.8. The model still
// draws the domes at dome_h, where the flanges sit. Whether a real cap is hollow underneath (the
// plunger rises into it) or presses the dome down by the difference is open: check a cap's underside
dome_free = [2.3, 2.8];
// measured: each A/B/X/Y cap's stem under its flange, which presses its dome: diameter, length.
// At rest it reaches dome_h - 1.3 = 0.45 over the board, through a 2.8 dome drawn at dome_h; unless
// each dome's top has a recess the stem fits in (Alex is checking), the flanges must sit higher
cap_stem = [2.4, 1.3];
ss_well = [5.6, 0.5, 1.4];                              // Start/Select plunger wells: outside, wall, top over the board
// Start/Select close on metal snap domes since 2026-10-22 (row 4; SW20-SW23 on the board's
// Dome_4mm_Round): one per contact under a 0.06 Kapton dot, the pad's plunger over it. The plunger
// must rest clear of the dome's apex; how high it rests over the board in a real DS Lite strip
// (ss_plunger_gap) is unmeasured: 0.36 is the least that clears, and a strip made without carbon
// pills is probably made for domes. If a strip measures less, raise ss_lift by the difference (at
// most 0.15 with the lip as it is; up to 0.45 with a 0.3 Start/Select chamfer instead of cap_chamfer)
ss_metal_dome = [4.0, 0.31];                            // HYP 600-415S-260 + Kapton dot: diameter, apex over the board (worst case)
ss_plunger_gap = 0.36;                                  // ESTIMATE until Alex measures the Start/Select strip
module dome(h) { cylinder(d1 = dome_base_d, d2 = plunger_d + 0.6, h = h - 0.4, $fn = 40);
                 cylinder(d = plunger_d, h = h, $fn = 32); }
// a pad's rim, round its outer edge
module rim2d(c) difference() { pad2d(c); offset(delta = -sil_rim[0]) pad2d(c); }
// domes = false leaves the domes out: a pressed cap squashes its dome, so a check of pressed caps
// against the pads looks at everything else
module silicone(domes = true) {
  for (c = [abxy1, abxy2]) at(c, z_top_cu) {
    linear_extrude(sil_web_t) pad2d(c);
    linear_extrude(sil_rim[1]) rim2d(c);
    if (domes) for (t = [0:90:270]) rotate(t) translate([a, 0, 0]) dome(dome_h);
    difference() { translate([0, 0, abxy_boss[1] / 2]) cube([abxy_boss[0], abxy_boss[0], abxy_boss[1]], center = true);
                   translate([0, 0, -0.01]) cube([abxy_boss[0] - 1.2, abxy_boss[0] - 1.2, 2 * abxy_boss[1] - 1.2], center = true); }
  }
  for (c = [dpad1, dpad2]) at(c, z_top_cu) difference() {
    union() {
      linear_extrude(sil_web_t) pad2d(c);
      linear_extrude(sil_rim[1]) rim2d(c);
      cylinder(d = dpad_collar[0], h = dpad_collar[1], $fn = 48);
      if (domes) for (t = [0:90:270]) rotate(t) translate([d, 0, 0]) dome(dome_h);
    }
    translate([0, 0, -1]) cylinder(d = dpad_hole_d, h = 3, $fn = 32);
  }
  for (c = ss_centres) at(c, z_top_cu) linear_extrude(sil_web_t) rrect(ss_pad[0], ss_pad[1], 2);
  for (b = ss_buttons) at(b[2], z_top_cu) {
    if (domes) difference() {                           // the plunger, resting ss_plunger_gap over the board
      dome(dome_h);
      translate([0, 0, -0.01]) cylinder(d = ss_metal_dome[0] + 0.3, h = ss_plunger_gap + ss_lift - 0.35 + 0.01, $fn = 40);
    }
    if (domes) cylinder(d = ss_metal_dome[0], h = ss_metal_dome[1] - 0.05, $fn = 40);   // its metal dome, nominal
    linear_extrude(ss_well[2]) difference() { square(ss_well[0], center = true); square(ss_well[0] - 2 * ss_well[1], center = true); }
  }
}

if (part == "3d") {
  color("#3a3a44") tub();
  color("#c9c9d1") cell3d();
  color("#b8912e") speaker3d();
  board3d();
  color("#101014") translate([0, 0, z_glass]) linear_extrude(panel_t) square([panel_w, panel_l], center = true);
  color("#8f8f99") silicone();
  color("#4a4a56") translate([0, 0, explode]) top();
  color("#e8e8ee") caps();
}
if (part == "tub") tub();
if (part == "top") translate([0, 0, case_h]) rotate([180, 0, 0]) top();         // printed face down
if (part == "top_up") translate([0, 0, -z_bot]) top();                          // face up, its skirts' tips on the bed
