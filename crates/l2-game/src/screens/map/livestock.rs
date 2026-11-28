//! Standalone cattle use world-anchored motion, not the classic herd sheets.
use std::{cell::RefCell, f32::consts::TAU};

use l2_view::{Canvas, Clip, campaign::{self, Viewport, Zoom}, detail::SCALE};
use tiny_skia::{FillRule, LineCap, Paint, PathBuilder, Pixmap, Rect, Stroke, Transform};

pub(super) fn visible<'a>(screen: &'a super::MapScreen, ctx: &'a crate::screen::Ctx)
    -> impl Iterator<Item = ((usize, usize), u8)> + 'a
{
    let map = &ctx.game.kingdom.campaign.map;
    campaign::visible_cells(screen.view, &screen.zoom).filter_map(move |(row, col)| {
        let (x, y) = campaign::cell_to_tile(row, col)?;
        let i = l2_kingdom::map::index(x as u8, y as u8);
        let terrain = map.terrain[i];
        (matches!(terrain, 0x10..=0x12 | 0x14..=0x16)
            && map.flags[i] & l2_kingdom::map::flags::FARMLAND != 0
            && !ctx.game.hides_tile(i)).then_some(((x, y), terrain))
    })
}

const W: i32 = 18;
const H: i32 = 12;
type Point = (f32, f32);
type Color = [u8; 3];
thread_local! {
    static STAMP: RefCell<Pixmap> = RefCell::new(Pixmap::new(W as u32 * SCALE as u32, H as u32 * SCALE as u32).unwrap());
}

pub(super) fn draw(canvas: &mut Canvas, view: Viewport, zoom: &Zoom, tile: (usize, usize), terrain: u8, tick: u32, clip: Clip) {
    if zoom.id == campaign::FAR.id { return; }
    let (count, dead) = match terrain {
        0x14..=0x16 => (terrain - 0x13, false),
        0x10..=0x12 => (terrain - 0x0f, true),
        _ => return,
    };
    let (row, col) = campaign::tile_to_cell(tile.0, tile.1);
    let (x, y) = campaign::cell_to_screen(view, zoom, row, col);
    let seed = (tile.0 + tile.1 * l2_kingdom::map::MAP_DIM) as u32;
    let mut poses = std::array::from_fn::<_, 3, _>(|i| {
        let id = mix(seed.wrapping_mul(7).wrapping_add(i as u32 + 1));
        (id, roam(id, i, count, if dead { 0.0 } else { tick as f32 * 0.016 }))
    });
    let poses = &mut poses[..count as usize];
    poses.sort_unstable_by(|a, b| a.1.at.1.total_cmp(&b.1.at.1));
    STAMP.with_borrow_mut(|stamp| {
        for &(id, pose) in poses.iter() {
            let (cx, cy) = (x as f32 + zoom.half_pitch as f32 * (1.0 + pose.at.0),
                y as f32 + zoom.row_step as f32 * (1.0 + pose.at.1));
            let (sx, sy) = (cx.floor(), cy.floor());
            let scale = SCALE as f32;
            stamp.data_mut().fill(0);
            let transform = Transform::from_row(scale, 0.0, 0.0, scale,
                (W as f32 * 0.5 + cx - sx) * scale, (H as f32 - 3.0 + cy - sy) * scale);
            paint(stamp, transform, id, pose, dead);
            canvas.blit_detail(stamp.data(), stamp.width() as usize, sx as i32 - W / 2, sy as i32 - H + 3, clip);
        }
    });
}

#[derive(Clone, Copy)]
struct Pose {
    at: Point,
    heading: Point,
    gait: f32,
    exertion: f32,
}
// The grazing pause is a continuous slowdown along the same path. It never
// snaps to a target or resets every few frames; headings follow the tangent.
fn roam(id: u32, i: usize, count: u8, seconds: f32) -> Pose {
    let spread = match count { 1 => [0.0, 0.0, 0.0], 2 => [-0.23, 0.23, 0.0], _ => [-0.26, 0.0, 0.26] };
    let n = |shift| ((id >> shift) & 255_u32) as f32 / 255.0;
    let clock = seconds * (0.58 + n(0) * 0.34) + n(8) * TAU;
    let cycle = clock - 0.87 * clock.sin();
    let phase = cycle * (0.51 + n(16) * 0.56) + n(24) * TAU;
    let (a, b) = phase.sin_cos();
    let (p, q) = (phase * 0.53 + n(16) * TAU).sin_cos();
    let (r, s) = (phase * 1.31 + n(0) * TAU).sin_cos();
    let width = if count == 1 { 0.48 } else { 0.23 };
    let x = spread[i] + width * (a * 0.75 + p * 0.25);
    let y = (n(24) - 0.5) * 0.14 + b * 0.24 + r * 0.10;
    let dx = width * (b * 0.75 + q * 0.1325);
    let dy = -a * 0.24 + s * 0.131;
    let norm = dx.hypot(dy).max(0.001);
    Pose { at: (x, y), heading: (dx / norm, dy / norm), gait: cycle * 6.2,
        exertion: ((1.0 - clock.cos()) * 0.66).min(1.0) }
}

fn mix(mut n: u32) -> u32 {
    n ^= n >> 16;
    n = n.wrapping_mul(0x7feb_352d);
    n ^= n >> 15;
    n = n.wrapping_mul(0x846c_a68b);
    n ^ (n >> 16)
}

fn paint(p: &mut Pixmap, t: Transform, id: u32, pose: Pose, dead: bool) {
    let (dx, dy) = pose.heading;
    let f = (dx, dy * 0.62);
    let across = (-dy, dx * 0.62);
    let local = |along: f32, width: f32, h: f32| (f.0 * along + across.0 * width, f.1 * along + across.1 * width - h);
    let coat: Color = [[193, 169, 127], [156, 114, 79], [209, 187, 147], [92, 72, 56]][(id & 3) as usize];
    let shade = |delta: i32| coat.map(|c| (i32::from(c) + delta).clamp(0, 255) as u8);
    let line = |p: &mut Pixmap, a, b, width, ink| {
        let mut path = PathBuilder::new();
        let ((x, y), (u, v)): (Point, Point) = (a, b);
        path.move_to(x, y);
        path.line_to(u, v);
        if let Some(path) = path.finish() {
            p.stroke_path(&path, &pigment(ink, 255), &Stroke { width, line_cap: LineCap::Round, ..Stroke::default() }, t, None);
        }
    };
    oval(p, t, (0.9, 0.62), (4.1, 1.65), [35, 49, 31], 57);
    oval(p, t, (0.52, 0.38), (3.28, 0.92), [41, 43, 31], 75);
    if dead {
        line(p, local(-2.65, 0.0, 0.78), local(2.0, 0.0, 0.82), 2.53, shade(-42));
        oval(p, t, local(3.04, 0.0, 0.21), (1.03, 0.60), shade(-49), 255);
        return;
    }
    let bob = (pose.gait * 2.0).sin() * 0.19 * pose.exertion;
    for (fore, side, offset) in [(-1.95, -0.61, 0.0), (1.68, -0.62, 3.14), (-1.95, 0.61, 3.42), (1.68, 0.62, 0.28)] {
        let (swing, lift) = (pose.gait + offset).sin_cos();
        let foot = local(fore + swing * pose.exertion * 0.59, side, lift.max(0.0) * pose.exertion * 0.49);
        let knee = local(fore + swing * 0.19, side, 0.88 + bob);
        line(p, local(fore, side, 1.98 + bob), knee, 0.61, shade(-44));
        line(p, knee, foot, 0.49, shade(-60));
    }
    let stern = local(-2.87, 0.0, 2.88 + bob);
    let tail = local(-3.31, (pose.gait * 0.41).sin() * 0.71, 1.16 + bob);
    line(p, stern, tail, 0.42, shade(-45));
    let withers = local(1.96, 0.0, 2.75 + bob);
    line(p, stern, withers, 2.85, shade(-33));
    line(p, local(-2.82, -0.21, 3.21 + bob), local(1.89, -0.18, 3.14 + bob), 2.24, coat);
    line(p, local(-2.62, -0.40, 3.75 + bob), local(1.42, -0.43, 3.65 + bob), 0.74, shade(31));
    if id & 8 == 0 {
        oval(p, t, local(-1.88, 0.27, 3.38 + bob), (0.83, 0.55), shade(-61), 220);
    }
    let head_height = 0.84 + pose.exertion * 2.28 + (pose.gait * 0.54).sin() * 0.12;
    let poll = local(3.21, 0.0, head_height);
    line(p, withers, poll, 1.33, shade(-15));
    line(p, local(3.08, -0.83, head_height + 0.12), local(3.05, 0.75, head_height + 0.12), 0.59, shade(19));
    let muzzle = local(4.19, 0.0, head_height - 0.32);
    line(p, poll, muzzle, 1.06, coat);
    oval(p, t, muzzle, (0.56, 0.38), shade(-53), 255);
}

fn pigment([r, g, b]: Color, a: u8) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color_rgba8(r, g, b, a);
    paint
}

fn oval(p: &mut Pixmap, t: Transform, (x, y): Point, (rx, ry): Point, color: Color, alpha: u8) {
    let path = PathBuilder::from_oval(Rect::from_xywh(x - rx, y - ry, 2.0 * rx, 2.0 * ry).unwrap()).unwrap();
    p.fill_path(&path, &pigment(color, alpha), FillRule::Winding, t, None);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn livestock_stays_in_pasture_and_does_not_lock_step() {
        let (one, two) = (mix(137), mix(138));
        let mut different = 0;
        for n in 0..6400 {
            let t = n as f32 * 0.016;
            let a = roam(one, 0, 2, t);
            let b = roam(two, 1, 2, t);
            different += usize::from((a.exertion - b.exertion).abs() > 0.25);
            for p in [a, b] {
                assert!(p.at.0.abs() + p.at.1.abs() < 0.91);
                assert!((p.heading.0.hypot(p.heading.1) - 1.0).abs() < 0.001);
            }
            let next = roam(one, 0, 2, t + 0.016);
            assert!((a.at.0 - next.at.0).hypot(a.at.1 - next.at.1) < 0.02);
        }
        assert!(different > 1500);
    }
}
