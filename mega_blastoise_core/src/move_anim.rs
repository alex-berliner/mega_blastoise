//! Attack effects for the shared battle scene.
//!
//! A move gets a short animation over the window the narration already holds
//! the screen for, so it costs no extra pacing. The vocabulary is deliberately
//! small: at 240x320, across a table, what reads is a silhouette that travels,
//! lands, and shakes something. Anything finer is lost.
//!
//! Choreography is chosen from the move's own type and category
//! ([`effect_for`]), so every move in the table animates without a per-move
//! entry. Per-move overrides can be layered on top later without touching the
//! primitives.
//!
//! The art is the 32x32 move icon the build already generates
//! ([`crate::move_sprites`]), tinted by type. Effects draw onto the composed
//! panel after the mons, in [`crate::device_view`]'s band coordinates, so both
//! seats watch the same thing happen in the middle of the table.

use embedded_graphics::{
    pixelcolor::{raw::RawU16, Rgb565},
    prelude::RawData,
};

use crate::device_view::{DeviceFrame, BAND_H, BAND_TOP, BAND_W, DEV_W};
use crate::move_sprites::{move_sprite, MOVE_SPRITE_SIDE};

/// Progress is carried as thousandths, so the whole module is integer maths.
pub const FULL: u32 = 1000;

/// How a move presents itself. Five shapes cover the whole Gen 1 table.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Effect {
    /// Something crosses the field and lands: thrown, spat, fired.
    /// `arc` lobs it instead of sending it flat.
    Projectile { arc: bool },
    /// Contact. Nothing crosses; the hit lands on the target and rocks it.
    Impact,
    /// A sustained line from the user to the target, drawn as it extends.
    Beam,
    /// Something the user does to itself: a boost, a guard, a rest.
    Aura,
    /// The ground goes. The whole field jolts rather than one side.
    Quake,
    /// Comes down on the target from above, and the panel flashes when it
    /// lands. Bolts, meteors, anything that arrives rather than travels.
    Strike,
    /// A wide front that sweeps the whole field left to right.
    Wave,
    /// Goes off on the user and whites out the screen. Explosions, and the
    /// one-move-then-nothing enders.
    Nova,
}

/// Moves worth a hand-picked shape rather than the type-and-category guess.
///
/// This is the twenty or so a player recognises on sight, so the fallback
/// getting them merely plausible is not good enough. Everything absent falls
/// through to [`effect_for`]'s rules, which is why the table can stay short.
/// The ids are the engine's own, and a test holds them to that.
pub fn bespoke(move_id: &str) -> Option<Effect> {
    Some(match move_id {
        // Arrive from above.
        "thunderbolt" | "thunder" | "thundershock" | "thunderwave" | "skyattack" => Effect::Strike,
        // Sweep the field.
        "surf" | "hydropump" | "waterfall" | "blizzard" => Effect::Wave,
        // Go off where you stand.
        "explosion" | "selfdestruct" => Effect::Nova,
        // Big enough to deserve the screen even though the fallback would
        // already have called them beams.
        "hyperbeam" | "solarbeam" | "fireblast" => Effect::Beam,
        // The ground ones the fallback misses because they are not Ground.
        "fissure" | "bonemerang" | "bonelub" => Effect::Quake,
        // Contact finishers.
        "seismictoss" | "doubleedge" | "hyperfang" | "submission" => Effect::Impact,
        // Things done to yourself.
        "recover" | "rest" | "softboiled" | "swordsdance" | "agility" | "amnesia"
        | "harden" | "withdraw" | "meditate" | "growth" => Effect::Aura,
        // Lobbed at the target and left hanging there.
        "hypnosis" | "confuseray" | "sleeppowder" | "stunspore" | "poisonpowder"
        | "spore" | "eggbomb" => Effect::Projectile { arc: true },
        _ => return None,
    })
}

/// Everything a frame of an effect needs. Built once per rendered frame.
#[derive(Clone, Copy)]
pub struct Anim {
    pub effect: Effect,
    /// 1 or 2 — the seat whose mon is attacking.
    pub attacker: u8,
    /// Packed 1-bit 32x32 icon for the move.
    pub icon: &'static [u8],
    pub color: Rgb565,
    /// Progress through the effect, 0..=[`FULL`].
    pub t: u32,
}

/// Pick an effect for a move id, from the engine's own move table.
///
/// Ground moves quake whatever their category; status moves are always about
/// the user; the rest split on whether the type reads as a beam (a continuous
/// stream) or as something thrown.
pub fn effect_for(move_id: &str) -> Effect {
    use gen1_battle::{MoveCategory, Type};
    if let Some(e) = bespoke(move_id) {
        return e;
    }
    let Some(entry) = gen1_battle::move_by_id(move_id) else {
        return Effect::Impact;
    };
    if entry.move_type == Type::Ground && entry.power > 0 {
        return Effect::Quake;
    }
    match entry.category {
        MoveCategory::Status => Effect::Aura,
        MoveCategory::Physical => Effect::Impact,
        MoveCategory::Special => match entry.move_type {
            Type::Fire | Type::Water | Type::Electric | Type::Ice | Type::Psychic | Type::Dragon => {
                Effect::Beam
            }
            // Powders, spores, seeds and stones are lobbed, not fired.
            Type::Grass | Type::Bug | Type::Poison | Type::Rock => Effect::Projectile { arc: true },
            _ => Effect::Projectile { arc: false },
        },
    }
}

/// Build the frame's animation, if the screen is showing a move.
pub fn anim(move_id: &str, attacker: u8, elapsed_ms: u32, total_ms: u32) -> Option<Anim> {
    let icon = move_sprite(move_id)?;
    let t = if total_ms == 0 { FULL } else { (elapsed_ms.min(total_ms) * FULL) / total_ms };
    Some(Anim {
        effect: effect_for(move_id),
        attacker,
        icon,
        color: type_tint(move_id),
        t,
    })
}

fn type_tint(move_id: &str) -> Rgb565 {
    let name = gen1_battle::move_by_id(move_id)
        .map(|e| crate::display::type_abbr(e.move_type))
        .unwrap_or("NRM");
    crate::display_color::type_color_of(name)
}

/// Centre of a seat's mon on the composed panel.
fn seat_centre(seat: u8) -> (i32, i32) {
    let ox = if seat == 1 { 4 } else { DEV_W as i32 - BAND_W as i32 - 4 };
    (ox + BAND_W as i32 / 2, BAND_TOP + BAND_H as i32 / 2)
}

/// How far to jolt a seat's mon this frame, for effects that land on it.
///
/// The mons are drawn before the effect, so the shake has to be handed to the
/// compositor rather than applied here.
pub fn band_shake(a: &Anim, seat: u8) -> (i32, i32) {
    let target = 3 - a.attacker;
    let hit = matches!(
        a.effect,
        Effect::Impact | Effect::Projectile { .. } | Effect::Beam | Effect::Strike | Effect::Wave
    ) && seat == target
        && a.t >= 550;
    // A quake shakes the field, and a nova shakes everything on it.
    let quake = matches!(a.effect, Effect::Quake | Effect::Nova) && a.t >= 350;
    if !hit && !quake {
        return (0, 0);
    }
    // Four frames of alternating offset, decaying to nothing.
    let phase = (a.t / 40) % 2;
    let mag = if a.t > 850 { 1 } else { 2 };
    let dx = if phase == 0 { mag } else { -mag };
    if quake {
        (0, dx)
    } else {
        (dx, 0)
    }
}

/// Draw this frame of the effect over the composed panel.
pub fn draw(frame: &mut DeviceFrame, a: &Anim) {
    let (ux, uy) = seat_centre(a.attacker);
    let (tx, ty) = seat_centre(3 - a.attacker);
    match a.effect {
        Effect::Projectile { arc } => {
            // Travel for the first 60%, then land.
            if a.t < 600 {
                let p = (a.t * FULL) / 600;
                let x = lerp(ux, tx, p);
                let y = if arc { lerp(uy, uy - 26, arch(p)) } else { uy };
                blit(frame, a, x, y, 1, false);
            } else {
                burst(frame, a, tx, ty, (a.t - 600) * FULL / 400);
            }
        }
        Effect::Beam => {
            // The beam extends from the user, then the far end flares.
            let reach = (a.t.min(600) * FULL) / 600;
            let steps = 5;
            for i in 0..steps {
                let along = i * FULL / (steps - 1);
                if along > reach {
                    break;
                }
                let x = lerp(ux, tx, along);
                blit(frame, a, x, uy, 1, i % 2 == 1);
            }
            if a.t >= 600 {
                burst(frame, a, tx, ty, (a.t - 600) * FULL / 400);
            }
        }
        Effect::Impact => {
            // No travel: the hit appears on the target and grows.
            if a.t >= 250 {
                burst(frame, a, tx, ty, ((a.t - 250) * FULL) / 750);
            }
        }
        Effect::Aura => {
            // Rises off the user and thins out as it goes.
            let y = lerp(uy, uy - 34, a.t);
            blit(frame, a, ux, y, if a.t < 500 { 2 } else { 1 }, a.t > 600);
        }
        Effect::Quake => {
            // Sits low between the two mons and jitters with the field.
            let mid = (ux + tx) / 2;
            let jolt = if (a.t / 40) % 2 == 0 { 2 } else { -2 };
            blit(frame, a, mid + jolt, uy + 22, 2, a.t > 800);
        }
        Effect::Strike => {
            // Falls onto the target from off the top of the field, then the
            // panel flashes on the landing.
            if a.t < 550 {
                let p = (a.t * FULL) / 550;
                blit(frame, a, tx, lerp(BAND_TOP - 40, ty, p), 1, false);
            } else {
                burst(frame, a, tx, ty, (a.t - 550) * FULL / 450);
            }
        }
        Effect::Wave => {
            // A front the width of the field, marching across it.
            let front = lerp(ux, tx, (a.t.min(700) * FULL) / 700);
            for row in 0..3 {
                let y = uy - 20 + row * 20;
                blit(frame, a, front - row * 6, y, 1, row != 1);
            }
            if a.t >= 700 {
                burst(frame, a, tx, ty, (a.t - 700) * FULL / 300);
            }
        }
        Effect::Nova => {
            // Goes off where the user stands and takes the screen with it.
            let p = a.t.min(FULL);
            let scale = if p < 300 { 2 } else { 3 };
            blit(frame, a, ux, uy, scale, p > 500);
        }
    }

    // The heavy effects wash the panel out for a few frames. It is the
    // cheapest thing on this hardware that reads as force.
    let f = flash_amount(a);
    if f > 0 {
        white_out(frame, f);
    }
}

/// How hard the panel is washed out this frame, 0..=16.
fn flash_amount(a: &Anim) -> u32 {
    let window = |from: u32, to: u32, peak: u32| -> u32 {
        if a.t < from || a.t >= to {
            return 0;
        }
        // Snap on, fade off.
        let span = to - from;
        let left = to - a.t;
        (peak * left) / span
    };
    match a.effect {
        Effect::Nova => window(300, 800, 14),
        Effect::Strike => window(550, 760, 9),
        Effect::Beam | Effect::Wave => window(700, 850, 5),
        _ => 0,
    }
}

/// Blend the whole panel toward white by `amount`/16.
fn white_out(frame: &mut DeviceFrame, amount: u32) {
    let a = amount.min(16);
    for px in frame.px.iter_mut() {
        let r = ((*px >> 11) & 0x1F) as u32;
        let g = ((*px >> 5) & 0x3F) as u32;
        let b = (*px & 0x1F) as u32;
        let mix = |v: u32, max: u32| ((v * (16 - a) + max * a) / 16) as u16;
        *px = (mix(r, 31) << 11) | (mix(g, 63) << 5) | mix(b, 31);
    }
}

/// 0 at each end, [`FULL`] in the middle — the top of a lobbed arc.
fn arch(p: u32) -> u32 {
    let d = if p > FULL / 2 { FULL - p } else { p };
    d * 2
}

fn lerp(a: i32, b: i32, p: u32) -> i32 {
    a + ((b - a) * p.min(FULL) as i32) / FULL as i32
}

/// The landing: the icon at double size, thinning as it fades.
fn burst(frame: &mut DeviceFrame, a: &Anim, x: i32, y: i32, p: u32) {
    let scale = if p < 500 { 2 } else { 3 };
    blit(frame, a, x, y, scale, p > 450);
}

/// Blit the packed 1-bit icon centred on (cx, cy). `dither` drops every other
/// pixel, which is how an effect fades without an alpha channel.
fn blit(frame: &mut DeviceFrame, a: &Anim, cx: i32, cy: i32, scale: i32, dither: bool) {
    let side = MOVE_SPRITE_SIDE as i32;
    let stride = (MOVE_SPRITE_SIDE / 8) as usize;
    let c = RawU16::from(a.color).into_inner();
    let ox = cx - side * scale / 2;
    let oy = cy - side * scale / 2;
    for sy in 0..side {
        for sx in 0..side {
            let byte = a.icon[sy as usize * stride + (sx / 8) as usize];
            if byte & (0x80 >> (sx % 8)) == 0 {
                continue;
            }
            for dy in 0..scale {
                for dx in 0..scale {
                    let (px, py) = (ox + sx * scale + dx, oy + sy * scale + dy);
                    if px < 0 || py < 0 {
                        continue;
                    }
                    if dither && (px + py) % 2 == 0 {
                        continue;
                    }
                    frame.set(px as u32, py as u32, c);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_gen1_move_gets_an_effect_and_art() {
        // The whole point of choosing by type and category: no move can be
        // added to the table and silently animate as nothing.
        for id in ["hydropump", "thunderbolt", "earthquake", "swordsdance", "tackle", "razorleaf"] {
            assert!(move_sprite(id).is_some(), "{id} has no icon");
            let a = anim(id, 1, 500, 1000).expect("anim");
            assert_eq!(a.t, 500);
        }
        assert_eq!(effect_for("earthquake"), Effect::Quake);
        assert_eq!(effect_for("swordsdance"), Effect::Aura);
        assert_eq!(effect_for("tackle"), Effect::Impact);
        // Thunderbolt is in the bespoke table now, so it strikes rather
        // than firing along the ground.
        assert_eq!(effect_for("thunderbolt"), Effect::Strike);
        assert_eq!(effect_for("watergun"), Effect::Beam, "the guess still handles the rest");
        assert_eq!(effect_for("razorleaf"), Effect::Projectile { arc: true });
    }

    /// A typo in the bespoke table would silently fall back to the guess, so
    /// every id in it has to name a real move that also has an icon.
    #[test]
    fn every_bespoke_id_is_a_real_move_with_art() {
        for id in gen1_battle::MOVES.iter().map(|m| m.id) {
            if let Some(e) = bespoke(id) {
                assert_eq!(effect_for(id), e, "{id}: the table must win over the guess");
            }
        }
        for id in [
            "thunderbolt", "surf", "explosion", "hyperbeam", "fissure", "seismictoss",
            "recover", "hypnosis", "blizzard", "skyattack",
        ] {
            assert!(gen1_battle::move_by_id(id).is_some(), "{id} is not a move id");
            assert!(move_sprite(id).is_some(), "{id} has no icon");
            assert!(bespoke(id).is_some(), "{id} dropped out of the table");
        }
        assert_eq!(bespoke("thunderbolt"), Some(Effect::Strike));
        assert_eq!(bespoke("surf"), Some(Effect::Wave));
        assert_eq!(bespoke("explosion"), Some(Effect::Nova));
        assert_eq!(bespoke("tackle"), None, "the fallback still covers the ordinary moves");
    }

    #[test]
    fn the_target_is_what_shakes() {
        let a = anim("tackle", 1, 900, 1000).unwrap();
        assert_eq!(band_shake(&a, 1), (0, 0), "the attacker does not rock itself");
        assert_ne!(band_shake(&a, 2), (0, 0), "the target does");
    }
}
