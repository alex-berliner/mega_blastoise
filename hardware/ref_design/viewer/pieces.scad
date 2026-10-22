// The assembly viewer's bought parts, each where case.scad puts it (case frame, mm).
// build.py renders one at a time: openscad -D part="none" -D piece="cell" pieces.scad
include <../case/case.scad>
piece = "none";
if (piece == "cell") cell3d();                                          // in the board's hole, on the floor's pocket
if (piece == "speaker") speaker3d();                                    // on the board's top, under the grille
if (piece == "panel") translate([0, 0, z_glass]) linear_extrude(panel_t) square([panel_w, panel_l], center = true);
if (piece == "silicone") {
  for (c = clusters) at(c, z_top_cu) cylinder(d = piece_d, h = dome_h);
  for (c = ss_centres) at(c, z_top_cu) linear_extrude(dome_h) rrect(ss_mem[0], ss_mem[1], 2);   // Start/Select
}
if (piece == "caps") caps();
if (piece == "decks") for (sy = [-1, 1]) deck(sy);                      // in place (the print pose is face down)
if (piece == "screws") for (s = pins) at(s, 0) { cylinder(d = 4, h = 1, $fn = 32); cylinder(d = 2, h = screw_cb[1] + screw_l, $fn = 24); }
echo(str("VIEWER ", case_h, " ", z_board));
