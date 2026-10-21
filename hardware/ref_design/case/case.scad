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

// The cell lies on the tub's floor under the board, lengthwise, its wires leaving its seat-1
// end; its plug comes up through a hole in the board to J2 under the spine between seat 1's
// clusters, and the panel sits 2.4 mm over the board. 2026-10-05 to -09 it lay on the board
// under the screen, for a 13.9 mm case: branch ref-design-cell-on-board.
cell = [34, 50, 4];                                     // 403450 Li-ion, x by y (length) by thickness;
                                                        // ESTIMATE: length with its protection board
cell_y = 0;                                             // centre: its wire end 25 mm towards seat 1, past the
                                                        // plug's hole (-12 in the 156 mm case)
cell_gap = 0.5;                                         // its top to the board's back
// The speaker (Alex, 2026-10-20): INGHAi's GSPK151035PN, a 15 x 10 mm racetrack 3.5 mm thick
// with its adhesive front foam ring, whose 35 mm leads end in a 1.25 mm 2-pin plug (the
// PicoBlade pattern) for J4: no soldering at assembly. It fired down through a grille in the
// floor, which a table covers when two players share the screen; now it lies face down beside
// the left wall, its foam ring stuck to a pad round a front chamber cut into the floor, and the
// chamber opens through the wall's rounded lower edge in three slots: the sound leaves the side,
// a millimetre over a table. Its back faces up into the case, which is its back volume. Its
// leads leave the end towards the middle and come up through a slot in the board to J4
spk = [15, 10, 3.5];                                    // length (along x), width, thickness with the ring
spk_tab = 2.5;                                          // ESTIMATE: its solder tab, past the end away from the leads
spk_lead = 35;                                          // lead length to the plug (datasheet: "35mm/1.25/2P")
spk_pad = 0.6;  spk_seal = 1.0;                         // the pad it lies on; the ring's width on the pad
spk_chamber = 0.8;                                      // the front chamber's floor, over the tub's outside
spk_duct = [0.8, 1.6];  spk_slots = 3;  spk_slot_w = 1.8;   // the outlet: its z range, its slots


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
                                                        // (faceplate 9.18)
cap_chamfer = 0.6;                                      // 45 degree chamfer round each hole at the face (faceplate)
lip_t = 1.6;                                            // the face down to the A/B/X/Y flanges' tops (faceplate 1.60;
                                                        // Pi Tin 1.65; was 1.16)
dpad_gap = 0.65;          // D-pad opening past the cross each side: 0.5 (the faceplate's 18.80 x 6.90 opening) was
                          // too tight for Alex's AliExpress D-pad in the first print, 0.8 too loose in the next
                          // (Alex, 2026-10-21), so halfway (tune: tune_openings, 0.55 / 0.65 / 0.75)
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
case_w = 75.4;            // set by the clusters (edge_margin, piece_d, the spacing between them), not the panel:
                          // the 2.4 in panel leaves it as it was (Alex, 2026-10-21)
glass_side = (case_w - bay_w) / 2;                      // panel bay edge to long case edge (15.6: USB-C and the
                                                        // debug parts beside the glass)
edge_margin = 4;          // case edge to silicone piece
piece_to_glass = 5;       // silicone piece to glass edge
wall = 2.0;  board_clear = 0.5;
corner_r = 4.3;           // plan corners: a DS Lite's 4.3 to 4.4 (8 felt round and toy-like, 2026-10-19)
board_r = 5.5;            // the board's own corners (its KiCad outline): no longer follow the case's
// The flattest a face may slope downwards in its print pose, in degrees from vertical: no part needs
// supports. 45 until Alex's 6_tests ladder says how far each printer goes (one line, 2026-10-21)
overhang = 45;
// Every outside edge rounds over as far as that allows: an arc into the wall, and where the arc would
// go flatter than `overhang` in the print pose, a tangent chamfer at `overhang` (the tub prints back
// down, the top piece and the decks face down, so all their rounds end in a chamfer at the bed)
back_r = 4.2;             // the back rolls into the walls through back_r, as a DS Lite's does (4.2 mm)
face_r = 3.0;             // the top piece's and the decks' faces roll into the walls (Alex, 2026-10-21, item
                          // 10: the top pieces curve over the sides, the tub no longer forms the outer lip)
support_pitch = 12;       // the floor's support ribs under the board
// The seam: the top pieces (the top piece and both decks) meet the tub flush where the back's round
// meets the walls. The tub's wall carries a tongue, the inner half of the wall tongue_h up past the
// seam, and the top pieces' skirts come down outside it (their outer half to the seam): they sit
// flush with the tub, can't move inwards, and the seam has no straight path into the case. Six screws
// hold it all; the snap hooks went (2026-10-21: built upside down, nothing has to hold the top piece
// on the tub while the screws go in, and PLA hooks snapped)
z_seam = back_r;
tongue_h = 1.5;  seam_fit = 0.15;                       // the tongue's height; its clearance to the skirts
seam_gap = 0.1;                                         // the skirts stop this short of the tub at the seam, so the
                                                        // screws clamp the board, not the seam

// ---------------- stack (z) ----------------
floor_t = 1.5;  pcb_t = 1.6;  top_t = 2.0;               // top 2.0, not 1.5: the first print felt flimsy
board_lift = cell[2] + cell_gap;                         // the board sits this high over the tub's floor, on ribs
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
// behind, centres itself and can't come out of the front; its own screw (the end screw, through the
// board into a boss on the deck) pulls the wedges home. One part for both ends
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
deck_land = 0.8;  deck_gap = 0.15;                      // the joint: straight down from the face, then the wedge; its
                                                        // clearance (the inserts' 0.15)
step_wall_t = 1.2;                                      // the top piece's wall behind the undercut (1.2: room for
                                                        // the taller step of the 1.6 tuning height)
e0 = deck_face - deck_land;                             // the wedge's top edge
wedge_w = e0 - z_deck_under;                            // how far it reaches in under the top piece (45 degrees)
joint_w = wedge_w + deck_gap + step_wall_t;             // the band the joint takes inside the top piece's edge
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
cluster_spacing = case_w - piece_d - 2 * edge_margin;
band = edge_margin + piece_d + piece_to_glass;
case_l = bay_l + 2 * band;
inset = wall + board_clear;
board_w = case_w - 2 * inset;
board_l = case_l - 2 * inset;
// the speaker's centre, beside the left wall past the cell's seat-2 end, clear of the side screws
spk_pos = [-(case_w / 2 - wall - back_in(floor_t)) + spk_tab + 0.3 + spk[0] / 2, 31];
z_spk = floor_t + spk_pad;                              // its face, on the pad
spk_exit = spk_pos + [spk[0] / 2, 0];                   // where its leads leave it
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
pin_x = 31.4;             // beside the glass, between the bay and the board edge
pin_y = 22;               // the side screws: clear of the speaker (y 26 to 36), inside the step
pin_key = 3.0;            // the end bosses sit off the centre line (seat 1's board hole is there)
pin_out = 1.4;            // and this much further out than the clusters
// seat 2's end boss is seat 1's turned round (2026-10-20), so one deck fits both ends
pins = [[pin_x, pin_y], [-pin_x, pin_y], [pin_x, -pin_y], [-pin_x, -pin_y], [pin_key, seat_y + pin_out], [-pin_key, -seat_y - pin_out]];
boss_d = 5.0;  hole_d = 2.2;
// M2 x 8 self-tapping, wafer head (4 mm, 1 mm)
screw_cb = [4.4, 1.0];  screw_clear = 2.4;  screw_pilot = 1.6;
screw_engage = min(6.0, case_h - z_top_cu - 0.8);       // the pilot stops 0.8 under the face
deck_engage = deck_face - z_top_cu - 0.8;               // and 0.8 under the deck's face

// edge parts, all on the right (+X) long edge
usb_y = 0;   usb_w = 8.94;  usb_h = 3.25;               // TYPE-C-31-M-12 shell
usb_r = 1.0;                                            // its corner radius: a rounded rectangle with 1.2 mm flats
                                                        // on its ends, not a stadium (HRO's 3D model, 2026-10-20)
// The receptacle's face sits usb_recess under the case's surface, so a plug's overmold rests on
// the case and the opening only has to clear the receptacle's shell: the shell's shape (Alex,
// 2026-10-20: the stadium it used to be was "a different kind of oval"), usb_gap round it (Alex's fit
// test: the overmold-sized opening left a big gap round a port sunk 1.15 mm in the wall). To reach the
// surface the board grows a tab under the receptacle (its face stays 1.35 past the board's edge, as
// its footprint expects), which sits in a slot in the wall. The opening is split at the port's
// centre: the upper half in the top piece's skirt, the lower half in a tab of the tub that rises from
// the seam into a notch in the skirt (no roof over a hole, in either print pose). The seams round the
// tab step halfway through the wall (usb_step), so they don't run straight through it (Alex,
// 2026-10-20: light showed through them)
usb_recess = 0.15;  usb_gap = 0.25;
usb_hole = [usb_w + 2 * usb_gap, usb_h + 2 * usb_gap, usb_r + usb_gap];   // width, height, corner radius
usb_step = 1.0;                                         // the step in the tab's seams, inside the wall's outer half
usb_notch_w = 18.0;  usb_fit = 0.2;    // the notch in the top piece's skirt, and the tub's tab's clearance in it
usb_front = case_w / 2 - usb_recess;                    // receptacle face
usb_tab = usb_front - 1.35 - board_w / 2;              // the board's tab under it, past the board's edge
usb_tab_w = 12.0;  usb_tab_fit = 0.3;
// left long edge: debug
pin_pos = [[-(board_w / 2 - 4.0), 13.0], [-(board_w / 2 - 4.0), 6.0]];   // SW18 BOOTSEL, SW19 RESET (20 and 12
                                                                          // until the side screws moved to y 22)
swd_pos = [-(board_w / 2 - 3.5), -2.0];  uart_pos = [-(board_w / 2 - 3.5), -12.0];

// J2 sits under the spine between seat 1's clusters, its mouth towards the panel. The cell's
// plug has to pass up through the board (the cell lies under it), so the hole is the plug's
// size, not the wires'. The hole lies a plug's length plus a wire bend past J2's mouth (Alex,
// 2026-10-20: with the hole right at the mouth the wires had to double back over into it and
// could crimp): the plug comes up, lies flat and pushes into J2, and its wires run straight from
// its tail down through the hole, under the panel's edge (only wires there once it's plugged)
// J2 is a Molex PicoBlade 53261-0271, 3.4 mm tall (2026-10-19; the 5.5 mm JST PH needed a pad on
// the top once the screen face dropped, and the pad lifted the whole face off the print bed)
batt_conn = [0, -(bay_l / 2 + 8.2)];                   // J2's footprint origin
j2_front = 3.145;  j2_back = 3.745;                     // J2's courtyard, in front of and behind batt_conn
j2_w = 7.65;                                            // its body, across (3D model)
j2_h = 3.4;                                             // 53261-0271, from its 3D model
j2_mouth = 2.6;                                         // its mating face, in front of batt_conn (3D model, fab layer)
plug_xs = [4.25, 3.2];  plug_l = 4.15;                  // PicoBlade 51021-0200 housing: across the pins, through,
                                                        // and along the wires (distributor data; Molex SD-51021)
plug_out = 3.0;                                         // ESTIMATE: the mated plug's tail past J2's mouth (at least
                                                        // 1.2 of the housing inside J2's shroud)
wire_bend = 2.5;                                        // from the plug's tail to the hole: the wires turn down there
hole_fit = 0.6;                                         // each side, round the plug (the hole is drilled: exact)
plug_tail = batt_conn[1] + j2_mouth + plug_out;        // y of the mated plug's tail
slot = [[-(plug_xs[0] / 2 + hole_fit), plug_tail + wire_bend + plug_xs[1] + 2 * hole_fit],
        [plug_xs[0] / 2 + hole_fit, plug_tail + wire_bend]];           // hole corners (x0,y0)-(x1,y1)
cut_r = 0.6;                                            // its corner radius
// J4, the speaker's plug-in header (2026-10-20): J2's part, turned round under seat 2's spine,
// with the speaker's plug coming up through a hole the same size the same way. It sits as far
// from the cell's hole as the board allows, since the cell's plug fits it too and would put the
// cell across the amplifier's outputs
spk_conn = rot180(batt_conn);                           // J4's footprint origin
spk_slot = [for (c = [slot[1], slot[0]]) rot180(c)];    // its hole, corners as slot's
cell_lead = 60;                                         // ESTIMATE: the cell's leads, its end to its plug's tail
// the spine stops past J2 (or J4) and the band its walls take; the end screws sit in the deck past it
spine_end = abs(batt_conn[1]) + j2_back + 0.4 + joint_w;
// a pocket in the spine's underside over J2, up to j2_h plus 0.2, where the top piece's underside is lower
pocket = [9.4, 11.0];
skin_min = 0.6;
j2_top = z_top_cu + j2_h + 0.2;

// ---------------- checks ----------------
assert(cluster_spacing >= piece_d + 2, "a seat's two silicone pieces would touch");
assert(seat_y - piece_d / 2 >= bay_l / 2 + piece_to_glass - 1e-6, "cluster runs into the glass");
assert(cluster_spacing / 2 + piece_d / 2 <= board_w / 2, "silicone piece hangs off the board");
assert(glass_side >= 9.7, "no room for USB-C beside the glass");
assert(cell[0] / 2 <= pin_x - boss_d / 2 - 2, "cell runs into a boss");
assert(plug_tail <= -(panel_l / 2 + 1.5), "the mated plug runs into the panel's FPC fold");
assert(cell_y - cell[1] / 2 >= slot[0][1] + 1.5, "the cell lies over the plug's hole: its wires can't turn up");
assert(spk_slot[1][1] >= cell_y + cell[1] / 2 + 1.5, "the cell lies under the speaker's hole");
assert(j2_top + skin_min <= case_h, "no room for the mated plug under the spine");
assert(j2_w / 2 + 0.3 <= spine_w / 2 - joint_w, "J2 is wider than the spine's inside");
assert(pins[5][1] + boss_d / 2 + 0.2 <= batt_conn[1] - j2_back, "seat 1's end boss runs into J2");
assert(abs(slot[0][0]) <= spine_w / 2 - joint_w, "plug hole wider than the spine");
assert(spk_pos[0] + spk[0] / 2 <= -cell[0] / 2 - 0.5 || spk_pos[1] - spk[1] / 2 >= cell_y + cell[1] / 2 + 1, "the speaker runs into the cell");
assert(z_spk + spk[2] <= z_board - 0.3, "the speaker touches the board");
assert(spk_chamber >= 0.6 && spk_duct[0] >= spk_chamber && spk_duct[1] <= z_spk - 0.4, "the speaker's chamber or outlet leaves too thin a floor or roof");
for (p = [[-pin_x, pin_y], [-pin_x, -pin_y]])
  assert(abs(spk_pos[1] - p[1]) >= spk[1] / 2 + (boss_d + 1) / 2 + 0.3, "the speaker runs into a screw boss");
for (c = [dpad2]) assert(norm([max(0, abs(spk_pos[0] - c[0]) - spk[0] / 2), max(0, abs(spk_pos[1] - c[1]) - spk[1] / 2)])
                         >= (piece_d - 4) / 2 + 1.2 + 0.3, "the speaker's pad runs into the floor's ring under a cluster");
// its leads: out along the floor to below J4's hole, up through it, and to the plug's tail at J4
spk_reach = norm(spk_exit - (spk_slot[0] + spk_slot[1]) / 2) + (z_top_cu - z_spk - spk[2] / 2)
            + abs(spk_conn[1] - j2_mouth - plug_out - (spk_slot[0][1] + spk_slot[1][1]) / 2);
assert(spk_reach + 2 <= spk_lead, str("the speaker's leads don't reach J4: ", spk_reach, " mm of ", spk_lead));
// the cell's plug can't reach J4: its leads come from the cell's seat-1 end, under the board, up
// through its own hole at seat 1, and what is left of them doesn't reach J4's mouth (10 to spare)
cell_reach = cell_lead - norm([0, cell_y - cell[1] / 2] - [0, (slot[0][1] + slot[1][1]) / 2]) - (z_top_cu - floor_t - cell[2] / 2);
assert(norm([0, spk_conn[1] - j2_mouth] - [0, (slot[0][1] + slot[1][1]) / 2]) >= cell_reach + 10,
       str("the cell's plug could reach J4: ", cell_reach, " mm of lead past its hole"));
assert(pins[4][1] - boss_d / 2 - 0.2 >= spk_conn[1] + j2_back, "seat 2's end boss runs into J4");
assert(pin_x - boss_d / 2 >= bay_w / 2 + 0.5 && pin_x + boss_d / 2 <= board_w / 2, "no room for side bosses beside the glass");
assert(pin_y + (boss_d + 1) / 2 <= joint_y - joint_w, "a side boss runs into the step's wall");
for (p = pin_pos) for (q = pins) assert(norm(p - q) >= boss_d / 2 + 2.4 + 0.5, "a pinhole switch (SW18, SW19) runs into a screw boss");
assert(pin_pos[1][1] - 2.4 >= swd_pos[1] + 2.5 + 0.5, "the RESET switch runs into the SWD connector");
assert(panel_w + panel_fit <= bay_w && panel_l + panel_fit <= bay_l, "panel doesn't fit its bay");
assert(glass_gap >= 1.0, "no room under the panel");
assert(face_t >= 0.6 && face_t <= top_t, "face_t out of range");
assert(retainer_z < z_ceil, "button retainer above the top piece");
// the overhang limit and the rounds (case/SHELL.md): every round fits its wall
assert(overhang >= 30 && overhang <= 80, "overhang out of range");
assert(z_seam + tongue_h + seam_gap <= z_board - 0.2, "the tongue reaches the board");
assert(deck_face - face_r >= z_seam + tongue_h + seam_gap, "the decks' round runs into the seam's tongue");
assert(step_s(0) <= joint_w - 0.2, "the step runs past the joint's wall: the screen face over it would be too thin");
assert(wedge_w >= 0.8 && deck_land >= 0.6, "the joint's wedge is too small to hold a deck");
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
  "\"slot\": [", jpt(slot[0]), ", ", jpt(slot[1]), "], \"slot_r\": ", cut_r, ", ",
  "\"spk_slot\": [", jpt(spk_slot[0]), ", ", jpt(spk_slot[1]), "], \"spk_conn\": ", jpt(spk_conn), ", ",
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
  "\"batt_conn\": ", jpt(batt_conn), ", \"pocket\": [", pocket[0], ", ", pocket[1], "], \"pocket_limit\": ", jnum(j2_top - z_top_cu), ", ",
  "\"cell\": [", jpt([-cell[0] / 2 - 0.5, cell_y + cell[1] / 2 + 0.25]), ", ", jpt([cell[0] / 2 + 0.5, cell_y - cell[1] / 2 - 0.25]), "], ",
  "\"back_limit\": ", cell_gap, ", ",
  "\"speaker\": {\"c\": ", jpt(spk_pos), ", \"size\": [", spk[0], ", ", spk[1], "]}, ",
  "\"crush\": [", join([for (c = concat(crush_top, crush_deck(-1), crush_deck(1))) jpt(c)]), "], ",
  "\"frame\": {\"outer\": [", jpt([-(panel_w + panel_fit) / 2 - frame_wall, (panel_l + panel_fit) / 2 + frame_wall]), ", ",
    jpt([(panel_w + panel_fit) / 2 + frame_wall, -(panel_l + panel_fit) / 2 - frame_wall]), "], \"inner\": [",
    jpt([-(panel_w + panel_fit) / 2, (panel_l + panel_fit) / 2]), ", ", jpt([(panel_w + panel_fit) / 2, -(panel_l + panel_fit) / 2]),
    "], \"limit\": ", jnum(z_ceil - z_top_cu - frame_h - 0.3), "}, ",
  "\"decks\": [", join(concat([for (sy = [-1, 1], sx = [-1, 1]) str("[", jpt([sx * (spine_w / 2 - joint_w), sy * (joint_y - joint_w)]), ", ",
    jpt([sx * case_w / 2, sy * case_l / 2]), "]")], [for (sy = [-1, 1]) str("[", jpt([-spine_w / 2, sy * (spine_end - joint_w)]), ", ",
    jpt([spine_w / 2, sy * case_l / 2]), "]")])), "], \"deck_limit\": ", jnum(z_deck_under - z_top_cu - 0.3),
  "}"));
for (b = buttons) echo(str(b[0], "  ", b[1], "   KiCad (", kicad(b[2])[0], ", ", kicad(b[2])[1], ")"));
echo(str("CAPS flange top ", retainer_z - z_top_cu, " above the board, cap top ", cap_top - z_top_cu, ", deck face ",
         deck_face - z_top_cu, ", screen face ", case_h - z_top_cu, ": ends ", deck_face, " mm thick, a ", step_d, " mm step"));
echo(str("STACK floor ", floor_t, " + cell ", cell[2], " + gap ", cell_gap, " + PCB ", pcb_t, " + under the panel ", glass_gap,
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
// the case's outline at height z under a face at height h (the faces roll into the walls through face_r)
module outline_at(z, h) offset(delta = -edge_in(h - z, face_r)) outline2d();
// the top piece's outside at height z, in plan: a point is inside if, for some height h it reaches
// there, it is inside the outline as rounded from a face at h (so the side edges' round follows the
// step down: at the joint it is the decks' own round)
module top_layer2d(z) union() for (i = [0:ceil(step_d / lay)]) let(h = min(case_h, deck_face + lay * i)) if (h >= z)
  intersection() { offset(delta = -step_s(case_h - h)) top_region2d(); outline_at(z, h); }
module slot2d() translate(slot[0]) square([slot[1][0] - slot[0][0], slot[0][1] - slot[1][1]] * 1) ;
module board_outline() difference() {
  union() {
    rrect(board_w, board_l, board_r);
    if (usb_tab > 0) translate([board_w / 2 - 1, usb_y - usb_tab_w / 2]) square([usb_tab + 1, usb_tab_w]);
  }
  for (s = [slot, spk_slot]) translate([s[0][0], s[1][1]]) offset(r = cut_r) offset(delta = -cut_r)   // the cell's
    square([s[1][0] - s[0][0], s[0][1] - s[1][1]]);                                                // and speaker's plugs
}
if (part == "board") board_outline();

if (part == "layout") {
  color("#34343d") linear_extrude(0.05) outline2d();
  color("#4a4a56") translate([0, 0, 0.07]) linear_extrude(0.02) intersection() { outline2d(); top_region2d(); }
  color("#2e7d4f") translate([0, 0, 0.1]) linear_extrude(0.05) board_outline();
  color([0.85, 0.35, 0.35, 0.5]) translate([0, cell_y, 0.2]) linear_extrude(0.05) square([cell[0], cell[1]], center = true);
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
  color("#f08a3c") at(batt_conn + [0, (j2_front - j2_back) / 2], 0.8) linear_extrude(0.05) square([9.55, j2_front + j2_back], center = true);
  for (p = pin_pos) color("red") at(p, 0.9) linear_extrude(0.05) circle(d = 1.2);
}

// ---------------- 3D ----------------
in_x = case_w / 2 - wall;  in_y = case_l / 2 - wall;
// Crush ribs centre the board as it goes in face down: on the top piece's side walls (x) and the decks'
// end walls (y), [x, y, direction of the wall], each with a 45 degree lead-in at its lower end (the
// board comes up to them from the back)
crush_top = [[in_x, 28, 0], [in_x, -28, 0], [-in_x, 28, 180], [-in_x, -28, 180]];
function crush_deck(sy) = [[22, sy * in_y, sy * 90], [-22, sy * in_y, sy * 90]];
crush_reach = board_clear - 0.05;   // how far a rib stands off the wall: 0.05 mm short of the board
crush_z0 = z_seam + tongue_h + seam_gap;                // the skirts' inner half starts here (the tongue is below)
module crush_rib(c) at(c) rotate(c[2]) rotate([90, 0, 0]) linear_extrude(1.2, center = true)
  polygon([[0.01, crush_z0], [-0.01, crush_z0], [-crush_reach, crush_z0 + crush_reach], [-crush_reach, z_top_cu + 1.0], [0.01, z_top_cu + 1.0]]);

// what the floor's ribs keep clear of under the board: [x0, y0, x1, y1] (and the speaker's circle)
rib_free = [
  [-cell[0] / 2 - 0.5, cell_y - cell[1] / 2 - 0.5, cell[0] / 2 + 0.5, cell_y + cell[1] / 2 + 0.5],   // the cell
  [slot[0][0] - 1, slot[1][1] - 1, slot[1][0] + 1, cell_y - cell[1] / 2 + 1],                      // its plug and wires
  [spk_exit[0] - 1, min(spk_pos[1] - 3, spk_slot[1][1] - 1), spk_slot[1][0] + 1, spk_pos[1] + 3],  // speaker leads to J4's hole
  [spk_slot[0][0] - 1, spk_slot[1][1] - 1, spk_slot[1][0] + 1, spk_slot[0][1] + 1],                // and the hole itself
  [spk_pos[0] - spk[0] / 2 - spk_tab - 0.5, spk_pos[1] - 2.5, spk_pos[0] - spk[0] / 2 + 0.5, spk_pos[1] + 2.5],   // its solder tab
  [-10, seat_y - 14.5, 10, seat_y - 9.5],                                                         // TP1-TP5 on the back
  [usb_front - 8.5, usb_y - usb_tab_w / 2, board_w / 2 + usb_tab, usb_y + usb_tab_w / 2],         // J1's shell legs
];
module rib_free2d() {
  for (r = rib_free) translate([r[0], r[1]]) square([r[2] - r[0], r[3] - r[1]]);
  spk2d(1.2 + 0.5);
}
// the speaker's outline in the plan (grow: out from it), and the pad it lies on, out to the wall
module spk2d(grow = 0) at(spk_pos) hull() for (s = [-1, 1]) translate([s * (spk[0] - spk[1]) / 2, 0]) circle(d = spk[1] + 2 * grow);
module spk_pad2d() hull() { spk2d(1.2); translate([-case_w / 2, spk_pos[1] - spk[1] / 2 - 1.2]) square([1, spk[1] + 2.4]); }

// ---- the tub (the back): printed back down, as exported ----
// its outside: the back rolls into the walls (in 0.1 mm layers up the round), up to the seam
module tub_outside(h = z_seam) union() {
  for (k = [0:ceil(back_r / 0.1) - 1]) translate([0, 0, 0.1 * k])
    linear_extrude(0.1 + 0.01) offset(delta = -back_in(0.1 * k + 0.05)) outline2d();
  if (h > back_r) translate([0, 0, back_r]) linear_extrude(h - back_r) outline2d();
}
module tub() difference() {
  union() {
    difference() {
      tub_outside();
      // the cavity follows the round so the wall keeps its thickness through it (from floor_t, not
      // the layer's bottom: 0.1 * 14 + 0.1 > 1.5 in floating point, which cut the floor to 1.4)
      for (k = [0:ceil(back_r / 0.1) - 1]) if (0.1 * k + 0.1 > floor_t + 1e-6)
        translate([0, 0, max(0.1 * k, floor_t)]) linear_extrude(0.1 * k + 0.1 + 0.01 - max(0.1 * k, floor_t))
          offset(delta = -wall - back_in(0.1 * k + 0.05)) outline2d();
      translate([0, 0, back_r - 0.01]) linear_extrude(case_h) inner2d();
    }
    // the tongue: the wall's inner half, up past the seam inside the top pieces' skirts
    translate([0, 0, z_seam - 0.01]) linear_extrude(tongue_h + 0.01) difference() { inner2d(wall / 2 - seam_fit); inner2d(); }
    // the tab that carries the USB-C opening's lower half, up into the top piece's notch
    intersection() { usb_notch(usb_fit, 0.1); linear_extrude(case_h) outline2d(); }
    // bosses up to the board (the screws come up through them)
    for (s = pins) at(s) cylinder(d = boss_d + 1.0, h = z_board);
    // ribs under each silicone piece carry the button presses into the floor
    for (c = clusters) at(c) difference() {
      cylinder(d = piece_d - 4, h = z_board);
      translate([0, 0, -1]) cylinder(d = piece_d - 6, h = z_board + 2);
    }
    // the board rests on a grid of ribs board_lift high over the whole floor, clear of the cell,
    // the speaker, its pads and the test pads on the board's back, and the cell's wires
    translate([0, 0, floor_t - 0.01]) linear_extrude(board_lift + 0.01) difference() {
      offset(delta = -1.5) rrect(board_w, board_l, board_r);
      offset(delta = -2.7) rrect(board_w, board_l, board_r);
      rib_free2d();
    }
    translate([0, 0, floor_t - 0.01]) linear_extrude(board_lift + 0.01) difference() {
      intersection() {
        offset(delta = -1.5) rrect(board_w, board_l, board_r);
        union() {
          for (x = [-board_w / 2:support_pitch:board_w / 2]) translate([x - 0.6, -board_l / 2]) square([1.2, board_l]);
          for (y = [-board_l / 2:support_pitch:board_l / 2]) translate([-board_w / 2, y - 0.6]) square([board_w, 1.2]);
        }
      }
      rib_free2d();
    }
    // the speaker's pad, out to the wall and no further: cut to the case's plain outline, it stood
    // out of the back's round as a 0.6 mm slab, up to 1.0 mm proud (Alex's print, 2026-10-21)
    intersection() {
      translate([0, 0, floor_t - 0.01]) linear_extrude(spk_pad + 0.01) spk_pad2d();
      tub_outside();
    }
  }
  // screws: a counterbore for the head from below, then clearance up through the boss
  for (s = pins) at(s, -1) { cylinder(d = screw_cb[0], h = 1 + screw_cb[1]); cylinder(d = screw_clear, h = z_board + 2); }
  // the speaker's front chamber, under its face inside the foam ring, and its outlet: slots from
  // the chamber's wall end out through the wall's rounded lower edge
  translate([0, 0, spk_chamber]) linear_extrude(z_spk - spk_chamber + 0.01) spk2d(-spk_seal);
  for (i = [0:spk_slots - 1]) let(y = spk_pos[1] + (i - (spk_slots - 1) / 2) * (spk_slot_w + 0.8))
    translate([-case_w / 2 - 1, y - spk_slot_w / 2, spk_duct[0]])
      cube([spk_pos[0] - spk[0] / 2 + spk_seal + 2 + case_w / 2 + 1, spk_slot_w, spk_duct[1] - spk_duct[0]]);
  // USB-C: the opening's lower half, in the tab
  usb_opening();
  // the slot the board's tab drops into
  if (usb_tab > 0) translate([in_x - 0.01, usb_y - usb_tab_w / 2 - usb_tab_fit, z_board - 0.3])
    cube([usb_tab - board_clear + usb_tab_fit + 0.01, usb_tab_w + 2 * usb_tab_fit, case_h]);
}

usb_zc = z_top_cu + usb_h / 2;   // the port's centre
// the notch in the top piece's skirt under the port (fit 0: as cut from the top piece) or the tub's
// tab in it (fit, lift: its clearance each side and above): full width in the wall's inner half,
// usb_step narrower and lower in its outer half, so the top piece frames the tab outside
module usb_notch(fit = 0, lift = 0) let(o = fit == 0 ? 0.01 : 0, w = usb_notch_w - 2 * fit, dn = fit == 0 ? 1 : 0) {
  translate([in_x - o, usb_y - w / 2, z_seam - dn]) cube([wall / 2 + 2 * o, w, usb_zc - lift - z_seam + dn]);
  translate([in_x + wall / 2, usb_y - w / 2 + usb_step, z_seam - dn])
    cube([wall / 2 + dn, w - 2 * usb_step, usb_zc - usb_step - lift - z_seam + dn]);
}
module usb_opening() translate([in_x - 1, usb_y, usb_zc]) rotate([0, 90, 0])
  linear_extrude(wall + 2) rrect(usb_hole[1], usb_hole[0], usb_hole[2]);

// every ABXY cap: [centre, tab direction in OpenSCAD's counter-clockwise degrees]. Seat 2's
// cluster is seat 1's turned 180 degrees, caps and all
function abxy_keys() = [for (s = [[abxy1, 0], [abxy2, 180]]) for (i = [0:3]) let(t = abxy_tabs[i])
  [s[0] + (s[1] == 0 ? t[1] * a : -t[1] * a), s[1] - t[2], [for (n = abxy_notches[i][1]) t[2] - n]]];
// one A/B/X/Y cap's deck cut, in its own frame (tab along +x), from the pocket's floor up: the hole,
// the flange's pocket less its notch ribs, the tab's slot
// face: the deck's face over the cut's origin, where the 45 degree chamfer opens out (face down it
// prints as a 45 degree overhang, no support)
cut_face = deck_face - retainer_bot + 0.01;
module abxy_cut(h, notches, hole_d = cap_hole_d, face = cut_face) {
  cylinder(d = hole_d, h = h);
  translate([0, 0, face - cap_chamfer]) cylinder(d1 = hole_d, d2 = hole_d + 2 * cap_chamfer + 0.02, h = cap_chamfer + 0.01);
  difference() {
    cylinder(d = pocket_d, h = pocket_h + 0.01);
    if (notch_keys) for (n = notches) rotate(n) translate([cap_flange_d / 2 - notch_reach, -notch_w / 2, -1])
      cube([pocket_d / 2 - cap_flange_d / 2 + notch_reach + 0.05, notch_w, pocket_h + 2]);
  }
  linear_extrude(pocket_h + 0.01) polygon([[0, 0], for (i = [0:6]) let(t = slot_off - slot_a / 2 + slot_a * i / 6) slot_r * [cos(t), sin(t)]]);
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
// its outside, from the seam up: below the rounds a plain prism, then in layers
z_round0 = deck_face - face_r;                          // the rounds start here (the step's is shallower)
module top_outer() {
  translate([0, 0, z_seam + seam_gap]) linear_extrude(z_round0 - z_seam - seam_gap + 0.01)
    intersection() { outline2d(); top_region2d(); }
  for (k = [0:ceil((case_h - z_round0) / lay - 1e-6) - 1]) let(z = z_round0 + lay * k, h = min(lay, case_h - z))
    translate([0, 0, z]) linear_extrude(h + (z + h < case_h - 1e-6 ? 0.01 : 0)) top_layer2d(z + h / 2);
}
module top() difference() {
  union() {
    difference() {
      top_outer();
      // the inside, under the screen block and the spines, behind the joint's band
      translate([0, 0, z_seam - 1]) linear_extrude(z_ceil - z_seam + 1)
        intersection() { inner2d(); offset(delta = -joint_w) top_region2d(); }
      // nothing of the band below the decks' underside
      translate([0, 0, z_seam - 1]) linear_extrude(z_deck_under - z_seam + 1) intersection() { inner2d(); top_region2d(); }
      // the undercut the decks' wedges sit in (45 degrees: face down it prints as an upward face)
      for (k = [0:ceil(wedge_w / lay - 1e-6) - 1]) let(z = z_deck_under + lay * k)
        translate([0, 0, z - (k == 0 ? 0.01 : 0)]) linear_extrude(lay + 0.01 + (k == 0 ? 0.01 : 0)) intersection() {
          inner2d();
          difference() { offset(delta = 1) top_region2d(); offset(delta = -(e0 - z - lay / 2)) top_region2d(); }
        }
      // the skirts' inner half stops over the tub's tongue
      translate([0, 0, z_seam - 1]) linear_extrude(crush_z0 - z_seam + 1) inner2d(wall / 2);
    }
    // bosses down to the board top: with the tub's bosses they clamp it
    for (s = [for (i = [0:3]) pins[i]]) at(s, z_top_cu) cylinder(d = boss_d, h = z_ceil - z_top_cu + 0.01);
    // pinhole guides reach down towards the switches
    for (p = pin_pos) at(p, z_top_cu + 2.4) cylinder(d = 3.0, h = z_ceil - z_top_cu - 2.4 + 0.01);
    // panel locating frame
    translate([0, 0, z_ceil - frame_h]) linear_extrude(frame_h + 0.01) difference() {
      square([panel_w + panel_fit + 2 * frame_wall, panel_l + panel_fit + 2 * frame_wall], center = true);
      square([panel_w + panel_fit, panel_l + panel_fit], center = true);
      translate([fpc_x, -panel_l / 2]) square([fpc_w + 4, 6], center = true);   // the FPC's fold passes here
    }
    for (c = crush_top) crush_rib(c);
  }
  // USB-C: the opening's upper half, and below it the notch the tub's tab fills
  usb_opening();
  usb_notch(0);
  // the panel's pocket: its top face_t under the screen face
  if (face_t < top_t) translate([0, 0, z_ceil - 0.01]) linear_extrude(top_t - face_t + 0.01)
    square([panel_w + panel_fit, panel_l + panel_fit], center = true);
  // viewing window: the panel's own glass shows through, no lens
  translate([0, 0, z_ceil - 1]) linear_extrude(top_t + 2) window();
  // pilot holes for the screws
  for (s = [for (i = [0:3]) pins[i]]) at(s, z_top_cu - 0.01) cylinder(d = screw_pilot, h = screw_engage);
  // pinholes, and a pocket over J2 where the spine's underside is lower than the mated plug
  for (p = pin_pos) at(p, 0) cylinder(d = 1.2, h = case_h + 1);
  if (j2_top > z_ceil) for (c = [batt_conn, spk_conn]) at(c, z_ceil - 1) linear_extrude(j2_top - z_ceil + 1) square(pocket, center = true);
}

// ---- a deck (sy: -1 seat 1's end, 1 seat 2's): printed face down; seat 2's is seat 1's turned round ----
function end_pin(sy) = sy < 0 ? pins[5] : pins[4];
module deck(sy) difference() {
  union() {
    difference() {
      union() {
        // its face and outside: the outline's round from the deck face, out to the case's outline
        translate([0, 0, z_seam + seam_gap]) linear_extrude(z_round0 - z_seam - seam_gap + 0.01)
          intersection() { outline2d(); deck_region2d(sy); }
        for (k = [0:ceil(face_r / lay - 1e-6) - 1]) let(z = z_round0 + lay * k, h = min(lay, deck_face - z))
          translate([0, 0, z]) linear_extrude(h + (z + h < deck_face - 1e-6 ? 0.01 : 0))
            intersection() { outline_at(z + h / 2, deck_face); deck_region2d(sy); }
      }
      translate([0, 0, z_seam - 1]) linear_extrude(z_deck_under - z_seam + 1) inner2d();   // under the plate
      translate([0, 0, z_seam - 1]) linear_extrude(crush_z0 - z_seam + 1) inner2d(wall / 2);   // over the tongue
    }
    // the wedge along its inner edges, in under the top piece's undercut
    for (k = [0:ceil(wedge_w / lay - 1e-6) - 1]) let(z = z_deck_under + lay * k)
      translate([0, 0, z]) linear_extrude(lay + 0.01) intersection() {
        half2d(sy);
        inner2d(-deck_gap);
        difference() { offset(delta = deck_gap + 0.01) top_region2d(); offset(delta = -(e0 - z - lay / 2 - deck_gap)) top_region2d(); }
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
    // its boss, down to the board: the end screw pulls the deck onto the board and its wedges home
    at(end_pin(sy), z_top_cu) cylinder(d = boss_d, h = z_deck_under - z_top_cu + 0.01);
    for (c = crush_deck(sy)) crush_rib(c);
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
}

module board3d() color("#2e7d4f") translate([0, 0, z_board]) linear_extrude(pcb_t) board_outline();

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
    if (domes) dome(dome_h);
    linear_extrude(ss_well[2]) difference() { square(ss_well[0], center = true); square(ss_well[0] - 2 * ss_well[1], center = true); }
  }
}

if (part == "3d") {
  color("#3a3a44") tub();
  color("#c9c9d1") translate([0, cell_y, floor_t]) linear_extrude(cell[2]) square([cell[0], cell[1]], center = true);
  color("#b8912e") translate([0, 0, z_spk]) linear_extrude(spk[2]) spk2d();
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
