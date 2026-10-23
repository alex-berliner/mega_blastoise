// Reference design case: the single source of truth for every position the
// board depends on (outline, contacts, screw holes, connectors, height limits).
//
//   part="layout"   coloured top view
//   part="board"    board outline with its cable slots, 2D (DXF for Edge.Cuts)
//   part="3d"       assembly; explode=<mm> lifts the top piece and the decks
//   part="tub", "top", "deck"   printable parts (STL), each in its print pose
//
// The shell (2026-10-21, case/SHELL.md): a top piece (the screen face, the steps down to the
// decks, the sides) and a deck at each end (the button faces, which seat into the top piece from
// behind) curve over the sides and meet the tub, the back, flush at one seam low on the walls.
// Built upside down: caps and silicone into the decks, the decks into the top piece, the panel
// into its pocket, the board face down, the tub over it all, six screws from the back.
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
// EastRising ER-TFT024IPS-3 (ST7789V, 240 x 320 IPS), from its datasheet's outline drawing (page 6).
// The 2.4 in panel replaced the 3.5 in ER-TFT035IPS-6 (Alex, 2026-10-21): the case is 23.5 mm shorter
panel_w = 42.72;  panel_l = 59.46;  panel_t = 2.4;      // outline with the FPC folded; 2.3 +-0.1 thick: the thickest
tape_t = 0.15;                                          // double-sided tape between the top piece's skin and the panel
panel_fit = 0.8;   // the top piece's locating frame is this much larger: the panel is +-0.2, FDM openings print small
frame_wall = 1.1;  frame_h = 1.0;                       // that frame's walls, hanging from the top piece
aa_w = 36.72;     aa_l = 48.96;                         // active area
aa_to_top = 2.90;  aa_to_fpc = panel_l - aa_l - aa_to_top;   // active area to the glass edges (the drawing: 2.90 from
                                                        // the edge away from the FPC, centred across)
aa_off = (aa_to_fpc - aa_to_top) / 2;                   // active area sits 2.35 mm towards seat 2
window_margin = 1.0;                                    // bezel opening past the active area (it is then the panel's
                                                        // 38.72 x 50.96 visual area): covers the panel's float in its
                                                        // frame, its active-area tolerance and a printed opening's error
// Its FPC (50 contacts, 0.5 mm pitch) leaves the seat-1 edge, folds under the panel and plugs into a
// 50-pin ZIF connector under the panel (J3, Hirose FH12-50S, bottom contact). The drawing: 31.5 wide,
// its edge 6.0 from the panel's +x edge (seen from the front, seat 1 nearest), 27.0 long unfolded
// from the glass edge to its tip, the contacts 25.5 wide
fpc_w = 31.5;  fpc_tail_w = 25.5;  fpc_x = panel_w / 2 - 6.0 - fpc_w / 2;   // FPC centre, x
fpc_l = 27.0;
// Folded, the tip lands somewhere in tip_range in from the seat-1 glass edge (the fold isn't drawn:
// the schematic session's estimate, 2026-10-21). J3 isn't fixed yet, so the board keeps the whole
// range clear; tip_in is where the board places it for now
tip_range = [22.5, 25.0];                               // ESTIMATE
tip_in = (tip_range[0] + tip_range[1]) / 2;
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
// pushed to the hole's right end, where the charger's thermistor TH1 reads it through the gap
cell = [52, 36, 4.5];                                   // 403450, worst case: 50 +-2 by 34 +-2 by 4.0 +-0.5
cell_nom = [50, 34, 4.0];                               // and nominal (the dummies and pictures)
cell_hole = [[-28.7, -3.8], [25.3, 33.2]];  cell_hole_r = 1.0;   // the board's hole: corners, corner radius
cell_top = 1.6;                                         // its top over the top copper, worst case: the decks'
                                                        // band (deck_limit) allows 1.6 where its seat-2 edge reaches
cell_pocket = 0.6;                                      // the tub's floor pocketed under it: the case 0.6 thinner
cell_pocket_fit = 0.25;                                 // the pocket inside the hole's outline, each side
// The speaker (Alex, 2026-10-22): INGHAi's GSPK151035PN, a 15 x 10 mm racetrack 3.5 mm thick with
// its adhesive front foam ring, whose 35 mm leads end in a 1.25 mm 2-pin plug for J4. It lies face
// up on the board's top in the right bezel beside the screen, its long axis along the case, and fires
// up through a grille in the face; the board keeps parts out of its outline plus spk_keep (SPEAKER in
// placement.py). Its top stands 3.5 over the board, 3.8 worst case with tolerance and its tape, over
// the bezel's 3.45: the top piece's underside has a pocket over it. The side outlet, the floor chamber
// and the pad it lay on went with it (Alex, 2026-10-22: the side bezel, not the side wall)
spk = [10, 15, 3.5];                                    // across (x), along (y, the racetrack's length), height
spk_h_max = 3.8;                                        // its top over the board, worst case
spk_pos = [28.7, -11.4];                                // its centre and its cone's (placement.py SPEAKER; 2.1 up
                                                        // since the side screws came back inward, 2026-10-22)
spk_keep = 0.5;                                         // the board's keep-out round it, and the pocket's
spk_pocket = 0.5;                                       // the pocket's depth into the top piece's underside
grille_in = 1.5;                                        // the grille stays this far inside the speaker's outline:
                                                        // the rim round it is solid, for its front ring to seal on
grille_slot = [1.0, 1.8];                               // the grille's slots along the racetrack: width, pitch
// J4 (the speaker's plug, PicoBlade like J2, 3.4 tall) below the speaker against the right wall, its
// mouth towards the glass; the mated plug goes in from the left. A 0.3 pocket over both gives the plug
// room; the side screw H3 stands between the speaker and the plug, and the pocket goes round its boss
j4_body = [[28.75, -31.73], [35.65, -22.17]];           // its body's corners (centre 31.9, -27.0: CELLCUT_FOR_CASE.md)
j4_plug = [[24.75, -29.25], [28.75, -24.75]];           // the mated plug's (about)
j4_pocket = 0.3;  j4_pocket_grow = 0.25;                // the pocket's depth, and how far it is grown round them
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
dpad_gap = 0.65;          // D-pad opening past the cross each side: 0.5 (the faceplate's 18.80 x 6.90 opening) was
                          // too tight for Alex's AliExpress D-pad in the first print, 0.8 too loose in the next
                          // (Alex, 2026-10-21), so halfway (tune: tune_openings, 0.55 / 0.65 / 0.75).
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
glass_side = (case_w - bay_w) / 2;                      // panel bay edge to long case edge (16.6: USB-C, the
                                                        // speaker and the debug parts beside the glass)
edge_margin = 4;          // case end to silicone piece
edge_margin_x = edge_margin + 1;                        // and case side to silicone piece, 1 more since the case
                                                        // grew 2 mm wider round the same clusters (2026-10-22)
piece_to_glass = 5;       // silicone piece to glass edge
wall = 2.0;  board_clear = 0.5;
corner_r = 4.3;           // plan corners: a DS Lite's 4.3 to 4.4 (8 felt round and toy-like, 2026-10-19)
board_r = 5.5;            // the board's own corners (its KiCad outline): no longer follow the case's
// The flattest a face may slope downwards in its print pose, in degrees from vertical: no part needs
// supports. 45 until Alex's 6_tests ladder says how far each printer goes (one line, 2026-10-21).
// GUESS (Alex, 2026-10-21: best guess until 6_tests): 45, the conservative limit; test_overhangs on
// both printers settles it, and every round and the step to the decks follow it
overhang = 45;
// Every outside edge rounds over as far as that allows: an arc into the wall, and where the arc would
// go flatter than `overhang` in the print pose, a tangent chamfer at `overhang` (the tub prints back
// down, the top piece and the decks face down, so all their rounds end in a chamfer at the bed)
back_r = 4.2;             // the back rolls into the walls through back_r, as a DS Lite's does (4.2 mm)
face_r = 3.0;             // the top piece's and the decks' faces roll into the walls (Alex, 2026-10-21, item
                          // 10: the top pieces curve over the sides, the tub no longer forms the outer lip)
support_pitch = 12;       // the floor's support ribs under the board
// The seam: the top pieces (the top piece and both decks) meet the tub flush, low on the back's round.
// The tub's wall carries a tongue, the inner half of the wall standing up past the seam, and the top
// pieces' skirts come down outside it (their outer half to the seam): they sit flush with the tub,
// can't move inwards, and the seam has no straight path into the case. Six screws lock it all; the
// snap hooks went (2026-10-21: PLA hooks snapped)
// The joints (Alex, 2026-10-22, after printing this shell: the top pieces "need to flow better and
// interlock", "the tub has a tall inner wall that the top piece slides farther down", tabs on the tub
// wall into the top pieces, and the four parts must hold together before the screws, as the last
// shell did): the tongue reaches 7.2 under the top piece and 6.7 under the decks (it was 1.5 tall), and it
// carries the joints: tabs into pockets in the top piece, posts in the corners that push each deck
// onto its wedges, and grip ribs (case/SHELL.md, "The joints")
// The seam came down onto the back's round (Alex, 2026-10-22, after the joints: the top pieces' walls
// over most of the round, to a much lower ledge on the tub, the tub keeping its tongue): it was at the
// round's end (back_r, 4.2). The skirts follow the round down to it, their inner faces still straight
// down the tongue, so they slide straight on. 2.55 is as low as the skirts' tips stay skirt_t thick
// with the tongue where it is (the round comes 0.30 in at their tips, 2.65; at a 2.5 seam it would be
// 0.32 and the tips 0.68); beside the board's edge there is 0.25 to spare (asserted below)
z_seam = 2.55;
// Printed part sliding on printed part, each side: the tub's tongue inside the skirts, the tabs in
// their pockets. 0.15 was too tight for the old deck insert's ring round the lid's boss.
// GUESS (Alex, 2026-10-21: best guess until 6_tests): 0.25; test_rings (0.2 to 0.5) settles it
slide_fit = 0.25;
seam_fit = slide_fit;                                   // the tongue's clearance to the skirts (and the tabs')
tongue_top = 7.2;         // the tongue's top along the top piece's sides (0.35 under the top piece's underside),
deck_tongue_top = 6.7;    // and under the decks and across the step's band, where the faces are 1.85 lower (1.0
                          // under the decks' face): there the tongue leans in as the face's round comes in, and
                          // its tip is chamfered. It rises from the seam: 4.65 and 4.15 tall since the seam came
                          // down (3.0 and 2.5 over the seam at 4.2; 1.5 before "The joints")
tongue_h = tongue_top - z_seam;  deck_tongue_h = deck_tongue_top - z_seam;
skirt_t = 0.7;            // the skirts' outer half is never thinner than this, outside the tongue's groove
tongue_t = wall / 2 - seam_fit;                         // the tongue's thickness (0.75; its tip tapers)
tongue_lead = 0.3;        // the tip's outer edge chamfered 45 degrees, a lead-in into the skirts
seam_gap = 0.1;                                         // the skirts stop this short of the tub at the seam (and
                                                        // the grooves this far over the tongue's top), so the
                                                        // screws clamp the board, not the seam

// ---------------- stack (z) ----------------
floor_t = 1.5;  pcb_t = 1.6;  top_t = 2.0;               // top 2.0, not 1.5: the first print felt flimsy
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
// each end drops to a button deck where the caps stand as on a DS Lite, except a spine between the
// clusters (J2 or J4 sits under it), which stops short of the end so Start and Select and the end
// screw sit in the deck below it
cap_proud = cap_h - flange_t - lip_t;                   // above the deck: 1.41, Pi Tin's 1.3 (1.85 until 2026-10-21,
                                                        // over a 1.16 lip from scans of aftermarket shells)
deck_face = cap_top - cap_proud;                        // 3.90 over the board (the faceplate's face is 3.85 over it)
z_deck_under = deck_face - top_t;                       // the deck's underside
// The decks (Alex, 2026-10-21, items 14 and 15): each end's deck (both clusters and Start/Select) is
// its own part, as the inserts were, printed face down so neither it nor the top piece needs supports.
// But it no longer drops into the case from the front between the tub's walls and under a lip on the
// end wall: it reaches out to the case's outline and curves over the end and the sides itself, down to
// the seam, and seats into the top piece from behind. Its inner edges (along the step and round the
// spine) are a 45 degree wedge under a matching undercut in the top piece, so it goes in only from
// behind, centres itself and can't come out of the front; the tub's corner posts push it onto the
// wedges (2026-10-22), and its own screw (the end screw, through the board into a boss on the deck)
// holds it on the board. One part for both ends
step_y = bay_l / 2 + 2.4;                               // the step's foot: clear of the panel's frame and FPC fold
spine_w = 13.5;                                         // the spine's width at the deck
// The step from the screen face down to each deck (Alex, 2026-10-21, item 13: a curve, not the 45
// degree ridge): the same round as the outside edges, an arc of step_r that ends in a chamfer at
// `overhang` at the screen face (face down that's where it leaves the bed). It reaches the deck
// steep, a crease that hides the seam. When the overhang limit relaxes the arc runs further up
step_r = 3.0;
step_d = case_h - deck_face;                            // the step's height (1.85)
joint_land = 0.6;          // the top piece runs on flat at the deck's level past the step's foot, so its edge at the
                           // joint is square, not a knife edge (face down, a 0.6 ledge it prints without support)
joint_y = step_y + joint_land;                          // the joint between the top piece and a deck, across the case
deck_land = 0.8;          // the joint: straight down from the face, then the wedge
deck_gap = 0.15;          // the joint's clearance on its straight faces (the land, the skirts' ends at the side
                          // walls, the spine's sides; the inserts' 0.15, which slid on straight faces) and on
                          // the wedges along the spine's sides. The wedges along the step and round the
                          // spine's end have none: since 2026-10-22 the tub's corner posts push each deck
                          // onto them as the tub goes on, so the wedge is where the deck sits along the
                          // case, at its place over the board's buttons (0.25, slide_fit, from 2026-10-21,
                          // when nothing pushed the deck home)
step_wall_t = 1.35;                                     // the top piece's wall behind the undercut (room for the
                                                        // taller step of the 1.6 tuning height). It was 1.2 + deck_gap
                                                        // 0.15, but the gap is the deck's (its wedge stops deck_gap
                                                        // short of the undercut), not the band's: the wall is as it was
e0 = deck_face - deck_land;                             // the wedge's top edge
wedge_w = e0 - z_deck_under;                            // how far the undercut reaches in under the top piece (45
                                                        // degrees); the deck's wedge reaches wedge_w - deck_gap
joint_w = wedge_w + step_wall_t;                        // the band the joint takes inside the top piece's edge
// The step blends down the side walls (2026-10-22, Alex on the joint's outside: "These parts should flow
// into each other better"): across the face the step is as it was, but the side's round comes down from
// the screen face's height to the deck's over side_run before the joint, in a smooth S (no slope at
// either end), so the top piece's side round sweeps into the deck's instead of dropping in the step's 2.1.
// Over the round's top, between it and the screen face, a 45 degree chamfer (the round's own top
// chamfer, carried up): every face of it still slopes 45 degrees or steeper face down. side_run is as long
// as the top piece's keys allow (they sit under the full screen face, on the tongue's tall part)
side_run = 9.4;            // (to the land at the step's foot, joint_land before the joint: over the land it is the deck's)
side_fill = board_clear - 0.1;                          // (the top piece's underside under it: below)
function side_drop(d) = let(t = max(0, min(1, (joint_y - joint_land - d) / side_run))) step_d * (1 - t * t * (3 - 2 * t));   // the round's
                                                        // top under the screen face at |y| = d (0 to step_d)
// how far in from the joint the top piece's face is at depth dd under the screen face (0 at the deck)
function step_s(dd) = dd >= step_d - 1e-6 ? 0 : joint_land + edge_in(dd, step_r) - edge_in(step_d, step_r);
lay = 0.1;                                              // the rounds' and the step's layers, as they print
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
// clamping the board between the tub's bosses and the top pieces' (the first print's pins through the
// board's holes snapped, so they went; crush ribs centre the board and the screws line it up). Four
// go into the top piece's bosses beside the glass, one into each deck's boss between its clusters.
// The board goes in one way only: turned round, J1's receptacle hits the wall away from the USB-C
// opening, and the end bosses are off the centre line
// The side screws are the board's (placement.py SIDE_SCREWS; Alex, 2026-10-22: |y| <= 29, inside the
// top piece, no shoulders): H1 and H2 at the seat-2 ends of the strips beside the cell's hole, H3 in
// the right bezel between the speaker and J4, H4 in the left bezel between J2 and the LDO
side_pins = [[32.5, 29.0], [-32.5, 29.0], [25.9, -21.95], [-33.0, -15.2]];
pin_key = 3.0;            // the end bosses sit off the centre line (seat 1's board hole is there)
pin_out = 1.4;            // and this much further out than the clusters
// seat 2's end boss is seat 1's turned round (2026-10-20), so one deck fits both ends
pins = concat(side_pins, [[pin_key, seat_y + pin_out], [-pin_key, -seat_y - pin_out]]);
boss_d = 5.0;  hole_d = 2.2;
// The tub's bosses under the board (0.7 tall, board_lift) are 6.0, except at H2 and H4, where 6.0
// would come within 0.8 of the cell's hole (H2) and 0.2 of the board's edge (H4): there they are
// boss_d, 1.3 round the screw's 2.4 clearance (the head's counterbore is in the floor under them)
tub_boss_d = [6.0, boss_d, 6.0, boss_d, 6.0, 6.0];
// M2 x 6 self-tapping, wafer head (4 mm, 1 mm): M2 x 8 until the case got 3.8 mm thinner (2026-10-22),
// when an 8 would have run past the decks' pilots
screw_cb = [4.4, 1.0];  screw_clear = 2.4;  screw_pilot = 1.6;  screw_l = 6;
screw_engage = min(6.0, case_h - z_top_cu - 0.8);       // the pilot stops 0.8 under the face
deck_engage = deck_face - z_top_cu - 0.6;               // and 0.6 under the deck's face (the screw's tip 0.1 short)

// edge parts, all on the right (+X) long edge. J1 slid 2 mm up its wall (towards seat 2) when the side
// screws came back inward (2026-10-22, CELLCUT_FOR_CASE.md): its opening, tab, notch and slot follow usb_y
usb_y = 2.0; usb_w = 8.94;  usb_h = 3.25;               // TYPE-C-31-M-12 shell
usb_r = 1.0;                                            // its corner radius: a rounded rectangle with 1.2 mm flats
                                                        // on its ends, not a stadium (HRO's 3D model, 2026-10-20)
// The receptacle's face sits usb_recess under the case's surface, so a plug's overmold comes up to
// the case (a mated plug's stands about 0.25 off it: plug_len_min) and the opening only has to clear
// the receptacle's shell: the shell's shape (Alex,
// 2026-10-20: the stadium it used to be was "a different kind of oval"), usb_gap round it (Alex's fit
// test: the overmold-sized opening left a big gap round a port sunk 1.15 mm in the wall). To reach the
// surface the board grows a tab under the receptacle (its face stays 1.35 past the board's edge, as
// its footprint expects), which sits in a slot in the wall. The opening is split at the port's
// centre: the upper half in the top piece's skirt, the lower half in a tab of the tub that rises from
// the seam into a notch in the skirt (no roof over a hole, in either print pose). The seams round the
// tab step halfway through the wall (usb_step), so they don't run straight through it (Alex,
// 2026-10-20: light showed through them). The tab stays with the seam under the port (2026-10-22):
// the receptacle stands 0.85 into the skirt, and the board goes onto the face-down top piece straight
// down, so whatever lies under the receptacle's mouth in the wall can't be the top piece's
usb_recess = 0.15;  usb_gap = 0.25;
usb_hole = [usb_w + 2 * usb_gap, usb_h + 2 * usb_gap, usb_r + usb_gap];   // width, height, corner radius
usb_step = 1.0;                                         // the step in the tab's seams, inside the wall's outer half
usb_notch_w = 18.0;  usb_fit = 0.2;    // the notch in the top piece's skirt, and the tub's tab's clearance in it
usb_front = case_w / 2 - usb_recess;                    // receptacle face
usb_tab = usb_front - 1.35 - board_w / 2;              // the board's tab under it, past the board's edge
usb_tab_w = 12.0;  usb_tab_fit = 0.3;
// a USB-C plug (USB-IF Type-C compliance document rev 1.2): its shell 6.65 +-0.10 long from the
// overmold's front, the overmold 12.35 x 6.5 at most. Mated, its tip is usb_mate inside the
// receptacle's face (the receptacle's shell length, 6.20 in the Type-C spec)
plug_len_min = 6.55;  usb_mate = 6.20;
// left long edge: debug
pin_pos = [[-31.2, 8.6], [-31.2, 1.6]];                 // SW18 BOOTSEL, SW19 RESET under pinholes in the face
// J5 (SWD) and J6 (UART), JST SH, their mouths towards the left edge (the board's, 2026-10-22): plugged
// with the case open (the tub off), as the plugs stand past the board's edge; no slot in the wall
swd_pos = [-32.6, 15.2];  uart_pos = [-32.6, 22.7];
debug_h = 2.9;
// J2, the cell's plug (PicoBlade 53261, 3.4 tall), at the cell hole's seat-1 corner in the left
// bezel, its mouth into the hole's corner: the mated plug stands 3 mm out over the left strip and
// the hole's corner (placement.py), its leads down beside the cell's left end
j2_pos = [-30.7, -7.24];  j2_h = 3.4;
j2_plug = [[-32.8, -4.6], [-28.6, -1.6]];               // the mated plug's outline, corners
// the spines (between each seat's clusters, at the screen face's height) keep their length: they
// covered J2 and J4 until 2026-10-22; now seat 1's covers U6 (the backlight's op-amp, 1.75 tall, at
// y -36.4, over the decks' 1.6) and seat 2's only U9 (1.2), so they could be shorter (not done: the
// decks and positions.json stay as they are)
spine_end = 43.695 + joint_w;                           // the spine's end, its inside to 43.7

// ---------------- checks ----------------
// the distance from point p to the rectangle r = [[x0, y0], [x1, y1]] (0 inside it)
function rect_dist(p, r) = norm([max(0, min(r[0][0], r[1][0]) - p[0], p[0] - max(r[0][0], r[1][0])),
                                 max(0, min(r[0][1], r[1][1]) - p[1], p[1] - max(r[0][1], r[1][1]))]);
assert(cluster_spacing >= piece_d + 2, "a seat's two silicone pieces would touch");
assert(seat_y - piece_d / 2 >= bay_l / 2 + piece_to_glass - 1e-6, "cluster runs into the glass");
assert(cluster_spacing / 2 + piece_d / 2 <= board_w / 2, "silicone piece hangs off the board");
assert(glass_side >= 9.7, "no room for USB-C beside the glass");
// the cell in its hole: on the board, under the panel and the decks, and its pocket in the floor
assert(cell_hole[0][0] >= -board_w / 2 + 7 && cell_hole[1][0] <= board_w / 2 - 7, "the strips beside the cell's hole are too narrow");
assert(cell_hole[1][0] - cell_hole[0][0] >= cell[0] + 1.0 && cell_hole[1][1] - cell_hole[0][1] >= cell[1] + 1.0,
       "a worst-case cell doesn't fit its hole");
// (the fit kit's tuning slices render other silicone heights, which move the decks: tune skips the
// checks of the decks against the cell and the screws)
tune = false;
assert(tune || cell_top <= glass_gap - 0.4 && cell_top <= z_deck_under - z_top_cu - 0.3 + 1e-6, "the cell's top runs into the panel or a deck");
assert(cell[2] - cell_top >= pcb_t + 0.5, "the cell doesn't reach through the board to the floor");
assert(floor_t - cell_pocket >= 0.8, "the floor under the cell is too thin");
assert(board_lift >= 0.7 - 1e-6, "the board rests on ribs too low to print");
// the side screws' bosses (the tub's under the board, the top piece's on it) on the board, clear of
// the cell's hole and the board's edge
for (i = [0:3]) let(s = pins[i], b = max(tub_boss_d[i], boss_d)) {
  assert(rect_dist(s, cell_hole) >= b / 2 + 1.0, "a side screw's boss runs into the cell's hole");
  assert(abs(s[0]) + b / 2 <= board_w / 2 - 0.5, "a side screw's boss runs past the board's edge");
}
// the speaker in the right bezel, under its pocket, clear of the panel's frame, the cell and J4
assert(spk_pos[0] - spk[0] / 2 - spk_keep >= (panel_w + panel_fit) / 2 + frame_wall + 0.3, "the speaker's pocket runs into the panel frame");
assert(spk_pos[0] + spk[0] / 2 + spk_keep <= case_w / 2 - wall - 1.0, "the speaker's pocket runs into the side wall");
assert(z_ceil + spk_pocket - z_top_cu >= spk_h_max + 0.15, "no room for the speaker under the top piece");
assert(top_t - spk_pocket >= 1.2, "the face over the speaker is too thin");
// (since 2026-10-22 its seat-2 end is 0.1 short of the hole's edge: it lies on the board, and its
// pocket, over the ceiling, reaches 0.4 over the hole, far above the cell's top)
assert(spk_pos[1] + spk[1] / 2 <= cell_hole[0][1] + 1e-6, "the speaker doesn't lie on the board");
assert(spk_pos[1] - spk[1] / 2 - spk_keep >= j4_body[1][1], "the speaker runs into J4");
assert(spk_pos[0] - spk[0] / 2 + grille_in >= (panel_w + panel_fit) / 2 + frame_wall + 2.0, "the grille runs into the screen's bezel");
assert(j4_body[0][1] >= -(joint_y - joint_w), "J4 runs under the step's band");
assert(z_ceil + j4_pocket - z_top_cu >= j4_h + 0.3, "no room for J4 and its plug under the top piece");
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
// tall or more), and its upper half the top piece's: the receptacle (on the board, face down in the top
// piece) goes into the notch from behind, past nothing of the top piece
assert(usb_zc - usb_hole[1] / 2 - z_seam >= 0.8 && usb_zc - usb_step - 0.1 >= usb_zc - usb_hole[1] / 2 + 0.3,
       "the tub's USB-C tab under the opening is too thin");
assert(usb_front - (case_w / 2 - wall / 2) > 0 && z_seam < z_top_cu, "the USB-C notch isn't needed: drop the tab");
// J2, J5, J6 under the left bezel
assert(z_ceil - z_top_cu >= j2_h + 0.3 && z_ceil - z_top_cu >= debug_h + 0.3, "J2 or a debug connector touches the top piece");
assert(j2_plug[1][0] <= cell_hole[0][0] + 0.2, "J2's plug isn't at the hole's corner");
assert(swd_pos[1] - 3.5 >= pin_pos[0][1] + 2.4 + 0.3, "the BOOTSEL switch runs into the SWD connector");
assert(rect_dist(pin_pos[1], j2_plug) >= 2.4 + 0.3, "the RESET switch runs into J2's plug");
// the side screws in the top piece, beside the glass: clear of the panel's frame, inside the side
// walls, under the whole screen face (not the step), clear of the joint's band (the decks' wedges and
// plates start there) and of the speaker and J4 round H3
frame_rect = [[-(panel_w + panel_fit) / 2 - frame_wall, -(panel_l + panel_fit) / 2 - frame_wall],
              [(panel_w + panel_fit) / 2 + frame_wall, (panel_l + panel_fit) / 2 + frame_wall]];
function racetrack_dist(p, c, w, l) = norm(p - [c[0], max(c[1] - (l - w) / 2, min(c[1] + (l - w) / 2, p[1]))]) - w / 2;
for (s = side_pins) {
  assert(rect_dist(s, frame_rect) >= boss_d / 2 + 0.3, "a side boss runs into the panel frame");
  assert(abs(s[0]) + boss_d / 2 <= in_x - 1.0, "a side boss runs into the side wall");
  assert(abs(s[1]) + boss_d / 2 + 0.5 <= joint_y - step_in(0), "a side boss runs under the step");
  assert(abs(s[1]) + boss_d / 2 + 0.3 <= joint_y - joint_w + 1e-6, "a side boss runs into the joint's band");
  assert(racetrack_dist(s, spk_pos, spk[0] + 2 * spk_keep, spk[1] + 2 * spk_keep) >= boss_d / 2 + 0.3, "a side boss runs into the speaker's pocket");
  assert(rect_dist(s, j4_body) >= boss_d / 2 + 0.3 && rect_dist(s, j4_plug) >= boss_d / 2 + 0.3 - 1e-6, "a side boss runs into J4 or its plug");
}
for (p = pin_pos) for (q = pins) assert(norm(p - q) >= boss_d / 2 + 2.4 + 0.5, "a pinhole switch (SW18, SW19) runs into a screw boss");
// the screws: each tip short of its pilot's end
assert(tune || screw_cb[1] + screw_l <= z_top_cu + deck_engage && screw_cb[1] + screw_l <= z_top_cu + screw_engage, "the screws are longer than their pilots");
assert(z_top_cu + screw_engage <= case_h - 0.8 + 1e-6 && z_top_cu + deck_engage <= deck_face - 0.6 + 1e-6, "a pilot runs too near a face");
assert(panel_w + panel_fit <= bay_w && panel_l + panel_fit <= bay_l, "panel doesn't fit its bay");
assert(glass_gap >= 1.0, "no room under the panel");
assert(face_t >= 0.6 && face_t <= top_t, "face_t out of range");
assert(retainer_z < z_ceil, "button retainer above the top piece");
// the overhang limit and the rounds (case/SHELL.md): every round fits its wall
assert(overhang >= 30 && overhang <= 80, "overhang out of range");
// the board lies inside the tub's walls (the cell went into the board, 2026-10-22): its crush ribs
// reach past its top, its walls keep board_clear - tub_round_in off its edge
assert(z_top_cu + crush_reach <= z_seam + deck_tongue_h && z_board - 0.3 >= floor_t, "the tub's crush ribs miss the board's edge");
assert(tub_round_in <= board_clear - 0.3, "the tub's wall comes within 0.3 of the board");
// the seam on the back's round (2026-10-22): the skirts' tips (at z_bot, where the round has come in
// furthest over the skirts) skirt_t thick outside their grooves, at the top piece and the decks; and
// beside the board's edge, at every height the skirts cover, the skirt, its fit, the tongue and 0.25
// to the board inside the round (the round comes in furthest at the skirts' tips)
assert(z_seam <= back_r && z_seam >= back_r * (1 - sin(overhang)), "the seam isn't on the back's round");
for (H = [case_h, deck_face]) assert(groove_in(z_bot, H) - back_in(z_bot) >= skirt_t - 1e-6, "a skirt's tip is too thin");
assert(wall + board_clear - back_in(z_bot) >= skirt_t + seam_fit + tongue_t + 0.25, "no room beside the board's edge for a skirt and the tongue");
// The joints (2026-10-22, case/SHELL.md "The joints"; tune skips them: its slices move the decks' face).
// The tongue: as tall as Alex asked (at least 2.5, about 3), its tip whole, 0.2 or more from the
// board's edge (it passes the edge as the tub goes on), and the groove over it leaves the skirts
// skirt_t (by groove_in), the decks' plates and the screen face's bezel at least 0.8, and the top
// piece's underside 0.2
function inner_max(H, z0, zt) = max([for (l = tongue_levels(H, z0, zt)) l[2]]);
function tip_min(H, z0, zt) = min([for (l = tongue_levels(H, z0, zt)) l[2] - l[1]]);
if (!tune) {
  assert(tongue_h >= 2.5 && deck_tongue_h >= 2.5, "the tongue is shorter than the 2.5 Alex asked for");
  assert(tip_min(case_h, z_seam, z_tongue) >= 0.4 && tip_min(deck_face, z_seam, z_deck_tongue) >= 0.25 && tip_min(case_h, z_seam, z_key) >= 0.25,
         "the tongue's (or a tab's) tip is too thin");
  assert(tongue_in(z_deck_tongue - 0.4, deck_face, z_deck_tongue) - tongue_out(z_deck_tongue - 0.4, deck_face, z_deck_tongue) >= 0.6,
         "the tongue is too thin 0.4 under its tip");
  assert(max(inner_max(case_h, z_seam, z_tongue), inner_max(deck_face, z_seam, z_deck_tongue), inner_max(case_h, z_seam, z_key))
         <= wall + board_clear - 0.2, "the tongue comes within 0.2 of the board's edge");
  assert(deck_face - (z_deck_tongue + seam_gap) >= 0.8, "the decks' plates over the tongue are too thin");
  assert(z_tongue + seam_gap <= z_ceil - 0.2, "the groove runs into the top piece's underside");
  assert(tongue_y + tongue_y_fit <= joint_y - joint_land - side_run + 1e-6, "the tongue's tall part runs under the side's blend");
  assert(z_deck_tongue <= z_tongue && deck_face - face_r < z_deck_tongue, "the tongue's heights are the wrong way round");
  // the top piece's keys: in the top piece's sides (under the screen face, clear of where the tongue steps
  // down), the bezel over their pockets 0.8, clear of the USB-C notch, the side screws' bosses, the
  // speaker's and J4's pockets and the pinholes' guides
  assert(case_h - (z_key + seam_gap) >= 0.8, "the screen face over a key's pocket is too thin");
  assert(key_lead < key_w / 2 && key_lead <= key_h, "a key's lead-in is out of range");
  for (k = keys) let(r = [[k[0] > 0 ? key_x_in : -case_w, k[1] - key_w / 2 - seam_fit], [k[0] > 0 ? case_w : -key_x_in, k[1] + key_w / 2 + seam_fit]]) {
    assert(abs(k[1]) + key_w / 2 + seam_fit <= tongue_y - 1.0, "a key runs into the tongue's step");
    assert(k[0] < 0 || abs(k[1] - usb_y) >= usb_notch_w / 2 + key_w / 2 + seam_fit + 1.0, "a key runs into the USB-C notch");
    for (s = side_pins) assert(rect_dist(s, r) >= boss_d / 2, "a key's pocket runs into a side screw's boss");
    assert(rect_dist(spk_pos, r) >= spk[1] / 2 + spk_keep || abs(k[0] * case_w / 2 - spk_pos[0]) - (case_w / 2 - key_x_in) >= spk[0] / 2 + spk_keep,
           "a key's pocket runs into the speaker's pocket");
    for (j = [j4_body, j4_plug]) assert(rect_dist((j[0] + j[1]) / 2, r) >= norm(j[1] - j[0]) / 2 + j4_pocket_grow + 0.01 || k[0] < 0 ||
           min(r[1][1], j[1][1] + j4_pocket_grow) < max(r[0][1], j[0][1] - j4_pocket_grow) || j[1][0] + j4_pocket_grow <= key_x_in,
           "a key's pocket runs into J4's pocket");
    for (p = pin_pos) assert(rect_dist(p, r) >= 1.5 + 0.3, "a key's pocket runs into a pinhole's guide");
  }
  // the decks' corner posts: 0.3 outside the board's corner (the board's straight edges stop at its corners'
  // centres, so the corner is the nearest), inside the deck's plate where no pad or Start/Select is, their
  // lead-in longer than the push
  assert(norm(post_c - [board_w / 2 - board_r, board_l / 2 - board_r]) - board_r >= 0.3 - 1e-6 && post_c[0] > board_w / 2 - board_r && post_c[1] > board_l / 2 - board_r,
         "a corner post comes within 0.3 of the board");
  assert(post_c[0] < in_x - 1.0 && post_c[1] < in_y - 1.0, "a corner post is too small to locate a deck");
  for (c = clusters) assert(norm([abs(c[0]), abs(c[1])] - (post_c - [seam_fit, 0])) >= abxy_pitch / 2 + abxy_lobe_r + 0.5, "a corner post's notch runs into a silicone pad");
  assert(post_c[0] - seam_fit - (ss_pitch + ss_cell + 2 * ss_frame_w) / 2 >= 1.0, "a corner post's notch runs into Start/Select's frame");
  for (p = [pins[4], pins[5]]) assert(norm([abs(p[0]), abs(p[1])] - post_c) >= boss_d / 2 + 1.0, "a corner post's notch runs into an end boss");
  assert(post_lead >= deck_gap + 0.2, "a corner post's lead-in can't catch a deck that has slid down its wedge");
  // the grip ribs: on the tongue's straight part, 0.05 short of the skirt, each on its height of tongue,
  // clear of the laps' slots
  for (g = grips) assert(grip_top(g[2]) - z_seam >= 1.5, "a grip rib is too short");
  for (g = grips) let(w = len(g) > 3 ? g[3] : grip_w) {
    assert(g[2] == case_h ? abs(g[1]) + w / 2 <= tongue_y - 0.3 : abs(g[1]) - w / 2 >= tongue_y + 0.3, "a grip rib is on the wrong height of tongue");
    assert(abs(g[1]) + w / 2 <= lap_slot[0] - 0.5 || abs(g[1]) - w / 2 >= lap_slot[1] + 0.5, "a grip rib runs into a lap's slot");
  }
  // the side joints' laps (2026-10-22): 0.8 thick or more behind the top piece's skirt (deck_gap off it,
  // seam_fit inside the groove's inner side, which the tongue's asserts keep 0.2 off the board), their
  // free ends at least 0.8 tall, all of them behind the top piece's skirt (in its groove, under its top)
  // and on the decks' height of tongue, and their slots in the tongue clear of the crush ribs, the keys
  // and the corner posts
  assert(tongue_in(z_bot, deck_face, z_deck_tongue) - (groove_in(z_bot, deck_face) + deck_gap) >= 0.8, "a lap is too thin");
  assert(lap_top(joint_y - lap_l) - z_bot >= 0.8, "a lap's free end is too short");
  assert(lap_top(lap_y0) <= z_deck_tongue && joint_y - lap_l - seam_fit >= tongue_y + tongue_y_fit + 0.5, "a lap isn't in the groove of the decks' height");
  for (c = crush) if (c[2] == 0 || c[2] == 180) assert(abs(c[1]) + 0.6 + crush_reach <= lap_slot[0] - 0.3, "a lap's slot runs into a crush rib");
  for (k = keys) assert(abs(k[1]) + key_w / 2 <= lap_slot[0] - 1.0, "a lap's slot runs into a key");
  assert(lap_slot[1] + 1.0 <= post_c[1], "a lap's slot runs into a corner post");
  assert(lap_slot[1] <= joint_y + joint_w + 2.0, "a lap's slot is longer than it needs to be");
  // the side's blend (2026-10-22): the keys' pockets under the full screen face (the tongue's assert
  // above), the face's chamfer over the underside where the blend has come down furthest over the hollow
  // (the band's edge, joint_y - joint_w) at least 0.7 off its corner, the underside's fill outside the
  // board's edge, and clear of the side screws' bosses
  assert(side_fill <= board_clear - 0.1, "the underside's fill under the blend comes within 0.1 of the board's edge");
  assert((case_h - side_drop(joint_y - joint_w) + (wall + side_fill - edge_in(0, face_r)) - z_ceil) / sqrt(2) >= 0.7, "the face over the blend's underside is too thin");
  for (s = side_pins) assert(abs(s[0]) + boss_d / 2 <= case_w / 2 - edge_in(0, face_r) - step_d || abs(s[1]) + boss_d / 2 <= joint_y - joint_land - side_run,
                             "a side screw's boss is under the blend's chamfer");
  assert(grip_reach > 0 && grip_reach < seam_fit, "a grip rib reaches the skirt");
}
assert(step_s(0) <= joint_w - 0.2, "the step runs past the joint's wall: the screen face over it would be too thin");
assert(wedge_w - deck_gap >= 0.8 && deck_land >= 0.6, "the joint's wedge is too small to hold a deck");
// The button stack's design rules (check_buttons.py checks the geometry itself: the pads clear
// the shell, every cap sits and presses without touching it, at rest the caps only touch the domes)
assert(sil_rib_gap >= sil_web_t, "the ribs crush the silicone pads' web");
assert(retainer_bot > z_top_cu + sil_web_t, "the deck reaches down onto the pads");
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
assert(joint_y - joint_w > (panel_l + panel_fit) / 2 + frame_wall + 0.3, "the step's wall runs into the panel frame");
for (c = clusters) assert(abs(c[1]) - piece_d / 2 >= joint_y + 0.5 || abs(c[0]) - piece_d / 2 >= spine_w / 2 + 0.5,
                          "a silicone piece runs under the top piece");
// the openings and their chamfers lie on the deck's face, clear of the joint
for (b = buttons) assert(abs(b[2][1]) - cap_hole_d / 2 - cap_chamfer >= joint_y + deck_gap + 1.0, "a cap's opening runs into the step");
for (c = [dpad1, dpad2]) assert(abs(c[1]) - dpad_arm_l / 2 - dpad_gap - dpad_chamfer >= joint_y + deck_gap + 1.0, "the D-pad's opening runs into the step");
for (s = pins) for (c = clusters) assert(norm(s - c) >= piece_d / 2 + boss_d / 2, "screw boss under a silicone piece");
assert(seat_y + pin_out - boss_d / 2 >= spine_end + 0.5, "an end boss runs under the spine");
assert(seat_y + pin_out + boss_d / 2 + 0.3 <= ss_y - (ss_cell + 2 * ss_frame_w) / 2, "an end boss runs into Start/Select's frame");
assert(pins[4] == rot180(pins[5]), "the end bosses aren't each other turned round: the two decks would differ");
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
  "\"glass\": [", jpt([-bay_w / 2, bay_l / 2]), ", ", jpt([bay_w / 2, -bay_l / 2]), "], ",
  "\"glass_limit\": ", glass_gap - 0.2, ", ",
  "\"bezel_limit\": ", jnum(z_ceil - z_top_cu - 0.3), ", ",
  "\"retainer_limit\": ", jnum(retainer_bot - z_top_cu - 0.3), ", ",
  "\"fpc_zones\": [[", jpt([fpc_x - fpc_w / 2 - 0.5, fpc_y(tip_range[1] + 0.5)]), ", ",
    jpt([fpc_x + fpc_w / 2 + 0.5, fpc_y(-1.5)]), "]], ",
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
  "\"swd\": ", jpt(swd_pos), ", \"uart\": ", jpt(uart_pos), ", ",
  "\"cell_hole\": [", jpt([cell_hole[0][0], cell_hole[1][1]]), ", ", jpt([cell_hole[1][0], cell_hole[0][1]]), "], \"cell_top\": ", cell_top, ", ",
  "\"back_limit\": ", jnum(board_lift - 0.2), ", ",
  "\"speaker\": {\"c\": ", jpt(spk_pos), ", \"size\": [", spk[0], ", ", spk[1], "], \"keep\": ", spk_keep,
    ", \"limit\": ", jnum(z_ceil + spk_pocket - z_top_cu - 0.15), "}, ",
  "\"crush\": [", join([for (c = crush) jpt(c)]), "], ",
  "\"frame\": {\"outer\": [", jpt([-(panel_w + panel_fit) / 2 - frame_wall, (panel_l + panel_fit) / 2 + frame_wall]), ", ",
    jpt([(panel_w + panel_fit) / 2 + frame_wall, -(panel_l + panel_fit) / 2 - frame_wall]), "], \"inner\": [",
    jpt([-(panel_w + panel_fit) / 2, (panel_l + panel_fit) / 2]), ", ", jpt([(panel_w + panel_fit) / 2, -(panel_l + panel_fit) / 2]),
    "], \"limit\": ", jnum(z_ceil - z_top_cu - frame_h - 0.3), "}, ",
  "\"decks\": [", join(concat([for (sy = [-1, 1], sx = [-1, 1]) str("[", jpt([sx * (spine_w / 2 - joint_w), sy * (joint_y - joint_w)]), ", ",
    jpt([sx * case_w / 2, sy * case_l / 2]), "]")], [for (sy = [-1, 1]) str("[", jpt([-spine_w / 2, sy * (spine_end - joint_w)]), ", ",
    jpt([spine_w / 2, sy * case_l / 2]), "]")])), "], \"deck_limit\": ", jnum(z_deck_under - z_top_cu - 0.3),
  "}"));
// the USB-C opening for check_fit.py (KiCad x, y; z over the board's top), not in positions.json
echo(str("USBC {\"hole_y\": [", jnum(kicad([0, usb_y + usb_hole[0] / 2])[1]), ", ", jnum(kicad([0, usb_y - usb_hole[0] / 2])[1]),
  "], \"hole_z\": [", jnum(usb_zc - usb_hole[1] / 2 - z_top_cu), ", ", jnum(usb_zc + usb_hole[1] / 2 - z_top_cu),
  "], \"wall_x\": [", jnum(kicad([in_x, 0])[0]), ", ", jnum(kicad([case_w / 2, 0])[0]),
  "], \"plug_len_min\": ", plug_len_min, ", \"mate\": ", usb_mate, "}"));

// the decks' flange for check_fit.py (KiCad corners; limits over the board's top), not in positions.json:
// its plan at both ends grown part_clear (the tongue is inside it) and, where the board's taller parts are
// under it, the keepouts it stops short over
function jrect(r) = str("[", jpt([r[0][0], r[1][1]]), ", ", jpt([r[1][0], r[0][1]]), "]");
echo(str("FLANGE {\"limit\": ", jnum(z_flange - z_top_cu - part_clear), ", \"zones\": [",
  join([for (z = concat(flange_zones, [for (z = flange_zones) rot_rect(z)])) jrect([z[0] - [part_clear, part_clear], z[1] + [part_clear, part_clear]])]), "], \"keep\": [",
  join([for (k = flange_keep, t = [0, 1]) let(b = keep_box(k)) str("{\"ref\": \"", k[0], "\", \"box\": ", jrect(t ? rot_rect(b) : b), ", \"limit\": ", k[3], "}")]), "]}"));
for (b = buttons) echo(str(b[0], "  ", b[1], "   KiCad (", kicad(b[2])[0], ", ", kicad(b[2])[1], ")"));
echo(str("CAPS flange top ", retainer_z - z_top_cu, " above the board, cap top ", cap_top - z_top_cu, ", deck face ",
         deck_face - z_top_cu, ", screen face ", case_h - z_top_cu, ": ends ", deck_face, " mm thick, a ", step_d, " mm step"));
echo(str("STACK floor ", floor_t, " + ribs ", board_lift, " + PCB ", pcb_t, " + under the panel ", glass_gap,
         " + panel ", panel_t, " + tape ", tape_t, " + top over the panel ", face_t, " = ", case_h, " mm; top piece's underside ",
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
// The top piece's plan up to the joint: the screen block across the case (past its sides: intersect
// with an outline) and a spine at each end; the decks are the rest
module top_region2d() union() {
  square([4 * case_w, 2 * joint_y], center = true);
  square([spine_w, 2 * spine_end], center = true);
}
module half2d(sy) scale([1, sy]) translate([-2 * case_w, 0]) square([4 * case_w, case_l]);
// a deck's plan (sy: -1 seat 1's end, 1 seat 2's), past the case outline: intersect with one
module deck_region2d(sy) difference() { half2d(sy); offset(delta = deck_gap) top_region2d(); }
// the top piece's plan grown dx across the case and dy along it (negative: shrunk), as offset(delta) would
module top_region_xy2d(dx, dy) union() {
  square([4 * case_w + 2 * dx, 2 * (joint_y + dy)], center = true);
  square([spine_w + 2 * dx, 2 * (spine_end + dy)], center = true);
}
// the decks, both ends, as the board's height limits see them (no clearance)
module deck2d() difference() { square([4 * case_w, 4 * case_l], center = true); top_region2d(); }
module outline2d() rrect(case_w, case_l, corner_r);
module inner2d(grow = 0) offset(delta = -wall + grow) outline2d();
// how far in from an edge's wall line a round is at depth dd under the face it rolls into: an arc of
// radius r into the wall, and where the arc would be flatter than `overhang` from vertical, its
// tangent chamfer at `overhang` (as the part prints, the chamfer is the part nearest the bed)
function edge_in(dd, r) = let(s = sin(overhang), c = cos(overhang))
  dd >= r ? 0 : dd >= r * (1 - s) ? r - sqrt(r * r - (r - dd) * (r - dd)) : r * (1 - c) + (r * (1 - s) - dd) * tan(overhang);
// how far in from the case's outline the back's round (and chamfer) has come at height z
function back_in(z) = edge_in(z, back_r);
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
// ribs were on the top piece's walls and the decks' (the board was over the seam)
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
// wall whose face is at H (case_h along the top piece's sides; deck_face round the decks and across
// the step's band) under a tongue whose top is zt. The groove's outer side (the skirt's inside) is the
// wall's middle until the face's round comes within skirt_t of it, then follows the round in; the
// tongue keeps seam_fit inside it, tongue_t thick, its tip chamfered: its outer edge (tongue_lead, a
// lead-in into the skirt) and its inner edge at 45 degrees from the wall's inside (the tongue passes
// the board's edge as the tub goes on: the chamfer leads the board in, and the tongue never comes
// nearer it than the wall does). The groove's inner side keeps seam_fit inside the tongue (it matters
// in the decks' plates and the step's band, where the parts are solid there)
function groove_in(z, H) = max(wall / 2, edge_in(H - z, face_r) + skirt_t);
function tongue_out(z, H, zt) = max(groove_in(z, H) + seam_fit, wall / 2 + seam_fit + max(0, z - (zt - tongue_lead)));
function tongue_in(z, H, zt) = min(groove_in(z, H) + seam_fit + tongue_t, wall + (zt - z));
function groove_inner(z, H, zt) = tongue_in(min(z, zt), H, zt) + seam_fit;
// [z, outer inset, inner inset] every 0.05 from z0 up to zt: the tongue's, and the groove's (to zt + seam_gap)
function tongue_levels(H, z0, zt) = [for (k = [0:ceil((zt - z0) / 0.05 - 1e-6)]) let(z = min(zt, z0 + 0.05 * k))
  [z, tongue_out(z, H, zt), tongue_in(z, H, zt)]];
function groove_levels(H, zt, z0 = z_seam - 0.987) = [for (k = [0:ceil((zt + seam_gap - z0) / 0.05 - 1e-6)])
  let(z = min(zt + seam_gap, z0 + 0.05 * k)) [z, groove_in(z, H), groove_inner(z, H, zt)]];
// (the grooves' levels start off the parts' planes: a level on the top piece's underside left
// non-manifold edges)
// a ring round the case between two insets, lofted through [z, outer, inner] levels (the inner loft runs
// a little past both ends, so no face of the two coincides); shrink: both faces that much into the ring,
// for a solid that overlaps another's faces (eps: touching solids leave non-manifold edges)
module ring_loft(levels, shrink = 0) let(n = len(levels) - 1) difference() {
  rr_loft([for (l = levels) [l[0], l[1] + shrink]]);
  rr_loft(concat([[levels[0][0] - 0.01, levels[0][2] - shrink]], [for (l = levels) [l[0], l[2] - shrink]], [[levels[n][0] + 0.01, levels[n][2] - shrink]]));
}
// The tongue is tongue_h tall along the top piece's sides and deck_tongue_h from where the side's blend
// starts (tongue_y) round the ends: under the blend the top piece's side comes down towards the deck's
// round, so the tongue there is the decks'. The skirts' grooves overlap that step tongue_y_fit each way,
// as the tub can shift along the case by its keys' fit (until 2026-10-22's side joint it stepped where
// the step's band starts, joint_y - joint_w)
tongue_y_fit = seam_fit + 0.1;
tongue_y = joint_y - joint_land - side_run - tongue_y_fit;
module yband(y0, y1, z0 = -1, z1 = 20) translate([-2 * case_w, y0, z0]) cube([4 * case_w, y1 - y0, z1 - z0]);
module y_in(y) yband(-y, y);                            // |y| <= y
module y_out(y) { yband(y, case_l); yband(-case_l, -y); }   // |y| >= y
z_tongue = z_seam + tongue_h;  z_deck_tongue = z_seam + deck_tongue_h;
// Where it is the wall's inner half (all round up to z_tongue_straight, along the top piece's sides up to
// its tip's chamfer) it is cut from the plan's own offsets, as the USB-C tab and the crush ribs that meet
// it are; the leaning and chamfered parts are lofts, eps inside it where they overlap
z_tongue_straight = max([for (k = [0:800]) let(z = z_seam + 0.01 * k)
  if (z <= z_deck_tongue && tongue_out(z, deck_face, z_deck_tongue) <= wall / 2 + seam_fit + 1e-6) z]);
module tongue_ring2d() difference() { inner2d(wall / 2 - seam_fit); inner2d(); }
module tongue() {
  translate([0, 0, z_seam - eps]) linear_extrude(z_tongue_straight - z_seam + eps) tongue_ring2d();
  // under the round's end, where the board's edge is, the tongue reaches in to the cavity's wall below
  // the seam (tub_round_in), 0.3 off the board, so the tub's top 1.65 isn't only the tongue's 0.75
  if (z_seam < back_r) translate([0, 0, z_seam - eps]) linear_extrude(back_r - z_seam + eps)
    difference() { inner2d(wall / 2 - seam_fit); inner2d(-tub_round_in); }
  intersection() {
    union() {
      translate([0, 0, z_tongue_straight - 0.13]) linear_extrude(z_tongue - tongue_lead - z_tongue_straight + 0.13) tongue_ring2d();
      ring_loft(tongue_levels(case_h, z_tongue - tongue_lead - 0.05, z_tongue), eps);
    }
    y_in(tongue_y + eps);
  }
  intersection() { ring_loft(tongue_levels(deck_face, z_tongue_straight - 0.13, z_deck_tongue), eps); y_out(tongue_y); }
}
module groove(top = true) {                             // top: the top piece's (both heights), else a deck's
  if (top) intersection() { ring_loft(groove_levels(case_h, z_tongue)); y_in(tongue_y + tongue_y_fit); }
  intersection() { ring_loft(groove_levels(deck_face, z_deck_tongue)); if (top) y_out(tongue_y - tongue_y_fit); }
}
// The keys into the top piece: tabs that carry the tongue key_h higher, key_w along the wall, into
// pockets in the top piece's sides (seam_fit round them along the wall, its groove's profile across
// it), their tips' corners chamfered as lead-ins. [side, y] each, two a side, clear of the USB-C, the
// screws' bosses, J4's and the speaker's pockets. The top piece's skirt over the USB-C notch keys it too
// (the left two were at 22 and -24 until the side's blend, 2026-10-22: they sit under the full screen
// face, the tongue's tall part, which now ends where the blend starts)
keys = [[1, 18], [1, -16], [-1, 20], [-1, -20]];
key_w = 5.0;  key_h = 1.3;  key_lead = 0.5;
z_key = z_tongue + key_h;
module key_span(k, w, lead = 0, z0 = z_seam - 1, z1 = case_h) translate([k[0] > 0 ? 0 : -case_w, k[1], 0]) rotate([90, 0, 90])
  linear_extrude(case_w) polygon([[-w / 2, z0], [w / 2, z0], [w / 2, z1 - lead], [w / 2 - lead, z1], [-w / 2 + lead, z1], [-w / 2, z1 - lead]]);
module key_tab(k) intersection() { ring_loft(tongue_levels(case_h, z_seam - 0.01, z_key), eps); key_span(k, key_w, key_lead, z1 = z_key); }
module key_pocket(k) intersection() { ring_loft(groove_levels(case_h, z_key)); key_span(k, key_w + 2 * seam_fit); }
key_x_in = case_w / 2 - (max([for (l = groove_levels(case_h, z_key)) l[2]]) + 0.3);   // a pocket's inner side, less 0.3 (the checks)
// The decks' keys: a post in each of the tub's corners, outside the board's corner (board_r is larger
// than corner_r, so there is room), up to the tongue's top, in a notch in the deck's plate. Its inner
// faces locate the deck: across the case with seam_fit, and along it with none: it pushes the deck
// towards the screen onto its wedges as the tub goes on (the 45 degree lead-in on its tip catches a
// deck that has slid back down its wedge, by up to post_lead), and holds it there
post_c = [board_w / 2 - board_r, board_l / 2 - board_r] + (board_r + 0.3) / sqrt(2) * [1, 1];   // its inner corner
post_lead = 0.4;
module post(sx, sy) scale([sx, sy, 1]) intersection() {
  hull() {
    translate([post_c[0], post_c[1], floor_t - 0.01]) cube([case_w, case_l, z_deck_tongue - post_lead - floor_t + 0.01]);
    translate([post_c[0] + post_lead, post_c[1] + post_lead, z_deck_tongue - post_lead]) cube([case_w, case_l, post_lead]);
  }
  rr_loft([for (l = tongue_levels(deck_face, floor_t - 0.01, z_deck_tongue)) [l[0], l[1] + eps]]);   // inside the tongue's outside
}
module post_notch(sx, sy) scale([sx, sy, 1]) intersection() {
  translate([post_c[0] - seam_fit, post_c[1], z_seam - 1]) cube([case_w, case_l, z_deck_tongue + seam_gap - z_seam + 1]);
  rr_loft([for (l = groove_levels(deck_face, z_deck_tongue)) [l[0], l[1]]]);                    // inside the groove's outside
}
// Grip ribs on the tongue's outside, standing grip_reach off it: 0.05 short of the skirt, as the board's
// crush ribs are of the board, so a printed skirt (an inside face, which prints small) grips them. They
// hold each part on the tub until the screws go in and centre it across the case, so the parts' sides
// line up. Two on each side of the top piece (under the blend, on the decks' height of tongue) and two
// on each side of each deck. The one across each side joint (2.0 wide, pressing both skirts' ends out to
// one line) went with the lap there (2026-10-22): the lap holds the two skirts flush, and the tongue
// stops either side of it. [side, y, the face's height, width]
grip_reach = seam_fit - 0.05;  grip_w = 1.0;                 // GUESS (2026-10-22): the first print of these joints
                                                        // settles it (case/SHELL.md, "Best guesses")
grips = concat([for (sx = [-1, 1], sy = [-1, 1]) [sx, sy * 26, deck_face]],
               [for (sx = [-1, 1], sy = [-1, 1]) [sx, sy * 40, deck_face]],
               [for (sx = [-1, 1], sy = [-1, 1]) [sx, sy * 52, deck_face]]);
// The side joint (2026-10-22, Alex on the joint's outside: "These parts should flow into each other
// better"): a lap. At each side wall the deck's skirt carries a tongue of its own (the lap) lap_l along
// the wall, behind the top piece's skirt end, in the skirts' groove: deck_gap behind the top piece's
// skirt, out to the tub's tongue's inside, lap_root along the deck's own skirt (the groove filled there,
// so the lap is all one with the skirt). The deck drops onto its wedges from behind and the lap slides up
// the groove with it; the tub's tongue stops either side of it (lap_slot), as the tongue would otherwise
// fill the groove. Outside there is only the joint's line, and behind it the lap: the two skirts can't
// move in or out of each other. Face down the lap's free end hangs off the deck, so its top (the
// print's underside) rises from its root at 45 degrees (lap_top)
lap_l = 3.0;  lap_root = 1.5;
lap_y0 = joint_y + deck_gap;                            // the deck's skirt's end, where the lap leaves it
function lap_top(d) = z_deck_tongue - 0.025 - max(0, lap_y0 - d);   // the lap's top at |y| = d (0.025 off the
                                                        // lofts' levels: a corner on one left slivers)
// the tongue's slot round the lap, along the wall: seam_fit off its free end, and off its root seam_fit
// and the most a deck can lie back from its wedges before the corner posts push it home (post_lead)
lap_slot = [joint_y - lap_l - seam_fit, lap_y0 + lap_root + seam_fit + post_lead];
// (its inside eps short of the tongue's: on the wall's inner line, where the plan's other cuts lie, it left
// non-manifold edges in a fit kit render)
function lap_levels(outer) = [for (k = [0:ceil((z_deck_tongue - z_bot) / 0.05 - 1e-6)]) let(z = min(z_deck_tongue, z_bot + 0.05 * k))
  [z, outer(z), tongue_in(z, deck_face, z_deck_tongue) - eps]];
module lap(sx, sy) let(top_z = z_deck_tongue + seam_gap + eps) intersection() {
  // the groove filled, into the skirt and up into the plate (eps into both), less, in front of the
  // skirt's end, deck_gap behind the top piece's skirt (one loft and a cut: two lofts unioned left slivers)
  difference() {
    ring_loft(concat(lap_levels(function(z) groove_in(z, deck_face) - eps), [[top_z, groove_in(top_z, deck_face) - eps, tongue_in(z_deck_tongue, deck_face, z_deck_tongue) - eps]]));
    intersection() {
      difference() {
        translate([-case_w, -case_l, z_bot - 1]) cube([2 * case_w, 2 * case_l, case_h]);
        rr_loft([for (z = [z_bot - 0.5:0.05:top_z + 0.5]) [z, groove_in(z, deck_face) + deck_gap]]);
      }
      yband(sy > 0 ? 0 : -(lap_y0 - eps), sy > 0 ? lap_y0 - eps : 0);
    }
  }
  // its side and its span along the wall, its top 45 degrees down from the root
  translate([sx > 0 ? 0 : -case_w, -case_l, -1]) cube([case_w, 2 * case_l, case_h + 2]);
  scale([1, sy, 1]) rotate([90, 0, 90]) translate([0, 0, -case_w]) linear_extrude(2 * case_w)
    polygon([[joint_y - lap_l, z_bot - 1], [lap_y0 + lap_root, z_bot - 1], [lap_y0 + lap_root, top_z + 1], [lap_y0 - 2 * eps, top_z + 1],
             [lap_y0 - 2 * eps, lap_top(lap_y0)], [joint_y - lap_l, lap_top(joint_y - lap_l)]]);   // (2 eps: off the skirt's end)
}
// the tongue's slot round a lap: through the tongue (and its thicker foot under the round's end) from the seam up
module lap_slot_cut(sx, sy) translate([sx > 0 ? case_w / 2 - wall - tub_round_in - 0.1 : -case_w / 2 + wall / 2 - seam_fit - 0.1,
                                       sy > 0 ? lap_slot[0] : -lap_slot[1], z_seam])
  cube([wall / 2 + seam_fit + tub_round_in + 0.2, lap_slot[1] - lap_slot[0], case_h]);
// a rib runs up the tongue to 0.1 short of where its outside starts to lean in, its top a 45 degree lead-in
function grip_top(H) = let(zt = H == case_h ? z_tongue : z_deck_tongue)
  max([for (k = [0:800]) let(z = z_seam + 0.01 * k) if (z <= zt && tongue_out(z, H, zt) <= wall / 2 + seam_fit + 1e-6) z]);
module grip(g) let(sx = g[0], w = len(g) > 3 ? g[3] : grip_w, z1 = grip_top(g[2]) - 0.1)
  translate([sx * (case_w / 2 - wall / 2 - seam_fit), g[1], 0]) scale([sx, 1, 1]) rotate([90, 0, 0]) linear_extrude(w, center = true)
    polygon([[-0.1, z_seam - 0.3], [grip_reach, z_seam - 0.3], [grip_reach, z1 - grip_reach - 0.05], [-0.05, z1], [-0.1, z1]]);

// ---- the decks' flange and the top piece's tongue (2026-10-23, case/SHELL.md "The tongue under the step") ----
// Alex, on the print: the top piece "kind of just sits on top of the deck", so it moves; he asked for an
// indentation across their whole contact that the top piece inserts into, and sketched the deck running on
// under the step's block with a tongue of the top piece down into a groove in it. Both parts print face
// down and the deck goes in from behind (+z), so all of a deck that lies under the top piece must stay
// under the 45 degree line from its land (the wedge: anything further in would start in mid air), and a
// groove under the top piece can't have a far wall unless that wall is bridged. So:
//  - each deck's wedge runs on down to z_flange (the flange), flange_in under the top piece at its foot;
//  - the top piece carries a tongue lock_t thick in its land's plane (the joint's line), down through slots
//    in the flange to z_flange, its tip's edges and ends chamfered lock_lead as lead-ins;
//  - the slots are lock_slot_max long at most, between ties at least lock_tie long where the wedge is whole:
//    beyond each slot the flange is a fin under the wedge's line (lock_relief under it), its first layers
//    a bridge from tie to tie, its top flat at z_lock_cap where it is lock_wall thick;
//  - the tongue's faces keep deck_gap to the slot's sides (the deck's land, the fin) and its ends to the ties.
// Across the step the fins hold the deck from sliding away from the screen and the ties hold it across;
// down the spine's sides the ties hold it along the case and the fins across. The wedges (the ties, and
// the fins' slopes) still hold it in from the front, so the corner posts still push it home onto them.
// Where one of the board's taller parts is under it (either end's: one deck fits both), the flange stops
// part_clear over the part (flange_keep) and there is no slot; at the spine's end U8 and U9 are under
// most of it, so it has none. check_fit.py checks the board against the flange (the FLANGE echo line)
part_clear = 0.3;                                       // the flange and the tongue over the board's parts
flange_part_h = 0.5;                                    // the tallest part allowed under them (the 0402 capacitors)
flange_n = floor((z_deck_under - z_top_cu - flange_part_h - part_clear) / lay + 1e-6);   // the wedge's layers under the plate
z_flange = z_deck_under - lay * flange_n;               // the flange's foot (4.6, 0.8 over the board)
flange_in = e0 - z_flange;                              // how far in under the top piece the wedge reaches there (2.3)
flange_x = in_x - 1.0;                                  // the flange stops short of the side walls (the laps, the tub's tongue)
lock_t = 0.8;  lock_lead = 0.3;                         // the tongue's thickness and its lead-ins
lock_relief = 0.12;  lock_wall = 0.7;                   // the fin under the wedge's line, and its least thickness (0.12,
                                                        // not 0.1 or 0.15: its slope would run through the slabs' corners)
lock_tie = 1.5;  lock_slot_max = 4.5;                   // a tie's least length, a slot's most (its fin bridges it)
lock_far = lock_t + deck_gap;                           // the fin's face, in from the joint's line
z_lock_cap = max(z_flange + 0.1, e0 - lock_relief - lock_far - lock_wall);   // the fin's flat top (5.13)
// the board's parts under the flange taller than flange_part_h, from check_fit's board model (2026-10-23),
// in seat 1's frame (seat 2's turned round, as its deck is): [ref, [x0, y0], [x1, y1], their top over the
// board, rounded up]
flange_keep = [["Q6", [-7.65, -34.9], [-5.15, -31.9], 1.20], ["Q5", [5.15, -34.9], [7.65, -31.9], 1.20],
               ["Q3", [24.25, -34.9], [26.75, -31.9], 1.20], ["X1", [-23.8, -34.75], [-20.6, -32.25], 0.80],
               ["D3", [-33.95, -34.03], [-31.45, -32.78], 1.11],                   // seat 2's
               ["C5", [-34.0, -31.8], [-33.2, -30.2], 0.80], ["C6", [-31.8, -31.8], [-31.0, -30.2], 0.80],
               ["C7", [-29.6, -31.8], [-28.8, -30.2], 0.80],
               ["U8", [-1.3, -47.4], [3.7, -41.0], 1.10], ["U9", [-2.5, -44.7], [2.5, -38.3], 1.10]];   // U9 seat 2's
// (each grown 0.02 more, up and round: on part_clear exactly, Q5's and Q6's inner faces lie on a layer of the
// spine's wedges, and their tops on the layers' planes)
function keep_box(k) = let(g = part_clear + 0.02) [k[1] - [g, g], k[2] + [g, g]];
function keep_z(k) = z_top_cu + k[3] + part_clear + 0.02;                  // the flange's foot over it
function keep_at(n) = keep_box(flange_keep[search([n], flange_keep)[0]]);
module flange_keep_cuts() for (k = flange_keep) let(b = keep_box(k))
  translate([b[0][0], b[0][1], z_flange - 1]) cube([b[1][0] - b[0][0], b[1][1] - b[0][1], keep_z(k) - z_flange + 1]);
// the runs of the tongue, seat 1's frame: [along ("x": across the step, "y": down a spine's side), the
// joint's line there, which way is in under the top piece, from, to], each between keepouts, the flange's
// end and the spine's end; seat 2's are these turned round
lock_runs = [["x", -joint_y, 1, keep_at("Q5")[1][0], keep_at("Q3")[0][0]],
             ["x", -joint_y, 1, keep_at("Q3")[1][0], flange_x],
             ["x", -joint_y, 1, keep_at("D3")[1][0], keep_at("X1")[0][0]],
             ["x", -joint_y, 1, keep_at("X1")[1][0], keep_at("Q6")[0][0]],
             ["y", spine_w / 2, -1, -spine_end, keep_at("Q5")[0][1]],
             ["y", -spine_w / 2, 1, -spine_end, keep_at("Q6")[0][1]]];
// a run's slots, [from, to] along it: as few as keep each within lock_slot_max, a tie lock_tie long between
// them and at each end (over a keepout the flange is cut short, under most fins' tops: it holds no bridge)
function lock_slots(r) = let(n = max(1, ceil((r[4] - r[3] - lock_tie) / (lock_slot_max + lock_tie))), l = (r[4] - r[3] - (n + 1) * lock_tie) / n)
  [for (i = [0:n - 1]) [r[3] + lock_tie + i * (l + lock_tie), r[3] + lock_tie + i * (l + lock_tie) + l]];
// (u in under the top piece, z, t along the run from t0) to the case's frame
module along(r, t0) multmatrix(r[0] == "x" ? [[0, 0, 1, t0], [r[2], 0, 0, r[1]], [0, 1, 0, 0], [0, 0, 0, 1]]
                                           : [[r[2], 0, 0, r[1]], [0, 0, 1, t0], [0, 1, 0, 0], [0, 0, 0, 1]]) children();
// a slot's plan, [[x0, y0], [x1, y1]]: from the deck's land to the fin's foot, along its span (for the checks)
function lock_rect(r, s) = let(u0 = -(deck_gap + 0.02), u1 = e0 - lock_relief - z_flange, c = [r[1] + r[2] * u0, r[1] + r[2] * u1])
  r[0] == "x" ? [[s[0], min(c)], [s[1], max(c)]] : [[min(c), s[0]], [max(c), s[1]]];
// the deck's slot: through the flange from the deck's land (deck_gap off the tongue; 0.02 into the plate's
// edge under the land, past the wedge's back face, so no face coincides) to the fin, and over the fin down
// to its top (lock_relief under the wedge's line, flat at z_lock_cap)
module lock_slot_cut(r, s) along(r, s[0]) linear_extrude(s[1] - s[0])
  polygon([[-(deck_gap + 0.02), z_flange - 1], [lock_far, z_flange - 1], [lock_far, z_lock_cap], [e0 - lock_relief - z_lock_cap, z_lock_cap],
           [4, e0 - lock_relief - 4], [4, e0 + 0.5], [0, e0 + 0.5], [0, e0 + 0.02], [-(deck_gap + 0.02), e0 + 0.02]]);
// the top piece's tongue in it: eps in from the land's face, up into the band over the undercut, deck_gap
// short of the ties, its tip chamfered all round
module lock_tongue(r, s) let(l = s[1] - s[0] - 2 * deck_gap) along(r, s[0] + deck_gap) hull() {
  translate([eps, z_flange + lock_lead, 0]) cube([lock_t - eps, e0 + eps - z_flange - lock_lead, l]);
  translate([eps + lock_lead, z_flange, lock_lead]) cube([lock_t - eps - 2 * lock_lead, 0.01, l - 2 * lock_lead]);
}
// the flange's plan (seat 1's end): its bounding rectangles across the step each side of the spine, down
// the spine's sides and round its end (the tongue is inside them)
flange_zones = let(o = deck_gap + 0.01, sw = spine_w / 2)
  [[[sw - flange_in, -(joint_y + o)], [flange_x, -(joint_y - flange_in)]], [[-flange_x, -(joint_y + o)], [-(sw - flange_in), -(joint_y - flange_in)]],
   [[sw - flange_in, -(spine_end + o)], [sw + o, -(joint_y - flange_in)]], [[-(sw + o), -(spine_end + o)], [-(sw - flange_in), -(joint_y - flange_in)]],
   [[-(sw + o), -(spine_end + o)], [sw + o, -(spine_end - flange_in)]]];
function rect_gap(a, b) = max(max(a[0][0], b[0][0]) - min(a[1][0], b[1][0]), max(a[0][1], b[0][1]) - min(a[1][1], b[1][1]));   // < 0: they overlap
function rot_rect(r) = [rot180(r[1]), rot180(r[0])];
// The flange and the tongue's checks (tune skips them: its slices move the decks' face). The tongue and
// the fins as sized, every slot's fin a bridge the printers make (overhangs.py: 2.6 from a held edge) and
// clear of every keepout; and the flange (both ends': seat 2's turned round) clear of the screws' bosses,
// the panel's frame and FPC, J4, the speaker, the pads, Start/Select, the tub's tongue and laps, and its ribs
if (!tune) {
  assert(lock_t >= 0.8 && lock_lead < lock_t / 2 && lock_far + lock_wall < e0 - lock_relief - z_lock_cap + 1e-6, "the tongue or its fin is too thin");
  assert(z_lock_cap - z_flange >= 0.5, "the tongue engages the fins too little");
  assert(z_flange - z_top_cu >= flange_part_h + part_clear - 1e-6 && flange_in > wedge_w, "the flange is too low, or no deeper than the wedge");
  for (r = lock_runs) let(sl = lock_slots(r)) {
    for (s = sl) assert(s[1] - s[0] >= 2.0 && s[1] - s[0] <= 2 * 2.6, "a slot is too short, or its fin's bridge too long");
    for (i = [1:1:len(sl) - 1]) assert(sl[i][0] - sl[i - 1][1] >= lock_tie - 1e-6, "a tie is too short");
    // (a keepout that cuts the flange off over a fin's top can't hold its bridge: a tie between them)
    for (s = sl, k = flange_keep) let(a = lock_rect(r, s), b = keep_box(k), i = r[0] == "x" ? 0 : 1, gap = function(j) max(a[0][j], b[0][j]) - min(a[1][j], b[1][j]))
      assert(gap(1 - i) >= 0 || gap(i) >= (keep_z(k) > z_lock_cap ? lock_tie - 1e-6 : 0), str("a slot runs over ", k[0], ", or ends at it without a tie"));
  }
  for (z = flange_zones) {
    for (p = concat(side_pins, [for (s = side_pins) rot180(s)])) assert(rect_dist(p, z) >= boss_d / 2 + 0.3, "the flange runs into a side screw's boss");
    for (p = [pins[5], rot180(pins[4])]) assert(rect_dist(p, z) >= boss_d / 2 + 0.3, "the flange runs into an end screw's boss");
    assert(rect_gap(z, frame_rect) >= 0.3 && rect_gap(z, [[fpc_x - fpc_w / 2 - 0.5, fpc_y(tip_range[1] + 0.5)], [fpc_x + fpc_w / 2 + 0.5, fpc_y(-1.5)]]) >= 0.3,
           "the flange runs into the panel's frame or its FPC's fold");
    assert(rect_gap(z, j4_body) >= 0.3 && rect_gap(z, j4_plug) >= 0.3, "the flange runs into J4 or its plug");
    assert(z[1][1] <= spk_pos[1] - spk[1] / 2 - spk_keep - 0.3, "the flange runs into the speaker");
    assert(z[0][1] >= -ss_y + (ss_cell + 2 * ss_frame_w) / 2 + 0.3, "the flange runs into Start/Select's frame");
    for (c = clusters) assert(rect_dist(c, z) >= piece_d / 2 + 0.5, "the flange runs under a silicone piece");
    assert(max(abs(z[0][0]), abs(z[1][0])) <= case_w / 2 - inner_max(deck_face, z_seam, z_deck_tongue) - 0.5, "the flange runs into the tub's tongue or a deck's lap");
    for (g = grips) assert(rect_gap(z, [[g[0] * in_x - 1, g[1] - grip_w], [g[0] * in_x + 1, g[1] + grip_w]]) >= 0.3, "the flange runs into a grip rib");
    assert(rect_gap(z, [post_c - [0.5, 0.5], post_c + [5, 5]]) >= 1.0 && rect_gap(z, [-post_c - [5, 5], -post_c + [0.5, 0.5]]) >= 1.0, "the flange runs into a corner post");
  }
}

ss_vent = 1.25;           // each Start/Select dome's 0.5 mm vent, this far towards seat 1 from its centre
// what the floor's ribs keep clear of under the board: [x0, y0, x1, y1]
rib_free = concat([
  [cell_hole[0][0] - 0.5, cell_hole[0][1] - 0.5, cell_hole[1][0] + 0.5, cell_hole[1][1] + 0.5],   // the cell's hole
  [usb_front - 8.5, usb_y - usb_tab_w / 2, board_w / 2 + usb_tab, usb_y + usb_tab_w / 2],         // J1's shell legs
  [-10, 45.5, 10, 48.5], [-12.0, -15.5, -9.2, -12.5], [-9.3, -11.5, 1.3, -8.5], [-23.0, -28.3, -19.8, -25.1],  // test pads
  [-8.0, -43.7, 8.0, -38.6],                                                                      // J7's pogo pads
], [for (b = ss_buttons) [b[2][0] - 0.75, b[2][1] - ss_vent - 0.75, b[2][0] + 0.75, b[2][1] - ss_vent + 0.75]]);   // dome vents
module rib_free2d() for (r = rib_free) translate([r[0], r[1]]) square([r[2] - r[0], r[3] - r[1]]);
// Ribs under the strips beside the cell's hole, the board's weakest part (2026-10-22 review), along
// their length: the left one under the debug switches a pin presses (SW18, SW19) and J5/J6, where a
// cable is pushed in; the right one under the power latch and TH1, with one across it at y 14. The
// floor's grid and its ring round the board's edge cross them
strip_ribs = [[[pin_pos[0][0] - 0.6, cell_hole[0][1]], [pin_pos[0][0] + 0.6, cell_hole[1][1]]],
              [[29.6 - 0.6, cell_hole[0][1]], [29.6 + 0.6, cell_hole[1][1]]],
              [[cell_hole[1][0], 14 - 0.6], [board_w / 2, 14 + 0.6]]];
// the speaker's outline in the plan, a racetrack along y (grow: out from it), and its grille: slots
// along it, inside the outline less grille_in
module spk2d(grow = 0) at(spk_pos) hull() for (s = [-1, 1]) translate([0, s * (spk[1] - spk[0]) / 2]) circle(d = spk[0] + 2 * grow);
module grille2d() offset(r = grille_slot[0] / 2, $fn = 24) intersection() {
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
// a solid lofted through the case's outline at each level [z, inset], from the first level to the last
module rr_loft(levels) loft([for (l = levels) rr_pts(l[1], l[0])]);
// [z, inset] up the back's round every 0.1 from z0 to z1 (both ends included); grow: that much more in
function back_levels(z0, z1, grow = 0) = let(k0 = floor(z0 / (back_r / back_n) + 1e-6) + 1, k1 = ceil(z1 / (back_r / back_n) - 1e-6) - 1)
  [for (z = concat([z0], [for (k = [k0:1:k1]) k * back_r / back_n], [z1])) [z, back_in(z) + grow]];
// the case's outside up the back's round to h (and straight up past its end), shrink: that much inside it
// (one loft: a straight prism on its top left non-manifold edges where the USB-C tab met the joint)
module tub_outside(h = z_seam, shrink = 0) rr_loft(concat(back_levels(0, min(h, back_r), shrink), h > back_r ? [[h, shrink]] : []));
module tub() difference() {
  union() {
    difference() {
      tub_outside();
      // the cavity: tub_round_in inside the walls up the round (the board's edge is in it, since
      // 2026-10-22), the wall's inside above it
      translate([0, 0, floor_t]) linear_extrude(z_seam - floor_t + 0.01) inner2d(-tub_round_in);
    }
    // the tongue: the wall's inner half, up past the seam inside the top pieces' skirts, its keys (the
    // top piece's tabs, the decks' corner posts) and its grip ribs
    tongue();
    for (k = keys) key_tab(k);
    for (sx = [-1, 1], sy = [-1, 1]) post(sx, sy);
    for (g = grips) grip(g);
    // the tab that carries the USB-C opening's lower half, up from the seam into the top piece's notch,
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
  // the tongue's slots round the decks' laps at the side joints
  for (sx = [-1, 1], sy = [-1, 1]) lap_slot_cut(sx, sy);
  // USB-C: the opening's lower half, in the tab; over the tab the tongue stops either side of the
  // top piece's skirt, which comes down to the opening's centre there (usb_fit along the wall)
  usb_opening();
  translate([in_x - 1, usb_y - usb_notch_w / 2 - usb_fit, usb_zc - 0.11]) cube([wall + 1, usb_notch_w + 2 * usb_fit, case_h]);
  // the slot the board's tab drops into
  if (usb_tab > 0) translate([in_x - tub_round_in - 0.01, usb_y - usb_tab_w / 2 - usb_tab_fit, z_board - 0.3])
    cube([usb_tab - board_clear + tub_round_in + usb_tab_fit + 0.01, usb_tab_w + 2 * usb_tab_fit, case_h]);
}

usb_zc = z_top_cu + usb_h / 2;   // the port's centre
usb_open_x = [in_x - 1, in_x + wall + 1];   // the opening's cut, through the whole wall
// the notch in the top piece's skirt under the port (fit 0: as cut from the top piece) or the tub's
// tab in it (fit, lift: its clearance each side and above): full width in the wall's inner half,
// usb_step narrower and lower in its outer half, so the top piece frames the tab outside (the tab
// starts 0.01 under the seam, in the tub's wall: a solid only touching it leaves non-manifold edges)
module usb_notch(fit = 0, lift = 0) let(o = fit == 0 ? 0.01 : 0, w = usb_notch_w - 2 * fit, dn = fit == 0 ? 1 : 0.01) {
  translate([in_x - o, usb_y - w / 2, z_seam - dn]) cube([wall / 2 + 2 * o, w, usb_zc - lift - z_seam + dn]);
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

// ---- the top piece: the screen face, the steps, the spines and the sides; printed face down ----
// Its outside is smooth solids, not 0.1 mm slabs (2026-10-22): slabs put their flat faces on the
// slicer's planes, and it printed their steps unevenly (row 9 on the tub, 4df3f96). Every face here
// is a ruled surface between levels every 0.05 down the step and the rounds, so the slicer meets the
// real slope. The step's foot, the screen block's edges and the side walls' round meet as the slabs
// had them: a point is inside if it is inside the round from the face height the step reaches there
z_round0 = deck_face - face_r;                          // the rounds start here (the step's is shallower)
z_bot = z_seam + seam_gap;                              // the skirts' bottom
round_n = round(face_r / 0.05);  step_n = round(step_d / 0.05);
// how far in from the joint the step is at depth dd under the screen face, down to its foot (the
// land at the deck's height, joint_land wide, is where dd reaches step_d)
function step_in(dd) = joint_land + edge_in(min(dd, step_d), step_r) - edge_in(step_d, step_r);
// [z, how far in from the joint] up the step, from the land at the deck's height (a little under it,
// to overlap the prism below) to the screen face
step_levels = concat([[deck_face - 0.01, joint_land]], [for (k = [step_n:-1:0]) [case_h - step_d * k / step_n, step_in(step_d * k / step_n)]]);
// a box [x0, y0, x1, y1] up to the deck's height, and over it the step: its sides in from the joint
// at each step level (which sides step: 1 or 0 for each of x0, y0, x1, y1)
module step_box(r, sides) {
  translate([r[0], r[1], z_bot]) cube([r[2] - r[0], r[3] - r[1], deck_face - z_bot]);
  loft([for (l = step_levels) let(s = l[1])
    [for (c = [[r[0] + sides[0] * s, r[1] + sides[1] * s], [r[2] - sides[2] * s, r[1] + sides[1] * s],
               [r[2] - sides[2] * s, r[3] - sides[3] * s], [r[0] + sides[0] * s, r[3] - sides[3] * s]]) [c[0], c[1], l[0]]]]);
}
// the right side's wall and its round, swept along y under a face whose height follows the step down
// to the deck at each end (the left is its mirror): its section at each y, clockwise seen from +x. It
// starts at side_x0, inside the screen block's edge
side_x0 = 26.4;
// Under the round's end the skirt follows the back's round in, down to its tip at z_bot (2026-10-22)
skirt_levels = back_levels(z_bot, back_r);              // [z, inset] up the round, z_bot to back_r
// h: the face's height there (the step's, across the case); the round's top is h or the side's blend,
// the lower, and between them its 45 degree chamfer (c: its height; at least 0.002 wide, so the section
// keeps its point count)
function side_section(y, h) = let(hr = min(h, case_h - side_drop(abs(y))), c = max(h - hr, 0.002))
  concat([[side_x0, y, z_bot], [side_x0, y, h], [case_w / 2 - edge_in(0, face_r) - c, y, min(h, hr + c)]],
  [for (k = [0:round_n]) let(dd = face_r * k / round_n) [case_w / 2 - edge_in(dd, face_r), y, hr - dd]],
  [for (i = [len(skirt_levels) - 1:-1:0]) [case_w / 2 - skirt_levels[i][1], y, skirt_levels[i][0]]]);
// the sections' [y, h]: the joint, the land and the step, as before, and every 0.2 over the blend's run
// back from the step
function side_ys() = let(top = [for (k = [step_n:-1:0]) [joint_y - step_in(step_d * k / step_n), case_h - step_d * k / step_n]],
                         y0 = joint_y - joint_land - side_run, y1 = joint_y - step_in(0), n = ceil((y1 - y0) / 0.2),
                         run = [for (i = [0:n - 1]) [y0 + (y1 - y0) * i / n, case_h]])
  concat([[-joint_y, deck_face]], [for (i = [0:len(top) - 1]) [-top[i][0], top[i][1]]], [for (i = [n - 1:-1:0]) [-run[i][0], case_h]],
         run, [for (i = [len(top) - 1:-1:0]) top[i]], [[joint_y, deck_face]]);
module side_sweep() loft([for (yh = side_ys()) side_section(yh[0], yh[1])]);
module top_outer() {
  step_box([-(side_x0 + 3.6), -joint_y, side_x0 + 3.6, joint_y], [0, 1, 0, 1]);          // the screen block
  step_box([-spine_w / 2, -spine_end, spine_w / 2, spine_end], [1, 1, 1, 1]);              // the spines
  for (sx = [-1, 1]) scale([sx, 1, 1]) side_sweep();                                       // the sides
}
module j4_pocket2d() difference() {
  for (r = [j4_body, j4_plug]) corners2d(r, grow = j4_pocket_grow);
  at(pins[2]) circle(d = boss_d);
  translate([-case_w, -case_l]) square([2 * case_w, case_l - (joint_y - joint_w)]);
}
module top() difference() {
  union() {
    difference() {
      top_outer();
      // the inside, under the screen block and the spines, behind the joint's band
      translate([0, 0, z_seam - 1]) linear_extrude(z_ceil - z_seam + 1)
        intersection() { inner2d(); offset(delta = -joint_w) top_region2d(); }
      // nothing of the band below the decks' underside (out past the spines' ends, so the cut's end
      // isn't the end faces')
      translate([0, 0, z_seam - 1]) linear_extrude(z_deck_under - z_seam + 1) intersection() { inner2d(); offset(delta = 0.01) top_region2d(); }
      // the undercut the decks' wedges sit in (45 degrees: face down it prints as an upward face)
      for (k = [0:ceil(wedge_w / lay - 1e-6) - 1]) let(z = z_deck_under + lay * k)
        translate([0, 0, z - (k == 0 ? 0.01 : 0)]) linear_extrude(lay + 0.01 + (k == 0 ? 0.01 : 0)) intersection() {
          inner2d();
          difference() { offset(delta = 1) top_region2d(); offset(delta = -(e0 - z - lay / 2)) top_region2d(); }
        }
      // the groove the tub's tongue slides up: the skirts' inner half, up to the tongue's top (and,
      // across the step's band, into the band where the tongue leans in)
      groove();
    }
    // over the USB-C notch the skirt's inner half stays, from the opening's centre up: the tongue
    // stops either side of it, so it frames the opening's upper half and keys the top piece along the wall
    translate([in_x - eps, usb_y - usb_notch_w / 2, usb_zc - eps]) cube([wall / 2 + 2 * eps, usb_notch_w, z_tongue + seam_gap + 0.1 - usb_zc + eps]);
    // under the side's blend, where it brings the face down towards the deck's height, the underside
    // comes down to the groove's top out to side_fill inside the wall (outside the board's edge), so the
    // chamfer over it keeps its thickness
    intersection() {
      translate([0, 0, z_deck_tongue + seam_gap]) linear_extrude(z_ceil - z_deck_tongue - seam_gap + eps) difference() { inner2d(eps); inner2d(-side_fill); }
      difference() { y_out(joint_y - joint_land - side_run); y_out(joint_y - joint_w + eps); }
    }
    // the tongue down through the decks' flanges, at both ends
    for (t = [0, 180]) rotate(t) for (r = lock_runs) for (s = lock_slots(r)) lock_tongue(r, s);
    // bosses down to the board top: with the tub's bosses they clamp it
    for (s = side_pins) at(s, z_top_cu) cylinder(d = boss_d, h = z_ceil - z_top_cu + 0.01);
    // pinhole guides reach down towards the switches
    for (p = pin_pos) at(p, z_top_cu + 2.4) cylinder(d = 3.0, h = z_ceil - z_top_cu - 2.4 + 0.01);
    // panel locating frame (its opening is the panel's pocket, cut below: one cut, so no two faces
    // coincide there)
    translate([0, 0, z_ceil - frame_h]) linear_extrude(frame_h + 0.01) difference() {
      square([panel_w + panel_fit + 2 * frame_wall, panel_l + panel_fit + 2 * frame_wall], center = true);
      translate([fpc_x, -panel_l / 2]) square([fpc_w + 4, 6], center = true);   // the FPC's fold passes here
    }
  }
  // the pockets the tub's tabs key into
  for (k = keys) key_pocket(k);
  // USB-C: the opening's upper half, and below it the notch the tub's tab fills
  usb_opening();
  usb_notch(0);
  // the panel's frame and its pocket: its top face_t under the screen face
  translate([0, 0, z_ceil - frame_h - 0.01]) linear_extrude(frame_h + top_t - face_t + 0.01)
    square([panel_w + panel_fit, panel_l + panel_fit], center = true);
  // viewing window: the panel's own glass shows through, no lens
  translate([0, 0, z_ceil - 1]) linear_extrude(top_t + 2) window();
  // pilot holes for the screws
  for (s = side_pins) at(s, z_top_cu - 0.01) cylinder(d = screw_pilot, h = screw_engage);
  // pinholes
  for (p = pin_pos) at(p, 0) cylinder(d = 1.2, h = case_h + 1);
  // the speaker (2026-10-22): a pocket over it spk_keep round its outline, and its grille, slots
  // through the face over its cone (face down they are plain holes in the plate on the bed); the
  // rim round them stays solid for its front ring to seal on
  translate([0, 0, z_ceil - 0.01]) linear_extrude(spk_pocket + 0.01) spk2d(spk_keep);
  translate([0, 0, z_ceil]) linear_extrude(top_t + 1) grille2d();
  // a shallow pocket over J4 and the speaker's mated plug, round H3's boss and inside the joint's band
  translate([0, 0, z_ceil - 0.01]) linear_extrude(j4_pocket + 0.01) j4_pocket2d();
}

// ---- a deck (sy: -1 seat 1's end, 1 seat 2's): printed face down; seat 2's is seat 1's turned round ----
function end_pin(sy) = sy < 0 ? pins[5] : pins[4];
module deck(sy) {
  deck_body(sy);
  for (sx = [-1, 1]) lap(sx, sy);                       // the side joints' laps, in under the top piece's skirt
}
module deck_body(sy) difference() {
  union() {
    difference() {
      union() {
        // its face and outside: the outline's round from the deck face, out to the case's outline,
        // one smooth loft (as the top piece's and the tub's)
        intersection() {
          rr_loft(concat(skirt_levels, [for (k = [round_n:-1:0]) [deck_face - face_r * k / round_n, edge_in(face_r * k / round_n, face_r)]]));
          translate([0, 0, z_bot - 1]) linear_extrude(deck_face - z_bot + 2) deck_region2d(sy);
        }
      }
      translate([0, 0, z_seam - 1]) linear_extrude(z_deck_under - z_seam + 1) inner2d();   // under the plate
    }
    // the wedge along its inner edges, in under the top piece's undercut, and on down under the band to
    // z_flange (the flange, short of the side walls), less where the board's taller parts are under it
    difference() {
      for (k = [0:ceil(wedge_w / lay - 1e-6) + flange_n - 1]) let(z = z_flange + lay * k)
        translate([0, 0, z]) linear_extrude(lay + 0.01) intersection() {
          half2d(sy);
          inner2d(-deck_gap);
          square([2 * (z < z_deck_under - 1e-6 ? flange_x : case_w), 4 * case_l], center = true);
          difference() { offset(delta = deck_gap + 0.01) top_region2d(); top_region_xy2d(-(e0 - z - lay / 2 - deck_gap), -(e0 - z - lay / 2)); }
        }
      rotate(sy > 0 ? 180 : 0) flange_keep_cuts();
    }
    // the walls that rest on the silicone pads (the pockets are cut below)
    // (the A/B/X/Y clover is solid between its pockets, where the faceplate leaves it open round
    // pinwheel ears: here the tab slots are cut into it and key the caps with the notch ribs)
    for (c = [abxy1, abxy2]) if (c[1] * sy > 0) at(c, retainer_bot) linear_extrude(z_deck_under - retainer_bot + 0.01) union() {
      abxy_clover2d();
      square(abxy_square[0] + 2 * abxy_square[1], center = true);
    }
    for (c = [dpad1, dpad2]) if (c[1] * sy > 0) at(c, retainer_bot) linear_extrude(z_deck_under - retainer_bot + 0.01) dpad_wall2d();
    for (c = ss_centres) if (c[1] * sy > 0) at(c, retainer_bot) linear_extrude(z_deck_under - retainer_bot + 0.01) ss_frame2d();
    // its boss, down to the board: the end screw pulls the deck onto the board
    at(end_pin(sy), z_top_cu) cylinder(d = boss_d, h = z_deck_under - z_top_cu + 0.01);
  }
  // buttons
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
  // the groove the tub's tongue slides up (into the plate, where the tongue leans in), and the notches
  // its corner posts key into
  groove(false);
  for (sx = [-1, 1]) post_notch(sx, sy);
  // the slots the top piece's tongue drops through
  rotate(sy > 0 ? 180 : 0) for (r = lock_runs) for (s = lock_slots(r)) lock_slot_cut(r, s);
}

module board3d() color("#2e7d4f") translate([0, 0, z_board]) linear_extrude(pcb_t) board_outline();
// the cell in its hole on the pocket's floor, and the speaker on the board (nominal sizes)
module cell3d() translate([0, 0, floor_t - cell_pocket]) linear_extrude(cell_nom[2]) offset(r = 1) offset(delta = -1) cell2d(cell_nom);
module speaker3d() translate([0, 0, z_top_cu]) linear_extrude(spk[2]) spk2d();

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
  color("#5a5a68") translate([0, 0, explode * 0.8]) for (sy = [-1, 1]) deck(sy);
  color("#e8e8ee") caps();
}
if (part == "tub") tub();
if (part == "top") translate([0, 0, case_h]) rotate([180, 0, 0]) top();         // printed face down
// one part for both ends (seat 2's is seat 1's turned round): print two, face down
if (part == "deck") translate([0, 0, deck_face]) rotate([180, 0, 0]) deck(-1);
