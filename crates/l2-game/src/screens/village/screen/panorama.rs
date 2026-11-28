//! Painterly medieval valley panorama for the standalone workforce planner.
//! Built from the embedded Age of Empires II / Empire Earth illustration with
//! live seasonal, weather, and resource-site grading at native 3x density.

use std::cell::RefCell;

use l2_kingdom::tables::{Commodity, Season, Weather};
use l2_view::{Canvas, Clip, detail::SCALE, village as vill};
use tiny_skia::{Color, FillRule, GradientStop, Paint, PathBuilder, Pixmap, Point, RadialGradient,
    SpreadMode, Transform};

use crate::original::art_pack;

const W: i32 = vill::SCENE_W;
const H: i32 = super::SCENE_BOTTOM - super::SCENE_TOP;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Scene {
    season: Season,
    weather: Option<Weather>,
    resources: [bool; 4],
}

thread_local! {
    static PANORAMA: RefCell<Option<(Scene, Pixmap)>> = const { RefCell::new(None) };
}

pub(super) fn draw(
    canvas: &mut Canvas,
    top: i32,
    season: u8,
    weather: Option<Weather>,
    resources: [bool; 4],
) {
    let scene = Scene {
        season: Season::from_index(season).unwrap_or(Season::Spring),
        weather,
        resources,
    };
    PANORAMA.with_borrow_mut(|cache| {
        if cache.as_ref().is_none_or(|(key, _)| *key != scene) {
            *cache = Some((scene, paint(scene)));
        }
        let (_, image) = cache.as_ref().unwrap();
        canvas.blit_detail_opaque(
            image.data(),
            image.width() as usize,
            vill::SCENE_X,
            top,
            Clip::new(vill::SCENE_X, top, vill::SCENE_X + W, top + H),
        );
    });
}

fn paint(scene: Scene) -> Pixmap {
    let src = art_pack::village_panorama();
    let pw = (W as usize) * SCALE;
    let ph = (H as usize) * SCALE;
    let mut pix = Pixmap::new(pw as u32, ph as u32).unwrap();
    let snow = scene.season == Season::Winter || scene.weather == Some(Weather::Frost);
    let drought = !snow && scene.weather == Some(Weather::Drought);
    let storms = scene.weather == Some(Weather::Storms);
    let cloudy = matches!(scene.weather, Some(Weather::Cloudy | Weather::Flooding));

    let out = pix.data_mut();
    for y in 0..ph {
        let sy = (y * src.height / ph).min(src.height - 1);
        let yf = y as f32 / ph as f32;
        for x in 0..pw {
            let sx = (x * src.width / pw).min(src.width - 1);
            let si = (sy * src.width + sx) * 4;
            let (mut r, mut g, mut b) = (
                f32::from(src.rgba[si]),
                f32::from(src.rgba[si + 1]),
                f32::from(src.rgba[si + 2]),
            );
            let luma = 0.299 * r + 0.587 * g + 0.114 * b;
            let greenness = ((g - r.max(b)) / 65.0).clamp(0.0, 1.0);
            let blueness = ((b - r.max(g)) / 65.0).clamp(0.0, 1.0);

            if snow {
                // Crisp frost/winter grading: snowy sunlit turf & pale winter sky
                let frost = ((0.35 + greenness * 0.55 + (luma / 255.0) * 0.25) * (1.0 - blueness * 0.7))
                    .clamp(0.0, 0.82);
                let cool_r = r * 0.84 + luma * 0.16;
                let cool_g = g * 0.88 + luma * 0.12;
                let cool_b = (b * 0.94 + luma * 0.12).min(255.0);
                r = cool_r + (226.0 - cool_r) * frost;
                g = cool_g + (234.0 - cool_g) * frost;
                b = cool_b + (240.0 - cool_b) * frost;
            } else if drought {
                // Parched golden-ochre summer drought
                let dry = greenness * 0.72;
                r = (r + 36.0 * dry).min(255.0);
                g = (g + 4.0 * dry).min(255.0);
                b = (b * (1.0 - 0.30 * dry)).max(0.0);
            } else {
                match scene.season {
                    Season::Spring => {}
                    Season::Summer => {
                        // Slightly richer golden-warm sunlit summer
                        r = (r * 1.05 + 4.0).min(255.0);
                        g = (g * 1.02).min(255.0);
                        b = (b * 0.94).max(0.0);
                    }
                    Season::Autumn => {
                        // Warm amber/russet autumn foliage on greens
                        let fall = greenness * 0.78;
                        r = (r + 44.0 * fall).min(255.0);
                        g = (g - 8.0 * fall).max(0.0);
                        b = (b * (1.0 - 0.28 * fall)).max(0.0);
                    }
                    Season::Winter => unreachable!(),
                }
            }

            if storms {
                let sky_w = (1.0 - yf * 1.6).clamp(0.18, 0.65);
                r = r * (1.0 - sky_w * 0.45) + 74.0 * sky_w * 0.45;
                g = g * (1.0 - sky_w * 0.40) + 88.0 * sky_w * 0.40;
                b = b * (1.0 - sky_w * 0.30) + 106.0 * sky_w * 0.30;
            } else if cloudy {
                r = r * 0.86 + luma * 0.14;
                g = g * 0.88 + luma * 0.12;
                b = (b * 0.90 + luma * 0.12).min(255.0);
            }

            let di = (y * pw + x) * 4;
            out[di] = r.clamp(0.0, 255.0) as u8;
            out[di + 1] = g.clamp(0.0, 255.0) as u8;
            out[di + 2] = b.clamp(0.0, 255.0) as u8;
            out[di + 3] = 255;
        }
    }

    // Subtle soft atmospheric glints for active county resources (never hard circles).
    let t = Transform::from_scale(SCALE as f32, SCALE as f32);
    let mut glow = |cx: f32, cy: f32, rad: f32, rgba: [u8; 4]| {
        if let Some(path) = PathBuilder::from_circle(cx, cy, rad) {
            let center = Point::from_xy(cx, cy);
            let paint = Paint {
                anti_alias: true,
                shader: RadialGradient::new(
                    center,
                    center,
                    rad,
                    vec![
                        GradientStop::new(0.0, Color::from_rgba8(rgba[0], rgba[1], rgba[2], rgba[3])),
                        GradientStop::new(1.0, Color::from_rgba8(rgba[0], rgba[1], rgba[2], 0)),
                    ],
                    SpreadMode::Pad,
                    Transform::identity(),
                )
                .unwrap(),
                ..Paint::default()
            };
            pix.fill_path(&path, &paint, FillRule::Winding, t, None);
        }
    };
    if scene.resources[Commodity::Iron.index()] {
        glow(295.0, 59.0, 9.0, [255, 212, 130, 115]);
    }
    if scene.resources[Commodity::Stone.index()] {
        glow(308.0, 48.0, 8.0, [238, 232, 210, 90]);
    }
    if scene.resources[Commodity::Wood.index()] {
        glow(328.0, 64.0, 8.0, [196, 152, 92, 85]);
    }
    if scene.resources[Commodity::Weapons.index()] {
        glow(322.0, 89.0, 11.0, [255, 168, 64, 145]);
    }
    pix
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panorama_is_opaque_at_native_resolution() {
        let image = paint(Scene { season: Season::Spring, weather: None, resources: [true; 4] });
        assert_eq!(image.width(), W as u32 * SCALE as u32);
        assert_eq!(image.height(), H as u32 * SCALE as u32);
        assert!(image.data().chunks_exact(4).all(|pixel| pixel[3] == 255));
    }

    #[test]
    fn frost_and_resources_change_the_illustration() {
        let base = Scene { season: Season::Summer, weather: None, resources: [false; 4] };
        let summer = paint(base);
        let frost = paint(Scene { weather: Some(Weather::Frost), ..base });
        let mining = paint(Scene { resources: [false, true, false, false], ..base });
        let scale = SCALE as u32;
        assert_ne!(summer.pixel(50 * scale, 77 * scale), frost.pixel(50 * scale, 77 * scale));
        assert_ne!(summer.pixel(295 * scale, 59 * scale), mining.pixel(295 * scale, 59 * scale));
    }
}
