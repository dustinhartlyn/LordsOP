//! Full-color meadow, woods, slopes, rivers and bridges for field battles.

mod landscape;

use std::cell::RefCell;

use l2_sim::terrain::{Battlefield, DIM};
use l2_view::{Canvas, detail::SCALE, scene::*};
use tiny_skia::{Color, FillRule, GradientStop, LinearGradient, Paint, Path, PathBuilder, Pixmap,
    Point, RadialGradient, Rect, SpreadMode, Stroke, Transform};

use super::{art_pack, noise};
use landscape::{DECK_HALF, Kind, Land, paint_surface};

pub(super) use landscape::{meadow_color, water};

const SIZE: usize = TILE as usize * SCALE;
const CACHE_COLS: usize = VIEW_COLS + 5;
const CACHE_ROWS: usize = VIEW_ROWS + 4;

// Position fixes the lighting and relief; the feature stencil fixes the
// local shore and any approaching roads. The bounded scrolling cache
// keeps overlapping cells without a battle lifetime or revision ID.
struct CachedTile {
    key: Option<(usize, usize, Land)>,
    pixels: Option<Pixmap>,
}

thread_local! {
    static TERRAIN: RefCell<[CachedTile; CACHE_COLS * CACHE_ROWS]> =
        const { RefCell::new([const { CachedTile { key: None, pixels: None } }; CACHE_COLS * CACHE_ROWS]) };
}

pub(crate) fn draw_field_detail(canvas: &mut Canvas, field: &Battlefield, cam: Camera) {
    TERRAIN.with_borrow_mut(|cache| {
        for row in 0..VIEW_ROWS {
            for col in 0..VIEW_COLS {
                let (mx, my) = (cam.x + col, cam.y + row);
                if mx >= DIM || my >= DIM {
                    continue;
                }
                let land = Land::at(field, mx, my);
                if land.frame() >= 252 {
                    continue;
                }
                let slot = &mut cache[my % CACHE_ROWS * CACHE_COLS + mx % CACHE_COLS];
                let key = (mx, my, land);
                let tile = slot.pixels.get_or_insert_with(|| Pixmap::new(SIZE as u32, SIZE as u32).unwrap());
                if slot.key != Some(key) {
                    paint_tile(tile, land, mx, my);
                    slot.key = Some(key);
                }
                canvas.blit_detail_opaque(tile.data(), SIZE, ORIGIN_X + col as i32 * TILE,
                    ORIGIN_Y + row as i32 * TILE, FIELD_CLIP);
            }
        }
    });
}

fn scale() -> Transform { Transform::from_scale(SCALE as f32, SCALE as f32) }

fn paint_tile(tile: &mut Pixmap, land: Land, mx: usize, my: usize) {
    let n = land.frame() as usize;
    let origin = ((mx * TILE as usize) as f32, (my * TILE as usize) as f32);
    // Relief, road cuts, and the floodplain are continuous across cells.
    // Only discrete features need another pass above the ground.
    paint_surface(tile, land, origin);
    match land.kind() {
        Kind::Bridge => {
            let column = (n - 0x8c) % 4;
            let start = mx as i32 - column as i32;
            let boards = noise(start, my as i32, 0x32aa_70f4);
            let center = (2 - column as i32) as f32 * TILE as f32;
            bridge(tile, boards, center, land.bridge_ends(), my % 2 == 0);
            return;
        }
        Kind::Water => return,
        Kind::Woodland => {
            let seed = noise(mx as i32, my as i32, 0xf376_9d31);
            let count = 2 + (seed & 1) as i32;
            for k in 0..count {
                let hash = noise(k, 11, seed);
                let x = 5.8 + k as f32 * (25.0 / count as f32) + (hash & 7) as f32 * 0.56;
                let y = 17.0 + ((hash >> 3) % 12) as f32;
                tree(tile, x, y, 3.7 + ((hash >> 7) & 7) as f32 * 0.33);
            }
            return;
        }
        Kind::Slope => return,
        Kind::Meadow => {}
    }
    if matches!(n, 0x20..=0x3f | 0x7c..=0x8b) {
        let seed = noise(mx as i32, my as i32, 0xf376_9d31);
        let x = 13.0 + (seed & 7) as f32 * 0.95;
        let y = 13.0 + ((seed >> 3) & 7) as f32 * 1.22;
        crag(tile, x, y, seed);
        if n >= 0x80 {
            tree(tile, 24.0 - (seed & 3) as f32, y + 7.2, 3.8);
        }
    }
}

fn bridge(tile: &mut Pixmap, boards: u32, center: f32, [north, south]: [bool; 2], bent: bool) {
    const EDGE: f32 = TILE as f32;
    const COURSE: f32 = EDGE / 7.0;
    let mut paint = Paint::default();
    let transform = scale();
    let left = center - DECK_HALF;
    let width = DECK_HALF * 2.0;
    for (side, reach, shade) in [(-1.0, 3.2, 102), (1.0, 21.0, 211)] {
        let edge = center + side * DECK_HALF;
        let outside = edge + side * reach;
        let (start, end) = (edge.min(outside), edge.max(outside));
        if end <= 0.0 || start >= EDGE { continue; }
        paint.shader = LinearGradient::new(Point::from_xy(edge, 0.0), Point::from_xy(outside, 0.0),
            vec![GradientStop::new(0.0, rgba([22, 32, 31, shade])),
                 GradientStop::new(0.47, rgba([22, 32, 31, shade - shade / 3])),
                 GradientStop::new(1.0, rgba([22, 32, 31, 0]))],
            SpreadMode::Pad, Transform::identity()).unwrap();
        tile.fill_rect(Rect::from_xywh(start, 0.0, end - start, EDGE).unwrap(), &paint, transform, None);
    }
    // Side elevation of a pile-supported span. The truss is drawn outside
    // the deck on the lee side, below the rail and above its river shadow.
    let east = left + width;
    if (-13.0..=EDGE + 1.0).contains(&east) {
        paint.set_color_rgba8(64, 47, 32, 255);
        tile.fill_rect(Rect::from_xywh(east - 0.8, 0.0, 4.9, EDGE).unwrap(), &paint, transform, None);
        paint.set_color_rgba8(112, 82, 52, 255);
        tile.fill_rect(Rect::from_xywh(east + 0.5, 0.0, 1.5, EDGE).unwrap(), &paint, transform, None);
        let mut timber = PathBuilder::new();
        for (a, b) in [(0.5, 8.0), (10.0, 17.5), (20.0, 27.5)] {
            timber.move_to(east + 1.24, a);
            timber.line_to(east + 3.48, b);
        }
        stroke(tile, &timber.finish().unwrap(), [152, 111, 66, 158], 0.92);
        if bent && !north && !south {
            // A cutwater throws an elongated shadow and splits surface
            // flow before the dark pile meets the deck's stringer.
            oval(tile, east + 7.0, 16.0, 8.0, 6.1, [17, 29, 32, 165], [17, 29, 32, 0]);
            for (offset, pigment) in [(0.0, [119, 117, 93, 255]), (2.0, [92, 91, 73, 255])] {
                let mut point = PathBuilder::new();
                point.move_to(east, 11.4 + offset);
                point.line_to(east + 4.7, 9.7 + offset);
                point.line_to(east + 7.1, 14.5 + offset);
                point.line_to(east + 6.9, 19.1 + offset);
                point.line_to(east, 20.5 + offset);
                point.close();
                paint.set_color(rgba(pigment));
                tile.fill_path(&point.finish().unwrap(), &paint, FillRule::Winding, transform, None);
            }
            let mut wake = PathBuilder::new();
            wake.move_to(east + 6.4, 9.1);
            wake.quad_to(east + 10.3, 10.2, east + 15.5, 11.2);
            wake.move_to(east + 7.9, 20.6);
            wake.quad_to(east + 11.9, 22.7, east + 19.7, 21.9);
            stroke(tile, &wake.finish().unwrap(), [132, 172, 166, 127], 0.65);
        }
    }
    if left < EDGE && left + width > 0.0 {
        // Fill bridge deck directly from our real weathered oak timber planks texture!
        let planks = art_pack::planks();
        let x0 = (left.max(0.0) * SCALE as f32).floor() as usize;
        let x1 = ((left + width).min(EDGE) * SCALE as f32).ceil() as usize;
        let data = tile.data_mut();
        let offset_y = (boards & 255) as i32;
        for py in 0..SIZE {
            let row = py * SIZE;
            for px in x0..x1.min(SIZE) {
                let lx = px as f32 / SCALE as f32;
                let norm = ((lx - left) / width).clamp(0.0, 1.0);
                // Subtle wheel-rut dust streaks along the deck + sunlit west rail edge
                let rut = (1.0 - ((norm - 0.30).abs() / 0.12).min(1.0))
                    .max(1.0 - ((norm - 0.70).abs() / 0.12).min(1.0));
                let edge_shade = if norm < 0.06 { 1.12 } else if norm > 0.94 { 0.80 } else { 1.0 };
                let s = planks.sample_u8(px as i32 * 2, py as i32 * 2 + offset_y);
                let r = (f32::from(s[0]) * edge_shade + rut * 22.0).clamp(0.0, 255.0) as u8;
                let g = (f32::from(s[1]) * edge_shade + rut * 19.0).clamp(0.0, 255.0) as u8;
                let b = (f32::from(s[2]) * edge_shade + rut * 13.0).clamp(0.0, 255.0) as u8;
                let di = (row + px) * 4;
                data[di..di + 4].copy_from_slice(&[r, g, b, 255]);
            }
        }
        let deck = Rect::from_xywh(left, 0.0, width, EDGE).unwrap();
        let _ = (COURSE, deck);
        for (end, active) in [(0.0, north), (EDGE - 1.9, south)] {
            if !active { continue; }
            paint.set_color_rgba8(91, 78, 59, 255);
            tile.fill_rect(Rect::from_xywh(left, end, width, 1.9).unwrap(), &paint, transform, None);
            paint.set_color_rgba8(169, 150, 117, 255);
            tile.fill_rect(Rect::from_xywh(left, end + 0.35, width, 0.80).unwrap(), &paint, transform, None);
        }
    }
    // The edge timbers and pinned joints connect the planked surface to
    // the truss. They stay clear of the two traversable deck columns.
    for rail in [left, left + width] {
        if !(-2.0..=EDGE + 2.0).contains(&rail) { continue; }
        paint.set_color_rgba8(70, 63, 50, 255);
        tile.fill_rect(Rect::from_xywh(rail - 1.15, 0.0, 2.6, EDGE).unwrap(), &paint, transform, None);
        paint.set_color_rgba8(138, 123, 96, 255);
        tile.fill_rect(Rect::from_xywh(rail - 1.12, 0.0, 1.68, EDGE).unwrap(), &paint, transform, None);
        paint.set_color_rgba8(169, 150, 117, 211);
        tile.fill_rect(Rect::from_xywh(rail - 1.10, 0.0, 0.42, EDGE).unwrap(), &paint, transform, None);
        for y in [6.7, 23.0] {
            paint.set_color_rgba8(75, 72, 62, 255);
            tile.fill_rect(Rect::from_xywh(rail - 0.55, y, 0.78, 0.86).unwrap(), &paint, transform, None);
        }
    }
}

fn rgba(c: [u8; 4]) -> Color { Color::from_rgba8(c[0], c[1], c[2], c[3]) }

fn oval(tile: &mut Pixmap, x: f32, y: f32, rx: f32, ry: f32, inner: [u8; 4], edge: [u8; 4]) {
    let path = PathBuilder::from_circle(0.0, 0.0, 1.0).unwrap();
    let mut paint = Paint::default();
    paint.shader = RadialGradient::new(Point::from_xy(-0.28, -0.35), Point::zero(), 1.0,
        vec![GradientStop::new(0.10, rgba(inner)), GradientStop::new(1.0, rgba(edge))],
        SpreadMode::Pad, Transform::identity()).unwrap();
    let transform = Transform::from_row(rx, 0.0, 0.0, ry, x, y).post_concat(scale());
    tile.fill_path(&path, &paint, FillRule::Winding, transform, None);
}

fn crag(tile: &mut Pixmap, x: f32, y: f32, seed: u32) {
    let w = 5.8 + ((seed >> 6) & 7) as f32 * 0.34;
    let h = 3.4 + ((seed >> 9) & 7) as f32 * 0.30;
    oval(tile, x + 2.0, y + 0.80, w * 1.36, h * 0.77, [43, 52, 35, 148], [43, 52, 35, 0]);
    let skew = ((seed >> 12) & 7) as f32 * 0.20 - 0.70;
    let cast = ((seed >> 15) & 7) as u8;
    let fill = |tile: &mut Pixmap, points: &[(f32, f32)], color: [u8; 4]| {
        let mut path = PathBuilder::new();
        let &(a, b) = &points[0];
        path.move_to(x + (a + skew * b) * w, y + b * h);
        for &(a, b) in &points[1..] { path.line_to(x + (a + skew * b) * w, y + b * h); }
        path.close();
        let mut paint = Paint::default();
        paint.set_color(rgba(color));
        tile.fill_path(&path.finish().unwrap(), &paint, FillRule::Winding, scale(), None);
    };
    fill(tile, &[(-0.98, -0.31), (-0.44, -0.80), (0.27, -0.84), (0.76, -0.57),
        (0.65, -0.21), (1.09, 0.11), (0.96, 0.45), (0.32, 0.70), (-0.49, 0.55), (-1.04, 0.33)],
        [77 + cast, 77 + cast, 61 + cast, 255]);
    fill(tile, &[(-0.98, -0.31), (-0.44, -0.80), (0.27, -0.84), (0.76, -0.57),
        (0.39, -0.25), (-0.09, -0.10), (-0.45, 0.25), (-1.04, 0.33)],
        [139 + cast, 132 + cast, 103 + cast, 255]);
    fill(tile, &[(0.39, -0.25), (0.65, -0.21), (1.09, 0.11), (0.73, 0.28), (0.30, 0.45),
        (-0.28, 0.39), (-0.45, 0.25), (-0.05, 0.17)], [112 + cast, 110 + cast, 83 + cast, 255]);
    let mut strata = PathBuilder::new();
    strata.move_to(x - w * 0.71, y - h * 0.54);
    strata.line_to(x - w * 0.26, y - h * 0.37);
    strata.line_to(x + w * 0.16, y - h * 0.42);
    strata.move_to(x - w * 0.68, y - h * 0.02);
    strata.line_to(x - w * 0.44, y - h * 0.20);
    strata.line_to(x + w * 0.07, y - h * 0.26);
    stroke(tile, &strata.finish().unwrap(), [188, 173, 131, 123], 0.48);
}

fn tree(tile: &mut Pixmap, x: f32, y: f32, r: f32) {
    // Grounded high-overhead circular shadow + pre-rendered AoE2 sunlit oak tree sprite!
    oval(tile, x + 2.0, y + 0.2, r * 1.45, r * 0.72, [26, 44, 30, 158], [26, 44, 30, 0]);
    let sprite = art_pack::prop_oak();
    let dest_h = (r * 3.35 * SCALE as f32).max(16.0);
    let dest_w = dest_h * (sprite.width as f32 / sprite.height as f32);
    let flip = ((x * 13.0 + y * 7.0) as i32 & 1) != 0;
    let tint_shift = (((x * 5.0 + y * 11.0) as i32 & 7) as f32 - 3.5) * 0.018;
    art_pack::stamp_sprite(
        tile,
        sprite,
        x * SCALE as f32,
        (y + 1.2) * SCALE as f32,
        dest_w,
        dest_h,
        [1.0 + tint_shift, 1.02 + tint_shift * 0.6, 0.96],
        0.0,
        flip,
    );
}

fn stroke(tile: &mut Pixmap, path: &Path, colour: [u8;4], width: f32) {
    let mut paint = Paint::default();
    paint.set_color(rgba(colour));
    let stroke = Stroke { width, line_cap: tiny_skia::LineCap::Round, ..Stroke::default() };
    tile.stroke_path(path, &paint, &stroke, scale(), None);
}
