//! Native-resolution intro backdrop, medieval menu plaques, and heraldic shields.

use std::sync::LazyLock;

use l2_view::{Canvas, Clip, detail::SCALE};
use tiny_skia::{Color, FillRule, GradientStop, LinearGradient, Paint, PathBuilder, Pixmap, Point,
    Rect, SpreadMode, Stroke, Transform};

use super::art_pack::{self, RgbaImage};

static COVER: LazyLock<RgbaImage> =
    LazyLock::new(|| RgbaImage::qoi(include_bytes!("art/gateway.qoi")));
static TITLE: LazyLock<RgbaImage> =
    LazyLock::new(|| RgbaImage::qoi(include_bytes!("art/gateway-title.qoi")));
static BUTTONS: LazyLock<RgbaImage> =
    LazyLock::new(|| RgbaImage::qoi(include_bytes!("art/gateway-buttons.qoi")));
static SHIELDS: LazyLock<RgbaImage> =
    LazyLock::new(|| RgbaImage::qoi(include_bytes!("art/shields.qoi")));

pub(crate) fn draw_gateway(canvas: &mut Canvas) {
    canvas.blit_detail_opaque(&COVER.rgba, COVER.width, 0, 0, Clip::WHOLE);
}

pub(crate) fn draw_gateway_mark(canvas: &mut Canvas) {
    canvas.blit_detail(&TITLE.rgba, TITLE.width, 140, 15, Clip::WHOLE);
}

pub(crate) fn draw_gateway_button(canvas: &mut Canvas, x: i32, y: i32, selected: bool) {
    let width = (BUTTONS.width / SCALE) as i32;
    let height = (BUTTONS.height / (2 * SCALE)) as i32;
    canvas.blit_detail(
        &BUTTONS.rgba,
        BUTTONS.width,
        x,
        y - i32::from(selected) * height,
        Clip::new(x, y, x + width, y + height),
    );
}

pub(crate) fn draw_shield(canvas: &mut Canvas, x: i32, y: i32, index: usize, selected: bool) {
    // The atlas keeps the setup's 88 pixel column spacing and two 65 pixel rows.
    const WIDTH: i32 = 60;
    const HEIGHT: i32 = 65;
    const STEP: i32 = 88;
    canvas.blit_detail(
        &SHIELDS.rgba,
        SHIELDS.width,
        x - index as i32 * STEP,
        y - i32::from(selected) * HEIGHT,
        Clip::new(x, y, x + WIDTH, y + HEIGHT),
    );
}

/// Medieval carved dark-slate and burnished-gold modal panel for native setup menus.
pub(crate) fn draw_menu_frame(canvas: &mut Canvas, x: i32, y: i32, w: i32, h: i32) {
    if !canvas.has_detail() || w <= 4 || h <= 4 {
        return;
    }
    let pw = (w as usize) * SCALE;
    let ph = (h as usize) * SCALE;
    let mut pix = Pixmap::new(pw as u32, ph as u32).unwrap();
    let t = Transform::from_scale(SCALE as f32, SCALE as f32);
    let (fw, fh) = (w as f32, h as f32);

    // Subtle dark medieval slate-oak gradient body
    let body_rect = Rect::from_xywh(1.0, 1.0, fw - 2.0, fh - 2.0).unwrap();
    let paint = Paint {
        shader: LinearGradient::new(
            Point::from_xy(0.0, 0.0),
            Point::from_xy(0.0, fh),
            vec![
                GradientStop::new(0.0, Color::from_rgba8(24, 30, 38, 240)),
                GradientStop::new(0.5, Color::from_rgba8(16, 21, 28, 244)),
                GradientStop::new(1.0, Color::from_rgba8(12, 16, 22, 248)),
            ],
            SpreadMode::Pad,
            Transform::identity(),
        )
        .unwrap(),
        ..Paint::default()
    };
    pix.fill_rect(body_rect, &paint, t, None);

    // Subtle wood/stone grain modulation from our embedded oak planks texture
    let grain = art_pack::planks();
    let data = pix.data_mut();
    for py in 3..ph.saturating_sub(3) {
        let row = py * pw;
        for px in 3..pw.saturating_sub(3) {
            let g = grain.sample_u8(px as i32 / 2, py as i32 / 2);
            let delta = (i32::from(g[0]) - 110) / 20;
            let i = (row + px) * 4;
            for ch in 0..3 {
                data[i + ch] = (i32::from(data[i + ch]) + delta).clamp(0, 255) as u8;
            }
        }
    }

    // Outer forged-iron + burnished-gold double border + corner rivets
    let mut stroke_box = |inset: f32, width: f32, rgba: [u8; 4]| {
        let r = Rect::from_xywh(inset, inset, fw - inset * 2.0, fh - inset * 2.0).unwrap();
        let path = PathBuilder::from_rect(r);
        let mut p = Paint::default();
        p.set_color_rgba8(rgba[0], rgba[1], rgba[2], rgba[3]);
        pix.stroke_path(&path, &p, &Stroke { width, ..Stroke::default() }, t, None);
    };
    stroke_box(0.5, 1.3, [48, 36, 20, 255]);
    stroke_box(1.5, 1.2, [196, 158, 92, 245]);
    stroke_box(3.5, 0.7, [104, 86, 56, 195]);

    let mut rivet_paint = Paint::default();
    rivet_paint.set_color_rgba8(228, 188, 108, 250);
    for (rx, ry) in [(6.5, 6.5), (fw - 6.5, 6.5), (6.5, fh - 6.5), (fw - 6.5, fh - 6.5)] {
        if let Some(c) = PathBuilder::from_circle(rx, ry, 1.6) {
            pix.fill_path(&c, &rivet_paint, FillRule::Winding, t, None);
        }
    }

    canvas.blit_detail(pix.data(), pw, x, y, Clip::new(x, y, x + w, y + h));
}
