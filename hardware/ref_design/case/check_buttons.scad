// The button stack's geometry check, run by check_buttons.py: each `check` renders what two parts
// share (seat 1's end; seat 2's is the same turned round). Every one must come out empty, except
// caps_silicone, which may only touch (the domes hold the flanges up against the lip, no preload)
include <case.scad>

check = "none";
dpad_dir = 0;
module seat1() translate([-case_w, -case_l, -5]) cube([2 * case_w, case_l - bay_l / 2 + 2, case_h + 10]);   // y up to the glass
// the shell over the buttons: the lid, and with deck_inserts the insert
module shell() intersection() { seat1(); union() { lid(); if (deck_inserts) deck_insert(-1); } }
module seat(p) intersection() { seat1(); children(); }

if (check == "silicone_shell") intersection() { shell(); seat() silicone(); }
if (check == "caps_shell")     intersection() { shell(); seat() caps(); }
if (check == "pressed_shell")  intersection() { shell(); seat() caps(press = true, dpad_dir = dpad_dir); }
if (check == "caps_silicone")  intersection() { seat() caps(); seat() silicone(); }
if (check == "silicone_tub")   intersection() { seat() tub(); seat() silicone(); }
