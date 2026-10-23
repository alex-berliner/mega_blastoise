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
    // (cut to the outline: until the board is re-placed for the 133 mm case some lie past it)
    intersection() {
      linear_extrude(case_h) board_outline();
      for (p = KIT_PARTS) if (p[0] != "J3" && p[0] != "J1" && p[6] >= min_part_h)
        translate([p[1], p[2], 0]) cube([p[3] - p[1], p[4] - p[2], pcb_t + p[6]]);
    }
    // the USB-C receptacle as its shell (a stadium), face at the case's surface less usb_recess
    translate([usb_front - 7.3, usb_y, pcb_t + usb_h / 2]) rotate([0, 90, 0])
      linear_extrude(7.3) rrect(usb_h, usb_w, usb_h / 2 - 0.01);
  }
  for (s = pins) at(s, -1) cylinder(d = hole_d + 0.2, h = case_h, $fn = 32);           // FDM holes print small
  translate([0, 0, pcb_t - engrave]) {
    // membrane contacts: the combs' edge (4.5 mm) and centre, where the silicone pills should land
    for (b = buttons) at(b[2]) groove() { ring(0.4) circle(r = 2.26, $fn = 48); circle(d = 0.8, $fn = 16); }
    // Start/Select: the small contacts and each membrane's outline (ESTIMATE in case.scad: ss_mem)
    for (b = ss_buttons) at(b[2]) groove() { ring(0.4) circle(r = 1.8, $fn = 40); circle(d = 0.8, $fn = 16); }
    for (c = ss_centres) at(c) groove() ring(0.4) rrect(ss_pad[0], ss_pad[1], 2);
    // each silicone pad's outline, where it goes (ESTIMATE in case.scad: pad2d)
    for (c = clusters) at(c) groove() ring(0.4) pad2d(c);
    // J3: tape a real Hirose FH12-50S on this outline (mouth towards seat 1, pin 1 marked), at its
    // nominal place; the case keeps the range it may move over clear (ticks at its two ends)
    let(y0 = -(panel_l / 2 - conn_in) - conn_front, y1 = -(panel_l / 2 - conn_in) + 2.0, x0 = fpc_x - j3_w / 2) {
      groove() translate([x0, y0]) ring(0.4) square([j3_w, y1 - y0]);
      groove() translate([fpc_x, y0 - 1.6]) polygon([[-2, 1.2], [2, 1.2], [0, -0.6]]);   // mouth
      groove() translate([x0 + 1.5, y1 + 1.2]) circle(d = 1.2, $fn = 16);                 // pin 1 end
      for (t = tip_range) groove() translate([x0 - 2.5, -(panel_l / 2 - conn_in_at(t)) - 0.2]) square([2, 0.4]);
    }
    // under the folded FPC, where the board has nothing tall (the cell's hole is where they were)
    groove() translate([0, -22]) text("REF-A FIT MOCK", size = 3.2, halign = "center", valign = "center");
    groove() translate([0, -17.5]) text("J3: tape a real FH12-50S above", size = 2.0, halign = "center", valign = "center");
  }
}

// ---------------- loose-part dummies ----------------
module kit_panel() difference() {                 // the panel at its thickest (panel_t), glass side up
  translate([-panel_w / 2, -panel_l / 2, 0]) cube([panel_w, panel_l, panel_t]);
  translate([0, 0, panel_t - 0.3]) {
    translate([0, aa_off]) linear_extrude(1) ring(0.4) square([aa_w, aa_l], center = true);   // active area
    translate([fpc_x, -panel_l / 2 + 0.6]) linear_extrude(1) square([fpc_w, 1.2], center = true);   // where the FPC leaves
    translate([0, aa_off]) linear_extrude(1) text("PANEL  seat 1 v", size = 3, halign = "center", valign = "center");
  }
}

module kit_cell() {                                // 403450 cell, nominal, as it lies in the board's hole; its leads leave its left end
  linear_extrude(cell_nom[2]) rrect(cell_nom[0], cell_nom[1], 1.0);
  translate([-cell_nom[0] / 2 - 1.5, -1.5, 0]) cube([1.51, 3, 1.0]);                       // where its leads leave it
}

module kit_plug() {                                // PicoBlade 51021-0200 plug, with a cable stub
  cube([plug_xs[0], plug_xs[1], plug_l]);
  translate([plug_xs[0] / 2 - 1.3, plug_xs[1] / 2 - 0.5, plug_l]) cube([2.6, 1.0, 8]);
}

module kit_speaker() difference() {               // GSPK151035PN: 10 x 15 x 3.5 racetrack, face up, as it lies on the board
  translate(-[spk_pos[0], spk_pos[1], 0]) linear_extrude(spk[2]) spk2d();
  translate(-[spk_pos[0], spk_pos[1], 0] + [0, 0, spk[2] - engrave]) linear_extrude(engrave + 0.01) ring(0.4) spk2d(-grille_in);   // its cone
}

// ---------------- coupons: slices of the real case geometry, quick to print ----------------
// the seam: the left wall round a crush rib, the tub's tongue and the top piece's skirt over it
module seam_box() translate([-case_w / 2 - 1, crush[2][1] - 12, -1]) cube([12, 24, case_h + 2]);
module flip() translate([0, 0, case_h]) rotate([180, 0, 0]) children();               // the top piece prints face down
// the tub as it prints, floor down (it has no feet since 2026-10-20, so that's the tub itself)
module tub_flat() tub();

band_y1 = -(bay_l / 2) + 2;                          // seat 1's end band: from the case end into the glass by 2 mm
module band_box() translate([-case_w / 2 - 1, -case_l / 2 - 1, -1]) cube([case_w + 2, band_y1 + case_l / 2 + 1, case_h + 2]);
module usb_box() translate([case_w / 2 - 12, usb_y - 12, -1]) cube([13, 24, case_h + 2]);
module frame_box() let(w = panel_w + panel_fit + 2 * frame_wall + 8, l = panel_l + panel_fit + 2 * frame_wall + 8)
  translate([-w / 2, -l / 2, -1]) cube([w, l, case_h + 2]);

// ---------------- tuning coupons: small, quick, one question each ----------------
// tune_openings: the deck's button cuts, seen from the face, at the deck's real thickness (pocket
// floor to deck face). Top row: D-pad openings 0.1 under, at and 0.1 over dpad_gap past the cross. Bottom row: a
// whole keyed A/B/X/Y cluster (tab slots and notch ribs) and three plain hole sizes round it. Printed
// face down like the deck; the numbers are engraved in the face
tune_t = deck_face - retainer_bot;
tune_holes = [for (o = [-0.4, -0.2, 0.2]) cap_hole_d + o];
tune_gaps = [for (o = [-0.1, 0, 0.1]) dpad_gap + o];
module tune_openings() difference() {
  translate([-40, -31, 0]) cube([80, 60, tune_t]);
  for (i = [0:2]) translate([-26 + 26 * i, 15, -0.01]) dpad_cut(tune_t + 1, gap = tune_gaps[i]);
  for (i = [0:3]) let(t = abxy_tabs[i]) translate([-20 + t[1][0] * a, -15 + t[1][1] * a, -0.01])
    rotate(-t[2]) abxy_cut(tune_t + 1, [for (n = abxy_notches[i][1]) t[2] - n]);
  for (i = [0:2]) translate([12 + 10 * i, -15, -0.01]) rotate(-135) abxy_cut(tune_t + 1, [90], hole_d = tune_holes[i]);
  translate([0, 0, tune_t - engrave]) linear_extrude(engrave + 0.01) {
    for (i = [0:2]) translate([-26 + 26 * i, 2.5]) text(str(tune_gaps[i]), size = 2.6, halign = "center", valign = "center");
    for (i = [0:2]) translate([12 + 10 * i, -25.5]) text(str(tune_holes[i]), size = 2.2, halign = "center", valign = "center");
    translate([-20, -29]) text("A B X Y keyed", size = 2.0, halign = "center", valign = "center");
  }
}
// tune_stack: one cluster's slice of the board and of the deck (its kit names still say "lid"), the
// deck slice on four legs at its real height over the board, so the silicone and caps sit as in the
// case. Print one at each dome_h
// (the silicone's height, ESTIMATE in case.scad) and keep the one where the caps stand right
stack_half = 16;
// Start/Select's stack is smaller and off-centre: it sits against the case's end, so its box and
// legs stay inside the case (legs in the deck, clear of the membrane's retainer)
ss_box = [12.6, 5.1, 8.6];          // half width, reach towards the end, reach towards the screen
// (a cluster's box 0.01 inside stack_half: at stack_half its sides lie on the case's inner wall
// faces, which leaves non-manifold edges)
function box_of(c, ss) = let(h = stack_half - 0.01) ss ? [c[0] - ss_box[0], c[1] - ss_box[1], 2 * ss_box[0], ss_box[1] + ss_box[2]]
                            : [c[0] - h, c[1] - h, 2 * h, 2 * h];
module stack_box(c, ss = false) let(b = box_of(c, ss)) translate([b[0], b[1], -1]) cube([b[2], b[3], case_h + 2]);
module stack_legs(c, ss = false)
  for (p = ss ? [[-10.5, -4.3], [10.5, -4.3], [-10.5, 3.5], [10.5, 3.5]]   // 3.5: clear of seat 1's Y cap hole
              : [for (sx = [-1, 1], sy = [-1, 1]) [sx * (stack_half - 3), sy * (stack_half - 3)]])
    // up into the deck (0.6 under its face), so a leg under one of the deck's reliefs still meets
    // solid deck and doesn't hang over the relief when printed face down (turned half a facet:
    // unturned, the old wedge's top layer ran exactly through two of a cluster leg's corners)
    at(c + p, z_top_cu) rotate(180 / $fn) cylinder(d = 3, h = deck_face - 0.6 - z_top_cu);
// It prints deck face down on the bed, as the deck does, so it needs no supports: a slice of the deck
// itself (less the laps at its side joints), its skirts and boss cut off at the walls' bottom so it
// stands on its legs
module tune_stack_lid(c, ss = false) difference() {
  union() {
    intersection() { deck_body(-1); stack_box(c, ss); translate([-case_w, -case_l, retainer_bot - 0.01]) cube([2 * case_w, 2 * case_l, case_h]); }
    stack_legs(c, ss);
  }
  at(c + (ss ? [0, 5.6] : [0, -stack_half + 1.8]), deck_face - engrave) linear_extrude(engrave + 0.01)
    text(str("DOME ", dome_h), size = ss ? 1.6 : 2.0, halign = "center", valign = "center");
}
module tune_stack_board(c, ss = false) intersection() { kit_board(); stack_box(c, ss); }

if (kit == "tune_openings")       flip_at(deck_face) translate([0, 0, retainer_bot]) tune_openings();
if (kit == "tune_stack_dpad_lid") flip_at(deck_face) tune_stack_lid(dpad1);
if (kit == "tune_stack_abxy_lid") flip_at(deck_face) tune_stack_lid(abxy1);
if (kit == "tune_stack_dpad_board") tune_stack_board(dpad1);
if (kit == "tune_stack_abxy_board") tune_stack_board(abxy1);
if (kit == "tune_stack_ss_lid")   flip_at(deck_face) tune_stack_lid(ss_centres[0], true);
if (kit == "tune_stack_ss_board") tune_stack_board(ss_centres[0], true);
if (kit == "usb_board")    intersection() { kit_board(); usb_box(); }
module flip_at(z) translate([0, 0, z]) rotate([180, 0, 0]) children();

if (kit == "board")        kit_board();
if (kit == "panel")        kit_panel();
if (kit == "cell")         kit_cell();
if (kit == "plug")         kit_plug();
if (kit == "speaker")      kit_speaker();
if (kit == "seam_tub")     intersection() { tub_flat(); seam_box(); }
if (kit == "seam_top")     flip() intersection() { top(); seam_box(); }
if (kit == "buttons_tub")  intersection() { tub_flat(); band_box(); }
if (kit == "buttons_board") intersection() { kit_board(); band_box(); }
if (kit == "buttons_top")  flip() intersection() { top(); band_box(); }
// seat 1's deck, face down
if (kit == "buttons_deck") flip_at(deck_face) deck(-1);
if (kit == "usb_tub")      intersection() { tub_flat(); usb_box(); }
if (kit == "usb_top")      flip() intersection() { top(); usb_box(); }
if (kit == "frame_top")    flip() intersection() { top(); frame_box(); }

// ---------------- test plate (6_tests): questions for the shell's redesign ----------------
// Overhangs: a ladder of fins, each a vertical wall then an underside sloping out test_reach at
// its angle from vertical (45 to 75), the angle engraved on its top; and a convex quarter round,
// as the shell's rounded edge would print, from near flat (85 degrees from vertical, where it
// leaves its stem) up to vertical, test_round in radius, ticks at 45, 60 and 75 degrees on its end
// face (the ladder's end). Printed as exported, no supports: the steepest clean fin and the curve's last clean tick
// are what the printer can do
test_angles = [45, 50, 55, 60, 65, 70, 75];
test_reach = 5;  test_wall = 2;  test_slab = 1.2;  test_fin = 5;  test_back = 5;
test_round = 6;  test_round_from = 85;  test_round_w = 10;
module test_fin(t) let(top = test_wall + test_reach / tan(t) + test_slab) difference() {
  rotate([90, 0, 0]) translate([0, 0, -test_fin]) linear_extrude(test_fin)
    polygon([[-test_back, 0], [0, 0], [0, test_wall], [test_reach, test_wall + test_reach / tan(t)], [test_reach, top], [-test_back, top]]);
  translate([-test_back / 2, test_fin / 2, top - engrave]) linear_extrude(engrave + 0.01)
    text(str(t), size = 2.6, halign = "center", valign = "center");
}
// the round's point at a tangent phi degrees from vertical, its stem's wall at x = 0
function test_round_c() = [-test_round * sin(90 - test_round_from), test_wall + test_round * cos(90 - test_round_from)];
function test_round_at(phi) = test_round_c() + test_round * [cos(phi), -sin(phi)];
module test_curve() let(c = test_round_c(), top = c[1] + test_slab) difference() {
  rotate([90, 0, 0]) translate([0, 0, -test_round_w]) linear_extrude(test_round_w)
    polygon([[-test_back, 0], [0, 0], for (p = [test_round_from:-2.5:0]) test_round_at(p), [c[0] + test_round, top], [-test_back, top]]);
  // ticks on the end face (y = 0), in from the surface along its normal (from 0.01 past it: at 45
  // degrees a tick's corner would touch a corner of the round), and their angles
  // (the numbers stand in a staircase up and in from their ticks: by the ticks they'd overlap)
  for (t = [[45, [2.5, -2.1]], [60, [0.3, -3.5]], [75, [-1.9, -4.9]]]) let(p = t[0], q = test_round_at(p), n = [cos(p), -sin(p)])
    translate([0, -0.01, 0]) rotate([-90, 0, 0]) linear_extrude(engrave + 0.01) mirror([0, 1]) {
      hull() { translate(q - 0.19 * n) circle(d = 0.4, $fn = 8); translate(q - 1.0 * n) circle(d = 0.4, $fn = 8); }
      translate(test_round_c() + t[1] * test_round / 6) text(str(p), size = 1.6, halign = "center", valign = "center");
    }
  translate([-test_back / 2, test_round_w / 2, top - engrave]) linear_extrude(engrave + 0.01)
    text(str("R", test_round), size = 2.4, halign = "center", valign = "center");
}
module test_overhangs() {        // the round first, its ticked face at the ladder's end
  test_curve();
  for (i = [0:len(test_angles) - 1]) translate([0, test_round_w + 2 + i * (test_fin + 2), 0]) test_fin(test_angles[i]);
  // the back strip that joins them
  translate([-test_back, 0, 0]) cube([test_back, len(test_angles) * (test_fin + 2) + test_round_w, test_wall]);
}
// Screw holes: two of the tub's (H4's slimmed side boss by the left wall and an end boss), cut from
// the real tub up to the board with the floor, ribs and wall round them, printed flat as the tub prints;
// and one of the top piece's bosses with its pilot hole, printed face down as the top piece prints
module screws_box(s, r) translate([s[0] - r, s[1] - r, -1]) cube([2 * r, 2 * r, case_h + 2]);
module coupon_screws_tub() for (i = [0, 1]) let(s = pins[[3, 4][i]]) translate([16 * i, 0, 0] - [s[0], s[1], 0])
  intersection() { tub_flat(); screws_box(s, 6); translate([-case_w, -case_l, -1]) cube([2 * case_w, 2 * case_l, z_board + 1]); }
module coupon_screws_top() translate(-[pins[0][0], -pins[0][1], 0]) flip() intersection() { top(); screws_box(pins[0], 5); }
// Clearance round a screw column (the old deck insert's ring round the lid's 5.0 boss was too tight
// at 0.15 a side): a boss_d column printed upright as the top piece's bosses are, and a strip of
// rings (test_ring_w walls, a deck's height over the board), holes test_fits a side over boss_d,
// printed on their plate as a deck prints. Since 2026-10-21 no column passes through another printed
// part (each deck has its own boss); the answer sets the clearance if one ever does again
test_fits = [0.2, 0.3, 0.4, 0.5];
test_ring_w = 1.2;                  // the rings' wall (the old inserts' anchor ring)
test_ring_h = deck_face - z_top_cu;
module test_column() difference() {
  union() {
    translate([-6, -6, 0]) cube([12, 12, top_t]);
    cylinder(d = boss_d, h = top_t + test_ring_h + 2, $fn = 96);
  }
  translate([0, -4.3, top_t - engrave]) linear_extrude(engrave + 0.01) text(str(boss_d), size = 2.4, halign = "center", valign = "center");
}
module test_rings() difference() {
  union() {
    translate([-6, -10, 0]) cube([11 * len(test_fits) + 1, 16, top_t]);
    for (i = [0:len(test_fits) - 1]) translate([11 * i, 0, 0])
      cylinder(d = boss_d + 2 * test_fits[i] + 2 * test_ring_w, h = top_t + test_ring_h, $fn = 96);
  }
  for (i = [0:len(test_fits) - 1]) translate([11 * i, 0, -1]) {
    cylinder(d = boss_d + 2 * test_fits[i], h = top_t + test_ring_h + 2, $fn = 96);
    translate([0, -7.2, top_t + 1 - engrave]) linear_extrude(engrave + 0.01) text(str(test_fits[i]), size = 2.4, halign = "center", valign = "center");
  }
}
// ---------------- overhang kit (7_overhangs): the steepest clean overhang, finely ----------------
// For `overhang` above: printed as exported, no supports. Every fin and round reaches out both ways
// along y, towards the bed's front (-y, engraved F on the pad) and its back (B), so one print says
// whether the side the part fan blows on does better. Each starts oh_lift up a stem with a 0.4 mm
// chamfer at the bed, clear of the first layers' elephant's foot.
// Ladder: double fins, each arm a case wall (`wall`, normal to its slope) reaching oh_reach out
// at its angle from vertical (less where that would rise past oh_rise: 30 to 50, which print anyway;
// short is quick), the angle engraved along the fin's column. 30 to 80, 2.5 apart from 50 to 70.
// Side by side with no gaps between them, as the usual overhang ladders: as fifteen islands every
// layer the plate took 29 minutes on a P1S at 0.20 mm, side by side 20
oh_angles = [30, 35, 40, 45, 50, 52.5, 55, 57.5, 60, 62.5, 65, 67.5, 70, 75, 80];
oh_reach = 8;  oh_rise = 6;  oh_lift = 2;  oh_col = 2.5;  oh_fin = 4;  oh_pitch = oh_fin;  oh_chamfer = 0.4;
oh_layer = "0.20";               // the layer height it's sliced at (fitkit.py renders one per plate)
oh_tag = "";                     // or the slicer settings it's printed with, in its place (DEF, TUNED)
oh_pad = [6, 2 * (oh_col + oh_reach), 1.0];        // the pad with F, B and the layer height, beside each part
module oh_pad(x) difference() {
  translate([x, -oh_pad[1] / 2, 0]) cube(oh_pad);
  translate([x + oh_pad[0] / 2, 0, oh_pad[2] - engrave]) linear_extrude(engrave + 0.01) {
    for (s = [-1, 1]) translate([0, s * (oh_pad[1] / 2 - 2.5)]) text(s < 0 ? "F" : "B", size = 3, halign = "center", valign = "center");
    rotate(90) text(oh_tag == "" ? oh_layer : oh_tag, size = 2.4, halign = "center", valign = "center");
  }
}
// a profile in the x-z plane (x out from the column's middle, both ways) extruded w along y, then
// turned so its x runs along the bed's y and its extrusion along the bed's x, from x = 0
module oh_extrude(w) translate([w, 0, 0]) rotate([90, 0, -90]) linear_extrude(w) children();
// one side of the stem: from the bed's chamfer up to oh_lift, where the overhang leaves it
function oh_stem() = [[oh_col - oh_chamfer, 0], [oh_col, oh_chamfer], [oh_col, oh_lift]];
function oh_mirror(side) = concat(side, [for (i = [len(side) - 1:-1:0]) [-side[i][0], side[i][1]]]);
module oh_fin(t) let(reach = min(oh_reach, oh_rise * tan(t)), rise = reach / tan(t), v = wall / sin(t),
                     top = oh_lift + rise + v) difference() {
  oh_extrude(oh_fin) polygon(oh_mirror(concat(oh_stem(), [[oh_col + reach, oh_lift + rise], [oh_col + reach, top],
                                                         [oh_col, oh_lift + v], [oh_col, top]])));
  translate([oh_fin / 2, 0, top - engrave]) linear_extrude(engrave + 0.01)
    rotate(90) text(str(t), size = 1.5, halign = "center", valign = "center");
}
module test_overhang_ladder() {
  for (i = [0:len(oh_angles) - 1]) translate([i * oh_pitch, 0, 0]) oh_fin(oh_angles[i]);
  oh_extrude((len(oh_angles) - 1) * oh_pitch + oh_fin) polygon(oh_mirror(oh_stem()));   // the strip joining them
  oh_pad(-oh_pad[0] - 1);
  translate([-1.01, -oh_col, 0]) cube([1.02, 2 * oh_col, oh_pad[2]]);
}
// Rounds: solid convex quarter rounds as the shell's edges print (face down, the round leaving the
// bed flat and turning up towards vertical), from 80 degrees from vertical at the stem up to 25,
// then 1 mm straight up. Ticks on each block's -x end face, in along the surface's normal at its angle
// from vertical (from 0.01 past the surface, as test_curve's): every 5 degrees (R10) or 10 (R3, R6) from 30 to 80, long at 30, 50 and 70
// (R10's numbered). Its radius on its top
oh_radii = [10, 6, 3];  oh_from = 80;  oh_to = 25;  oh_block = 5;  oh_gap = 6;
function oh_c(r) = [oh_col - r * cos(oh_from), oh_lift + r * sin(oh_from)];
function oh_at(r, phi) = oh_c(r) + r * [cos(phi), -sin(phi)];
module oh_round(r) let(e = oh_at(r, oh_to), top = e[1] + 1, step = r >= 10 ? 5 : 10) difference() {
  oh_extrude(oh_block) polygon(oh_mirror(concat(oh_stem(), [for (p = [oh_from:-2.5:oh_to]) oh_at(r, p)],
                                               [[e[0], top], [0, top]])));
  // -x end face: the profile's plane, seen from -x the front (-y) is on the left
  // (the numbers mirrored so they read from -x)
  for (s = [-1, 1]) translate([-0.01, 0, 0]) rotate([90, 0, 90]) linear_extrude(engrave + 0.01) {
    for (p = [30:step:oh_from]) let(q = oh_at(r, p), n = [cos(p), -sin(p)], d = p % 20 == 10 ? 2.0 : 1.0)
      scale([s, 1]) hull() { translate(q - 0.14 * n) circle(d = 0.3, $fn = 8); translate(q - d * n) circle(d = 0.3, $fn = 8); }
    if (r >= 10) for (p = [30, 50, 70]) let(n = [cos(p), -sin(p)], q = oh_at(r, p) - 3.7 * n)
      translate([s * q[0], q[1]]) mirror([1, 0]) text(str(p), size = 1.2, halign = "center", valign = "center");
  }
  translate([oh_block / 2, 0, top - engrave]) linear_extrude(engrave + 0.01)
    rotate(90) text(str("R", r), size = 2.2, halign = "center", valign = "center");
}
module test_overhang_rounds() {
  for (i = [0:len(oh_radii) - 1]) translate([i * (oh_block + oh_gap), 0, 0]) oh_round(oh_radii[i]);
  n = len(oh_radii) * (oh_block + oh_gap) - oh_gap;
  oh_extrude(n) polygon(oh_mirror(oh_stem()));
  oh_pad(n + 1);
  translate([n - 0.01, -oh_col, 0]) cube([1.02, 2 * oh_col, oh_pad[2]]);
}

// ---------------- first layer (8_first_layer_*): the tub's bottom edge ----------------
// A corner of the tub's outside, fl_size square and fl_h tall, solid: the round from the bed on its
// two outer sides as the tub prints it (tub_outside), straight cut sides inside. Printed with the
// first layer and elephant's-foot compensation fl_label names (L20 E15: 0.20 mm and 0.15 mm), to
// find the pair that leaves no lip where the round meets the bed
// (the 45 degree tub's edge, whatever `overhang` is: the overhang kit is one kit for both lines of the
// case, and the 60 degree variant only differs in the round's last 1.2 mm, where it leaves the bed)
fl_size = 25;  fl_h = 3;  fl_label = "L20 E15";  fl_overhang = 45;
module test_first_layer() difference() {
  translate([fl_size - case_w / 2, fl_size - case_l / 2, 0]) intersection() {
    tub_outside(ov = fl_overhang);
    translate([case_w / 2 - fl_size, case_l / 2 - fl_size, -1]) cube([fl_size + 1, fl_size + 1, fl_h + 1]);
  }
  translate([fl_size / 2 - 1.5, fl_size / 2 - 1.5, fl_h - engrave]) linear_extrude(engrave + 0.01)
    text(fl_label, size = 3.2, halign = "center", valign = "center");
}

if (kit == "test_overhangs")    test_overhangs();
if (kit == "overhang_ladder")   test_overhang_ladder();
if (kit == "overhang_rounds")   test_overhang_rounds();
if (kit == "first_layer")       test_first_layer();
if (kit == "test_screws_tub")   coupon_screws_tub();
if (kit == "test_screws_top")   coupon_screws_top();
if (kit == "test_column")       test_column();
if (kit == "test_rings")        test_rings();
