//! Original, deterministic indexed-pixel artwork. The legacy container is used
//! only as a drawing interface: no publisher assets are inputs to these routines.

mod battle;
mod campaign;
mod cursors;
mod shell;
mod tableau;
mod world;

pub(crate) use world::{minimaps, COUNTY_NAMES, MAP_NAMES};
pub(crate) use cursors::pictures;
pub(crate) use tableau::tableau;

pub(crate) fn asset(name: &str) -> Option<Vec<u8>> {
    let name = name.to_ascii_lowercase();
    match name.as_str() {
        "l2_maps.dat" => Some(world::maps()),
        "base01.256" => Some(palette()),
        // The generated sheets share the same indexed palette. The original
        // renderers still request it by each screen's resource name.
        name if crate::shell::PALETTES.iter().any(|file| file.eq_ignore_ascii_case(name)) => {
            Some(palette())
        }
        name => campaign::asset(name).or_else(|| battle::asset(name)).or_else(|| shell::asset(name)),
    }
}

const SLATE: u8 = 8;
const STONE: u8 = 16;
const PARCHMENT: u8 = 24;
const GOLD: u8 = 32;
const WOOD: u8 = 40;
const GRASS: u8 = 48;
const PINE: u8 = 56;
const WATER: u8 = 64;
const ICE: u8 = 72;
const RED: u8 = 80;
const BLUE: u8 = 88;
const VIOLET: u8 = 96;
const COPPER: u8 = 104;
const IRON: u8 = 112;
const CREAM: u8 = 120;
const INK: u8 = 128;
const MOSS: u8 = 136;

fn palette() -> Vec<u8> {
    // This format stores six-bit components, not RGB888. Start with a neutral
    // ramp so legacy UI pen numbers outside the authored ramps remain legible.
    let mut bytes = vec![0; 256 * 3];
    for (i, rgb) in bytes.chunks_exact_mut(3).enumerate() {
        rgb.fill((i / 4) as u8);
    }
    let ramps = [
        (SLATE, [12, 22, 34], [145, 177, 188]),
        (STONE, [28, 29, 30], [190, 187, 163]),
        (PARCHMENT, [65, 48, 33], [244, 224, 173]),
        (GOLD, [65, 38, 15], [255, 216, 112]),
        (WOOD, [29, 15, 12], [162, 109, 63]),
        (GRASS, [22, 47, 36], [146, 165, 86]),
        (PINE, [9, 29, 28], [95, 140, 95]),
        (WATER, [8, 31, 49], [125, 195, 191]),
        (ICE, [40, 70, 87], [234, 244, 240]),
        (RED, [48, 14, 21], [239, 112, 87]),
        (BLUE, [15, 30, 55], [100, 167, 239]),
        (VIOLET, [30, 19, 49], [191, 125, 196]),
        (COPPER, [61, 24, 21], [235, 153, 91]),
        (IRON, [24, 33, 41], [220, 224, 215]),
        (CREAM, [77, 55, 43], [255, 245, 215]),
        (INK, [8, 11, 16], [69, 74, 77]),
        (MOSS, [29, 37, 25], [168, 175, 102]),
    ];
    for (base, low, high) in ramps {
        for shade in 0..8 {
            let at = (base as usize + shade) * 3;
            for c in 0..3 {
                bytes[at + c] = ((low[c] * (7 - shade) + high[c] * shade) / 28) as u8;
            }
        }
    }
    // The classic minimap and sidebar use fixed indices for realm pens.
    for (i, source) in [(1, RED+4), (3, GOLD+3), (4, BLUE+3), (5, VIOLET+4),
        (0xF0, BLUE+7), (0xF1, BLUE+5), (0xF2, GOLD+4), (0xF3, GOLD+6),
        (0xF4, VIOLET+5), (0xF5, VIOLET+6), (0xF9, RED+7), (0xFB, GOLD+7),
        (0xFC, BLUE+4), (0xFD, VIOLET+7), (0xFE, INK), (0xFF, CREAM+7)] {
        let dst = i * 3;
        let src = source as usize * 3;
        bytes.copy_within(src..src+3, dst);
    }
    bytes
}

struct Art {
    w: i32,
    h: i32,
    pixels: Vec<u8>,
}

impl Art {
    fn new(w: i32, h: i32) -> Self {
        Self { w, h, pixels: vec![0; (w * h) as usize] }
    }

    fn dot(&mut self, x: i32, y: i32, c: u8) {
        if (0..self.w).contains(&x) && (0..self.h).contains(&y) {
            self.pixels[(y * self.w + x) as usize] = c;
        }
    }

    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: u8) {
        let left = x.max(0);
        let right = (x + w).min(self.w);
        if left < right {
            for y in y.max(0)..(y + h).min(self.h) {
                let row = y * self.w;
                self.pixels[(row + left) as usize..(row + right) as usize].fill(c);
            }
        }
    }

    fn line(&mut self, mut x: i32, mut y: i32, bx: i32, by: i32, c: u8) {
        let dx = (bx - x).abs();
        let dy = -(by - y).abs();
        let sx = if x < bx { 1 } else { -1 };
        let sy = if y < by { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            self.dot(x, y, c);
            if (x, y) == (bx, by) { break; }
            let e = 2 * err;
            if e >= dy { err += dy; x += sx; }
            if e <= dx { err += dx; y += sy; }
        }
    }

    fn ellipse(&mut self, cx: i32, cy: i32, rx: i32, ry: i32, c: u8) {
        let (a, b) = (i64::from(rx).pow(2), i64::from(ry).pow(2));
        let mut dx = rx;
        for dy in 0..=ry {
            while dx > 0 && i64::from(dx).pow(2) * b + i64::from(dy).pow(2) * a > a * b {
                dx -= 1;
            }
            self.rect(cx - dx, cy + dy, dx * 2 + 1, 1, c);
            if dy != 0 { self.rect(cx - dx, cy - dy, dx * 2 + 1, 1, c); }
        }
    }

    fn poly(&mut self, points: &[(i32, i32)], c: u8) {
        let Some(top) = points.iter().map(|p| p.1).min() else { return };
        let bottom = points.iter().map(|p| p.1).max().unwrap();
        let mut crossings = Vec::with_capacity(points.len());
        for y in top.max(0)..bottom.min(self.h) {
            for (&(ax, ay), &(bx, by)) in points.iter().zip(points.iter().cycle().skip(1)) {
                if (ay > y) != (by > y) {
                    crossings.push(ax + (y - ay) * (bx - ax) / (by - ay));
                }
            }
            crossings.sort_unstable();
            for pair in crossings.chunks_exact(2) {
                self.rect(pair[0], y, pair[1] - pair[0] + 1, 1, c);
            }
            crossings.clear();
        }
        for (&(ax, ay), &(bx, by)) in points.iter().zip(points.iter().cycle().skip(1)) {
            self.line(ax, ay, bx, by, c);
        }
    }
}

fn noise(x: i32, y: i32, seed: u32) -> u32 {
    let mut n = (x as u32).wrapping_mul(0x9e37_79b1) ^ (y as u32).wrapping_mul(0x85eb_ca6b) ^ seed;
    n = (n ^ (n >> 16)).wrapping_mul(0x7feb_352d);
    n = (n ^ (n >> 15)).wrapping_mul(0x846c_a68b);
    n ^ (n >> 16)
}

fn sheet(frames: impl IntoIterator<Item = Art>) -> Vec<u8> {
    use l2_formats::pl8::{FRAME_RECORD_LEN, HEADER_LEN};
    let frames: Vec<_> = frames.into_iter().collect();
    let header = HEADER_LEN + FRAME_RECORD_LEN * frames.len();
    let mut out = Vec::with_capacity(header + frames.iter().map(|f| f.pixels.len()).sum::<usize>());
    out.resize(header, 0);
    out[2..4].copy_from_slice(&(frames.len() as u16).to_le_bytes());
    for (i, f) in frames.into_iter().enumerate() {
        let at = HEADER_LEN + FRAME_RECORD_LEN * i;
        let start = out.len() as u32;
        out[at..at + 2].copy_from_slice(&(f.w as u16).to_le_bytes());
        out[at + 2..at + 4].copy_from_slice(&(f.h as u16).to_le_bytes());
        out[at + 4..at + 8].copy_from_slice(&start.to_le_bytes());
        out.extend(f.pixels);
    }
    out
}
