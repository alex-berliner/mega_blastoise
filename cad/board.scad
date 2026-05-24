// Mega Blastoise — enclosure  ::  VARIANT: nintendo-style
// Based on single-st7789, redesigned the way Nintendo (Yokoi/Iwata) would:
//  - NFC the amiibo way: antenna BEHIND the screen, no slot, nothing to
//    lose; coins are optional sugar — base game still works on buttons.
//  - Screwless play face: fasteners hidden on the BOTTOM.
//  - Rounded, friendly shell: big radius + chamfered top edge.
//  - Molded-key feel: raised bezel ring around every button.
//  - Speaker (sampled cries), not a bare piezo.
//  - Sized for hand-feel, not minimum dimensions.
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
pmod    = [34.0, 22.0, 7.0];
lipo    = [50.0, 34.0, 5.5];
slide_sw   = [8.7, 3.6, 3.5];
spk_d   = 14.0;  spk_h = 5.0;               // small speaker, not a piezo
btn        = 12.0;  btn_pitch = 15.24;
btn_hole_d = 4.5;   key_bezel_d = 11.0;  key_bezel_h = 0.9;
hp_led_d     = 5.0;
party_led_d  = 3.0;  party_led_n = 3;  party_led_pitch = 6.0;
led_group_gap = 5.0;
led_win_clear = 0.6;
nfc_coil_d = 26.0;                          // antenna behind the display
strip_grid = 2.54;
clearance  = 0.4;
wall_t     = 2.0;
floor_t    = 2.0;
faceplate_t   = 3.0;                        // a touch thicker → solid feel
led_diffuser_t = 1.0;
interior_depth = 9.0;
boost_bumpout  = false;
// M2.5 heat-set inserts — chosen for an iteration-heavy prototype that gets
// opened/closed constantly (metal threads survive ~unlimited cycles; plastic
// self-tap threads strip in ~10). Boss wall ≈1.7 mm so the hot insert
// doesn't split it. insert_bore_d: tune to the actual insert on hand.
insert_boss_od = 7.0;  insert_bore_d = 3.6;
screw_d        = 2.7;  screw_head_d  = 5.2;
bed = [220, 220];
corner_r = 6;  shell_cham = 1.6;  $fn = 56; // rounder, chamfered top edge
eps = 0.02;

/* ───────────────────────── derived geometry ─────────────────────────────── */
side_margin = 6;        // more generous than the size-minimised variants
end_margin  = 6;
g_disp = 4;  g_led = 3;  g_move = 3;

led_row_h  = max(hp_led_d, party_led_d);
move_row_w = 3*btn_pitch + btn;
content_w  = max(move_row_w, disp[0]);
board_w    = content_w + 2*side_margin;

y_led  = disp[1]/2 + g_disp + led_row_h/2;
y_move = y_led + led_row_h/2 + g_led + btn/2;
y_sw   = y_move + btn/2 + g_move + btn/2;
half_l = y_sw + btn/2 + end_margin;
board_l = 2*half_l;
tub_h   = floor_t + interior_depth;
total_h = tub_h + faceplate_t;

function yc(p, o) = p * o;

inset = side_margin + 2;
screw_pts = concat(
  [ for (sx=[-1,1], sy=[-1,1]) [sx*(board_w/2-inset), sy*(board_l/2-inset)] ],
  [ [board_w/2-inset, y_move], [-(board_w/2-inset), -y_move] ]);
post_h = interior_depth;                    // faceplate posts span the cavity

spk_x = disp[0]/2 + spk_d/2 + 3;
led_row_w = hp_led_d + led_group_gap + (party_led_n-1)*party_led_pitch + party_led_d;
led0_x = -led_row_w/2 + hp_led_d/2;
function party_x(i) = -led_row_w/2 + hp_led_d + led_group_gap
                      + i*party_led_pitch + party_led_d/2;

/* ───────────────────────── self-checks ──────────────────────────────────── */
assert(board_w <= bed[0] && board_l <= bed[1],
       str("Board ", board_w, "x", board_l, " exceeds bed ", bed));
assert(abs(btn_pitch/strip_grid - round(btn_pitch/strip_grid)) < 1e-3,
       "btn_pitch must be a multiple of stripboard grid 2.54");
assert(spk_x - spk_d/2 - 1.5 > disp[0]/2, "speaker overlaps display");
assert(faceplate_t > led_diffuser_t && faceplate_t > disp_lip_t,
       "faceplate too thin");
assert(shell_cham < faceplate_t && shell_cham < corner_r,
       "shell_cham too large");
assert(boost_bumpout || interior_depth >= pmod[2] + clearance,
       "power module won't fit");

echo(VARIANT="nintendo-style", board_w=board_w, board_l=board_l,
     total_h=total_h, half_l=half_l, faceplate_t=faceplate_t);

/* ───────────────────────── helpers ──────────────────────────────────────── */
module rrect(w, l, r) hull() for (sx=[-1,1], sy=[-1,1])
  translate([sx*(w/2-r), sy*(l/2-r)]) circle(r);
module slab(w, l, h, r) linear_extrude(h) rrect(w, l, r);

// slab with the TOP outer edge chamfered (friendly, hand-soft)
module cham_slab(w, l, h, r, ch) {
  union() {
    slab(w, l, h-ch+0.5, r);                 // base overlaps the cap by 0.5
    translate([0,0,h-ch]) hull() {           // (no coincident face → 1 part)
      linear_extrude(eps) rrect(w, l, r);
      translate([0,0,ch]) linear_extrude(eps)
        rrect(w-2*ch, l-2*ch, max(1, r-ch));
    }
  }
}
module led_window(d)
  translate([0,0,-eps])
    cylinder(d=d+2*led_win_clear, h=faceplate_t - led_diffuser_t + eps);

module button_centres() {
  for (p=[-1,1]) {
    for (i=[0:3])  translate([(i-1.5)*btn_pitch, yc(p,y_move)]) children();
    for (i=[-1:1]) translate([i*btn_pitch,       yc(p,y_sw)])  children();
  }
}

/* ───────────────────────── bottom tub ───────────────────────────────────── */
boost_drop = pmod[2] + clearance - interior_depth;
boost_c = [ board_w/2 - pmod[0]/2 - side_margin,
           -board_l/2 + pmod[1]/2 + end_margin ];

module bottom_tub() {
  difference() {
    union() {
      slab(board_w, board_l, tub_h, corner_r);     // bottom chamfer
      // soften the very bottom edge too
      mirror([0,0,1]) translate([0,0,-eps]) hull() {
        linear_extrude(eps) rrect(board_w, board_l, corner_r);
        translate([0,0,shell_cham]) linear_extrude(eps)
          rrect(board_w-2*shell_cham, board_l-2*shell_cham,
                max(1,corner_r-shell_cham));
      }
    }
    translate([0,0,floor_t])
      slab(board_w-2*wall_t, board_l-2*wall_t, tub_h, max(1,corner_r-wall_t));
    translate([board_w/2-wall_t-1, -board_l/2+end_margin+pmod[1]/2, floor_t+2])
      cube([wall_t+3, 9, 4], center=true);
    translate([board_w/2-wall_t-1, -board_l/2+end_margin+pmod[1]+8,
               floor_t+slide_sw[2]/2+1])
      cube([wall_t+3, slide_sw[0]+1, slide_sw[2]+1], center=true);
    // hidden bottom fasteners: clearance bore + countersink on the underside
    for (pt = screw_pts) translate([pt[0], pt[1], 0]) {
      translate([0,0,-eps]) cylinder(d=screw_d+0.6, h=tub_h+2*eps);
      translate([0,0,-eps]) cylinder(d1=screw_head_d, d2=screw_d, h=2.2);
    }
  }
}

/* ───────────────────────── top faceplate ────────────────────────────────── */
module top_faceplate() {
  difference() {
    union() {
      cham_slab(board_w, board_l, faceplate_t, corner_r, shell_cham);
      // molded-key bezel ring around every button (raised, friendly)
      button_centres() translate([0,0,faceplate_t-0.6])
        cylinder(d=key_bezel_d, h=key_bezel_h+0.6);   // sink 0.6 into plate
      // hidden screw posts hanging from the underside
      for (pt = screw_pts) translate([pt[0], pt[1], -post_h])
        cylinder(d=insert_boss_od, h=post_h+1.2);      // overlap plate solidly
    }
    // central display
    translate([0,0,-eps])
      linear_extrude(faceplate_t - disp_lip_t + eps)
        square([disp[0]+2*clearance, disp[1]+2*clearance], center=true);
    translate([0,0,-eps])
      linear_extrude(faceplate_t + key_bezel_h + 2*eps)
        square([disp_active[0]+2, disp_active[1]+2], center=true);
    translate([0, -(disp[1]/2), -eps])
      linear_extrude(faceplate_t - disp_lip_t + eps)
        square([disp[0]+2*clearance, disp_conn], center=true);
    // button actuator holes (through bezels too)
    button_centres()
      translate([0,0,-eps]) cylinder(d=btn_hole_d,
                                     h=faceplate_t+key_bezel_h+2*eps);
    // LED windows
    for (p=[-1,1]) translate([0, yc(p,y_led), 0]) {
      translate([led0_x,0,0]) led_window(hp_led_d);
      for (i=[0:party_led_n-1])
        translate([party_x(i),0,0]) led_window(party_led_d);
    }
    // embossed amiibo-style tap glyph beside the screen (cosmetic, shallow)
    translate([-spk_x, 0, faceplate_t-0.5]) difference() {
      cylinder(d=11, h=0.5+eps);
      translate([0,0,-eps]) cylinder(d=8, h=0.5+3*eps);
    }
    // speaker grille beside the display (sparse: holes never merge into a
    // ring that would isolate the centre)
    translate([spk_x,0,0]) {
      translate([0,0,-eps]) cylinder(d=1.6, h=faceplate_t+2*eps);
      for (a=[0:60:359]) translate([cos(a)*3, sin(a)*3, -eps])
        cylinder(d=1.6, h=faceplate_t+2*eps);
      for (a=[0:36:359]) translate([cos(a)*6.5, sin(a)*6.5, -eps])
        cylinder(d=1.6, h=faceplate_t+2*eps);
    }
    // insert bores up into the hidden posts (no front-face holes)
    for (pt = screw_pts) translate([pt[0], pt[1], -post_h-eps])
      cylinder(d=insert_bore_d, h=post_h+1);
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
  // NFC antenna coil hidden behind the display (the amiibo way)
  color("gold",0.5)   translate([0,0,tub_h-2]) difference() {
    cylinder(d=nfc_coil_d, h=1); translate([0,0,-eps])
      cylinder(d=nfc_coil_d-5, h=1+2*eps); }
  for (p=[-1,1]) color("red",0.6)
    translate([led0_x, yc(p,y_led), tub_h]) cylinder(d=hp_led_d, h=1);
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
