// Fit kit: printable stand-ins and case coupons, to test the case against the real parts before
// the boards are ordered. Built by fitkit.py (python3 case/fitkit.py), which first writes
// fitkit_parts.scad: every board part's envelope from its 3D model, the boxes check_fit.py tests.
// Everything else comes from case.scad, so the kit always matches the case.
//
//   openscad -o x.stl -D 'part="kit"' -D 'kit="board"' fitkit.scad
include <case.scad>
include <fitkit_parts.scad>      // KIT_PARTS = [[ref, x0, y0, x1, y1, z0, z1], ...]: case coords, z above the board top

kit = "none";
engrave = 0.4;                   // groove depth for marks on the board mock and the panel dummy
min_part_h = 0.8;                // envelopes lower than this are left off the mock (0402 passives)

module groove(w = 0.4) linear_extrude(engrave + 0.01) children();

// ---------------- board mock: outline, locating holes, part envelopes, contact targets ----------------
module kit_board() difference() {
  union() {
    linear_extrude(pcb_t) board_outline();
    // each part's envelope, from the bed up so parts past the board edge (USB-C) print unsupported-free
    for (p = KIT_PARTS) if (p[0] != "J3" && p[0] != "J1" && p[6] >= min_part_h)
      translate([p[1], p[2], 0]) cube([p[3] - p[1], p[4] - p[2], pcb_t + p[6]]);
    // the USB-C receptacle as its shell (a stadium), face at the case's surface less usb_recess
    translate([usb_front - 7.3, usb_y, pcb_t + usb_h / 2]) rotate([0, 90, 0])
      linear_extrude(7.3) rrect(usb_h, usb_w, usb_h / 2 - 0.01);
  }
  for (s = pins) at(s, -1) cylinder(d = hole_d + 0.2, h = pcb_t + 2, $fn = 32);        // FDM holes print small
  translate([0, 0, pcb_t - engrave]) {
    // membrane contacts: the combs' edge (4.5 mm) and centre, where the silicone pills should land
    for (b = buttons) at(b[2]) groove() { ring(0.4) circle(r = 2.26, $fn = 48); circle(d = 0.8, $fn = 16); }
    // Start/Select: the small contacts and each membrane's outline (ESTIMATE in case.scad: ss_mem)
    for (b = ss_buttons) at(b[2]) groove() { ring(0.4) circle(r = 1.8, $fn = 40); circle(d = 0.8, $fn = 16); }
    for (c = ss_centres) at(c) groove() ring(0.4) rrect(ss_pad[0], ss_pad[1], 2);
    // each silicone pad's outline, where it goes (ESTIMATE in case.scad: pad2d)
    for (c = clusters) at(c) groove() ring(0.4) pad2d(c);
    // J3: tape a real Hirose FH12-50S on this outline (mouth towards seat 1, pin 1 marked)
    for (p = KIT_PARTS) if (p[0] == "J3") {
      groove() translate([p[1], p[2]]) ring(0.4) square([p[3] - p[1], p[4] - p[2]]);
      groove() translate([(p[1] + p[3]) / 2, p[2] - 1.6]) polygon([[-2, 1.2], [2, 1.2], [0, -0.6]]);   // mouth
      groove() translate([p[1] + 1.5, p[4] + 1.2]) circle(d = 1.2, $fn = 16);                          // pin 1 end
    }
    groove() translate([0, -31]) text("REF-A FIT MOCK", size = 3.2, halign = "center", valign = "center");
    groove() translate([0, -35.5]) text("J3: tape a real FH12-50S here", size = 2.0, halign = "center", valign = "center");
  }
}

// ---------------- loose-part dummies ----------------
module kit_panel() difference() {                 // the panel at its thickest (panel_t), glass side up
  translate([-panel_w / 2, -panel_l / 2, 0]) cube([panel_w, panel_l, panel_t]);
  translate([0, 0, panel_t - 0.3]) {
    translate([0, aa_off]) linear_extrude(1) ring(0.4) square([aa_w, aa_l], center = true);   // active area
    translate([fpc_neck[0], -panel_l / 2 + 0.6]) linear_extrude(1) square([fpc_neck[1], 1.2], center = true);  // FPC neck (ESTIMATE)
    translate([0, aa_off]) linear_extrude(1) text("PANEL DUMMY  seat 1 edge v", size = 3, halign = "center", valign = "center");
  }
}

module kit_cell() {                                // 403450 cell, as it lies sideways; wires leave its right-hand end
  linear_extrude(cell[2]) rrect(cell[0], cell[1], 1.0);
  translate([cell[0] / 2 - 0.01, -4, 0]) cube([5.01, 8, 1.0]);                        // protection board lead tab
}

module kit_plug() {                                // PicoBlade 51021-0200 plug (approximate), with a cable stub
  cube([plug_xs[0], plug_xs[1], 6.5]);
  translate([plug_xs[0] / 2 - 1.3, plug_xs[1] / 2 - 0.5, 6.5]) cube([2.6, 1.0, 8]);
}

module kit_speaker() difference() {               // 15 mm speaker (ESTIMATE in case.scad)
  cylinder(d = spk_d, h = spk_h, $fn = 96);
  translate([spk_d / 2 - 2.5, -1.5, spk_h - 1]) cube([3, 3, 2]);                       // lead exit
}

// ---------------- coupons: slices of the real case geometry, quick to print ----------------
function hook_near(p) = hooks[search(min([for (h = hooks) norm([h[0], h[1]] - p)]),
                                     [for (h = hooks) norm([h[0], h[1]] - p)])[0]];
module snap_box(h) translate([h[0], h[1], -1]) rotate(h[2]) translate([-8, -14, 0]) cube([16, 28, case_h + 2]);
module flip() translate([0, 0, case_h]) rotate([180, 0, 0]) children();               // the lid prints face down
// the tub as it prints: without its four feet, so the floor sits flat on the bed
module tub_flat() intersection() { tub(); translate([-200, -200, 0]) cube([400, 400, 100]); }

band_y1 = -(bay_l / 2) + 2;                          // seat 1's end band: from the case end into the glass by 2 mm
module band_box() translate([-case_w / 2 - 1, -case_l / 2 - 1, -1]) cube([case_w + 2, band_y1 + case_l / 2 + 1, case_h + 2]);
module usb_box() translate([case_w / 2 - 12, usb_y - 12, -1]) cube([13, 24, case_h + 2]);
module frame_box() let(w = panel_w + panel_fit + 2 * frame_wall + 8, l = panel_l + panel_fit + 2 * frame_wall + 8)
  translate([-w / 2, -l / 2, -1]) cube([w, l, case_h + 2]);

// ---------------- tuning coupons: small, quick, one question each ----------------
// tune_openings: the lid's button cuts, seen from the face, at the deck's real thickness (pocket
// floor to deck face). Top row: D-pad openings 0.6 / 0.8 / 1.0 mm past the cross. Bottom row: a
// whole keyed A/B/X/Y cluster (tab slots and notch ribs) and three plain hole sizes. Printed face
// down like the lid; the numbers are engraved in the face
tune_t = deck_face - retainer_bot;
module tune_openings() difference() {
  translate([-40, -31, 0]) cube([80, 60, tune_t]);
  for (i = [0:2]) translate([-26 + 26 * i, 15, -0.01]) dpad_cut(tune_t + 1, gap = 0.6 + 0.2 * i);
  for (i = [0:3]) let(t = abxy_tabs[i]) translate([-20 + t[1][0] * a, -15 + t[1][1] * a, -0.01])
    rotate(-t[2]) abxy_cut(tune_t + 1, [for (n = abxy_notches[i][1]) t[2] - n]);
  for (i = [0:2]) translate([12 + 10 * i, -15, -0.01]) rotate(-135) abxy_cut(tune_t + 1, [90], hole_d = 7.4 + 0.2 * i + (i > 0 ? 0.2 : 0));
  translate([0, 0, tune_t - engrave]) linear_extrude(engrave + 0.01) {
    for (i = [0:2]) translate([-26 + 26 * i, 2.5]) text(str(0.6 + 0.2 * i), size = 2.6, halign = "center", valign = "center");
    for (i = [0:2]) translate([12 + 10 * i, -25.5]) text(str(7.4 + 0.2 * i + (i > 0 ? 0.2 : 0)), size = 2.2, halign = "center", valign = "center");
    translate([-20, -29]) text("A B X Y keyed", size = 2.0, halign = "center", valign = "center");
  }
}
// tune_stack: one cluster's slice of the board and of the lid, the lid on four legs at its real
// height over the board, so the silicone and caps sit as in the case. Print a lid at each dome_h
// (the silicone's height, ESTIMATE in case.scad) and keep the one where the caps stand right
stack_half = 16;
// Start/Select's stack is smaller and off-centre: it sits against the case's end, so its box and
// legs stay inside the case (legs in the deck, clear of the membrane's retainer)
ss_box = [12.6, 5.1, 8.6];          // half width, reach towards the end, reach towards the screen
function box_of(c, ss) = ss ? [c[0] - ss_box[0], c[1] - ss_box[1], 2 * ss_box[0], ss_box[1] + ss_box[2]]
                            : [c[0] - stack_half, c[1] - stack_half, 2 * stack_half, 2 * stack_half];
module stack_box(c, ss = false) let(b = box_of(c, ss)) translate([b[0], b[1], -1]) cube([b[2], b[3], case_h + 2]);
module stack_legs(c, ss = false)
  for (p = ss ? [[-10.5, -4.3], [10.5, -4.3], [-10.5, 6.7], [10.5, 6.7]]
              : [for (sx = [-1, 1], sy = [-1, 1]) [sx * (stack_half - 3), sy * (stack_half - 3)]])
    at(c + p, z_top_cu) cylinder(d = 3, h = z_deck_lid - z_top_cu + 0.01);
module tune_stack_lid(c, ss = false) difference() {
  // lid_whole: the deck as one with the step, whether or not the case splits it into inserts
  union() { intersection() { lid_whole(); stack_box(c, ss); } stack_legs(c, ss); }
  at(c + (ss ? [0, 5.6] : [0, -stack_half + 1.8]), deck_face - engrave) linear_extrude(engrave + 0.01)
    text(str("DOME ", dome_h), size = ss ? 1.6 : 2.0, halign = "center", valign = "center");
}
module tune_stack_board(c, ss = false) intersection() { kit_board(); stack_box(c, ss); }

if (kit == "tune_openings")       flip_at(deck_face) translate([0, 0, retainer_bot]) tune_openings();
if (kit == "tune_stack_dpad_lid") flip() tune_stack_lid(dpad1);
if (kit == "tune_stack_abxy_lid") flip() tune_stack_lid(abxy1);
if (kit == "tune_stack_dpad_board") tune_stack_board(dpad1);
if (kit == "tune_stack_abxy_board") tune_stack_board(abxy1);
if (kit == "tune_stack_ss_lid")   flip() tune_stack_lid(ss_centres[0], true);
if (kit == "tune_stack_ss_board") tune_stack_board(ss_centres[0], true);
if (kit == "usb_board")    intersection() { kit_board(); usb_box(); }
module flip_at(z) translate([0, 0, z]) rotate([180, 0, 0]) children();

if (kit == "board")        kit_board();
if (kit == "panel")        kit_panel();
if (kit == "cell")         kit_cell();
if (kit == "plug")         kit_plug();
if (kit == "speaker")      kit_speaker();
if (kit == "snap_tub")     intersection() { tub_flat(); snap_box(hook_near([case_w / 2, 0])); }
if (kit == "snap_lid")     flip() intersection() { lid(); snap_box(hook_near([case_w / 2, 0])); }
if (kit == "buttons_tub")  intersection() { tub_flat(); band_box(); }
if (kit == "buttons_board") intersection() { kit_board(); band_box(); }
if (kit == "buttons_lid")  flip() intersection() { lid(); band_box(); }
// seat 1's deck insert, face down
if (kit == "buttons_insert" && deck_inserts) flip_at(deck_face) deck_insert(-1);
if (kit == "usb_tub")      intersection() { tub_flat(); usb_box(); }
if (kit == "usb_lid")      flip() intersection() { lid(); usb_box(); }
if (kit == "frame_lid")    flip() intersection() { lid(); frame_box(); }
