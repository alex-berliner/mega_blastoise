// The assembly viewer's bought parts, each where case.scad puts it (case frame, mm).
// build.py renders one at a time: openscad -D part="none" -D piece="cell" pieces.scad
include <../case/case.scad>
piece = "none";
if (piece == "cell") translate([0, cell_y, floor_t]) linear_extrude(cell[2]) square([cell[0], cell[1]], center = true);
if (piece == "foam") translate([0, cell_y, floor_t + cell[2]]) linear_extrude(cell_gap - 0.05) square([cell[0] - 4, cell[1] - 4], center = true);
if (piece == "speaker") at(spk_pos, floor_t) cylinder(d = spk_d, h = spk_h);
if (piece == "panel") translate([0, 0, z_glass]) linear_extrude(panel_t) square([panel_w, panel_l], center = true);
if (piece == "silicone") {
  for (c = clusters) at(c, z_top_cu) cylinder(d = piece_d, h = dome_h);
  for (c = ss_centres) at(c, z_top_cu) linear_extrude(dome_h) rrect(ss_mem[0], ss_mem[1], 2);   // Start/Select
}
if (piece == "caps") caps();
if (piece == "screws") for (s = pins) at(s, 0) { cylinder(d = 4, h = 1, $fn = 32); cylinder(d = 2, h = 8, $fn = 24); }
echo(str("VIEWER ", case_h, " ", z_board));
