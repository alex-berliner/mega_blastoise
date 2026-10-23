// The island's coupons at 45 and 60 degrees (island_coupons.py renders each piece twice, with -D overhang=45 and
// -D overhang=60, and lays both sets out on one plate). Each piece is cut from the real part as case.scad builds it at
// that limit, in that part's print pose (the island and the decks face down), with a label tab on one of its cut
// faces: a plate on the bed carrying the limit in raised figures, "45" or "60". The tab joins the cut face only
// (eps into it), so the real surfaces are as the case has them.
//
//   piece = "corner_island"  seat 1's right corner of the island: its edge profile round the corner and its wedge
//           "corner_deck"    seat 1's deck round it: the core's lip across the step and the right arm's beside it
//           "meet_seat1"     seat 1's left arm's end at arm_meet_y, its lap
//           "meet_seat2"     seat 2's left arm's end there, the skirt the lap goes behind
//           "ear_island"     the ear's seat-1 end (its lap) with the island's side beside it
//           "ear_arm"        seat 1's right arm's end under it
include <case.scad>

piece = "none";
tab = [7.0, 5.0, 0.8];         // the label tab: across, out from the cut face, thick (on the bed)
tab_text = 3.2;  tab_relief = 0.4;

// the boxes each pair is cut from (case coordinates): [[x0, y0], [x1, y1]], from z = -1 to over the screen face
corner_c = [isl_x, -joint_y];  // the island's corner (its plan's corner, r isl_r round)
box_corner_island = [[corner_c[0] - 11, corner_c[1] - 3], [case_w / 2 + 1, corner_c[1] + 9]];
// (the deck's strip across the step stops 0.35 short of the X button's clover, 36.6 from the case's centre)
box_corner_deck = [[corner_c[0] - 11, -(joint_y + 1.9)], [case_w / 2 + 1, corner_c[1] + 9]];
box_meet = [[-case_w / 2 - 1, arm_meet_y - 7], [-case_w / 2 + 6, arm_meet_y + 7]];
box_ear = [[case_w / 2 - 6, ear_y[0] - 5], [case_w / 2 + 1, ear_y[0] + 5.5]];
module box(b) translate([b[0][0], b[0][1], -1]) cube([b[1][0] - b[0][0], b[1][1] - b[0][1], case_h + 2]);

// the tab, in the print pose: at the cut face's point p (print x, y), out along d, on the bed
module label_tab(p, d) let(a = atan2(d[1], d[0])) translate([p[0], p[1], 0]) rotate(a) {
  translate([-eps * 5, -tab[0] / 2, 0]) cube([tab[1] + eps * 5, tab[0], tab[2]]);
  translate([tab[1] / 2, 0, tab[2] - eps]) linear_extrude(tab_relief + eps)
    rotate(-a) text(str(overhang), size = tab_text, font = "Liberation Sans:style=Bold", halign = "center", valign = "center");
}
// case (x, y) to the print pose's (both poses turn the part over about x)
function pp(p) = [p[0], -p[1]];
module island_pose() translate([0, 0, case_h]) rotate([180, 0, 0]) children();
module deck_pose() translate([0, 0, deck_face]) rotate([180, 0, 0]) children();

if (piece == "corner_island") {
  island_pose() intersection() { top(); box(box_corner_island); }
  label_tab(pp([box_corner_island[0][0], corner_c[1] + 4]), [-1, 0]);
}
if (piece == "corner_deck") {
  deck_pose() intersection() { deck(-1); box(box_corner_deck); }
  label_tab(pp([case_w / 2 - 1.5, box_corner_deck[1][1]]), pp([0, 1]));
}
if (piece == "meet_seat1") {
  deck_pose() intersection() { deck(-1); box(box_meet); }
  label_tab(pp([-case_w / 2 + 1.5, box_meet[0][1]]), pp([0, -1]));
}
if (piece == "meet_seat2") {
  deck_pose() intersection() { deck(1); box(box_meet); }
  label_tab(pp([-case_w / 2 + 1.5, box_meet[1][1]]), pp([0, 1]));
}
if (piece == "ear_island") {
  island_pose() intersection() { top(); box(box_ear); }
  label_tab(pp([case_w / 2 - 3, box_ear[1][1]]), pp([0, 1]));
}
if (piece == "ear_arm") {
  deck_pose() intersection() { deck(-1); box(box_ear); }
  label_tab(pp([case_w / 2 - 1.5, box_ear[0][1]]), pp([0, -1]));
}
