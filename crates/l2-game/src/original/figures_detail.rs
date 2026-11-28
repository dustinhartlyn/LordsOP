//! Pre-rendered high-overhead medieval soldiers and siege engines in the Age of
//! Empires II / Empire Earth style, with live six-faction AoE2 heraldic livery
//! recoloring, grounded directional shadows, and combat/movement animation.

use std::cell::RefCell;

use l2_sim::{facing::FACING_DELTA, runner::Fighter, Motion, Troop};
use l2_view::{Canvas, detail::SCALE, figures::{Colour, Pose}, scene::FIELD_CLIP};
use tiny_skia::{Color, FillRule, GradientStop, LineCap, Paint, PathBuilder, Pixmap, Point,
    RadialGradient, SpreadMode, Stroke, Transform};

use super::art_pack;

const W: usize = 48;
const H: usize = 48;
const CELL: usize = 128;
const CACHE: usize = 512;
type Rgb = [u8; 3];

#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    troop: Troop,
    team: Colour,
    facing: u8,
    motion: u8,
    step: u8,
}

struct Entry {
    key: Key,
    pic: Pixmap,
}

thread_local! {
    static ATLAS: RefCell<[Option<Entry>; CACHE]> = const { RefCell::new([const { None }; CACHE]) };
}

pub(crate) fn draw_troop(
    canvas: &mut Canvas,
    fighter: &Fighter,
    team: Colour,
    pose: Pose,
    (x, y): (i32, i32),
) {
    let (motion, step) = match fighter.anim {
        Motion::Idle => (0, 0),
        Motion::Walking => (1, (pose.phase / 4) % 6),
        Motion::Attacking => (2, if pose.defending { 0 } else { 1 + (pose.phase / 3) % 4 }),
        Motion::Shooting => (3, (pose.swing / 5).min(4) as u8),
        Motion::Shovelling => (4, (pose.phase / 4) % 3),
        Motion::Dying => (5, (pose.corpse / 4).min(5) as u8),
    };
    let facing = match fighter.anim {
        Motion::Walking | Motion::Dying | Motion::Shovelling => fighter.facing,
        Motion::Idle if fighter.troop == Troop::Knights => fighter.facing,
        _ => fighter.facing_drawn,
    } & 7;
    let key = Key { troop: fighter.troop, team, facing, motion, step };
    draw_cached(canvas, key, x - 8, y - 14);
}

pub(crate) fn draw_siege_engine(
    canvas: &mut Canvas,
    fighter: &Fighter,
    team: Colour,
    pose: Pose,
    (x, y): (i32, i32),
) {
    let (motion, step) = match fighter.anim {
        Motion::Idle => (0, 0),
        Motion::Walking => (1, (pose.phase / 5) % 4),
        Motion::Attacking | Motion::Shooting => (3, (pose.swing / 5).min(4) as u8),
        Motion::Dying => (5, (pose.corpse / 4).min(5) as u8),
        Motion::Shovelling => (0, 0),
    };
    let facing = fighter.facing & 7;
    let key = Key { troop: fighter.troop, team, facing, motion, step };
    draw_cached(canvas, key, x - 8, y - 14);
}

fn draw_cached(canvas: &mut Canvas, key: Key, ox: i32, oy: i32) {
    let slot = (key.troop as usize * 2459
        ^ key.team as usize * 6791
        ^ key.facing as usize * 17
        ^ key.motion as usize * 389
        ^ key.step as usize * 131)
        & (CACHE - 1);
    ATLAS.with_borrow_mut(|atlas| {
        let entry = &mut atlas[slot];
        if let Some(prior) = entry {
            if prior.key != key {
                prior.pic.data_mut().fill(0);
                paint(&mut prior.pic, key);
                prior.key = key;
            }
        } else {
            let mut pic = Pixmap::new((W * SCALE) as u32, (H * SCALE) as u32).unwrap();
            paint(&mut pic, key);
            *entry = Some(Entry { key, pic });
        }
        canvas.blit_detail(entry.as_ref().unwrap().pic.data(), W * SCALE, ox, oy, FIELD_CLIP);
    });
}

// Rich Age of Empires II / Empire Earth heraldic faction liveries:
// 0 Cream/White, 1 Crimson Red, 2 Royal Gold, 3 Dark Slate, 4 Royal Purple, 5 Azure Blue
const LIVERY: [Rgb; 6] = [
    [228, 218, 194],
    [198, 56, 44],
    [224, 174, 52],
    [94, 108, 118],
    [156, 86, 168],
    [58, 122, 196],
];

fn troop_slot(troop: Troop) -> Option<usize> {
    match troop {
        Troop::Peasants => Some(0),
        Troop::Archers => Some(1),
        Troop::Crossbowmen => Some(2),
        Troop::Pikemen => Some(3),
        Troop::Macemen => Some(4),
        Troop::Swordsmen => Some(5),
        Troop::Knights => Some(6),
        _ => None,
    }
}

fn engine_slot(troop: Troop) -> Option<usize> {
    match troop {
        Troop::Catapults => Some(0),
        Troop::SiegeTowers => Some(1),
        Troop::BatteringRams => Some(2),
        Troop::Oil => Some(3),
        _ => None,
    }
}

fn paint(p: &mut Pixmap, k: Key) {
    if let Some(slot) = engine_slot(k.troop) {
        paint_engine(p, k, slot);
        return;
    }
    let Some(slot) = troop_slot(k.troop) else { return };
    let (u, v) = FACING_DELTA[k.facing as usize];
    let (uf, vf) = (u as f32, v as f32);
    // Master sprites face 3/4 right except Knight (which faces 3/4 left) -> flip cleanly by bearing.
    let flip_x = if k.troop == Troop::Knights { uf > 0.0 } else { uf < 0.0 };
    let north_back = vf < -0.25;

    let mounted = k.troop == Troop::Knights;
    let cx = 24.0_f32;
    let foot_y = 39.0_f32;

    // Animation offsets & pose angles
    let (dx, dy, angle_deg, dim) = match k.motion {
        1 => {
            // Walking: brisk rhythmic stride bounce + slight directional lean
            let bob = [0.0, -1.3, -0.5, 0.0, -1.3, -0.5][k.step as usize % 6];
            let sway = [0.0, 0.9, 1.3, 0.0, -0.9, -1.3][k.step as usize % 6];
            (sway * uf * 0.65, bob + sway * vf * 0.35, sway * 3.2, 1.0)
        }
        2 => {
            // Attacking: quick lunge forward along facing vector + dynamic swing tilt
            let lunge = [0.0, -1.2, 2.8, 1.8, 0.4][k.step as usize % 5];
            let tilt = [0.0, -7.0, 11.0, 6.0, 1.0][k.step as usize % 5] * if flip_x { -1.0 } else { 1.0 };
            (lunge * uf, lunge * vf * 0.55 - lunge.max(0.0) * 0.35, tilt, 1.0)
        }
        3 => {
            // Shooting: steady braced draw / crisp recoil
            let recoil = [0.0, -0.6, -1.1, 0.7, 0.2][k.step as usize % 5];
            (recoil * uf, recoil * vf * 0.4, recoil * 3.0, 1.0)
        }
        4 => {
            // Shovelling: forward digging lean
            let dip = 1.2 + k.step as f32 * 0.6;
            (dip * uf * 0.5, dip * 0.45, if flip_x { -12.0 } else { 12.0 }, 1.0)
        }
        5 => {
            // Dying: smooth tip to fallen corpse on the battlefield
            let t = (k.step as f32 / 3.0).min(1.0);
            let dir = if flip_x { -1.0 } else { 1.0 };
            (dir * t * 3.0, t * 2.2, dir * t * 84.0, 1.0 - t * 0.26)
        }
        _ => (0.0, 0.0, 0.0, 1.0),
    };

    // High-overhead grounded contact shadow beneath feet / hooves
    let (srx, sry) = if k.motion == 5 && k.step >= 3 {
        (12.5, 4.2)
    } else if mounted {
        (13.8, 6.6)
    } else {
        (8.6, 4.4)
    };
    shadow(p, (cx + dx + 2.2, foot_y + 0.6), (srx, sry));

    // Scale sprite cleanly to crisp battle-tile play size (higher-overhead compact footprint)
    let target_size = if mounted { 39.5 } else if k.troop == Troop::Pikemen { 38.5 } else { 34.0 };
    let soldiers = art_pack::soldiers();
    let mask = art_pack::soldiers_mask();
    let livery = LIVERY[k.team as usize];
    let shade = if north_back { 0.90 } else { 1.0 } * dim;

    blit_recolored_cell(
        p,
        soldiers,
        mask,
        slot,
        (cx + dx) * SCALE as f32,
        (foot_y + dy) * SCALE as f32,
        target_size * SCALE as f32,
        livery,
        shade,
        flip_x,
        angle_deg.to_radians(),
    );

    // Subtle metallic weapon flash arc at peak melee strike frame
    if k.motion == 2 && k.step == 2 {
        let dir = if flip_x { -1.0 } else { 1.0 };
        let sx = cx + dx + uf * 7.5 + dir * 3.0;
        let sy = foot_y + dy - 16.0 + vf * 4.0;
        let mut arc = PathBuilder::new();
        arc.move_to(sx - dir * 2.0, sy - 6.5);
        arc.quad_to(sx + dir * 7.5, sy - 1.0, sx + dir * 2.5, sy + 5.5);
        if let Some(path) = arc.finish() {
            let mut paint = Paint::default();
            paint.set_color(Color::from_rgba8(244, 240, 214, 195));
            let stroke = Stroke { width: 1.15, line_cap: LineCap::Round, ..Stroke::default() };
            p.stroke_path(&path, &paint, &stroke, Transform::from_scale(SCALE as f32, SCALE as f32), None);
        }
    }
}

fn paint_engine(p: &mut Pixmap, k: Key, slot: usize) {
    let (u, v) = FACING_DELTA[k.facing as usize];
    let (uf, vf) = (u as f32, v as f32);
    let flip_x = uf < 0.0;
    let bob = if k.motion == 1 { [0.0, -0.8, 0.0, -0.8][k.step as usize % 4] } else { 0.0 };
    let recoil = if k.motion == 3 { [0.0, -1.4, 2.0, 0.8, 0.0][k.step as usize % 5] } else { 0.0 };
    let dim = if k.motion == 5 { 0.65 } else { 1.0 };

    let cx = 24.0 + recoil * uf * 0.7;
    let foot_y = 40.0 + bob + recoil * vf * 0.4;
    shadow(p, (cx + 2.5, foot_y - 2.0), (17.5, 9.5));

    let engines = art_pack::siege_engines();
    let dest = 43.0 * SCALE as f32;
    let cell_x0 = slot * CELL;
    let dw = p.width() as i32;
    let dh = p.height() as i32;
    let left = cx * SCALE as f32 - dest * 0.5;
    let top = foot_y * SCALE as f32 - dest;
    let x0 = (left.floor() as i32).max(0);
    let y0 = (top.floor() as i32).max(0);
    let x1 = ((left + dest).ceil() as i32).min(dw);
    let y1 = ((top + dest).ceil() as i32).min(dh);
    let data = p.data_mut();
    let inv = CELL as f32 / dest;

    for py in y0..y1 {
        let sv = ((py as f32 + 0.5) - top) * inv - 0.5;
        let iy0 = (sv.floor() as i32).clamp(0, CELL as i32 - 1) as usize;
        let iy1 = (iy0 + 1).min(CELL - 1);
        let fy = (sv - sv.floor()).clamp(0.0, 1.0);
        let row = py as usize * dw as usize;
        for px in x0..x1 {
            let mut su = ((px as f32 + 0.5) - left) * inv - 0.5;
            if flip_x {
                su = (CELL as f32 - 1.0) - su;
            }
            let ix0 = (su.floor() as i32).clamp(0, CELL as i32 - 1) as usize;
            let ix1 = (ix0 + 1).min(CELL - 1);
            let fx = (su - su.floor()).clamp(0.0, 1.0);

            let p00 = (iy0 * engines.width + cell_x0 + ix0) * 4;
            let p10 = (iy0 * engines.width + cell_x0 + ix1) * 4;
            let p01 = (iy1 * engines.width + cell_x0 + ix0) * 4;
            let p11 = (iy1 * engines.width + cell_x0 + ix1) * 4;
            let bilerp = |ch: usize| {
                let s = &engines.rgba;
                let a = f32::from(s[p00 + ch]) + (f32::from(s[p10 + ch]) - f32::from(s[p00 + ch])) * fx;
                let b = f32::from(s[p01 + ch]) + (f32::from(s[p11 + ch]) - f32::from(s[p01 + ch])) * fx;
                a + (b - a) * fy
            };
            let a = bilerp(3);
            if a < 1.0 {
                continue;
            }
            let af = (a / 255.0).clamp(0.0, 1.0);
            let sr = (bilerp(0) * dim * af).clamp(0.0, 255.0) as u8;
            let sg = (bilerp(1) * dim * af).clamp(0.0, 255.0) as u8;
            let sb = (bilerp(2) * dim * af).clamp(0.0, 255.0) as u8;
            let sa = a.round().clamp(0.0, 255.0) as u8;
            blend_over(&mut data[(row + px as usize) * 4..], [sr, sg, sb, sa]);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn blit_recolored_cell(
    dst: &mut Pixmap,
    atlas: &art_pack::RawImage,
    mask: &art_pack::RawImage,
    slot: usize,
    cx: f32,
    foot_y: f32,
    dest_size: f32,
    livery: Rgb,
    shade: f32,
    flip_x: bool,
    angle_rad: f32,
) {
    let dw = dst.width() as i32;
    let dh = dst.height() as i32;
    let cell_x0 = slot * CELL;
    let inv = CELL as f32 / dest_size;
    let (sin_a, cos_a) = angle_rad.sin_cos();
    let pivot_x = cx;
    let pivot_y = foot_y - dest_size * 0.14;
    let pad = (dest_size * 0.72).ceil() as i32;
    let x0 = (pivot_x as i32 - pad).max(0);
    let x1 = (pivot_x as i32 + pad).min(dw);
    let y0 = (pivot_y as i32 - (dest_size * 1.05) as i32).max(0);
    let y1 = (pivot_y as i32 + (dest_size * 0.45) as i32).min(dh);

    let liv = [f32::from(livery[0]), f32::from(livery[1]), f32::from(livery[2])];
    let data = dst.data_mut();
    let pitch = dw as usize;
    let aw = atlas.width;

    for py in y0..y1 {
        let ry = (py as f32 + 0.5) - pivot_y;
        let row = py as usize * pitch;
        for px in x0..x1 {
            let rx = (px as f32 + 0.5) - pivot_x;
            // Inverse rotate around foot pivot
            let ux = rx * cos_a + ry * sin_a;
            let uy = -rx * sin_a + ry * cos_a;

            let mut su = (ux + dest_size * 0.5) * inv - 0.5;
            let sv = (uy + dest_size * 0.86) * inv - 0.5;
            if su < -0.5 || su >= CELL as f32 - 0.5 || sv < -0.5 || sv >= CELL as f32 - 0.5 {
                continue;
            }
            if flip_x {
                su = (CELL as f32 - 1.0) - su;
            }
            let ix0 = (su.floor() as i32).clamp(0, CELL as i32 - 1) as usize;
            let iy0 = (sv.floor() as i32).clamp(0, CELL as i32 - 1) as usize;
            let ix1 = (ix0 + 1).min(CELL - 1);
            let iy1 = (iy0 + 1).min(CELL - 1);
            let fx = (su - su.floor()).clamp(0.0, 1.0);
            let fy = (sv - sv.floor()).clamp(0.0, 1.0);

            let p00 = (iy0 * aw + cell_x0 + ix0) * 4;
            let p10 = (iy0 * aw + cell_x0 + ix1) * 4;
            let p01 = (iy1 * aw + cell_x0 + ix0) * 4;
            let p11 = (iy1 * aw + cell_x0 + ix1) * 4;

            let sample = |buf: &[u8], ch: usize| {
                let top = f32::from(buf[p00 + ch]) + (f32::from(buf[p10 + ch]) - f32::from(buf[p00 + ch])) * fx;
                let bot = f32::from(buf[p01 + ch]) + (f32::from(buf[p11 + ch]) - f32::from(buf[p01 + ch])) * fx;
                top + (bot - top) * fy
            };
            let a = sample(&atlas.rgba, 3);
            if a < 1.5 {
                continue;
            }
            let mut r = sample(&atlas.rgba, 0);
            let mut g = sample(&atlas.rgba, 1);
            let mut b = sample(&atlas.rgba, 2);

            let mw = sample(&mask.rgba, 0) / 255.0;
            if mw > 0.01 {
                let ml = sample(&mask.rgba, 1) / 178.0;
                r = r * (1.0 - mw) + (liv[0] * ml).clamp(0.0, 255.0) * mw;
                g = g * (1.0 - mw) + (liv[1] * ml).clamp(0.0, 255.0) * mw;
                b = b * (1.0 - mw) + (liv[2] * ml).clamp(0.0, 255.0) * mw;
            }

            let af = (a / 255.0).clamp(0.0, 1.0);
            let sr = (r * shade * af).clamp(0.0, 255.0) as u8;
            let sg = (g * shade * af).clamp(0.0, 255.0) as u8;
            let sb = (b * shade * af).clamp(0.0, 255.0) as u8;
            let sa = a.round().clamp(0.0, 255.0) as u8;
            blend_over(&mut data[(row + px as usize) * 4..], [sr, sg, sb, sa]);
        }
    }
}

#[inline]
fn blend_over(dst: &mut [u8], src: [u8; 4]) {
    if src[3] == 255 {
        dst[..4].copy_from_slice(&src);
    } else if src[3] > 0 {
        let inv = 255 - u16::from(src[3]);
        dst[0] = src[0].saturating_add(((u16::from(dst[0]) * inv + 127) / 255) as u8);
        dst[1] = src[1].saturating_add(((u16::from(dst[1]) * inv + 127) / 255) as u8);
        dst[2] = src[2].saturating_add(((u16::from(dst[2]) * inv + 127) / 255) as u8);
        dst[3] = src[3].saturating_add(((u16::from(dst[3]) * inv + 127) / 255) as u8);
    }
}

fn shadow(p: &mut Pixmap, (x, y): (f32, f32), (rx, ry): (f32, f32)) {
    let Some(path) = PathBuilder::from_circle(0.0, 0.0, 1.0) else { return };
    let origin = Point::zero();
    let pigment = |a| Color::from_rgba8(22, 34, 24, a);
    let paint = Paint {
        shader: RadialGradient::new(
            origin,
            origin,
            1.0,
            vec![
                GradientStop::new(0.0, pigment(142)),
                GradientStop::new(0.55, pigment(82)),
                GradientStop::new(1.0, pigment(0)),
            ],
            SpreadMode::Pad,
            Transform::identity(),
        )
        .unwrap(),
        ..Paint::default()
    };
    let transform = Transform::from_row(rx, 0.0, 0.0, ry, x, y)
        .post_concat(Transform::from_scale(SCALE as f32, SCALE as f32));
    p.fill_path(&path, &paint, FillRule::Winding, transform, None);
}
