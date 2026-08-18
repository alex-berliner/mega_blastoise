//! Render every single-screen UI state to PPM files for visual review.
//!
//! The color renderer is `DrawTarget`-generic, so the exact pixels the device
//! and the browser will show can be produced on a host with no hardware and
//! no browser. Run: `cargo run -p mega-blastoise-test --bin screens -- <dir>`

use std::fs;
use std::io::Write;

use mega_blastoise_core::board_event::MoveSlot;
use mega_blastoise_core::display::PartySlotData;
use mega_blastoise_core::display_color as dc;
use mega_blastoise_core::device_view::{DeviceFrame, HalfFrame, Region};

fn mv(name: &str, ty: &str, cat: &str, pow: Option<u32>, acc: Option<u8>, pp: u8, max: u8) -> MoveSlot {
    MoveSlot {
        name: name.into(),
        type_name: ty.into(),
        category: cat.into(),
        power: pow,
        accuracy: acc,
        pp,
        max_pp: max,
    }
}

fn slot(name: &str, hp: u16, max: u16, active: bool, status: Option<&str>) -> PartySlotData {
    PartySlotData {
        name: name.into(),
        active,
        level: 55,
        hp,
        max_hp: max,
        status: status.map(|s| s.to_string()),
        atk: 180,
        def: 200,
        spe: 160,
        spc: 190,
        types: Vec::new(),
        moves: Vec::new(),
        boost_atk: 0,
        boost_def: 0,
        boost_spe: 0,
        boost_spc: 0,
        item: None,
    }
}

fn write_ppm(path: &str, w: u32, h: u32, rgba: &[u8]) {
    let mut f = fs::File::create(path).unwrap();
    write!(f, "P6\n{w} {h}\n255\n").unwrap();
    for px in rgba.chunks(4) {
        f.write_all(&px[..3]).unwrap();
    }
    println!("wrote {path}");
}

fn half(name: &str, dir: &str, draw: impl FnOnce(&mut HalfFrame)) {
    let mut f = HalfFrame::new(dc::HALF_W, dc::HALF_H);
    draw(&mut f);
    // Same last pass the compositor does, so a dump shows what the device
    // shows rather than a spill the device would have covered.
    dc::draw_play_frame_edge(&mut f, 1);
    write_ppm(&format!("{dir}/half_{name}.ppm"), f.w, f.h, &f.to_rgba());
}

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    fs::create_dir_all(&dir).unwrap();

    let blastoise_moves = vec![
        mv("Hydro Pump", "Water", "Special", Some(120), Some(80), 8, 8),
        mv("Ice Beam", "Ice", "Special", Some(95), Some(100), 10, 10),
        mv("Body Slam", "Normal", "Physical", Some(85), Some(100), 15, 15),
        mv("Rest", "Psychic", "Status", None, None, 0, 10),
    ];
    let party = vec![
        slot("Blastoise", 210, 260, true, None),
        slot("Snorlax", 380, 380, false, None),
        slot("Gengar", 90, 260, false, Some("PSN")),
    ];

    let ctx = dc::HalfCtx {
        seat: 1,
        own_name: "Blastoise",
        own_hp: 81,
        own_level: 55,
        foe_name: "Charizard",
        foe_hp: 88,
        foe_level: 53,
        foe_status: Some("PSN"),
        foe_bob: false,
        own_status: Some("PAR"),
        own_hp_numbers: Some((210, 260)),
        cursor: 1,
        foe_locked: true,
        bob: false,
    };

    half("choice", &dir, |f| dc::render_choice(f, &blastoise_moves, &ctx));
    half("party", &dir, |f| dc::render_party(f, &party, &ctx, false));
    half("forced_switch", &dir, |f| dc::render_party(f, &party, &ctx, true));
    half("locked", &dir, |f| dc::render_locked(f, Some("Ice Beam"), &ctx));
    half("playback", &dir, |f| {
        dc::render_playback(f, "Blue's Charizard used Fire Blast! It's not very effective...", &ctx)
    });
    half("lobby_idle", &dir, |f| dc::render_lobby(f, false, false, 1));
    half("lobby_ready", &dir, |f| dc::render_lobby(f, true, false, 1));
    half("result", &dir, |f| dc::render_result(f, "WINNER!", &ctx));
    half("move_info", &dir, |f| {
        dc::render_move_info(
            f,
            &blastoise_moves[1],
            "Has a 10% chance to freeze the target. Ice-type damage, special category.",
            1,
        )
    });
    half("log", &dir, |f| {
        let lines = [
            "Turn 3",
            "Blue's Charizard used Fire Blast!",
            "  92 damage (1.0x, no crit)",
            "White's Blastoise used Hydro Pump!",
            "  It's super effective!",
            "  148 damage (2.0x, no crit)",
            "Blue's Charizard fainted!",
        ];
        dc::render_log(f, &lines, 0, 1)
    });

    // The default view: each seat's own battle, upright to that seat, with
    // the same narration at the bottom of both halves.
    let caption = "Blue's Charizard used Fire Blast! It's not very effective...";
    let mut scene = DeviceFrame::new();
    {
        let mut top = Region::half(&mut scene, false, true);
        let p2 = dc::HalfCtx {
            seat: 2,
            own_name: "Charizard",
            own_hp: 88,
            own_level: 53,
            foe_name: "Blastoise",
            foe_hp: 81,
            foe_level: 55,
            ..Default::default()
        };
        dc::render_playback(&mut top, caption, &p2);
    }
    {
        let mut bottom = Region::half(&mut scene, true, false);
        dc::render_playback(&mut bottom, caption, &ctx);
    }
    mega_blastoise_core::device_view::draw_split_divider(&mut scene);
    write_ppm(&format!("{dir}/device_battle.ppm"), 240, 320, &scene.to_rgba());

    // Composed 240x320 panel, head-to-head: far half rotated 180.
    let mut dev = DeviceFrame::new();
    {
        let mut top = Region::half(&mut dev, false, true);
        let foe_ctx = dc::HalfCtx {
            seat: 2,
            own_name: "Charizard",
            own_hp: 88,
            own_level: 53,
            foe_name: "Blastoise",
            foe_hp: 81,
            foe_level: 55,
            cursor: 0,
            foe_locked: false,
            ..Default::default()
        };
        let charizard_moves = vec![
            mv("Fire Blast", "Fire", "Special", Some(120), Some(85), 5, 5),
            mv("Slash", "Normal", "Physical", Some(70), Some(100), 20, 20),
            mv("Earthquake", "Ground", "Physical", Some(100), Some(100), 10, 10),
            mv("Hyper Beam", "Normal", "Special", Some(150), Some(90), 0, 5),
        ];
        dc::render_choice(&mut top, &charizard_moves, &foe_ctx);
    }
    {
        let mut bottom = Region::half(&mut dev, true, false);
        dc::render_choice(&mut bottom, &blastoise_moves, &ctx);
    }
    mega_blastoise_core::device_view::draw_split_divider(&mut dev);
    write_ppm(&format!("{dir}/device_headtohead.ppm"), 240, 320, &dev.to_rgba());

    // Attack effects, sampled across their window so the motion is reviewable
    // as a strip rather than by watching the browser.
    for (id, label) in [
        ("watergun", "beam"),
        ("razorleaf", "projectile"),
        ("earthquake", "quake"),
        ("swordsdance", "aura"),
        ("bodyslam", "impact"),
        ("thunderbolt", "strike"),
        ("surf", "wave"),
        ("explosion", "nova"),
    ] {
        for (i, t) in [200u32, 500, 750, 950].iter().enumerate() {
            let mut f = DeviceFrame::new();
            let a = mega_blastoise_core::move_anim::anim(id, 1, *t, 1000).unwrap();
            let (own, foe) = (dc::OWN_MON_CENTRE, dc::FOE_MON_CENTRE);
            for (bottom_half, seat) in [(false, 2u8), (true, 1u8)] {
                let mut r = Region::half(&mut f, bottom_half, !bottom_half);
                let hc = if seat == 1 {
                    ctx.clone()
                } else {
                    dc::HalfCtx {
                        seat: 2,
                        own_name: "Charizard",
                        own_hp: 88,
                        own_level: 53,
                        foe_name: "Blastoise",
                        foe_hp: 81,
                        foe_level: 55,
                        ..Default::default()
                    }
                };
                dc::render_playback(&mut r, caption, &hc);
                dc::draw_play_frame_edge(&mut r, seat);
                let (user, target) = if a.attacker == seat { (own, foe) } else { (foe, own) };
                mega_blastoise_core::move_anim::draw(&mut r, &a, user, target);
            }
            mega_blastoise_core::device_view::draw_split_divider(&mut f);
            let flash = mega_blastoise_core::move_anim::flash_amount(&a);
            if flash > 0 {
                mega_blastoise_core::move_anim::white_out(&mut f, flash);
            }
            write_ppm(&format!("{dir}/fx_{label}_{i}.ppm"), 240, 320, &f.to_rgba());
        }
    }

    // Menus are head-to-head like everything else: the same screen in both
    // halves, each upright to the seat reading it.
    let rows = [
        dc::OptionRow { label: "Team size", value: "3 v 3" },
        dc::OptionRow { label: "Text speed", value: "Normal" },
        dc::OptionRow { label: "Sound", value: "On" },
        dc::OptionRow { label: "Tutorial", value: "First game" },
        dc::OptionRow { label: "Turn timer", value: "60 s" },
    ];

    let mut gp_dev = DeviceFrame::new();
    {
        let mut top = Region::half(&mut gp_dev, false, true);
        dc::render_gen_picker(&mut top, 0, dc::HALF_W, dc::HALF_H, 2);
    }
    {
        let mut bottom = Region::half(&mut gp_dev, true, false);
        dc::render_gen_picker(&mut bottom, 0, dc::HALF_W, dc::HALF_H, 1);
    }
    mega_blastoise_core::device_view::draw_split_divider(&mut gp_dev);
    write_ppm(&format!("{dir}/device_gen_picker.ppm"), 240, 320, &gp_dev.to_rgba());

    let mut op_dev = DeviceFrame::new();
    {
        let mut top = Region::half(&mut op_dev, false, true);
        dc::render_options(&mut top, &rows, 2, dc::HALF_W, dc::HALF_H, 2);
    }
    {
        let mut bottom = Region::half(&mut op_dev, true, false);
        dc::render_options(&mut bottom, &rows, 2, dc::HALF_W, dc::HALF_H, 1);
    }
    mega_blastoise_core::device_view::draw_split_divider(&mut op_dev);
    write_ppm(&format!("{dir}/device_options.ppm"), 240, 320, &op_dev.to_rgba());

    // The same halves unrotated, so a single menu layout is reviewable at the
    // size it is actually drawn.
    let mut gp = HalfFrame::new(dc::HALF_W, dc::HALF_H);
    dc::render_gen_picker(&mut gp, 0, dc::HALF_W, dc::HALF_H, 1);
    write_ppm(&format!("{dir}/gen_picker.ppm"), dc::HALF_W, dc::HALF_H, &gp.to_rgba());

    let mut op = HalfFrame::new(dc::HALF_W, dc::HALF_H);
    dc::render_options(&mut op, &rows, 2, dc::HALF_W, dc::HALF_H, 1);
    write_ppm(&format!("{dir}/options.ppm"), dc::HALF_W, dc::HALF_H, &op.to_rgba());
}
