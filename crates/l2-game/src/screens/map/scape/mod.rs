//! Native-resolution relief for standalone campaign worlds. The indexed map still
//! owns the interaction plane; this painter works from the same cells and state.

mod art;
mod atlas;

use std::ops::Range;
use std::sync::OnceLock;

use l2_formats::maps::{MapSlot, Plane};
use l2_kingdom::map::{self, flags, CampaignMap, SiteState, MAP_DIM, MAP_TILES};
use l2_view::campaign::{self, Fog, Lattice, Overrides, Viewport, Zoom};
use l2_view::detail::SCALE;
use l2_view::{Canvas, Clip};
use tiny_skia::Pixmap;

const FOG: Rgb = [24, 35, 40];
const SIDES: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];
const EDGE: [(f32, f32); 4] = [(15.0, -7.5), (15.0, 7.5), (-15.0, 7.5), (-15.0, -7.5)];
const NEAR_STEPS: i32 = 32;
type Rgb = [u8; 3];
type Point = (f32, f32);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Key {
    pub slot: usize,
    pub zoom: u8,
    pub view: Viewport,
    pub turn: u32,
    pub season: u8,
    pub fields: u64,
    pub castles: u64,
    pub sites: u64,
    pub fog: u64,
}

pub(super) struct Cache {
    key: Key,
    land: LandState,
    atlas: atlas::Atlas,
    active: Vec<Motion>,
}

#[derive(Clone, Copy)]
struct Motion {
    tile: (usize, usize),
    center: (i32, i32),
    site: Site,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Site {
    Quarry,
    Forge,
    Logging,
    Mine,
}

impl Site {
    fn of_frame(frame: u8) -> Option<Self> {
        match frame {
            0..=9 => Some(Self::Quarry),
            10..=19 => Some(Self::Forge),
            20..=29 => Some(Self::Logging),
            30..=46 => Some(Self::Mine),
            _ => None,
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw(
    cache: &mut Option<Cache>,
    canvas: &mut Canvas,
    key: Key,
    zoom: &Zoom,
    slot: &MapSlot<'_>,
    lattice: &Lattice,
    map: &CampaignMap,
    graphics: &Overrides,
    fog: Fog<'_>,
) {
    // The indexed layer still needs the view in its key. Here the camera is
    // only a translation of the historically staggered lattice.
    let scene_key = Key { view: WORLD_VIEW, ..key };
    if !cache.as_ref().is_some_and(|c| c.key == scene_key) {
        let land = LandState::new(map, lattice, fog);
        let active = active_sites(&land.borrow(map, lattice), slot, graphics, zoom);
        *cache = Some(Cache { key: scene_key, land, atlas: atlas::Atlas::new(), active });
    }
    let cache = cache.as_mut().expect("scenery cache initialized");
    let land = cache.land.borrow(map, lattice);
    let shift = (key.view.col * zoom.pitch, key.view.row * zoom.row_step);
    let clip = zoom.clip();
    cache.atlas.ensure(clip, shift, |region| {
        paint_scene(scene_key, zoom, slot, &land, graphics, region)
    });
    canvas.enable_detail();
    cache.atlas.blit(canvas, clip, shift);
    for &motion in &cache.active {
        let (rx, ry) = motion_radius(zoom);
        let origin = (motion.center.0 - shift.0 - rx, motion.center.1 - shift.1 - ry);
        if origin.0 >= clip.x1 || origin.0 + rx * 2 <= clip.x0
            || origin.1 >= clip.y1 || origin.1 + ry * 2 <= clip.y0
        {
            continue;
        }
        let Some((_, frame)) = graphics.get(motion.tile.0, motion.tile.1) else { continue };
        let Some(stamp) = motion_stamp(zoom, motion.site, frame) else { continue };
        canvas.blit_detail(stamp.data(), stamp.width() as usize, origin.0, origin.1, clip);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Ground {
    Water,
    Meadow,
    Wood,
    Hill,
    Fog,
}

struct LandState {
    grounds: [Ground; MAP_TILES],
    tracks: [u8; MAP_TILES],
    exploration: bool,
}

impl LandState {
    fn new(map: &CampaignMap, lattice: &Lattice, fog: Fog<'_>) -> Self {
        let grounds = std::array::from_fn(|t| {
            let (x, y) = map::coords(t);
            let f = map.flags[t];
            if fog.is_some_and(|hidden| hidden(x as usize, y as usize)) {
                Ground::Fog
            } else if f & flags::NO_COUNTY != 0 {
                Ground::Water
            } else if f & flags::ROUGH == 0 {
                Ground::Meadow
            } else if map.is_mountain(t) {
                Ground::Hill
            } else {
                Ground::Wood
            }
        });
        let mut state = Self { grounds, tracks: [0; MAP_TILES], exploration: fog.is_some() };
        for t in 0..MAP_TILES {
            if map.flags[t] & flags::ROAD != 0 && state.grounds[t] != Ground::Fog {
                let (x, y) = map::coords(t);
                if state.borrow(map, lattice).bridge(i32::from(x), i32::from(y)).is_some() {
                    state.grounds[t] = Ground::Water;
                }
            }
        }
        state.tracks = state.borrow(map, lattice).access_tracks();
        state
    }

    fn borrow<'a>(&'a self, map: &'a CampaignMap, lattice: &'a Lattice) -> Land<'a> {
        Land { map, lattice, grounds: &self.grounds, tracks: &self.tracks, exploration: self.exploration }
    }
}

struct Land<'a> {
    map: &'a CampaignMap,
    lattice: &'a Lattice,
    grounds: &'a [Ground; MAP_TILES],
    tracks: &'a [u8; MAP_TILES],
    exploration: bool,
}

impl Land<'_> {
    fn at(&self, x: i32, y: i32) -> Ground {
        if let Some(t) = index(x, y) {
            return self.grounds[t];
        }
        if self.exploration {
            return Ground::Fog;
        }
        let row = x + y + 1;
        let col = (x - y + MAP_DIM as i32) >> 1;
        if (6..=21).contains(&self.lattice.surround(row, col)) { Ground::Meadow } else { Ground::Water }
    }

    fn field(&self, x: i32, y: i32) -> bool {
        index(x, y).is_some_and(|t| {
            self.grounds[t] != Ground::Fog && self.map.flags[t] & flags::NO_COUNTY == 0
                && self.map.flags[t] & flags::FARMLAND != 0
                && self.map.terrain[t] != 0
        })
    }

    fn farm_kind(&self, x: i32, y: i32) -> Option<u8> {
        let t = index(x, y)?;
        (self.grounds[t] == Ground::Meadow
            && self.map.flags[t] & flags::FARMLAND != 0
            && self.map.flags[t] & (flags::ROAD | flags::CASTLE | flags::SETTLEMENT) == 0)
            .then_some(match self.map.terrain[t] {
                0 => 0,
                0x0f..=0x16 => 2,
                _ => 1,
            })
    }

    fn parcel(&self, x: i32, y: i32, seed: u32) -> Option<Parcel> {
        let kind = self.farm_kind(x, y)?;
        let tile = index(x, y)?;
        let county = self.map.county[tile];
        let adjacent = SIDES.map(|(dx, dy)| {
            let n = index(x + dx, y + dy)?;
            (self.map.county[n] == county).then(|| self.farm_kind(x + dx, y + dy)).flatten()
        });
        (kind != 0 || adjacent.iter().flatten().any(|&n| n != 0)).then(|| Parcel {
            state: self.map.terrain[tile],
            joined: adjacent.map(|other| other == Some(kind)),
            gates: std::array::from_fn(|side| {
                let (dx, dy) = SIDES[side];
                index(x + dx, y + dy).is_some_and(|n| self.tracks[n] & (1 << ((side + 2) % 4)) != 0)
            }),
            turned: county & 1 == 0,
            seed: hash(x, y, seed),
        })
    }

    fn access_tracks(&self) -> [u8; MAP_TILES] {
        // Walk outward from the roads and village lanes. Only retain branches
        // that serve a site or reach the edge of a working field.
        const REACH: u8 = 5;
        const NO_SIDE: u8 = 4;
        let mut links = [0; MAP_TILES];
        let mut distance = [u8::MAX; MAP_TILES];
        let mut toward = [NO_SIDE; MAP_TILES];
        let mut queue = Vec::with_capacity(MAP_TILES);
        for (t, d) in distance.iter_mut().enumerate() {
            if self.grounds[t] == Ground::Meadow
                && self.map.flags[t] & (flags::ROAD | flags::CASTLE) != 0
                && self.map.county[t] != 0
            {
                *d = 0;
                queue.push(t);
            }
        }
        let mut head = 0;
        while let Some(&t) = queue.get(head) {
            head += 1;
            if distance[t] == REACH || distance[t] != 0 && self.map.flags[t] & flags::SETTLEMENT != 0 {
                continue;
            }
            let (x, y) = map::coords(t);
            for (side, (dx, dy)) in SIDES.into_iter().enumerate() {
                let (nx, ny) = (i32::from(x) + dx, i32::from(y) + dy);
                let Some(n) = index(nx, ny) else { continue };
                let site = self.map.flags[n] & flags::SETTLEMENT != 0;
                let ground = self.grounds[n];
                let approach = ground == Ground::Meadow
                    || site && matches!(ground, Ground::Hill | Ground::Wood);
                if approach && distance[n] == u8::MAX
                    && self.map.county[n] == self.map.county[t] && !self.field(nx, ny)
                {
                    distance[n] = distance[t] + 1;
                    toward[n] = ((side + 2) % 4) as u8;
                    queue.push(n);
                }
            }
        }
        let connect = |mut t: usize, links: &mut [u8; MAP_TILES]| {
            while toward[t] != NO_SIDE {
                let side = toward[t] as usize;
                if links[t] & (1 << side) != 0 {
                    break;
                }
                links[t] |= 1 << side;
                let (x, y) = map::coords(t);
                let (dx, dy) = SIDES[side];
                t = map::index((i32::from(x) + dx) as u8, (i32::from(y) + dy) as u8);
                links[t] |= 1 << ((side + 2) % 4);
            }
        };
        let mut approached = [[false; 2]; 256];
        for t in queue {
            let (x, y) = map::coords(t);
            let (x, y) = (i32::from(x), i32::from(y));
            let county = self.map.county[t];
            if self.map.flags[t] & flags::SETTLEMENT != 0 {
                connect(t, &mut links);
                continue;
            }
            for (side, (dx, dy)) in SIDES.into_iter().enumerate() {
                let (nx, ny) = (x + dx, y + dy);
                let Some(field) = index(nx, ny) else { continue };
                let Some(kind @ 1..=2) = self.farm_kind(nx, ny) else { continue };
                let visited = &mut approached[county as usize][kind as usize - 1];
                if *visited || self.map.county[field] != county {
                    continue;
                }
                // Route one entrance to each working crop/pasture network
                // per county, never across the crop itself.
                let connected = SIDES.into_iter().any(|(a, b)| {
                    index(nx + a, ny + b).is_some_and(|n| self.map.county[n] == county)
                        && self.farm_kind(nx + a, ny + b) == Some(kind)
                });
                if connected {
                    *visited = true;
                    links[t] |= 1 << side;
                    connect(t, &mut links);
                }
            }
        }
        for (t, link) in links.iter_mut().enumerate() {
            if self.grounds[t] != Ground::Meadow || self.map.flags[t] & flags::CASTLE == 0 {
                continue;
            }
            let (x, y) = map::coords(t);
            for (side, (dx, dy)) in SIDES.into_iter().enumerate() {
                if self.field(i32::from(x) + dx, i32::from(y) + dy) {
                    *link |= 1 << side;
                }
            }
        }
        links
    }

    fn road_links(&self, x: i32, y: i32) -> [bool; 4] {
        SIDES.map(|(dx, dy)| {
            index(x + dx, y + dy).is_some_and(|t| {
                self.grounds[t] != Ground::Fog
                    && self.map.flags[t] & (flags::ROAD | flags::CASTLE) != 0
            })
        })
    }

    fn works(&self, x: i32, y: i32) -> Works {
        let Some(t) = index(x, y) else { return Works::default() };
        if matches!(self.grounds[t], Ground::Fog | Ground::Water) {
            return Works::default();
        }
        let mask = |sides: [bool; 4]| {
            sides.into_iter().enumerate().fold(0, |bits, (i, side)| bits | (u8::from(side) << i))
        };
        let f = self.map.flags[t];
        let kind = f & (flags::CASTLE | flags::SETTLEMENT);
        let foundation = (kind != 0).then(|| mask(SIDES.map(|(dx, dy)| {
            index(x + dx, y + dy).is_some_and(|n| {
                !matches!(self.grounds[n], Ground::Fog | Ground::Water)
                    && self.map.county[n] == self.map.county[t] && self.map.flags[n] & kind != 0
            })
        })));
        let links = if f & (flags::ROAD | flags::CASTLE) != 0 {
            mask(self.road_links(x, y))
        } else {
            0
        };
        let road = (f & flags::ROAD != 0).then_some(links);
        let tracks = self.tracks[t] | if f & flags::CASTLE != 0 { links } else { 0 };
        Works { road, tracks, foundation }
    }

    fn bridge(&self, x: i32, y: i32) -> Option<usize> {
        let nearby = SIDES.map(|(dx, dy)| index(x + dx, y + dy));
        if nearby.iter().any(|t| t.is_some_and(|t| self.grounds[t] == Ground::Fog)) {
            return None;
        }
        let wet = nearby.map(|t| t.is_none_or(|t| self.map.flags[t] & flags::NO_COUNTY != 0));
        let links = self.road_links(x, y);
        (0..2).find(|&axis| links[axis] && links[axis + 2] && wet[1 - axis] && wet[3 - axis])
    }
}

fn index(x: i32, y: i32) -> Option<usize> {
    ((0..MAP_DIM as i32).contains(&x) && (0..MAP_DIM as i32).contains(&y))
        .then(|| y as usize * MAP_DIM + x as usize)
}

fn new_image(w: i32, h: i32) -> Pixmap {
    Pixmap::new(w as u32 * SCALE as u32, h as u32 * SCALE as u32)
        .expect("campaign clip has valid dimensions")
}

const WORLD_VIEW: Viewport = Viewport::new(0, 0);

fn world_center(zoom: &Zoom, row: i32, col: i32) -> (i32, i32) {
    let (x, y) = campaign::cell_to_screen(WORLD_VIEW, zoom, row, col);
    (x + zoom.half_pitch, y + zoom.row_step)
}

fn cells_in(clip: Clip, zoom: &Zoom, (dx, dy): (i32, i32)) -> impl Iterator<Item = (i32, i32)> + '_ {
    let top = (clip.y0 - zoom.top() - dy).div_euclid(zoom.row_step);
    let bottom = (clip.y1 - zoom.top() + dy).div_euclid(zoom.row_step);
    (top..=bottom).flat_map(move |row| {
        let (x, _) = world_center(zoom, row, 0);
        let left = (clip.x0 - x - dx).div_euclid(zoom.pitch);
        let right = (clip.x1 - x + dx).div_euclid(zoom.pitch);
        (left..=right).map(move |col| (row, col))
    })
}

fn frame_at(slot: &MapSlot<'_>, graphics: &Overrides, x: usize, y: usize) -> (u8, u8) {
    let (bank, frame) = graphics.get(x, y)
        .unwrap_or_else(|| (slot.at(Plane::GfxBank, x, y), slot.at(Plane::GfxIndex, x, y)));
    ((bank & campaign::BANK_MASK) >> 2, frame)
}

fn industry(map: &CampaignMap, tile: usize, frame: u8) -> Option<(Site, bool, bool)> {
    let site = Site::of_frame(frame)?;
    let state = map::industry_state(map.terrain[tile]).map(|(_, state)| state);
    let wrecked = state == Some(SiteState::Wrecked) || matches!(frame, 9 | 19 | 29 | 46);
    Some((site, wrecked, !wrecked && state == Some(SiteState::Working)))
}

fn active_sites(land: &Land<'_>, slot: &MapSlot<'_>, graphics: &Overrides, zoom: &Zoom) -> Vec<Motion> {
    let mut sites: Vec<_> = (0..MAP_TILES).filter_map(|t| {
        if land.grounds[t] == Ground::Fog
            || land.map.flags[t] & (flags::SETTLEMENT | flags::CASTLE) != flags::SETTLEMENT
        {
            return None;
        }
        let (x, y) = map::coords(t);
        let tile = (x as usize, y as usize);
        let (bank, frame) = frame_at(slot, graphics, tile.0, tile.1);
        if bank != 3 {
            return None;
        }
        let (site, _, working) = industry(land.map, t, frame)?;
        let (row, col) = campaign::tile_to_cell(tile.0, tile.1);
        working.then(|| Motion { tile, center: world_center(zoom, row, col), site })
    }).collect();
    sites.sort_unstable_by_key(|motion| (motion.center.1, motion.center.0));
    sites
}

fn motion_radius(zoom: &Zoom) -> (i32, i32) {
    (zoom.half_pitch / 2, zoom.row_step / 2 + 1)
}

fn motion_stamp(zoom: &Zoom, site: Site, frame: u8) -> Option<&'static Pixmap> {
    const FRAMES: usize = 47;
    static STAMPS: [[OnceLock<Pixmap>; FRAMES]; 2] =
        [const { [const { OnceLock::new() }; FRAMES] }; 2];
    if Site::of_frame(frame)? != site {
        return None;
    }
    Some(STAMPS[zoom.set][frame as usize].get_or_init(|| {
        let (rx, ry) = motion_radius(zoom);
        let mut stamp = new_image(rx * 2, ry * 2);
        // Motion pigments are season-independent; clip each pose to its own
        // diamond so a working site can never overpaint an unexplored neighbor.
        art::Painter::new(&mut stamp, zoom, (rx, ry), 1).motion(site, frame);
        let width = stamp.width() as usize;
        let (hw, hh) = (zoom.half_pitch as usize * SCALE, zoom.row_step as usize * SCALE);
        let (ox, oy) = (width - 1, stamp.height() as usize - 1);
        for (i, pixel) in stamp.data_mut().chunks_exact_mut(4).enumerate() {
            let x = (2 * (i % width)).abs_diff(ox);
            let y = (2 * (i / width)).abs_diff(oy);
            if x * hh + y * hw >= 2 * hw * hh {
                pixel.fill(0);
            }
        }
        stamp
    }))
}

fn paint_scene(
    key: Key,
    zoom: &Zoom,
    slot: &MapSlot<'_>,
    land: &Land<'_>,
    graphics: &Overrides,
    clip: Clip,
) -> Pixmap {
    let map = land.map;
    let mut image = new_image(clip.width(), clip.height());
    let fog_runs = paint_ground(&mut image, land, key, zoom, clip);
    let cell = |(row, col)| {
        let (x, y) = land.lattice.tile(row, col)?;
        let t = map::index(x as u8, y as u8);
        (land.grounds[t] != Ground::Fog).then(|| (x, y, t, world_center(zoom, row, col)))
    };
    let frame_at = |x, y| frame_at(slot, graphics, x, y);
    // Dry routes and woodland are part of the terrain projection; bridges and
    // survey stones are surface objects on the original interaction lattice.
    let ground = cells_in(clip, zoom, (zoom.half_pitch, 2 * zoom.row_step));
    for (x, y, t, (px, py)) in ground.filter_map(cell) {
        let f = map.flags[t];
        let bridge = f & flags::ROAD != 0 && land.grounds[t] == Ground::Water;
        if f & flags::NO_COUNTY != 0 || f & flags::BOUNDARY == 0 && !bridge {
            continue;
        }
        let (x, y) = (x as i32, y as i32);
        let seed = hash(x, y, key.slot as u32);
        let mut painter = art::Painter::new(&mut image, zoom, (px - clip.x0, py - clip.y0), key.season);
        if f & flags::BOUNDARY != 0 {
            for (i, &(dx, dy)) in SIDES.iter().enumerate() {
                let Some(n) = index(x + dx, y + dy) else { continue };
                if map.county[n] != 0 && map.county[n] < map.county[t]
                    && !matches!(land.grounds[n], Ground::Fog | Ground::Water)
                {
                    painter.boundary(EDGE[i], seed.rotate_left(i as u32 * 5));
                }
            }
        }
        if bridge {
            if let Some(axis) = land.bridge(x, y) {
                painter.bridge(EDGE[axis]);
            }
        }
    }
    // Draw in the original back-to-front cell order, including foundations
    // outside this region whose walls, towers or shadows extend into it.
    let structures = cells_in(clip, zoom, (4 * zoom.half_pitch, 5 * zoom.row_step));
    for (x, y, t, (px, py)) in structures.filter_map(cell) {
        let f = map.flags[t];
        if f & (flags::CASTLE | flags::SETTLEMENT) == 0 {
            continue;
        }
        let mut painter = art::Painter::new(&mut image, zoom, (px - clip.x0, py - clip.y0), key.season);
        let seed = hash(x as i32, y as i32, key.slot as u32);
        let (bank, frame) = frame_at(x, y);
        if f & flags::CASTLE != 0 {
            let same = |dx: i32, dy: i32| {
                index(x as i32 + dx, y as i32 + dy).is_some_and(|n| {
                    map.flags[n] & flags::CASTLE != 0 && map.county[n] == map.county[t]
                })
            };
            let part = same(-1, 0) as usize + 2 * same(0, -1) as usize;
            let density = frame.saturating_sub(47).saturating_sub(slot.at(Plane::GfxIndex, x, y)) / 4;
            painter.town(part, density.min(2), seed);
        } else if bank == 4 && f & flags::SETTLEMENT != 0 {
            let part = (frame % 4) as usize;
            let origin = (x as i32 - (part >> 1) as i32, y as i32 - (part & 1) as i32);
            let plots: [Option<usize>; 4] = std::array::from_fn(|p| {
                let (x, y) = (origin.0 + (p >> 1) as i32, origin.1 + (p & 1) as i32);
                let n = index(x, y)?;
                if land.grounds[n] == Ground::Fog || map.county[n] != map.county[t]
                    || map.flags[n] & flags::SETTLEMENT == 0
                {
                    return None;
                }
                let (b, f) = frame_at(x as usize, y as usize);
                (b == bank && f as usize == frame as usize - part + p).then_some(n)
            });
            if plots[part + 1..].iter().any(Option::is_some) {
                continue;
            }
            let delta = |p: usize| {
                let (u, v) = ((p >> 1) as f32 - 0.5, (p & 1) as f32 - 0.5);
                (30.0 * (u - v), 15.0 * (u + v))
            };
            let (cx, cy) = delta(part);
            let center = (-cx, -cy);
            // Choose an exterior wall with an actual path, favoring front gates.
            let approach = [(2, [1, 3], 3), (1, [2, 3], 2), (3, [0, 1], 0), (0, [0, 2], 1)]
                .into_iter().find_map(|(side, parts, wall)| {
                    parts.into_iter().find_map(|p| {
                        let n = plots[p]?;
                        let (x, y) = delta(p);
                        (land.tracks[n] & (1 << side) != 0).then_some((wall,
                            (center.0 + x + EDGE[side].0, center.1 + y + EDGE[side].1)))
                    })
                });
            painter.castle(frame, center, approach);
        } else if bank == 3 && f & flags::SETTLEMENT != 0 {
            if let Some((site, wrecked, working)) = industry(map, t, frame) {
                painter.industry(site, wrecked, working);
            }
        }
    }
    // A visible crest cannot paint new information across the exploration edge.
    for run in fog_runs {
        let [r, g, b] = FOG;
        for pixel in image.data_mut()[run].chunks_exact_mut(4) {
            pixel.copy_from_slice(&[r, g, b, 255]);
        }
    }
    image
}

struct Pigments {
    turf: Rgb,
    rock: Rgb,
    pine: Rgb,
    shallows: Rgb,
    sea: Rgb,
    sand: Rgb,
}

const PALETTES: [Pigments; 4] = [
    Pigments { turf: [89, 110, 54], rock: [129, 128, 117], pine: [44, 77, 51],
        shallows: [67, 138, 132], sea: [33, 76, 103], sand: [171, 147, 100] },
    Pigments { turf: [99, 124, 53], rock: [136, 128, 110], pine: [43, 76, 49],
        shallows: [66, 141, 124], sea: [25, 76, 103], sand: [182, 155, 101] },
    Pigments { turf: [116, 119, 60], rock: [136, 121, 103], pine: [59, 77, 52],
        shallows: [74, 127, 129], sea: [34, 72, 100], sand: [174, 147, 98] },
    Pigments { turf: [130, 125, 98], rock: [103, 113, 113], pine: [45, 69, 63],
        shallows: [87, 143, 156], sea: [43, 76, 106], sand: [156, 174, 168] },
];

#[derive(Clone, Copy)]
struct Parcel {
    state: u8,
    joined: [bool; 4],
    gates: [bool; 4],
    turned: bool,
    seed: u32,
}

#[derive(Clone, Copy, Default)]
struct Works {
    road: Option<u8>,
    tracks: u8,
    foundation: Option<u8>,
}

#[derive(Clone, Copy, Default)]
struct PaintedTile {
    setting: Setting,
    parcel: Option<Parcel>,
    works: Works,
}

#[derive(Clone, Copy, Default)]
struct Setting {
    coast: f32,
    tended: f32,
    wood: f32,
    rock: f32,
}

impl Setting {
    fn at(land: &Land<'_>, x: i32, y: i32) -> Self {
        let mut setting = Self::default();
        let mut total = 0.0;
        for dy in -2_i32..=2 {
            for dx in -2_i32..=2 {
                let ground = land.at(x + dx, y + dy);
                if ground == Ground::Fog {
                    continue;
                }
                let weight = ((3 - dx.abs()) * (3 - dy.abs())) as f32;
                let core = (weight / 9.0) * (weight / 9.0);
                total += weight;
                match ground {
                    Ground::Water => setting.coast += weight,
                    Ground::Wood => setting.wood += core * 0.38,
                    Ground::Hill => setting.rock += core,
                    Ground::Meadow => {
                        if let Some(t) = index(x + dx, y + dy) {
                            let f = land.map.flags[t];
                            if f & (flags::CASTLE | flags::SETTLEMENT) != 0
                                || f & flags::FARMLAND != 0 && land.map.terrain[t] != 0
                            {
                                setting.tended += weight;
                            } else if f & flags::ROAD != 0 {
                                setting.tended += weight * 0.35;
                            }
                        }
                    }
                    Ground::Fog => unreachable!(),
                }
            }
        }
        if land.at(x, y) == Ground::Wood {
            setting.wood += 0.62;
        }
        if total > 0.0 {
            setting.coast /= total;
            setting.tended /= total;
        }
        setting
    }

    fn mix(self, other: Self, t: f32) -> Self {
        Self {
            coast: lerp(self.coast, other.coast, t),
            tended: lerp(self.tended, other.tended, t),
            wood: lerp(self.wood, other.wood, t),
            rock: lerp(self.rock, other.rock, t),
        }
    }
}

#[derive(Clone, Copy, Default)]
struct Surface {
    elevation: f32,
    canopy: f32,
    lift: f32,
    pigment: Rgb,
}

// A column has constant x−y. Along it, increasing both x and y by one
// advances exactly one sample. Storing this band rather than its tile-space
// bounding box keeps a newly exposed strip proportional to its screen area.
struct Band {
    diagonal: i32,
    along: i32,
    stride: usize,
}

impl Band {
    fn index(&self, diagonal: i32, along: i32) -> usize {
        (diagonal - self.diagonal) as usize * self.stride + (along - self.along) as usize
    }

    fn offset(&self, diagonal: i32, x: i32, y: i32) -> isize {
        let delta = x - y;
        delta as isize * self.stride as isize + (y + ((diagonal & 1) + delta).div_euclid(2)) as isize
    }
}

struct Relief {
    band: Band,
    steps: i32,
    pixels: Vec<Rgb>,
}

impl Relief {
    fn new(
        land: &Land<'_>, (a0, b0): Point, (a1, b1): Point,
        seed: u32, season: usize, near: bool,
    ) -> Self {
        let steps = if near { NEAR_STEPS } else { 6 };
        let scale = steps as f32;
        // Two samples cover bilinear taps on either side of the crop. An
        // additional two tiles downhill cover the maximum terrain lift.
        let d0 = (a0 * 2.0 * scale).floor() as i32 - 2;
        let d1 = (a1 * 2.0 * scale).ceil() as i32 + 3;
        let k0 = (b0 * scale).floor() as i32 - 2;
        let k1 = (b1 * scale).ceil() as i32 + 3 + 2 * steps;
        let band = Band { diagonal: d0, along: k0, stride: (k1 - k0) as usize };

        // The surface halo supplies the strongest up-sun occluder and the
        // two central-difference normals. It is not shaded or projected.
        let sd0 = d0 - steps - 2;
        let sk0 = k0 - steps - 2;
        let sk1 = k1 + 1;
        // The near woodland projection also needs the roots of crowns whose
        // left tips overlap the rightmost relief sample.
        let sd1 = d1 + if near { art::CROWN_MARGIN } else { 1 };
        let samples = Band { diagonal: sd0, along: sk0, stride: (sk1 - sk0) as usize };
        let mut surface = vec![Surface::default(); (sd1 - sd0) as usize * samples.stride];
        let colors = &PALETTES[season];

        let x0 = (sk0 + (sd0 + 1).div_euclid(2)).div_euclid(steps);
        let y0 = (sk0 - (sd1 - 1).div_euclid(2)).div_euclid(steps);
        let x1 = (sk1 - 1 + sd1.div_euclid(2)).div_euclid(steps) + 1;
        let y1 = (sk1 - 1 - sd0.div_euclid(2)).div_euclid(steps) + 1;
        let width = (x1 - x0 + 1) as usize;
        let descriptions: Vec<_> = (y0..=y1).flat_map(|y| (x0..=x1).map(move |x| PaintedTile {
            setting: Setting::at(land, x, y),
            parcel: land.parcel(x, y, seed),
            works: land.works(x, y),
        })).collect();
        for d in sd0..sd1 {
            let (ix, iy) = ((d + 1).div_euclid(2), -d.div_euclid(2));
            let start = samples.index(d, sk0);
            let column = &mut surface[start..start + samples.stride];
            let mut cell = (i32::MIN, i32::MIN);
            let mut corners = [PaintedTile::default(); 4];
            for (k, out) in (sk0..sk1).zip(column) {
                let (u, v) = (k + ix, k + iy);
                let (x, y) = (u.div_euclid(steps), v.div_euclid(steps));
                if cell != (x, y) {
                    cell = (x, y);
                    let p = (y - y0) as usize * width + (x - x0) as usize;
                    corners = [descriptions[p], descriptions[p + 1],
                        descriptions[p + width], descriptions[p + width + 1]];
                }
                let (i, j) = (u.rem_euclid(steps), v.rem_euclid(steps));
                let (s, t) = (i as f32 / scale, j as f32 / scale);
                let [nw, ne, sw, se] = corners;
                let setting = nw.setting.mix(sw.setting, t).mix(ne.setting.mix(se.setting, t), s);
                let tile = corners[usize::from(i >= steps / 2) + 2 * usize::from(j >= steps / 2)];
                let (u, v) = (x as f32 + s, y as f32 + t);
                let mut sample = art::terrain(setting, tile.works, colors, (u, v), seed, season, near);
                if let Some(parcel) = tile.parcel {
                    art::field(parcel, &mut sample, (u, v), season, seed, near);
                }
                *out = sample;
            }
        }

        let mut pixels = vec![[0; 3]; (d1 - d0) as usize * band.stride];
        for d in d0..d1 {
            let start = band.index(d, k0);
            let column = &mut pixels[start..start + band.stride];
            let start = samples.index(d, k0);
            let east = samples.offset(d, 1, 0);
            let west = samples.offset(d, -1, 0);
            let south = samples.offset(d, 0, 1);
            let north = samples.offset(d, 0, -1);
            let occluders = [(steps / 8 + 1, 1), (steps / 4 + 1, steps / 12 + 1),
                (steps / 2 + 1, steps / 6 + 1), (steps + 1, steps / 3 + 1)]
                .map(|(u, v)| (samples.offset(d, -u, -v), (u as f32 + v as f32 * 0.4) / scale));
            for (i, color) in column.iter_mut().enumerate() {
                let p = start + i;
                let Surface { elevation, canopy, pigment, .. } = surface[p];
                let elevation_at = |offset| surface[p.wrapping_add_signed(offset)].elevation;
                let du = (elevation_at(east) - elevation_at(west)) * (scale * 0.5);
                let dv = (elevation_at(south) - elevation_at(north)) * (scale * 0.5);
                // Normals and short terrain shadows both face the same
                // upper-left sun. The cold sky still lights leeward slopes.
                let sun = ((0.59 + du * 0.74 + dv * 0.32) / (1.0 + du * du + dv * dv).sqrt()).max(0.0);
                let mut obstruction: f32 = 0.0;
                for (offset, distance) in occluders {
                    let upstream = surface[p.wrapping_add_signed(offset)];
                    let rise = upstream.elevation - elevation + upstream.canopy * 0.085 - canopy * 0.085;
                    obstruction = obstruction.max((rise / distance - 0.54).clamp(0.0, 0.9));
                }
                let shadow = (0.59 - sun * (1.0 - obstruction)).max(0.0);
                let light = (sun - 0.59).max(0.0) * (1.0 - obstruction);
                *color = std::array::from_fn(|c| {
                    (f32::from(pigment[c]) + light * [112.0, 105.0, 93.0][c]
                        - shadow * [93.0, 77.0, 61.0][c]).clamp(0.0, 255.0) as u8
                });
            }
            project_strip(column, &surface[start..start + band.stride], scale);
        }
        if near {
            art::woodland(&band, &samples, k1 - steps, &surface, &mut pixels, seed, season);
        }
        Self { band, steps, pixels }
    }

    fn color(&self, (x, y): (i32, i32), (s, t): Point) -> Rgb {
        let (s, t) = (s * self.steps as f32, t * self.steps as f32);
        let (i, j) = (s as i32, t as i32);
        let (x, y) = (x * self.steps + i, y * self.steps + j);
        let d = x - y;
        let p = self.band.index(d, y + d.div_euclid(2));
        let (s, t) = (s - i as f32, t - j as f32);
        let (nw, ne, sw, se) = (
            self.pixels[p], self.pixels[p.wrapping_add_signed(self.band.offset(d, 1, 0))],
            self.pixels[p.wrapping_add_signed(self.band.offset(d, 0, 1))], self.pixels[p + 1],
        );
        std::array::from_fn(|c| {
            lerp(lerp(f32::from(nw[c]), f32::from(ne[c]), s),
                lerp(f32::from(sw[c]), f32::from(se[c]), s), t) as u8
        })
    }
}

fn project_strip(pixels: &mut [Rgb], surface: &[Surface], scale: f32) {
    // Back to front: a front bank overwrites the back slope. Express each
    // segment relative to its first sample so rounding cannot depend on
    // where a cache region started.
    let mut previous = (surface[0].lift * scale, pixels[0]);
    for i in 1..pixels.len() {
        let next = (surface[i].lift * scale, pixels[i]);
        let end = 1.0 - next.0;
        let span = end + previous.0;
        if span > 0.0 && (next.0 > 0.0 || previous.0 > 0.0) {
            let base = i as i32 - 1;
            let from = (-previous.0).ceil() as i32;
            let to = end.floor() as i32;
            for j in from.max(-base)..=to {
                pixels[(base + j) as usize] = mix(previous.1, next.1, (j as f32 + previous.0) / span);
            }
        }
        previous = next;
    }
}

fn paint_ground(
    image: &mut Pixmap,
    land: &Land<'_>,
    key: Key,
    zoom: &Zoom,
    clip: Clip,
) -> Vec<Range<usize>> {
    let season = campaign::season_slot(key.season);
    let colors = &PALETTES[season];
    let (row, col) = campaign::tile_to_cell(0, 0);
    let (x, y) = campaign::cell_to_screen(key.view, zoom, row, col);
    let (origin_x, origin_y) = (x + zoom.half_pitch, y + zoom.row_step);
    let (dx, dy) = (1.0 / (2 * zoom.half_pitch) as f32, 1.0 / (2 * zoom.row_step) as f32);
    let seed = key.slot as u32;
    let setup_relief = || Relief::new(
        land,
        ((clip.x0 - origin_x) as f32 * dx, (clip.y0 - origin_y) as f32 * dy),
        ((clip.x1 - origin_x) as f32 * dx, (clip.y1 - origin_y) as f32 * dy),
        seed,
        season,
        zoom.id == campaign::NEAR.id,
    );
    let mut relief = None;
    let world_x = (clip.x0 - origin_x) * SCALE as i32;
    let world_y = (clip.y0 - origin_y) * SCALE as i32;
    let (dx, dy) = (dx / SCALE as f32, dy / SCALE as f32);
    let physical_width = image.width() as usize;
    let mut fog_runs = Vec::new();
    for (row, scan) in image.data_mut().chunks_exact_mut(physical_width * 4).enumerate() {
        let b = ((world_y + row as i32) as f32 + 0.5) * dy;
        let mut fog_from = None;
        let mut patch = (i32::MIN, i32::MIN);
        let mut corners = [Ground::Water; 4];
        for (col, pixel) in scan.chunks_exact_mut(4).enumerate() {
            let a = ((world_x + col as i32) as f32 + 0.5) * dx;
            let (u, v) = (b + a, b - a);
            let (x, y) = (u.floor() as i32, v.floor() as i32);
            let (s, t) = (u - x as f32, v - y as f32);
            if (x, y) != patch {
                patch = (x, y);
                corners = [land.at(x, y), land.at(x + 1, y), land.at(x, y + 1), land.at(x + 1, y + 1)];
            }
            let center = corners[(s >= 0.5) as usize + 2 * (t >= 0.5) as usize];
            let rgb = if center == Ground::Fog {
                fog_from.get_or_insert(col);
                FOG
            } else {
                if let Some(start) = fog_from.take() {
                    let base = row * physical_width;
                    fog_runs.push((base + start) * 4..(base + col) * 4);
                }
                ground_color(colors, (x, y), (s, t), corners, center, seed, relief.get_or_insert_with(setup_relief))
            };
            pixel.copy_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
        }
        if let Some(start) = fog_from {
            let base = row * physical_width;
            fog_runs.push((base + start) * 4..(base + physical_width) * 4);
        }
    }
    fog_runs
}

fn ground_color(
    colors: &Pigments,
    (x, y): (i32, i32),
    (s, t): Point,
    corners: [Ground; 4],
    center: Ground,
    seed: u32,
    relief: &Relief,
) -> Rgb {
    let (mut wet, mut dark) = (0.0, 0.0);
    if corners == [center; 4] {
        wet = if center == Ground::Water { 1.0 } else { 0.0 };
    } else {
        let weights = [(1.0 - s) * (1.0 - t), s * (1.0 - t), (1.0 - s) * t, s * t];
        for (g, weight) in corners.into_iter().zip(weights) {
            if g == Ground::Fog {
                dark += weight;
            }
            if g == Ground::Water || g == Ground::Fog && center == Ground::Water {
                wet += weight;
            }
        }
    }
    let (u, v) = (x as f32 + s, y as f32 + t);
    let mut shore = 0.5 - wet;
    if (0.03..0.97).contains(&wet) {
        shore += (noise(u * 4.6, v * 4.6, seed + 7) - 0.5) * 0.146;
    }
    let water = || {
        let (wx, wy) = (u - v, u + v);
        let weather = noise(u * 0.73 + 11.0, v * 0.78, seed);
        let swell = noise(wx * 1.1, wy * 9.8 + weather * 0.8, seed + 3);
        let depth = ramp(shore, -0.49, -0.05);
        let wave = noise(wx * 6.9 + wy * 3.2, wy * 24.0 + swell * 1.21, seed ^ 0x8f13);
        let tex_d = crate::original::art_pack::water().detail(wx * 44.0, wy * 44.0);
        let crest = ramp(swell * 0.62 + wave * 0.38, 0.61, 0.87);
        let glint = crest * (20.0 + weather * 10.0) + (wave - 0.5) * 8.0;
        let breaker = (1.0 - ((shore + 0.051 + (swell - 0.5) * 0.033) * 63.0).abs()).max(0.0);
        let base_w = mix(colors.sea, colors.shallows, depth * 0.82);
        let rippled = [
            (f32::from(base_w[0]) + tex_d[0] * 0.95 + (weather - 0.5) * 12.0 + glint).clamp(0.0, 255.0) as u8,
            (f32::from(base_w[1]) + tex_d[1] * 0.95 + (weather - 0.5) * 12.0 + glint).clamp(0.0, 255.0) as u8,
            (f32::from(base_w[2]) + tex_d[2] * 0.95 + (weather - 0.5) * 12.0 + glint).clamp(0.0, 255.0) as u8,
        ];
        mix(rippled, [222, 236, 232], breaker * ramp(wave, 0.25, 0.80) * 0.64)
    };
    let result = if shore <= -0.06 {
        water()
    } else {
        let base = relief.color((x, y), (s, t));
        if shore < 0.28 {
            let basin = noise(u * 1.44 + 4.0, v * 1.69 + 7.0, seed + 92);
            let wrack = noise(u * 5.8, v * 4.1, seed + 43);
            let grain = noise(u * 18.3, v * 13.7, seed + 44);
            let deposit = ramp(basin * 0.75 + wrack * 0.25, 0.24, 0.80);
            let dry = ramp(shore, -0.025, 0.09 + deposit * 0.11);
            let grit = mix(shade(colors.rock, -16.0), colors.sand, deposit);
            let beach = shade(grit, lerp(-38.0, 16.0 + deposit * 11.0, dry) + (grain - 0.5) * 23.0);
            let dune = ramp(shore, 0.028 + deposit * 0.067, 0.102 + deposit * 0.132 + wrack * 0.038);
            let exposure = 1.0 - ramp(shore, 0.18, 0.28);
            let shoots = noise(u * 12.0 - v * 8.0, u * 2.6 + v * 4.3, seed + 45);
            let saltgrass = ramp(shoots, 0.44, 0.84) * ramp(wrack, 0.29, 0.64) * dune * exposure;
            let bank = mix(mix(beach, base, dune), shade(colors.turf, -23.0), saltgrass * 0.86);
            // Where exposed rock meets the water, pick out the exposed face
            // and damp foot. Sand bars elsewhere retain a gentler transition.
            let stone = (1.0 - ramp(deposit, 0.20, 0.54)) * ramp(wrack, 0.16, 0.67);
            let bank_top = (1.0 - ramp((shore - 0.106).abs(), 0.013, 0.059)) * stone;
            let bed = ((u * 3.1 + v * 1.7 + wrack * 0.36).rem_euclid(1.0) * 2.0 - 1.0).abs();
            let face = (1.0 - ramp(shore, 0.078, 0.116)) * ramp(shore, -0.052, -0.002);
            let color = shade(colors.rock, -49.0 + grain * 24.0 + bed * 18.0);
            let bank = mix(bank, color, face * stone * 0.93);
            let leafy = mix(bank, shade(colors.pine, 15.0 + grain * 7.0), (1.0 - deposit) * saltgrass * 0.48);
            let backshore = mix(leafy, shade(base, 20.0 + grain * 10.0), bank_top * 0.76);
            if shore < 0.012 { mix(water(), backshore, ramp(shore, -0.060, 0.012)) } else { backshore }
        } else {
            base
        }
    };
    if dark == 0.0 { result } else { mix(result, FOG, (dark * 0.8).min(0.48)) }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    std::array::from_fn(|i| lerp(f32::from(a[i]), f32::from(b[i]), t) as u8)
}

fn shade(c: Rgb, light: f32) -> Rgb {
    c.map(|channel| (f32::from(channel) + light).clamp(0.0, 255.0) as u8)
}

fn hash(x: i32, y: i32, seed: u32) -> u32 {
    let n = (x as u32).wrapping_mul(0x9e37_79b1) ^ (y as u32).wrapping_mul(0x85eb_ca77)
        ^ seed.wrapping_mul(0xc2b2_ae3d);
    let n = (n ^ (n >> 16)).wrapping_mul(0x7feb_352d);
    n ^ (n >> 15)
}

fn eased(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn ramp(value: f32, low: f32, high: f32) -> f32 {
    eased(((value - low) / (high - low)).clamp(0.0, 1.0))
}

fn noise(x: f32, y: f32, seed: u32) -> f32 {
    let (a, b) = (x.floor() as i32, y.floor() as i32);
    let (s, t) = (eased(x - a as f32), eased(y - b as f32));
    let sample = |dx, dy| (hash(a + dx, b + dy, seed) >> 24) as f32 / 255.0;
    let near = sample(0, 0) * (1.0 - s) + sample(1, 0) * s;
    let far = sample(0, 1) * (1.0 - s) + sample(1, 1) * s;
    near * (1.0 - t) + far * t
}

#[cfg(test)]
mod tests {
    use super::*;
    use l2_formats::maps::{MapSet, SLOT_LEN};

    #[test]
    fn raised_rock_occludes_the_back_slope_without_moving_adjacent_ground() {
        let mut surface = [Surface::default(); 5];
        let mut pixels = [0, 40, 80, 120, 160].map(|shade| [shade; 3]);
        surface[2].lift = 1.0;
        surface[3].lift = 1.0;
        project_strip(&mut pixels, &surface, 1.0);
        assert_eq!(pixels, [[0; 3], [80; 3], [120; 3], [140; 3], [160; 3]]);
    }

    #[test]
    fn physical_ground_samples_follow_the_world_and_hide_unexplored_tiles() {
        let bytes = vec![0; SLOT_LEN];
        let set = MapSet::parse(&bytes).unwrap();
        let lattice = Lattice::build(&set.slot(0).unwrap());
        let mut map = CampaignMap::empty();
        map.flags[map::index(27, 29)] = flags::NO_COUNTY;
        let zoom = &campaign::NEAR;
        let mut key = Key {
            slot: 0, zoom: zoom.id, view: Viewport::centred_on_tile(28, 29, zoom),
            turn: 0, season: 1, fields: 0, castles: 0, sites: 0, fog: 0,
        };
        let sample = |land: &Land<'_>, key: Key| {
            let (row, col) = campaign::tile_to_cell(28, 29);
            let (x, y) = campaign::cell_to_screen(key.view, zoom, row, col);
            let (x, y) = (x + zoom.half_pitch, y + zoom.row_step);
            let clip = Clip::new(x - 2, y - 2, x + 3, y + 3);
            let mut image = new_image(clip.width(), clip.height());
            let runs = paint_ground(&mut image, land, key, zoom, clip);
            (image, runs)
        };
        let revealed = LandState::new(&map, &lattice, None);
        let revealed = revealed.borrow(&map, &lattice);
        let (first, clear) = sample(&revealed, key);
        assert!(clear.is_empty());
        assert!(first.data().chunks_exact(4).all(|p| p[3] == 255));
        key.view.row += 2;
        key.view.col += 1;
        let (moved, _) = sample(&revealed, key);
        assert_eq!(first.data(), moved.data());
        let fogged = LandState::new(&map, &lattice, Some(&|x, y| (x, y) == (28, 29)));
        let fogged = fogged.borrow(&map, &lattice);
        let (dark, runs) = sample(&fogged, key);
        assert_eq!(runs.iter().map(Range::len).sum::<usize>(), dark.data().len());
        assert!(dark.data().chunks_exact(4).all(|p| p == [FOG[0], FOG[1], FOG[2], 255]));
    }

    #[test]
    fn relief_and_fog_are_identical_in_independently_rasterized_crops() {
        let bytes = vec![0; SLOT_LEN];
        let set = MapSet::parse(&bytes).unwrap();
        let lattice = Lattice::build(&set.slot(0).unwrap());
        let mut map = CampaignMap::empty();
        for y in 26..=28 {
            for x in 27..=29 {
                let t = map::index(x, y);
                map.flags[t] = flags::ROUGH | if y == 28 { flags::ROAD } else { 0 };
                map.bank[t] = if y == 28 || x == 29 { 4 } else { 0 };
            }
        }
        let state = LandState::new(&map, &lattice, Some(&|x, y| x == 29 && y >= 28));
        let land = state.borrow(&map, &lattice);
        let (row, col) = campaign::tile_to_cell(28, 28);
        for zoom in [&campaign::NEAR, &campaign::FAR] {
            let (x, y) = world_center(zoom, row, col);
            let key = Key {
                slot: 0, zoom: zoom.id, view: WORLD_VIEW, turn: 0, season: 4,
                fields: 0, castles: 0, sites: 0, fog: 1,
            };
            let wide = Clip::new(x - 21, y - 23, x + 26, y + 29);
            let crop = Clip::new(x - 7, y - 8, x + 8, y + 9);
            let mut full = new_image(wide.width(), wide.height());
            let mut part = new_image(crop.width(), crop.height());
            paint_ground(&mut full, &land, key, zoom, wide);
            paint_ground(&mut part, &land, key, zoom, crop);
            let stride = full.width() as usize * 4;
            let x = (crop.x0 - wide.x0) as usize * SCALE * 4;
            let y = (crop.y0 - wide.y0) as usize * SCALE;
            for (i, scan) in part.data().chunks_exact(part.width() as usize * 4).enumerate() {
                let start = (y + i) * stride + x;
                assert_eq!(scan, &full.data()[start..start + scan.len()]);
            }
        }
    }

    #[test]
    fn atlas_pan_preserves_edges_where_roofs_roads_and_relief_cross_chunk_boundaries() {
        use std::time::Instant;

        let bytes = vec![0; SLOT_LEN];
        let set = MapSet::parse(&bytes).unwrap();
        let slot = set.slot(0).unwrap();
        let lattice = Lattice::build(&slot);
        let mut map = CampaignMap::empty();
        map.county.fill(1);
        for u in 21..35 {
            map.flags[map::index(u, 26)] = flags::ROAD;
        }
        for (x, y) in [(27, 26), (28, 26), (27, 27), (28, 27)] {
            map.flags[map::index(x, y)] |= flags::CASTLE;
        }
        map.flags[map::index(26, 27)] = flags::ROUGH;
        map.bank[map::index(26, 27)] = 4;
        let zoom = &campaign::NEAR;
        let first = Viewport::centred_on_tile(27, 26, zoom);
        let next = Viewport::new(first.row, first.col + 1);
        let scene = Key { slot: 0, zoom: zoom.id, view: first, turn: 1, season: 1,
            fields: 0, castles: 0, sites: 0, fog: 0 };
        let clip = zoom.clip();
        let overrides = Overrides::default();
        let mut actual = Canvas::screen();
        let mut cache = None;
        draw(&mut cache, &mut actual, scene, zoom, &slot, &lattice, &map, &overrides, None);
        let start = Instant::now();
        draw(&mut cache, &mut actual, Key { view: next, ..scene }, zoom, &slot, &lattice,
            &map, &overrides, None);
        let pan = start.elapsed();

        let (dx, dy) = (next.col * zoom.pitch, next.row * zoom.row_step);
        let region = Clip::new(clip.x0 + dx, clip.y0 + dy, clip.x1 + dx, clip.y1 + dy);
        let land = LandState::new(&map, &lattice, None);
        let start = Instant::now();
        let rendered = paint_scene(Key { view: WORLD_VIEW, ..scene }, zoom, &slot, &land.borrow(&map, &lattice), &overrides, region);
        let full = start.elapsed();
        let mut expected = Canvas::screen();
        expected.enable_detail();
        expected.blit_detail_opaque(rendered.data(), rendered.width() as usize, clip.x0, clip.y0, clip);
        eprintln!("Campaign CPU scenery: {:?} retained pan vs {:?} full redraw", pan, full);
        let palette = l2_formats::Palette::from_bytes(&[0; 768]).unwrap();
        let length = 640 * 480 * SCALE * SCALE * 4;
        let (mut cached, mut direct) = (vec![0; length], vec![0; length]);
        actual.to_rgba_at(&palette, SCALE, &mut cached);
        expected.to_rgba_at(&palette, SCALE, &mut direct);
        let differences = cached.chunks_exact(4).zip(direct.chunks_exact(4)).enumerate();
        let mut max_error = 0;
        let mut count = 0;
        let mut worst_at = (0, 0);
        for (i, (a, b)) in differences {
            let error = a.iter().zip(b).map(|(&a, &b)| a.abs_diff(b)).max().unwrap();
            if error != 0 { count += 1; }
            if error > max_error {
                max_error = error;
                worst_at = (i % (640 * SCALE), i / (640 * SCALE));
            }
        }
        // Rasterizing a clipped vector path can round coverage by a few
        // levels at its antialiased edge; a seam is longer and higher contrast.
        assert!(count <= 128 && max_error <= 14,
            "{count} differing pixels; max channel error {max_error} at {worst_at:?}");
    }

    #[test]
    fn site_tracks_reach_a_road_without_crossing_fields_water_or_fog() {
        let bytes = vec![0; SLOT_LEN];
        let set = MapSet::parse(&bytes).unwrap();
        let lattice = Lattice::build(&set.slot(0).unwrap());
        let mut map = CampaignMap::empty();
        map.county.fill(1);
        let tile = map::index;
        map.flags[tile(10, 10)] = flags::ROAD;
        map.flags[tile(11, 10)] = flags::FARMLAND;
        map.terrain[tile(11, 10)] = 1;
        map.flags[tile(11, 9)] = flags::NO_COUNTY;
        map.flags[tile(12, 10)] = flags::SETTLEMENT;
        map.flags[tile(10, 9)] = flags::SETTLEMENT;
        let land = LandState::new(&map, &lattice, Some(&|x, y| (x, y) == (10, 9)));
        let links = land.tracks;
        assert_eq!(links[tile(10, 10)], 1 << 2);
        assert_eq!(links[tile(11, 11)], (1 << 1) | (1 << 3));
        assert_eq!(links[tile(12, 10)], 1 << 2);
        assert_eq!(links[tile(11, 10)], 0);
        assert_eq!(links[tile(11, 9)], 0);
        assert_eq!(links[tile(10, 9)], 0);
    }

    #[test]
    fn connected_fields_receive_a_lane_and_leave_the_crop_plane_untouched() {
        let bytes = vec![0; SLOT_LEN];
        let set = MapSet::parse(&bytes).unwrap();
        let lattice = Lattice::build(&set.slot(0).unwrap());
        let mut map = CampaignMap::empty();
        map.county.fill(1);
        let tile = map::index;
        map.flags[tile(10, 10)] = flags::ROAD;
        for x in [11, 12] {
            map.flags[tile(x, 10)] = flags::FARMLAND;
            map.terrain[tile(x, 10)] = 1;
        }
        let land = LandState::new(&map, &lattice, None);
        let land = land.borrow(&map, &lattice);
        assert_eq!(land.tracks[tile(10, 10)], 1 << 1);
        assert_eq!(land.tracks[tile(11, 10)], 0);
        let parcel = land.parcel(11, 10, 0).unwrap();
        assert!(parcel.joined[1] && parcel.gates[3]);
    }
}
