use l2_view::campaign::{self, Zoom};
use l2_view::detail::SCALE;
use tiny_skia::{FillRule, LineCap, LineJoin, Paint, Path, PathBuilder, Pixmap, Rect, Stroke, Transform};

use crate::original::art_pack;
use super::{hash, lerp, mix, noise, ramp, shade, Band, Parcel, Pigments, Point, Rgb, Setting, Site, Surface, Works};

fn add_detail(base: Rgb, delta: [f32; 3], strength: f32) -> Rgb {
    [
        (f32::from(base[0]) + delta[0] * strength).clamp(0.0, 255.0) as u8,
        (f32::from(base[1]) + delta[1] * strength).clamp(0.0, 255.0) as u8,
        (f32::from(base[2]) + delta[2] * strength).clamp(0.0, 255.0) as u8,
    ]
}

pub(super) struct Painter<'a> {
    image: &'a mut Pixmap,
    transform: Transform,
    season: usize,
    far: bool,
}

const CREAM: Rgb = [220, 203, 161];
const MASON: Rgb = [166, 151, 116];
const SHADE: Rgb = [104, 102, 87];
const INK: Rgb = [68, 70, 61];
const TIMBER: Rgb = [107, 79, 52];
const CLAY: Rgb = [155, 62, 38];
const SLATE: Rgb = [57, 85, 107];
const SNOW: Rgb = [228, 237, 231];
pub(super) const CROWN_MARGIN: i32 = 10;

const GRASS: [[Rgb; 3]; 4] = [
    [[71, 101, 44], [110, 134, 57], [144, 136, 65]],
    [[74, 102, 41], [121, 139, 53], [154, 138, 63]],
    [[87, 99, 43], [130, 128, 55], [156, 127, 62]],
    [[94, 91, 71], [132, 123, 96], [157, 143, 110]],
];

const LEAVES: [[Rgb; 3]; 4] = [
    [[35, 62, 38], [60, 96, 46], [107, 133, 61]],
    [[33, 62, 34], [65, 104, 44], [120, 141, 62]],
    [[47, 63, 34], [88, 101, 44], [149, 128, 62]],
    [[26, 49, 43], [54, 82, 65], [110, 140, 112]],
];

fn unit(bits: u32) -> f32 {
    (bits & 255) as f32 * (1.0 / 255.0)
}

fn append_polygon(path: &mut PathBuilder, points: &[Point]) {
    path.move_to(points[0].0, points[0].1);
    for &(x, y) in &points[1..] {
        path.line_to(x, y);
    }
    path.close();
}

pub(super) fn terrain(
    setting: Setting, works: Works, colors: &Pigments, (u, v): Point, seed: u32, season: usize, near: bool,
) -> Surface {
    let (x, y) = (u - v, u + v);
    let [grass_dark, grass_mid, grass_dry] = GRASS[season];
    let shoreline = 1.0 - ramp(setting.coast, 0.32, 0.78);
    let reach = noise(u * 0.37 + 8.0, v * 0.31, seed ^ 0x804);
    let knolls = noise(u * 0.72 - v * 0.14, v * 0.62 + u * 0.12, seed ^ 0x474);
    let landform = reach * 0.64 + knolls * 0.36;
    let tended = ramp(setting.tended, 0.018, 0.31);
    let outcrop = (1.0 - 1.0 / (1.0 + setting.rock * 1.45)) * shoreline;
    let (along, across) = (u * 1.38 + v * 0.76, v * 1.54 - u * 0.66);
    let bedding = if outcrop > 0.07 {
        noise(along * 0.46 + 0.72, across * 1.56 + knolls * 0.67, seed ^ 0x881)
    } else {
        0.5
    };
    // Continuous shelves with short scarps above longer aprons of scree.
    let shelf = ramp(bedding, 0.28, 0.57);
    let crown = outcrop * outcrop * (0.33 + landform * 0.71 + shelf * 0.54);
    let elevation = shoreline * landform * (0.19 - tended * 0.09) + crown - setting.coast * 0.10;
    let rock = ramp(outcrop, 0.14, 0.54) * ramp(bedding * 0.59 + outcrop * 0.41, 0.26, 0.67);

    let sod = noise(x * 1.91 + y * 0.42, y * 2.5 + reach, seed ^ 0xf91);
    let dry = ramp(landform * 0.58 + sod * 0.42, 0.48, 0.76) * (1.0 - tended * 0.43);
    let dark = ramp((1.0 - landform) * 0.59 + sod * 0.41, 0.45, 0.73);
    let mut pigment = mix(mix(grass_mid, grass_dark, dark * 0.49), grass_dry, dry * 0.51);
    let grain = if near { noise(x * 7.8 - y * 2.15, y * 8.6 + sod, seed ^ 0x4d3) } else { sod };
    if near {
        // Real painterly AoE2 meadow turf detail sampled in world space.
        let grass_d = art_pack::grass().detail(x * 54.0, y * 54.0);
        let meadow = ramp(grain * 0.54 + sod * 0.46, 0.32, 0.70) * (1.0 - rock);
        pigment = mix(pigment, grass_dark, meadow * 0.28);
        pigment = add_detail(pigment, grass_d, 0.92 * (1.0 - rock) * (1.0 - tended * 0.22));
    }
    if rock > 0.0 {
        let slab = noise(along * 2.67, across * 3.28 + bedding, seed ^ 0x7ef1);
        let warp = noise(along * 0.82, across * 1.37, seed ^ 0xb5a7);
        let face = across * (2.4 + warp * 1.63) + along * 0.72 + slab * 0.43;
        let layer = face.rem_euclid(1.0);
        let exposed_bed = ramp(slab * 0.64 + bedding * 0.36, 0.30, 0.69);
        let bedding_joint = (1.0 - ramp(layer.min(1.0 - layer), 0.029, 0.19)) * exposed_bed;
        let lip = ramp(layer, 0.05, 0.23) * (1.0 - ramp(layer, 0.27, 0.55)) * exposed_bed;
        let base = mix(shade(colors.rock, -36.0), shade(colors.rock, 26.0),
            ramp(slab * 0.55 + bedding * 0.45, 0.28, 0.80));
        let rock_d = art_pack::rock().detail(x * 68.0, y * 68.0);
        let strata = add_detail(shade(base, lip * 34.0 - bedding_joint * 30.0), rock_d, if near { 0.88 } else { 0.45 });
        let exposed = ramp(rock + (slab - 0.5) * 0.22, 0.13, 0.51);
        pigment = mix(pigment, strata, exposed);
    }

    let woodland = setting.wood * shoreline;
    let stand = if woodland > 0.08 {
        ramp(woodland + (sod - 0.50) * 0.37 + (knolls - 0.50) * 0.23, 0.17, 0.80)
    } else {
        0.0
    };
    if season == 3 {
        // Broad drifts collect between wind-scoured banks and under the lee
        // of the woodland. Fine grass only breaks their exposed fringe.
        let drift = noise(x * 0.80 + y * 0.45, y * 1.67 + reach, seed ^ 0x2e7);
        let lee = ramp(stand, 0.05, 0.31) * (1.0 - ramp(stand, 0.36, 0.64));
        let weather = drift * 0.65 + sod * 0.26 + grain * 0.09 + lee * 0.14
            - rock * (0.12 + (1.0 - shelf) * 0.17) - tended * 0.13;
        let cover = ramp(weather, 0.31, 0.48);
        let crust = mix([195, 207, 208], [226, 234, 225], ramp(weather, 0.35, 0.64));
        let wind = if near { noise(x * 1.6 + y * 3.6, y * 19.0, seed ^ 0xed29) - 0.5 } else { 0.0 };
        let lip = ramp(cover, 0.18, 0.55) * (1.0 - ramp(cover, 0.62, 1.0));
        pigment = mix(pigment, shade(crust, wind * 19.0 - lip * 23.0 + (grain - 0.5) * 5.0), cover);
    }
    let litter = mix(LEAVES[season][0], shade(colors.pine, 17.0), ramp(sod, 0.24, 0.73));
    pigment = mix(pigment, litter, stand * if near { 0.76 } else { 0.94 });
    let lift = elevation.max(0.0) * 0.075 + crown * 0.37 / (1.0 + crown * 1.52);
    // Damp, uncultivated hollows carry occasional low hawthorn and scrub,
    // distinct from the dense forest and clear of building lots and traffic.
    let scrub = if near && woodland < 0.12 {
        let hollow = (1.0 - knolls) * 0.58 + sod * 0.42;
        ramp(hollow, 0.47, 0.66) * 0.29 * shoreline * (1.0 - tended) * (1.0 - rock)
    } else { 0.0 };
    let mut surface = Surface { pigment, elevation, canopy: stand.max(scrub), lift };
    if works.foundation.is_some() || works.road.is_some() || works.tracks != 0 {
        grade_works(works, &mut surface, (u, v), (landform, sod, grain), colors.rock, seed, season);
    }
    surface
}

fn texture_fleck(x: f32, y: f32, seed: u32) -> f32 {
    let (ix, iy) = (x.floor(), y.floor());
    let key = hash(ix as i32, iy as i32, seed);
    if key & 3 == 0 {
        return 0.0;
    }
    let a = x - ix - (0.26 + unit(key >> 9) * 0.47);
    let b = y - iy - (0.32 + unit(key >> 17) * 0.36);
    let edge = (a + b * 0.47).abs() * 1.42 + b.abs();
    let profile = 1.0 - ramp(edge, 0.08, 0.32);
    profile * if b < 0.0 { 19.0 } else { -23.0 }
}

fn earth(season: usize) -> [Rgb; 3] {
    if season == 3 { [[94, 87, 70], [134, 114, 88], [163, 142, 108]] }
    else { [[91, 79, 46], [144, 109, 64], [174, 136, 81]] }
}

fn level(surface: &mut Surface, grade: f32, amount: f32) {
    surface.elevation = lerp(surface.elevation, grade, amount);
    surface.lift *= (1.0 - amount) * (1.0 - amount);
}

fn grade_works(
    works: Works, surface: &mut Surface, (u, v): Point, (landform, sod, grain): (f32, f32, f32), stone: Rgb,
    seed: u32, season: usize,
) {
    let (s, t) = (u - (u + 0.5).floor(), v - (v + 0.5).floor());
    let grade = 0.039 + landform * 0.10;
    let [ditch, loam, gravel] = earth(season);
    if let Some(joins) = works.foundation {
        let edge = |side: u32, distance: f32| if joins & (1u8 << side) == 0u8 { distance } else { 1.0 };
        let a = edge(1, 0.5 - s).min(edge(3, s + 0.5));
        let b = edge(2, 0.5 - t).min(edge(0, t + 0.5));
        let inset = a.min(b) - (0.26 - a.max(b)).max(0.0) * 0.58;
        level(surface, grade, ramp(inset, 0.0, 0.17));
        surface.canopy *= 1.0 - ramp(inset, 0.0, 0.15);
        let wear = texture_fleck((u - v) * 19.0, (u + v) * 14.0, seed ^ 0x30b);
        let worn = ramp(inset, 0.054 + grain * 0.13, 0.28 + sod * 0.064);
        let courtyard = shade(mix(loam, gravel, sod * 0.68), (grain - 0.5) * 13.0 + wear);
        surface.pigment = mix(surface.pigment, courtyard, worn * 0.94);
    }

    if works.tracks == 0 && works.road.is_none() {
        return;
    }
    // Bend within each cell but keep the existing edge mouths (also used by
    // bridges, gates and fields). Width and ground material are world-space
    // fields rather than new strokes ending at every mouth.
    let wander = (noise(u * 1.66 + 7.0, v * 1.53, seed ^ 0x46f) - 0.5) * 0.17;
    let bend = (wander * ramp(0.5 - t.abs(), 0.016, 0.21),
        wander * ramp(0.5 - s.abs(), 0.016, 0.21));
    for (links, main) in [(Some(works.tracks).filter(|&bits| bits != 0), false), (works.road, true)] {
        let Some(links) = links else { continue };
        let flex = if main { 1.0 } else { 0.38 };
        let (distance, transverse) =
            lane_distance(links, (s, t), (t + bend.1 * flex, s - bend.0 * flex));
        let width = (if main { 0.100 } else { 0.050 }) + (sod - 0.5) * 0.041 + (grain - 0.5) * 0.017;
        let reach = width + if main { 0.286 } else { 0.186 };
        if distance >= reach {
            continue;
        }
        let rise = ramp(surface.elevation - grade, 0.060, 0.50);
        let outer = width + if main { 0.068 + rise * 0.17 } else { 0.049 + rise * 0.089 };
        let excavation = 1.0 - ramp(distance, width * 0.94, outer);
        level(surface, grade, excavation);
        surface.canopy *= ramp(distance, width + 0.025, outer + 0.048);
        // Mineral soil in the cut follows the physical slope. Lowland lanes
        // have only a soft verge; the same profile becomes a bench on rock.
        let face = ramp(distance, width * 0.78, width + 0.044) * (1.0 - ramp(distance, outer - 0.061, outer));
        let bank = shade(mix(ditch, stone, rise * 0.80), (grain - 0.5) * 39.0);
        surface.pigment = mix(surface.pigment, bank, face * lerp(0.40, 0.97, rise));
        if season == 3 {
            let drift = ramp(distance, width + 0.017, width + 0.068)
                * (1.0 - ramp(distance, width + 0.074, width + 0.136));
            let deposit = ramp(sod * 0.72 + grain * 0.28 + transverse * 0.32, 0.37, 0.70);
            surface.pigment = mix(surface.pigment, shade(SNOW, -13.0 + grain * 12.0), drift * deposit * (1.0 - rise * 0.59));
        }
        let tread = 1.0 - ramp(distance, width * 0.64, width + 0.026);
        let rut = (1.0 - ramp((transverse.abs() - width * 0.50).abs(), 0.010, 0.028))
            * ramp(sod, 0.34, 0.69);
        let earth_d = art_pack::earth().detail((u - v) * 62.0, (u + v) * 62.0);
        let ballast = add_detail(
            shade(mix(loam, gravel, 0.22 + sod * 0.45), (grain - 0.5) * 12.0 - rut * 18.0),
            earth_d,
            0.78,
        );
        surface.pigment = mix(surface.pigment, ballast, tread);
    }
}

fn lane_distance(links: u8, (s, t): Point, (across_u, across_v): Point) -> Point {
    let mut best = (s * s + t * t, 0.0);
    if links & 0b1010 != 0 {
        let side = if s < 0.0 { 0b1000 } else { 0b0010 };
        let end = if links & side == 0 { s * s } else { 0.0 };
        best = (end + across_u * across_u, across_u);
    }
    if links & 0b0101 != 0 {
        let side = if t < 0.0 { 0b0001 } else { 0b0100 };
        let end = if links & side == 0 { t * t } else { 0.0 };
        let distance = end + across_v * across_v;
        if links & 0b1010 == 0 || distance < best.0 {
            best = (distance, across_v);
        }
    }
    (best.0.sqrt(), best.1)
}

// Near woodland is a projection from sampled roots rather than bubbles in
// the DEM with a second row of unrelated vector props around each tile.
pub(super) fn woodland(
    band: &Band, samples: &Band, end_y: i32, surface: &[Surface], pixels: &mut [Rgb],
    seed: u32, season: usize,
) {
    let x0 = band.diagonal;
    let x1 = x0 + (pixels.len() / band.stride) as i32;
    let y0 = band.along;
    let y1 = y0 + band.stride as i32;
    const DX: i32 = 12;
    const DY: i32 = 8;
    // Stable world-space cohorts. Shadows go down before any of the trees;
    // then deeper roots paint first, including crowns clipped by the atlas.
    for shadow in [true, false] {
        for row in (y0 - 7).div_euclid(DY)..=(end_y - 1).div_euclid(DY) {
            for col in (x0 - 18).div_euclid(DX)..=(x1 + CROWN_MARGIN - 1).div_euclid(DX) {
                let key = hash(col, row, seed ^ 0x31b);
                let d = col * DX + (key as i32 & 7) + (row & 1) * 3;
                let k = row * DY + ((key >> 4) as i32 & 7);
                if !(x0 - 18..x1 + CROWN_MARGIN).contains(&d) || !(y0 - 7..end_y).contains(&k) {
                    continue;
                }
                let ground = surface[samples.index(d, k)];
                if ground.canopy <= 0.17 + unit(key >> 9) * 0.66 {
                    continue;
                }
                let root_y = k - (ground.lift * super::NEAR_STEPS as f32).round() as i32;
                let grove = hash(col.div_euclid(4), row.div_euclid(5), seed ^ 0x39bf);
                let sapling = ground.canopy < 0.31;
                let spruce = !sapling && ((key >> 28) & 7) < 3 + ((grove >> 6) & 3);
                let height = if sapling { 5 + ((key >> 20) as i32 & 3) }
                    else { 8 + ((key >> 20) as i32 & 7) + (ground.canopy * 2.6) as i32 };
                let radius = height as f32 * if spruce { 0.36 } else if sapling { 0.73 } else { 0.47 };
                if shadow {
                    // A narrow root contact and a longer cast toward the same
                    // lower-right direction used by buildings.
                    for y in root_y.max(y0)..(root_y + 6).min(y1) {
                        let a = (y - root_y) as f32 / 6.0;
                        let tip = (height as f32 * 0.63 * a + 2.8) as i32;
                        for x in (d - 2).max(x0)..(d + tip).min(x1) {
                            let p = band.index(x, y);
                            let taper = 1.0 - ramp((x - d) as f32, tip as f32 * 0.41, tip as f32);
                            pixels[p] = mix(pixels[p], [32, 42, 31], (1.0 - a) * taper * 0.47);
                        }
                    }
                    continue;
                }
                let lean = (unit(key >> 12) - 0.5) * 3.1;
                let reach = radius.ceil() as i32 + 2;
                let inv_radius = 1.0 / radius;
                let [dark, middle, light] = LEAVES[season];
                let tone = (unit(key >> 24) - 0.5) * 26.0;
                let colors = [shade(dark, tone), shade(middle, tone), shade(light, tone),
                    shade(mix(light, SNOW, 0.74), tone)];
                for y in (root_y - height).max(y0)..=root_y.min(y1 - 1) {
                    let z = (root_y - y) as f32 / height as f32;
                    for x in (d - reach).max(x0)..(d + reach + 1).min(x1) {
                        let a = (x - d) as f32 - lean * z;
                        let crown = if spruce { spruce_face(a * inv_radius, z) } else { broadleaf_face(a * inv_radius, z) };
                        let color = if let Some(face) = crown {
                            colors[if face == 3 && (season != 3 || key & 3 == 0) { 2 } else { face }]
                        } else if z < 0.65 && a.abs() <= 0.63 {
                            if a <= 0.0 { [104, 76, 49] } else { [63, 50, 39] }
                        } else {
                            continue;
                        };
                        pixels[band.index(x, y)] = color;
                    }
                }
            }
        }
    }
}

fn spruce_face(x: f32, z: f32) -> Option<usize> {
    let phase = (1.0 - z) * 4.83 + x * 0.20;
    let branch = phase - phase.floor();
    let spread = (1.06 - z) * (0.42 + branch * 0.91);
    if z < 0.23 || x.abs() > spread {
        None
    } else if branch > 0.81 {
        Some(0)
    } else if x + spread * 0.15 < 0.0 {
        Some(if branch > 0.42 && z > 0.36 { 3 } else { 2 })
    } else {
        Some(1)
    }
}

fn broadleaf_face(x: f32, z: f32) -> Option<usize> {
    let lobes = [(x + 0.10, (z - 0.75) * 2.65), (x + 0.46, (z - 0.55) * 3.05),
        (x - 0.37, (z - 0.54) * 3.18)];
    for (a, b) in lobes {
        if a.abs() * 0.76 + b.abs() + a.abs().max(b.abs()) * 0.34 < 0.64 {
            return Some(if b > 0.12 && a < 0.04 { 3 }
                else if a * 0.61 - b < -0.12 { 2 }
                else if b > -0.18 { 1 } else { 0 });
        }
    }
    None
}

pub(super) fn field(
    parcel: Parcel, surface: &mut Surface, (u, v): Point, season: usize, seed: u32, near: bool,
) {
    let (s, t) = (u - u.round(), v - v.round());
    let (inset, side) = [t + 0.5, 0.5 - s, 0.5 - t, s + 0.5]
        .into_iter().enumerate().filter(|&(i, _)| !parcel.joined[i])
        .fold((1.0_f32, 0), |best, (i, d)| if d < best.0 { (d, i) } else { best });
    let (along, across) = if parcel.turned { (v, u) } else { (u, v) };
    let pasture = (0x0f..=0x16).contains(&parcel.state);
    let fallow = parcel.state == 0;
    let winter = season == 3;
    let damp = noise(u * 3.4 + 8.0, v * 3.4, seed ^ 0x12d9);
    let edge = 0.035 + damp * 0.062;
    let worked = ramp(inset, edge, edge + if pasture { 0.084 } else { 0.052 });
    let bend = (noise(along * 0.94, across * 0.51, seed ^ 0x466f) - 0.5) * 0.11;
    let span = if near { 6.0 } else { 4.0 };
    let phase = (across + bend + surface.elevation * 0.06) * span;
    let row = phase.rem_euclid(1.0);
    let streak = if pasture || fallow {
        noise(along * 4.1, across * 5.7, seed ^ 0x3ead)
    } else {
        noise(along * 6.9, phase.floor() * 0.81, seed ^ 0x3ead)
    };
    let variation = (unit(parcel.seed >> 24) - 0.5) * 10.0;
    let (soil, leaf) = match parcel.state {
        0 => (surface.pigment, if winter { [182, 185, 169] } else { [132, 120, 63] }),
        0x0f..=0x16 if winter => ([108, 121, 101], [163, 179, 155]),
        0x0f..=0x16 => (GRASS[season][0], shade(GRASS[season][1], 14.0)),
        _ if winter => ([113, 91, 76], [179, 162, 139]),
        1 => ([102, 64, 40], [156, 109, 64]),
        _ => ([107, 74, 41], [153 + campaign::field_variant(parcel.state) * 10, 142, 61]),
    };
    let ridge = ramp(row, 0.08, 0.28) * (1.0 - ramp(row, 0.46, 0.81));
    let quality = damp * 0.47 + streak * 0.53;
    let cover = if pasture || fallow { ramp(quality, 0.21, 0.78) }
        else { ridge * lerp(0.63, 1.0, quality) };
    let wear = if pasture || fallow { 0.0 } else { ramp(row, 0.53, 0.83) * (1.0 - ramp(row, 0.91, 1.0)) };
    let mut bed = shade(mix(soil, leaf, cover), (quality - 0.5) * 14.0 + variation - wear * 11.0);
    if near && !winter {
        let (tx, ty) = ((u - v) * 60.0, (u + v) * 60.0);
        if pasture {
            bed = add_detail(bed, art_pack::grass().detail(tx, ty), 0.78);
        } else if parcel.state == 1 {
            bed = add_detail(bed, art_pack::earth().detail(tx, ty), 0.65);
        } else if !fallow {
            bed = add_detail(bed, art_pack::wheat().detail(tx, ty), 0.72);
        }
    }
    let exposure = if fallow { 0.30 } else if winter { 0.46 + damp * 0.45 } else { 0.97 };
    surface.pigment = mix(surface.pigment, bed, worked * exposure);
    surface.canopy *= 1.0 - ramp(inset, 0.02, 0.13);
    if fallow || inset > 0.24 {
        return;
    }

    let lateral = if side % 2 == 0 { s } else { t };
    let entrance = if parcel.gates[side] {
        let mouth = 1.0 - ramp((lateral + inset * 0.18).abs(), 0.036, 0.090);
        let earth = if winter { [152, 134, 108] } else { [153, 120, 76] };
        let threshold = mouth * (1.0 - ramp(inset, 0.11, 0.24));
        surface.pigment = mix(surface.pigment, earth, threshold * 0.89);
        1.0 - mouth
    } else {
        1.0
    };
    let line = if side % 2 == 0 { u } else { v };
    let density = noise(line * 4.2, (u + v) * 0.86, seed ^ 0x9812);
    let bush = noise(line * 10.9, (u + v) * 1.53, seed ^ 0x25ac);
    let bank = ((inset - edge * 0.77 - bush * 0.019) / (0.023 + bush * 0.035)).abs();
    let cover = ramp(density + bush * 0.19, 0.32, 0.63) * entrance;
    let leaves = (1.0 - ramp(bank, 0.29, 1.0)) * cover;
    let base = (1.0 - ramp(bank, 0.36, 1.77)) * cover;
    let [shade_leaf, mid_leaf, lit_leaf] = LEAVES[season];
    surface.pigment = mix(surface.pigment, shade_leaf, base * 0.70);
    surface.pigment = mix(surface.pigment, mix(mid_leaf, lit_leaf, bush * 0.67), leaves);
    surface.lift += leaves * (0.011 + bush * 0.018);
    if pasture && near {
        // Low drystone pasture walls. Oriented courses stop at the same gates
        // as the hedge and the worn entrances; no internal tile dividers.
        let skew = 0.118 + (density - 0.5) * 0.045;
        let p = ((line + (side % 2) as f32 * 0.29) * 9.4).rem_euclid(1.0);
        let normal = inset - skew + (bush - 0.5) * 0.017;
        let footing = (1.0 - ramp((normal + 0.015).abs(), 0.015, 0.050)) * entrance;
        let blocks = (1.0 - ramp(normal.abs(), 0.012, 0.026)) * entrance;
        let joint = ramp(p, 0.03, 0.18) * (1.0 - ramp(p, 0.83, 0.99));
        let limestone = shade(if winter { [160, 170, 164] } else { [142, 134, 102] },
            bush * 21.0 - (1.0 - joint) * 37.0 - ramp(normal, 0.0, 0.014) * 34.0);
        surface.pigment = mix(surface.pigment, [61, 67, 42], footing * 0.72);
        surface.pigment = mix(surface.pigment, limestone, blocks);
        surface.lift += blocks * 0.036;
    }
}

impl<'a> Painter<'a> {
    pub(super) fn new(image: &'a mut Pixmap, zoom: &Zoom, (x, y): (i32, i32), season: u8) -> Self {
        let s = SCALE as f32;
        let (sx, sy) = (s * zoom.half_pitch as f32 / 30.0, s * zoom.row_step as f32 / 15.0);
        let transform = Transform::from_row(sx, 0.0, 0.0, sy, s * x as f32, s * y as f32);
        Self { image, transform, season: campaign::season_slot(season), far: zoom.set != 0 }
    }

    fn fill(&mut self, path: Path, rgb: Rgb, opacity: u8) {
        let mut paint = Paint::default();
        paint.set_color_rgba8(rgb[0], rgb[1], rgb[2], opacity);
        paint.anti_alias = true;
        self.image.fill_path(&path, &paint, FillRule::Winding, self.transform, None);
    }

    fn stroke(&mut self, path: &Path, width: f32, rgb: Rgb, opacity: u8) {
        let mut paint = Paint::default();
        paint.set_color_rgba8(rgb[0], rgb[1], rgb[2], opacity);
        paint.anti_alias = true;
        let stroke = Stroke { width, line_cap: LineCap::Round, line_join: LineJoin::Round, ..Stroke::default() };
        self.image.stroke_path(path, &paint, &stroke, self.transform, None);
    }

    fn poly(&mut self, points: &[Point], rgb: Rgb) {
        self.wash(points, rgb, 255);
    }

    fn wash(&mut self, points: &[Point], rgb: Rgb, opacity: u8) {
        let mut path = PathBuilder::with_capacity(points.len() + 1, points.len());
        append_polygon(&mut path, points);
        if let Some(path) = path.finish() {
            self.fill(path, rgb, opacity);
        }
    }

    fn flow(&mut self, from: Point, control: Point, to: Point, width: f32, rgb: Rgb, opacity: u8) {
        let mut path = PathBuilder::with_capacity(2, 3);
        path.move_to(from.0, from.1);
        path.quad_to(control.0, control.1, to.0, to.1);
        if let Some(path) = path.finish() {
            self.stroke(&path, width, rgb, opacity);
        }
    }

    fn line(&mut self, from: Point, to: Point, width: f32, rgb: Rgb, opacity: u8) {
        let mut path = PathBuilder::with_capacity(2, 2);
        path.move_to(from.0, from.1);
        path.line_to(to.0, to.1);
        if let Some(path) = path.finish() {
            self.stroke(&path, width, rgb, opacity);
        }
    }

    fn mark(&mut self, (x, y): Point, rx: f32, ry: f32, rgb: Rgb, opacity: u8) {
        if let Some(path) = Rect::from_xywh(x - rx, y - ry, rx * 2.0, ry * 2.0)
            .and_then(PathBuilder::from_oval)
        {
            self.fill(path, rgb, opacity);
        }
    }

    fn cast_shadow(&mut self, (x, y): Point, w: f32, h: f32) {
        let (dx, dy) = (h * 0.56, h * 0.24);
        let umbra = [33, 43, 28];
        self.wash(&[(x - w, y), (x + dx - w * 0.55, y + dy - w * 0.35),
            (x + dx + w * 0.73, y + dy + w * 0.07), (x + w * 0.79, y + w * 0.45),
            (x, y + w * 0.52)], umbra, 119);
        self.line((x - w * 0.91, y + 0.30), (x + w * 0.75, y + 0.56), 1.12, umbra, 219);
    }

    fn segments(
        &mut self, pairs: impl IntoIterator<Item = (Point, Point)>, width: f32, color: Rgb, opacity: u8,
    ) {
        let pairs = pairs.into_iter();
        let capacity = pairs.size_hint().0 * 2;
        let mut path = PathBuilder::with_capacity(capacity, capacity);
        for (a, b) in pairs {
            path.move_to(a.0, a.1);
            path.line_to(b.0, b.1);
        }
        if let Some(path) = path.finish() {
            self.stroke(&path, width, color, opacity);
        }
    }

    fn apron_link(&mut self, start: Point, control: Point, end: Point, width: f32) {
        let [_, loam, gravel] = earth(self.season);
        self.flow(start, control, end, width * 1.32, loam, 211);
        self.flow(start, control, end, width, gravel, 191);
    }

    pub(super) fn bridge(&mut self, (x, y): Point) {
        let p = |t: f32, h: f32| (x * t, y * t + h);
        self.wash(&[p(-1.05, 0.0), p(1.05, 0.0), p(1.08, 4.8), p(-0.84, 5.6)], [26, 49, 45], 145);
        self.poly(&[p(-1.05, -0.5), p(1.05, -0.5), p(1.05, 2.55), p(-1.05, 2.55)], [91, 63, 38]);
        self.poly(&[p(-1.05, -2.8), p(1.05, -2.8), p(1.05, 1.3), p(-1.05, 1.3)], [169, 122, 70]);
        self.line(p(-1.05, -2.8), p(1.05, -2.8), 0.80, shade(TIMBER, 42.0), 245);
        if !self.far {
            self.segments((-6..=6).map(|i| {
                let t = i as f32 * 0.145;
                (p(t, -2.5), p(t, 1.1))
            }), 0.54, shade(TIMBER, -20.0), 191);
            self.line(p(-0.90, -4.30), p(0.90, -4.30), 0.72, TIMBER, 245);
            for t in [-0.84, 0.0, 0.84] {
                self.line(p(t, -4.4), p(t, 2.0), 1.24, TIMBER, 255);
                self.mark(p(t, -4.35), 0.87, 0.52, CREAM, 255);
            }
        }
    }

    pub(super) fn boundary(&mut self, (x, y): Point, seed: u32) {
        let t = (if y < 0.0 { 1.0 } else { -1.0 }, if x < 0.0 { -0.5 } else { 0.5 });
        // Survey markers stay legible without suggesting a continuous wall.
        if self.far {
            self.line((x - 8.0 * t.0, y - 8.0 * t.1), (x + 8.0 * t.0, y + 8.0 * t.1), 1.1, CREAM, 124);
        } else {
            for i in 0..2 {
                let n = seed.rotate_left(i * 11);
                let a = (n & 15) as f32 * 0.33 - 8.0 + i as f32 * 11.0;
                let (x, y) = (x + a * t.0, y + a * t.1 + (n >> 7 & 3) as f32 * 0.44);
                self.wash(&[(x - 1.5, y + 0.2), (x - 0.61, y - 0.88), (x + 0.59, y - 0.7),
                    (x + 1.25, y + 0.3), (x + 0.26, y + 0.68)], SHADE, 168);
                self.line((x - 0.86, y - 0.56), (x + 0.50, y - 0.74), 0.50, CREAM, 162);
            }
        }
    }

    fn tree(&mut self, (x, y): Point, h: f32, _lean: f32) {
        self.cast_shadow((x, y), h * 0.24, h * 0.75);
        let sprite = art_pack::prop_oak();
        let px = self.transform.sx * x + self.transform.tx;
        let py = self.transform.sy * y + self.transform.ty;
        let dest_h = (h * 1.48 * self.transform.sy).max(6.0);
        let dest_w = dest_h * (sprite.width as f32 / sprite.height as f32);
        let tint = match self.season {
            0 => [1.02, 1.04, 0.96],
            1 => [1.04, 1.01, 0.92],
            2 => [1.22, 0.92, 0.72],
            _ => [0.88, 0.96, 1.02],
        };
        let snow = if self.season == 3 { 0.75 } else { 0.0 };
        art_pack::stamp_sprite(self.image, sprite, px, py + self.transform.sy * 1.2, dest_w, dest_h, tint, snow, false);
    }

    pub(super) fn town(&mut self, part: usize, tier: u8, seed: u32) {
        let offset = ((seed >> 11 & 7) as f32 - 3.5) * 0.42;
        let w = 9.4 + f32::from(tier) * 0.8;
        if part == 0 {
            self.apron((4.2, 4.8), w, (0.0, 0.2));
            if tier > 0 {
                self.tower((-5.1, -2.0), 4.2, 16.0 + f32::from(tier) * 3.7, false);
            }
            self.house((4.2, 4.8), w, 8.5 + f32::from(tier) * 0.9, SLATE);
            if tier == 2 {
                self.house((-10.2, 5.5), 4.8, 5.7, CLAY);
            }
            return;
        }
        let (back, front, roof) = match part {
            1 => ((-12.5 + offset, -1.8), (1.8, 3.3), CLAY),
            2 => ((-7.6, -3.2), (10.4 + offset, 0.7), TIMBER),
            _ => ((-10.1 + offset, -2.9), (7.7, 4.2), SLATE),
        };
        self.apron(back, 5.6, (0.0, 1.6));
        self.apron(front, 7.1, (0.0, 1.6));
        let ochre = shade(CLAY, 8.0 + (seed >> 5 & 7) as f32);
        self.house(back, 5.4 + offset * 0.4, 5.7 + part as f32 * 0.36, ochre);
        self.house(front, 7.0, 6.5, roof);
        if tier > 0 {
            self.house((-3.0, 8.0), 4.7, 5.2, TIMBER);
        } else if part == 3 {
            let (x, y) = (-4.9, 6.8);
            self.cast_shadow((x, y + 0.26), 2.4, 2.9);
            self.mark((x, y), 2.9, 1.60, SHADE, 255);
            self.mark((x, y - 0.81), 2.7, 1.42, MASON, 255);
            self.mark((x + 0.16, y - 0.69), 1.68, 0.89, [43, 76, 77], 255);
            self.line((x - 2.1, y - 1.32), (x - 0.69, y - 1.74), 0.48, CREAM, 224);
        }
    }

    fn apron(&mut self, (x, y): Point, w: f32, lane: Point) {
        let [edge, loam, worn] = earth(self.season);
        let landing = (x - w * 0.10, y + w * 0.47);
        let corner = (lerp(landing.0, lane.0, 0.62), lerp(landing.1, lane.1, 0.37));
        // Wear follows the foundation and fans out where traffic meets the lane.
        let yard = [(x - w * 1.01, y + w * 0.20), (x - w * 0.33, y - w * 0.42),
            (x + w * 0.78, y - w * 0.15), (x + w * 0.97, y + w * 0.25),
            (x + w * 0.64, y + w * 0.45), landing, (x - w * 0.77, y + w * 0.37)];
        self.wash(&yard, edge, 131);
        self.wash(&[yard[0], (x - w * 0.24, y - w * 0.23), yard[3],
            (x + w * 0.52, y + w * 0.39), landing, (x - w * 0.70, y + w * 0.28)], loam, 206);
        self.apron_link(landing, corner, lane, 2.2);
        if !self.far {
            let (cx, cy) = landing;
            // End of a deliberately worn footpath, not a full-color decal
            // across the lot. The stone chips stay inside the packed apron.
            self.line((cx - w * 0.52, cy - w * 0.13), landing, 0.79, worn, 188);
            for i in 0..11 {
                let key = hash((cx * 4.0) as i32 + i, (cy * 4.0) as i32, 0x4fd2);
                let sx = x + (unit(key >> 5) - 0.5) * w * 1.29;
                let sy = y + (0.22 + unit(key >> 13) * 0.16) * w;
                let tint = if key & 1 == 0 { shade(worn, 17.0) } else { edge };
                self.line((sx, sy), (sx + 0.38 + unit(key >> 22) * 0.52, sy - 0.28), 0.39, tint, 164);
            }
        }
    }

    fn house(&mut self, (x, y): Point, w: f32, h: f32, roof: Rgb) {
        let wooden = roof == TIMBER;
        let turned = roof == SLATE || wooden;
        let (long, short) = if w > 8.0 { (0.69, 0.31) } else { (0.61, 0.39) };
        let p = |s: f32, t: f32, z: f32| {
            let (a, b) = if turned { (t * short, s * long) } else { (s * long, t * short) };
            (x + (a - b) * w, y + (a + b) * w * 0.49 - z)
        };
        let ridge = h + w * 0.61;
        let frost = self.season == 3;
        let (lit, dark) = if wooden { ([167, 127, 80], [102, 73, 50]) }
            else { ([211, 173, 120], [142, 113, 78]) };
        let (facing, gable) = if turned { (dark, lit) } else { (lit, dark) };
        let projected = |(x, y): Point| (x + ridge * 0.54, y + ridge * 0.24);
        let contact = [42, 43, 24];
        self.wash(&[p(-1.0, -1.0, 0.0), projected(p(0.19, -1.0, 0.0)),
            projected(p(1.0, 0.27, 0.0)), projected(p(1.0, 1.0, 0.0)),
            p(1.0, 1.0, 0.0), p(-1.0, 1.0, 0.0)], contact, 117);
        self.line(p(-1.01, 1.01, -0.37), p(1.02, 1.03, -0.34), 1.12, contact, 237);
        self.line(p(1.02, 1.02, -0.36), p(1.02, -1.0, -0.24), 0.88, contact, 227);
        self.poly(&[p(-1.0, 1.0, 0.0), p(-1.0, 1.0, h), p(1.0, 1.0, h), p(1.0, 1.0, 0.0)], facing);
        self.poly(&[p(1.0, -1.0, 0.0), p(1.0, -1.0, h), p(1.0, 0.0, ridge),
            p(1.0, 1.0, h), p(1.0, 1.0, 0.0)], gable);
        let base = h * 0.24;
        self.stone_face(|t, z| p(t * 1.99 - 0.99, 1.02, z), shade(facing, -31.0), w, base);
        self.stone_face(|t, z| p(1.01, t * 1.99 - 0.99, z), shade(gable, -31.0), w * 0.6, base);

        let oak = shade(TIMBER, if turned { -37.0 } else { -22.0 });
        if !self.far {
            self.line(p(-0.99, 1.02, h * 0.88), p(0.99, 1.02, h * 0.88), 0.82, oak, 234);
            self.segments([-0.94_f32, 0.17, 0.94].into_iter().map(|t| {
                (p(t, 1.02, base), p(t, 1.02, h * 0.87))
            }), 0.63, oak, 231);
            self.line(p(0.23, 1.02, base), p(0.91, 1.02, h * 0.86), 0.58, oak, 214);
            self.line(p(1.02, -0.93, h * 0.88), p(1.02, 0.93, h * 0.88), 0.75, shade(gable, -56.0), 242);
            self.line(p(1.02, 0.0, h * 0.87), p(1.02, 0.0, ridge - 0.82), 0.77, shade(gable, -54.0), 240);
        }
        let opening = [49, 36, 23];
        self.poly(&[p(-0.61, 1.04, 0.11), p(-0.61, 1.04, h * 0.64), p(-0.23, 1.04, h * 0.64),
            p(-0.23, 1.04, 0.11)], opening);
        self.wash(&[p(-0.71, 1.06, 0.0), p(-0.15, 1.06, 0.0),
            p(-0.12, 1.39, 0.0), p(-0.72, 1.39, 0.0)], shade(facing, -26.0), 239);
        self.line(p(-0.70, 1.11, 0.0), p(-0.14, 1.11, 0.0), 0.70, shade(facing, 3.0), 244);
        self.poly(&[p(1.03, -0.68, h * 0.41), p(1.03, -0.68, h * 0.70),
            p(1.03, -0.36, h * 0.70), p(1.03, -0.36, h * 0.41)], opening);
        if !self.far {
            self.line(p(-0.48, 1.04, 0.39), p(-0.48, 1.04, h * 0.58), 0.41, TIMBER, 192);
            self.line(p(0.53, 1.03, h * 0.48), p(0.53, 1.03, h * 0.70), 1.02, opening, 255);
            self.line(p(0.37, 1.03, h * 0.41), p(0.70, 1.03, h * 0.41), 0.56, shade(facing, 18.0), 235);
            self.line(p(1.03, -0.74, h * 0.36), p(1.03, -0.30, h * 0.36), 0.51, shade(gable, 19.0), 227);
        }

        let profile = |s: f32, t: f32| p(s, t, h + (ridge - h) * (1.0 - t.abs()) + 0.41);
        for side in [-1.0_f32, 1.0] {
            let sun = (side > 0.0) != turned;
            let color = shade(roof, if sun { 17.0 } else { -23.0 });
            let q = |s, t| profile(s, side * t);
            self.poly(&[q(-1.14, 1.20), q(1.13, 1.20), q(1.13, 0.0), q(-1.14, 0.0)], color);
            if !self.far {
                if wooden {
                    self.segments((-4..=4).map(|i| {
                        let t = i as f32 * 0.235;
                        (q(t, 0.04), q(t, 1.17))
                    }), 0.43, shade(color, -27.0), 208);
                    self.segments([-0.78, -0.29, 0.46, 0.85].into_iter().map(|t| {
                        (q(t, 0.15), q(t + 0.025, 1.14))
                    }), 0.47, shade(color, 17.0), 211);
                } else {
                    self.shingles(q, color, w);
                }
            }
            if frost {
                let snow = if sun { SNOW } else { [161, 184, 186] };
                self.poly(&[q(-1.13, 0.90), q(-0.49, 1.02), q(0.12, 0.78),
                    q(0.68, 0.97), q(1.11, 0.85), q(1.12, 0.0), q(-1.13, 0.0)], snow);
            }
            self.line(q(-1.13, 1.20), q(1.12, 1.20), 0.77, shade(color, -35.0), 253);
            if sun && !self.far {
                self.line(q(-1.12, 1.13), q(1.11, 1.13), 0.42, shade(color, 24.0), 241);
            }
        }
        self.line(profile(-1.12, 0.0), profile(1.12, 0.0), 0.65,
            if frost { SNOW } else { shade(roof, 23.0) }, 244);
        if w > 6.1 {
            let (cx, cy) = profile(-0.52, if turned { -0.20 } else { 0.20 });
            self.chimney((cx, cy), if w > 8.0 { 1.28 } else { 0.91 }, 3.53, frost);
        }
    }

    fn shingles(&mut self, p: impl Fn(f32, f32) -> Point, color: Rgb, width: f32) {
        let courses = if width > 8.0 { 5 } else { 4 };
        let step = 1.19 / courses as f32;
        let ends = if width > 8.0 { 7 } else { 5 };
        for row in 0..courses {
            let s = row as f32 * step;
            let seam = s + step - 0.015;
            self.poly(&[p(-1.12, s + 0.027), p(1.11, s + 0.02), p(1.11, s + step * 0.64),
                p(-1.12, s + step * 0.70)], shade(color, 8.0 + (row % 3) as f32 * 4.0));
            self.line(p(-1.11, seam), p(1.10, seam), 0.44, shade(color, -26.0), 237);
            self.segments((1..ends).map(|i| {
                let t = -1.11 + (i as f32 + (row % 2) as f32 * 0.43) * 2.19 / ends as f32;
                (p(t, s + 0.016), p(t + 0.012, seam))
            }), 0.34, shade(color, -25.0), 216);
        }
    }

    fn chimney(&mut self, (x, y): Point, w: f32, h: f32, frost: bool) {
        let p = |a: f32, b: f32| (x + a * w, y + b);
        self.poly(&[p(-1.0, 0.0), p(-1.0, -h), p(0.16, -h + 0.48), p(1.0, -h),
            p(1.0, 0.10), p(0.16, 0.62)], shade(CLAY, -12.0));
        self.poly(&[p(-1.0, 0.0), p(-1.0, -h), p(0.16, -h + 0.48), p(0.16, 0.62)], shade(CLAY, 23.0));
        self.poly(&[p(-1.17, -h), p(0.0, -h - 0.60), p(1.17, -h), p(0.10, -h + 0.71)],
            if frost { SNOW } else { shade(CLAY, 49.0) });
        self.line(p(-0.40, -h), p(0.42, -h + 0.09), 0.46, shade(INK, -16.0), 253);
        if !self.far {
            self.segments([0.36, 0.68].into_iter().map(|z| (p(-0.98, -h * z), p(0.12, -h * z + 0.48))),
                0.39, shade(CLAY, -31.0), 218);
        }
    }

    fn stone_face(&mut self, p: impl Fn(f32, f32) -> Point, color: Rgb, width: f32, height: f32) {
        self.poly(&[p(0.0, 0.0), p(1.0, 0.0), p(1.0, height), p(0.0, height)], color);
        if self.far {
            return;
        }
        let courses = ((height / 2.16).round() as usize).clamp(1, 9);
        let bays = ((width / 3.42).round() as usize).clamp(1, 9);
        let rise = height / courses as f32;
        let span = 1.0 / bays as f32;
        let (mut warm, mut cool) = (PathBuilder::new(), PathBuilder::new());
        let mut joints = PathBuilder::with_capacity(2 * courses * (bays + 1), 2 * courses * (bays + 1));
        for row in 0..courses {
            let z = row as f32 * rise;
            let shift = if row % 2 == 0 { 0.0 } else { -0.48 };
            for i in 0..=bays {
                let a = ((i as f32 + shift) * span).max(0.0);
                let b = ((i as f32 + shift + 1.0) * span).min(1.0);
                if b - a > 0.14 * span && (row + i) % 3 != 1 {
                    let to = if (row * 5 + i) % 4 < 2 { &mut warm } else { &mut cool };
                    append_polygon(to, &[p(a, z), p(b, z), p(b, z + rise), p(a, z + rise)]);
                }
                let t = (i as f32 + shift) * span;
                if t > 0.0 && t < 1.0 {
                    let (a, b) = (p(t, z + 0.14), p(t, z + rise - 0.14));
                    joints.move_to(a.0, a.1);
                    joints.line_to(b.0, b.1);
                }
            }
            if row > 0 {
                let (a, b) = (p(0.015, z), p(0.985, z));
                joints.move_to(a.0, a.1);
                joints.line_to(b.0, b.1);
            }
        }
        for (path, tint) in [(warm, 13.0), (cool, -11.0)] {
            if let Some(path) = path.finish() {
                self.fill(path, shade(color, tint), 255);
            }
        }
        if let Some(path) = joints.finish() {
            self.stroke(&path, 0.45, shade(color, -31.0), 223);
        }
    }

    fn tower(&mut self, (x, y): Point, w: f32, h: f32, stone: bool) {
        let p = |a: f32, b: f32| (x + a * w, y + b);
        let face = |a: f32, b: f32, z: f32| p(a, b - z);
        let left = |t, z| face(lerp(-1.0, 0.04, t), lerp(-1.5, 2.0, t), z);
        let right = |t, z| face(lerp(0.04, 1.0, t), lerp(2.0, -1.5, t), z);
        let top = -h;
        self.cast_shadow((x, y), w, h + if stone { 0.0 } else { w });
        self.stone_face(left, MASON, w, h);
        self.stone_face(right, SHADE, w, h);
        self.line(left(0.02, 1.4), left(0.99, 1.4), 0.74, shade(MASON, 29.0), 235);
        self.line(right(0.01, 1.4), right(0.98, 1.4), 0.67, shade(SHADE, 22.0), 227);
        self.line(left(1.0, 1.65), left(1.0, h - 0.02), 0.54, shade(MASON, 3.0), 186);
        let opening = [46, 43, 29];
        for z in [0.27_f32, 0.66] {
            self.line(left(0.53, z * h), left(0.53, z * h + 2.45), w * 0.19, opening, 255);
            if !self.far {
                self.line(left(0.37, z * h - 0.17), left(0.69, z * h - 0.17), 0.53, shade(MASON, 24.0), 249);
            }
        }
        self.line(right(0.57, h * 0.52), right(0.57, h * 0.52 + 2.51), w * 0.15, opening, 250);
        if stone {
            let coping = if self.season == 3 { SNOW } else { CREAM };
            self.poly(&[p(-1.1, top - 1.4), p(0.0, top - 4.13), p(1.06, top - 1.4), p(0.04, top + 2.04)], coping);
            self.poly(&[p(-0.74, top - 1.46), p(0.0, top - 3.28), p(0.73, top - 1.46), p(0.04, top + 0.90)], SHADE);
            for (offset, down) in [(-0.88, -1.45), (-0.30, 0.3), (0.30, 0.5), (0.88, -1.41)] {
                self.poly(&[p(offset - 0.20, top + down), p(offset - 0.20, top + down - 2.5),
                    p(offset + 0.17, top + down - 2.3), p(offset + 0.17, top + down + 0.3)],
                    shade(MASON, if offset < 0.0 { 26.0 } else { -10.0 }));
            }
        } else {
            let [peak, a, b, c] = [p(-0.09, top - w * 1.68), p(-1.15, top - 1.4),
                p(0.04, top + 2.35), p(1.14, top - 1.4)];
            let roof = if self.season == 3 { SNOW } else { shade(SLATE, 19.0) };
            self.poly(&[peak, a, b], roof);
            self.poly(&[peak, b, c], shade(roof, -36.0));
            self.line(a, b, 0.78, shade(roof, -29.0), 245);
            self.line(b, c, 0.78, shade(roof, -55.0), 245);
            if !self.far {
                let lerp_point = |(x, y): Point, (u, v): Point, t| (lerp(x, u, t), lerp(y, v, t));
                for (edge, tint) in [(a, -24.0), (c, -49.0)] {
                    self.segments([0.29, 0.48, 0.66, 0.83].into_iter().map(|t| {
                        (lerp_point(peak, edge, t), lerp_point(peak, b, t))
                    }), 0.39, shade(roof, tint), 219);
                }
            }
        }
    }

    pub(super) fn castle(&mut self, frame: u8, (x, y): Point, approach: Option<(usize, Point)>) {
        let rank = f32::from((frame % 20) / 4);
        let built = frame >= 80 || frame < 40;
        let scaffold = (40..60).contains(&frame);
        let court = [(x, y - 11.5), (x + 27.2, y), (x, y + 14.0), (x - 27.2, y)];
        let [back, right, front, left] = court;
        let surfaces = [(left, back), (back, right), (right, front), (front, left)];
        let [_, loam, paving] = earth(self.season);
        self.wash(&court, loam, 242);
        let rise = if scaffold { 1.7 } else if built { 6.0 + rank * 1.05 } else { 3.7 + rank * 0.62 };
        if scaffold {
            for (a, b) in surfaces {
                self.curtain(a, b, rise, MASON, false);
                let at = |t, h| (lerp(a.0, b.0, t), lerp(a.1, b.1, t) - h);
                for t in [0.09, 0.36, 0.63, 0.91] {
                    self.line(at(t, 0.0), at(t, 6.8), 0.91, TIMBER, 255);
                }
                self.line(at(0.07, 5.8), at(0.96, 5.8), 0.95, shade(TIMBER, 26.0), 225);
                self.line(at(0.07, 1.2), at(0.79, 6.0), 0.77, TIMBER, 219);
            }
            return;
        }

        let entry = approach.map_or(3, |(side, _)| side);
        let (ga, gb) = surfaces[entry];
        let gate = (lerp(ga.0, gb.0, 0.48), lerp(ga.1, gb.1, 0.48));
        if let Some((_, lane)) = approach {
            self.apron(gate, 4.6, lane);
        }
        let hub = (x - 0.2, y + 3.5);
        let bend = (gate.0 * 0.64 + x * 0.36, y + 1.0);
        self.apron_link(hub, bend, gate, 3.42);
        if !self.far {
            self.segments((-2..=3).map(|row| {
                let t = row as f32;
                ((x - 11.0 + t * 2.0, y - t * 0.73), (x + 7.0 + t * 2.0, y + 1.8 - t * 0.73))
            }), 0.48, shade(paving, 11.0), 137);
        }
        if built {
            // Grounded stone apron, subtle approach path, and pre-rendered AoE2 limestone castle keep!
            self.cast_shadow((x + 2.0, y + 6.0), 26.0 + rank * 1.4, 24.0 + rank * 1.6);
            let sprite = art_pack::prop_castle_keep();
            let px = self.transform.sx * x + self.transform.tx;
            let py = self.transform.sy * (y + 16.5) + self.transform.ty;
            let scale_f = 0.92 + rank * 0.045;
            let dest_w = 64.0 * scale_f * self.transform.sx;
            let dest_h = dest_w * (sprite.height as f32 / sprite.width as f32);
            let tint = match self.season {
                2 => [1.04, 0.98, 0.92],
                3 => [0.94, 0.98, 1.04],
                _ => [1.0, 1.0, 1.0],
            };
            let snow = if self.season == 3 { 0.72 } else { 0.0 };
            art_pack::stamp_sprite(self.image, sprite, px, py, dest_w, dest_h, tint, snow, false);
            return;
        }
        let buff = shade(MASON, 16.0);
        let walls = [shade(MASON, -20.0), MASON, SHADE, buff];
        let wall = |p: &mut Self, side: usize| {
            let (a, b) = surfaces[side];
            p.curtain(a, b, rise, walls[side], built);
            if side == entry {
                p.gate(a, b, rise, walls[side]);
            }
        };
        wall(self, 0);
        wall(self, 1);
        self.bastion(back, 3.75, rise + 4.9, built);
        self.tower((x - 4.8, y - 1.0), 7.0 + rank * 0.57,
            rise + if built { 14.1 + rank * 1.1 } else { 7.0 }, !built);
        if built {
            self.house((x + 8.1, y + 2.0), 4.6, 5.0, CLAY);
        }
        wall(self, 2);
        wall(self, 3);
        self.bastion(left, 4.35, rise + 5.1 + rank * 0.34, built);
        self.bastion(right, 4.48, rise + 5.8 + rank * 0.34, built);
        self.bastion(front, 3.85, rise + 3.6 + rank * 0.19, built);
    }

    fn curtain(&mut self, a: Point, b: Point, rise: f32, face: Rgb, built: bool) {
        let p = |t, h| (lerp(a.0, b.0, t), lerp(a.1, b.1, t) - h);
        let coping = if self.season == 3 { SNOW } else { shade(face, 35.0) };
        self.wash(&[a, b, (b.0 + rise * 0.56, b.1 + rise * 0.24),
            (a.0 + rise * 0.56, a.1 + rise * 0.24)], [42, 46, 31], 114);
        self.stone_face(p, face, (b.0 - a.0).abs(), rise);
        self.line(a, b, 1.05, shade(face, -53.0), 225);
        self.line(p(0.0, rise - 0.67), p(1.0, rise - 0.67), 0.73, shade(face, -24.0), 200);
        self.line(p(0.0, rise), p(1.0, rise), 1.30, coping, 253);
        if built {
            for i in 0..8 {
                let t = i as f32 * 0.133 + 0.008;
                let [a, b, c, d] = [p(t, rise - 0.13), p(t + 0.067, rise - 0.13),
                    p(t + 0.067, rise + 1.84), p(t, rise + 1.84)];
                self.poly(&[a, b, c, d], shade(face, 12.0));
                self.poly(&[d, c, (c.0 + 0.26, c.1 - 0.58), (d.0 + 0.26, d.1 - 0.58)], coping);
            }
        }
    }

    fn gate(&mut self, a: Point, b: Point, rise: f32, face: Rgb) {
        let p = |s: f32, h| (lerp(a.0, b.0, 0.48 + s * 0.105), lerp(a.1, b.1, 0.48 + s * 0.105) - h);
        let h = rise * 0.79;
        self.poly(&[p(-1.15, 0.0), p(-1.15, h * 0.57), p(-0.75, h * 0.91), p(0.0, h),
            p(0.75, h * 0.91), p(1.15, h * 0.57), p(1.15, 0.0)], shade(face, 32.0));
        self.poly(&[p(-0.73, 0.0), p(-0.73, h * 0.53), p(-0.43, h * 0.76), p(0.0, h * 0.83),
            p(0.43, h * 0.76), p(0.73, h * 0.53), p(0.73, 0.0)], shade(INK, -13.0));
        if !self.far {
            self.segments([
                (p(-1.09, h * 0.45), p(-0.73, h * 0.45)),
                (p(-0.85, h * 0.83), p(-0.57, h * 0.66)),
                (p(0.0, h), p(0.0, h * 0.84)),
                (p(0.85, h * 0.83), p(0.57, h * 0.66)),
                (p(1.09, h * 0.45), p(0.73, h * 0.45)),
            ], 0.43, shade(face, -27.0), 223);
            self.segments([-0.43, 0.0, 0.43].into_iter().map(|s| (p(s, 0.29), p(s, h * 0.64))),
                0.51, shade(TIMBER, 18.0), 249);
            self.line(p(-0.63, h * 0.32), p(0.63, h * 0.32), 0.55, TIMBER, 253);
            self.line(p(-0.67, 0.0), p(0.71, 0.0), 0.77, shade(MASON, 14.0), 249);
        }
    }

    fn bastion(&mut self, (x, y): Point, r: f32, h: f32, built: bool) {
        let p = |(u, v): Point, z, scale| (x + u * r * scale, y + v * r * scale * 0.51 - z);
        let ring = [(-1.0, 0.0), (-0.72, 0.72), (0.0, 1.0), (0.72, 0.72), (1.0, 0.0),
            (0.72, -0.72), (0.0, -1.0), (-0.72, -0.72)];
        let coping = if self.season == 3 { SNOW } else { CREAM };
        self.cast_shadow((x, y), r, h);
        for (i, pair) in ring.windows(2).take(4).enumerate() {
            let (a, b) = (pair[0], pair[1]);
            let face = shade(MASON, [3.0, 16.0, -23.0, -47.0][i]);
            let panel = |t, z| p((lerp(a.0, b.0, t), lerp(a.1, b.1, t)), z, 1.1 - z / h * 0.10);
            self.stone_face(panel, face, r * 0.72, h);
            self.line(panel(0.0, 1.10), panel(1.0, 1.10), 0.61, shade(face, 19.0), 237);
        }
        self.poly(&ring.map(|point| p(point, h, 1.06)), coping);
        self.poly(&ring.map(|point| p(point, h - 0.24, 0.70)), shade(SHADE, -39.0));
        if built {
            for i in [0, 2, 4, 6] {
                let point = ring[i];
                let (a, b) = p(point, h, 1.01);
                self.poly(&[(a - 0.78, b + 0.12), (a + 0.75, b + 0.13), (a + 0.76, b - 1.65),
                    (a - 0.79, b - 1.65)], shade(MASON, if i < 3 { 17.0 } else { -5.0 }));
                self.line((a - 0.72, b - 1.68), (a + 0.78, b - 1.68), 0.49, coping, 255);
            }
        }
        self.line((x - r * 0.33, y - h * 0.54), (x - r * 0.33, y - h * 0.73), 0.66, INK, 231);
    }

    pub(super) fn industry(&mut self, site: Site, wrecked: bool, working: bool) {
        if wrecked {
            for (x, y, r) in [(-10.4, 0.5, 3.9), (-1.6, 1.5, 4.9), (2.9, 6.0, 2.3), (9.9, 3.8, 3.7)] {
                self.rock_fragment((x, y), r, MASON);
                self.line((x - r, y + 1.22), (x + r * 0.63, y - 1.26), 1.15, shade(TIMBER, -15.0), 255);
            }
            return;
        }
        match site {
            Site::Quarry => {
                self.cut_bank(MASON);
                for (point, r) in [((-13.3, 2.8), 2.7), ((-3.1, 3.8), 2.3), ((13.1, 5.5), 2.3)] {
                    self.rock_fragment(point, r, MASON);
                }
                self.cast_shadow((8.8, 4.4), 2.14, 13.0);
                self.line((9.3, 4.1), (8.7, -10.9), 2.11, shade(TIMBER, -29.0), 255);
                self.line((8.1, 4.4), (8.0, -11.0), 0.82, shade(TIMBER, 29.0), 255);
                self.line((4.1, -11.0), (13.3, -10.2), 1.90, shade(TIMBER, 24.0), 255);
                self.line((5.3, -10.8), (8.4, -5.2), 0.78, TIMBER, 255);
                self.line((4.8, -10.9), (4.8, -2.5), 0.55, INK, 234);
                self.rock_fragment((4.7, -1.5), 1.9, CREAM);
            }
            Site::Forge => {
                self.cast_shadow((5.7, -3.0), 1.8, 14.0);
                self.chimney((5.7, -3.0), 1.8, 13.0, self.season == 3);
                self.house((-1.5, 3.9), 9.2, 9.1, SLATE);
                self.flow((-7.7, 3.2), (-5.5, -4.3), (-3.3, 3.2), 1.98, shade(MASON, -22.0), 255);
                self.mark((-5.4, 1.5), 1.82, 2.18, shade(INK, -23.0), 255);
                self.rock_fragment((-8.8, 4.4), 1.9, SHADE);
            }
            Site::Logging => {
                self.tree((-11.4, -1.1), 10.9, -0.60);
                self.tree((8.4, -3.8), 14.9, 0.74);
                self.house((-4.8, 4.5), 5.9, 5.3, TIMBER);
                self.cast_shadow((7.2, 7.6), 5.6, 2.3);
                for (x, y, length) in [(4.3, 2.6, 11.4), (3.6, 4.5, 10.2), (3.2, 6.1, 10.4), (6.1, 3.2, 8.6)] {
                    self.line((x, y), (x + length, y - 2.58), 2.11, shade(TIMBER, -20.0), 255);
                    self.line((x + 0.27, y - 0.51), (x + length, y - 3.12), 0.64, shade(TIMBER, 34.0), 235);
                    self.mark((x, y), 1.22, 0.94, [179, 129, 76], 255);
                    if !self.far {
                        self.mark((x + 0.10, y - 0.03), 0.70, 0.54, [132, 91, 48], 255);
                        self.mark((x + 0.04, y - 0.11), 0.34, 0.27, [192, 142, 82], 255);
                    }
                }
            }
            Site::Mine => {
                let slate = if self.season == 3 { [125, 141, 140] } else { [111, 112, 92] };
                self.cut_bank(slate);
                let p = |x, y| (x - 5.0, y + 0.86);
                self.poly(&[p(-5.0, 1.2), p(-4.20, -3.65), p(-0.10, -5.13),
                    p(4.3, -3.83), p(5.20, 1.13)], shade(TIMBER, -22.0));
                self.poly(&[p(-3.52, 1.11), p(-3.12, -2.9), p(-0.11, -4.0),
                    p(3.23, -2.92), p(3.63, 1.11)], [34, 33, 26]);
                self.line(p(-4.25, -3.64), p(-0.11, -5.06), 0.69, shade(TIMBER, 31.0), 255);
                self.line(p(-4.35, -3.02), p(-4.70, 0.92), 0.72, shade(TIMBER, 18.0), 239);
                if !self.far {
                    self.segments((-1..5).map(|i| {
                        let x = i as f32 * 3.56 + 0.75;
                        (p(x + 0.4, x * 0.16 - 0.32), p(x - 0.6, x * 0.16 + 2.98))
                    }), 0.90, TIMBER, 227);
                }
                self.line(p(-0.91, 0.43), (13.5, 4.5), 0.70, shade(SHADE, -27.0), 255);
                self.line(p(-2.0, 2.66), (12.6, 6.5), 0.79, shade(SHADE, -27.0), 255);
                if !working {
                    self.cart((1.5, 3.5), SHADE);
                }
            }
        }
    }

    fn cut_bank(&mut self, shale: Rgb) {
        let loam = earth(self.season)[0];
        self.wash(&[(-21.9, 1.0), (-16.4, -4.1), (-9.0, -8.2), (-1.4, -9.0), (6.8, -6.0), (12.4, -2.1),
            (10.5, 3.0), (5.8, 5.1), (-3.7, 6.2), (-13.9, 4.5)], loam, 213);
        self.poly(&[(-19.3, 0.4), (-15.4, -4.6), (-8.7, -7.4), (-0.7, -8.4), (7.5, -5.6),
            (9.4, -2.6), (5.6, 3.0), (-1.7, 4.7), (-11.7, 2.98)], shade(shale, -43.0));
        self.poly(&[(-19.3, 0.4), (-15.4, -4.6), (-8.7, -7.4), (-0.7, -8.4), (7.5, -5.6),
            (5.8, -4.12), (-0.7, -6.44), (-7.1, -5.49), (-13.2, -3.46), (-16.5, -0.03)], shade(shale, 15.0));
        self.poly(&[(-16.3, 1.25), (-12.4, -2.55), (-6.4, -4.72), (-0.1, -5.35), (4.8, -3.40),
            (3.5, -1.76), (-0.67, -3.38), (-6.82, -2.76), (-10.77, -1.19), (-13.96, 2.48)], shade(shale, -3.0));
        self.poly(&[(-12.1, 2.17), (-8.5, -0.4), (-2.5, -1.51), (2.8, -0.33), (5.6, 3.0),
            (-1.7, 4.7), (-7.71, 3.66)], shade(shale, -21.0));
        if !self.far {
            self.segments([((-13.4, -5.21), (-11.56, -4.02)), ((-4.23, -8.04), (-3.30, -6.88)),
                ((1.95, -7.50), (0.63, -6.46)), ((-10.59, -3.13), (-9.42, -1.90)),
                ((-3.15, -4.95), (-2.11, -3.77)), ((-4.46, -1.04), (-3.65, 1.10))],
                0.61, shade(shale, -40.0), 227);
        }
    }

    fn rock_fragment(&mut self, (x, y): Point, r: f32, stone: Rgb) {
        let p = |a: f32, b: f32| (x + a * r, y + b * r * 0.56);
        self.wash(&[p(-0.92, 0.17), p(0.52, -0.37), p(1.41, 0.31), p(0.84, 0.63), p(-0.3, 0.67)],
            [44, 49, 32], 153);
        self.poly(&[p(-1.0, 0.18), p(-0.51, -0.77), p(0.15, -1.18), p(0.88, -0.54), p(0.79, 0.32), p(-0.22, 0.51)],
            shade(stone, -26.0));
        self.poly(&[p(-1.0, 0.18), p(-0.51, -0.77), p(0.15, -1.18), p(0.47, -0.63), p(0.06, -0.22)],
            shade(stone, 13.0));
    }

    fn cart(&mut self, (x, y): Point, color: Rgb) {
        self.poly(&[(x - 2.6, y - 2.1), (x + 2.2, y - 2.0), (x + 1.7, y + 0.7),
            (x - 2.0, y + 0.6)], color);
        self.mark((x - 1.6, y + 0.9), 0.62, 0.60, INK, 238);
        self.mark((x + 1.3, y + 1.0), 0.68, 0.61, INK, 238);
    }

    pub(super) fn motion(&mut self, site: Site, frame: u8) {
        match site {
            Site::Quarry => {
                let y = 0.4 + f32::from(frame.saturating_sub(1).min(7)) * 0.40;
                self.mark((4.7, y), 1.65, 0.75, shade(CREAM, 37.0), 222);
            }
            Site::Forge => {
                let flicker = f32::from(frame % 3) * 0.25;
                self.mark((-5.25, 2.1 - flicker), 1.31, 1.05 + flicker, [204, 104, 52], 228);
                self.mark((-5.55, 2.5 - flicker), 0.55, 0.70, [255, 191, 105], 255);
            }
            Site::Logging => {
                let x = 5.5 + f32::from(frame.saturating_sub(20).min(8)) * 0.44;
                self.line((x - 2.0, 3.0), (x + 2.3, 1.72), 0.84, shade(CREAM, 16.0), 241);
            }
            Site::Mine => {
                let x = -0.8 + f32::from(frame.saturating_sub(30).min(15)) * 0.55;
                self.cart((x, 3.0 + x * 0.115), shade(SHADE, 18.0));
            }
        }
    }

}
