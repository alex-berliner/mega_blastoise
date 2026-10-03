// Fit-test print variant of the tub: the same fit geometry, easier to print.
//   tub_print: the tub without its four feet. They stood 0.8 mm proud of the floor, so the
//              whole floor floated off the bed and needed support; without them it sits flat.
//              Nothing the tub fits against changes.
// The lid needs no variant: printed face down, only its four button decks need the slicer's
// supports.
//
//   openscad -o print/tub_print.stl -D 'variant="tub"' print_variants.scad
include <case.scad>

variant = "none";

if (variant == "tub") intersection() { tub(); translate([-200, -200, 0]) cube([400, 400, 100]); }
