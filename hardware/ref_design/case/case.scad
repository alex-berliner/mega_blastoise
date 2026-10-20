// Reference design case: the single source of truth for every position the
// board depends on (outline, contacts, locating pins, connectors, height limits).
//
//   part="layout"   coloured top view
//   part="board"    board outline with its cable slot, 2D (DXF for Edge.Cuts)
//   part="3d"       assembly; explode=<mm> lifts the lid
//   part="tub", "lid"   printable parts (STL); the lid snaps onto the tub
//
// Frame: origin at the case centre, +Y towards seat 2 (seat 1 at -Y), z = 0 at
// the bottom of the tub. KiCad frame in the echoes: centre (100,100), Y down.
// tools/export.sh writes the echo block to positions.json for the board script.
//
// ESTIMATE marks a number that needs a real part and calipers before ordering.

part = "layout";
explode = 0;
$fn = 72;

// ---------------- bought parts ----------------
// EastRising ER-TFT035IPS-6 (ILI9488), from its datasheet's outline drawing
panel_w = 54.50;  panel_l = 83.00;  panel_t = 2.45;     // outline with the FPC folded; the drawing's side view says
                                                        // 2.35 +-0.1 (its table 2.3): the thickest
tape_t = 0.15;                                          // double-sided tape between the lid's window ledge and the panel
panel_fit = 0.8;   // the lid's locating frame is this much larger: the panel is +-0.2, FDM openings print small
frame_wall = 1.1;  frame_h = 1.0;                       // that frame's walls, hanging from the lid
aa_w = 48.96;     aa_l = 73.44;                         // active area
aa_to_top = 2.5;  aa_to_fpc = panel_l - aa_l - aa_to_top;   // active area to the glass edges (centred across)
aa_off = (aa_to_fpc - aa_to_top) / 2;                   // active area sits 2.28 mm towards seat 2
window_margin = 1.0;                                    // bezel opening past the active area: covers the panel's
                                                        // +-0.5 float in its frame, its +-0.3 active-area tolerance
                                                        // and a printed opening's error; at most 0.5 mm of the
                                                        // panel's black border shows
// Its FPC leaves the seat-1 edge, folds under the panel and plugs into a 50-pin ZIF
// connector (J3, Hirose FH12-50S, bottom contact). Seen from above, x from the panel's
// centre, lengths along the FPC from that edge: a 4.59 mm neck (35.6 wide, centre +3.6)
// wraps the edge, then a 14.41 mm body (47.9 wide, centre +2.5) lies under the panel,
// then an 11.7 mm tail (25.5 wide, centred) ends in the 3.5 mm contact strip.
fpc_neck = [3.6, 35.6, 4.59];   // ESTIMATE: width and offset scaled from the drawing's views (not dimensioned)
fpc_body = [2.5, 47.9, 14.41];   fpc_tail = [0, 25.5, 11.7];
fpc_slack = 1.0;                                        // the connector 1 mm short of the tail's tip: a
                                                        // long FPC bows, a short one can't reach (ESTIMATE)
conn_rear = 1.2;  conn_front = 4.4;                     // FH12 footprint origin to its back and to its mouth
tip_in = fpc_body[2] + fpc_tail[2];                     // tip, in from the edge (the neck is the fold)
// The panel's bay: the case and board layout are built around it, so a panel that
// fits the bay (with its tape and a 0.25 mm gap each side) moves nothing else.
bay_w = 56.0;     bay_l = 86.24;

// The cell lies on the tub's floor under the board, lengthwise, its wires leaving its seat-1
// end; its plug comes up through a hole in the board to J2 under the spine between seat 1's
// clusters, and the panel sits 2.4 mm over the board. 2026-10-05 to -09 it lay on the board
// under the screen, for a 13.9 mm case: branch ref-design-cell-on-board.
cell = [34, 50, 4];                                     // 403450 Li-ion, x by y (length) by thickness;
                                                        // ESTIMATE: length with its protection board
cell_y = -17;                                           // centre: its wire end 42 mm towards seat 1
cell_gap = 0.5;                                         // its top to the board's back
// the speaker lies on the floor too, beside the cell, firing down through the tub's grille;
// its leads go to pads on the board's back (J4)
spk_d = 15;  spk_h = 4.0;                               // ESTIMATE: 15 mm 8 ohm speaker
spk_pos = [0, 26];                                      // its centre, past the cell's seat-2 end

abxy_pitch = 15.0;  dpad_pitch = 14.0;                  // DS Lite (gerbers)
piece_d = 26;                                           // ESTIMATE: silicone piece outline
// Genuine DS Lite caps, measured from open models of them and of shells built for them
// (jackw01/pi-tin, CERN-OHL-W; Printables 1178557, 205921, 986494): see case/DSLITE.md
cap_abxy_d = 7.0;  cap_flange_d = 8.8;  cap_h = 3.56;  // ABXY: body, flange, flange bottom to top
cap_tab_r = 5.6;  cap_tab_w = 2.0;                      // each flange's keying tab, out from the cap's centre
dpad_arm_l = 17.8;  dpad_arm_w = 5.9;  dpad_h = 3.34;   // D-pad cross
dpad_flange_r = 6.5;                                    // its flange fills the corners between the arms
flange_t = 0.55;                                        // both caps' flanges
cap_hole_d = 7.6;  pocket_d = 9.2;                      // lid: hole (the DS Lite's own 7.6; Pi Tin 7.34), flange pocket
dpad_gap = 0.8;           // D-pad opening past the cross each side: the DS Lite's 18.9 mm opening is 0.55, and
                          // 0.5 was too tight for Alex's AliExpress D-pad in the first print (tune: tune_openings)
dpad_pocket_gap = 0.5;    // its flange's pocket past the flange (0.35 bound with the cross)
dpad_lift = 0.35;         // raises (or, negative, lowers) the D-pad's retaining lip against the A/B/X/Y lips: the
                          // D-pad sat at the wrong height in the first print (tune: tune_stack_dpad_*)
slot_r = 5.95;  slot_a = 30;                            // tab slot in the pocket, a 30 degree wedge: the cap
                                                        // turns a few degrees at most, and the lid still goes on
                                                        // over caps set by eye (facelesstech's are 35)
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
                                                        // agree, 2026-10-19); 1.2 left the silicone no room

// ---------------- layout ----------------
glass_side = 9.7;         // panel bay edge to long case edge: room for USB-C beside the glass
edge_margin = 4;          // case edge to silicone piece
piece_to_glass = 5;       // silicone piece to glass edge
wall = 2.0;  board_clear = 0.5;  corner_r = 8;
support_pitch = 12;       // the floor's support ribs under the board

// ---------------- stack (z) ----------------
floor_t = 1.5;  pcb_t = 1.6;  lid_t = 2.0;               // lid 2.0, not 1.5: the first print felt flimsy
board_lift = cell[2] + cell_gap;                         // the board sits this high over the tub's floor, on ribs
glass_gap = 2.4;          // panel back above the board: J3 (2.0 mm) under the panel keeps 0.4 mm against
                          // the connector's and a printed lid's tolerances (Alex, 2026-09-26)
// The panel's top sits in a pocket in the lid's underside, under a skin face_t thick over its
// black border, so the screen face is only face_t above the glass instead of lid_t (2026-10-19,
// to lower the ridge to the button decks: 3.54 -> 2.34 mm, Alex's option A). face_t = lid_t is
// the old flat lid with no pocket. The lid over J2 then needs a pad (j2_bump, below)
face_t = 0.8;
bay_t = glass_gap + panel_t + tape_t + face_t - lid_t;  // board top to the lid's underside (outside the panel)
// The FPC's lengths were fitted to a panel 2.4 mm above the board; a higher panel spends the
// extra drop on the way down to J3, so J3 moves that much closer to the edge (the safe side:
// a long FPC bows, a short one can't reach)
conn_in = tip_in - fpc_slack - conn_rear - (glass_gap - 2.4);   // connector origin, in from the edge
z_board = floor_t + board_lift;                         // board bottom
z_top_cu = z_board + pcb_t;                             // board top: every height limit counts from here
z_glass = z_top_cu + glass_gap;
case_h = z_glass + panel_t + tape_t + face_t;
z_lid = case_h - lid_t;                                 // lid underside (the panel's pocket goes higher)
retainer_z = z_top_cu + dome_h + flange_t;              // the lip the flanges rest against
// Under the deck, as in a DS Lite shell (measured from a scan of an aftermarket shell and the Game
// Boy Macro faceplates, 2026-10-19): no solid block, just ribs that rest on each silicone pad's
// flat web and locate it. A/B/X/Y: a ring round each flange pocket (the rings merge) and a walled
// pocket for the pad's raised centre square; D-pad: a four-lobed ring round its pad (its cap's peg
// through the pad's centre hole does the rest); Start/Select: a square frame round each cap.
// Elsewhere the deck's underside clears the pads. The flange pockets run from the ribs' bottom up
// to the lip. ESTIMATE until Alex measures his pads: the web (sil_rib_gap; the sources say 0.25
// to 1.0, Macro F, a printed shell, uses 0.8)
sil_rib_gap = 0.8;                                      // the ribs' bottom over the board: room for the web
retainer_bot = z_top_cu + sil_rib_gap;
pocket_h = retainer_z - retainer_bot;
abxy_ring = 5.55;                                       // A/B/X/Y rings' outer radius, round each hole (scan, Macro)
abxy_square = [5.15, 0.65, 2.1];                        // centre square's pocket: inside, wall, roof over the board
dpad_ring = [4.95, 0.8, 7.0];                           // D-pad ring: lobe radius inside, width, waist radius
ss_frame_w = 0.9;                                       // Start/Select frames' walls
cap_top = retainer_z + cap_h - flange_t;                // at rest
// Stepped ends: the lid face over the screen is set by the panel stack, which leaves genuine
// caps below it. So each end drops to a button deck where the caps stand as on a DS Lite,
// except a spine between the clusters (J2 and seat 1's screw boss sit under it), which stops
// short of the end so Start and Select sit in the deck below it
cap_proud = 1.85;                                       // above the deck: keeps the deck 3.46 over the board, a
                                                        // DS Lite's 3.45 to 3.67 (Pi Tin's caps stand 1.3)
deck_face = cap_top - cap_proud;                        // lip over the flanges: 1.16, DS Lite shells 1.08 to 1.25
z_deck_lid = deck_face - lid_t;                         // deck underside; the tub's walls stop here
step_y = bay_l / 2 + 2.4;                               // the step: clear of the panel and its FPC fold
step_wall = 1.5;  spine_w = 13.0;  deck_r = 3;          // step wall, spine width, deck corner rounding
// The screen block meets each deck in a smooth S-curve (smoothstep) for the thumbs, not a
// chamfer and a wall (Alex, 2026-10-19: the jump down was too sharp). It starts ramp_out out on
// the deck (short of the cap retainers) and ends ramp_in into the screen block (where the skin
// over the panel's pocket still has 0.6 mm); the raised pad over J2 keeps its edge, since cutting
// into it would open J2's pocket
ramp_out = 1.5;  ramp_in = 3.0;  ramp_layers = 48;      // 0.05 mm layers: no terraces at print resolution
function inv_smoothstep(v) = 0.5 - sin(asin(1 - 2 * v) / 3);

// EXPERIMENT (Alex, 2026-10-19): deck inserts. Face down, the decks print 2.3 mm above the bed, so
// they need supports, and a face printed on supports keeps a peel-away texture. With deck_inserts
// each end's deck (both clusters and Start/Select) is its own U-shaped part that prints face down
// on the bed like the screen face: no supports anywhere. Its inner edges carry a 45 degree wedge
// that the lid's step walls hold down (insert_tongue); its outer end edge tucks under a 45 degree lip on the
// tub's end wall (tilt it in, then lower it). The tub's walls rise to deck height round it. The
// step becomes a rounded top edge and a 45 degree slope (step_round), which prints unsupported.
deck_inserts = false;
insert_gap = 0.15;                                      // clearance round the insert
insert_tongue = 0.8;                                    // the wedge under the lid's step walls: width at its foot
end_lip = 0.8;                                          // the tub end wall's lip over the insert (45 degrees)
step_round = 1.0;                                       // radius of the step's top edge
// the inserts' step profile: how far into the screen block (t) the surface is at depth d under the face
round_d = step_round * (1 - 1 / sqrt(2));               // where the round meets the 45 degree slope
round_w = sqrt(2 * step_round * round_d - round_d * round_d);
step_t0 = round_w + (case_h - deck_face) - round_d;     // the slope's top edge, in from the deck's edge
function step45_t(d) = d <= round_d ? step_t0 - sqrt(max(0, 2 * step_round * d - d * d))
                                    : step_t0 - round_w - (d - round_d);
// how far into the screen block (negative: out on the deck) the surface is at layer k
function ramp_t(k) = deck_inserts
  ? step45_t((case_h - deck_face) * (1 - (k + 0.5) / ramp_layers))
  : -ramp_out + (ramp_out + ramp_in) * inv_smoothstep((k + 0.5) / ramp_layers);
// Start and Select (2026-10-19): each seat's DS Lite Start/Select membrane (it comes in the same
// pad kit: one piece, two pills) between its clusters, near the case's end, Start on the
// player's right. Seat 1's Start is the power button. ESTIMATE: membrane, pill pitch and caps
ss_pitch = 7.5;  ss_mem = [14, 6];                      // pill to pill, membrane outline
ss_cap_d = 4.5;  ss_flange_d = 6.0;  ss_cap_h = 3.0;    // cap body, flange, flange bottom to top
ss_gap = 0.3;                                           // hole past the cap each side

// ---------------- derived ----------------
case_w = bay_w + 2 * glass_side;
cluster_spacing = case_w - piece_d - 2 * edge_margin;
band = edge_margin + piece_d + piece_to_glass;
case_l = bay_l + 2 * band;
inset = wall + board_clear;
board_w = case_w - 2 * inset;
board_l = case_l - 2 * inset;
board_r = corner_r - inset;
seat_y = case_l / 2 - edge_margin - piece_d / 2;
spine_end = seat_y + 4.9;                               // the spine stops here, past seat 1's screw boss
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

// Bosses that clamp the board: four beside the glass in the side bezels, two between each
// seat's clusters; a tub boss under the board meets a lid boss above it. (They were printed
// pins through the board's holes: 1.9 mm FDM pins bound and snapped off, so the first print
// dropped them. Crush ribs on the tub's walls now centre the board.) The board goes in one
// way only anyway: turned round, J1's receptacle hits the wall away from the USB opening,
// and the lid turned round lands on J2, which has its pocket at seat 1's end only.
pin_x = 31.4;             // midway between the bay and the board edge
pin_key = 3.0;            // seat 1's centre boss sits off the centre line (the board's hole is there)
pin_out = 1.4;            // seat 1's centre boss sits this much further out, behind J2
pins = [[pin_x, 36], [-pin_x, 36], [pin_x, -36], [-pin_x, -36], [0, seat_y], [-pin_key, -seat_y - pin_out]];
boss_d = 5.0;  hole_d = 2.2;  pin_d = 1.9;
// As the DS Lite closes: screws from the back, through the tub's bosses and the board's holes,
// into the lid's bosses, clamping the board between them (the snap hooks only hold the lid
// while it is screwed). M2 x 8 self-tapping, wafer head (4 mm, 1 mm)
screw_cb = [4.4, 1.0];  screw_clear = 2.4;  screw_pilot = 1.6;
screw_engage = min(6.0, case_h - z_top_cu - 0.8);       // the pilot stops 0.8 under the face (it came through it
                                                        // once the screen face dropped to 5.8 over the board)
// Stepped seam, as the DS Lite's face nests in its back shell: a lip under the lid's edge drops
// into a step in the inner half of the tub's wall. It locates the lid and hides the seam
lip_h = 1.2;  lip_fit = 0.2;

// edge parts, all on the right (+X) long edge
usb_y = 0;   usb_w = 8.94;  usb_h = 3.25;               // TYPE-C-31-M-12 shell
// The receptacle's face sits usb_recess under the case's surface, so a plug's overmold rests on
// the case and the opening only has to clear the receptacle's shell: a stadium the port's shape,
// usb_gap round it (Alex's fit test: the overmold-sized opening left a big gap round a port
// sunk 1.15 mm in the wall). To reach the surface the board grows a tab under the receptacle
// (its face stays 1.35 past the board's edge, as its footprint expects), which sits in a slot
// in the tub wall. The opening is split at the port's centre: the lower half in the tub wall,
// the upper half in a tongue of the lid that fills a notch in the wall (no roof over a hole)
usb_recess = 0.15;  usb_gap = 0.25;
usb_hole = [usb_w + 2 * usb_gap, usb_h + 2 * usb_gap, (usb_h + 2 * usb_gap) / 2 - 0.01];   // width, height, end radius
usb_tongue_w = 18.0;  usb_fit = 0.2;   // the lid's tongue, and its clearance each side in the notch
usb_front = case_w / 2 - usb_recess;                    // receptacle face
usb_tab = usb_front - 1.35 - board_w / 2;              // the board's tab under it, past the board's edge
usb_tab_w = 12.0;  usb_tab_fit = 0.3;
// left long edge: debug
pin_pos = [[-(board_w / 2 - 4.0), 20.0], [-(board_w / 2 - 4.0), 12.0]];  // SW18 BOOTSEL, SW19 RESET
swd_pos = [-(board_w / 2 - 3.5), -2.0];  uart_pos = [-(board_w / 2 - 3.5), -12.0];

// J2 sits under the spine between seat 1's clusters, its mouth towards the panel. The cell's
// plug has to pass up through the board (the cell lies under it), so the hole is the plug's
// size, not the wires'. The hole lies right in front of J2's mouth, so the plug comes up, turns
// and goes in, and its wires drop back through the hole from the plug's tail.
// J2 is a Molex PicoBlade 53261-0271, 3.4 mm tall (2026-10-19; the 5.5 mm JST PH needed a pad on
// the lid once the screen face dropped, and the pad lifted the whole face off the print bed)
batt_conn = [0, -(bay_l / 2 + 8.2)];                   // J2's footprint origin
j2_front = 3.145;  j2_back = 3.745;                     // J2's courtyard, in front of and behind batt_conn
j2_h = 3.4;                                             // 53261-0271, from its 3D model
plug_xs = [4.3, 3.3];                                   // ESTIMATE: PicoBlade 51021-0200 housing end on: across
                                                        // the pins, through
hole_fit = 0.6;                                         // each side, round the plug (the hole is drilled: exact)
slot = [[-(plug_xs[0] / 2 + hole_fit), batt_conn[1] + j2_front + 0.2 + plug_xs[1] + 2 * hole_fit],
        [plug_xs[0] / 2 + hole_fit, batt_conn[1] + j2_front + 0.2]];    // hole corners (x0,y0)-(x1,y1)
cut_r = 0.6;                                            // its corner radius
// lid pocket over J2: up to j2_h plus 0.2. When that leaves less than skin_min under the screen
// face (a thin face_t), a pad rises over it on the spine
pocket = [9.4, 11.0];
skin_min = 0.6;
j2_top = z_top_cu + j2_h + 0.2;
j2_bump = max(0, j2_top + skin_min - case_h);

// ---------------- checks ----------------
assert(cluster_spacing >= piece_d + 2, "a seat's two silicone pieces would touch");
assert(seat_y - piece_d / 2 >= bay_l / 2 + piece_to_glass - 1e-6, "cluster runs into the glass");
assert(cluster_spacing / 2 + piece_d / 2 <= board_w / 2, "silicone piece hangs off the board");
assert(cell[0] / 2 <= pin_x - boss_d / 2 - 2, "cell runs into a boss");
assert(slot[0][1] <= -(panel_l / 2 + 1.9), "plug hole under the panel's FPC fold");
assert(norm(pins[5] - batt_conn) >= 0 && pins[5][1] + boss_d / 2 + 0.2 <= batt_conn[1] - j2_back, "seat 1's centre boss runs into J2");
assert(abs(slot[0][0]) <= spine_w / 2 - step_wall, "plug hole wider than the spine");
assert(spk_pos[1] - spk_d / 2 >= cell_y + cell[1] / 2 + 2, "speaker runs into the cell");
assert(spk_h <= board_lift - 0.3, "speaker touches the board");
assert(pin_x - boss_d / 2 >= bay_w / 2 + 0.5 && pin_x + boss_d / 2 <= board_w / 2, "no room for pin bosses beside the glass");
assert(panel_w + panel_fit <= bay_w && panel_l + panel_fit <= bay_l, "panel doesn't fit its bay");
assert(glass_gap >= 1.0, "no room under the panel");
assert(face_t >= 0.6 && face_t <= lid_t, "face_t out of range");
assert(retainer_z < z_lid, "button retainer above the lid");
assert(z_deck_lid > retainer_bot + 0.5, "button deck lower than the retainers");
assert(step_y - step_wall > (panel_l + panel_fit) / 2 + frame_wall + 0.3, "step over the panel frame");
for (s = pins) for (c = clusters) assert(norm(s - c) >= piece_d / 2 + boss_d / 2, "pin under a silicone piece");
assert(pin_key + boss_d / 2 + 0.5 <= spine_w / 2, "seat 1's centre boss runs out under a button deck");
assert(seat_y + pin_out + boss_d / 2 <= spine_end, "seat 1's screw boss runs out under the end deck");
assert(ss_y - ss_mem[1] / 2 - 0.75 >= spine_end, "Start/Select under the spine");
assert(ss_y + ss_mem[1] / 2 + 1 <= board_l / 2, "Start/Select off the board's end");
for (c = clusters) assert(norm([ss_mem[0] / 2, abs(abs(c[1]) - ss_y) < ss_mem[1] / 2 ? 0 : abs(abs(c[1]) - ss_y) - ss_mem[1] / 2] - [abs(c[0]), 0]) >= piece_d / 2 + 0.5, "Start/Select runs into a silicone piece");

// ---------------- echo: everything the board script needs ----------------
// a snap hook's footprint (two corners) from its wall position and direction
function hook_rect(h) = let(c = cos(h[2]), sn = sin(h[2]), u0 = -snap_gap - snap_t_root - snap_fillet, v0 = -h[3] / 2, v1 = h[3] / 2)
  [[h[0] + u0 * c - v0 * sn, h[1] + u0 * sn + v0 * c], [h[0] - v1 * sn, h[1] + v1 * c]];
function jnum(v) = str(round(v * 1000) / 1000);
function jpt(p) = str("[", jnum(kicad(p)[0]), ", ", jnum(kicad(p)[1]), "]");
function join(l, i = 0) = i >= len(l) ? "" : str(i ? ", " : "", l[i], join(l, i + 1));
echo(str("CASE ", case_w, " x ", case_l, " x ", case_h, " mm   board ", board_w, " x ", board_l, " r ", board_r));
echo(str("JSON {",
  "\"board\": {\"w\": ", jnum(board_w), ", \"l\": ", jnum(board_l), ", \"r\": ", jnum(board_r), ", \"t\": ", pcb_t, "}, ",
  "\"slot\": [", jpt(slot[0]), ", ", jpt(slot[1]), "], \"slot_r\": ", cut_r, ", ",
  "\"glass\": [", jpt([-bay_w / 2, bay_l / 2]), ", ", jpt([bay_w / 2, -bay_l / 2]), "], ",
  "\"glass_limit\": ", glass_gap - 0.2, ", ",
  "\"bezel_limit\": ", z_lid - z_top_cu - 0.3, ", ",
  "\"retainer_limit\": ", retainer_bot - z_top_cu - 0.3, ", ",
  "\"fpc_zones\": [[", jpt([fpc_tail[0] - fpc_tail[1] / 2 - 0.5, -(panel_l / 2 - (conn_in - conn_front))]), ", ",
    jpt([fpc_tail[0] + fpc_tail[1] / 2 + 0.5, -(panel_l / 2 - fpc_body[2])]), "], [",
    jpt([fpc_body[0] - fpc_body[1] / 2 - 0.5, -(panel_l / 2 - fpc_body[2])]), ", ",
    jpt([fpc_body[0] + fpc_body[1] / 2 + 0.5, -(panel_l / 2 + 1.5)]), "]], ",
  "\"fpc_conn\": {\"x\": ", jnum(kicad([fpc_tail[0], 0])[0]), ", \"y\": ", jnum(kicad([0, -(panel_l / 2 - conn_in)])[1]), "}, ",
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
  "\"batt_conn\": ", jpt(batt_conn), ", \"pocket\": [", pocket[0], ", ", pocket[1], "], \"pocket_limit\": ", j2_top - z_top_cu, ", ",
  "\"cell\": [", jpt([-cell[0] / 2 - 0.5, cell_y + cell[1] / 2 + 0.25]), ", ", jpt([cell[0] / 2 + 0.5, cell_y - cell[1] / 2 - 0.25]), "], ",
  "\"back_limit\": ", cell_gap, ", ",
  "\"speaker\": {\"c\": ", jpt(spk_pos), ", \"d\": ", spk_d, "}, ",
  "\"hooks\": [", join([for (h = hooks) str("[", jpt(hook_rect(h)[0]), ", ", jpt(hook_rect(h)[1]), "]")]), "], ",
  "\"frame\": {\"outer\": [", jpt([-(panel_w + panel_fit) / 2 - frame_wall, (panel_l + panel_fit) / 2 + frame_wall]), ", ",
    jpt([(panel_w + panel_fit) / 2 + frame_wall, -(panel_l + panel_fit) / 2 - frame_wall]), "], \"inner\": [",
    jpt([-(panel_w + panel_fit) / 2, (panel_l + panel_fit) / 2]), ", ", jpt([(panel_w + panel_fit) / 2, -(panel_l + panel_fit) / 2]),
    "], \"limit\": ", z_lid - z_top_cu - frame_h - 0.3, "}, ",
  "\"hook_clear\": ", bay_t - snap_len, ", ",
  "\"decks\": [", join(concat([for (sy = [-1, 1], sx = [-1, 1]) str("[", jpt([sx * (spine_w / 2 - step_wall), sy * (step_y - step_wall)]), ", ",
    jpt([sx * case_w / 2, sy * case_l / 2]), "]")], [for (sy = [-1, 1]) str("[", jpt([-spine_w / 2, sy * (spine_end - step_wall)]), ", ",
    jpt([spine_w / 2, sy * case_l / 2]), "]")])), "], \"deck_limit\": ", z_deck_lid - z_top_cu - 0.3, ", ",
  "\"lid_ribs\": [", join([for (r = lid_ribs) str("[", jpt([r[0] - 0.6, r[1]]), ", ", jpt([r[0] + 0.6, r[2]]), "]")]), "], ",
  "\"rib_limit\": ", z_lid - rib_h - z_top_cu - 0.3,
  "}"));
for (b = buttons) echo(str(b[0], "  ", b[1], "   KiCad (", kicad(b[2])[0], ", ", kicad(b[2])[1], ")"));
echo(str("CAPS flange top ", retainer_z - z_top_cu, " above the board, cap top ", cap_top - z_top_cu, ", deck face ",
         deck_face - z_top_cu, ", screen face ", case_h - z_top_cu, ": ends ", deck_face, " mm thick, a ", case_h - deck_face, " mm step"));
echo(str("STACK floor ", floor_t, " + cell ", cell[2], " + gap ", cell_gap, " + PCB ", pcb_t, " + under the panel ", glass_gap,
         " + panel ", panel_t, " + tape ", tape_t, " + lid over the panel ", face_t, " = ", case_h, " mm; lid underside ",
         bay_t, " over the board; J2 pad ", j2_bump, " mm"));

// ---------------- 2D ----------------
module rrect(w, l, r) offset(r = r) square([w - 2 * r, l - 2 * r], center = true);
module at(p, z = 0) translate([p[0], p[1], z]) children();
module ring(t = 0.35) difference() { offset(r = t) children(); children(); }
// the D-pad's pad as its ring sees it: a lobe round each dome and the waist between them
module dpad_clover() union() {
  circle(r = dpad_ring[2], $fn = 64);
  for (t = [0:90:270]) rotate(t) translate([d, 0]) circle(r = dpad_ring[0], $fn = 48);
}
// a silicone pad's outline, at its cluster's centre: a lobe round each dome on the buttons' axes
abxy_lobe_r = 6.0;  abxy_waist_r = 9.2;  dpad_lobe_r = 5.0;  dpad_waist_r = 7.9;   // ESTIMATE (HHL photos)
ss_pad = [15.8, 8.0];
module pad2d(c) let(dp = c == dpad1 || c == dpad2) union() {
  circle(r = dp ? dpad_waist_r : abxy_waist_r, $fn = 64);
  for (t = [0:90:270]) rotate(t) translate([(dp ? dpad_pitch : abxy_pitch) / 2, 0])
    circle(r = dp ? dpad_lobe_r : abxy_lobe_r, $fn = 48);
}
module cross(w, l) union() { square([l, w], center = true); square([w, l], center = true); }
module window() translate([0, aa_off]) square([aa_w + 2 * window_margin, aa_l + 2 * window_margin], center = true);
// the two lowered decks at each end (past the case outline; intersect with it), and the
// band the tub's walls drop in: the decks plus the step walls
module deck2d() for (sy = [-1, 1]) scale([1, sy]) offset(r = deck_r) offset(delta = -deck_r) union() {
  for (sx = [-1, 1]) scale([sx, 1]) translate([spine_w / 2, step_y]) square([case_w, case_l]);
  translate([-case_w, spine_end]) square([2 * case_w, case_l]);    // across the end, below the spine
}
module deck_cut2d() offset(delta = step_wall) deck2d();
// the inner half of the case wall, along the full-height (screen-section) edges only: not at the
// decks (lower walls) or over the USB-C notch (the lid's tongue). fit shrinks it from the outside
module seam_band(fit) difference() {
  offset(delta = -wall / 2 - fit) rrect(case_w, case_l, corner_r);
  offset(delta = -wall - 0.01) rrect(case_w, case_l, corner_r);
  offset(delta = deck_fit + 1) deck_cut2d();
  translate([case_w / 2 - wall - 1, usb_y - usb_tongue_w / 2 - 1]) square([wall + 2, usb_tongue_w + 2]);
}
module slot2d() translate(slot[0]) square([slot[1][0] - slot[0][0], slot[0][1] - slot[1][1]] * 1) ;
module board_outline() difference() {
  union() {
    rrect(board_w, board_l, board_r);
    if (usb_tab > 0) translate([board_w / 2 - 1, usb_y - usb_tab_w / 2]) square([usb_tab + 1, usb_tab_w]);
  }
  translate([slot[0][0], slot[1][1]]) offset(r = cut_r) offset(delta = -cut_r)   // the cell plug's hole
    square([slot[1][0] - slot[0][0], slot[0][1] - slot[1][1]]);
}
if (part == "board") board_outline();

if (part == "layout") {
  color("#34343d") linear_extrude(0.05) rrect(case_w, case_l, corner_r);
  color("#2e7d4f") translate([0, 0, 0.1]) linear_extrude(0.05) board_outline();
  color([0.85, 0.35, 0.35, 0.5]) translate([0, cell_y, 0.2]) linear_extrude(0.05) square([cell[0], cell[1]], center = true);
  color([0.9, 0.7, 0.2, 0.6]) at(spk_pos, 0.2) linear_extrude(0.05) circle(d = spk_d);
  color("#101014") translate([0, 0, 0.3]) linear_extrude(0.05) square([panel_w, panel_l], center = true);
  color("#3b6fd1") translate([0, 0, 0.4]) linear_extrude(0.05) window();
  color("#c9a227") translate([0, 0, 0.45]) linear_extrude(0.05) {
    translate([fpc_body[0], -(panel_l / 2 - fpc_body[2] / 2)]) square([fpc_body[1], fpc_body[2]], center = true);
    translate([fpc_tail[0], -(panel_l / 2 - fpc_body[2] - fpc_tail[2] / 2)]) square([fpc_tail[1], fpc_tail[2]], center = true);
  }
  for (c = clusters) color([0.85, 0.85, 0.9, 0.45]) at(c, 0.5) linear_extrude(0.05) circle(d = piece_d);
  for (c = ss_centres) color([0.85, 0.85, 0.9, 0.45]) at(c, 0.5) linear_extrude(0.05) rrect(ss_mem[0], ss_mem[1], 2);
  for (b = concat(buttons, ss_buttons)) color("#f2c94c") at(b[2], 0.6) linear_extrude(0.05) circle(d = 4.8);
  for (s = pins) color("white") at(s, 0.7) linear_extrude(0.05) ring(0.4) circle(d = boss_d);
  for (h = hooks) color("#56ccf2") at(h, 0.75) rotate(h[2]) translate([-snap_t_root / 2, 0]) linear_extrude(0.05) square([snap_t_root, h[3]], center = true);
  color("#f08a3c") at([usb_front - 3.65, usb_y], 0.8) linear_extrude(0.05) square([7.3, usb_w], center = true);
  color("#f08a3c") at(batt_conn + [0, (j2_front - j2_back) / 2], 0.8) linear_extrude(0.05) square([9.55, j2_front + j2_back], center = true);
  for (p = pin_pos) color("red") at(p, 0.9) linear_extrude(0.05) circle(d = 1.2);
}

// ---------------- 3D ----------------
// Snap fit: hooks hang from the lid just inside the tub wall and their barbs click into
// grooves in the wall. The lid prints face down, so a hook bends across its layers, a print's
// weakest direction: the first print's hooks (1.0 mm, barb 0.35) snapped in PLA. Now tapered
// from 1.3 at the root to 0.8 at the tip with a fillet, a 0.25 barb and 12 mm wide on the long
// walls: about half the root strain. The grip is the barb less the gap to the wall (0.1 mm
// drawn; FDM prints the gap smaller). PETG takes the bending far better than PLA.
// A lower lid (thin face_t) shortens them to fit over the board: a short hook is stiffer, so
// its barb shrinks with it to keep the root strain about the same
snap_t_root = 1.3;  snap_t_tip = 0.8;  snap_fillet = 0.6;  snap_len = min(4.3, bay_t - 0.4);
snap_barb = snap_len < 4.0 ? 0.2 : 0.25;  snap_barb_h = 1.2;  snap_gap = 0.15;
deck_fit = 0.3;           // gap between the lid's step walls and the tub's walls they drop between
in_x = case_w / 2 - wall;  in_y = case_l / 2 - wall;
// [x, y, direction of the wall]: clear of the USB-C (right wall, y 0), the debug connectors
// (left wall, y -2 and -12) and the button retainers (|y| beyond 45 on the long walls)
// [x, y, direction of the wall, width]: 12 mm on the long walls. The two on the ends went when
// Start/Select took the end of the spine (2026-10-19): the six screws hold the case shut
hooks = [[in_x, 30, 0, 12], [in_x, -30, 0, 12], [-in_x, 30, 180, 12], [-in_x, -30, 180, 12]];
// crush ribs on the tub's inner walls centre the board: [x, y, direction of the wall]
crush = [[in_x, 48, 0], [in_x, -48, 0], [-in_x, 48, 180], [-in_x, -48, 180],
         [22, in_y, 90], [-22, in_y, 90], [22, -in_y, 270], [-22, -in_y, 270]];
crush_reach = board_clear - 0.05;   // how far a rib stands off the wall: 0.05 mm short of the board
// stiffening ribs under the lid beside the screen: [x, y0, y1], 1.2 wide, rib_h deep
rib_h = 1.5;
lid_ribs = [[32.6, -40, -8], [32.6, 8, 40], [-32.6, -40, -17], [-32.6, 25, 40]];   // gaps over J1, and over
                                                                                  // J5, J6 and the pinhole buttons
assert(snap_len < bay_t - 0.3, "snap hooks reach the board");

// what the floor's ribs keep clear of under the board: [x0, y0, x1, y1] (and the speaker's circle)
rib_free = [
  [-cell[0] / 2 - 0.5, cell_y - cell[1] / 2 - 0.5, cell[0] / 2 + 0.5, cell_y + cell[1] / 2 + 0.5],   // the cell
  [slot[0][0] - 1, slot[1][1] - 1, slot[1][0] + 1, cell_y - cell[1] / 2 + 1],                      // its plug and wires
  [-6, spk_pos[1] + spk_d / 2 - 1, 6, spk_pos[1] + spk_d / 2 + 5],                                // speaker leads to J4
  [-10, seat_y - 14.5, 10, seat_y - 9.5],                                                         // TP1-TP5 on the back
  [usb_front - 8.5, usb_y - usb_tab_w / 2, board_w / 2 + usb_tab, usb_y + usb_tab_w / 2],         // J1's shell legs
];
module rib_free2d() {
  for (r = rib_free) translate([r[0], r[1]]) square([r[2] - r[0], r[3] - r[1]]);
  at(spk_pos) circle(d = spk_d + 1.0);
}

module tub() difference() {
  union() {
    difference() {
      linear_extrude(z_lid) rrect(case_w, case_l, corner_r);
      translate([0, 0, floor_t]) linear_extrude(z_lid) rrect(case_w - 2 * wall, case_l - 2 * wall, corner_r - wall);
      // walls drop at the decks, deck_fit clear of the lid's step walls that drop in between
      if (!deck_inserts) translate([0, 0, z_deck_lid]) linear_extrude(z_lid) offset(delta = deck_fit) deck_cut2d();
      else {
        // the walls drop to the deck's underside only where the lid's step walls come down,
        // and rise to the deck face round the insert
        translate([0, 0, z_deck_lid]) linear_extrude(z_lid) difference() { offset(delta = deck_fit) deck_cut2d(); deck2d(); }
        translate([0, 0, deck_face]) linear_extrude(z_lid) offset(delta = 0.01) deck2d();
      }
    }
    // the end walls' lips over the deck inserts
    if (deck_inserts) for (sy = [-1, 1]) end_lip_cut(sy, 0);
    // bosses up to the board (the screws come up through them)
    for (s = pins) at(s) cylinder(d = boss_d + 1.0, h = z_board);
    // crush ribs: a 45 degree lead-in at the top, so the board slides down onto them
    for (c = crush) at(c) rotate(c[2]) rotate([90, 0, 0]) linear_extrude(1.2, center = true)
      polygon([[0.01, floor_t], [-crush_reach, floor_t], [-crush_reach, z_top_cu], [0.01, z_top_cu + crush_reach + 0.01]]);
    // ribs under each silicone piece carry the button presses into the floor
    for (c = clusters) at(c) difference() {
      cylinder(d = piece_d - 4, h = z_board);
      translate([0, 0, -1]) cylinder(d = piece_d - 6, h = z_board + 2);
    }
    // the board rests on a grid of ribs board_lift high over the whole floor (the old
    // battery-under-board case was hollow there: the board felt flimsy), clear of the cell,
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
    // feet
    for (sx = [-1, 1], sy = [-1, 1]) at([sx * (case_w / 2 - 7), sy * (case_l / 2 - 9)], -0.8) cylinder(d = 6, h = 0.8 + 0.01);
  }
  // screws: a counterbore for the head from below, then clearance up through the boss
  for (s = pins) at(s, -1) { cylinder(d = screw_cb[0], h = 1 + screw_cb[1]); cylinder(d = screw_clear, h = z_board + 2); }
  // the seam's step: the wall's inner half drops lip_h where the lid sits on it
  translate([0, 0, z_lid - lip_h]) linear_extrude(lip_h + 1) seam_band(0);
  // grooves for the lid's snap hooks
  for (h = hooks) at(h) rotate(h[2]) translate([-0.01, -(h[3] + 0.6) / 2, z_lid - snap_len + 0.1])
    cube([snap_barb + 0.1 + 0.01, h[3] + 0.6, snap_barb_h + 0.2]);
  // speaker grille, under the speaker
  for (i = [-2:2], j = [-2:2]) if (norm([i, j]) <= 2.2) at(spk_pos + [i * 2.4, j * 2.4], -2) cylinder(d = 1.4, h = floor_t + 3);
  // USB-C: the opening's lower half, and above it the notch the lid's tongue fills
  usb_opening();
  translate([in_x - 0.01, usb_y - usb_tongue_w / 2, usb_zc]) cube([wall + 1, usb_tongue_w, z_lid]);
  // the slot the board's tab drops into
  if (usb_tab > 0) translate([in_x - 0.01, usb_y - usb_tab_w / 2 - usb_tab_fit, z_board - 0.3])
    cube([usb_tab - board_clear + usb_tab_fit + 0.01, usb_tab_w + 2 * usb_tab_fit, z_lid]);
}

usb_zc = z_top_cu + usb_h / 2;   // the port's centre
module usb_opening() translate([in_x - 1, usb_y, usb_zc]) rotate([0, 90, 0])
  linear_extrude(wall + 2) rrect(usb_hole[1], usb_hole[0], usb_hole[2]);

// every ABXY cap: [centre, tab direction in OpenSCAD's counter-clockwise degrees]. Seat 2's
// cluster is seat 1's turned 180 degrees, caps and all
function abxy_keys() = [for (s = [[abxy1, 0], [abxy2, 180]]) for (i = [0:3]) let(t = abxy_tabs[i])
  [s[0] + (s[1] == 0 ? t[1] * a : -t[1] * a), s[1] - t[2], [for (n = abxy_notches[i][1]) t[2] - n]]];
// one A/B/X/Y cap's lid cut, in its own frame (tab along +x), from the pocket's floor up: the hole,
// the flange's pocket less its notch ribs, the tab's slot
module abxy_cut(h, notches, hole_d = cap_hole_d) {
  cylinder(d = hole_d, h = h);
  difference() {
    cylinder(d = pocket_d, h = pocket_h + 0.01);
    if (notch_keys) for (n = notches) rotate(n) translate([cap_flange_d / 2 - notch_reach, -notch_w / 2, -1])
      cube([pocket_d / 2 - cap_flange_d / 2 + notch_reach + 0.05, notch_w, pocket_h + 2]);
  }
  linear_extrude(pocket_h + 0.01) polygon([[0, 0], for (t = [-slot_a / 2:slot_a / 6:slot_a / 2]) slot_r * [cos(t), sin(t)]]);
}
module dpad_cut(h, gap = dpad_gap) {
  translate([0, 0, dpad_lift]) linear_extrude(h) offset(r = gap) cross(dpad_arm_w, dpad_arm_l);
  linear_extrude(pocket_h + dpad_lift + 0.01) offset(r = dpad_pocket_gap) dpad_flange();
}
module dpad_flange() union() { circle(r = dpad_flange_r, $fn = 64); cross(dpad_arm_w, dpad_arm_l); }

// with inserts the lid keeps nothing under the deck faces: the tub's walls rise there instead
module lid() if (deck_inserts) difference() {
  lid_whole();
  insert_zone(insert_gap);
  translate([0, 0, z_top_cu - 1]) linear_extrude(deck_face + 1.01 - z_top_cu) offset(delta = 0.01) deck2d();
} else lid_whole();
module lid_whole() union() {
  lid_body();
  // the S-curve's lower part, added on the deck along the step in layers
  for (k = [0:ramp_layers - 1]) if (ramp_t(k) < 0)
    translate([0, 0, deck_face - 0.01 + (case_h - deck_face) * k / ramp_layers])
      linear_extrude((case_h - deck_face) / ramp_layers + 0.02) intersection() {
        rrect(case_w, case_l, corner_r);
        difference() { deck2d(); offset(delta = ramp_t(k)) deck2d(); }
      }
}
// the inserts: the deck's outline inside the tub's walls (sy: -1 seat 1's end, 1 seat 2's)
module insert2d(sy, grow = 0) intersection() {
  offset(delta = grow) deck2d();
  scale([1, sy]) translate([-case_w, 0]) square([2 * case_w, case_l]);
  offset(delta = -insert_gap) rrect(case_w - 2 * wall, case_l - 2 * wall, corner_r - wall);   // grow stops at the walls
}
// the space an insert takes (grow > 0: plus clearance, cut from the lid): its plate and all below,
// and its wedge under the step walls, in 0.1 mm slices (45 degrees: face down it needs no support)
module insert_zone(grow = 0, sy = 0) for (e = sy == 0 ? [-1, 1] : [sy]) {
  translate([0, 0, z_top_cu - 1]) linear_extrude(case_h + 2 - z_top_cu) insert2d(e, grow);
  w = insert_tongue + 1.5 * grow;
  for (i = [0:ceil(w / 0.1) - 1]) translate([0, 0, z_deck_lid + 0.1 * i - (i == 0 ? 1 : 0)])
    linear_extrude(0.11 + (i == 0 ? 1 : 0)) difference() {
      insert2d(e, max(0.01, w - 0.1 * (i + 0.5)) + grow);
      insert2d(e, -0.01);
    }
}
// one insert, in place: the lid's deck, retainers and button cuts within it, plus the tongue, less
// the 45 degree rebate under the tub end wall's lip
module deck_insert(sy) difference() {
  intersection() { lid_whole(); insert_zone(0, sy); }
  end_lip_cut(sy, insert_gap);
}
// the tub end wall's lip over the insert's end edge, a 45 degree wedge from the wall's inner face
module end_lip_cut(sy, grow = 0) scale([1, sy, 1])
  translate([-(case_w / 2 - corner_r), in_y + 0.01, deck_face]) rotate([0, 90, 0])
    linear_extrude(case_w - 2 * corner_r) polygon([[0.01, 0], [end_lip + grow + 0.01, 0], [0.01, -end_lip - grow]]);
// the raised pad over J2 and its chamfer: the S-curve leaves it whole
module j2_keep2d() at(batt_conn) square(pocket + [2 * (skin_min + j2_bump) + 1, 2 * (skin_min + j2_bump) + 1], center = true);

module lid_body() difference() {
  union() {
    translate([0, 0, z_lid]) linear_extrude(lid_t) difference() { rrect(case_w, case_l, corner_r); deck2d(); }
    // the decks, and the step walls from them up to the screen face
    translate([0, 0, z_deck_lid]) linear_extrude(lid_t) intersection() { rrect(case_w, case_l, corner_r); deck2d(); }
    translate([0, 0, z_deck_lid]) linear_extrude(case_h - z_deck_lid) intersection() {
      rrect(case_w, case_l, corner_r); difference() { deck_cut2d(); deck2d(); }
    }
    // the seam's lip, in the tub wall's step
    translate([0, 0, z_lid - lip_h + 0.1]) linear_extrude(lip_h - 0.1 + 0.01) seam_band(lip_fit);
    // bosses down to the board top: with the tub's bosses they clamp it
    for (s = pins) at(s, z_top_cu) cylinder(d = boss_d, h = z_lid - z_top_cu + 0.01);
    // stiffening ribs beside the screen
    for (r = lid_ribs) translate([r[0] - 0.6, r[1], z_lid - rib_h]) cube([1.2, r[2] - r[1], rib_h + 0.01]);
    // the pad over J2's pocket, when the screen face is too low to cover it (chamfered all round)
    if (j2_bump > 0) at(batt_conn, case_h - 0.01) hull() {
      linear_extrude(0.01) square(pocket + [2 * skin_min + 2 * j2_bump, 2 * skin_min + 2 * j2_bump], center = true);
      translate([0, 0, j2_bump]) linear_extrude(0.01) square(pocket + [2 * skin_min, 2 * skin_min], center = true);
    }
    // the tongue that fills the tub's notch over the USB-C opening
    translate([in_x, usb_y - usb_tongue_w / 2 + usb_fit, usb_zc + 0.1])
      cube([wall, usb_tongue_w - 2 * usb_fit, z_lid - usb_zc - 0.1 + 0.01]);
    // snap hooks, just inside the tub wall
    for (h = hooks) at(h) rotate(h[2]) snap_hook(h[3]);
    // the ribs that rest on the silicone pads (the pockets are cut below)
    for (c = [abxy1, abxy2]) at(c, retainer_bot) linear_extrude(z_lid - retainer_bot + 0.01) union() {
      for (t = [0:90:270]) rotate(t) translate([a, 0]) circle(r = abxy_ring, $fn = 64);
      square(abxy_square[0] + 2 * abxy_square[1], center = true);
    }
    for (c = [dpad1, dpad2]) at(c, retainer_bot) linear_extrude(z_lid - retainer_bot + 0.01)
      difference() { offset(r = dpad_ring[1]) dpad_clover(); dpad_clover(); }
    for (b = ss_buttons) at(b[2], retainer_bot) linear_extrude(z_lid - retainer_bot + 0.01)
      square(ss_flange_d + 0.4 + 2 * ss_frame_w, center = true);
    // pinhole guides reach down towards the switches
    for (p = pin_pos) at(p, z_top_cu + 2.4) cylinder(d = 3.0, h = z_lid - z_top_cu - 2.4 + 0.01);
    // panel locating frame
    translate([0, 0, z_lid - frame_h]) linear_extrude(frame_h + 0.01) difference() {
      square([panel_w + panel_fit + 2 * frame_wall, panel_l + panel_fit + 2 * frame_wall], center = true);
      square([panel_w + panel_fit, panel_l + panel_fit], center = true);
      translate([fpc_neck[0], -panel_l / 2]) square([fpc_neck[1] + 7, 6], center = true);   // FPC fold passes here: wide,
                                                                                             // the neck is only estimated
    }
  }
  usb_opening();                                                              // its upper half
  // the panel's pocket: its top face_t under the screen face. No FPC notch here: the FPC leaves
  // the panel's back and folds under it, below the lid (the frame below keeps its notch), and a
  // notch in the pocket put open slits under the step's S-curve
  if (face_t < lid_t) translate([0, 0, z_lid - 0.01]) linear_extrude(lid_t - face_t + 0.01)
    square([panel_w + panel_fit, panel_l + panel_fit], center = true);
  translate([0, 0, deck_face]) linear_extrude(case_h) deck2d();              // nothing above the decks
  // the step's top edge, chamfered (in 0.1 mm layers, as it prints)
  // the S-curve's upper part, cut out of the screen block in layers
  for (k = [0:ramp_layers - 1]) if (ramp_t(k) > 0)
    translate([0, 0, deck_face + (case_h - deck_face) * k / ramp_layers]) linear_extrude(case_h)
      difference() { offset(delta = ramp_t(k)) deck2d(); if (j2_bump > 0) j2_keep2d(); }
  // viewing window: the panel's own glass shows through, no lens
  translate([0, 0, z_lid - 1]) linear_extrude(lid_t + 2) window();
  // buttons
  for (k = abxy_keys()) at(k[0], retainer_bot - 0.01) rotate(k[1]) abxy_cut(case_h, k[2]);
  for (c = [dpad1, dpad2]) at(c, retainer_bot - 0.01) dpad_cut(case_h);
  for (b = ss_buttons) at(b[2], retainer_bot - 0.01) {
    cylinder(d = ss_cap_d + 2 * ss_gap, h = case_h);
    cylinder(d = ss_flange_d + 0.4, h = pocket_h + 0.01);
  }
  for (c = [abxy1, abxy2]) at(c, retainer_bot - 0.01)                        // the A/B/X/Y pad's centre square
    linear_extrude(z_top_cu + abxy_square[2] - retainer_bot + 0.01) square(abxy_square[0], center = true);
  // pilot holes for the screws
  for (s = pins) at(s, z_top_cu - 0.01) cylinder(d = screw_pilot, h = screw_engage);
  // pinholes, battery connector pocket
  for (p = pin_pos) at(p, 0) cylinder(d = 1.2, h = case_h + 1);
  if (j2_top > z_lid) at(batt_conn, z_lid - 1) linear_extrude(j2_top - z_lid + 1) square(pocket, center = true);
}

// one snap hook in the frame of its wall: the wall's inner face at x = 0, the case inside at -x
module snap_hook(w) {
  z0 = z_lid - snap_len;  zl = z_lid + 0.01;  g = snap_gap;
  // tapered from the root to the tip, with a 45 degree fillet into the lid on the inside
  rotate([90, 0, 0]) linear_extrude(w, center = true)
    polygon([[-g, z0], [-g, zl], [-g - snap_t_root - snap_fillet, zl], [-g - snap_t_root, zl - snap_fillet], [-g - snap_t_tip, z0]]);
  // barb: a 45 degree lead-in below, a steep face above that holds the lid down
  translate([0, w / 2, 0]) rotate([90, 0, 0]) linear_extrude(w)
    polygon([[-snap_gap, z0 + 0.2], [-snap_gap + snap_barb, z0 + 0.2 + snap_barb],
             [-snap_gap + snap_barb, z0 + snap_barb_h - 0.25], [-snap_gap, z0 + snap_barb_h]]);
}

module board3d() color("#2e7d4f") translate([0, 0, z_board]) linear_extrude(pcb_t) board_outline();

// button caps, for the assembly view only (they are bought DS Lite parts)
module caps() {
  z0 = retainer_z - flange_t;
  for (k = abxy_keys()) at(k[0], z0) rotate(k[1]) {
    cylinder(d = cap_flange_d, h = flange_t, $fn = 48);
    translate([0, -cap_tab_w / 2, 0]) cube([cap_tab_r, cap_tab_w, flange_t]);
    cylinder(d = cap_abxy_d, h = cap_h, $fn = 48);
  }
  for (c = [dpad1, dpad2]) at(c, z0 + dpad_lift) {
    linear_extrude(flange_t) dpad_flange();
    linear_extrude(dpad_h) cross(dpad_arm_w, dpad_arm_l);
  }
  for (b = ss_buttons) at(b[2], z0) { cylinder(d = ss_flange_d, h = flange_t, $fn = 48); cylinder(d = ss_cap_d, h = ss_cap_h, $fn = 48); }
}

if (part == "3d") {
  color("#3a3a44") tub();
  color("#c9c9d1") translate([0, cell_y, floor_t]) linear_extrude(cell[2]) square([cell[0], cell[1]], center = true);
  color("#b8912e") at(spk_pos, floor_t) cylinder(d = spk_d, h = spk_h);
  board3d();
  color("#101014") translate([0, 0, z_glass]) linear_extrude(panel_t) square([panel_w, panel_l], center = true);
  for (c = clusters) color("#8f8f99") at(c, z_top_cu) cylinder(d = piece_d, h = dome_h);
  for (c = ss_centres) color("#8f8f99") at(c, z_top_cu) linear_extrude(dome_h) rrect(ss_mem[0], ss_mem[1], 2);
  color("#4a4a56") translate([0, 0, explode]) lid();
  color("#e8e8ee") caps();
}
if (part == "tub") tub();
if (part == "lid") translate([0, 0, case_h]) rotate([180, 0, 0]) lid();       // printed face down
