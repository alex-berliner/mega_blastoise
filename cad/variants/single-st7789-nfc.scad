// Mega Blastoise — enclosure  ::  VARIANT: single-st7789-nfc
// Based on single-st7789. The 3 per-player switch/party buttons are REMOVED
// and replaced by one octagonal recess per player sized for a flat NFC
// "Pokémon coin" (octagonal chip with an embedded NFC sticker). Dropping a
// coin into a player's recess selects/plays that Pokémon. Move buttons (4)
// stay; one shared central ST7789 colour TFT.
//
// NFC is electronics-out-of-scope today (see DESIGN.md): a per-player NFC
// reader (e.g. PN532) under each recess is assumed but only ghosted here.
//
// Render selectors:  part assembly|internals|tub|faceplate  cut|check|show_ghosts

part  = "assembly";
cut   = "none";
check = false;
show_ghosts = true;

/* ───────────────────────── pinned parts ─────────────────────────────────── */
rp2040  = [23.5, 18.0, 4.5];
disp    = [32.0, 33.0, 4.5];                // ST7789 1.54" 240x240 SPI
disp_active = [27.7, 27.7];
disp_lip_t  = 1.0;
disp_conn   = 5.0;
pmod    = [34.0, 22.0, 7.0];                // MH-CD42 charge+boost+protect
lipo    = [50.0, 34.0, 5.5];
slide_sw   = [8.7, 3.6, 3.5];
buzzer_d   = 12.0;  buzzer_h = 9.0;
btn        = 12.0;  btn_pitch = 15.24;
btn_hole_d = 4.5;
hp_led_d     = 5.0;
party_led_d  = 3.0;  party_led_n = 3;  party_led_pitch = 6.0;
led_group_gap = 5.0;
led_win_clear = 0.6;
// NFC coin: regular octagon, flat-topped. nfc_coin_d = circumscribed dia
// (corner-to-corner); flat-to-flat ≈ 0.92×. Fits an NFC sticker ≤ ~22 mm.
nfc_coin_d = 24.0;  nfc_coin_t = 1.2;
nfc_reader = [40.0, 30.0, 4.0];             // PN532-class, ghost only
strip_grid = 2.54;
clearance  = 0.4;
wall_t     = 2.0;
floor_t    = 2.0;
faceplate_t   = 2.5;
led_diffuser_t = 1.0;
interior_depth = 9.0;
boost_bumpout  = false;
insert_boss_od = 5.0;  insert_bore_d = 3.6;
screw_d        = 2.7;  screw_head_d  = 5.0;
bed = [220, 220];
corner_r = 4;  $fn = 48;
eps = 0.02;  cs_depth = 1.2;

/* ───────────────────────── derived geometry ─────────────────────────────── */
side_margin = 5;
end_margin  = 5;
g_disp = 4;     // display ↔ LED row
g_led  = 3;     // LED row ↔ move row
g_nfc  = 4;     // move row ↔ NFC recess

led_row_h  = max(hp_led_d, party_led_d);
move_row_w = 3*btn_pitch + btn;             // 4 move buttons
nfc_recess_d = nfc_coin_d + 2*clearance;
nfc_r        = nfc_recess_d/2;
nfc_floor    = faceplate_t - (nfc_coin_t + clearance);  // plastic under coin
content_w  = max(move_row_w, disp[0], nfc_recess_d);
board_w    = content_w + 2*side_margin;

y_led  = disp[1]/2 + g_disp + led_row_h/2;
y_move = y_led + led_row_h/2 + g_led + btn/2;
y_nfc  = y_move + btn/2 + g_nfc + nfc_r;
half_l = y_nfc + nfc_r + end_margin;
board_l = 2*half_l;
tub_h   = floor_t + interior_depth;
total_h = tub_h + faceplate_t;

function yc(p, o) = p * o;

inset = side_margin + 1;
screw_pts = concat(
  [ for (sx=[-1,1], sy=[-1,1]) [sx*(board_w/2-inset), sy*(board_l/2-inset)] ],
  [ [board_w/2-inset, y_move], [-(board_w/2-inset), -y_move] ]);

bz_x = disp[0]/2 + buzzer_d/2 + 3;
led_row_w = hp_led_d + led_group_gap + (party_led_n-1)*party_led_pitch + party_led_d;
led0_x = -led_row_w/2 + hp_led_d/2;
function party_x(i) = -led_row_w/2 + hp_led_d + led_group_gap
                      + i*party_led_pitch + party_led_d/2;

/* ───────────────────────── self-checks ──────────────────────────────────── */
assert(board_w <= bed[0] && board_l <= bed[1],
       str("Board ", board_w, "x", board_l, " exceeds bed ", bed));
assert(abs(btn_pitch/strip_grid - round(btn_pitch/strip_grid)) < 1e-3,
       "btn_pitch must be an integer multiple of stripboard grid 2.54");
assert(nfc_floor >= 0.6,
       "NFC recess too deep — thin coin or thicker faceplate");
assert(bz_x - buzzer_d/2 - 1.5 > disp[0]/2, "buzzer overlaps display");
assert(interior_depth >= lipo[2] + clearance + 1, "cavity too shallow for LiPo");
assert(boost_bumpout || interior_depth >= pmod[2] + clearance,
       "power module won't fit");

echo(VARIANT="single-st7789-nfc", board_w=board_w, board_l=board_l,
     total_h=total_h, half_l=half_l, nfc_recess_d=nfc_recess_d,
     nfc_floor=nfc_floor, y_move=y_move, y_nfc=y_nfc);

/* ───────────────────────── helpers ──────────────────────────────────────── */
module rrect(w, l, r) hull() for (sx=[-1,1], sy=[-1,1])
  translate([sx*(w/2-r), sy*(l/2-r)]) circle(r);
module slab(w, l, h, r) linear_extrude(h) rrect(w, l, r);
module led_window(d)
  translate([0,0,-eps])
    cylinder(d=d+2*led_win_clear, h=faceplate_t - led_diffuser_t + eps);
module octagon(d) rotate([0,0,22.5]) circle(d=d, $fn=8);  // flat-topped

module button_centres() {                    // moves only (no switch row)
  for (p=[-1,1], i=[0:3])
    translate([(i-1.5)*btn_pitch, yc(p,y_move)]) children();
}

/* ───────────────────────── bottom tub ───────────────────────────────────── */
boost_drop = pmod[2] + clearance - interior_depth;
boost_c = [ board_w/2 - pmod[0]/2 - side_margin,
           -board_l/2 + pmod[1]/2 + end_margin ];

module bottom_tub() {
  difference() {
    union() {
      slab(board_w, board_l, tub_h, corner_r);
      if (boost_bumpout && boost_drop > 0)
        translate([boost_c[0], boost_c[1], -boost_drop])
          slab(pmod[0]+2*(wall_t+clearance), pmod[1]+2*(wall_t+clearance),
               boost_drop + floor_t, 2);
    }
    translate([0,0,floor_t])
      slab(board_w-2*wall_t, board_l-2*wall_t, tub_h, max(1,corner_r-wall_t));
    if (boost_bumpout && boost_drop > 0)
      translate([boost_c[0], boost_c[1], -boost_drop+floor_t])
        linear_extrude(boost_drop + eps)
          square([pmod[0]+2*clearance, pmod[1]+2*clearance], center=true);
    translate([board_w/2-wall_t-1, -board_l/2+end_margin+pmod[1]/2, floor_t+2])
      cube([wall_t+3, 9, 4], center=true);
    translate([board_w/2-wall_t-1, -board_l/2+end_margin+pmod[1]+8,
               floor_t+slide_sw[2]/2+1])
      cube([wall_t+3, slide_sw[0]+1, slide_sw[2]+1], center=true);
    for (pt = screw_pts) translate([pt[0], pt[1], floor_t])
      cylinder(d=insert_bore_d, h=tub_h);
  }
  for (pt = screw_pts) translate([pt[0], pt[1], floor_t])
    difference() {
      cylinder(d=insert_boss_od, h=interior_depth);
      cylinder(d=insert_bore_d, h=interior_depth+1);
    }
  translate([bz_x,0,floor_t]) difference() {
    cylinder(d=buzzer_d+3, h=3);
    translate([0,0,-eps]) cylinder(d=buzzer_d+1, h=3+2*eps);
  }
}

/* ───────────────────────── top faceplate ────────────────────────────────── */
module top_faceplate() {
  difference() {
    slab(board_w, board_l, faceplate_t, corner_r);
    // central display
    translate([0,0,-eps])
      linear_extrude(faceplate_t - disp_lip_t + eps)
        square([disp[0]+2*clearance, disp[1]+2*clearance], center=true);
    translate([0,0,-eps])
      linear_extrude(faceplate_t + 2*eps)
        square([disp_active[0]+2, disp_active[1]+2], center=true);
    translate([0, -(disp[1]/2), -eps])
      linear_extrude(faceplate_t - disp_lip_t + eps)
        square([disp[0]+2*clearance, disp_conn], center=true);
    // move buttons (4 per player)
    button_centres()
      translate([0,0,-eps]) cylinder(d=btn_hole_d, h=faceplate_t+2*eps);
    // LED windows
    for (p=[-1,1]) translate([0, yc(p,y_led), 0]) {
      translate([led0_x,0,0]) led_window(hp_led_d);
      for (i=[0:party_led_n-1])
        translate([party_x(i),0,0]) led_window(party_led_d);
    }
    // octagonal NFC-coin recess per player (front pocket, leaves nfc_floor;
    // NFC reads through the thin plastic) + a thumb notch to lift the coin
    for (p=[-1,1]) translate([0, yc(p,y_nfc), 0]) {
      translate([0,0,nfc_floor])
        linear_extrude(faceplate_t - nfc_floor + eps) octagon(nfc_recess_d);
      translate([0, p*(nfc_r-1), -eps])              // thumb notch, outward
        cylinder(d=7, h=faceplate_t+2*eps);
    }
    // buzzer grille beside display
    translate([bz_x,0,0]) {
      for (a=[0:45:359]) translate([cos(a)*3.5, sin(a)*3.5, -eps])
        cylinder(d=1.6, h=faceplate_t+2*eps);
      translate([0,0,-eps]) cylinder(d=1.6, h=faceplate_t+2*eps);
    }
    for (pt = screw_pts) translate([pt[0], pt[1], 0]) {
      translate([0,0,-eps]) cylinder(d=screw_d+0.4, h=faceplate_t+2*eps);
      translate([0,0,faceplate_t-cs_depth])
        cylinder(d1=screw_d, d2=screw_head_d, h=cs_depth+eps);
    }
  }
}

/* ───────────────────────── ghosts ───────────────────────────────────────── */
module ghosts() {
  color("teal",0.5)   translate([-board_w/2+side_margin,
                                  -board_l/2+end_margin+8, floor_t])
                         cube([lipo[1], lipo[0], lipo[2]]);
  color("orange",0.6) translate([board_w/2-side_margin-pmod[0],
                                  -board_l/2+end_margin, floor_t]) cube(pmod);
  color("blue",0.4)   translate([-disp[0]/2,-disp[1]/2,tub_h])
                         cube([disp[0],disp[1],1]);
  for (p=[-1,1]) {
    color("red",0.6)  translate([led0_x, yc(p,y_led), tub_h])
                        cylinder(d=hp_led_d, h=1);
    color("green",0.4) translate([-nfc_reader[0]/2, yc(p,y_nfc)-nfc_reader[1]/2,
                                   floor_t]) cube(nfc_reader);   // NFC reader
  }
}

/* ───────────────────────── assemble + section ───────────────────────────── */
module scene() {
  if (part=="tub" || part=="assembly" || part=="internals") bottom_tub();
  if (part=="faceplate" || part=="assembly")
    translate([0,0, tub_h + (part=="assembly" ? 6 : 0)]) top_faceplate();
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
