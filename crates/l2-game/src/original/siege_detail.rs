//! Native siege materials, keyed by the same building and damage codes as the
//! indexed renderer. Terrain colors follow world coordinates, not the viewport.

use std::{cell::RefCell, sync::OnceLock};
use l2_sim::{siege::{self, code}, terrain::{Battlefield, DIM, id}};
use l2_view::{Canvas, detail::SCALE, scene::*};
use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, Rect, Transform};

use super::{art_pack, field_detail::{meadow_color, water}, noise};

const SIDE: usize = TILE as usize * SCALE;
// Roofs and low sun shadows can be visible when their foundations are not.
const PAD: usize = 2;
const COLS: usize = VIEW_COLS + 2 * PAD;
const ROWS: usize = VIEW_ROWS + 2 * PAD;
type Surface = [Tile; COLS * ROWS];
static STONE: [OnceLock<Pixmap>; 256] = [const { OnceLock::new() }; 256];
static WOOD: [OnceLock<Pixmap>; 256] = [const { OnceLock::new() }; 256];
static WATER: [OnceLock<Pixmap>; 70] = [const { OnceLock::new() }; 70];
thread_local! {
    static SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Timber {
    #[default]
    None,
    Curtain,
    Gallery,
    Keep,
    Breach,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct Tile {
    frame: u8,
    ground_bank: bool,
    rubble: u8,
    timber: Timber,
}

struct Scene {
    cam: Camera,
    stone: bool,
    cells: Surface,
    layer: Pixmap,
}

pub(crate) fn draw_siege_detail(canvas: &mut Canvas, field: &Battlefield, cam: Camera, ground: Ground) {
    let stone = ground == Ground::Stone;
    let tile = |i: usize| {
        let x = cam.x as isize + (i % COLS) as isize - PAD as isize;
        let y = cam.y as isize + (i / COLS) as isize - PAD as isize;
        if !(0..DIM as isize).contains(&x) || !(0..DIM as isize).contains(&y) {
            return Tile::default();
        }
        let raw = field.at(x as usize, y as usize);
        let ground_bank = raw.tileset() != 0;
        let timber = if stone { Timber::None }
            else if raw.flags & siege::FLAG_KEEP != 0 { Timber::Keep }
            else if raw.flags & siege::FLAG_WALL != 0 { Timber::Curtain }
            else if raw.surface == siege::SURFACE_COLLAPSED || !ground_bank
                && siege::STRUCTURE_WOOD[raw.gfx as usize * 2] == code::WALL { Timber::Breach }
            else if raw.surface == siege::SURFACE_RAMPART_WALK && !ground_bank
                && !(raw.flags & 1 != 0 && siege::DOCK_FRAMES.contains(&raw.gfx)) { Timber::Gallery }
            else { Timber::None };
        let rubble = if timber == Timber::None && OVERLAY_ELEVATIONS.contains(&raw.elevation)
            && raw.flags & (siege::FLAG_WALL | siege::FLAG_KEEP | siege::FLAG_DRAWBRIDGE) == 0
        { raw.terrain } else { 0 };
        Tile { frame: raw.gfx, ground_bank, rubble, timber }
    };
    SCENE.with_borrow_mut(|cache| {
        if cache.as_ref().is_none_or(|c| c.cam != cam || c.stone != stone || c.cells.iter().enumerate().any(|(i, c)| *c != tile(i))) {
            *cache = Some(paint(cam, stone, std::array::from_fn(tile), plan(field, stone)));
        }
        let scene = cache.as_ref().unwrap();
        let width = scene.layer.width() as usize;
        canvas.blit_detail_opaque(scene.layer.data(), width, ORIGIN_X, ORIGIN_Y, FIELD_CLIP);
    });
}

fn paint(cam: Camera, stone: bool, cells: Surface, plan: Option<Plan>) -> Scene {
    let mut layer = Pixmap::new((SIDE * VIEW_COLS) as u32, (SIDE * VIEW_ROWS) as u32).unwrap();
    let pitch = layer.width() as usize;
    let mut tile = Pixmap::new(SIDE as u32, SIDE as u32).unwrap();
    for i in 0..VIEW_COLS * VIEW_ROWS {
        let (col, row) = (i % VIEW_COLS, i / VIEW_COLS);
        let cell = cells[(row + PAD) * COLS + col + PAD];
        let n = cell.frame as usize;
        let structure = !cell.ground_bank && cell.timber == Timber::None
            && ((if stone { siege::STRUCTURE_STONE[n * 2] } else { siege::STRUCTURE_WOOD[n * 2] }) != 0
                || n != 0 && siege::DOCK_FRAMES.contains(&cell.frame)
                || stone && (siege::DRAWBRIDGE_FRAMES.contains(&cell.frame) || (204..=207).contains(&n)));
        let material = if cell.ground_bank && (0xa0..=0xe5).contains(&n) {
            Some(WATER[n - 0xa0].get_or_init(|| {
                let mut p = Pixmap::new(SIDE as u32, SIDE as u32).unwrap();
                water(&mut p, n);
                p
            }))
        } else if structure {
            let atlas = if stone { &STONE } else { &WOOD };
            Some(atlas[n].get_or_init(|| building(n, stone)))
        } else { None };
        ground_tile(&mut tile, (cam.x + col, cam.y + row), structure, plan.as_ref());
        if let Some(material) = material {
            tile.draw_pixmap(0, 0, material.as_ref(), &tiny_skia::PixmapPaint::default(), Transform::identity(), None);
        }
        if cell.rubble != 0 || cell.timber == Timber::None && cell.ground_bank && (OVERLAY_BASE..=OVERLAY_CAP).contains(&n) {
            rubble(&mut tile, u32::from(cell.rubble) + n as u32);
        }
        for (dy, scan) in tile.data().chunks_exact(SIDE * 4).enumerate() {
            let at = ((row * SIDE + dy) * pitch + col * SIDE) * 4;
            layer.data_mut()[at..at + SIDE * 4].copy_from_slice(scan);
        }
    }
    if !stone {
        timber_fort(&mut layer, &cells, cam, plan.as_ref());
    }
    Scene { cam, stone, cells, layer }
}

fn ground_tile(tile: &mut Pixmap, (x, y): (usize, usize), of_wall: bool, plan: Option<&Plan>) {
    // Sample at logical pixels; the native grid only interpolates materials.
    const N: usize = TILE as usize + 1;
    let origin = ((x * TILE as usize) as f32, (y * TILE as usize) as f32);
    let samples: [[[f32; 3]; N]; N] = std::array::from_fn(|j| std::array::from_fn(|i| {
        let (wx, wy) = (origin.0 + i as f32, origin.1 + j as f32);
        let grass = meadow_color(wx, wy).map(f32::from);
        plan.map_or(grass, |p| p.pigment((wx, wy), grass, of_wall))
    }));
    let blend = |a: f32, b: f32, s: f32| a + (b - a) * s;
    for (i, pixel) in tile.data_mut().chunks_exact_mut(4).enumerate() {
        let (px, py) = (i % SIDE, i / SIDE);
        let pitch = SIDE / (N - 1);
        let (u, v) = (((px % pitch) as f32 + 0.5) / pitch as f32, ((py % pitch) as f32 + 0.5) / pitch as f32);
        let (col, row) = (px / pitch, py / pitch);
        let (nw, ne, sw, se) = (samples[row][col], samples[row][col + 1],
            samples[row + 1][col], samples[row + 1][col + 1]);
        for (c, ch) in pixel[..3].iter_mut().enumerate() {
            let value = blend(blend(nw[c], ne[c], u), blend(sw[c], se[c], u), v);
            *ch = value.clamp(0.0, 255.0) as u8;
        }
        pixel[3] = 255;
    }
}

type Point2 = (f32, f32);

#[derive(Clone, Copy)]
struct Bed {
    center: Point2,
    half: Point2,
}

struct Yard {
    beds: [Bed; 3],
    work: [Point2; 3],
    junction: Point2,
}

struct Plan {
    keep: Point2,
    gate: Point2,
    bounds: (Point2, Point2),
    entrance: bool,
    jambs: Option<(usize, usize)>,
    yard: Option<Yard>,
}

fn plan(field: &Battlefield, stone: bool) -> Option<Plan> {
    let castle = if stone { &siege::STRUCTURE_STONE } else { &siege::STRUCTURE_WOOD };
    let kind = |x, y| {
        let cell = field.at(x, y);
        if cell.tileset() == 0 { castle[cell.gfx as usize * 2] } else { 0 }
    };
    let mut keep = None;
    let mut min = (DIM, DIM);
    let mut max = (0, 0);
    for y in 0..DIM {
        for x in 0..DIM {
            let cell = field.at(x, y);
            if cell.flags & siege::FLAG_KEEP != 0 {
                keep = Some((x, y));
            }
            match kind(x, y) {
                code::KEEP => keep = Some((x, y)),
                code::WALL | code::DRAWBRIDGE => {
                    min.0 = min.0.min(x); min.1 = min.1.min(y);
                    max.0 = max.0.max(x); max.1 = max.1.max(y);
                }
                _ => {}
            }
        }
    }
    let keep = keep?;
    if max.0 <= min.0 + 2 || max.1 <= min.1 + 2 || max.1 + 1 >= DIM {
        return None;
    }
    let south = max.1;
    let (start, end) = (min.0 + 2, max.0 - 2);
    let valid_gap = |x| {
        let cell = field.at(x, south + 1);
        let passage = field.at(x, south);
        passage.tileset() == 0 && passage.gfx == 1 && cell.terrain != id::WATER
    };
    let pick = (start..=end).filter(|&x| kind(x, south) == code::DRAWBRIDGE).min_by_key(|x| x.abs_diff(keep.0))
        .or_else(|| (start..=end).filter(|&x| valid_gap(x)).min_by_key(|x| x.abs_diff(keep.0)));
    let opening = pick.map(|first| {
        let l = (start..first).rev().take_while(|&x| valid_gap(x)).last().unwrap_or(first);
        let r = (first + 1..=end).take_while(|&x| valid_gap(x)).last().unwrap_or(first);
        (l, r)
    });
    // A closed fortress still has a courtyard path, but it ends short of the wall.
    let gate_x = opening.map_or(keep.0 as f32, |(l, r)| (l + r) as f32 * 0.5);
    let units = |(x,y): (usize,usize)| ((x as f32 + 0.5) * TILE as f32, (y as f32 + 0.5) * TILE as f32);
    let (keep, (l, n), (r, s)) = (units(keep), units(min), units(max));
    let gate = ((gate_x + 0.5) * TILE as f32, s);
    let yard = (!stone).then(|| {
        let w = r - l;
        let h = s - n;
        let bed = |x, y, hw, hh| Bed { center: (x, y), half: (hw, hh) };
        Yard {
            beds: [
                bed(l + w * 0.103, n + h * 0.37, (w * 0.035).min(27.0), (h * 0.082).min(48.0)),
                bed(l + w * 0.174, n + h * 0.41, (w * 0.023).min(18.0), (h * 0.061).min(35.0)),
                bed(r - w * 0.108, n + h * 0.42, (w * 0.036).min(28.0), (h * 0.086).min(49.0)),
            ],
            work: [
                (l + w * 0.139, n + h * 0.634),
                (r - w * 0.12, n + h * 0.704),
                (keep.0 - w * 0.14, keep.1 + h * 0.078),
            ],
            junction: (keep.0 * 0.55 + gate.0 * 0.45, keep.1 + (s - keep.1) * 0.53),
        }
    });
    Some(Plan { keep, gate, bounds: ((l, n), (r, s)), entrance: pick.is_some(),
        jambs: opening.filter(|_| !stone).map(|(l, r)| (l - 1, r + 1)), yard })
}

impl Plan {
    fn pigment(&self, q: Point2, grass: [f32; 3], wall: bool) -> [f32; 3] {
        if let Some(yard) = &self.yard {
            return self.timber_ground(q, grass, yard);
        }
        self.stone_ground(q, grass, wall)
    }

    fn timber_ground(&self, q @ (x, y): Point2, grass: [f32; 3], yard: &Yard) -> [f32; 3] {
        let ((l, n), (r, s)) = self.bounds;
        let dx = (l - x).max(x - r);
        let dy = (n - y).max(y - s);
        let edge = dx.max(dy);
        let lit = if dx > dy { if x < l { 0.6 } else { -0.7 } }
            else if y < n { 0.55 } else { -1.0 };
        let fold = (x * 0.055 + y * 0.084).sin() + (x * 0.146 - y * 0.178).cos() * 0.43;
        let drift = (x * 0.018 + y * 0.032).sin() * (x * 0.022 - y * 0.021).cos();
        let ragged = (x * 0.13 + y * 0.20).sin() * 1.5 + drift * 3.2;
        let porch = (self.keep.0, self.keep.1 + 21.0);
        let end = if self.entrance { self.gate } else { (self.gate.0, s - 64.0) };
        let (ax, ay) = self.gate;
        let approach = [(ax - 13.0, ay + 83.0), (ax - 60.0, ay + 240.0)];
        let mut drive = distance(q, porch, yard.junction).min(distance(q, yard.junction, end));
        if self.entrance {
            drive = drive.min(distance(q, self.gate, approach[0])).min(distance(q, approach[0], approach[1]));
        }
        let road = 1.0 - smooth(15.0, 21.0, drive + ragged);
        let lane = |a, b, width| 1.0 - smooth(width - 5.5, width, distance(q, a, b) + ragged);
        let pad = |(cx, cy), (hx, hy)| {
            let (u, v) = ((x - cx) / hx, (y - cy) / hy);
            1.0 - smooth(0.77, 1.06, u * u + v * v + fold * 0.11)
        };

        // The hardpan is continuous beneath the hall apron, workshop, and
        // kitchen-garden headlands. It meets the drive but leaves a grass bailey.
        let [west, east, near] = yard.work;
        let headland = |i: usize| {
            let bed = yard.beds[i];
            (bed.center.0, bed.center.1 + bed.half.1 + 4.0)
        };
        let tracks = lane(porch, near, 28.0).max(lane(near, headland(1), 18.0))
            .max(lane(west, headland(1), 16.0)).max(lane(east, headland(2), 15.0))
            .max(lane(west, yard.junction, 21.0)).max(lane(yard.junction, east, 20.0));
        let turning = pad(yard.junction, (((r - l) * 0.12).clamp(38.0, 76.0), 35.0));
        let work = pad(west, (43.0, 31.0)).max(pad(east, (43.0, 34.0))).max(pad(near, (43.0, 32.0)));
        let apron = pad(porch, (41.0, 29.0))
            .max(lane(headland(0), headland(1), 18.0));
        let mut gardens: f32 = 0.0;
        let mut loam: f32 = 0.0;
        let mut bed_toe: f32 = 0.0;
        for bed in &yard.beds {
            let bx = (x - bed.center.0).abs() - bed.half.0;
            let by = (y - bed.center.1).abs() - bed.half.1;
            let rim = bx.max(by) + bx.min(by).max(0.0) * 0.42 + ragged * 0.42;
            gardens = gardens.max(1.0 - smooth(6.5, 12.0, rim));
            loam = loam.max(1.0 - smooth(-0.8, 1.8, rim));
            let toe = (bx.max(y - bed.center.1 - bed.half.1 - 1.0) + 0.7).abs();
            bed_toe = bed_toe.max((1.0 - smooth(0.7, 3.3, toe))
                * smooth(bed.center.1 + bed.half.1 - 1.0, bed.center.1 + bed.half.1 + 1.0, y));
        }
        let base = ((x - self.keep.0).abs() - 16.2).max((y - self.keep.1 - 3.4).abs() - 12.0);
        let plinth = 1.0 - smooth(-0.2, 4.6, base);
        let path = (1.0 - smooth(5.0, 13.5, (edge + 58.0 + ragged).abs())) * 0.84;
        let verge = 0.58 + 0.36 * smooth(-0.63, 0.60, fold + drift * 0.40);
        let hardpan = (tracks * verge).max(turning * 0.83).max(work * 0.89)
            .max(apron * 0.92).max(gardens * 0.73).max(path);
        let earth = (1.0 - smooth(4.0, 23.0, (edge - 10.0 - ragged * 0.5).abs()))
            * (1.0 - road) * 0.68;
        let dust = hardpan.max(road).max(earth);
        let ruts = (1.0 - smooth(0.9, 3.5, (drive - 7.5 - ragged * 0.32).abs())) * road;
        let hash = noise(x as i32, y as i32, 0xd8ec_12a5);
        let grain = (hash >> 24) as f32 * (1.0 / 255.0) - 0.5;
        let grit = grain * (2.0 + dust * 13.0);
        let fleck = match hash & 63 {
            0..=3 => 21.0 * dust,
            4..=6 => -14.0 * dust,
            _ => 0.0,
        };

        let packed = [142.0, 113.0, 79.0];
        let worn = [175.0, 143.0, 100.0];
        let soil = [111.0, 81.0, 52.0];
        let road_mix = road * (1.0 - apron * 0.56 - work * 0.21);
        let ground: [f32; 3] = std::array::from_fn(|i| {
            let color = grass[i] + (packed[i] - grass[i]) * dust;
            let color = color + (worn[i] - color) * road_mix;
            color + (soil[i] - color) * loam + fold * dust * 12.0 + grit + fleck
                - ruts * (13.0 + fold * 5.0) - plinth * 25.0 - bed_toe * 13.0
        });

        // Cut faces have exposed earth and a turf crest; unlike cast shadows
        // the ditch has a floor and a lit bank on its opposite side.
        let cut = edge - 45.0 + ragged * 0.79;
        let across = cut.abs();
        if across >= 17.1 { return ground; }
        let unbridged = if self.entrance {
            1.0 - (1.0 - smooth(22.0, 29.0, drive)) * smooth(s + 10.0, s + 26.0, y)
        } else { 1.0 };
        let ditch = (1.0 - smooth(9.5, 13.5, across)) * unbridged;
        let floor = 1.0 - smooth(2.0, 5.9, across + cut * 0.16);
        let lip = (1.0 - smooth(0.9, 3.9, (across - 13.2).abs())) * unbridged;
        let along = if dx > dy { y } else { x };
        let ribs = (along * 0.78 + cut * 0.58).sin() * 4.1 + fold * 8.5;
        let face_light = -(1.0 - floor) * cut.signum() * lit;
        let bank = [139.0, 120.0, 86.0];
        let bottom = [79.0, 71.0, 53.0];
        std::array::from_fn(|i| {
            let cut_earth = bank[i] + (bottom[i] - bank[i]) * floor
                + face_light * 26.0 + grain * 17.0 + ribs * (1.0 - floor);
            ground[i] + (cut_earth - ground[i]) * ditch
                + lip * (0.70 + face_light * 0.28) * [20.0, 22.0, 8.0][i]
        })
    }

    fn stone_ground(&self, (x, y): Point2, grass: [f32; 3], wall: bool) -> [f32; 3] {
        let ((left, north), (right, south)) = self.bounds;
        let (dx, dy) = ((left - x).max(x - right), (north - y).max(y - south));
        let border = dx.max(dy);
        let light = if dx > dy { if x < left { 1.0 } else { -0.7 } }
            else if y < north { 0.74 } else { -1.0 };
        let smooth = |a: f32, b: f32, d: f32| {
            let t = ((d - a) / (b - a)).clamp(0.0, 1.0);
            t * t * (3.0 - 2.0 * t)
        };
        let wear = self.wear((x, y));
        let ripple = (x * 0.057 + y * 0.039).sin() + (x * 0.132 - y * 0.104).cos() * 0.45;
        let compacted = (1.0 - smooth(-24.0, 3.0, border)) * (0.52 + ripple * 0.12);
        let drive = smooth(0.14, 0.94, wear);
        let terrace = (1.0 - smooth(9.0, 42.0, (border - 18.0).abs())) * (1.0 - drive);
        let cut_at = border + ripple * 3.5 - 43.0;
        let trench = (1.0 - smooth(4.5, 19.0, cut_at.abs())) * (1.0 - drive);
        let cut_light = trench * cut_at.clamp(-13.0, 13.0) * light * 1.34;
        let shadow = self.wall_shadow((x,y)) * (1.0 - drive * 0.65);
        let (sx, sy) = if x < self.keep.0 {
            (x - (left + 68.0) - ripple * 1.7, y - (north + (south - north) * 0.29))
        } else {
            (right - 68.0 - x + ripple * 1.7, y - (north + (south - north) * 0.38))
        };
        // Low kitchen gardens: use pigment rather than obstacle silhouettes
        // and leave the road from the gate clear.
        let garden = (1.0 - smooth(39.0, 47.0, sx.abs()))
            * (1.0 - smooth(45.0, 60.0, sy.abs()))
            * (1.0 - smooth(0.05, 0.31, wear))
            * (1.0 - smooth(-57.0, -37.0, border));
        let row = ((sx + sy * 0.08) * 0.43).sin();
        let gravel = (x * 0.38 + y * 0.71).sin() * (x * 0.29 - y * 0.62).cos();
        let road = [176.0, 153.0, 117.0];
        std::array::from_fn(|i| {
            let dust = (compacted + drive * 0.79).clamp(0.0, 0.94);
            let mixed = grass[i] + (road[i] - grass[i]) * dust;
            let bed = [104.0, 105.0, 63.0][i] + row * [13.0, 17.0, 6.0][i] + gravel * 2.9;
            let mixed = mixed + (bed - mixed) * garden * 0.91;
            mixed + terrace * [14.0, 11.0, 2.0][i] - trench * [21.0, 27.0, 14.0][i]
                + cut_light + ripple * (1.0 + dust * 2.6) + gravel * dust * 2.2
                - shadow * 27.0 - if wall { 8.0 } else { 0.0 }
        })
    }

    fn wear(&self, q: Point2) -> f32 {
        let (x,y) = q;
        let ((left, north), (right, south)) = self.bounds;
        let middle_y = self.keep.1 + (south - self.keep.1) * 0.55;
        let junction = (self.keep.0 * 0.62 + self.gate.0 * 0.38, middle_y);
        let end = if self.entrance { self.gate } else { (self.gate.0, south - 42.0) };
        let near = |a,b,w: f32| (1.0 - distance(q,a,b) / w).clamp(0.0,1.0);
        let mut lane = near(self.keep,junction,36.0).max(near(junction,end,34.0));
        if self.entrance {
            lane = lane.max(near(self.gate,(self.gate.0 - 46.0, south + 250.0),32.0));
        }
        let quadrangle = near((left + 40.0,middle_y + 20.0),junction,24.0)
            .max(near(junction,(right - 34.0,middle_y - 18.0),25.0));
        let wallwalk = if (north + 6.0..=south - 18.0).contains(&y) {
            (1.0 - ((x - left - 26.0).abs().min((right - x - 23.0).abs()) / 34.0)).max(0.0)
        } else { 0.0 };
        let yard = (1.0 - ((x - self.keep.0).hypot(y - self.keep.1) / 84.0)).max(0.0);
        lane.max(quadrangle * 0.79).max(yard * 0.8).max(wallwalk * 0.52)
    }

    fn wall_shadow(&self, (x,y): Point2) -> f32 {
        let ((l,n),(r,s)) = self.bounds;
        let along_x = (l - 6.0..=r + 22.0).contains(&x);
        let along_y = (n + 3.0..=s + 20.0).contains(&y);
        let band = |a: f32, b| {
            let d = a - b;
            if (0.0..=31.0).contains(&d) { 1.0 - d / 31.0 } else { 0.0 }
        };
        let horizontal = if along_x { band(y,n).max(band(y,s)) } else { 0.0 };
        let vertical = if along_y { band(x,l).max(band(x,r)) } else { 0.0 };
        horizontal.max(vertical)
    }
}

fn smooth(a: f32, b: f32, d: f32) -> f32 {
    let t = ((d - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn distance((x,y): Point2, (ax,ay): Point2, (bx,by): Point2) -> f32 {
    let (dx,dy) = (bx-ax,by-ay);
    let t = (((x-ax)*dx+(y-ay)*dy)/(dx*dx+dy*dy)).clamp(0.0,1.0);
    (x-ax-t*dx).hypot(y-ay-t*dy)
}

type Ink = [u8; 4];
const TARRED: Ink = [63, 46, 34, 255];
const BEAM: Ink = [107, 70, 44, 255];
const SAPWOOD: Ink = [170, 123, 76, 255];

#[derive(Clone, Copy)]
struct Run {
    vertical: bool,
    inward: f32,
}

impl Run {
    fn point(self, (x, y): Point2, along: f32, across: f32) -> Point2 {
        let across = across * self.inward;
        if self.vertical { (x + across, y + along) } else { (x + along, y + across) }
    }

    fn face(self, p: &mut Pixmap, at: Point2, (a, b, c, d): (f32, f32, f32, f32), ink: Ink) {
        let (x1, y1) = self.point(at, a, b);
        let (x2, y2) = self.point(at, c, d);
        rect(p, (x1.min(x2), y1.min(y2), (x1 - x2).abs(), (y1 - y2).abs()), ink);
    }
}

#[derive(Clone, Copy)]
enum Work {
    Empty,
    Wall(Run),
    Platform,
    Corner,
    Jamb,
    Gallery { run: Run, tie: Option<f32>, stairs: bool },
    Hall,
    Breach,
}

fn timber_fort(p: &mut Pixmap, cells: &Surface, cam: Camera, plan: Option<&Plan>) {
    let origin = ((cam.x as i32 - PAD as i32) * TILE, (cam.y as i32 - PAD as i32) * TILE);
    let center = |i: usize| ((origin.0 + (i % COLS) as i32 * TILE + TILE / 2) as f32,
        (origin.1 + (i / COLS) as i32 * TILE + TILE / 2) as f32);
    let local = |(x, y): Point2| (x - cam.x as f32 * TILE as f32, y - cam.y as f32 * TILE as f32);
    let works: [Work; COLS * ROWS] = std::array::from_fn(|i| {
        let tile = cells[i];
        let (x, y) = center(i);
        let mut nearest = Run { vertical: tile.frame == 119, inward: 1.0 };
        let mut corner = false;
        let mut jamb = false;
        if let Some(plan) = plan {
            let ((l, n), (r, s)) = plan.bounds;
            let side = (x - l).abs().min((r - x).abs());
            let end = (y - n).abs().min((s - y).abs());
            nearest = if side < end {
                Run { vertical: true, inward: if x < (l + r) * 0.5 { 1.0 } else { -1.0 } }
            } else {
                Run { vertical: false, inward: if y < (n + s) * 0.5 { 1.0 } else { -1.0 } }
            };
            corner = side == 0.0 && end == 0.0;
            jamb = y == s && plan.jambs.is_some_and(|(a, b)| {
                let x = (x as usize) / TILE as usize;
                x == a || x == b
            });
        }
        match tile.timber {
            Timber::Curtain if jamb => Work::Jamb,
            Timber::Curtain if corner => Work::Corner,
            Timber::Curtain if tile.frame == 120 => Work::Platform,
            Timber::Curtain => Work::Wall(nearest),
            Timber::Gallery => {
                let neighbor = |run: Run, steps| {
                    let (nx, ny) = run.point(((i % COLS) as f32, (i / COLS) as f32), 0.0, steps);
                    if (0.0..COLS as f32).contains(&nx) && (0.0..ROWS as f32).contains(&ny) {
                        cells[ny as usize * COLS + nx as usize].timber
                    } else { Timber::None }
                };
                let left = Run { vertical: true, inward: 1.0 };
                let right = Run { vertical: true, inward: -1.0 };
                let north = Run { vertical: false, inward: 1.0 };
                let south = Run { vertical: false, inward: -1.0 };
                let mut tie = None;
                for steps in [1.0, 2.0] {
                    if let Some(run) = [nearest, left, north, right, south].into_iter().find(|&run| {
                        neighbor(run, -steps) == Timber::Curtain
                            && (steps == 1.0 || neighbor(run, -1.0) == Timber::None)
                    }) {
                        nearest = run;
                        tie = Some(steps * TILE as f32);
                        break;
                    }
                }
                let beat = (if nearest.vertical { y } else { x }) as i32 / TILE;
                let stairs = beat.rem_euclid(7) == 2 && neighbor(nearest, 1.0) == Timber::None;
                Work::Gallery { run: nearest, tie, stairs }
            }
            Timber::Keep => Work::Hall,
            Timber::Breach => Work::Breach,
            Timber::None => Work::Empty,
        }
    });

    // Match the landings to the actual stairs; the supports and shadows pass
    // over these ground marks rather than competing with them.
    for (i, work) in works.iter().enumerate() {
        let at = local(center(i));
        if !in_view(at, 61.0) { continue; }
        match *work {
            Work::Gallery { run, stairs: true, .. } => gallery_landing(p, at, run),
            Work::Hall => hall_landing(p, at),
            _ => {}
        }
    }

    // First lay all shadows on the landscape, not the neighbouring roof or
    // walkway. Ground, framing, decks and towers can then overlap naturally.
    for (i, work) in works.iter().enumerate() {
        let at = local(center(i));
        if !in_view(at, 61.0) { continue; }
        match *work {
            Work::Wall(run) => {
                let a = run.point(at, -16.0, 0.0);
                let b = run.point(at, 16.0, 0.0);
                cast_band(p, a, b, 21.0);
            }
            Work::Corner | Work::Jamb => cast_box(p, at, (13.5, 10.0), 34.0),
            Work::Platform => cast_box(p, at, (14.0, 10.0), 24.0),
            Work::Gallery { run, .. } => {
                cast_band(p, run.point(at, -16.0, -8.0), run.point(at, 16.0, -8.0), 13.0);
            }
            Work::Hall => cast_box(p, at, (17.0, 11.0), 42.0),
            _ => {}
        }
    }
    if let Some(yard) = plan.and_then(|plan| plan.yard.as_ref()) {
        dress_yard(p, yard, local);
    }
    for (i, work) in works.iter().enumerate() {
        if let Work::Gallery { run, tie, stairs } = *work {
            let (x, y) = center(i);
            let at = local((x, y));
            if !in_view(at, 58.0) { continue; }
            let seed = noise(x as i32, y as i32, 0x2367);
            gallery(p, at, run, tie, stairs, seed);
        }
    }
    for (i, work) in works.iter().enumerate() {
        let (x, y) = center(i);
        let at = local((x, y));
        if !in_view(at, 42.0) { continue; }
        let seed = || noise(x as i32, y as i32, 0xbc41);
        match *work {
            Work::Wall(run) => curtain(p, at, run, seed()),
            Work::Corner => guardhouse(p, at, false, seed()),
            Work::Jamb => guardhouse(p, at, true, seed()),
            Work::Platform => platform(p, at, seed()),
            Work::Hall => longhouse(p, at, seed()),
            Work::Breach => breach(p, at, seed()),
            Work::Empty | Work::Gallery { .. } => {}
        }
    }
}

fn offset((x, y): Point2, dx: f32, dy: f32) -> Point2 { (x + dx, y + dy) }

fn gallery_landing(p: &mut Pixmap, at: Point2, run: Run) {
    let verge = [(-10.9, 22.0), (9.1, 22.0), (12.6, 37.0), (6.3, 46.6), (-8.3, 44.9), (-12.6, 35.1)]
        .map(|(a, d)| run.point(at, a, d));
    poly(p, &verge, [146, 114, 78, 222]);
    let toe = run.point(at, 0.0, 29.3);
    let radii = if run.vertical { (4.6, 8.8) } else { (8.5, 3.3) };
    oval(p, offset(toe, 0.4, 0.5), radii, [66, 51, 34, 111]);
    for (a, d, w, h, shade) in [
        (-3.5, 32.0, 3.5, 1.9, 2), (3.5, 34.8, 3.1, 2.1, -12),
        (-1.2, 38.2, 3.2, 1.8, 8), (5.2, 40.0, 1.7, 1.0, -4), (-3.8, 42.3, 2.2, 1.2, -13),
    ] {
        sett(p, run.point(at, a, d), (w, h), shade);
    }
}

fn hall_landing(p: &mut Pixmap, at: Point2) {
    poly(p, &[offset(at, -17.0, 12.8), offset(at, 12.2, 11.7), offset(at, 12.8, 17.7),
        offset(at, -10.9, 18.6), offset(at, -17.4, 16.7)], [80, 63, 46, 132]);
    for (dx, dy, w, h, shade) in [
        (-5.6, 20.4, 4.0, 2.1, 5), (2.9, 20.8, 3.8, 2.3, -3),
        (-6.2, 25.4, 4.3, 2.3, -8), (2.8, 25.6, 3.8, 2.1, 9),
        (-5.1, 30.3, 3.6, 1.9, 6), (3.3, 29.8, 3.2, 1.7, -11),
        (-1.9, 34.4, 3.6, 1.6, -15),
    ] {
        sett(p, offset(at, dx, dy), (w, h), shade);
    }
}

fn sett(p: &mut Pixmap, (x, y): Point2, (w, h): Point2, shade: i16) {
    let stone = [(x - w, y - h * 0.16), (x - w * 0.57, y - h),
        (x + w * 0.47, y - h * 0.87), (x + w, y - h * 0.10),
        (x + w * 0.74, y + h * 0.76), (x - w * 0.48, y + h)];
    poly(p, &stone.map(|q| offset(q, 0.32, 0.62)), [78, 64, 48, 152]);
    poly(p, &stone, timber_tone([156, 143, 112], shade));
    stroke(p, stone[1], stone[2], 0.53, timber_tone([186, 165, 126], shade));
}

fn cast_band(p: &mut Pixmap, a: Point2, b: Point2, reach: f32) {
    for (fade, ink) in [(1.13, [32, 47, 38, 17]), (0.97, [32, 47, 38, 34]),
        (0.72, [32, 47, 38, 47]), (0.34, [32, 47, 38, 48])]
    {
        let d = reach * fade;
        poly(p, &[a, b, offset(b, d * 0.79, d * 0.57), offset(a, d * 0.79, d * 0.57)], ink);
    }
}

fn cast_box(p: &mut Pixmap, (x, y): Point2, (w, h): Point2, reach: f32) {
    oval(p, (x + 2.4, y + 3.1), (w + 2.8, h + 3.5), [32, 47, 38, 65]);
    for (fade, ink) in [(1.13, [32, 47, 38, 18]), (0.94, [32, 47, 38, 26]),
        (0.74, [32, 47, 38, 36]), (0.56, [32, 47, 38, 43]), (0.29, [32, 47, 38, 58])]
    {
        let dy = reach * fade * 0.57;
        let dx = reach * fade * 0.79;
        poly(p, &[(x - w, y - h), (x + w, y - h), (x + w + dx, y - h + dy),
            (x + w + dx, y + h + dy), (x - w + dx, y + h + dy), (x - w, y + h)], ink);
    }
}

fn timber_tone(base: [u8; 3], shade: i16) -> Ink {
    let [r, g, b] = base.map(|ch| (i16::from(ch) + shade).clamp(0, 255) as u8);
    [r, g, b, 255]
}

fn gallery(p: &mut Pixmap, (x, y): Point2, run: Run, tie: Option<f32>, stairs: bool, seed: u32) {
    let at = (x, y);
    let floor = (x, y - 6.0);
    if let Some(depth) = tie {
        let inner = if run.vertical { 11.0 } else { 8.0 };
        // The bank rises beneath the fighting gallery. Raised cross-ties
        // socket into the stakes, with feet below the open inner edge.
        run.face(p, at, (-16.0, -depth + 2.0, 16.0, 12.8), [129, 105, 67, 255]);
        run.face(p, at, (-16.0, -depth + 2.0, 16.0, -10.0), [138, 110, 70, 255]);
        run.face(p, at, (-16.0, 9.7, 16.0, 12.5), [102, 80, 51, 255]);
        for post in [-11.0, 7.0] {
            let a = run.point(floor, post, -depth);
            let b = run.point(floor, post, inner);
            stroke(p, a, b, 3.7, TARRED);
            stroke(p, offset(a, -0.5, -0.7), offset(b, -0.5, -0.7), 1.2, SAPWOOD);
            let foot = run.point(at, post, 12.0);
            stroke(p, b, offset(foot, 0.0, 3.5), 2.6, BEAM);
            let brace = run.point(floor, post + 5.6, 11.5);
            stroke(p, brace, offset(foot, 0.2, 2.5), 1.7, TARRED);
        }
    }
    run.face(p, at, (-16.0, -11.6, 16.0, 12.3), TARRED);
    run.face(p, floor, (-16.0, -12.4, 16.0, 11.5), [102, 73, 50, 255]);
    for board in 0..5 {
        let u = -11.8 + board as f32 * 4.6;
        let n = noise(board, 4, seed);
        let tone = timber_tone([153, 119, 82], (n % 21) as i16 - 10);
        run.face(p, floor, (-16.0, u, 16.0, u + 4.13), tone);
        run.face(p, floor, (-16.0, u, 16.0, u + 0.52), timber_tone([179, 142, 96], (n % 17) as i16 - 8));
        let join = ((n >> 8) % 23) as f32 - 11.0;
        run.face(p, floor, (join, u + 0.5, join + 0.55, u + 4.13), BEAM);
    }
    run.face(p, floor, (-16.0, 10.4, 16.0, 12.4), SAPWOOD);
    for along in [-11.0, 7.0] {
        let foot = run.point(at, along, 11.8);
        let post = offset(foot, 0.0, -11.0);
        stroke(p, foot, post, 2.2, TARRED);
        stroke(p, offset(foot, -0.55, -0.7), offset(post, -0.55, -0.7), 0.75, SAPWOOD);
    }
    if stairs {
        // The staircase runs from the inner rail down into the bailey.
        run.face(p, at, (-9.0, 10.0, 9.0, 32.0), [82, 59, 38, 103]);
        for tread in 0..6 {
            let t = tread as f32;
            let at = (x, y - (6.0 - t));
            let v = 10.0 + t * 3.35;
            run.face(p, at, (-7.5, v, 7.5, v + 4.3), BEAM);
            run.face(p, at, (-7.5, v + 0.18, 7.5, v + 2.6), [170, 131, 87, 255]);
            run.face(p, at, (-7.1, v + 0.35, 7.1, v + 0.95), [190, 153, 106, 255]);
        }
        for along in [-8.0, 8.0] {
            let top = run.point((x, y - 9.0), along, 10.0);
            let end = run.point((x, y - 1.5), along, 29.7);
            stroke(p, top, end, 1.6, BEAM);
        }
    }
}

fn curtain(p: &mut Pixmap, (x, y): Point2, run: Run, seed: u32) {
    if run.vertical {
        flank(p, (x, y), run.inward, seed);
        return;
    }
    let at = (x, y);
    run.face(p, at, (-16.0, -6.0, 16.0, 8.0), [101, 74, 49, 255]);
    let face = |along, across, rise: f32| offset(run.point(at, along, across), 0.0, -rise);
    for i in 0..6 {
        // Sharpened split stakes expose bright sapwood above the bark.
        let along = -16.0 + (i as f32 + 0.5) * (32.0 / 6.0);
        let n = noise(i, 2, seed);
        let h = 16.7 + (n % 13) as f32 * 0.26;
        let (a, b) = (along - 2.69, along + 2.69);
        let front = -4.5 * run.inward;
        let shade = (n >> 6 & 23) as i16 - 11;
        let peak = face(along, front * 0.12, h + 4.8);
        let head = face(a, front, h);
        poly(p, &[head, peak, face(b, front, h), face(b, front, -3.0), face(a, front, -3.0)],
            timber_tone([122, 83, 54], shade));
        poly(p, &[head, peak, face(along + 0.48, front, h - 0.35), face(a + 1.05, front, -2.5),
            face(a + 0.3, front, -2.5), face(a + 0.3, front, h - 0.3)],
            timber_tone([161, 113, 71], shade));
        poly(p, &[head, peak, face(along + 1.25, front, h + 0.1), face(along - 0.1, front, h - 0.95)],
            timber_tone([194, 147, 91], shade));
    }
    let rail = -5.0 * run.inward;
    for height in [4.8, 10.6] {
        let a = face(-16.0, rail, height);
        let b = face(16.0, rail, height);
        stroke(p, a, b, 2.5, BEAM);
        stroke(p, offset(a, 0.0, -1.0), offset(b, 0.0, -1.0), 0.5, SAPWOOD);
    }
    let foot = face(8.4, rail, -3.7);
    let head = face(8.4, rail, 11.7);
    stroke(p, foot, head, 2.7, TARRED);
    stroke(p, offset(foot, -0.7, 0.0), offset(head, -0.7, 0.0), 0.75, SAPWOOD);
}

fn flank(p: &mut Pixmap, (x, y): Point2, inward: f32, seed: u32) {
    // Lay the stake heads in a narrow plane and rake the sides towards
    // their sockets rather than rotating a horizontal wall stamp.
    let lit = if inward < 0.0 { -8 } else { 2 };
    let y0 = y - 26.0;
    rect(p, (x - 7.1, y - 16.0, 18.0, 32.0), [103, 78, 48, 255]);
    for i in 0..6 {
        let n = noise(i, 3, seed);
        let shade = (n % 15) as i16 - 7 + lit;
        let a = y0 + i as f32 * (32.0 / 6.0);
        let b = a + 32.0 / 6.0;
        let point = (x - 5.0 - ((n >> 6) % 4) as f32 * 0.6, a + 2.3);
        poly(p, &[(x + 2.0, a), (x + 2.0, b), (x + 8.65, b + 11.2),
            (x + 8.65, a + 11.2)], timber_tone([119, 81, 51], shade));
        stroke(p, (x + 2.0, b), (x + 8.65, b + 11.2), 0.59, TARRED);
        poly(p, &[point, (x - 1.65, a + 0.08), (x + 2.7, a + 0.55),
            (x + 2.7, b - 0.1), (x - 1.75, b - 0.26)], timber_tone([144, 101, 64], shade));
        poly(p, &[point, (x - 1.65, a + 0.08), (x + 0.85, a + 0.47), (x - 1.6, a + 2.55)],
            timber_tone([197, 151, 90], shade));
    }
    let a = (x + 6.7, y0 + 7.3);
    let b = offset(a, 0.0, 32.0);
    stroke(p, a, b, 2.3, TARRED);
    stroke(p, offset(a, -0.55, 0.0), offset(b, -0.55, 0.0), 0.65, SAPWOOD);
    stroke(p, (x + 1.7, y0 + 21.8), (x + 9.6, y0 + 33.6), 2.6, BEAM);
    stroke(p, (x + 1.2, y0 + 21.8), (x + 9.1, y0 + 33.6), 0.62, SAPWOOD);
}

fn stroke(p: &mut Pixmap, a: Point2, b: Point2, width: f32, ink: Ink) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length = dx.hypot(dy);
    if length < f32::EPSILON { return; }
    let (u, v) = (dy * width / (2.0 * length), -dx * width / (2.0 * length));
    poly(p, &[offset(a, u, v), offset(b, u, v), offset(b, -u, -v), offset(a, -u, -v)], ink);
}

fn platform(p: &mut Pixmap, (x, y): Point2, seed: u32) {
    timber_front(p, (x, y), 13.7, -13.0, 8.0, seed);
    poly(p, &[(x - 14.8, y - 12.0), (x - 10.0, y - 23.0), (x + 15.8, y - 23.0),
        (x + 15.0, y - 11.0)], [129, 95, 61, 255]);
    for i in 0..5 {
        let rise = i as f32 * 1.85;
        stroke(p, (x - 13.7 + rise * 0.4, y - 12.5 - rise),
            (x + 14.0, y - 12.5 - rise), 0.6, SAPWOOD);
    }
    for post in [-13.5, 12.0] {
        rect(p, (x + post, y - 25.6, 2.8, 19.5), BEAM);
        rect(p, (x + post, y - 25.6, 1.0, 18.6), SAPWOOD);
    }
    rect(p, (x - 14.7, y - 16.0, 29.3, 4.3), BEAM);
    rect(p, (x - 14.7, y - 16.0, 29.3, 0.9), SAPWOOD);
}

fn guardhouse(p: &mut Pixmap, (x, y): Point2, jamb: bool, seed: u32) {
    let at = (x, y);
    let w = if jamb { 12.8 } else { 13.9 };
    let eave = if jamb { -26.0 } else { -21.0 };
    rect(p, (x - 15.95, y - 1.0, 31.9, 11.5), [90, 74, 51, 255]);
    rect(p, (x - 15.95, y - 1.0, 31.9, 2.4), [136, 106, 69, 255]);
    timber_front(p, at, w, eave + 1.0, 8.5, seed);
    for side in [-1.0, 1.0] {
        stroke(p, (x + side * 1.9, y + 5.9), (x + side * (w - 1.4), y - 10.9), 2.0, BEAM);
        stroke(p, (x + side * 1.9, y + 4.7), (x + side * (w - 1.4), y - 12.1), 0.55, SAPWOOD);
    }
    rect(p, (x - w - 1.0, y - 12.4, w * 2.0 + 2.0, 2.4), TARRED);
    rect(p, (x - w - 1.0, y - 13.0, w * 2.0 + 2.0, 0.9), SAPWOOD);
    for dx in [-6.1, 5.0] {
        rect(p, (x + dx - 0.85, y + eave + 6.0, 2.5, 6.2), [58, 40, 28, 255]);
        rect(p, (x + dx - 1.3, y + eave + 12.0, 3.4, 0.85), SAPWOOD);
    }
    if jamb {
        gable(p, (x, y + eave), w + 2.0, 13.8, [122, 88, 58], seed);
    } else {
        let a = offset(at, -15.65, eave + 1.0);
        let b = offset(at, 15.65, eave - 0.3);
        let c = offset(at, 10.1, eave - 13.0);
        let d = offset(at, -11.7, eave - 12.0);
        let ridge = [offset(at, -5.2, eave - 17.0), offset(at, 4.8, eave - 17.4)];
        poly(p, &[d, c, ridge[1], ridge[0]], [115, 93, 65, 255]);
        shingles(p, [a, b, ridge[1], ridge[0]], [125, 89, 60], seed);
        poly(p, &[b, c, ridge[1]], [88, 62, 44, 255]);
        poly(p, &[a, ridge[0], d], [153, 117, 75, 255]);
        stroke(p, ridge[0], ridge[1], 1.05, [171, 130, 86, 255]);
        stroke(p, a, b, 1.4, TARRED);
    }
}

fn timber_front(p: &mut Pixmap, (x, y): Point2, w: f32, top: f32, bottom: f32, seed: u32) {
    let side = w * 0.23;
    let x0 = x - w;
    let x1 = x + w - side;
    let a = y + top;
    let b = y + bottom;
    rect(p, (x0, a, x1 - x0, b - a), [102, 72, 48, 255]);
    poly(p, &[(x1, a), (x + w, a - 4.7), (x + w, b - 4.7), (x1, b)],
        [88, 60, 40, 255]);
    for k in 0..7 {
        let at = x0 + (k as f32 + 0.02) * (x1 - x0) / 7.0;
        let shade = (noise(k, 8, seed) % 22) as i16 - 11;
        rect(p, (at, a, (x1 - x0) / 7.0 - 0.31, b - a - 0.8), timber_tone([143, 105, 68], shade));
        rect(p, (at + 0.16, a + 0.5, 0.32, b - a - 1.4), timber_tone([174, 127, 81], shade));
    }
    for dx in [x0, x1 - 1.3] {
        rect(p, (dx, a, 1.9, b - a), BEAM);
        rect(p, (dx, a, 0.66, b - a), SAPWOOD);
    }
    rect(p, (x0, b - 1.7, x1 - x0, 1.7), BEAM);
}

fn longhouse(p: &mut Pixmap, (x, y): Point2, seed: u32) {
    let at = (x, y);
    // A stone plinth fixes the timber hall to its objective cell. Only its
    // upper walls and eaves overhang the surrounding flat forecourt.
    poly(p, &[(x - 16.3, y + 8.5), (x + 11.9, y + 8.5), (x + 16.5, y + 4.7),
        (x + 16.5, y + 10.9), (x + 11.9, y + 15.8), (x - 16.3, y + 15.8)], [99, 93, 78, 255]);
    for row in 0..2 {
        for i in 0..7 {
            let left = x - 16.15 + i as f32 * 4.0 + row as f32 * 1.75;
            let shade = (noise(i, row, seed) % 19) as i16 - 8;
            rect(p, (left, y + 10.6 + row as f32 * 2.55, (x + 11.8 - left).min(3.7), 2.05),
                timber_tone([146, 136, 106], shade));
        }
    }
    timber_front(p, at, 15.4, -13.2, 10.9, seed);
    for dx in [-10.1, 7.2] {
        rect(p, (x + dx - 0.3, y - 4.1, 3.7, 5.4), [71, 47, 33, 255]);
        rect(p, (x + dx - 0.65, y + 0.75, 4.4, 0.8), SAPWOOD);
    }
    poly(p, &[(x - 5.4, y + 13.3), (x - 5.4, y + 1.7), (x - 2.7, y - 1.7),
        (x + 1.2, y - 1.7), (x + 4.0, y + 1.8), (x + 4.0, y + 13.3)], [54, 42, 31, 255]);
    stroke(p, (x - 5.45, y + 11.0), (x - 5.45, y + 1.55), 1.1, SAPWOOD);
    for i in 0..3 {
        let t = i as f32;
        rect(p, (x - 6.2 - t * 0.51, y + 12.9 + t * 2.55, 11.3 + t * 1.1, 2.7),
            timber_tone([163, 150, 121], -9 * i));
        rect(p, (x - 6.2 - t * 0.51, y + 12.9 + t * 2.55, 11.3 + t * 1.1, 0.55), [202, 184, 148, 255]);
    }
    gable(p, offset(at, 0.0, -11.2), 19.4, 23.6, [104, 83, 67], seed);
}

fn gable(p: &mut Pixmap, at: Point2, w: f32, height: f32, tone: [u8; 3], seed: u32) {
    let a = offset(at, -w, 1.9);
    let b = offset(at, w * 0.76, 0.0);
    let c = offset(at, w, -height * 0.63);
    let d = offset(at, -w * 0.75, -height * 0.48);
    let ridge = [offset(at, -w * 0.87, -height * 0.85), offset(at, w * 0.9, -height)];
    poly(p, &[d, c, ridge[1], ridge[0]], timber_tone(tone, -13));
    poly(p, &[b, c, ridge[1]], timber_tone(tone, -23));
    for k in 1..7 {
        let t = k as f32 / 7.0;
        stroke(p, lerp(b, ridge[1], t), lerp(c, ridge[1], t), 0.45, timber_tone(tone, -6));
    }
    shingles(p, [a, b, ridge[1], ridge[0]], tone, seed);
    stroke(p, a, ridge[0], 1.6, timber_tone(tone, 44));
    stroke(p, ridge[0], ridge[1], 1.4, timber_tone(tone, 41));
    stroke(p, b, ridge[1], 1.6, timber_tone(tone, 26));
    stroke(p, a, b, 1.5, TARRED);
}

fn lerp((x, y): Point2, (u, v): Point2, t: f32) -> Point2 {
    (x + (u - x) * t, y + (v - y) * t)
}

fn shingles(p: &mut Pixmap, [a, b, c, d]: [Point2; 4], tone: [u8; 3], seed: u32) {
    poly(p, &[a, b, c, d], timber_tone(tone, -22));
    let rows = ((d.1 - a.1).abs() / 2.3).round() as i32;
    let cols = ((b.0 - a.0).abs() / 3.9).round() as i32;
    for row in 0..rows {
        let t0 = row as f32 / rows as f32;
        let t1 = (row as f32 + 0.87) / rows as f32;
        let (nw, ne) = (lerp(a, d, t1), lerp(b, c, t1));
        let (sw, se) = (lerp(a, d, t0), lerp(b, c, t0));
        for col in 0..=cols {
            let u = col as f32 - (row % 2) as f32 * 0.5;
            let u0 = (u / cols as f32).max(0.0);
            let u1 = ((u + 0.94) / cols as f32).min(1.0);
            if u0 >= u1 { continue; }
            let n = noise(col, row, seed);
            let shade = ((n >> 4) % 29) as i16 - 9 + row as i16 - col as i16;
            let (nw, ne, sw, se) = (lerp(nw, ne, u0), lerp(nw, ne, u1), lerp(sw, se, u0), lerp(sw, se, u1));
            poly(p, &[nw, ne, se, sw], timber_tone(tone, shade));
            stroke(p, sw, se, 0.36, timber_tone(tone, shade + 15));
        }
    }
}

fn breach(p: &mut Pixmap, (x, y): Point2, seed: u32) {
    for k in 0..6 {
        let n = noise(k, 5, seed);
        let a = (x + (n % 25) as f32 - 12.0, y + ((n >> 8) % 18) as f32 - 6.0);
        let b = offset(a, ((n >> 16) % 7) as f32 - 2.0, -3.3 - ((n >> 21) % 4) as f32);
        stroke(p, offset(a, 1.4, 1.2), offset(b, 2.0, 2.0), 2.7, [53, 44, 31, 117]);
        stroke(p, a, b, 2.5, BEAM);
        stroke(p, offset(a, -0.4, -0.45), offset(b, -0.4, -0.45), 0.85, SAPWOOD);
    }
}

fn dress_yard(p: &mut Pixmap, yard: &Yard, local: impl Fn(Point2) -> Point2) {
    for (i, bed) in yard.beds.iter().enumerate() {
        let at = local(bed.center);
        if in_view(at, 62.0) { kitchen_bed(p, at, bed.half, i as u32 + 0x29); }
    }
    for (i, &work) in yard.work.iter().enumerate() {
        let at = local(work);
        if in_view(at, 46.0) { working_ground(p, at, i); }
    }
}

fn in_view((x, y): Point2, margin: f32) -> bool {
    (-margin..VIEW_COLS as f32 * TILE as f32 + margin).contains(&x)
        && (-margin..VIEW_ROWS as f32 * TILE as f32 + margin).contains(&y)
}

fn kitchen_bed(p: &mut Pixmap, (x, y): Point2, (w, h): Point2, seed: u32) {
    let rows = (w / 5.2).ceil() as i32;
    let plants = (h / 5.5).ceil() as i32;
    if rows < 2 || plants < 2 { return; }
    let spacing = 2.0 * w / (rows + 1) as f32;
    // Sunlit shoulders of the furrows and dark flanks are laid before the
    // separate crop rows, with a deliberate walking gap between beds.
    for row in 0..rows {
        let bx = x - w + (row as f32 + 1.0) * spacing;
        let wav = (noise(row, 0, seed) % 7) as f32 * 0.29;
        poly(p, &[(bx - 2.6, y - h + 4.1), (bx + 0.5 + wav, y - h + 1.7), (bx + 2.4, y + h - 4.2),
            (bx + 0.2, y + h - 1.2), (bx - 1.2 + wav, y + h * 0.4)],
            [151, 110, 68, 185]);
        stroke(p, (bx + 2.9, y - h + 4.0), (bx + 2.9 - wav, y + h - 3.1), 1.75, [74, 60, 39, 144]);
        for i in 0..plants {
            let n = noise(row, i, seed);
            let px = bx + ((n % 13) as f32 - 6.0) * 0.20;
            let py = y - h + 4.0 + (2.0 * h - 8.0) * (i as f32 + 0.5) / plants as f32
                + (((n >> 8) % 11) as f32 - 5.0) * 0.19;
            let delta = ((n >> 18) % 19) as i16 - 7;
            let roots = row % 2 == 1;
            oval(p, (px + 0.9, py + 0.8), (2.6, 1.7), [45, 52, 30, 111]);
            oval(p, (px, py), (2.15, 1.8), timber_tone(if roots { [84, 100, 55] } else { [74, 102, 54] }, delta));
            oval(p, (px - 0.55, py - 0.7), (1.2, 0.8),
                timber_tone(if roots { [129, 138, 65] } else { [113, 132, 78] }, delta));
        }
    }
    // Flush edging stays visually low; the bottom end is open to the track.
    stroke(p, (x - w, y + h * 0.44), (x - w, y - h + 1.6), 1.55, [103, 75, 43, 200]);
    stroke(p, (x - w - 0.45, y + h * 0.44), (x - w - 0.45, y - h + 1.6), 0.55, [185, 152, 91, 208]);
    stroke(p, (x - w, y - h + 1.6), (x + w, y - h + 1.6), 1.3, [167, 122, 70, 215]);
}

fn working_ground(p: &mut Pixmap, (x, y): Point2, kind: usize) {
    let at = (x, y);
    if kind == 0 || kind == 1 {
        // Short hewn stock laid on damp ground along the edge of a bare work
        // pad. The footprint is flat, with open space toward the footpaths.
        let run = if kind == 0 { (24.0, -9.0) } else { (11.0, 23.0) };
        for row in 0..5 {
            let j = row as f32;
            let a = if kind == 0 { offset(at, -29.5 + j * 1.9, -6.8 + j * 4.2) }
                else { offset(at, 17.0 - j * 4.2, -14.0 + j * 1.4) };
            let b = offset(a, run.0 + (j - 2.0) * 1.2, run.1);
            stroke(p, offset(a, 1.55, 2.1), offset(b, 1.55, 2.1), 4.8, [75, 58, 39, 124]);
            stroke(p, a, b, 3.6, timber_tone([117, 78, 46], 3 * row));
            stroke(p, offset(a, -0.25, -0.9), offset(b, -0.25, -0.9), 1.0, SAPWOOD);
            oval(p, a, (1.63, 1.45), [196, 151, 94, 255]);
            oval(p, a, (0.74, 0.66), [163, 113, 68, 255]);
        }
    }
    let stores: &[Point2] = match kind {
        0 => &[(20.0, -13.0), (26.0, -9.0), (24.0, -17.0)],
        1 => &[(-21.0, -5.0), (-20.5, -13.2), (-27.0, -9.4)],
        _ => &[(-11.0, -4.0), (-17.8, -5.6), (-14.4, -12.1)],
    };
    for &(a, b) in stores { barrel(p, offset(at, a, b)); }
    let stone_x = if kind == 1 { -13.0 } else if kind == 2 { 11.5 } else { 14.0 };
    for i in 0..8 {
        let n = noise(i, kind as i32, 0x5a6d);
        let pos = offset(at, stone_x + ((n >> 4) % 16) as f32 - 6.0, ((n >> 14) % 12) as f32 + 2.0);
        let long = 1.65 + (n % 7) as f32 * 0.34;
        oval(p, offset(pos, 0.9, 0.75), (long, 1.38), [92, 80, 56, 134]);
        oval(p, pos, (long * 0.87, 1.42), timber_tone([157, 145, 110], (n % 21) as i16 - 10));
    }
}

fn barrel(p: &mut Pixmap, (x, y): Point2) {
    oval(p, (x + 1.55, y + 1.1), (4.15, 2.15), [69, 53, 37, 133]);
    oval(p, (x, y - 0.8), (3.1, 2.4), BEAM);
    rect(p, (x - 3.1, y - 4.1, 6.2, 3.2), BEAM);
    rect(p, (x - 1.9, y - 4.1, 1.9, 3.8), [155, 105, 60, 255]);
    oval(p, (x, y - 4.35), (3.1, 1.58), [172, 126, 74, 255]);
    oval(p, (x, y - 4.45), (2.25, 0.92), [116, 78, 47, 255]);
    stroke(p, (x - 2.8, y - 1.8), (x + 2.8, y - 1.8), 0.5, TARRED);
}

fn oval(p: &mut Pixmap, (x, y): Point2, (w, h): Point2, [r, g, b, a]: Ink) {
    let Some(path) = Rect::from_xywh(x - w, y - h, w * 2.0, h * 2.0).and_then(PathBuilder::from_oval) else { return };
    let mut paint = Paint::default();
    paint.set_color_rgba8(r, g, b, a);
    p.fill_path(&path, &paint, FillRule::Winding, transform(), None);
}

fn rubble(tile: &mut Pixmap, seed: u32) {
    for i in 0..3 {
        let x = 7.0 + 8.0 * i as f32;
        let y = 19.0 + (noise(i, 0, seed) % 9) as f32;
        poly(tile, &[(x - 2.7, y + 1.0), (x - 2.2, y - 1.4), (x, y - 3.7), (x + 2.8, y - 1.8),
            (x + 3.4, y + 0.6), (x + 1.0, y + 1.9)], [120, 119, 103, 255]);
        poly(tile, &[(x - 2.2, y - 1.4), (x, y - 3.7), (x + 2.8, y - 1.8), (x - 0.6, y - 0.4)], [172, 169, 146, 255]);
    }
}

fn building(n: usize, stone: bool) -> Pixmap {
    let mut p = Pixmap::new(SIDE as u32, SIDE as u32).unwrap();
    let kind = if stone { siege::STRUCTURE_STONE[n * 2] } else { siege::STRUCTURE_WOOD[n * 2] };
    if stone && siege::DRAWBRIDGE_FRAMES.contains(&(n as u8)) {
        if n != 197 {
            water(&mut p, 0xa0);
            let x = if n == 198 { 0.0 } else { 6.0 + (n as f32 - 192.0) * 4.0 };
            deck(&mut p, (x, 0., 33. - x, 32.), false, n);
        }
        return p;
    }
    let dock = n != 0 && siege::DOCK_FRAMES.contains(&(n as u8));
    if dock || kind == code::DRAWBRIDGE {
        deck(&mut p, (0., 3., 32., 26.), false, n);
        rect(&mut p, (0., 5., 32., 1.1), [190, 150, 101, 217]);
        rect(&mut p, (0., 25., 32., 1.4), [82, 59, 41, 217]);
    } else if stone && kind == code::WALL {
        if n == 173 {
            rect(&mut p, (12., 0., 15., 32.), [40, 49, 36, 113]);
            deck(&mut p, (7.5, 0., 16., 32.), true, n);
            for y in [0.5, 11.0, 21.5] {
                rect(&mut p, (6.2, y, 3.2, 6.), [199, 188, 162, 255]);
            }
        } else {
            rect(&mut p, (0., 25., 32., 7.), [38, 47, 37, 124]);
            deck(&mut p, (0., 8.4, 32., 20.), true, n);
            for x in [0., 11., 22.] {
                deck(&mut p, (x, 5.5, 7.3, 6.8), true, n);
            }
        }
    } else if stone && kind == code::KEEP {
        rect(&mut p, (3.8, 13., 28.2, 17.6), [35, 48, 39, 132]);
        deck(&mut p, (2., 5., 27., 22.), true, n);
        rect(&mut p, (12., 18., 6., 9.), [48, 42, 37, 255]);
    } else if kind == 1 || stone && (204..=207).contains(&n) {
        rect(&mut p, (-1., 0., 34., 32.), [150, 136, 101, 179]);
        for k in 0..9 {
            let h = noise(k, 19, n as u32);
            let (x, y) = ((h & 31) as f32, ((h >> 8) & 31) as f32);
            rect(&mut p, (x, y, 1.4, 0.73), [193, 177, 142, 176]);
        }
    } else if kind != 0 {
        deck(&mut p, (0., 0., 32., 32.), stone, n);
        if kind == code::TEN {
            rect(&mut p, (1., 1., 30., 24.), [84, 102, 104, 211]);
        }
    }
    p
}

fn deck(p: &mut Pixmap, (x, y, w, h): (f32, f32, f32, f32), stone: bool, seed: usize) {
    let tex = if stone { art_pack::masonry() } else { art_pack::planks() };
    let x0 = (x.max(0.0) * SCALE as f32).floor() as usize;
    let y0 = (y.max(0.0) * SCALE as f32).floor() as usize;
    let x1 = ((x + w).min(TILE as f32) * SCALE as f32).ceil() as usize;
    let y1 = ((y + h).min(TILE as f32) * SCALE as f32).ceil() as usize;
    if x0 >= x1 || y0 >= y1 {
        return;
    }
    let pw = p.width() as usize;
    let ph = p.height() as usize;
    let data = p.data_mut();
    let ox = ((seed * 37) & 255) as i32;
    let oy = ((seed * 53) & 255) as i32;
    for py in y0..y1.min(ph) {
        let vy = (py - y0) as f32 / (y1 - y0).max(1) as f32;
        // Crisp high-overhead sunlit top coping and darker bottom bevel
        let shade = if py - y0 < 3 {
            1.14
        } else if y1 - py <= 3 {
            0.76
        } else {
            1.06 - vy * 0.14
        };
        let row = py * pw;
        for px in x0..x1.min(pw) {
            let side_shade = if px - x0 < 2 { 1.07 } else if x1 - px <= 2 { 0.84 } else { 1.0 };
            let s = tex.sample_u8(px as i32 * 2 + ox, py as i32 * 2 + oy);
            let k = shade * side_shade;
            let di = (row + px) * 4;
            data[di] = (f32::from(s[0]) * k).clamp(0.0, 255.0) as u8;
            data[di + 1] = (f32::from(s[1]) * k).clamp(0.0, 255.0) as u8;
            data[di + 2] = (f32::from(s[2]) * k).clamp(0.0, 255.0) as u8;
            data[di + 3] = 255;
        }
    }
}

fn transform() -> Transform { Transform::from_scale(SCALE as f32, SCALE as f32) }

fn rect(p: &mut Pixmap, (x, y, w, h): (f32, f32, f32, f32), [r,g,b,a]: [u8;4]) {
    let Some(rct) = Rect::from_xywh(x,y,w,h) else { return };
    let mut paint = Paint::default();
    paint.set_color_rgba8(r,g,b,a);
    p.fill_rect(rct, &paint, transform(), None);
}

fn poly(p: &mut Pixmap, points: &[(f32,f32)], [r,g,b,a]: [u8;4]) {
    let mut bld = PathBuilder::new();
    bld.move_to(points[0].0,points[0].1);
    for &(x,y) in &points[1..] { bld.line_to(x,y); }
    bld.close();
    let mut paint = Paint::default();
    paint.set_color_rgba8(r,g,b,a);
    p.fill_path(&bld.finish().unwrap(),&paint,FillRule::Winding,transform(),None);
}
