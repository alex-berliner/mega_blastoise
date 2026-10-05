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
    for (p = KIT_PARTS) if (p[0] != "J3" && p[6] >= min_part_h)
      translate([p[1], p[2], 0]) cube([p[3] - p[1], p[4] - p[2], pcb_t + p[6]]);
  }
  for (s = pins) at(s, -1) cylinder(d = hole_d + 0.2, h = pcb_t + 2, $fn = 32);        // FDM holes print small
  translate([0, 0, pcb_t - engrave]) {
    // membrane contacts: the combs' edge (4.5 mm) and centre, where the silicone pills should land
    for (b = buttons) at(b[2]) groove() { ring(0.4) circle(r = 2.26, $fn = 48); circle(d = 0.8, $fn = 16); }
    // each silicone piece's outline (ESTIMATE in case.scad: piece_d)
    for (c = clusters) at(c) groove() ring(0.4) circle(d = piece_d, $fn = 96);
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

module kit_plug() {                                // JST PHR-2 plug (approximate), with a cable stub
  cube([6.0, 4.5, 6.0]);
  translate([1.4, 1.65, 6]) cube([3.2, 1.2, 8]);
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

band_y1 = -(bay_l / 2) + 2;                          // seat 1's end band: from the case end into the glass by 2 mm
module band_box() translate([-case_w / 2 - 1, -case_l / 2 - 1, -1]) cube([case_w + 2, band_y1 + case_l / 2 + 1, case_h + 2]);
module usb_box() translate([case_w / 2 - 12, usb_y - 12, -1]) cube([13, 24, case_h + 2]);
module frame_box() let(w = panel_w + panel_fit + 2 * frame_wall + 8, l = panel_l + panel_fit + 2 * frame_wall + 8)
  translate([-w / 2, -l / 2, -1]) cube([w, l, case_h + 2]);

if (kit == "board")        kit_board();
if (kit == "panel")        kit_panel();
if (kit == "cell")         kit_cell();
if (kit == "plug")         kit_plug();
if (kit == "speaker")      kit_speaker();
if (kit == "snap_tub")     intersection() { tub(); snap_box(hook_near([case_w / 2, 0])); }
if (kit == "snap_lid")     flip() intersection() { lid(); snap_box(hook_near([case_w / 2, 0])); }
if (kit == "buttons_tub")  intersection() { tub(); band_box(); }
if (kit == "buttons_board") intersection() { kit_board(); band_box(); }
if (kit == "buttons_lid")  flip() intersection() { lid(); band_box(); }
if (kit == "usb_tub")      intersection() { tub(); usb_box(); }
if (kit == "usb_lid")      flip() intersection() { lid(); usb_box(); }
if (kit == "frame_lid")    flip() intersection() { lid(); frame_box(); }
