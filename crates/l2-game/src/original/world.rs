//! Fictional provinces and routes for both campaign ladders. Every world
//! is authored as a set of settlement sites, coast parameters and a watershed.
//! Tile metadata and the cartographic preview are made from the same geometry.

use super::{noise, sheet, Art, WATER};
use l2_formats::maps::{MapSet, MapSlot, Plane, LATTICE_LEN, PLANE_COUNT, PLANE_DIM, PLANE_LEN, SLOT_LEN};
use l2_kingdom::map::flags;

const SLOT_COUNT: usize = 54;
const N: i32 = PLANE_DIM as i32;
const DIRS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];
const ROAD: [u8; 16] = [0, 6, 7, 2, 8, 0, 3, 10, 9, 5, 1, 11, 4, 12, 13, 14];
const STARTS: [u8; 12] = [1, 0, 2, 0, 3, 0, 4, 0, 5, 0, 0, 0];

type Point = (i32, i32);

struct World {
    slot: usize,
    name: &'static str,
    counties: &'static [&'static str],
    // Grid orientation (flip/swap), width, height, and watershed axis offset.
    profile: (u8, i32, i32, i32),
}

const WORLDS: &[World] = &[
    World { slot: 0, name: "The Linden March", counties: &["Hawkridge", "Lindengate", "Willowby", "Barrow Cross", "Westmere", "Alderhold", "Rook's Ford", "Vale End", "Ashfield", "Gilden Tor", "Greyholt", "Wren's Rest"], profile: (0, 28, 28, 2) },
    World { slot: 2, name: "The Reedwater Vale", counties: &["Reed Hall", "Bridgeward", "Sedgewick", "Heron Fen", "Mallen Brook", "Whitebank", "Glasstarn", "Rush End", "Osier", "Fishwick"], profile: (2, 26, 28, -2) },
    World { slot: 3, name: "The Five Fords", counties: &["Oakenford", "Eel Ford", "Sunreach", "Sparrow Ford", "Claybank", "Brae Ford", "Moor Hall", "Tinker Ford", "Holt", "Crown Ash", "Flagstone", "Watch Knoll"], profile: (5, 28, 28, -5) },
    World { slot: 4, name: "The Iron Crown", counties: &["Hammerstead", "Iron Cap", "Redwall", "Anvil", "Hearthspire", "Hawksfell", "Mossgate", "Stagwatch", "Furnace End", "Longtarn", "Blackheath", "Flint Ridge"], profile: (3, 26, 29, 4) },
    World { slot: 5, name: "The Brass Coast", counties: &["Copperstrand", "Bell Quay", "Beacon Fell", "Sable Haven", "Netherport", "Silver Spit", "Windwick", "Abalone", "Cliff's End", "Marsh Bay", "Thornfleet"], profile: (4, 29, 26, 1) },
    World { slot: 7, name: "The High Lanterns", counties: &["Lantern Peak", "Shepherd's Rest", "Cragfoot", "Windglass", "High Hearth", "Cloud Ward", "Tower Glen", "Eagle's Cairn", "Bellrock", "Sleetfell", "Moon Hill", "Harroweth"], profile: (6, 27, 27, -3) },
    World { slot: 12, name: "The Foxglove Weald", counties: &["Foxglove", "Hazelward", "Fern Hill", "Yew Gate", "Otterbank", "Beech End", "Pine Hollow", "Hart's Walk"], profile: (1, 24, 23, 3) },
    World { slot: 17, name: "The Wrenwood", counties: &["Wren Hall", "Oatwick", "Woodbridge", "Briar Mill", "Birchleigh", "Millgate"], profile: (0, 24, 22, 4) },
    World { slot: 41, name: "The Falcon Road", counties: &["Kite Hall", "Haw's Watch", "Talonside", "Vale Road", "Swiftmere", "Larkspur", "Kestrel Tor", "Hillward", "Dry Mile", "Pilgrim's Well", "Far Oak", "Old Post"], profile: (7, 29, 26, 0) },
    World { slot: 42, name: "The Stormward Reach", counties: &["Stormgate", "Rainbarrow", "Whalebone", "Whitecap", "Tempest End", "Wicklow", "Gull Watch", "Hail's Rest", "Greyhaven", "West Reef", "Seabreak", "North Light"], profile: (1, 27, 27, -1) },
    World { slot: 44, name: "The Rowan Basin", counties: &["East Rowan", "Firethorn", "Swanwick", "The Weir", "Duskpool", "Crowbarrow", "Birch Ford", "Drift Bank", "Red Vale", "Cobblewell"], profile: (2, 28, 26, -4) },
    World { slot: 45, name: "The Cinder Fells", counties: &["Coal Ridge", "Cairn Gate", "Gorse Hall", "Carver's Gap", "Sootstead", "Dunwick", "Cinderbank", "Slate House", "Sparkside"], profile: (7, 25, 26, 5) },
    World { slot: 52, name: "The Glasswater", counties: &["Glassmouth", "Mirror Ward", "Iris Bank", "Frostwick", "Eider Hall", "Bluefen"], profile: (5, 27, 23, -2) },
    World { slot: 53, name: "The Ember Downs", counties: &["Emberhall", "Saffron Down", "Low Beacon", "Hayward", "Cricket Hill", "Clay Tor", "Poppy Vale", "Sunbarrow"], profile: (6, 26, 26, 1) },
];

pub(crate) const MAP_NAMES: &[&str] = &{
    let mut names = [""; SLOT_COUNT];
    let mut i = 0;
    while i < WORLDS.len() {
        names[WORLDS[i].slot] = WORLDS[i].name;
        i += 1;
    }
    names
};

pub(crate) const COUNTY_NAMES: &[&[&str]] = &{
    let mut names: [&[&str]; SLOT_COUNT] = [&[]; SLOT_COUNT];
    let mut i = 0;
    while i < WORLDS.len() {
        names[WORLDS[i].slot] = WORLDS[i].counties;
        i += 1;
    }
    names
};

fn idx(x: i32, y: i32) -> usize {
    (y * N + x) as usize
}

fn variant(x: i32, y: i32, seed: usize, count: u8) -> u8 {
    (noise(x, y, seed as u32) % u32::from(count)) as u8
}

fn centres(world: &World) -> Vec<Point> {
    const SMALL: [Point; 9] = [(20, 20), (43, 22), (19, 43), (42, 44), (31, 32),
        (32, 14), (31, 49), (46, 33), (15, 31)];
    let (orient, ..) = world.profile;
    // The 12-place template has four columns in each row, spaced so that
    // the fixed footprints cannot overlap even when reflected.
    const LARGE: [Point; 12] = [(17, 19), (29, 17), (41, 19), (52, 24), (14, 32), (26, 31),
        (38, 32), (49, 36), (17, 44), (29, 46), (41, 44), (51, 47)];
    let template = if world.counties.len() > SMALL.len() { &LARGE[..] } else { &SMALL[..] };
    template[..world.counties.len()].iter().map(|&(mut x, mut y)| {
        if orient & 1 != 0 { x = N - 1 - x; }
        if orient & 2 != 0 { y = N - 1 - y; }
        if orient & 4 != 0 { std::mem::swap(&mut x, &mut y); }
        (x, y)
    }).collect()
}

fn river_x(y: i32, offset: i32) -> i32 {
    let phase = (y + 8).rem_euclid(38);
    28 + offset + (phase.min(38 - phase) * 2 / 5)
}

fn road(plotted: &mut [bool; PLANE_LEN], (mut x, mut y): Point, (tx, ty): Point) {
    let dx = (tx - x).abs();
    let dy = (ty - y).abs();
    let (sx, sy) = ((tx - x).signum(), (ty - y).signum());
    let (mut a, mut b) = (0, 0);
    plotted[idx(x, y)] = true;
    while (x, y) != (tx, ty) {
        if b == dy || (a < dx && (2 * a + 1) * dy < (2 * b + 1) * dx) {
            x += sx;
            a += 1;
        } else {
            y += sy;
            b += 1;
        }
        plotted[idx(x, y)] = true;
    }
}

fn slot(world: &World) -> Vec<u8> {
    let mut planes = [[0; PLANE_LEN]; PLANE_COUNT];
    let towns = centres(world);
    let (orient, rx, ry, offset) = world.profile;
    let mut roads = [false; PLANE_LEN];
    let distance = |(a, b): Point, (x, y): Point| (a - x).pow(2) + (b - y).pow(2);
    // Connect each town to the nearest of the ones already connected.
    // Armies always have a cross-river route.
    for (i, &(x, y)) in towns.iter().enumerate() {
        if let Some(&to) = towns[..i].iter().min_by_key(|&&p| distance((x, y), p)) {
            road(&mut roads, (x, y), to);
        }
    }
    for y in 0..N {
        for x in 0..N {
            let i = idx(x, y);
            let (county, nearest) = towns.iter().enumerate().min_by_key(|&(_, &(tx, ty))| {
                distance((x, y), (tx, ty)) * 2 + i32::from(variant(x / 2 + tx, y / 2 + ty, world.slot, 9))
            }).unwrap();
            let near_town = distance((x, y), *nearest) <= 42;
            let swell = ((y + world.slot as i32) % 19 - 9).abs() - 4;
            let coast = ((x - 31) * 100 / (rx + swell / 2)).pow(2)
                + ((y - 31) * 100 / ry).pow(2);
            let (u, v) = if orient & 4 == 0 { (x, y) } else { (y, x) };
            let wet = (u - river_x(v, offset)).abs() <= (v > 40) as i32;
            let real_land = coast < 10_000 || near_town || roads[i];
            if real_land { planes[Plane::County as usize][i] = (county + 1) as u8; }
            let n = variant(x, y, world.slot, 16);
            let (tile_flags, bank, frame) = if !real_land || (wet && !roads[i] && !near_town) {
                (flags::NO_COUNTY, 0, 22 + n % 8)
            } else if roads[i] {
                let arms = DIRS.iter().enumerate().fold(0, |mask, (d, &(dx, dy))| {
                    let (a, b) = (x + dx, y + dy);
                    mask | (((0..N).contains(&a) && (0..N).contains(&b) && roads[idx(a, b)]) as usize) << d
                });
                (flags::ROAD, 8, ROAD[arms])
            } else if !near_town && noise(x / 3 + y / 9, y / 3, world.slot as u32 ^ 17) % 13 < 2 {
                let corner = (y % 2 + 2 * (x % 2)) as u8;
                (flags::ROUGH, 4, 4 * (n % 4) + corner)
            } else if !near_town && noise(x / 4, y / 4, world.slot as u32) % 10 < 3 {
                (flags::ROUGH, 8, 30 + n % 8)
            } else {
                (0, 0, 6 + n)
            };
            planes[Plane::Flags as usize][i] = tile_flags;
            planes[Plane::GfxBank as usize][i] = bank;
            planes[Plane::GfxIndex as usize][i] = frame;
        }
    }
    let mut place = |(x, y), county, flags, bank, frame, marker| {
        let i = idx(x, y);
        planes[Plane::County as usize][i] = county;
        planes[Plane::Flags as usize][i] = flags;
        planes[Plane::GfxBank as usize][i] = bank;
        planes[Plane::GfxIndex as usize][i] = frame;
        planes[Plane::Marker as usize][i] = marker;
    };
    for (c, &(x, y)) in towns.iter().enumerate() {
        let county = (c + 1) as u8;
        for dy in 0..2 {
            for dx in 0..2 {
                let first = dx == 0 && dy == 0;
                place((x + dx, y + dy), county, flags::CASTLE, 0, 6, if first { 1 + county % 3 } else { 0 });
                place((x + 3 + dx, y - 2 + dy), county, flags::SETTLEMENT, 0, 6, if first { STARTS[c] } else { 0 });
            }
        }
        for dy in 2..6 {
            for dx in -2..3 {
                place((x + dx, y + dy), county, flags::FARMLAND, 0, 6 + variant(x + dx, y + dy, world.slot, 4), 0);
            }
        }
        for (dx, dy) in [(-2, -1), (-2, 0), (-1, -1), (-1, 1)] {
            place((x + dx, y + dy), county, flags::PLOT, 0, 6, 0);
        }
        place((x - 3, y - 2), county, flags::SETTLEMENT, 12, 20, 0);
        place((x - 1, y - 3), county, flags::SETTLEMENT, 12, if c % 3 == 0 { 30 } else { 0 }, 0);
        // A clear tile beside farmland guarantees a usable weapons site.
        place((x + 3, y + 2), county, 0, 0, 6, 0);
    }
    // Keep documented boundary flags independent of art. Roads and land pass
    // through soft county lines; no county is enclosed by impassable tiles.
    for y in 1..N - 1 {
        for x in 1..N - 1 {
            let i = idx(x, y);
            let c = planes[Plane::County as usize][i];
            if c == 0 { continue; }
            let mut border = None;
            for (dir, &(dx, dy)) in DIRS.iter().enumerate() {
                let other = planes[Plane::County as usize][idx(x + dx, y + dy)];
                if other != 0 && other != c {
                    planes[Plane::Flags as usize][i] |= flags::BOUNDARY;
                    if c > other && border.is_none() { border = Some(dir as u8); }
                }
            }
            if let Some(dir) = border {
                if planes[Plane::Flags as usize][i] == flags::BOUNDARY {
                    planes[Plane::GfxBank as usize][i] = 8;
                    planes[Plane::GfxIndex as usize][i] = 124 + dir;
                } else if planes[Plane::Flags as usize][i] == flags::ROAD | flags::BOUNDARY {
                    let frame = &mut planes[Plane::GfxIndex as usize][i];
                    if let Some(base) = match *frame {
                        0 => Some(128),
                        1 => Some(132),
                        14 => Some(136),
                        _ => None,
                    } {
                        *frame = base + dir;
                    }
                }
            }
        }
    }
    let mut bytes = Vec::with_capacity(SLOT_LEN);
    bytes.extend(planes.into_iter().flatten());
    // Frame 22 is open water in both campaign zoom levels.
    bytes.resize(bytes.len() + LATTICE_LEN, 22);
    bytes
}

pub(super) fn maps() -> Vec<u8> {
    let mut data = vec![0; SLOT_COUNT * SLOT_LEN];
    for w in WORLDS {
        let start = w.slot * SLOT_LEN;
        data[start..start + SLOT_LEN].copy_from_slice(&slot(w));
    }
    data
}

fn map_card(slot: &MapSlot<'_>) -> (Art, Art) {
    let mut owners = Art::new(128, 128);
    let mut relief = Art::new(128, 128);
    for y in 0..128 {
        for x in 0..128 {
            let county = slot.county_at((x / 2) as usize, (y / 2) as usize);
            let f = slot.flags_at((x / 2) as usize, (y / 2) as usize);
            if f & flags::NO_COUNTY == 0 { owners.dot(x, y, county); }
            let pen = if f & flags::NO_COUNTY != 0 {
                WATER + 2 + variant(x / 3, y / 2, 2, 2)
            } else if f & flags::CASTLE != 0 {
                13
            } else if f & flags::BOUNDARY != 0 {
                10
            } else if f & flags::ROUGH != 0 {
                11
            } else {
                12
            };
            relief.dot(x, y, pen);
        }
    }
    (owners, relief)
}

pub(crate) fn minimaps(maps: &MapSet<'_>, group: usize) -> Option<Vec<u8>> {
    let slots = group * 4..group * 4 + 4;
    if !slots.clone().any(|s| maps.slot(s).is_ok_and(|map| !map.is_empty())) { return None; }
    let mut frames = Vec::with_capacity(20);
    for s in slots {
        let (owners, relief) = match maps.slot(s).ok().filter(|map| !map.is_empty()) {
            Some(map) => map_card(&map),
            None => (Art::new(128, 128), Art::new(128, 128)),
        };
        frames.extend([owners, relief]);
        // The format reserves five frames per map; the current minimap uses two.
        frames.extend((0..3).map(|_| Art::new(1, 1)));
    }
    Some(sheet(frames))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::victory::{TRACK_FIRST, TRACK_SECOND};
    use l2_scenario::newgame::{self, NewGame};
    use l2_view::chrome::Minimap;

    #[test]
    fn standalone_campaign_counties_have_connected_sites_and_enough_seats() {
        let raw = maps();
        let maps = MapSet::parse(&raw).unwrap();
        for chapter in TRACK_FIRST.iter().chain(&TRACK_SECOND[2..]) {
            let slot = maps.slot(chapter.scenario).unwrap();
            let name = MAP_NAMES[chapter.scenario];
            let setup = NewGame {
                slot: chapter.scenario,
                lords: 1 + chapter.ai_lords() as usize,
                campaign: true,
                ..NewGame::default()
            };
            let world = newgame::build(&slot, &setup).unwrap();
            assert!(world.player_start_count >= setup.lords, "{name}: not enough starting castles");
            let cards = minimaps(&maps, chapter.scenario / 4).unwrap();
            let card = Minimap::load(&cards, chapter.scenario).unwrap();
            let mut visited = [false; PLANE_LEN];
            let mut pending = vec![world.town_tile[1]];
            while let Some(i) = pending.pop() {
                if std::mem::replace(&mut visited[i], true) { continue; }
                let (x, y) = (i as i32 % N, i as i32 / N);
                for (dx, dy) in DIRS {
                    let (x, y) = (x + dx, y + dy);
                    if (0..N).contains(&x) && (0..N).contains(&y) {
                        let next = idx(x, y);
                        if !visited[next] && world.tiles.flags[next] & flags::IMPASSABLE == 0 {
                            pending.push(next);
                        }
                    }
                }
            }
            for c in 1..=world.county_count {
                let (x, y) = (world.anchor[c].0 as usize, world.anchor[c].1 as usize);
                assert!(world.town_tile[c] != 0 && visited[world.town_tile[c]], "{name}: county {c} town is unreachable");
                assert!(world.castle_tile[c] != 0 && visited[world.castle_tile[c]], "{name}: county {c} castle is unreachable");
                assert!(world.dwelling_plots[c].iter().all(|p| *p != 0), "{name}: missing dwelling plots in county {c}");
                assert!(world.has_resource[c][0] && world.has_resource[c][2], "{name}: missing production site in county {c}");
                assert_eq!(world.farm_tile_count[c], 20, "{name}: overlapping footprints in county {c}");
                assert_eq!(card.counties[(y * 2) * 128 + (x * 2)], c as u8, "{name}: bad minimap in county {c}");
            }
        }
    }
}
