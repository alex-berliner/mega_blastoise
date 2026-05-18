// Mega Blastoise — enclosure  ::  VARIANT: dual-oled-128x64
// Two per-player 128x64 mono SSD1306 OLEDs (web-mirrored layout).
// Parametric two-part clamshell. See cad/DESIGN.md, cad/PARTS.md, cad/METHODOLOGY.md.
//
// Layout mirrors the web client per player: OLED centred, 4 move buttons at
// the OLED corners, a 3-button switch row directly under it, then a small
// LED row (1 RGB HP LED + 3 party LEDs). Buzzer is centred (fair for 2P).
//
// Render selectors (override with -D):
//   part : "assembly" | "internals" | "tub" | "faceplate"
//          ("internals" = tub + component ghosts, no faceplate covering them)
//   cut  : "none" | "width" | "length"
//   check: false | true     (skip geometry, run asserts+echo only)

part  = "faceplate";
cut   = "none";
check = false;
show_ghosts = true;     // assembly view: draw placeholder components

/* ───────────────────────── pinned parts (cad/PARTS.md) ───────────────────── */
rp2040  = [23.5, 18.0, 4.5];
oled    = [27.3, 27.3, 4.1];
oled_active = [21.74, 10.86];
// Integrated charge+boost+protection (MH-CD42 / IP5306): replaces the
// separate TP4056 + MT3608. Lies flat (≈7 mm < cavity) → no bump-out.
pmod    = [34.0, 22.0, 7.0];
lipo    = [50.0, 34.0, 5.5];
slide_sw   = [8.7, 3.6, 3.5];
buzzer_d   = 12.0;  buzzer_h = 9.0;
btn        = 12.0;  btn_pitch = 15.24;     // switch row, grid-locked 6 * 2.54
btn_hole_d = 4.5;                          // actuator clearance through faceplate
move_gap   = 3.0;                          // gap between OLED edge and move btn
hp_led_d     = 5.0;                        // single RGB HP-LED window
party_led_d  = 3.0;  party_led_n = 3;  party_led_pitch = 6.0;
led_group_gap = 5.0;                       // HP-LED → party group
led_win_clear = 0.6;
strip_grid = 2.54;
clearance  = 0.4;
wall_t     = 2.0;
floor_t    = 2.0;
faceplate_t   = 2.5;
led_diffuser_t = 1.0;                      // translucent front skin over LEDs
oled_lip_t     = 1.0;                      // bezel lip that hides OLED PCB edge
interior_depth = 9.0;                      // tub cavity for battery/power stack
boost_bumpout  = false;     // not needed with the flat integrated pmod
insert_boss_od = 5.0;  insert_bore_d = 3.6;
screw_d        = 2.7;  screw_head_d  = 5.0;
bed = [220, 220];
corner_r = 4;  $fn = 48;
eps = 0.02;                                // overshoot so no cut face is coplanar
rib_w = 2.5;  rib_h = 2;  cs_depth = 1.2;

/* ───────────────────────── derived geometry ─────────────────────────────── */
side_margin = 7;
end_margin  = 6;
g_oled   = 5;       // OLED block → switch row
g_switch = 5;       // switch row → LED row
center_gap = 12;    // central band (divider + shared buzzer)

move_dx = oled[0]/2 + move_gap + btn/2;        // move-btn column offset from centre
move_dy = oled[1]/2 - btn/2;                   // move-btn rows = OLED top/bottom
switch_row_w = (btn_pitch)*(2) + btn;          // 3 switch buttons
content_w = max(2*move_dx + btn, switch_row_w);
board_w   = content_w + 2*side_margin;

led_row_w = hp_led_d + led_group_gap + (party_led_n-1)*party_led_pitch + party_led_d;
led_row_h = max(hp_led_d, party_led_d);

// per-player feature offsets from the player's outer edge
o_oled   = end_margin + oled[1]/2;
o_switch = end_margin + oled[1] + g_oled + btn/2;
o_led    = o_switch + btn/2 + g_switch + led_row_h/2;
half_l   = o_led + led_row_h/2 + center_gap/2;
board_l  = 2*half_l;
tub_h    = floor_t + interior_depth;
total_h  = tub_h + faceplate_t;

function yc(p, o) = p * (half_l - o);           // p=-1 P1 (bottom), p=+1 P2 (top)

inset = side_margin + 1;
screw_pts = concat(
  [ for (sx=[-1,1], sy=[-1,1]) [sx*(board_w/2-inset), sy*(board_l/2-inset)] ],
  [ [board_w/2-inset, 0], [-(board_w/2-inset), 0] ]);
rib_half = board_w/2 - inset - 4;               // stops short of mid-side screws

// LED-row X positions (group centred): HP led then 3 party leds
led0_x = -led_row_w/2 + hp_led_d/2;
function party_x(i) = -led_row_w/2 + hp_led_d + led_group_gap
                      + i*party_led_pitch + party_led_d/2;

/* ───────────────────────── self-checks ──────────────────────────────────── */
assert(board_w <= bed[0] && board_l <= bed[1],
       str("Board ", board_w, "x", board_l, " exceeds bed ", bed));
assert(abs(btn_pitch/strip_grid - round(btn_pitch/strip_grid)) < 1e-3,
       "btn_pitch must be an integer multiple of stripboard grid 2.54");
assert(move_dx - btn/2 >= oled[0]/2,
       "move buttons overlap the OLED — increase move_gap");
assert(interior_depth >= lipo[2] + clearance + 1,
       "interior_depth too shallow for the LiPo");
assert(faceplate_t > led_diffuser_t && faceplate_t > oled_lip_t,
       "faceplate too thin for diffuser skin / OLED lip");
assert(content_w + 2*side_margin <= board_w + 1e-6, "content overflows width");
assert(boost_bumpout || interior_depth >= pmod[2] + clearance,
       "power module won't fit: deepen interior_depth or enable boost_bumpout");

echo(board_w = board_w, board_l = board_l, total_h = total_h, tub_h = tub_h,
     half_l = half_l, content_w = content_w, led_row_w = led_row_w,
     o_oled = o_oled, o_switch = o_switch, o_led = o_led);

/* ───────────────────────── helpers ──────────────────────────────────────── */
module rrect(w, l, r) hull() for (sx=[-1,1], sy=[-1,1])
  translate([sx*(w/2-r), sy*(l/2-r)]) circle(r);
module slab(w, l, h, r) linear_extrude(h) rrect(w, l, r);

// blind LED window: pocket from the underside, led_diffuser_t translucent
// front skin remains (lit through it). Centred at origin.
module led_window(d)
  translate([0,0,-eps])
    cylinder(d=d+2*led_win_clear, h=faceplate_t - led_diffuser_t + eps);

module button_centres() {
  for (p=[-1,1]) {
    // 4 move buttons at the OLED corners
    for (sx=[-1,1], sy=[-1,1])
      translate([sx*move_dx, yc(p,o_oled) + sy*move_dy]) children();
    // 3 switch buttons in a row under the OLED
    for (i=[-1:1]) translate([i*btn_pitch, yc(p,o_switch)]) children();
  }
}

/* ───────────────────────── bottom tub ───────────────────────────────────── */
// blister geometry — only engages if the power module is taller than the
// cavity. With the flat integrated pmod this is negative, so no bump-out.
boost_drop = pmod[2] + clearance - interior_depth;       // depth below floor
boost_c = [ board_w/2 - pmod[0]/2 - side_margin,
           -board_l/2 + pmod[1]/2 + end_margin ];

module bottom_tub() {
  difference() {
    union() {
      slab(board_w, board_l, tub_h, corner_r);
      if (boost_bumpout && boost_drop > 0)
        translate([boost_c[0], boost_c[1], -boost_drop])
          slab(pmod[0]+2*(wall_t+clearance),
               pmod[1]+2*(wall_t+clearance),
               boost_drop + floor_t, 2);
    }
    translate([0,0,floor_t])
      slab(board_w-2*wall_t, board_l-2*wall_t, tub_h, max(1,corner_r-wall_t));
    // hollow the blister so the tall boost module actually drops in,
    // leaving floor_t of plastic at its base
    if (boost_bumpout && boost_drop > 0)
      translate([boost_c[0], boost_c[1], -boost_drop+floor_t])
        linear_extrude(boost_drop + eps)
          square([pmod[0]+2*clearance, pmod[1]+2*clearance], center=true);
    // USB-C charge pass-through (pmod) + slide-switch slot, +X edge, one end
    translate([board_w/2-wall_t-1, -board_l/2+end_margin+pmod[1]/2, floor_t+2])
      cube([wall_t+3, 9, 4], center=true);
    translate([board_w/2-wall_t-1, -board_l/2+end_margin+pmod[1]+8,
               floor_t+slide_sw[2]/2+1])
      cube([wall_t+3, slide_sw[0]+1, slide_sw[2]+1], center=true);
    for (pt = screw_pts) translate([pt[0], pt[1], floor_t])
      cylinder(d=insert_bore_d, h=tub_h);
  }
  // insert bosses
  for (pt = screw_pts) translate([pt[0], pt[1], floor_t])
    difference() {
      cylinder(d=insert_boss_od, h=interior_depth);
      cylinder(d=insert_bore_d, h=interior_depth+1);
    }
  // centred buzzer locating collar on the floor
  translate([0,0,floor_t]) difference() {
    cylinder(d=buzzer_d+3, h=3);
    translate([0,0,-eps]) cylinder(d=buzzer_d+1, h=3+2*eps);
  }
}

/* ───────────────────────── top faceplate ────────────────────────────────── */
module top_faceplate() {
  difference() {
    union() {
      slab(board_w, board_l, faceplate_t, corner_r);
      // centre divider rib (stops short of the mid-side screws)
      translate([-rib_half, -rib_w/2, 0])
        cube([2*rib_half, rib_w, faceplate_t + rib_h]);
    }
    // OLED windows — module seats from the tub side, front bezel lip remains
    for (p=[-1,1]) translate([0, yc(p,o_oled), 0]) {
      translate([0,0,-eps])
        linear_extrude(faceplate_t - oled_lip_t + eps)
          square([oled[0]+2*clearance, oled[1]+2*clearance], center=true);
      translate([0,0,-eps])
        linear_extrude(faceplate_t + 2*eps)
          square([oled_active[0]+2, oled_active[1]+2], center=true);
    }
    // button actuator holes (4 move + 3 switch per player)
    button_centres()
      translate([0,0,-eps]) cylinder(d=btn_hole_d, h=faceplate_t+2*eps);
    // LED windows: 1 HP + 3 party per player (blind, diffused front skin)
    for (p=[-1,1]) translate([0, yc(p,o_led), 0]) {
      translate([led0_x, 0, 0]) led_window(hp_led_d);
      for (i=[0:party_led_n-1])
        translate([party_x(i), 0, 0]) led_window(party_led_d);
    }
    // centred buzzer grille
    for (a=[0:45:359]) translate([cos(a)*3.5, sin(a)*3.5, -eps])
      cylinder(d=1.6, h=faceplate_t+2*eps);
    translate([0,0,-eps]) cylinder(d=1.6, h=faceplate_t+2*eps);
    // countersunk screw holes
    for (pt = screw_pts) translate([pt[0], pt[1], 0]) {
      translate([0,0,-eps]) cylinder(d=screw_d+0.4, h=faceplate_t+2*eps);
      translate([0,0,faceplate_t-cs_depth])
        cylinder(d1=screw_d, d2=screw_head_d, h=cs_depth+eps);
    }
  }
}

/* ───────────────────────── component placeholders ───────────────────────── */
module ghosts() {
  color("teal",0.5)   translate([-board_w/2+side_margin,
                                  -board_l/2+end_margin+8, floor_t])
                         cube([lipo[1], lipo[0], lipo[2]]);   // battery, one end
  color("purple",0.6) translate([-rp2040[0]/2, -rp2040[1]/2, floor_t])
                         cube(rp2040);                          // MCU, centre-ish
  color("orange",0.6) translate([board_w/2-side_margin-pmod[0],
                                  -board_l/2+end_margin, floor_t])
                         cube(pmod);                          // integrated pmod
  for (p=[-1,1]) color("red",0.6)
    translate([led0_x, yc(p,o_led), tub_h]) cylinder(d=hp_led_d, h=1);
}

/* ───────────────────────── assemble + section ───────────────────────────── */
module scene() {
  if (part=="tub" || part=="assembly" || part=="internals") bottom_tub();
  if (part=="faceplate" || part=="assembly")
    translate([0,0, tub_h + (part=="assembly" ? 6 : 0)]) top_faceplate();
  // ghosts only in assembly/internals — never in pure tub/faceplate, so
  // STL exports of the printed parts stay clean.
  if ((part=="assembly" || part=="internals") && show_ghosts) ghosts();
}

if (!check) {
  if (cut=="none") scene();
  else difference() {
    scene();
    big = max(board_w, board_l) + 50;
    if (cut=="width") translate([0, big/2, -50]) cube([big,big,big]);
    else              translate([big/2, 0, -50]) cube([big,big,big]);
  }
}
