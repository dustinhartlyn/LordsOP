//! World-space pigments, relief, and ground cover for the detail renderer.
//! A managed landscape underlies the live feature stencil. Slopes, cut
//! headlands and drainage margins are surface detail, not obstacles.

use l2_sim::fire::{SURFACE_WOODLAND, SURFACE_WOOD_CATCHING};
use l2_sim::terrain::{Battlefield, DIM, id};
use l2_view::scene::TILE;
use tiny_skia::{PathBuilder, Pixmap};

use super::{SIZE, noise, stroke};
use crate::original::art_pack;
use crate::original::battle::{road_mask, water_edges};

#[inline]
fn add_tex(base: Rgb, delta: [f32; 3], k: f32) -> Rgb {
    [
        (base[0] + delta[0] * k).clamp(0.0, 255.0),
        (base[1] + delta[1] * k).clamp(0.0, 255.0),
        (base[2] + delta[2] * k).clamp(0.0, 255.0),
    ]
}

const EDGE: f32 = TILE as f32;
const HALF: f32 = EDGE / 2.0;
const NODES: usize = TILE as usize + 1;
const APPROACH_ROWS: usize = 5;
const STENCIL: usize = 5;
const FULL: u32 = (1 << (STENCIL * STENCIL)) - 1;
pub(super) const DECK_HALF: f32 = 32.0;
const APPROACH_REACH: f32 = APPROACH_ROWS as f32 * EDGE;
const CROWN: f32 = 0.44;
const SCARP_FOOT: f32 = 0.76;
type Rgb = [f32; 3];

/// Features contributing pixels to this tile. Together with world position,
/// this is the complete terrain cache dependency.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Land {
    stamps: [Stamp; 9],
    river: u32,
    relief: Option<RidgeRuns>,
    landings: [Option<Landing>; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Stamp { frame: u8, kind: Kind }

impl Stamp {
    fn at(field: &Battlefield, x: usize, y: usize) -> Self {
        let cell = field.at(x, y);
        if cell.tileset() != 0 {
            return Self { frame: u8::MAX, kind: Kind::Meadow };
        }
        let kind = match cell.gfx {
            // The last bridge frame overlaps the hill catch-all. A
            // bridge stamp clears the terrain byte; a hill keeps its id.
            0x8c..=0x9e => Kind::Bridge,
            0x9f if cell.terrain == 0 => Kind::Bridge,
            0xa0..=0xe5 => Kind::Water,
            0x40..=0x6f | 0x9f if cell.terrain == id::OBSTACLE => Kind::Slope,
            _ if matches!(cell.surface, SURFACE_WOODLAND | SURFACE_WOOD_CATCHING) => Kind::Woodland,
            _ => Kind::Meadow,
        };
        Self { frame: cell.gfx, kind }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind { Meadow, Woodland, Slope, Water, Bridge }

impl Kind {
    fn wet(self) -> bool { matches!(self, Self::Water | Self::Bridge) }
}

// Keep the endpoints of actual slope runs, rather than treating each hill
// stamp as a height bump. Three rows also cover off-tile grass roots.
// These endpoints are part of the atlas key.
type RidgeRuns = [[RidgeRun; STENCIL]; 3];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct RidgeRun {
    north: u8,
    south: u8,
}

struct RidgeSection {
    span: f32,
    t: f32,
    height: f32,
    grade: f32,
}

impl RidgeRun {
    fn at(field: &Battlefield, x: usize, y: usize) -> Self {
        let slope = |y| Stamp::at(field, x, y).kind == Kind::Slope;
        let (mut north, mut south) = (y, y + 1);
        while north > 0 && slope(north - 1) { north -= 1; }
        while south < DIM && slope(south) { south += 1; }
        Self { north: north as u8, south: south as u8 }
    }

    fn present(self) -> bool { self.south != 0 }

    fn section(self, y: f32) -> Option<RidgeSection> {
        if !self.present() { return None; }
        let span = f32::from(self.south - self.north) * EDGE;
        let t = (y - f32::from(self.north) * EDGE) / span;
        // A broad, sunward back rises to one crown. Most of the drop on the
        // far side happens immediately below it; the scree apron rolls out
        // much more slowly to the foot.
        let (back, up) = ramp(0.0, CROWN - 0.01, t);
        let (scarp, down) = ramp(CROWN, SCARP_FOOT, t);
        let (apron, toe) = ramp(0.66, 1.0, t);
        let face = 1.0 - (0.8 * scarp + 0.2 * apron);
        let rise = (span * 0.48).min(67.0);
        Some(RidgeSection {
            span, t, height: rise * back * face,
            grade: rise / span * (up * face - back * (0.8 * down + 0.2 * toe)),
        })
    }
}

impl Land {
    pub(super) fn at(field: &Battlefield, x: usize, y: usize) -> Self {
        let stamps: [Stamp; 9] = std::array::from_fn(|i| {
            let x = (x + i % 3).saturating_sub(1).min(DIM - 1);
            let y = (y + i / 3).saturating_sub(1).min(DIM - 1);
            Stamp::at(field, x, y)
        });
        let mut river = 0;
        let mut runs = RidgeRuns::default();
        let mut has_ridge = false;
        for j in 0..STENCIL {
            let cy = (y + j).saturating_sub(2);
            for i in 0..STENCIL {
                let stamp = if (1..=3).contains(&i) && (1..=3).contains(&j) {
                    stamps[(j - 1) * 3 + i - 1]
                } else {
                    Stamp::at(field, (x + i).saturating_sub(2).min(DIM - 1), cy.min(DIM - 1))
                };
                let bit = 1 << (j * STENCIL + i);
                if stamp.kind.wet() { river |= bit; }
                if (1..=3).contains(&j) && cy < DIM && stamp.kind == Kind::Slope {
                    has_ridge = true;
                    let row = j - 1;
                    runs[row][i] = if row > 0 && cy < usize::from(runs[row - 1][i].south) {
                        runs[row - 1][i]
                    } else {
                        RidgeRun::at(field, (x + i).saturating_sub(2).min(DIM - 1), cy)
                    };
                }
            }
        }
        let landings = [false, true].map(|south| Landing::find(field, x, y, south));
        Self { stamps, river, relief: has_ridge.then_some(runs), landings }
    }

    pub(super) fn frame(self) -> u8 { self.stamps[4].frame }
    pub(super) fn kind(self) -> Kind { self.stamps[4].kind }

    pub(super) fn bridge_ends(self) -> [bool; 2] {
        self.landings.map(|entry| {
            entry.is_some_and(|end| end.y == if end.south { TILE as i16 } else { 0 })
        })
    }
}

/// The approaches use one canonical column in each deck. Extending the roads
/// doesn't require storing a larger square coast stencil.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Landing {
    x: i16,
    y: i16,
    south: bool,
}

impl Landing {
    fn find(field: &Battlefield, x: usize, y: usize, south: bool) -> Option<Self> {
        for offset in 0..=APPROACH_ROWS {
            let row = if south { y.checked_sub(offset) } else { y.checked_add(offset) }?;
            let bank = if south { row.checked_add(1) } else { row.checked_sub(1) };
            if row >= DIM { break; }
            let Some(bank) = bank.filter(|&row| row < DIM) else { continue; };
            let mut nearest = None;
            for col in x.saturating_sub(2)..=(x + 1).min(DIM - 1) {
                let frame = field.at(col, row).gfx;
                if !(0x8c..=0x9f).contains(&frame) || (frame - 0x8c) % 4 != 1
                    || Stamp::at(field, col, row).kind != Kind::Bridge
                    || Stamp::at(field, col, bank).kind.wet()
                {
                    continue;
                }
                let entry = Self {
                    x: (col as i16 - x as i16 + 1) * TILE as i16,
                    y: (row as i16 - y as i16 + south as i16) * TILE as i16,
                    south,
                };
                let distance = (entry.x - TILE as i16 / 2).abs();
                if nearest.is_none_or(|(old, _)| distance < old) {
                    nearest = Some((distance, entry));
                }
            }
            if let Some((_, entry)) = nearest { return Some(entry); }
        }
        None
    }

    fn footprint(self, x: f32, y: f32) -> Option<Track> {
        let direction = if self.south { 1.0 } else { -1.0 };
        let d = (y - f32::from(self.y)) * direction;
        let from_center = x - f32::from(self.x);
        if !(-17.0..APPROACH_REACH).contains(&d) || from_center.abs() >= 63.0 { return None; }
        let bend = direction * (16.0 * smooth(7.0, 94.0, d) - 19.0 * smooth(96.0, APPROACH_REACH, d));
        Some(Track { d, from_center, across: (from_center - bend).abs() })
    }
}

impl Track {
    fn paint(self, wx: f32, wy: f32, wet: f32, ground: Rgb) -> Rgb {
        let Track { d, from_center, across } = self;
        let rubble = value(wx / 5.7, wy / 3.4, 0xf73d_7a41);
        let rough = rubble - 0.5;
        let width = track_half_width(d);
        let lane = 1.0 - smooth(width - 1.8, width + 2.4, across + rough * 3.4);
        let rut = 1.0 - smooth(1.1, 3.6, (across - width * 0.53 - rough * 2.1).abs());
        let crown = 1.0 - smooth(3.0, 6.8, across);
        let use_amount = 1.0 - smooth(APPROACH_REACH - 45.0, APPROACH_REACH - 11.0, d);
        let packed = 1.0 - smooth(9.0, 43.0, d);
        let dust = tone(mix([155., 133., 94.], [187., 157., 108.], rut * 0.64),
            rough * 24.0 - crown * (7.0 - packed * 5.0));
        let dry = 1.0 - smooth(0.42, 0.59, wet) * smooth(5.0, 18.0, d);
        let road = mix(ground, dust, lane * use_amount * smooth(-2.0, 3.0, d)
            * (0.82 + packed * 0.18) * dry);

        // A low stone sill sits in a flared earth apron. Its battered
        // wing ends dissolve into the bank rather than projecting as posts.
        let away = from_center.abs();
        let flare = DECK_HALF + 8.0 + d.max(0.0) * 0.65;
        let apron = (1.0 - smooth(flare + 1.0, flare + 9.0, away + rough * 5.0))
            * smooth(-16.0, -2.5, d) * (1.0 - smooth(8.0, 29.0, d));
        let shoulder = apron * smooth(width - 4.0, width + 8.0, across);
        let earth = tone(mix([103., 98., 70.], [149., 130., 93.], rubble), rough * 12.0);
        let road = mix(road, earth, shoulder * 0.86);
        let wing = away - flare;
        let toe = smooth(-15.0, -9.5, d) * (1.0 - smooth(11.0, 22.0, d));
        let wall = (1.0 - smooth(2.9, 7.5, wing.abs() + rough * 2.8)) * toe;
        let bed = apron * (1.0 - smooth(0.0, 4.9, d.abs())) * smooth(DECK_HALF - 6.0, DECK_HALF, away);
        if wall + bed <= 0.0 { return road; }
        let row = (d + 21.0 + rough * 0.55) / 3.5;
        let course = (from_center + row.floor() * 2.9 + rough * 1.4) / 6.5;
        let joint = smooth(0.06, 0.23, row.fract()) * smooth(0.05, 0.19, course.rem_euclid(1.0));
        let lit = if from_center < 0.0 { wing } else { -wing };
        let stone = tone(mix([77., 74., 59.], [153., 143., 115.], joint),
            lit.clamp(-6.0, 6.0) * 2.1 + rough * 33.0);
        mix(road, stone, wall.max(bed * 0.77))
    }

    fn setback(self) -> f32 {
        let verge = lerp(9.0, 2.0, smooth(APPROACH_REACH - 27.0, APPROACH_REACH, self.d));
        self.across - track_half_width(self.d) - verge
    }

    fn clears_growth(self) -> bool {
        ((-15.0..25.0).contains(&self.d) && self.from_center.abs() < DECK_HALF + 23.0)
            || (self.d >= 0.0 && self.setback() < -5.0)
    }
}

fn track_half_width(d: f32) -> f32 { lerp(DECK_HALF - 3.0, 13.0, smooth(1.0, 58.0, d)) }

#[derive(Clone, Copy)]
struct Track { d: f32, from_center: f32, across: f32 }

struct Surface {
    wet: [bool; 9],
    river: u32,
    wood: [bool; 9],
    relief: Option<RidgeRuns>,
    roads: Option<[Option<u8>; 9]>,
    landings: [Option<Landing>; 2],
    field: bool,
}

impl Surface {
    fn new(land: Land) -> Self {
        let roads = land.stamps.map(|s| road_links(s.frame));
        Self {
            wet: land.stamps.map(|s| s.kind.wet()),
            river: land.river,
            wood: land.stamps.map(|s| s.kind == Kind::Woodland),
            relief: land.relief,
            roads: roads.iter().any(Option::is_some).then_some(roads),
            landings: land.landings,
            field: true,
        }
    }

    fn color(&self, x: f32, y: f32, wx: f32, wy: f32) -> Rgb {
        let weights = Weights::new(x, y);
        let (wet, light) = self.coast(x, y, &weights);
        if !self.field {
            return if wet > 0.01 { shore(|| meadow(wx, wy), wet, light, wx, wy) } else { meadow(wx, wy) };
        }

        let tracks = self.tracks(x, y);
        let road = self.road_distance(x, y);
        let bank = Bank::new(wet, wx, wy);
        let ground = || {
            let ridge = self.ridge(x, y, (wx, wy), &weights);
            let nap = sward_density(wx, wy);
            upland(|| {
                let wood = weights.sample(&self.wood);
                let clearance = field_setback(&tracks, road, &ridge, wood, &bank);
                ground_color(wood, clearance, wx, wy, nap)
            }, wx, wy, nap, &ridge)
        };
        let mut color = bank.paint(ground, light, wx, wy);
        if road < 6.7 && bank.depth < 0.48 {
            let grit = value(wx / 2.6, wy / 2.0, 0xf73d_7a41);
            let rut = 1.0 - smooth(0.25, 1.1, (road - 2.2).abs());
            let earth_d = art_pack::earth().detail(wx * 2.5, wy * 2.5);
            let dust = add_tex(tone([166., 139., 99.], grit * 14.0 - 7.0 + rut * 10.0), earth_d, 0.88);
            color = mix(color, dust, 1.0 - smooth(3.6, 5.8, road + (grit - 0.5) * 1.8));
        }
        for track in tracks.into_iter().flatten() {
            color = track.paint(wx, wy, bank.depth, color);
        }
        color
    }

    fn tracks(&self, x: f32, y: f32) -> [Option<Track>; 2] {
        self.landings.map(|landing| landing.and_then(|entry| entry.footprint(x, y)))
    }

    fn road_distance(&self, x: f32, y: f32) -> f32 {
        let Some(roads) = &self.roads else { return f32::INFINITY; };
        let mut distance = f32::INFINITY;
        for (i, links) in roads.iter().enumerate() {
            let Some(links) = *links else { continue; };
            let dx = x - (i % 3) as f32 * EDGE + HALF;
            let dy = y - (i / 3) as f32 * EDGE + HALF;
            for (mask, along, across) in [(1, -dy, dx), (2, dx, dy), (4, dy, dx), (8, -dx, dy)] {
                if links & mask != 0 {
                    let end = (-along).max(along - HALF).max(0.0);
                    let side = across.abs();
                    distance = distance.min(side.max(end) + side.min(end) * 0.39);
                }
            }
            if links == 0 { distance = distance.min(dx.abs().max(dy.abs())); }
        }
        distance
    }

    fn ridge(&self, x: f32, y: f32, (wx, wy): (f32, f32), weights: &Weights) -> Ridge {
        let Some(runs) = &self.relief else { return Ridge::FLAT; };
        let row = if y < 0.0 { 0 } else if y < EDGE { 1 } else { 2 };
        let (w, d) = coast_kernel(x / EDGE + 0.5 - weights.col as f32, 1.0);
        let mut ridge = Ridge::FLAT;
        let jitter = value((wx + wy * 0.24) / 17.0, wy / 30.0, 0x13e1_41b5) - 0.5;
        for (i, (w, d)) in w.into_iter().zip(d).enumerate() {
            let Some(section) = runs[row][i + weights.col].section(wy) else { continue; };
            let RidgeSection { span, t, height, grade } = section;
            ridge.surface.height += height * w;
            ridge.surface.dx += height * d / EDGE;
            ridge.surface.dy += grade * w;

            // A short hill still has a lip and foot; on a long run the
            // outcrop follows its break in grade, not a fixed steepness.
            let unit = (span * 0.08).clamp(2.8, 9.0);
            let lip = (t - CROWN) * span / unit + jitter * 1.35;
            let foot = (t - SCARP_FOOT) * span / unit + jitter * 0.85;
            let crown = smooth(-1.4, -0.32, lip) * (1.0 - smooth(-0.24, 0.29, lip));
            let face = smooth(-0.30, 0.55, lip) * (1.0 - smooth(-0.36, 0.57, foot));
            let talus = smooth(-0.66, 0.41, foot) * (1.0 - smooth(0.85, 1.0, t));
            ridge.crown += crown * w;
            ridge.face += face * w;
            ridge.talus += talus * w;
        }
        ridge
    }

    fn coast(&self, x: f32, y: f32, weights: &Weights) -> (f32, f32) {
        if !self.field {
            return (weights.sample(&self.wet), weights.bank_light(&self.wet));
        }
        match self.river {
            0 => (0.0, 0.0),
            FULL => (1.0, 0.0),
            mask => river_coast(mask, x, y, weights.col, weights.row),
        }
    }
}

struct Ridge {
    surface: Profile,
    crown: f32,
    face: f32,
    talus: f32,
}

impl Ridge {
    const FLAT: Self = Self {
        surface: Profile { height: 0.0, dx: 0.0, dy: 0.0 },
        crown: 0.0,
        face: 0.0,
        talus: 0.0,
    };
}

fn upland(ground: impl FnOnce() -> Rgb, x: f32, y: f32, nap: f32, ridge: &Ridge) -> Rgb {
    let Profile { height, dx, dy } = ridge.surface;
    let cover = smooth(0.3, 6.0, height).max(ridge.talus * 0.88);
    if cover <= 0.0 { return ground(); }
    let light = ((0.43 * dx + 0.83 * dy + 0.61) / (1.0 + dx * dx + dy * dy).sqrt()).max(0.0);
    let drifts = value((x - y * 0.24) / 24.0, y / 34.0, 0xddec_2841);
    let turf = tone(mix([89., 109., 63.], [110., 124., 75.], drifts * 0.57 + nap * 0.43),
        (light - 0.61) * 61.0);
    let crest = tone([147., 142., 101.], (nap - 0.5) * 31.0);
    let rim = smooth(0.11, 0.63, drifts * 0.73 + nap * 0.27);
    let mut material = mix(turf, crest, ridge.crown * (0.44 + rim * 0.51));
    let face = ridge.face.max(smooth(0.38, 1.21, dx.abs()) * smooth(6.0, 18.0, height) * 0.73);
    if face > 0.0 {
        let rock = bedrock(x, y, drifts);
        material = mix(material, tone(rock, (light - 0.61) * 27.0), face);
    }
    if ridge.talus > 0.0 {
        let gravel = tone(mix([91., 91., 67.], [130., 117., 81.], nap), (light - 0.61) * 16.0);
        let chips = scree(x, y, nap);
        material = mix(material, tone(gravel, chips), ridge.talus);
    }
    if cover >= 1.0 { return material; }
    mix(ground(), material, cover)
}

fn bedrock(x: f32, y: f32, drifts: f32) -> Rgb {
    // Real stratified granite rock texture combined with natural cliff ledges.
    let warp = value((x + y * 0.36) / 41.0, y / 26.0, 0x4934_1ca7);
    let flake = value((x - y * 0.42) / 13.0, y / 10.0, 0x391f_d75c);
    let bed = (y + x * 0.24 + warp * 29.0 + flake * 3.8) / (19.0 + warp * 8.5);
    let course = bed.rem_euclid(1.0);
    let exposure = smooth(0.30, 0.64, flake * 0.57 + drifts * 0.43);
    let overhang = (1.0 - smooth(0.014, 0.17, course)) * exposure;
    let lip = smooth(0.03, 0.19, course) * (1.0 - smooth(0.25, 0.53, course)) * exposure;
    let sampled = art_pack::rock().sample_f32(x * 2.5, y * 2.5);
    let base = mix([94., 91., 81.], sampled, 0.78);
    tone(base, lip * 28.0 - overhang * 32.0 + (flake - 0.5) * 16.0)
}

fn scree(x: f32, y: f32, density: f32) -> f32 {
    let (u, v) = ((x - y * 0.23) / 6.8, y / 5.3);
    let chip = noise(u.floor() as i32, v.floor() as i32, 0xb664_239f);
    let size = (chip >> 24) as f32 / 255.0;
    if size + density * 0.66 < 0.67 { return 0.0; }
    let u = u.rem_euclid(1.0) - (0.41 + (chip & 31) as f32 * 0.005);
    let v = v.rem_euclid(1.0) - (0.41 + ((chip >> 5) & 31) as f32 * 0.005);
    let radius = 0.14 + size * 0.12;
    let shape = (u + v * 0.26).abs().max(v.abs() * 1.29).max((u - v * 0.63).abs());
    let edge = 1.0 - smooth(radius - 0.11, radius, shape);
    let shadow = 1.0 - smooth(radius - 0.06, radius + 0.03, shape - (u + v).max(0.0) * 0.23);
    edge * (23.0 - (u + v) * 77.0) - (shadow - edge).max(0.0) * 23.0
}

fn field_setback(tracks: &[Option<Track>; 2], road: f32, ridge: &Ridge, wood: f32, bank: &Bank) -> f32 {
    let mut setback = (road - 10.0).min((2.2 - ridge.surface.height) * 5.0).min((0.22 - wood) * 47.0)
        .min(bank.setback()).min((0.24 - ridge.talus) * 31.0);
    for track in tracks.iter().flatten() {
        // Reserve the grassy verge before painting any wagon marks.
        let cut = track.setback().max(track.d - (APPROACH_REACH - 9.0)).max(-track.d - 3.0);
        setback = setback.min(cut);
    }
    setback
}

fn ground_color(wood: f32, setback: f32, x: f32, y: f32, nap: f32) -> Rgb {
    let color = field_meadow(x, y, setback, nap);
    if wood > 0.0 {
        let variation = value(x / 9.0, y / 6.0, 0xb2d6_d94f) - 0.5;
        mix(color, tone([59., 80., 49.], variation * 21.0), wood * 0.79)
    } else {
        color
    }
}

fn road_links(frame: u8) -> Option<u8> {
    (0xe6..=0xf8).contains(&frame).then(|| road_mask(usize::from(frame - 0xe6)))
}

/// Interpolate between cell centers. Both sides of a tile edge evaluate the
/// same four cells, so a bend can round off a corner without drawing a seam.
struct Weights {
    row: usize,
    col: usize,
    x: f32,
    y: f32,
    dx: f32,
    dy: f32,
}

impl Weights {
    fn new(x: f32, y: f32) -> Self {
        let col = usize::from(x >= HALF);
        let row = usize::from(y >= HALF);
        let (u, v) = (x / EDGE + 0.5 - col as f32, y / EDGE + 0.5 - row as f32);
        Self { row, col, x: ease(u), y: ease(v), dx: 4.0 * u * (1.0 - u), dy: 4.0 * v * (1.0 - v) }
    }

    fn sample(&self, plane: &[bool; 9]) -> f32 {
        let i = self.row * 3 + self.col;
        let [a, b, c, d] = [plane[i], plane[i + 1], plane[i + 3], plane[i + 4]];
        if a == b && a == c && a == d {
            return u8::from(a) as f32;
        }
        let [a, b, c, d] = [a, b, c, d].map(|on| u8::from(on) as f32);
        lerp(lerp(a, b, self.x), lerp(c, d, self.x), self.y)
    }

    fn bank_light(&self, plane: &[bool; 9]) -> f32 {
        let i = self.row * 3 + self.col;
        let [a, b, c, d] = [plane[i], plane[i + 1], plane[i + 3], plane[i + 4]]
            .map(|wet| u8::from(wet) as f32);
        // The bank loses height in the direction of increasing depth.
        -0.55 * self.dx * lerp(b - a, d - c, self.y)
            - 0.83 * self.dy * lerp(c - a, d - b, self.x)
    }
}

/// A mostly cubic reconstruction blurs along the bank without shortening
/// narrow creeks into a string of ponds. The two outer taps span the next
/// bend and drop out at cell centers; adjacent tiles see identical taps.
fn coast_kernel(t: f32, blur: f32) -> ([f32; 4], [f32; 4]) {
    let u = 1.0 - t;
    let (t2, u2) = (t * t, u * u);
    let (t3, u3) = (t2 * t, u2 * u);
    let local = ease(t);
    let grade = 6.0 * t * u;
    (
        [u3 * blur / 6.0, lerp(1.0 - local, (4.0 - 6.0 * t2 + 3.0 * t3) / 6.0, blur),
         lerp(local, (4.0 - 6.0 * u2 + 3.0 * u3) / 6.0, blur), t3 * blur / 6.0],
        [-0.5 * u2 * blur, lerp(-grade, -2.0 * t + 1.5 * t2, blur),
         lerp(grade, 2.0 * u - 1.5 * u2, blur), 0.5 * t2 * blur],
    )
}

fn river_coast(mask: u32, x: f32, y: f32, col: usize, row: usize) -> (f32, f32) {
    let profile = terrain_profile(mask, x, y, col, row, (0.81, 0.58));
    (profile.height, -0.54 * profile.dx - 0.87 * profile.dy)
}

struct Profile { height: f32, dx: f32, dy: f32 }

fn terrain_profile(mask: u32, x: f32, y: f32, col: usize, row: usize, blur: (f32, f32)) -> Profile {
    let (xw, xd) = coast_kernel(x / EDGE + 0.5 - col as f32, blur.0);
    let (yw, yd) = coast_kernel(y / EDGE + 0.5 - row as f32, blur.1);
    let (mut height, mut dx, mut dy) = (0.0, 0.0, 0.0);
    let mut taps = mask >> (row * STENCIL + col);
    for j in 0..4 {
        let (mut cross, mut grade) = (0.0, 0.0);
        for i in 0..4 {
            if taps & (1 << i) != 0 {
                cross += xw[i];
                grade += xd[i];
            }
        }
        height += yw[j] * cross;
        dx += yw[j] * grade;
        dy += yd[j] * cross;
        taps >>= STENCIL;
    }
    Profile { height: height.clamp(0.0, 1.0), dx, dy }
}

pub(super) fn paint_surface(tile: &mut Pixmap, land: Land, origin: (f32, f32)) {
    let surface = Surface::new(land);
    raster(tile, &surface, origin);
    if surface.river != FULL {
        growth(tile, &surface, origin);
    }
}

fn raster(tile: &mut Pixmap, surface: &Surface, (ox, oy): (f32, f32)) {
    // Ground pigments vary over logical pixels; sample the expensive color
    // field once per node, then interpolate at native density. Thin grasses
    // and the tactical features themselves are painted at native density.
    let nodes: [[Rgb; NODES]; NODES] = std::array::from_fn(|y| std::array::from_fn(|x| {
        let (x, y) = (x as f32, y as f32);
        surface.color(x, y, ox + x, oy + y)
    }));
    let axis: [(usize, f32); SIZE] = std::array::from_fn(|p| {
        let q = (p as f32 + 0.5) * EDGE / SIZE as f32;
        (q as usize, q.fract())
    });
    for (out, &(y, v)) in tile.data_mut().chunks_exact_mut(SIZE * 4).zip(axis.iter()) {
        let upper = &nodes[y];
        let lower = &nodes[y + 1];
        for (pixel, &(x, u)) in out.chunks_exact_mut(4).zip(axis.iter()) {
            let top = mix(upper[x], upper[x + 1], u);
            let bottom = mix(lower[x], lower[x + 1], u);
            let [r, g, b] = rgb8(mix(top, bottom, v));
            pixel.copy_from_slice(&[r, g, b, 255]);
        }
    }
}

/// Meadow pigment at logical world coordinates, shared by native scenery.
pub(in crate::original) fn meadow_color(x: f32, y: f32) -> [u8; 3] { rgb8(meadow(x, y)) }

fn meadow(x: f32, y: f32) -> Rgb {
    let pasture = value(x / 96.0, y / 58.0, 0x46d3_3892);
    let moss = smooth(0.18, 0.71, pasture);
    let base = mix([89., 116., 58.], [126., 146., 69.], moss);
    add_tex(base, art_pack::grass().detail(x * 2.3, y * 2.3), 0.92)
}

fn sward_density(x: f32, y: f32) -> f32 {
    value((x + y * 0.38) / 13.0, (y - x * 0.23) / 8.0, 0x8394_531a)
}

fn pasture(x: f32, y: f32, nap: f32, form: &MeadowForm) -> Rgb {
    let dry = value((x - y * 0.18) / 55.0, y / 38.0, 0x46d3_3892);
    let sward = smooth(0.26, 0.73, dry * 0.56 + nap * 0.44);
    let grass = mix([92., 118., 59.], [124., 142., 68.], sward);
    let grass = mix(grass, [79., 106., 64.], form.damp * 0.36);
    let straw = smooth(0.58, 0.84, dry) * smooth(0.45, 0.73, nap) * (1.0 - form.damp);
    let base = tone(mix(grass, [144., 140., 79.], straw * 0.42), form.light * 0.52);
    add_tex(base, art_pack::grass().detail(x * 2.3, y * 2.3), 0.94)
}

#[derive(Clone, Copy)]
enum Crop { Earth, Stubble, Rye, Fallow }

struct Plot {
    distance: f32,
    phase: f32,
    crop: Crop,
    hedge: f32,
    edge_light: f32,
}

struct MeadowForm {
    damp: f32,
    light: f32,
    plot: Option<Plot>,
}

fn meadow_form(x: f32, y: f32) -> MeadowForm {
    let bank = value((x + y * 0.22) / 185.0, y / 146.0, 0xcf88_134b);
    let bend = ((x - 990.0) * 0.0063).sin();
    let contour_wave = value(x / 63.0, y / 50.0, 0xbc87_0434);
    let level = y - (452.0 + (x - 1140.0) * 0.18 + bend * 28.0 + bank * 53.0
        + (contour_wave - 0.5) * 13.0);
    let band = ((level + 210.0) / 420.0).floor() as i32;
    let cross = level - band as f32 * 420.0;
    let style = noise(0, band, 0x8eaf_67d3);

    // Ground folds turn along the gentle field terraces. Their height
    // falls away in the hollows and rises into individual low shoulders.
    let shoulder = ((style >> 7) & 31) as f32 - 15.5;
    let (crown, grade) = contour(cross + 51.0 + shoulder - bank * 29.0, 66.0);
    let (dell, gully) = contour(cross - 12.0 - shoulder, 48.0);
    let damp = dell * smooth(0.19, 0.68, bank);
    let light = 540.0 * (grade * (0.17 + bank * 0.72) - gully * 0.32)
        + crown * (bank - 0.35) * 11.0;

    let plot = if cross < -62.0 {
        plot_at(x, cross, band * 2, (-184.0, -86.0 - shoulder), 231.0, 45.0)
    } else if cross > 42.0 {
        plot_at(x, cross, band * 2 + 1, (58.0 + shoulder * 0.5, 167.0), 208.0, 151.0)
    } else {
        None
    };
    MeadowForm { damp, light, plot }
}

fn plot_at(x: f32, cross: f32, band: i32, (high, low): (f32, f32),
    width: f32, stagger: f32) -> Option<Plot>
{
    let along = x + (0.28 + (band & 1) as f32 * 0.23) * cross + stagger + 58.0 * band as f32;
    let index = (along / width).floor() as i32;
    let inside = along - width * index as f32;

    let style = noise(index, band, 0xc155_b7e9);
    let (high, low) = (high + ((style >> 3) & 7) as f32 * 2.2, low - ((style >> 7) & 7) as f32 * 2.6);
    let from_top = cross - high;
    let from_bottom = low - cross;
    let across = from_top.min(from_bottom);
    let bank = value(x / 21.0, cross / 19.0, 0xb664_239f);
    let gap = 9.0 + ((style >> 11) & 7) as f32 * 2.2;
    let end = width - 9.0 - ((style >> 14) & 15) as f32 * 1.6;
    let from_left = inside - gap;
    let from_right = end - inside;
    let ends = from_left.min(from_right);
    let distance = across.min(ends) + (bank - 0.5) * 2.5;
    if distance < -6.0 { return None; }
    let edge_light = if across < ends {
        if from_top < from_bottom { 0.90 } else { -0.90 }
    } else if from_left < from_right { 0.56 } else { -0.56 };

    let boundary = if style & (1 << 18) == 0 { cross - high } else { low - cross };
    let hedge = if boundary.abs() < 5.5 && ends > -0.5 && style & 3 != 1 {
        let scrub = value(x / 19.0, cross / 8.0, 0x66d9_84a3);
        (1.0 - smooth(0.8, 2.8 + 2.4 * scrub, (boundary + (bank - 0.5) * 3.8).abs()))
            * smooth(0.32, 0.64, scrub) * smooth(-0.5, 13.0, ends)
    } else {
        0.0
    };
    let crop = match style & 7 {
        0 | 1 => Crop::Earth,
        2 | 3 => Crop::Rye,
        4 => Crop::Fallow,
        _ => Crop::Stubble,
    };
    let pitch = 3.6 + ((style >> 19) & 7) as f32 * 0.23;
    let slope = ((style >> 23) & 31) as f32 * 0.010 - 0.15;
    let drift = (inside * 0.026).sin() * 0.65 + inside * slope;
    let phase = (cross + drift) / pitch;
    Some(Plot { distance, phase, crop, hedge, edge_light })
}

fn field_meadow(x: f32, y: f32, setback: f32, nap: f32) -> Rgb {
    let form = meadow_form(x, y);
    let mut color = pasture(x, y, nap, &form);
    if let Some(plot) = form.plot.filter(|_| setback > -1.0) {
        let dirt = value(x / 7.0, y / 3.3, 0x7e27_e412) - 0.5;
        // Cultivation can give way to the slope and river margin without
        // creating a new ditch or a road around the edge of the cutout.
        let edge_dist = plot.distance + dirt * 0.85;
        let margin = edge_dist.min(setback);
        let work = smooth(-0.85, 0.75, margin);
        let cut = 1.0 - smooth(0.20, 1.70, (edge_dist + 1.45).abs());
        let sod = 1.0 - smooth(0.32, 2.10, (edge_dist + 3.3).abs());
        let edge = smooth(-1.0, 3.1, setback);
        let ditch = tone([67., 80., 49.], dirt * 17.0 + plot.edge_light * 10.0);
        color = tone(mix(color, ditch, cut * edge * 0.78), sod * edge * (6.0 - plot.edge_light * 6.0));

        if work > 0.0 {
            let rows = smooth(3.0, 6.8, margin);
            let turn = (1.0 - smooth(0.25, 1.48, (margin - 2.15).abs())) * (0.57 + dirt * 0.51);
            let row = (plot.phase + dirt * 0.22).rem_euclid(1.0);
            let furrow = 1.0 - smooth(0.045, 0.26, (row - 0.73).abs());
            let crest = smooth(0.05, 0.25, row) * (1.0 - smooth(0.32, 0.59, row));
            let grain = value((x - y * 0.14) / 4.4, plot.phase / 1.2, 0xe579_54ba);
            let (headland, bed) = match plot.crop {
                Crop::Earth => (
                    [130., 104., 70.],
                    add_tex(
                        tone([118., 87., 58.], crest * 16.0 - furrow * 26.0 + (grain - 0.5) * 14.0),
                        art_pack::earth().detail(x * 2.5, y * 2.5),
                        0.78,
                    ),
                ),
                Crop::Rye => (
                    [122., 114., 66.],
                    add_tex(
                        tone([114., 138., 64.], crest * 14.0 - furrow * 18.0),
                        art_pack::wheat().detail(x * 2.4, y * 2.4),
                        0.55,
                    ),
                ),
                Crop::Fallow => (
                    [116., 112., 66.],
                    add_tex(
                        tone(mix([112., 116., 66.], [116., 136., 65.], nap),
                            (crest * 12.0 - furrow * 16.0) * smooth(0.30, 0.70, nap)),
                        art_pack::grass().detail(x * 2.4, y * 2.4),
                        0.72,
                    ),
                ),
                Crop::Stubble => (
                    [154., 126., 74.],
                    add_tex(
                        tone([176., 145., 76.], crest * 12.0 - furrow * 16.0),
                        art_pack::wheat().detail(x * 2.4, y * 2.4),
                        0.90,
                    ),
                ),
            };
            let worked = tone(mix(headland, bed, rows), dirt * 22.0 + form.light * 0.45 - turn * 20.0
                + (1.0 - rows) * plot.edge_light * 8.0);
            color = mix(color, worked, work);
        }
        color = mix(color, [76., 91., 53.], plot.hedge * smooth(0.5, 4.0, setback) * 0.70);
    }
    color
}

fn ramp(lo: f32, hi: f32, t: f32) -> (f32, f32) {
    let u = ((t - lo) / (hi - lo)).clamp(0.0, 1.0);
    (ease(u), 6.0 * u * (1.0 - u) / (hi - lo))
}

struct Bank {
    depth: f32,
    deposit: f32,
    grain: f32,
}

impl Bank {
    fn new(wet: f32, x: f32, y: f32) -> Self {
        if !(0.005..0.93).contains(&wet) {
            return Self { depth: wet, deposit: 0.5, grain: 0.5 };
        }
        let deposit = value((x + y * 0.16) / 46.0, y / 16.0, 0xea26_63d0);
        let grain = value((x - y * 0.31) / 5.6, y / 2.7, 0xe1c1_051b);
        let sediment = (deposit - 0.5) * 0.19 + (grain - 0.5) * 0.047;
        let depth = wet + sediment * smooth(0.03, 0.31, wet) * (1.0 - smooth(0.64, 0.93, wet));
        Self { depth, deposit, grain }
    }

    fn setback(&self) -> f32 {
        (0.088 - self.depth) * 138.0 + (self.deposit - 0.5) * 6.5
    }

    fn paint(&self, ground: impl FnOnce() -> Rgb, light: f32, x: f32, y: f32) -> Rgb {
        let d = self.depth;
        if d <= 0.01 { return ground(); }
        if d >= 0.515 { return self.channel(x, y); }

        let damp = tone(mix([75., 101., 65.], [115., 125., 75.], self.deposit), (self.grain - 0.5) * 16.0);
        let floodplain = mix(ground(), damp, smooth(0.026, 0.21, d) * 0.94);
        let sand = tone(mix([104., 103., 74.], [170., 150., 106.], self.deposit),
            light * 29.0 + (self.grain - 0.5) * 25.0);
        let bar = smooth(0.28, 0.44, d) * (0.45 + self.deposit * 0.62);
        let land = mix(floodplain, sand, bar);
        let scour = tone([60., 73., 55.], (self.grain - 0.5) * 17.0 + light * 11.0);
        let land = mix(land, scour, smooth(0.435, 0.488, d) * 0.91);
        if d < 0.470 { return land; }
        mix(land, self.channel(x, y), smooth(0.470, 0.515, d))
    }

    fn channel(&self, x: f32, y: f32) -> Rgb {
        let flow = value(x / 130.0, y / 39.0, 0x51a8_719d);
        let drift = y + (flow - 0.5) * 23.0 + x * 0.035;
        let braid = value(x / 86.0, drift / 14.0, 0x7ae5_4112);
        let ripple = value((x - y * 0.4) / 27.0, (drift + braid * 3.0) / 1.65, 0xb732_92f0);
        let streak = smooth(0.70, 0.91, ripple) * smooth(0.45, 0.80, braid);
        let sheen = (flow - 0.5) * 8.0 + (braid - 0.5) * 13.0 + (ripple - 0.5) * 7.0 + streak * 18.0;
        let shoal = mix([84., 126., 108.], [118., 142., 110.], self.deposit);
        let shelf = smooth(0.52, 0.78, self.depth + (braid - 0.5) * 0.05);
        let mid = mix([46., 102., 112.], [68., 128., 134.], flow);
        let channel = mix(mid, [36., 78., 102.], smooth(0.72, 1.0, self.depth) * (1.0 - braid * 0.67));
        let glint = (1.0 - smooth(0.002, 0.016, (self.depth - 0.525).abs()))
            * smooth(0.50, 0.75, ripple);
        let base = tone(mix(shoal, channel, shelf), sheen * 0.7 + glint * 18.0);
        add_tex(base, art_pack::water().detail(x * 2.1, y * 2.1), 0.92)
    }
}

fn shore(ground: impl FnOnce() -> Rgb, water: f32, light: f32, x: f32, y: f32) -> Rgb {
    let bank = value(x / 38.0, y / 19.0, 0xea26_63d0);
    let grain = value(x / 3.7, y / 2.1, 0xe1c1_051b);
    let lip = (bank - 0.5) * 0.14 + (grain - 0.5) * 0.038;
    let depth = water + lip * smooth(0.12, 0.46, water) * (1.0 - smooth(0.62, 0.94, water));
    if depth >= 0.56 {
        return water_color(x, y, depth, bank, grain);
    }
    // The damp flood strip, broken earth face and dark wet foot have
    // different widths. Only the exposed bank takes directional light.
    let flood = tone(mix([74., 93., 62.], [109., 118., 72.], bank), (grain - 0.5) * 16.0);
    let land = mix(ground(), flood, smooth(0.02, 0.23, depth) * 0.95);
    let silt = tone(mix([92., 90., 67.], [155., 137., 94.], bank),
        light * 39.0 + (grain - 0.5) * 33.0);
    let toe = smooth(0.22, 0.39, depth) * (0.44 + bank * 0.56);
    let land = mix(land, silt, toe);
    let wet = tone([53., 68., 54.], (grain - 0.5) * 17.0);
    let land = mix(land, wet, smooth(0.41, 0.51, depth) * 0.94);
    if depth <= 0.48 { return land; }
    mix(land, water_color(x, y, depth, bank, grain), smooth(0.48, 0.56, depth))
}

fn water_color(x: f32, y: f32, depth: f32, bank: f32, grain: f32) -> Rgb {
    let current = value(x / 103.0, y / 27.0, 0x51a8_719d);
    let ripple = value(x / 38.0, (y + current * 5.8) / 2.4, 0xb732_92f0);
    let streak = value(x / 53.0, (y - current * 11.0) / 7.3, 0x7ae5_4112);
    let light = (current - 0.5) * 13.0 + (streak - 0.5) * 11.0
        + (ripple - 0.55) * 9.0 + smooth(0.67, 0.94, ripple) * 17.0;
    let shallows = tone(mix([54., 92., 82.], [94., 118., 88.], bank), light * 0.7 + (grain - 0.5) * 8.0);
    let deep = tone(mix([34., 68., 84.], [48., 90., 104.], current), light * 0.7);
    add_tex(mix(shallows, deep, smooth(0.54, 0.86, depth)), art_pack::water().detail(x * 2.1, y * 2.1), 0.88)
}

/// Opaque water frame for indexed moat layouts, whose simulation/editor has
/// already baked the coast into the frame index rather than a field stencil.
pub(in crate::original) fn water(tile: &mut Pixmap, kind: usize) {
    let (sides, corners) = water_edges(kind);
    let mut stamps = [Stamp { frame: 0xa0, kind: Kind::Water }; 9];
    let ground = Stamp { frame: 0, kind: Kind::Meadow };
    let edges = [[0, 1, 2], [2, 5, 8], [6, 7, 8], [0, 3, 6]];
    for (i, (edge, corner)) in edges.into_iter().zip([2, 8, 6, 0]).enumerate() {
        if sides & (1 << i) != 0 {
            for at in edge { stamps[at] = ground; }
        }
        if corners & (1 << i) != 0 { stamps[corner] = ground; }
    }
    let surface = Surface { field: false, ..Surface::new(Land { stamps, river: 0, relief: None, landings: [None; 2] }) };
    raster(tile, &surface, (0., 0.));
}

fn growth(tile: &mut Pixmap, surface: &Surface, (ox, oy): (f32, f32)) {
    let mut roots = PathBuilder::new();
    let mut blades = PathBuilder::new();
    let mut culms = PathBuilder::new();
    let mut rushes = PathBuilder::new();
    let mut seed_heads = PathBuilder::new();
    let mut brushwood = PathBuilder::new();
    let mut leaves = PathBuilder::new();
    // Stable world roots and a halo keep the fronds and their cast
    // shadows intact as the bank, or the camera, crosses a tile edge.
    const SPACING: i32 = 4;
    const ACROSS: i32 = TILE / SPACING + 2;
    let (gx, gy) = (ox as i32 / SPACING, oy as i32 / SPACING);
    for k in 0..ACROSS * (ACROSS + 2) {
        let (col, row) = (k % ACROSS - 1, k / ACROSS - 1);
        let h = noise(gx + col, gy + row, 0x472f_861b);
        let x = (col * SPACING) as f32 + (h & 255) as f32 * (SPACING as f32 / 256.0);
        let y = (row * SPACING) as f32 + ((h >> 8) & 255) as f32 * (SPACING as f32 / 256.0);
        if !(-4.0..EDGE + 4.0).contains(&x) || !(-1.5..EDGE + 9.1).contains(&y) { continue; }
        let (wx, wy) = (ox + x, oy + y);
        let weights = Weights::new(x, y);
        let wet = surface.coast(x, y, &weights).0;
        if wet > 0.58 { continue; }
        let bank = Bank::new(wet, wx, wy);
        if bank.depth > 0.535 { continue; }
        let tracks = surface.tracks(x, y);
        let road = surface.road_distance(x, y);
        if road < 6.7 || tracks.into_iter().flatten().any(Track::clears_growth) { continue; }
        let choice = ((h >> 16) & 15) as f32;
        if (0.14..0.535).contains(&bank.depth) {
            let patch = value((wx + wy * 0.42) / 26.0, wy / 12.4, 0xca72_a056);
            let mud = (bank.depth - 0.34).abs();
            if patch + bank.deposit * 0.15 > 0.50 + choice * 0.012 + mud * 0.95 {
                let height = 3.1 + choice * 0.24 + patch;
                let lean = ((h >> 21) & 7) as f32 * 0.41 - 1.2;
                let spread = 1.25 + patch * 1.5;
                roots.move_to(x - 0.85, y + 0.05);
                roots.quad_to(x + 0.8, y + 0.42, x + spread, y + 0.85);
                for (offset, tip, tall) in [(-0.64, -0.8, 0.77), (0.12, 0.0, 1.0), (0.66, 0.76, 0.86)] {
                    let end = x + lean + tip;
                    let top = y - height * tall;
                    culms.move_to(x + offset, y);
                    culms.quad_to(x + offset + lean * 0.25, y - height * 0.48, end, top);
                    if tall == 1.0 || h & (1 << 25) == 0 {
                        seed_heads.move_to(end - lean * 0.04, top + 0.17);
                        seed_heads.line_to(end + lean * 0.18, top - 0.83);
                    }
                }
                for (tip, rise) in [(-spread, 0.55), (-spread * 0.64, 0.82), (spread, 0.67), (spread * 0.71, 0.90)] {
                    rushes.move_to(x + tip * 0.23, y + 0.04);
                    rushes.quad_to(x + tip * 0.67, y - height * rise * 0.91,
                        x + tip + lean * 0.54, y - height * rise);
                }
                continue;
            }
        }
        if bank.depth > 0.29 { continue; }

        let ridge = surface.ridge(x, y, (wx, wy), &weights);
        if ridge.face > 0.53 || ridge.talus > 0.73 { continue; }
        let setback = field_setback(&tracks, road, &ridge, weights.sample(&surface.wood), &bank);
        let tuft = sward_density(wx, wy);
        let form = meadow_form(wx, wy);
        if let Some(plot) = form.plot {
            let hedge = plot.hedge * smooth(0.5, 4.0, setback);
            if hedge > 0.26 && tuft > 0.38 {
                let r = 0.85 + hedge * 1.35 + choice * 0.034;
                roots.move_to(x - r, y + 0.20);
                roots.line_to(x + r + 0.46, y + 0.70);
                brushwood.move_to(x - r, y - 0.35);
                brushwood.quad_to(x - 0.25, y - r * 1.7, x + r, y - 0.6);
                leaves.move_to(x - r * 0.82, y - r * 0.60);
                leaves.quad_to(x - 0.50, y - r * 1.65, x + r * 0.66, y - r * 0.95);
                continue;
            }
            if plot.distance.min(setback) > 1.2 { continue; }
        }

        let damp = form.damp * 0.53 + smooth(0.025, 0.22, bank.depth) * 0.53;
        if tuft < 0.65 + choice * 0.016 + smooth(0.0, 29.0, ridge.surface.height) * 0.12 - damp * 0.32 { continue; }
        let height = 0.95 + tuft * 0.84 + damp * 1.4;
        let spread = 0.58 + damp * 0.67;
        roots.move_to(x - spread * 0.51, y + 0.24);
        roots.line_to(x + spread * 1.19, y + 0.58);
        blades.move_to(x - spread, y - height * 0.62);
        blades.quad_to(x - spread * 0.61, y - height * 0.55, x, y);
        blades.quad_to(x + spread * 0.46, y - height * 0.72, x + spread, y - height);
    }
    for (brush, color, width) in [
        (roots, [49, 59, 36, 115], 0.87),
        (blades, [77, 96, 50, 157], 0.49),
        (culms, [47, 69, 41, 235], 0.53),
        (rushes, [120, 139, 73, 218], 0.66),
        (seed_heads, [83, 70, 43, 231], 0.73),
        (brushwood, [54, 76, 43, 209], 1.73),
        (leaves, [114, 137, 69, 180], 0.91),
    ] {
        if let Some(path) = brush.finish() { stroke(tile, &path, color, width); }
    }
}

fn rgb8(color: Rgb) -> [u8; 3] { color.map(|c| (c + 0.5) as u8) }
fn tone(color: Rgb, light: f32) -> Rgb { color.map(|c| c + light) }
fn lerp(a: f32, b: f32, t: f32) -> f32 { a + (b - a) * t }
fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb { std::array::from_fn(|i| lerp(a[i], b[i], t)) }
fn ease(t: f32) -> f32 { t * t * (3.0 - 2.0 * t) }
fn smooth(lo: f32, hi: f32, t: f32) -> f32 { ease(((t - lo) / (hi - lo)).clamp(0.0, 1.0)) }

/// A compact, smooth height lobe and its analytic derivative.
fn contour(distance: f32, radius: f32) -> (f32, f32) {
    let t = distance / radius;
    let h = (1.0 - t * t).max(0.0);
    (h * h, -4.0 * t * h / radius)
}

fn value(x: f32, y: f32, seed: u32) -> f32 {
    let (i, j) = (x.floor() as i32, y.floor() as i32);
    let (u, v) = (ease(x - i as f32), ease(y - j as f32));
    let at = |dx, dy| (noise(i + dx, j + dy, seed) >> 24) as f32 * (1.0 / 255.0);
    lerp(lerp(at(0, 0), at(1, 0), u), lerp(at(0, 1), at(1, 1), u), v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use l2_sim::terrain::{CELLS, build};

    #[test]
    fn bends_are_continuous_between_tiles_and_follow_terrain_edits() {
        let mut layer = [0; CELLS];
        let (x, y) = (30, 40);
        layer[y * DIM + x] = 0x09;
        layer[(y + 1) * DIM + x] = 0x09;
        layer[(y + 1) * DIM + x + 1] = 0x09;
        let mut field = build(&layer, 31);
        let west = Land::at(&field, x, y);
        let east = Land::at(&field, x + 1, y);
        let a = Surface::new(west);
        let b = Surface::new(east);
        let south = Surface::new(Land::at(&field, x, y + 1));
        let (wx, wy) = ((x + 1) as f32 * EDGE, y as f32 * EDGE);
        for i in 0..=TILE {
            let t = i as f32;
            assert_eq!(a.color(EDGE, t, wx, wy + t), b.color(0.0, t, wx, wy + t));
            assert_eq!(a.color(t, EDGE, wx - EDGE + t, wy + EDGE),
                south.color(t, 0.0, wx - EDGE + t, wy + EDGE));
        }
        assert_eq!(Weights::new(HALF, HALF).sample(&a.wet), 1.0);
        assert_eq!(Weights::new(HALF, HALF).sample(&b.wet), 0.0);

        field.cells[y * DIM + x].gfx = 0;
        let changed = Land::at(&field, x + 1, y);
        assert_ne!(east, changed);
        let c = Surface::new(changed);
        assert_ne!(b.color(4.0, HALF, wx + 4.0, wy + HALF), c.color(4.0, HALF, wx + 4.0, wy + HALF));

        let unchanged_center = Land::at(&field, x, y);
        field.cells[y * DIM + x - 2].gfx = 0xa0;
        let updated_halo = Land::at(&field, x, y);
        assert_eq!(unchanged_center.stamps, updated_halo.stamps);
        assert_ne!(unchanged_center.river, updated_halo.river);
    }

    #[test]
    fn shaded_upland_crosses_tile_borders_without_a_light_or_height_step() {
        let (x, y) = (22, 36);
        let mut field = build(&[0; CELLS], 31);
        for (cx, cy) in [(x + 1, y), (x, y + 1), (x + 1, y + 1), (x + 2, y + 1)] {
            let spot = &mut field.cells[cy * DIM + cx];
            spot.gfx = 0x42;
            spot.terrain = id::OBSTACLE;
        }
        let make = |cx, cy| Surface::new(Land::at(&field, cx, cy));
        let (a, b, c) = (make(x, y), make(x + 1, y), make(x, y + 1));
        let (wx, wy) = (x as f32 * EDGE, y as f32 * EDGE);
        let mut height_range = (f32::INFINITY, 0.0_f32);
        for i in 0..=TILE {
            let t = i as f32;
            let h = a.ridge(EDGE, t, (wx + EDGE, wy + t), &Weights::new(EDGE, t)).surface.height;
            let next = b.ridge(0.0, t, (wx + EDGE, wy + t), &Weights::new(0.0, t)).surface.height;
            assert!((h - next).abs() < 0.0001);
            height_range.0 = height_range.0.min(h);
            height_range.1 = height_range.1.max(h);
            let [u, v] = [a.color(t, EDGE, wx + t, wy + EDGE), c.color(t, 0.0, wx + t, wy + EDGE)];
            for (i, j) in u.into_iter().zip(v) {
                assert!((i - j).abs() <= 0.02);
            }
        }
        assert!(height_range.1 - height_range.0 > 4.0);
    }

    #[test]
    fn ridge_material_bands_follow_the_mask_without_an_outside_track() {
        let (col, north, south) = (30, 20, 30);
        let mut field = build(&[0; CELLS], 31);
        for y in north..south {
            for x in col - 2..=col + 2 {
                let spot = &mut field.cells[y * DIM + x];
                spot.gfx = 0x42;
                spot.terrain = id::OBSTACLE;
            }
        }
        let run = RidgeRun::at(&field, col, north + 5);
        let world_y = |t: f32| (north as f32 + (south - north) as f32 * t) * EDGE;
        let sample = |t| run.section(world_y(t)).unwrap();
        let (crest, rear, face) = (sample(CROWN), sample(0.24), sample(0.60));
        assert!(crest.height > rear.height && crest.height > face.height);
        assert!(crest.grade.abs() < 0.001 && face.grade < -rear.grade);

        let wx = col as f32 * EDGE + HALF;
        let material = |t| {
            let wy = world_y(t);
            let row = (wy / EDGE) as usize;
            let y = wy - row as f32 * EDGE;
            Surface::new(Land::at(&field, col, row)).ridge(HALF, y, (wx, wy), &Weights::new(HALF, y))
        };
        let span = world_y(1.0) - world_y(0.0);
        let crest_peak = (0..18).map(|i| material(CROWN + (i as f32 - 14.0) / span).crown)
            .fold(0.0, f32::max);
        assert!(crest_peak > 0.88);
        assert!(material(0.60).face > 0.99);
        assert!(material(0.85).talus > 0.99);

        let outside = Surface::new(Land::at(&field, col, north - 1));
        let (y, wy) = (EDGE - 7.5, north as f32 * EDGE - 7.5);
        let pasture = field_meadow(wx, wy, Bank::new(0.0, wx, wy).setback(), sward_density(wx, wy));
        assert_eq!(outside.color(HALF, y, wx, wy), pasture);

        let interior = Land::at(&field, col, north + 5);
        let spot = &mut field.cells[south * DIM + col];
        spot.gfx = 0x42;
        spot.terrain = id::OBSTACLE;
        let changed = Land::at(&field, col, north + 5);
        assert_eq!(interior.stamps, changed.stamps);
        assert_ne!(interior, changed);
    }

    #[test]
    fn authored_road_remains_continuous_without_procedural_lanes() {
        let (x, y) = (30, 40);
        let mut field = build(&[0; CELLS], 31);
        let (wx, wy) = ((x + 1) as f32 * EDGE, y as f32 * EDGE + HALF);
        let pasture = Surface::new(Land::at(&field, x, y)).color(EDGE, HALF, wx, wy);
        field.cells[y * DIM + x].gfx = 0xe8;
        field.cells[y * DIM + x + 1].gfx = 0xe8;
        let west = Surface::new(Land::at(&field, x, y));
        let east = Surface::new(Land::at(&field, x + 1, y));
        let road = west.color(EDGE, HALF, wx, wy);
        assert_eq!(road, east.color(0.0, HALF, wx, wy));
        assert_ne!(road, pasture);
    }

    #[test]
    fn river_filter_bends_across_cells_without_losing_small_water_features() {
        let mut mask = 0;
        for col in 0..STENCIL {
            for row in (if col < 2 { 1 } else { 3 })..STENCIL {
                mask |= 1 << (row * STENCIL + col);
            }
        }
        let left = river_coast(mask, 0.0, HALF, 0, 1).0;
        let right = river_coast(mask, EDGE - 3.0, HALF, 1, 1).0;
        assert!(left > 0.5 && left < 0.7);
        assert!(right > 0.1 && right < 0.25);
        let pond = river_coast(1 << 12, HALF, HALF, 1, 1).0;
        assert!(pond > 0.56);
    }

    #[test]
    fn approaches_reach_past_the_coast_and_invalidate_with_the_deck() {
        let (x, y) = (30, 40);
        let mut layer = [0; CELLS];
        layer[y * DIM + x] = 0x10;
        layer[(y + 2) * DIM + x] = 0x12;
        layer[(y + 3) * DIM + x] = 0x14;
        let mut field = build(&layer, 31);
        let (col, row) = (x + 1, y - 3);
        let land = Land::at(&field, col, row);
        assert!(land.landings[0].is_some());
        let after = Land::at(&field, col, y + 7);
        assert!(after.landings[1].is_some());
        assert_eq!(Land::at(&field, col, y).bridge_ends(), [true, false]);
        assert_eq!(Land::at(&field, col, y + 4).bridge_ends(), [false, true]);

        let west = Surface::new(land);
        let east = Surface::new(Land::at(&field, col + 1, row));
        let north = Surface::new(Land::at(&field, col, row - 1));
        let (wx, wy) = (col as f32 * EDGE, row as f32 * EDGE);
        for i in 0..=TILE {
            let t = i as f32;
            assert_eq!(west.color(EDGE, t, wx + EDGE, wy + t), east.color(0., t, wx + EDGE, wy + t));
            assert_eq!(west.color(t, 0., wx + t, wy), north.color(t, EDGE, wx + t, wy));
        }

        // The bridge itself is outside the nine-cell coastline stencil.
        field.cells[y * DIM + col].gfx = 0;
        let changed = Land::at(&field, col, row);
        assert_eq!(land.stamps, changed.stamps);
        assert_ne!(land, changed);
        assert_ne!(west.color(HALF, HALF, wx + HALF, wy + HALF),
            Surface::new(changed).color(HALF, HALF, wx + HALF, wy + HALF));
    }

    #[test]
    fn overlapping_frames_follow_the_actual_feature() {
        let (x, y) = (30, 40);
        let mut layer = [0; CELLS];
        layer[y * DIM + x] = 0x14;
        let mut field = build(&layer, 31);
        let end = Land::at(&field, x + 3, y + 1);
        assert_eq!(end.frame(), 0x9f);
        assert_eq!(end.kind(), Kind::Bridge);

        let i = (y + 1) * DIM + x + 3;
        field.cells[i].terrain = id::OBSTACLE;
        assert_eq!(Land::at(&field, x + 3, y + 1).kind(), Kind::Slope);
    }
}
