// Production case: preliminary top-view layout.
//
// The case owns every position the PCB depends on. This file is the single
// source of truth for the board outline, the glass window and the centre of
// every membrane contact; KiCad copies them, never the other way round.
//
//   part="layout"  coloured top view with labels (for looking at)
//   part="board"   board outline only, 2D, for DXF export to KiCad Edge.Cuts
//   part="pads"    contact centres + board outline, 2D, for a KiCad User layer
//   part="template" 1:1 outlines to print on paper and put your thumbs on
//   part="3d"      the case in 3D; explode=<mm> lifts the lid to show the inside
//   part="tub", part="lid"   the printable parts on their own, for STL export
//
// Frame: origin at the case centre, +Y towards seat 2, seat 1 sits at -Y.
// KiCad frame used in the echoes: centre at (100,100), Y pointing down.
//
// Every value marked ESTIMATE needs a real part and calipers before the
// board is committed. The echo block lists them.

part = "layout";
explode = 0;
$fn = 96;

// ---------------- parts ----------------
panel_w = 54.5;   panel_l = 83.0;     // 3.5" 320x480 IPS glass outline (shortlist shares it)
aa_w = 48.96;     aa_l = 73.44;       // active area
aa_offset_y = 0;                      // ESTIMATE: active area centred in the glass
window_margin = 0.5;                  // bezel window past the active area, per side

abxy_pitch = 15.0;                    // opposite ABXY contacts, DS Lite (measured, gerbers)
dpad_pitch = 14.0;                    // opposite D-pad contacts, DS Lite breakout (measured)
pad_mask_d = 4.8;                     // DSL_button_4ring mask opening
piece_d    = 26;                      // ESTIMATE: silicone piece outline, per cluster
cap_abxy_d = 6.0;                     // ESTIMATE: DS Lite face button cap
dpad_arm_l = 18;  dpad_arm_w = 6;     // ESTIMATE: D-pad cross cap

cell = [34, 50, 10];                  // 103450 LiPo, w x l x t (current prototype cell)
usb_w = 9.0;  usb_d = 7.5;            // ESTIMATE: USB-C receptacle footprint
pwr_w = 6.0;  pwr_d = 3.5;            // ESTIMATE: side power button

// ---------------- ergonomics + structure ----------------
// The side bezel sets the case width, and the width sets how far apart each
// seat's D-pad and ABXY can sit: they share it. Narrower bezel, closer thumbs.
glass_side      = 8;      // glass edge to long case edge (the side bezel)
edge_margin     = 4;      // case edge to silicone piece
piece_to_glass  = 5;      // silicone piece to glass edge
wall = 2.0;  board_clear = 0.5;  corner_r = 8;

// ---------------- derived ----------------
case_w  = panel_w + 2*glass_side;
cluster_spacing = case_w - piece_d - 2*edge_margin;   // D-pad centre to ABXY centre
band    = edge_margin + piece_d + piece_to_glass;
case_l  = panel_l + 2*band;
inset   = wall + board_clear;
board_w = case_w - 2*inset;
board_l = case_l - 2*inset;
board_r = max(0.5, corner_r - inset);
seat_y  = case_l/2 - edge_margin - piece_d/2;

function rot180(p) = [-p[0], -p[1]];
dpad1 = [-cluster_spacing/2, -seat_y];      // seat 1: D-pad on the player's left
abxy1 = [ cluster_spacing/2, -seat_y];
dpad2 = rot180(dpad1);  abxy2 = rot180(abxy1);
d = dpad_pitch/2;  a = abxy_pitch/2;

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
seat2_names = ["P2 up","P2 down","P2 left","P2 right","P2 A","P2 B","P2 X","P2 Y"];
seat2 = [for (i=[0:7]) [str("SW", i+9), seat2_names[i], rot180(seat1[i][2])]];
buttons = concat(seat1, seat2);

// ---------------- checks ----------------
assert(cluster_spacing >= piece_d + 2, "silicone pieces of one seat would touch");
assert(seat_y - piece_d/2 >= panel_l/2 + piece_to_glass - 1e-6, "cluster runs into the glass");
assert(cluster_spacing/2 + piece_d/2 <= board_w/2, "silicone piece hangs off the board");
assert(cell[0] <= board_w && cell[1] <= panel_l, "cell does not fit under the glass");
assert(cluster_spacing >= 30, "thumbs closer than the stripboard, which was already cramped");

echo(str("CASE    ", case_w, " x ", case_l, " mm   corner r ", corner_r));
echo(str("BOARD   ", board_w, " x ", board_l, " mm   corner r ", board_r, "   (case minus wall ", wall, " and clearance ", board_clear, ")"));
echo(str("GLASS   ", panel_w, " x ", panel_l, " centred;  window ", aa_w + 2*window_margin, " x ", aa_l + 2*window_margin));
echo(str("SEATS   cluster centres at y = +/-", seat_y, ", D-pad to ABXY ", cluster_spacing, " (set by glass_side ", glass_side, ")"));
for (b = buttons)
  echo(str(b[0], "  ", b[1], "   centre (", b[2][0], ", ", b[2][1], ")   KiCad (", 100 + b[2][0], ", ", 100 - b[2][1], ")"));
stack = 1.5 + cell[2] + 0.5 + 1.6 + 4.0 + 1.5;
echo(str("THICKNESS ESTIMATE ", stack, " mm = floor 1.5 + cell ", cell[2], " + 0.5 + PCB 1.6 + 4.0 above board + top 1.5"));
echo("ESTIMATES TO REPLACE WITH MEASUREMENTS: piece_d, cap sizes, aa_offset_y, usb, pwr, panel_t, 4.0 mm above-board height");

// ---------------- geometry ----------------
module rrect(w, l, r) offset(r=r) square([w - 2*r, l - 2*r], center=true);
module ring(t=0.35) difference() { offset(r=t) children(); children(); }
module at(p, z=0) translate([p[0], p[1], z]) children();
module label(p, s, size=2.2, z=2) at(p, z) color("white") linear_extrude(0.01) text(s, size=size, halign="center", valign="center");
module slab(z, h=0.05) translate([0,0,z]) linear_extrude(h) children();

module board_outline() rrect(board_w, board_l, board_r);

if (part == "board") board_outline();

if (part == "pads") {
  ring(0.1) board_outline();
  for (b = buttons) at(b[2]) { circle(d=pad_mask_d); }
}

if (part == "template") {
  ring(0.25) rrect(case_w, case_l, corner_r);
  ring(0.25) square([panel_w, panel_l], center=true);
  for (c = [dpad1, abxy1, dpad2, abxy2]) ring(0.25) circle(d=piece_d);
  for (c = [abxy1, abxy2]) for (o = [[a,0],[-a,0],[0,a],[0,-a]]) at(c + o) ring(0.25) circle(d=cap_abxy_d);
  for (c = [dpad1, dpad2]) at(c) ring(0.25) union() {
    square([dpad_arm_l, dpad_arm_w], center=true); square([dpad_arm_w, dpad_arm_l], center=true); }
  translate([-25, -case_l/2 - 12]) ring(0.25) square([50, 0.01]);           // 50 mm scale bar
  translate([0, -case_l/2 - 18]) text("50 mm: measure this before trusting the print", size=3, halign="center");
}

if (part == "layout") {
  color("#34343d") slab(0) rrect(case_w, case_l, corner_r);
  color("#2e7d4f") slab(0.1) board_outline();
  color([0.85, 0.35, 0.35, 0.55]) slab(0.2) square([cell[0], cell[1]], center=true);
  color("#101014") slab(0.3) square([panel_w, panel_l], center=true);
  color("#3b6fd1") slab(0.4) translate([0, aa_offset_y]) square([aa_w + 2*window_margin, aa_l + 2*window_margin], center=true);
  for (c = [dpad1, abxy1, dpad2, abxy2]) color([0.85, 0.85, 0.9, 0.45]) at(c, 0.5) linear_extrude(0.05) circle(d=piece_d);
  for (b = buttons) color("#f2c94c") at(b[2], 0.6) linear_extrude(0.05) circle(d=pad_mask_d);
  for (c = [abxy1, abxy2]) for (o = [[a,0],[-a,0],[0,a],[0,-a]])
    color("white") at(c + o, 0.7) linear_extrude(0.05) ring(0.3) circle(d=cap_abxy_d);
  for (c = [dpad1, dpad2]) color("white") at(c, 0.7) linear_extrude(0.05) ring(0.3) union() {
    square([dpad_arm_l, dpad_arm_w], center=true); square([dpad_arm_w, dpad_arm_l], center=true); }
  color("#f08a3c") slab(0.8) translate([case_w/2 - usb_d/2 - wall/2, 0]) square([usb_d, usb_w], center=true);
  color("#f08a3c") slab(0.8) translate([-(case_w/2 - pwr_d/2 - wall/2), 0]) square([pwr_d, pwr_w], center=true);
  // labels
  label([0, 0], "3.5in IPS", 3);
  label([0, -6], "cell 103450 under glass", 2);
  label([0, -seat_y - piece_d/2 - 3.4], "SEAT 1", 2.4);
  label([0,  seat_y + piece_d/2 + 3.4], "SEAT 2", 2.4);
  label([case_w/2 - 10, 9], "USB-C", 2);
  label([-(case_w/2 - 9), 9], "PWR", 2);
  for (b = buttons) label(b[2] + [0, (b[2][1] < 0 ? -1 : 1) * 0], b[0], 1.3, 2.1);
  label([0, -case_l/2 - 6], str(case_w, " x ", case_l, " mm case"), 3);
}

// ---------------- 3D ----------------
floor_t = 1.5;  top_t = 1.5;  pcb_t = 1.6;  above = 4.0;
panel_t = 2.4;                 // ESTIMATE: bare IPS glass + backlight
dome_h  = 2.0;                 // ESTIMATE: silicone piece height on the board
cap_proud = 1.2;               // caps stand this far above the top face
z_board = floor_t + cell[2] + 0.5;
z_top   = z_board + pcb_t + above;
case_h  = z_top + top_t;
z_glass = z_board + pcb_t + 1.0;
assert(z_glass + panel_t <= z_top, "glass pokes through the lid");
echo(str("3D      case height ", case_h, " mm, board top at ", z_board + pcb_t, ", glass top at ", z_glass + panel_t));

module cross(w, l) union() { square([l, w], center=true); square([w, l], center=true); }

module tub() difference() {
  linear_extrude(z_top) rrect(case_w, case_l, corner_r);
  translate([0, 0, floor_t]) linear_extrude(z_top) rrect(case_w - 2*wall, case_l - 2*wall, corner_r - wall);
  translate([case_w/2 - wall - 1, -usb_w/2, z_board + pcb_t]) cube([wall + 2, usb_w, 3.5]);
  translate([-case_w/2 - 1, -pwr_w/2, z_board + pcb_t]) cube([wall + 2, pwr_w, 3.0]);
}

module lid() difference() {
  linear_extrude(top_t) rrect(case_w, case_l, corner_r);
  translate([0, aa_offset_y, -1]) linear_extrude(top_t + 2)
    square([aa_w + 2*window_margin, aa_l + 2*window_margin], center=true);
  for (c = [abxy1, abxy2]) for (o = [[a,0],[-a,0],[0,a],[0,-a]])
    at(c + o, -1) cylinder(d=cap_abxy_d + 0.4, h=top_t + 2);
  for (c = [dpad1, dpad2]) at(c, -1) linear_extrude(top_t + 2) offset(r=0.2) cross(dpad_arm_w, dpad_arm_l);
}

module caps() {
  cap_h = case_h + cap_proud - (z_board + pcb_t + dome_h);
  for (c = [abxy1, abxy2]) for (o = [[a,0],[-a,0],[0,a],[0,-a]])
    at(c + o, z_board + pcb_t + dome_h) cylinder(d=cap_abxy_d, h=cap_h);
  for (c = [dpad1, dpad2]) at(c, z_board + pcb_t + dome_h) linear_extrude(cap_h) cross(dpad_arm_w, dpad_arm_l);
}

if (part == "3d") {
  color("#3a3a44") tub();
  color("#c9c9d1") translate([0, 0, floor_t]) linear_extrude(cell[2]) square([cell[0], cell[1]], center=true);
  color("#2e7d4f") translate([0, 0, z_board]) linear_extrude(pcb_t) board_outline();
  color("#101014") translate([0, 0, z_glass]) linear_extrude(panel_t) square([panel_w, panel_l], center=true);
  color("#3b6fd1") translate([0, aa_offset_y, z_glass + panel_t - 0.05]) linear_extrude(0.1) square([aa_w, aa_l], center=true);
  for (c = [dpad1, abxy1, dpad2, abxy2]) color("#8f8f99") at(c, z_board + pcb_t) cylinder(d=piece_d, h=dome_h);
  color("#e8e8ee") caps();                      // caps stay on the silicone when the lid lifts
  color("#4a4a56") translate([0, 0, z_top + explode]) lid();
}

// printable parts, each on its own
if (part == "tub") tub();
if (part == "lid") lid();
