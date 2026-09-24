// Reference design case: the single source of truth for every position the
// board depends on (outline, contacts, screws, connectors, height limits).
//
//   part="layout"   coloured top view
//   part="board"    board outline with its cable slot, 2D (DXF for Edge.Cuts)
//   part="3d"       assembly; explode=<mm> lifts the lid
//   part="tub", "lid", "pwr_cap"   printable parts (STL)
//   part="lens"     cover lens outline, 2D (DXF for laser cutting 1 mm acrylic)
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
panel_w = 54.40;  panel_l = 86.24;  panel_t = 3.50;     // DT035CTFT-IPS-SHB outline
aa_w = 48.96;     aa_l = 73.44;                         // active area
aa_to_top = 4.50; aa_to_fpc = 8.30;                     // active area to the glass edges
aa_off = (aa_to_fpc - aa_to_top) / 2;                   // active area sits 1.9 mm towards seat 2
window_margin = 0.5;                                    // bezel opening past the active area
fpc_x = -5.36;    fpc_w = 13.3;                         // FPC tail centre, connector width
fpc_front = 31.0;                                       // connector mouth to the glass FPC edge

cell = [34, 50, 10];                                    // 103450 Li-ion, w x l x t
cell_y = -16;                                           // cell centre (towards seat 1: wires end there)
spk_d = 20;  spk_h = 4.5;  spk_y = 26;                  // ESTIMATE: 20 mm 8 ohm 1 W speaker

abxy_pitch = 15.0;  dpad_pitch = 14.0;                  // DS Lite (gerbers)
piece_d = 26;                                           // ESTIMATE: silicone piece outline
cap_abxy_d = 6.0;  dpad_arm_l = 18;  dpad_arm_w = 6;    // ESTIMATE: DS Lite caps
dome_h = 2.0;  flange_t = 0.8;  cap_proud = 1.2;        // ESTIMATE: silicone height, cap flange, cap above lid

// ---------------- layout ----------------
glass_side = 10.5;        // glass edge to long case edge: room for USB-C beside the glass
edge_margin = 4;          // case edge to silicone piece
piece_to_glass = 5;       // silicone piece to glass edge
wall = 2.0;  board_clear = 0.5;  corner_r = 8;

// ---------------- stack (z) ----------------
floor_t = 1.5;  cell_gap = 0.5;  pcb_t = 1.6;  glass_gap = 1.4;  lid_t = 1.5;
lens_t = 1.0;             // acrylic cover lens, flush in the lid top
z_board = floor_t + cell[2] + cell_gap;                 // board bottom
z_top_cu = z_board + pcb_t;                             // board top: every height limit counts from here
z_glass = z_top_cu + glass_gap;
z_lid = z_glass + panel_t;                              // lid underside
case_h = z_lid + lid_t;
retainer_z = z_top_cu + dome_h + flange_t;              // lid underside around the buttons

// ---------------- derived ----------------
case_w = panel_w + 2 * glass_side;
cluster_spacing = case_w - piece_d - 2 * edge_margin;
band = edge_margin + piece_d + piece_to_glass;
case_l = panel_l + 2 * band;
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

// screws: four beside the glass in the side bezels, two between each seat's clusters
screw_x = panel_w / 2 + (board_w / 2 - panel_w / 2) / 2 + 0.2;
screws = [[screw_x, 36], [-screw_x, 36], [screw_x, -36], [-screw_x, -36], [0, seat_y], [0, -seat_y]];
boss_d = 5.0;  hole_d = 2.2;

// edge parts, all on the right (+X) long edge
usb_y = 0;   usb_w = 8.94;  usb_h = 3.25;               // TYPE-C-31-M-12 shell
usb_front = board_w / 2 + 1.35;                         // receptacle face: 1.35 past the board edge
pwr_y = 22;  pwr_plunger = board_w / 2;                 // SKRTLAE010 plunger tip at the board edge
led_pos = [[board_w / 2 - 4.0, 12.0], [board_w / 2 - 4.0, 8.0]];   // D1 charge, D4 error
// left long edge: debug
pin_pos = [[-(board_w / 2 - 4.0), 20.0], [-(board_w / 2 - 4.0), 12.0]];  // SW18 BOOTSEL, SW19 RESET
swd_pos = [-(board_w / 2 - 3.5), -2.0];  uart_pos = [-(board_w / 2 - 3.5), -12.0];

// the battery cable comes up through a slot next to its connector, between seat 1's clusters
batt_conn = [0, -(panel_l / 2 + 8.6)];                  // J2 body centre
slot = [[3.2, -(panel_l / 2 + 1.3)], [8.2, -(panel_l / 2 + 3.8)]];   // slot corners (x0,y0)-(x1,y1)
pocket = [9.4, 9.8];                                    // lid pocket over J2 (5.5 mm tall)

// ---------------- checks ----------------
assert(cluster_spacing >= piece_d + 2, "a seat's two silicone pieces would touch");
assert(seat_y - piece_d / 2 >= panel_l / 2 + piece_to_glass - 1e-6, "cluster runs into the glass");
assert(cluster_spacing / 2 + piece_d / 2 <= board_w / 2, "silicone piece hangs off the board");
assert(abs(cell_y) + cell[1] / 2 <= panel_l / 2, "cell sticks out from under the glass");
assert(spk_y + spk_d / 2 <= panel_l / 2 && spk_y - spk_d / 2 >= cell_y + cell[1] / 2 + 1, "speaker collides with the cell or leaves the glass area");
assert(board_w / 2 - panel_w / 2 >= boss_d + 1, "no room for screw bosses beside the glass");
assert(retainer_z < z_lid, "button retainer above the lid");
for (s = screws) for (c = clusters) assert(norm(s - c) >= piece_d / 2 + boss_d / 2, "screw under a silicone piece");

// ---------------- echo: everything the board script needs ----------------
function jnum(v) = str(round(v * 1000) / 1000);
function jpt(p) = str("[", jnum(kicad(p)[0]), ", ", jnum(kicad(p)[1]), "]");
function join(l, i = 0) = i >= len(l) ? "" : str(i ? ", " : "", l[i], join(l, i + 1));
echo(str("CASE ", case_w, " x ", case_l, " x ", case_h, " mm   board ", board_w, " x ", board_l, " r ", board_r));
echo(str("JSON {",
  "\"board\": {\"w\": ", jnum(board_w), ", \"l\": ", jnum(board_l), ", \"r\": ", jnum(board_r), ", \"t\": ", pcb_t, "}, ",
  "\"slot\": [", jpt(slot[0]), ", ", jpt(slot[1]), "], ",
  "\"glass\": [", jpt([-panel_w / 2, panel_l / 2]), ", ", jpt([panel_w / 2, -panel_l / 2]), "], ",
  "\"glass_limit\": ", glass_gap - 0.2, ", ",
  "\"bezel_limit\": ", z_lid - z_top_cu - 0.3, ", ",
  "\"retainer_limit\": ", retainer_z - z_top_cu - 0.3, ", ",
  "\"fpc_tail\": [", jpt([fpc_x - fpc_w / 2 - 0.5, -(panel_l / 2 - fpc_front)]), ", ", jpt([fpc_x + fpc_w / 2 + 0.5, -(panel_l / 2 + 1.5)]), "], ",
  "\"fpc_conn\": {\"x\": ", jnum(kicad([fpc_x, 0])[0]), ", \"front_y\": ", jnum(kicad([0, -(panel_l / 2 - fpc_front)])[1]), "}, ",
  "\"pieces\": [", join([for (c = clusters) jpt(c)]), "], \"piece_d\": ", piece_d, ", ",
  "\"buttons\": {", join([for (b = buttons) str("\"", b[0], "\": ", jpt(b[2]))]), "}, ",
  "\"screws\": [", join([for (s = screws) jpt(s)]), "], \"boss_d\": ", boss_d, ", \"hole_d\": ", hole_d, ", ",
  "\"usb\": {\"y\": ", jnum(kicad([0, usb_y])[1]), ", \"front_x\": ", jnum(kicad([usb_front, 0])[0]), "}, ",
  "\"pwr\": {\"y\": ", jnum(kicad([0, pwr_y])[1]), ", \"plunger_x\": ", jnum(kicad([pwr_plunger, 0])[0]), "}, ",
  "\"leds\": [", jpt(led_pos[0]), ", ", jpt(led_pos[1]), "], ",
  "\"pinholes\": [", jpt(pin_pos[0]), ", ", jpt(pin_pos[1]), "], ",
  "\"swd\": ", jpt(swd_pos), ", \"uart\": ", jpt(uart_pos), ", ",
  "\"batt_conn\": ", jpt(batt_conn), ", \"pocket\": [", pocket[0], ", ", pocket[1], "], ",
  "\"cell\": [", jpt([-cell[0] / 2, cell_y + cell[1] / 2]), ", ", jpt([cell[0] / 2, cell_y - cell[1] / 2]), "], ",
  "\"speaker\": {\"c\": ", jpt([0, spk_y]), ", \"d\": ", spk_d, "}",
  "}"));
for (b = buttons) echo(str(b[0], "  ", b[1], "   KiCad (", kicad(b[2])[0], ", ", kicad(b[2])[1], ")"));
echo(str("STACK floor ", floor_t, " + cell ", cell[2], " + gap ", cell_gap, " + PCB ", pcb_t, " + glass gap ", glass_gap,
         " + panel ", panel_t, " + lid ", lid_t, " = ", case_h, " mm"));

// ---------------- 2D ----------------
module rrect(w, l, r) offset(r = r) square([w - 2 * r, l - 2 * r], center = true);
module at(p, z = 0) translate([p[0], p[1], z]) children();
module ring(t = 0.35) difference() { offset(r = t) children(); children(); }
module cross(w, l) union() { square([l, w], center = true); square([w, l], center = true); }
module window() translate([0, aa_off]) square([aa_w + 2 * window_margin, aa_l + 2 * window_margin], center = true);
module slot2d() translate(slot[0]) square([slot[1][0] - slot[0][0], slot[0][1] - slot[1][1]] * 1) ;
module board_outline() difference() {
  rrect(board_w, board_l, board_r);
  translate([slot[0][0], slot[1][1]]) square([slot[1][0] - slot[0][0], slot[0][1] - slot[1][1]]);
}
module lens2d() offset(r = 1) square([panel_w + 1.0, panel_l + 1.0], center = true);

if (part == "board") board_outline();
if (part == "lens") lens2d();

if (part == "layout") {
  color("#34343d") linear_extrude(0.05) rrect(case_w, case_l, corner_r);
  color("#2e7d4f") translate([0, 0, 0.1]) linear_extrude(0.05) board_outline();
  color([0.85, 0.35, 0.35, 0.5]) translate([0, cell_y, 0.2]) linear_extrude(0.05) square([cell[0], cell[1]], center = true);
  color([0.9, 0.7, 0.2, 0.6]) at([0, spk_y], 0.2) linear_extrude(0.05) circle(d = spk_d);
  color("#101014") translate([0, 0, 0.3]) linear_extrude(0.05) square([panel_w, panel_l], center = true);
  color("#3b6fd1") translate([0, 0, 0.4]) linear_extrude(0.05) window();
  color("#c9a227") translate([fpc_x, -(panel_l / 2 - fpc_front / 2), 0.45]) linear_extrude(0.05) square([fpc_w, fpc_front], center = true);
  for (c = clusters) color([0.85, 0.85, 0.9, 0.45]) at(c, 0.5) linear_extrude(0.05) circle(d = piece_d);
  for (b = buttons) color("#f2c94c") at(b[2], 0.6) linear_extrude(0.05) circle(d = 4.8);
  for (s = screws) color("white") at(s, 0.7) linear_extrude(0.05) ring(0.4) circle(d = boss_d);
  color("#f08a3c") at([usb_front - 3.65, usb_y], 0.8) linear_extrude(0.05) square([7.3, usb_w], center = true);
  color("#f08a3c") at([pwr_plunger - 1.7, pwr_y], 0.8) linear_extrude(0.05) square([3.4, 4.5], center = true);
  color("#f08a3c") at(batt_conn, 0.8) linear_extrude(0.05) square([7.9, 7.6], center = true);
  for (p = pin_pos) color("red") at(p, 0.9) linear_extrude(0.05) circle(d = 1.2);
  for (p = led_pos) color("lime") at(p, 0.9) linear_extrude(0.05) circle(d = 2.0);
}

// ---------------- 3D ----------------
screw_cb_d = 4.2;  screw_cb_h = 1.2;  pilot_d = 1.7;   // M2 x 16 from below, self-tapping into the lid bosses

module tub() difference() {
  union() {
    difference() {
      linear_extrude(z_lid) rrect(case_w, case_l, corner_r);
      translate([0, 0, floor_t]) linear_extrude(z_lid) rrect(case_w - 2 * wall, case_l - 2 * wall, corner_r - wall);
    }
    // screw bosses up to the board
    for (s = screws) at(s) cylinder(d = boss_d + 1.0, h = z_board);
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
  // screw holes with a counterbore from below
  for (s = screws) at(s, -1) { cylinder(d = hole_d + 0.2, h = z_board + 2); cylinder(d = screw_cb_d, h = screw_cb_h + 1 - 0.8 + 0.8); }
  // speaker grille
  for (i = [-3:3], j = [-3:3]) if (norm([i, j]) <= 3.2) at([i * 2.4, spk_y + j * 2.4], -2) cylinder(d = 1.4, h = floor_t + 3);
  // USB-C: plug overmold recess outside, receptacle opening through
  at([case_w / 2 - 1.3, usb_y], z_top_cu + usb_h / 2) cube([3, 12.6, 6.6], center = true);
  at([case_w / 2 - wall / 2, usb_y], z_top_cu + usb_h / 2) cube([wall + 2, 9.4, 3.8], center = true);
  // power button: cap slides in this slot
  at([case_w / 2 - wall / 2, pwr_y], z_top_cu + 1.7) cube([wall + 2, 5.0, 3.0], center = true);
}

module lid() difference() {
  union() {
    translate([0, 0, z_lid]) linear_extrude(lid_t) rrect(case_w, case_l, corner_r);
    // screw bosses down to the board top
    for (s = screws) at(s, z_top_cu) cylinder(d = boss_d, h = z_lid - z_top_cu + 0.01);
    // button retainers: the lid comes down to the cap flanges around each cluster
    for (c = clusters) at(c, retainer_z) cylinder(d = piece_d + 1, h = z_lid - retainer_z + 0.01);
    // pinhole guides reach down towards the switches
    for (p = pin_pos) at(p, z_top_cu + 2.4) cylinder(d = 3.0, h = z_lid - z_top_cu - 2.4 + 0.01);
    // panel locating frame
    translate([0, 0, z_lid - 1.0]) linear_extrude(1.01) difference() {
      square([panel_w + 2.6, panel_l + 2.6], center = true); square([panel_w + 0.4, panel_l + 0.4], center = true);
      translate([fpc_x, -panel_l / 2]) square([fpc_w + 3, 6], center = true);   // FPC fold passes here
    }
  }
  for (s = screws) at(s, z_top_cu - 1) cylinder(d = pilot_d, h = z_lid - z_top_cu - 0.5);
  // viewing window and lens pocket
  translate([0, 0, z_lid - 1]) linear_extrude(lid_t + 2) window();
  translate([0, 0, case_h - lens_t]) linear_extrude(lens_t + 1) lens2d();
  // buttons
  for (c = [abxy1, abxy2]) for (o = [[a, 0], [-a, 0], [0, a], [0, -a]]) at(c + o, z_top_cu) cylinder(d = cap_abxy_d + 0.4, h = case_h);
  for (c = [dpad1, dpad2]) at(c, z_top_cu) linear_extrude(case_h) offset(r = 0.2) cross(dpad_arm_w, dpad_arm_l);
  // pinholes, light pipes (2 mm acrylic rod), battery connector pocket
  for (p = pin_pos) at(p, 0) cylinder(d = 1.2, h = case_h + 1);
  for (p = led_pos) at(p, z_lid - 1) cylinder(d = 2.1, h = lid_t + 2);
  at(batt_conn, z_lid - 1) linear_extrude(1 + 0.9) square(pocket, center = true);
}

module pwr_cap() {
  // slides in the tub slot; the inner flange keeps it in, the nose presses the plunger
  cube([2.8, 4.6, 2.6], center = true);
  translate([-1.4 - 0.5, 0, 0]) cube([1.0, 7.0, 2.6], center = true);
}

module board3d() color("#2e7d4f") translate([0, 0, z_board]) linear_extrude(pcb_t) board_outline();

// button caps, for the assembly view only (they are bought DS Lite parts)
module caps() {
  z0 = z_top_cu + dome_h;
  for (c = [abxy1, abxy2]) for (o = [[a, 0], [-a, 0], [0, a], [0, -a]]) at(c + o, z0) {
    cylinder(d = cap_abxy_d + 1.2, h = flange_t);
    cylinder(d = cap_abxy_d, h = case_h + cap_proud - z0);
  }
  for (c = [dpad1, dpad2]) at(c, z0) {
    linear_extrude(flange_t) offset(r = 0.6) cross(dpad_arm_w, dpad_arm_l);
    linear_extrude(case_h + cap_proud - z0) cross(dpad_arm_w, dpad_arm_l);
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
  color([0.8, 0.9, 1.0, 0.4]) translate([0, 0, case_h - lens_t - 0.02 + explode]) linear_extrude(lens_t) lens2d();
  color("#e8e8ee") caps();
  color("#e8e8ee") at([case_w / 2 - wall / 2 + 0.4, pwr_y], z_top_cu + 1.7) pwr_cap();
}
if (part == "tub") tub();
if (part == "lid") translate([0, 0, case_h]) rotate([180, 0, 0]) lid();       // printed face down
if (part == "pwr_cap") pwr_cap();
