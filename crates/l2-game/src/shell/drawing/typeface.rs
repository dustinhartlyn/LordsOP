//! Standalone type uses antialiased outlines instead of enlarged bitmap glyphs.
//! Legacy advances remain the layout budget, including the blank `@` sign.

use ab_glyph::{Font as _, FontRef, PxScale, ScaleFont, point};
use l2_view::{Canvas, detail::SCALE};
use std::sync::LazyLock;

use super::{Face, Pen, font};

const S: f32 = SCALE as f32;
static SANS: LazyLock<FontRef<'static>> = LazyLock::new(|| {
    FontRef::try_from_slice(include_bytes!("IBMPlexSansCondensed-Regular.ttf")).unwrap()
});
static SERIF: LazyLock<FontRef<'static>> = LazyLock::new(|| {
    FontRef::try_from_slice(include_bytes!("IBMPlexSerif-Medium.ttf")).unwrap()
});
// Standalone shares this palette across screens. Asset overrides select the
// indexed renderer at startup, so a mod's colors never get baked into outlines.
static COLORS: LazyLock<l2_formats::Palette> = LazyLock::new(|| {
    l2_formats::Palette::from_bytes(&crate::original::asset(l2_view::campaign::PALETTE).unwrap()).unwrap()
});

impl Pen<'_> {
    pub(super) fn outline_text(
        &self, face: Face, canvas: &mut Canvas, x: i32, y: i32, s: &str, colour: u8,
        centered_in: Option<i32>,
    ) -> Option<i32> {
        if !self.assets.original || !canvas.has_detail() {
            return None;
        }
        let face = match face {
            Face::Heading => &self.assets.heading,
            Face::Body => &self.assets.body,
            Face::Eight => &self.assets.eight,
        };
        face.as_ref()?.draw_outline(canvas, x, y, s, self.style(colour), centered_in)
    }
}

impl font::Font {
    pub(crate) fn draw_outline(
        &self, canvas: &mut Canvas, x: i32, y: i32, s: &str, style: font::Style,
        centered_in: Option<i32>,
    ) -> Option<i32> {
        if !canvas.has_detail() {
            return None;
        }
        let palette = &*COLORS;
        let (outline, height, baseline) = match self.line {
            24 => (&*SERIF, 24.0, 16.0),
            16 => (&*SANS, 16.4, 11.0),
            _ => (&*SANS, 13.8, 9.0),
        };
        let (width, blank) = (self.width(s), font::SPACE_ADVANCE as f32 * S);
        let size = height * S;
        let normal = outline.as_scaled(size);
        let mut previous = None;
        let mut letters = 0.0;
        let mut spaces = 0.0;
        let mut signs = 0;
        for ch in s.chars() {
            if matches!(ch, ' ' | '@') {
                if ch == ' ' { spaces += blank; } else { signs += 1; }
                previous = None;
                continue;
            }
            let glyph = normal.glyph_id(ch);
            letters += normal.h_advance(glyph) + previous.map_or(0.0, |p| normal.kern(p, glyph));
            previous = Some(glyph);
        }
        let fit = if letters > 0.0 {
            ((width as f32 * S - spaces) / letters).clamp(0.5, 1.0)
        } else { 1.0 };
        let scale = PxScale { x: size * fit, y: size };
        let scaled = outline.as_scaled(scale);
        let paper = palette.rgb(style.colour);
        let shadow = style.shadow.map(|(_, down)| palette.rgb(down));
        let center_offset = centered_in.map_or(0.0, |w| {
            (w as f32 * S - spaces - blank * signs as f32 - letters * fit).max(0.0) * 0.5
        });
        let mut px = x as f32 * S + center_offset;
        let py = (y as f32 + baseline) * S;
        let mut previous = None;
        for ch in s.chars() {
            if matches!(ch, ' ' | '@') {
                px += blank;
                previous = None;
                continue;
            }
            let id = scaled.glyph_id(ch);
            px += previous.map_or(0.0, |p| scaled.kern(p, id));
            let colour = if ch.is_uppercase() { style.caps.map_or(paper, |c| palette.rgb(c)) } else { paper };
            if let Some(glyph) = outline.outline_glyph(id.with_scale_and_position(scale, point(px, py))) {
                let bounds = glyph.px_bounds();
                let (x0, y0) = (bounds.min.x as i32, bounds.min.y as i32);
                glyph.draw(|x, y, a| {
                    let (x, y) = (x0 + x as i32, y0 + y as i32);
                    if let Some(shade) = shadow {
                        canvas.detail_sample(x, y + 2, shade, (a * 176.0) as u8);
                    }
                    canvas.detail_sample(x, y, colour, (a * 255.0) as u8);
                });
            }
            px += scaled.h_advance(id);
            previous = Some(id);
        }
        Some(width + font::SPACE_ADVANCE * signs + (center_offset / S).round() as i32)
    }
}
