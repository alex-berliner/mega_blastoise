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
conn_in = tip_in - fpc_slack - conn_rear;               // connector origin, in from the edge
// The panel's bay: the case and board layout are built around it, so a panel that
// fits the bay (with its tape and a 0.25 mm gap each side) moves nothing else.
bay_w = 56.0;     bay_l = 86.24;

cell = [34, 50, 5];                                     // 503450 Li-ion, w x l x t; ESTIMATE: length with its protection board
cell_y = -16;                                           // cell centre (towards seat 1: wires end there)
spk_d = 20;  spk_h = 4.5;  spk_y = 26;                  // ESTIMATE: 20 mm 8 ohm 1 W speaker

abxy_pitch = 15.0;  dpad_pitch = 14.0;                  // DS Lite (gerbers)
piece_d = 26;                                           // ESTIMATE: silicone piece outline
// Genuine DS Lite caps, measured from open models of them and of shells built for them
// (jackw01/pi-tin, CERN-OHL-W; Printables 1178557, 205921, 986494): see case/DSLITE.md
cap_abxy_d = 7.0;  cap_flange_d = 8.8;  cap_h = 3.56;  // ABXY: body, flange, flange bottom to top
cap_tab_r = 5.6;  cap_tab_w = 2.0;                      // each flange's keying tab, out from the cap's centre
dpad_arm_l = 17.8;  dpad_arm_w = 5.9;  dpad_h = 3.34;   // D-pad cross
dpad_flange_r = 6.5;                                    // its flange fills the corners between the arms
flange_t = 0.55;                                        // both caps' flanges
cap_hole_d = 7.4;  pocket_d = 9.2;  pocket_h = 1.4;     // lid: hole, flange pocket under it (Pi Tin's, for genuine caps)
slot_r = 5.95;  slot_a = 30;                            // tab slot in the pocket, a 30 degree wedge: the cap
                                                        // turns a few degrees at most, and the lid still goes on
                                                        // over caps set by eye (facelesstech's are 35)
// tab directions, degrees clockwise from the seat's right, seen from the front: a pinwheel,
// each 45 degrees clockwise from the flange's flat (which faces the diamond's centre)
abxy_tabs = [["A", [1, 0], 135], ["B", [0, -1], 225], ["X", [0, 1], 45], ["Y", [-1, 0], 315]];
dome_h = 2.0;                                           // ESTIMATE: board to the flange's underside at rest

// ---------------- layout ----------------
glass_side = 9.7;         // panel bay edge to long case edge: room for USB-C beside the glass
edge_margin = 4;          // case edge to silicone piece
piece_to_glass = 5;       // silicone piece to glass edge
wall = 2.0;  board_clear = 0.5;  corner_r = 8;

// ---------------- stack (z) ----------------
floor_t = 1.5;  cell_gap = 0.5;  pcb_t = 1.6;  lid_t = 2.0;     // lid 2.0, not 1.5: the first print felt flimsy
bay_t = 5.0;              // board top to lid underside: the panel and the air under it. 5.0, not 4.9:
                          // J3 (2.0 mm) under the panel keeps 0.4 mm against the connector's and a
                          // printed lid's tolerances (Alex, 2026-09-26)
glass_gap = bay_t - panel_t - tape_t;   // panel back above the board (the panel hangs from the lid on tape)
z_board = floor_t + cell[2] + cell_gap;                 // board bottom
z_top_cu = z_board + pcb_t;                             // board top: every height limit counts from here
z_glass = z_top_cu + glass_gap;
z_lid = z_glass + panel_t + tape_t;                     // lid underside
case_h = z_lid + lid_t;
retainer_z = z_top_cu + dome_h + flange_t;              // the lip the flanges rest against
// the lid comes down around the buttons to clamp the silicone sheet, as a DS Lite shell does:
// the flanges travel in pockets pocket_h deep, so the sheet's top is pocket_h under the lip
retainer_bot = retainer_z - pocket_h;
cap_top = retainer_z + cap_h - flange_t;                // at rest
// Stepped ends: the lid face over the screen is set by the panel stack, which leaves genuine
// caps below it. So each end drops to a button deck where the caps stand as on a DS Lite,
// except a spine between the clusters (J2 and the end snap hooks sit under it)
cap_proud = 1.3;                                        // above the deck, as a DS Lite (Pi Tin 1.3)
deck_face = cap_top - cap_proud;                        // lip over the flanges: 1.71, a DS Lite's 1.68
z_deck_lid = deck_face - lid_t;                         // deck underside; the tub's walls stop here
step_y = bay_l / 2 + 2.4;                               // the step: clear of the panel and its FPC fold
step_wall = 1.5;  spine_w = 13.0;  deck_r = 3;          // step wall, spine width, deck corner rounding

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

// Bosses that clamp the board: four beside the glass in the side bezels, two between each
// seat's clusters; a tub boss under the board meets a lid boss above it. (They were printed
// pins through the board's holes: 1.9 mm FDM pins bound and snapped off, so the first print
// dropped them. Crush ribs on the tub's walls now centre the board.) The board goes in one
// way only anyway: turned round, J1's receptacle hits the wall away from the USB opening,
// and the lid turned round lands on J2, which has its pocket at seat 1's end only.
pin_x = 31.4;             // midway between the bay and the board edge
pin_key = 3.0;            // seat 1's centre boss sits off the centre line (the board's hole is there)
pins = [[pin_x, 36], [-pin_x, 36], [pin_x, -36], [-pin_x, -36], [0, seat_y], [-pin_key, -seat_y]];
boss_d = 5.0;  hole_d = 2.2;  pin_d = 1.9;

// edge parts, all on the right (+X) long edge
usb_y = 0;   usb_w = 8.94;  usb_h = 3.25;               // TYPE-C-31-M-12 shell
// The opening takes the plug's overmold, not just its 8.25 x 2.4 shell: the receptacle's face is
// 1.15 mm inside the case's surface, so the overmold has to enter the wall. USB-C asks overmolds
// to fit 12.35 x 6.5 mm; the opening is a rounded oblong round that, centred on the port, split
// at its middle: the lower half in the tub wall, the upper half in a tongue of the lid that
// fills a notch in the wall (so neither part has a roof to print over the hole)
usb_hole = [13.2, 7.0, 2.6];   // width, height, corner radius: holds a 12.35 x 6.5 r1.75 overmold
usb_tongue_w = 18.0;  usb_fit = 0.2;   // the lid's tongue, and its clearance each side in the notch
usb_front = board_w / 2 + 1.35;                         // receptacle face: 1.35 past the board edge
// left long edge: debug
pin_pos = [[-(board_w / 2 - 4.0), 20.0], [-(board_w / 2 - 4.0), 12.0]];  // SW18 BOOTSEL, SW19 RESET
swd_pos = [-(board_w / 2 - 3.5), -2.0];  uart_pos = [-(board_w / 2 - 3.5), -12.0];

// the battery cable comes up through a slot next to its connector, between seat 1's clusters
batt_conn = [0, -(bay_l / 2 + 8.6)];                    // J2 body centre
slot = [[3.2, -(bay_l / 2 + 1.3)], [8.2, -(bay_l / 2 + 3.8)]];       // slot corners (x0,y0)-(x1,y1)
pocket = [9.4, 9.8];                                    // lid pocket over J2 (5.5 mm tall)

// ---------------- checks ----------------
assert(cluster_spacing >= piece_d + 2, "a seat's two silicone pieces would touch");
assert(seat_y - piece_d / 2 >= bay_l / 2 + piece_to_glass - 1e-6, "cluster runs into the glass");
assert(cluster_spacing / 2 + piece_d / 2 <= board_w / 2, "silicone piece hangs off the board");
assert(abs(cell_y) + cell[1] / 2 <= bay_l / 2, "cell sticks out from under the glass");
assert(spk_y + spk_d / 2 <= bay_l / 2 && spk_y - spk_d / 2 >= cell_y + cell[1] / 2 + 1, "speaker collides with the cell or leaves the glass area");
assert(pin_x - boss_d / 2 >= bay_w / 2 + 0.5 && pin_x + boss_d / 2 <= board_w / 2, "no room for pin bosses beside the glass");
assert(panel_w + panel_fit <= bay_w && panel_l + panel_fit <= bay_l, "panel doesn't fit its bay");
assert(glass_gap >= 1.0, "no room under the panel");
assert(retainer_z < z_lid, "button retainer above the lid");
assert(z_deck_lid > retainer_bot + 0.5, "button deck lower than the retainers");
assert(step_y - step_wall > (panel_l + panel_fit) / 2 + frame_wall + 0.3, "step over the panel frame");
for (s = pins) for (c = clusters) assert(norm(s - c) >= piece_d / 2 + boss_d / 2, "pin under a silicone piece");
assert(pin_key + boss_d / 2 + 0.5 <= spine_w / 2, "seat 1's centre boss runs out under a button deck");

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
  "\"slot\": [", jpt(slot[0]), ", ", jpt(slot[1]), "], ",
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
  "\"buttons\": {", join([for (b = buttons) str("\"", b[0], "\": ", jpt(b[2]))]), "}, ",
  "\"pins\": [", join([for (s = pins) jpt(s)]), "], \"boss_d\": ", boss_d, ", \"hole_d\": ", hole_d, ", ",
  "\"usb\": {\"y\": ", jnum(kicad([0, usb_y])[1]), ", \"front_x\": ", jnum(kicad([usb_front, 0])[0]), "}, ",
  "\"pinholes\": [", jpt(pin_pos[0]), ", ", jpt(pin_pos[1]), "], ",
  "\"swd\": ", jpt(swd_pos), ", \"uart\": ", jpt(uart_pos), ", ",
  "\"batt_conn\": ", jpt(batt_conn), ", \"pocket\": [", pocket[0], ", ", pocket[1], "], ",
  "\"cell\": [", jpt([-cell[0] / 2, cell_y + cell[1] / 2]), ", ", jpt([cell[0] / 2, cell_y - cell[1] / 2]), "], ",
  "\"speaker\": {\"c\": ", jpt([0, spk_y]), ", \"d\": ", spk_d, "}, ",
  "\"hooks\": [", join([for (h = hooks) str("[", jpt(hook_rect(h)[0]), ", ", jpt(hook_rect(h)[1]), "]")]), "], ",
  "\"frame\": {\"outer\": [", jpt([-(panel_w + panel_fit) / 2 - frame_wall, (panel_l + panel_fit) / 2 + frame_wall]), ", ",
    jpt([(panel_w + panel_fit) / 2 + frame_wall, -(panel_l + panel_fit) / 2 - frame_wall]), "], \"inner\": [",
    jpt([-(panel_w + panel_fit) / 2, (panel_l + panel_fit) / 2]), ", ", jpt([(panel_w + panel_fit) / 2, -(panel_l + panel_fit) / 2]),
    "], \"limit\": ", z_lid - z_top_cu - frame_h - 0.3, "}, ",
  "\"hook_clear\": ", bay_t - snap_len, ", ",
  "\"decks\": [", join([for (sy = [-1, 1], sx = [-1, 1]) str("[", jpt([sx * (spine_w / 2 - step_wall), sy * (step_y - step_wall)]), ", ",
    jpt([sx * case_w / 2, sy * case_l / 2]), "]")]), "], \"deck_limit\": ", z_deck_lid - z_top_cu - 0.3, ", ",
  "\"lid_ribs\": [", join([for (r = lid_ribs) str("[", jpt([r[0] - 0.6, r[1]]), ", ", jpt([r[0] + 0.6, r[2]]), "]")]), "], ",
  "\"rib_limit\": ", z_lid - rib_h - z_top_cu - 0.3,
  "}"));
for (b = buttons) echo(str(b[0], "  ", b[1], "   KiCad (", kicad(b[2])[0], ", ", kicad(b[2])[1], ")"));
echo(str("CAPS flange top ", retainer_z - z_top_cu, " above the board, cap top ", cap_top - z_top_cu, ", deck face ",
         deck_face - z_top_cu, ", screen face ", case_h - z_top_cu, ": ends ", deck_face, " mm thick, a ", case_h - deck_face, " mm step"));
echo(str("STACK floor ", floor_t, " + cell ", cell[2], " + gap ", cell_gap, " + PCB ", pcb_t, " + glass gap ", glass_gap,
         " + panel ", panel_t, " + tape ", tape_t, " + lid ", lid_t, " = ", case_h, " mm"));

// ---------------- 2D ----------------
module rrect(w, l, r) offset(r = r) square([w - 2 * r, l - 2 * r], center = true);
module at(p, z = 0) translate([p[0], p[1], z]) children();
module ring(t = 0.35) difference() { offset(r = t) children(); children(); }
module cross(w, l) union() { square([l, w], center = true); square([w, l], center = true); }
module window() translate([0, aa_off]) square([aa_w + 2 * window_margin, aa_l + 2 * window_margin], center = true);
// the two lowered decks at each end (past the case outline; intersect with it), and the
// band the tub's walls drop in: the decks plus the step walls
module deck2d() for (sy = [-1, 1]) scale([1, sy]) offset(r = deck_r) offset(delta = -deck_r)
  for (sx = [-1, 1]) scale([sx, 1]) translate([spine_w / 2, step_y]) square([case_w, case_l]);
module deck_cut2d() offset(delta = step_wall) deck2d();
module slot2d() translate(slot[0]) square([slot[1][0] - slot[0][0], slot[0][1] - slot[1][1]] * 1) ;
module board_outline() difference() {
  rrect(board_w, board_l, board_r);
  translate([slot[0][0], slot[1][1]]) square([slot[1][0] - slot[0][0], slot[0][1] - slot[1][1]]);
}
if (part == "board") board_outline();

if (part == "layout") {
  color("#34343d") linear_extrude(0.05) rrect(case_w, case_l, corner_r);
  color("#2e7d4f") translate([0, 0, 0.1]) linear_extrude(0.05) board_outline();
  color([0.85, 0.35, 0.35, 0.5]) translate([0, cell_y, 0.2]) linear_extrude(0.05) square([cell[0], cell[1]], center = true);
  color([0.9, 0.7, 0.2, 0.6]) at([0, spk_y], 0.2) linear_extrude(0.05) circle(d = spk_d);
  color("#101014") translate([0, 0, 0.3]) linear_extrude(0.05) square([panel_w, panel_l], center = true);
  color("#3b6fd1") translate([0, 0, 0.4]) linear_extrude(0.05) window();
  color("#c9a227") translate([0, 0, 0.45]) linear_extrude(0.05) {
    translate([fpc_body[0], -(panel_l / 2 - fpc_body[2] / 2)]) square([fpc_body[1], fpc_body[2]], center = true);
    translate([fpc_tail[0], -(panel_l / 2 - fpc_body[2] - fpc_tail[2] / 2)]) square([fpc_tail[1], fpc_tail[2]], center = true);
  }
  for (c = clusters) color([0.85, 0.85, 0.9, 0.45]) at(c, 0.5) linear_extrude(0.05) circle(d = piece_d);
  for (b = buttons) color("#f2c94c") at(b[2], 0.6) linear_extrude(0.05) circle(d = 4.8);
  for (s = pins) color("white") at(s, 0.7) linear_extrude(0.05) ring(0.4) circle(d = boss_d);
  for (h = hooks) color("#56ccf2") at(h, 0.75) rotate(h[2]) translate([-snap_t_root / 2, 0]) linear_extrude(0.05) square([snap_t_root, h[3]], center = true);
  color("#f08a3c") at([usb_front - 3.65, usb_y], 0.8) linear_extrude(0.05) square([7.3, usb_w], center = true);
  color("#f08a3c") at(batt_conn, 0.8) linear_extrude(0.05) square([7.9, 7.6], center = true);
  for (p = pin_pos) color("red") at(p, 0.9) linear_extrude(0.05) circle(d = 1.2);
}

// ---------------- 3D ----------------
// Snap fit: hooks hang from the lid just inside the tub wall and their barbs click into
// grooves in the wall. The lid prints face down, so a hook bends across its layers, a print's
// weakest direction: the first print's hooks (1.0 mm, barb 0.35) snapped in PLA. Now tapered
// from 1.3 at the root to 0.8 at the tip with a fillet, a 0.25 barb and 12 mm wide on the long
// walls: about half the root strain. The grip is the barb less the gap to the wall (0.1 mm
// drawn; FDM prints the gap smaller). PETG takes the bending far better than PLA.
snap_t_root = 1.3;  snap_t_tip = 0.8;  snap_fillet = 0.6;  snap_len = 4.3;
snap_barb = 0.25;  snap_barb_h = 1.2;  snap_gap = 0.15;
deck_fit = 0.3;           // gap between the lid's step walls and the tub's walls they drop between
in_x = case_w / 2 - wall;  in_y = case_l / 2 - wall;
// [x, y, direction of the wall]: clear of the USB-C (right wall, y 0), the debug connectors
// (left wall, y -2 and -12) and the button retainers (|y| beyond 45 on the long walls)
// [x, y, direction of the wall, width]: 12 mm on the long walls, 8 on the ends (the spine is 13)
hooks = [[in_x, 30, 0, 12], [in_x, -30, 0, 12], [-in_x, 30, 180, 12], [-in_x, -30, 180, 12],
         [0, in_y, 90, 8], [0, -in_y, 270, 8]];
// crush ribs on the tub's inner walls centre the board: [x, y, direction of the wall]
crush = [[in_x, 48, 0], [in_x, -48, 0], [-in_x, 48, 180], [-in_x, -48, 180],
         [22, in_y, 90], [-22, in_y, 90], [22, -in_y, 270], [-22, -in_y, 270]];
crush_reach = board_clear - 0.05;   // how far a rib stands off the wall: 0.05 mm short of the board
// stiffening ribs under the lid beside the screen: [x, y0, y1], 1.2 wide, rib_h deep
rib_h = 1.5;
lid_ribs = [[32.6, -40, -8], [32.6, 8, 40], [-32.6, -40, 40]];      // a gap over J1 on the right
assert(snap_len < bay_t - 0.3, "snap hooks reach the board");

module tub() difference() {
  union() {
    difference() {
      linear_extrude(z_lid) rrect(case_w, case_l, corner_r);
      translate([0, 0, floor_t]) linear_extrude(z_lid) rrect(case_w - 2 * wall, case_l - 2 * wall, corner_r - wall);
      // walls drop at the decks, deck_fit clear of the lid's step walls that drop in between
      translate([0, 0, z_deck_lid]) linear_extrude(z_lid) offset(delta = deck_fit) deck_cut2d();
    }
    // bosses up to the board
    for (s = pins) at(s) cylinder(d = boss_d + 1.0, h = z_board);
    // crush ribs: a 45 degree lead-in at the top, so the board slides down onto them
    for (c = crush) at(c) rotate(c[2]) rotate([90, 0, 0]) linear_extrude(1.2, center = true)
      polygon([[0.01, floor_t], [-crush_reach, floor_t], [-crush_reach, z_top_cu], [0.01, z_top_cu + crush_reach + 0.01]]);
    // ribs under each silicone piece carry the button presses into the floor
    for (c = clusters) at(c) difference() {
      cylinder(d = piece_d - 4, h = z_board);
      translate([0, 0, -1]) cylinder(d = piece_d - 6, h = z_board + 2);
    }
    // cell pocket walls and speaker ring
    translate([0, cell_y, 0]) linear_extrude(floor_t + 3) difference() {
      square([cell[0] + 3.2, cell[1] + 3.2], center = true); square([cell[0] + 0.8, cell[1] + 0.8], center = true);
    }
    at([0, spk_y]) linear_extrude(floor_t + 1.5) difference() { circle(d = spk_d + 2.4); circle(d = spk_d + 0.4); }
    // feet
    for (sx = [-1, 1], sy = [-1, 1]) at([sx * (case_w / 2 - 7), sy * (case_l / 2 - 9)], -0.8) cylinder(d = 6, h = 0.8 + 0.01);
  }
  // grooves for the lid's snap hooks
  for (h = hooks) at(h) rotate(h[2]) translate([-0.01, -(h[3] + 0.6) / 2, z_lid - snap_len + 0.1])
    cube([snap_barb + 0.1 + 0.01, h[3] + 0.6, snap_barb_h + 0.2]);
  // speaker grille
  for (i = [-3:3], j = [-3:3]) if (norm([i, j]) <= 3.2) at([i * 2.4, spk_y + j * 2.4], -2) cylinder(d = 1.4, h = floor_t + 3);
  // USB-C: the opening's lower half, and above it the notch the lid's tongue fills
  usb_opening();
  translate([in_x - 0.01, usb_y - usb_tongue_w / 2, usb_zc]) cube([wall + 1, usb_tongue_w, z_lid]);
}

usb_zc = z_top_cu + usb_h / 2;   // the port's centre
module usb_opening() translate([in_x - 1, usb_y, usb_zc]) rotate([0, 90, 0])
  linear_extrude(wall + 2) rrect(usb_hole[1], usb_hole[0], usb_hole[2]);

// every ABXY cap: [centre, tab direction in OpenSCAD's counter-clockwise degrees]. Seat 2's
// cluster is seat 1's turned 180 degrees, caps and all
function abxy_keys() = [for (s = [[abxy1, 0], [abxy2, 180]]) for (t = abxy_tabs)
  [s[0] + (s[1] == 0 ? t[1] * a : -t[1] * a), s[1] - t[2]]];
module dpad_flange() union() { circle(r = dpad_flange_r, $fn = 64); cross(dpad_arm_w, dpad_arm_l); }

module lid() difference() {
  union() {
    translate([0, 0, z_lid]) linear_extrude(lid_t) difference() { rrect(case_w, case_l, corner_r); deck2d(); }
    // the decks, and the step walls from them up to the screen face
    translate([0, 0, z_deck_lid]) linear_extrude(lid_t) intersection() { rrect(case_w, case_l, corner_r); deck2d(); }
    translate([0, 0, z_deck_lid]) linear_extrude(case_h - z_deck_lid) intersection() {
      rrect(case_w, case_l, corner_r); difference() { deck_cut2d(); deck2d(); }
    }
    // bosses down to the board top: with the tub's bosses they clamp it
    for (s = pins) at(s, z_top_cu) cylinder(d = boss_d, h = z_lid - z_top_cu + 0.01);
    // stiffening ribs beside the screen
    for (r = lid_ribs) translate([r[0] - 0.6, r[1], z_lid - rib_h]) cube([1.2, r[2] - r[1], rib_h + 0.01]);
    // the tongue that fills the tub's notch over the USB-C opening
    translate([in_x, usb_y - usb_tongue_w / 2 + usb_fit, usb_zc + 0.1])
      cube([wall, usb_tongue_w - 2 * usb_fit, z_lid - usb_zc - 0.1 + 0.01]);
    // snap hooks, just inside the tub wall
    for (h = hooks) at(h) rotate(h[2]) snap_hook(h[3]);
    // button retainers: the lid comes down to the cap flanges around each cluster
    for (c = clusters) at(c, retainer_bot) cylinder(d = piece_d + 1, h = z_lid - retainer_bot + 0.01);
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
  translate([0, 0, deck_face]) linear_extrude(case_h) deck2d();              // nothing above the decks
  // viewing window: the panel's own glass shows through, no lens
  translate([0, 0, z_lid - 1]) linear_extrude(lid_t + 2) window();
  // buttons
  for (k = abxy_keys()) at(k[0], retainer_bot - 0.01) rotate(k[1]) {
    cylinder(d = cap_hole_d, h = case_h);
    cylinder(d = pocket_d, h = pocket_h + 0.01);
    linear_extrude(pocket_h + 0.01) polygon([[0, 0], for (t = [-slot_a / 2:slot_a / 6:slot_a / 2]) slot_r * [cos(t), sin(t)]]);
  }
  for (c = [dpad1, dpad2]) at(c, retainer_bot - 0.01) {
    linear_extrude(case_h) offset(r = 0.2) cross(dpad_arm_w, dpad_arm_l);
    linear_extrude(pocket_h + 0.01) offset(r = 0.35) dpad_flange();
  }
  // pinholes, battery connector pocket
  for (p = pin_pos) at(p, 0) cylinder(d = 1.2, h = case_h + 1);
  at(batt_conn, z_lid - 1) linear_extrude(1 + 0.9) square(pocket, center = true);
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
  for (c = [dpad1, dpad2]) at(c, z0) {
    linear_extrude(flange_t) dpad_flange();
    linear_extrude(dpad_h) cross(dpad_arm_w, dpad_arm_l);
  }
}

if (part == "3d") {
  color("#3a3a44") tub();
  color("#c9c9d1") translate([0, cell_y, floor_t]) linear_extrude(cell[2]) square([cell[0], cell[1]], center = true);
  color("#b8912e") at([0, spk_y], floor_t) cylinder(d = spk_d, h = spk_h);
  board3d();
  color("#101014") translate([0, 0, z_glass]) linear_extrude(panel_t) square([panel_w, panel_l], center = true);
  for (c = clusters) color("#8f8f99") at(c, z_top_cu) cylinder(d = piece_d, h = dome_h);
  color("#4a4a56") translate([0, 0, explode]) lid();
  color("#e8e8ee") caps();
}
if (part == "tub") tub();
if (part == "lid") translate([0, 0, case_h]) rotate([180, 0, 0]) lid();       // printed face down
