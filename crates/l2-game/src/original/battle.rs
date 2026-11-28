//! The tactical and village ateliers share the campaign's indexed palette. No
//! artwork is sampled from the legacy sheets; their indices are draw contracts.

use super::*;
use l2_sim::facing::FACING_DELTA;
use l2_sim::siege::{self, code};
use l2_sim::Troop;
use l2_view::{figures, village};

// The battle renderer selects a colour bank by shield index, not a remap at
// blit time. The same order is used by the keep's banners and the overview.
const LIVERY: [u8; 6] = [CREAM, RED, GOLD, SLATE, VIOLET, BLUE];

pub(super) fn asset(name: &str) -> Option<Vec<u8>> {
    Some(match name {
        "t32_bat1.pl8" => terrain_tiles(TileBank::Field, false),
        "t32_stn1.pl8" => terrain_tiles(TileBank::Castle(true), false),
        "t32_wod1.pl8" => terrain_tiles(TileBank::Castle(false), false),
        "t32_stn2.pl8" | "t32_wod2.pl8" => terrain_tiles(TileBank::CastleGround, false),
        "t2_bat1.pl8" => terrain_tiles(TileBank::Field, true),
        "t2_stn1.pl8" => terrain_tiles(TileBank::Castle(true), true),
        "t2_wod1.pl8" => terrain_tiles(TileBank::Castle(false), true),
        "t2_stn2.pl8" | "t2_wod2.pl8" => terrain_tiles(TileBank::CastleGround, true),
        "t2_spri.pl8" => sheet((0..7).map(overview_man)),
        "batfield.pl8" => sheet((0..48).map(field_layout)),
        "batfiel2.pl8" => sheet((0..20).map(field_layout)),
        "stnfield.pl8" | "stnfiel2.pl8" => sheet((0..20).flat_map(castle_layout)),
        "a2_horse.pl8" => sheet((0..48).map(horse)),
        "a2_miss.pl8" => sheet((0..81).map(missile)),
        "engine.pl8" => sheet((0..46).map(engine)),
        "catarm1.pl8" => sheet((0..20).map(|f| catapult_arm(f / 5, f % 5))),
        "catarm2.pl8" => sheet((0..20).map(|f| catapult_arm(4 + f / 5, f % 5))),
        "vill.pl8" => sheet([village_scene()]),
        "villtops.pl8" => sheet((0..6).map(village_weather)),
        "vill_gd8.pl8" => sheet([village_grid()]),
        "villani1.pl8" => sheet((0..18).map(mine_animation)),
        "villani2.pl8" => sheet((0..44).map(village_animation)),
        _ => {
            let (letter, kind) = name.strip_prefix("a2")?.strip_suffix(".pl8")?.split_once('_')?;
            let team = LIVERY[match letter {
                "w" => 0,
                "r" => 1,
                "y" => 2,
                "k" => 3,
                "p" => 4,
                "b" => 5,
                _ => return None,
            }];
            let troop = match kind {
                "psnt" => Troop::Peasants,
                "cros" => Troop::Crossbowmen,
                "mace" => Troop::Macemen,
                "swor" => Troop::Swordsmen,
                "pike" => Troop::Pikemen,
                "arch" => Troop::Archers,
                "knig" => Troop::Knights,
                _ => return None,
            };
            let count = if troop == Troop::Knights {
                56
            } else {
                usize::from(figures::poses_per_facing(troop)) * 8 + 18
            };
            sheet((0..count).map(|f| soldier(troop, team, f)))
        }
    })
}

#[derive(Clone, Copy)]
enum TileBank {
    Field,
    CastleGround,
    Castle(bool),
}

fn terrain_tiles(bank: TileBank, overview: bool) -> Vec<u8> {
    let count = if matches!(bank, TileBank::Field) { 252 } else { 256 };
    sheet((0..count).map(|i| {
        let tile = match bank {
            TileBank::Field => field_tile(i),
            TileBank::CastleGround => castle_ground(i),
            TileBank::Castle(stone) => castle_tile(i, stone),
        };
        if !overview {
            return tile;
        }
        let mut mini = Art::new(2, 2);
        // A majority ramp per quadrant keeps the minimap consistent with the
        // full scene without amplifying an isolated grass blade or glint.
        for y in 0..2 {
            for x in 0..2 {
                let mut ramps = [0u16; 32];
                let mut shades = [0u16; 32];
                for py in y * 16..(y + 1) * 16 {
                    for px in x * 16..(x + 1) * 16 {
                        let colour = tile.pixels[(py * 32 + px) as usize];
                        let ramp = usize::from(colour / 8);
                        ramps[ramp] += 1;
                        shades[ramp] += u16::from(colour % 8);
                    }
                }
                let ramp = (0..ramps.len()).max_by_key(|&r| ramps[r]).unwrap();
                mini.dot(x, y, (ramp * 8) as u8 + (shades[ramp] / ramps[ramp]) as u8);
            }
        }
        mini
    }))
}

fn overview_man(i: usize) -> Art {
    let mut a = Art::new(2, 2);
    if i != 0 {
        let color = LIVERY.get(i).copied().unwrap_or(CREAM);
        a.rect(0, 0, 2, 2, color + 6);
        a.dot(0, 0, color + 7);
    }
    a
}

fn meadow(tile: usize) -> Art {
    let mut a = Art::new(32, 32);
    let seed = tile as u32;
    for y in 0..32 {
        for x in 0..32 {
            let n = noise(x, y, seed);
            let broad = ((x + y * 2 + (seed % 4) as i32) / 6) & 1;
            let ramp = if n % 31 < 3 { MOSS } else { GRASS };
            a.dot(x, y, ramp + 2 + broad as u8 + (n % 3) as u8);
        }
    }
    for k in 0..11 {
        let n = noise(k, 1, seed);
        let (x, y) = (2 + (n % 28) as i32, 3 + ((n >> 8) % 28) as i32);
        a.line(x, y, x - 1, y - 2, GRASS + 1);
        a.dot(x + 1, y - 1, GRASS + 6);
        if n & 7 == 0 {
            a.dot(x - 1, y - 3, PARCHMENT + 5);
        }
    }
    a
}

// North, east, south, west. This is the graphic index, not a simulation mask.
const N: u8 = 1;
const E: u8 = 2;
const S: u8 = 4;
const W: u8 = 8;
const ALL: u8 = N | E | S | W;
const CARDINAL: [u8; 4] = [N, E, S, W];
const CORNERS: [u8; 4] = [N | E, E | S, S | W, W | N];

fn edge_distance(x: i32, y: i32) -> [i32; 4] {
    [y, 31 - x, 31 - y, x]
}

pub(super) fn water_edges(i: usize) -> (u8, u8) {
    let missing = match i {
        0xa8..=0xab => N,
        0xac..=0xad | 0xc7 => N | E,
        0xae..=0xb1 | 0xce..=0xd0 => E,
        0xb2..=0xb3 | 0xc8 => E | S,
        0xb4..=0xb7 | 0xd1..=0xd3 => S,
        0xb8..=0xb9 | 0xc9 => S | W,
        0xba..=0xbd | 0xd4..=0xd6 => W,
        0xbe..=0xbf | 0xca => W | N,
        0xc0 => N | E | S,
        0xc1 => E | S | W,
        0xc2 => S | W | N,
        0xc3 => W | N | E,
        0xc4 => ALL,
        0xc5 => N | S,
        0xc6 => E | W,
        0xcb..=0xcd => N,
        _ => 0,
    };
    let notches: u8 = match i {
        0xc7 | 0xcb | 0xcf => 4,
        0xc8 | 0xce | 0xd2 => 8,
        0xc9 | 0xd1 | 0xd5 => 1,
        0xca | 0xcc | 0xd4 => 2,
        0xcd => 6,
        0xd0 => 12,
        0xd3 => 9,
        0xd6 => 3,
        0xd7..=0xda => 1 << (i - 0xd7),
        0xdb => 3,
        0xdc => 6,
        0xdd => 12,
        0xde => 9,
        0xdf => 5,
        0xe0 => 10,
        0xe1 => 7,
        0xe2 => 14,
        0xe3 => 13,
        0xe4 => 11,
        0xe5 => 15,
        _ => 0,
    };
    (missing, notches)
}

fn water_tile(i: usize) -> Art {
    let mut a = meadow(i);
    let (missing, notches) = water_edges(i);
    for y in 0..32 {
        for x in 0..32 {
            let n = noise(x, y, i as u32);
            let d = edge_distance(x, y);
            let mut wet = 32;
            for k in 0..4 {
                if missing & CARDINAL[k] != 0 {
                    wet = wet.min(d[k] - 4 - (n % 3) as i32);
                }
                if notches & (1 << k) != 0 {
                    wet = wet.min(d[k] + d[(k + 1) % 4] - 5 - (n % 2) as i32);
                }
            }
            let ripple = (y * 3 + x / 3 + (n & 1) as i32 + i as i32) % 17;
            let c = match wet {
                ..=-3 => continue,
                -2 | -1 => MOSS + 2 + (n % 3) as u8,
                0..=2 => WATER + 5 + (n % 2) as u8,
                3..=6 => WATER + 3 + (n % 3) as u8,
                _ if ripple <= 1 && n % 9 < 7 => WATER + 5,
                _ => WATER + 1 + (n % 3) as u8,
            };
            a.dot(x, y, c);
        }
    }
    a
}

fn rock(a: &mut Art, x: i32, y: i32, r: i32, seed: u32) {
    let q = (r / 2).max(2);
    let lift = (noise(x, y, seed) & 3) as i32;
    a.ellipse(x + 2, y + 1, r + 1, q, MOSS + 1);
    a.poly(&[(x-r,y), (x-r+2,y-q), (x-1,y-r-lift), (x+q,y-r), (x+r,y-q), (x+r-1,y), (x,y+q)], INK + 1);
    a.poly(&[(x-r+1,y-1), (x-r+3,y-q), (x-1,y-r-lift+1), (x+q,y-r+1), (x+r-1,y-q), (x+q-1,y), (x,y+q-1)], STONE + 3);
    a.poly(&[(x-r+3,y-q), (x-1,y-r-lift+1), (x+q,y-r+1), (x+q-1,y-q), (x+1,y-2)], STONE + 5);
    a.line(x-1,y-r-lift+2,x+q-1,y-r+2,STONE + 6);
    a.line(x+2,y-2,x+q-1,y,STONE + 1);
    a.dot(x-r+3,y-1,MOSS + 5);
}

fn tree(a: &mut Art, x: i32, y: i32, size: i32, spruce: bool, seed: u32) {
    let half = (size * 2 / 3).max(2);
    a.ellipse(x + size / 3, y + 1, half + 1, size / 3 + 1, MOSS + 1);
    a.line(x, y, x, y - size, WOOD + 2);
    a.dot(x + 1, y - 1, WOOD + 5);
    let top = y - size * 2;
    if spruce {
        for (h, w, c) in [(0, half, PINE + 2), (size / 2, half - 1, PINE + 3), (size, half / 2 + 1, PINE + 4)] {
            let py = y - size / 3 - h;
            a.poly(&[(x-w-1,py), (x-1,top-h/3-1), (x+w+1,py), (x,py+2)], PINE);
            a.poly(&[(x-w,py-1), (x-1,top-h/3), (x+w,py-1), (x,py+1)], c);
            a.line(x-w+1, py-1, x-1, top-h/3+1, c+1);
        }
    } else {
        for (dx, dy, r) in [(-half/2,-size,half-1), (half/2,-size,half-1), (-1,-size-half/2,half)] {
            a.ellipse(x+dx, y+dy, r+1, r+1, PINE + 1);
            a.ellipse(x+dx-1, y+dy-1, r, r, PINE + 3);
            a.ellipse(x+dx-2, y+dy-2, r/2+1, r/2+1, PINE + 5);
        }
        for k in 0..size {
            let n = noise(x+k, y, seed);
            let dx = (n % (half as u32 + 1)) as i32 - half / 2;
            let dy = ((n >> 8) % (half as u32 + 1)) as i32 - half / 2;
            a.dot(x+dx, y-size+dy, PINE + 4 + (n % 3) as u8);
        }
    }
}

fn hill_tile(i: usize) -> Art {
    let mut a = meadow(i);
    let slopes = match i {
        0x50..=0x53 => N,
        0x54..=0x57 => N | E,
        0x58..=0x5b => E,
        0x5c..=0x5f => E | S,
        0x60..=0x63 => S,
        0x64..=0x67 => S | W,
        0x68..=0x6b => W,
        0x6c..=0x6f => W | N,
        0x9f => ALL,
        _ => 0,
    };
    for y in 0..32 {
        for x in 0..32 {
            let n = noise(x, y, i as u32);
            let d = edge_distance(x, y);
            let mut slope = 31;
            for k in 0..4 {
                if slopes & CARDINAL[k] != 0 {
                    slope = slope.min(d[k]);
                }
            }
            if slope <= 2 + (n % 3) as i32 {
                continue;
            }
            let contour = (x + 3 * y + (n % 4) as i32) % 14;
            let shade = if slope < 10 { WOOD + 2 } else { MOSS + 2 };
            a.dot(x, y, shade + if contour < 3 { 2 } else { (n % 2) as u8 });
        }
    }
    if slopes != ALL || i % 2 == 0 {
        rock(&mut a, 10 + (i % 10) as i32, 19, 4 + (i % 4) as i32, i as u32);
        rock(&mut a, 22, 25, 4, i as u32 + 7);
    } else {
        rock(&mut a, 16, 21, 7, i as u32);
    }
    a
}

pub(super) fn road_mask(i: usize) -> u8 {
    match i {
        0 | 1 => N | S,
        2 | 3 => E | W,
        4..=7 => CORNERS[[0, 1, 2, 3][i - 4]],
        8 => N,
        9 => S,
        10 => E,
        11 => W,
        12 | 13 => 0,
        14 => ALL ^ W,
        15 => ALL ^ N,
        16 => ALL ^ E,
        17 => ALL ^ S,
        _ => ALL,
    }
}

fn plank(a: &mut Art, x: i32, y: i32, w: i32, h: i32, salt: u32) {
    a.rect(x, y, w, h, WOOD + 1);
    for row in (0..h).step_by(5) {
        let n = noise(x, y + row, salt);
        a.rect(x + 1, y + row + 1, w - 2, (h - row - 1).min(4), WOOD + 3 + (n % 3) as u8);
        if row + 1 < h {
            a.line(x + 1, y + row + 1, x + w - 2, y + row + 1, WOOD + 6);
        }
        if w > 8 && row + 3 < h {
            let k = (n % (w - 4) as u32) as i32;
            a.line(x + 2 + k / 2, y + row + 3, x + 2 + k, y + row + 3, WOOD + 2);
            a.dot(x + w - 3, y + row + 3, IRON + 2);
        }
    }
}

pub(super) fn bridge_layout(i: usize) -> (usize, i32, i32) {
    let col = (i - 0x8c) % 4;
    let water = match i {
        0x8c..=0x8f => 0xa8,
        0x9c..=0x9e => 0xb4,
        _ => 0xa0,
    };
    let (left, right) = match col { 0 => (19, 32), 3 => (0, 13), _ => (0, 32) };
    (water, left, right)
}

fn bridge_tile(i: usize) -> Art {
    let (water, left, right) = bridge_layout(i);
    let mut a = water_tile(water);
    plank(&mut a, left - 1, 0, right - left + 2, 32, i as u32);
    if left != 0 || right != 32 {
        let x = if left != 0 { left } else { right };
        a.rect(x - 3, 0, 6, 32, WOOD + 1);
        a.rect(x - 2, 0, 3, 32, WOOD + 6);
        for y in [4, 21] {
            a.rect(x - 4, y, 8, 7, WOOD + 2);
            a.line(x - 3, y + 1, x + 2, y + 1, GOLD + 4);
            a.dot(x + 1, y + 3, IRON + 2);
        }
    }
    a
}

fn field_tile(i: usize) -> Art {
    match i {
        0x40..=0x6f | 0x9f => hill_tile(i),
        0x8c..=0x9e => bridge_tile(i),
        0xa0..=0xe5 => water_tile(i),
        _ => {
            let mut a = meadow(i);
            match i {
                0x10..=0x23 => {
                    let links = road_mask(i - 0x10);
                    for k in 0..5 {
                        let n = noise(k, 11, i as u32);
                        let (x, y) = (5 + (n % 23) as i32, 12 + ((n >> 8) % 19) as i32);
                        if k == 4 && i >= 0x20 {
                            rock(&mut a, x, y, 6, n);
                        } else if k < 3 || links.count_ones() >= 2 {
                            tree(&mut a, x, y, 4 + (n % 4) as i32, n & 8 != 0, n);
                        }
                    }
                }
                0x24..=0x3f | 0x7c..=0x8b => {
                    for k in 0..4 {
                        let n = noise(k, 21, i as u32);
                        let x = 5 + k * 7 + (n & 3) as i32;
                        let y = 16 + ((n >> 4) % 11) as i32;
                        if i >= 0x80 && k % 2 == 0 {
                            tree(&mut a, x, y, 7, true, n);
                        } else {
                            rock(&mut a, x, y, 3 + ((n >> 9) % 4) as i32, n);
                        }
                    }
                }
                0x70..=0x7b => {
                    let edge = match i { 0x70..=0x74 => W, 0x75 => N | W, 0x76..=0x7a => N, _ => N | W };
                    for y in 0..32 {
                        for x in 0..32 {
                            let n = noise(x, y, i as u32);
                            if (edge & N != 0 && y < (n % 6) as i32) || (edge & W != 0 && x < (n % 6) as i32) {
                                a.dot(x, y, MOSS + 1 + (n % 4) as u8);
                            }
                        }
                    }
                }
                0xe6..=0xf8 => {
                    let links = road_mask(i - 0xe6);
                    for y in 0i32..32 {
                        for x in 0i32..32 {
                            let (h, v) = ((x - 16).abs(), (y - 16).abs());
                            let hor = if x < 16 { W } else { E };
                            let vert = if y < 16 { N } else { S };
                            if (h < 4 && vert & links != 0) || (v < 4 && hor & links != 0) || h + v < 5 {
                                let n = noise(x, y, i as u32);
                                a.dot(x, y, STONE + 2 + (n % 4) as u8);
                            }
                        }
                    }
                }
                _ => {}
            }
            a
        }
    }
}

fn castle_ground(i: usize) -> Art {
    match i {
        0xa0..=0xe5 => water_tile(i),
        // Ground-overlay strips for elevated damage and loose rubble; index
        // zero remains transparent when this layer is clipped over a wall.
        0x8b..=0x9a => {
            let mut a = Art::new(32, 32);
            for k in 0..3 {
                let n = noise(k, 0, i as u32);
                rock(&mut a, 7 + k * 8, 19 + (n % 9) as i32, 3, n);
            }
            a
        }
        _ => meadow(i),
    }
}

fn masonry(a: &mut Art, x: i32, y: i32, w: i32, h: i32, seed: u32) {
    a.rect(x, y, w, h, STONE + 1);
    for row in (0..h).step_by(6) {
        let offset = (row / 6 % 2) * 4;
        for col in (-offset..w).step_by(8) {
            let left = col.max(1);
            let right = (col + 7).min(w - 1);
            let b = STONE + 2 + (noise(col, row, seed) % 3) as u8;
            if right > left {
                a.rect(x + left, y + row + 1, right - left, 5.min(h - row - 1), b);
                if row + 1 < h {
                    a.line(x + left, y + row + 1, x + right - 1, y + row + 1, b + 2);
                }
            }
        }
    }
}

fn castle_tile(i: usize, stone: bool) -> Art {
    let table = if stone { &siege::STRUCTURE_STONE } else { &siege::STRUCTURE_WOOD };
    let kind = table[i * 2];
    let seed = i as u32;
    let mut a = meadow(i);
    // These frame IDs are stamped by the simulation after a drawbridge is
    // lowered, not picked from the structure raster in the standing castle.
    if stone && siege::DRAWBRIDGE_FRAMES.contains(&(i as u8)) {
        if i != 197 {
            a = water_tile(0xa0);
            let left = if i == 198 { 0 } else { 6 + (i - 192) as i32 * 4 };
            plank(&mut a, left, 0, 33 - left, 32, seed);
            a.rect(left, 0, 2, 32, WOOD + 6);
        }
        return a;
    }
    if i != 0 && siege::DOCK_FRAMES.contains(&(i as u8)) {
        a.ellipse(16, 22, 17, 10, MOSS + 1);
        plank(&mut a, -1, 2, 34, 27, seed);
        a.line(1, 3, 30, 3, GOLD + 5);
        a.line(1, 27, 30, 27, WOOD + 1);
        return a;
    }
    if stone && (204..=207).contains(&i) {
        a.rect(0, 4, 32, 24, PARCHMENT + 3);
        for k in 0..4 {
            let n = noise(k, 4, seed);
            let x = 4 + k * 8;
            let y = 10 + (n % 18) as i32;
            a.line(x - 2, y + 2, x + 4, y, WOOD + 2);
            a.line(x - 2, y + 1, x + 4, y - 1, WOOD + 6);
            a.dot(x, y + 1, IRON + 4);
        }
        return a;
    }
    if kind == code::WALL && i == if stone { 173 } else { 119 } {
        a.rect(9, 0, 17, 32, MOSS + 1);
        if stone {
            masonry(&mut a, 8, -1, 16, 34, seed);
        } else {
            plank(&mut a, 8, -1, 16, 34, seed);
        }
        let c = if stone { STONE } else { WOOD };
        a.line(8, 0, 8, 31, c + 6);
        for y in [1, 11, 22] {
            a.rect(6, y, 5, 7, c + 3);
            a.rect(20, y, 5, 7, c + 3);
            a.line(6, y, 10, y, c + 6);
            a.line(20, y, 24, y, c + 6);
        }
        return a;
    }
    if kind == code::WALL && i == if stone { 174 } else { 120 } {
        let c = if stone { STONE } else { WOOD };
        if stone {
            masonry(&mut a, 1, 3, 30, 28, seed);
        } else {
            plank(&mut a, 1, 3, 30, 28, seed);
        }
        a.rect(5, 5, 22, 13, c + 2);
        a.rect(8, 7, 16, 8, c + 4);
        a.line(1, 3, 29, 3, c + 7);
        a.line(1, 18, 29, 18, c + 6);
        for x in [1, 11, 23] {
            a.rect(x, 15, 7, 6, c + 3);
            a.line(x, 15, x + 6, 15, c + 6);
        }
        a.rect(14, 24, 4, 5, INK + 1);
        return a;
    }
    match kind {
        code::KEEP => {
            a.ellipse(19, 24, 16, 7, MOSS + 1);
            if stone {
                masonry(&mut a, 2, 4, 28, 24, seed);
                a.rect(1, 4, 30, 5, STONE + 6);
                for x in [1, 9, 21, 29] {
                    a.rect(x, 1, 3, 7, STONE + 5);
                }
            } else {
                plank(&mut a, 1, 8, 30, 20, seed);
                a.poly(&[(0,12),(15,1),(31,11),(15,16)], WOOD + 1);
                a.line(0,12,15,2,WOOD + 6);
                a.line(15,2,31,11,WOOD + 4);
            }
            a.ellipse(16, 20, 5, 6, INK + 1);
            a.rect(11, 20, 11, 8, INK + 1);
            a.line(9, 28, 24, 28, STONE + 5);
            a.line(11, 30, 21, 30, STONE + 3);
            a.dot(9, 17, GOLD + 6);
            a.dot(24, 17, GOLD + 6);
        }
        code::WALL if stone => {
            masonry(&mut a, 0, 9, 32, 22, seed);
            a.rect(0, 8, 32, 5, STONE + 5);
            for x in [0, 10, 21] {
                a.rect(x, 5, 7, 6, STONE + 6);
                a.line(x, 5, x + 6, 5, STONE + 7);
                a.line(x + 6, 6, x + 6, 10, STONE + 3);
            }
            a.line(0, 29, 31, 29, STONE + 1);
        }
        code::WALL => {
            a.ellipse(15, 26, 18, 5, MOSS + 1);
            for x in (0..32).step_by(4) {
                let top = 6 + (noise(x, 2, seed) % 3) as i32;
                a.poly(&[(x,27),(x,top+3),(x+2,top),(x+4,top+3),(x+4,27)], WOOD + 1);
                a.line(x + 1, top + 3, x + 1, 26, WOOD + 5);
                a.line(x + 2, top + 3, x + 2, 26, WOOD + 3);
            }
            a.rect(0, 16, 32, 3, WOOD + 2);
            a.line(0, 16, 31, 16, WOOD + 6);
            for x in [3, 11, 19, 27] { a.dot(x, 17, IRON + 2); }
        }
        code::DRAWBRIDGE => {
            plank(&mut a, -1, 4, 34, 25, seed);
            a.rect(0, 7, 32, 3, IRON + 2);
            a.rect(0, 23, 32, 3, IRON + 1);
            for x in [4, 12, 20, 28] {
                a.dot(x, 8, GOLD + 4);
                a.dot(x, 24, GOLD + 4);
            }
        }
        code::TEN => {
            masonry(&mut a, 0, 0, 32, 32, seed);
            a.rect(1, 1, 30, 25, SLATE + 2);
            for y in (2..26).step_by(5) {
                a.line(2, y, 29, y, SLATE + 3);
                a.line(3 + y % 5, y + 1, 3 + y % 5, y + 4, SLATE + 1);
            }
            a.line(0, 0, 31, 0, STONE + 6);
        }
        2..=5 | code::SEVEN | code::RAISED_1 | code::RAISED_2 => {
            if stone {
                masonry(&mut a, 0, 0, 32, 32, seed);
                a.rect(0, 0, 32, 3, STONE + 4);
                a.line(0, 0, 31, 0, STONE + 6);
                a.line(0, 29, 31, 29, STONE + 1);
                a.dot(3, 26, MOSS + 3);
            } else {
                plank(&mut a, -1, 0, 34, 32, seed);
                a.line(0, 0, 0, 31, WOOD + 2);
                a.line(31, 0, 31, 31, WOOD + 2);
            }
        }
        1 => {
            a.rect(0, 0, 32, 32, PARCHMENT + 2);
            for y in 0..32 {
                for x in 0..32 {
                    let n = noise(x, y, seed);
                    a.dot(x, y, PARCHMENT + 1 + (n % 3) as u8);
                }
            }
            for k in 0..5 {
                let n = noise(k, 0, seed);
                let (x, y) = ((n % 31) as i32, ((n >> 8) % 31) as i32);
                a.line(x, y, x + 2, y, STONE + 3);
                a.dot(x, y - 1, STONE + 6);
            }
        }
        _ => {}
    }
    a
}

const MAP: i32 = l2_sim::terrain::DIM as i32;

fn field_layout(i: usize) -> Art {
    let mut a = Art::new(MAP, MAP);
    let seed = i as u32;
    let variant = i % 6;
    // The army approaches stay predominantly clear. Terrain clusters have an
    // irregular edge and the river always leaves two routes across the map.
    for y in 16..64 {
        for x in 1..79 {
            let h = noise(x / 3, y / 3, seed);
            let d = |cx: i32, cy: i32, sx: i32, sy: i32| {
                let (dx, dy) = ((x - cx) * 16 / sx, (y - cy) * 16 / sy);
                dx * dx + dy * dy < 238 + (h % 63) as i32
            };
            let shift = (i / 6 % 7) as i32 - 3;
            let trees = d(12 + shift, 32, 9, 12)
                || d(64 + shift, 46, 12, 10)
                || variant == 1 && d(46, 38 + shift, 10, 7)
                || variant == 4 && d(24, 53, 8, 6);
            let ridge = variant != 1 && (d(58 - shift, 26, 12, 5) || d(16, 52 + shift, 9, 4))
                || variant == 3 && d(34 + shift, 37, 8, 4);
            let lake = variant == 2 && d(53, 44 + shift, 13, 7)
                || variant == 5 && d(18, 38, 8, 12);
            let river_y = 39 + ((x * 5 + shift * 5).rem_euclid(67) - 33).abs() / 8;
            let river = matches!(variant, 0 | 4)
                && (y - river_y).abs() <= 1 + (h % 2) as i32
                && !(10..=17).contains(&x) && !(39..=44).contains(&x);
            a.dot(x, y, if river || lake {
                0x09
            } else if ridge {
                0x02
            } else if trees {
                0x0a
            } else if (variant == 3 || variant == 5) && h % 83 == 0 {
                0x24 + (h % 24) as u8
            } else {
                0
            });
        }
    }
    // Leave a proper four-tile masonry bridge in the wider crossing.
    if matches!(variant, 0 | 4) {
        a.rect(34, 38, 14, 6, 0x09);
        a.dot(39, 37, 0x10);
        a.dot(39, 39, 0x12);
        a.dot(39, 40, 0x12);
        a.dot(39, 41, 0x14);
        a.rect(39, 43, 4, 2, 0);
    }
    for (side, y, marker) in [(0, 9, 0x04), (1, 67, 0x0f)] {
        for k in 0..12 {
            let x = 13 + (k % 6) * 9;
            let py = y + (k / 6) * 5;
            a.dot(x, py, marker);
            a.dot(x, py + 1, marker);
            a.dot(x + 2, py, 0x40 + k as u8);
        }
        let end_y = if side == 0 { 64 } else { 17 };
        for k in 0..4 {
            let x = 19 + k * 12;
            a.dot(x, end_y, marker);
            a.dot(x, end_y + 1, 0x07);
            if k != 0 { a.dot(x + k, end_y + 1, 0x08); }
        }
        for group in 0..2 {
            let ry = if side == 0 { 15 + group * 3 } else { 61 - group * 3 };
            for k in 0..3 {
                let x = 27 + k * 10;
                a.dot(x, ry, marker);
                a.dot(x + 1, ry, [0x41, 0, 0x42][k as usize]);
                a.dot(x, ry + 1, 0x08);
                if group != 0 { a.dot(x + 1, ry + 1, 0x07); }
            }
        }
    }
    a
}

fn castle_layout(i: usize) -> [Art; 2] {
    use l2_sim::castle::{ESCAPE_GROUND, ESCAPE_MOAT, MARKER_SIDE0, MARKER_SIDE4};

    let stone = i > 1;
    let (cx, cy) = (40 + (i / 5 % 3) as i32 - 1, 23);
    let (hw, hh) = (13 + (i % 5) as i32, 11 + (i % 3) as i32);
    let (left, right, north, south) = (cx - hw, cx + hw, cy - hh, cy + hh);
    let mut a = Art::new(MAP, MAP);
    a.rect(0, 0, MAP, MAP, ESCAPE_GROUND);
    let (curtain, flank, tower, walk) = if stone { (172, 173, 174, 113) } else { (118, 119, 120, 20) };
    let (gate_l, gate_r) = (cx - 2, cx + 1);
    let opening = if i == 0 || i == 3 { Some(cx + hw / 2) } else { None };
    let ditch = if stone { 1 + i32::from(i >= 4 && i % 2 == 0) } else { 0 };
    for y in north - ditch..=south + ditch {
        for x in left - ditch..=right + ditch {
            let (dx, dy) = ((x - cx).abs(), (y - cy).abs());
            let near_gate = (gate_l..=gate_r).contains(&x);
            let near_gap = opening.is_some_and(|g| (x - g).abs() <= 1);
            let frame = if dx > hw || dy > hh {
                if y >= south && (near_gate || near_gap) { 1 } else { ESCAPE_MOAT }
            } else if dy == hh {
                if y == south && near_gate && i >= 3 {
                    164 + (x - gate_l) as u8
                } else if y == south && near_gap {
                    1
                } else if dx == hw || dx % 7 == 0 {
                    tower
                } else {
                    curtain
                }
            } else if dx == hw {
                flank
            } else if dx == hw - 1 || y == north + 1
                || (y == south - 1 && !near_gate && !near_gap && dx > hw / 3)
            {
                // Elevation two is a functioning dock/boiling-oil rampart;
                // an elevation-one stair every seventh cell links the walk.
                if stone && (x + y) % 7 == 0 { 220 } else { walk + ((x + y) % 2) as u8 }
            } else if dx <= 2 || y >= south - 3 {
                1 + ((x + y) % 3) as u8
            } else {
                ESCAPE_GROUND
            };
            a.dot(x, y, frame);
        }
    }
    // Stepped stone forecourt: the door is elevation four in the builder.
    // A walkable stair on every side lets both sides reach its neighbours.
    let keep_y = cy - 3;
    if stone {
        for y in keep_y - 3..=keep_y + 3 {
            for x in cx - 3..=cx + 3 {
                let d = (x - cx).abs().max((y - keep_y).abs());
                let floor = match d { 1 => 16, 2 => 113, _ => 220 };
                a.dot(x, y, floor);
            }
        }
        // The bastion seeds the game's bailey surface flood immediately
        // beside the court, even though the steps separate it from the keep.
        a.dot(cx + 4, keep_y, 48);
    }
    a.dot(cx, keep_y, 0);

    let mut markers = Art::new(MAP, MAP);
    let mut deploy = |side, slot, x, y| {
        markers.dot(x - 2, y, side);
        markers.dot(x - 1, y, side);
        markers.dot(x, y, 0x40 + slot as u8);
    };
    for slot in 0..12 {
        let dx = [-2, 2, -6, 6, -10, 10][(slot % 6) as usize];
        deploy(MARKER_SIDE0, slot, cx + dx, south - 4 + (slot / 6) * 2);
        deploy(MARKER_SIDE4, slot, cx - 15 + (slot % 6) * 6, 65 + (slot / 6) * 5);
    }
    let mut mark = |side, kind, slot: usize, bits: usize, x, y| {
        markers.dot(x - 2, y - 2, side);
        markers.dot(x - 1, y - 2, kind);
        for b in 0..bits {
            if slot & (1 << (bits - 1 - b)) != 0 {
                markers.dot(x - 2 + b as i32, y - 1, 7);
            }
        }
    };
    for (kind, spread) in [(0x40, hw - 1), (0x41, hw - 5), (0x44, 3), (0x47, hw + 4)] {
        for slot in 0..16 {
            let x = cx + if slot % 2 == 0 { -spread } else { spread };
            mark(MARKER_SIDE0, kind, slot, 4, x, north + 3 + (slot / 2) as i32 * 2);
        }
    }
    for (kind, y) in [(0x40, 62), (0x41, 59), (0x47, 55), (0x44, south + ditch + 5)] {
        for (lane, dx) in [-12, -4, 4, 12].into_iter().enumerate() {
            mark(MARKER_SIDE4, kind, lane, 2, cx + dx, y);
        }
    }
    markers.dot(cx, cy + 2, MARKER_SIDE4);
    markers.dot(cx + 1, cy + 2, 0x43);
    [a, markers]
}

type Point = (i32, i32);

fn stroke(a: &mut Art, p: Point, q: Point, colour: u8) {
    a.line(p.0, p.1, q.0, q.1, INK + 1);
    a.line(p.0, p.1 - 1, q.0, q.1 - 1, colour);
    a.line(p.0 + 1, p.1, q.0 + 1, q.1, colour);
}

fn leonine_shield(a: &mut Art, x: i32, y: i32, team: u8, round: bool) {
    if round {
        a.ellipse(x, y, 5, 6, INK + 1);
        a.ellipse(x, y, 4, 5, GOLD + 5);
        a.ellipse(x, y, 3, 4, team + 4);
    } else {
        a.poly(&[(x-6,y-5),(x+5,y-5),(x+5,y+2),(x+2,y+6),(x,y+8),(x-5,y+2)], INK + 1);
        a.poly(&[(x-4,y-4),(x+4,y-4),(x+4,y+1),(x,y+6),(x-4,y+1)], GOLD + 5);
        a.poly(&[(x-3,y-3),(x+3,y-3),(x+3,y+1),(x,y+5),(x-3,y+1)], team + 4);
    }
    a.line(x - 2, y + 1, x, y - 1, team + 7);
    a.line(x, y - 1, x + 1, y + 2, team + 7);
    a.dot(x + 1, y - 2, CREAM + 6);
}

fn horse(frame: usize) -> Art {
    let (fx, fy) = FACING_DELTA[frame / 6];
    let gait: i32 = [0, -2, -3, 0, 3, 2][frame % 6];
    let mut a = Art::new(60, 58);
    let x = 30;
    let y = 36 - (gait.abs() / 2);
    let chest = (x + fx * 9, y + fy * 4 - 2);
    let rump = (x - fx * 9, y - fy * 4);
    a.ellipse(x + 1, 47, 17, 4, INK + 4);
    // Four grounded hooves. The near legs carry the stronger chestnut light.
    for (hip, step, light) in [(rump, gait, 3), (chest, -gait, 3), (rump, -gait, 5), (chest, gait, 5)] {
        let back = if light == 3 { -3 } else { 2 };
        let knee = (hip.0 + back + fx * step, hip.1 + 7);
        let foot = (knee.0 + fx * step / 2, 46 + fy * step / 2);
        stroke(&mut a, (hip.0 + back, hip.1), knee, WOOD + light);
        stroke(&mut a, knee, foot, WOOD + light - 1);
        a.rect(foot.0 - 1, foot.1, 3, 2, INK + 1);
    }
    let tail = (rump.0 - fx * 6 - 1, rump.1 - fy * 2 + 2);
    a.poly(&[(rump.0,rump.1-4), (tail.0-3,tail.1), (tail.0-2,tail.1+8), (tail.0+1,tail.1+11), (tail.0+2,tail.1+3)], WOOD + 1);
    a.line(tail.0, tail.1 + 3, tail.0, tail.1 + 8, WOOD + 3);
    let (rx, ry) = (11 + 5 * fx.abs(), 6 + 3 * fy.abs());
    a.ellipse(x, y, rx, ry, WOOD);
    a.ellipse(x, y - 1, rx - 1, ry - 1, WOOD + 3);
    a.ellipse(x - 3, y - 3, rx - 5, ry - 4, WOOD + 5);
    a.line(x - 5, y + ry - 3, x + 6, y + ry - 3, WOOD + 1);
    let head = (x + fx * 14, y + fy * 6 - 12);
    a.poly(&[(chest.0-5,chest.1+3), (head.0-4,head.1+1), (head.0+4,head.1+1), (chest.0+6,chest.1+3)], WOOD + 1);
    a.poly(&[(chest.0-3,chest.1+1), (head.0-3,head.1+1), (head.0+2,head.1+3), (chest.0+4,chest.1+1)], WOOD + 5);
    let muzzle = (head.0 + fx * 4, head.1 + 2 + fy * 2);
    a.ellipse(head.0, head.1, 4, 5, WOOD + 2);
    a.ellipse(muzzle.0, muzzle.1, 3, 3, WOOD + 4);
    a.line(head.0 - 2, head.1 - 3, head.0 - 3, head.1 - 8, WOOD + 1);
    a.line(head.0 + 2, head.1 - 3, head.0 + 3, head.1 - 7, WOOD + 5);
    a.line(muzzle.0 - 2, muzzle.1 + 1, muzzle.0 + 2, muzzle.1 + 1, WOOD + 2);
    a.dot(muzzle.0 + fx, muzzle.1, INK + 2);
    a.dot(head.0 - fx, head.1 - 1, GOLD + 5);
    a.line(muzzle.0, muzzle.1 + 1, x, 30, WOOD + 1);
    a.poly(&[(x-7,28), (x+7,27), (x+8,37), (x-3,40), (x-8,36)], SLATE + 2);
    a.line(x - 7, 29, x + 6, 28, GOLD + 5);
    a.line(x - 4, 37, x + 6, 35, GOLD + 2);
    a
}

fn soldier(troop: Troop, team: u8, frame: usize) -> Art {
    let mounted = troop == Troop::Knights;
    let stride = usize::from(figures::poses_per_facing(troop));
    let (facing, pose, collapse, dig) = if mounted {
        const SWING_FACING: [usize; 16] = [7, 0, 2, 3, 1, 2, 4, 5, 3, 4, 6, 5, 6, 6, 0, 7];
        if frame < 8 { (frame, 0, 0, false) }
        else { (SWING_FACING[(frame - 8) / 3], 6 + (frame - 8) % 3, 0, false) }
    } else if frame < stride * 8 {
        (frame / stride, frame % stride, 0, false)
    } else if frame < stride * 8 + 6 {
        (3, 0, frame - stride * 8 + 1, false)
    } else {
        let step = frame - stride * 8 - 6;
        ((step / 3) * 2, step % 3, 0, true)
    };
    let (fx, fy) = FACING_DELTA[facing];
    let mut a = Art::new(52, 53);
    let x = 26;
    let base = if mounted { 30 } else { 43 };
    if !mounted {
        a.ellipse(x + 2, base, if collapse > 2 { 13 } else { 10 }, 3, INK + 4);
    }
    if collapse >= 4 {
        let dx = collapse as i32 - 4;
        a.poly(&[(13,40),(18,35-dx),(29,37),(32,40),(25,43),(16,43)], INK + 1);
        a.poly(&[(15,40),(19,36-dx),(29,38),(29,41),(18,42)], team + 4);
        a.line(18, 38 - dx, 26, 39, team + 6);
        a.ellipse(30, 40 - dx, 4, 4, IRON + 2);
        a.ellipse(31, 39 - dx, 3, 3, IRON + 5);
        a.line(17, 41, 10, 42, WOOD + 2);
        a.line(19, 42, 13, 45, INK + 2);
        a.line(32, 43, 41, 44, IRON + 4);
        return a;
    }
    let bob = if !mounted && pose < 6 { [0, -1, -1, 0, -1, -1][pose] } else { 0 };
    let bend = if dig { 2 + pose as i32 } else { collapse as i32 * 3 };
    let waist = base - if mounted { 5 } else { 12 } + bob + bend;
    let shoulder = waist - if mounted { 11 } else { 9 };
    let head = (x + fx * (2 + bend), shoulder - 4);
    let trail = if pose < 6 { [0, -2, -4, 0, 3, 2][pose] } else { 0 };
    let cloth_top = if mounted { team + 5 } else { team + 4 };
    let mail = matches!(troop, Troop::Swordsmen | Troop::Knights | Troop::Pikemen);
    // The back shoulder, legs and redrawn outline give the walk frames real
    // silhouette changes even in the north/south facings.
    for (leg, swing) in [(-1, -trail), (1, trail)] {
        let dx = leg * if mounted { 5 } else { 3 };
        let knee = (x + dx + fx * swing, waist + if mounted { 4 } else { 5 });
        let foot = (knee.0 + fx * swing / 2, base - 1 + fy * swing / 2);
        stroke(&mut a, (x + dx, waist), knee, if mail { IRON + 3 } else { WOOD + 4 });
        stroke(&mut a, knee, foot, WOOD + 2);
        a.line(foot.0 - 1, foot.1 + 1, foot.0 + fx + 1, foot.1 + 1, INK + 1);
    }
    let back_x = x - fx * 3;
    a.poly(&[(x-6,shoulder), (head.0+4,shoulder-2), (x+6,shoulder+4), (x+5,waist+3), (x+1,waist+6), (x-5,waist+3)], INK + 1);
    a.poly(&[(x-4,shoulder+1), (head.0+3,shoulder-1), (x+4,shoulder+4), (x+4,waist+2), (x+1,waist+4), (x-4,waist+2)], cloth_top);
    a.line(back_x - 2, shoulder + 1, back_x - 1, waist + 2, team + 2);
    a.line(x, shoulder + 1, x + 2, waist - 2, team + 7);
    a.line(x - 4, waist, x + 4, waist, WOOD + 1);
    a.dot(x + 1, waist, GOLD + 6);
    if mail {
        for (dx, dy) in [(-4,2),(-2,3),(3,2),(4,3),(-4,4),(3,5)] {
            a.dot(x + dx, shoulder + dy, IRON + if dy % 2 == 0 { 6 } else { 3 });
        }
    }
    // The colour bank changes only textiles and heraldry. Leather, wood,
    // exposed faces and iron retain their material palette.
    a.ellipse(head.0, head.1, 4, 5, INK + 1);
    if fy >= 0 {
        a.ellipse(head.0, head.1 + 1, 3, 3, COPPER + 5);
        a.dot(head.0 + fx, head.1 + 1, WOOD + 1);
        a.dot(head.0 + fx, head.1 + 2, CREAM + 5);
    }
    match troop {
        Troop::Peasants => {
            a.ellipse(head.0, head.1 - 2, 4, 3, PARCHMENT + 4);
            a.line(head.0 - 5, head.1, head.0 + 5, head.1, WOOD + 3);
            a.line(head.0 - 3, head.1 - 2, head.0 + 2, head.1 - 3, PARCHMENT + 6);
        }
        Troop::Archers => {
            a.poly(&[(head.0-5,head.1+2),(head.0-3,head.1-4),(head.0,head.1-6),(head.0+4,head.1-3),(head.0+4,head.1+1)], team + 2);
            a.line(head.0-3, head.1-3, head.0, head.1-5, team + 6);
        }
        _ => {
            a.ellipse(head.0, head.1 - 2, 4, 3, IRON + 4);
            a.line(head.0 - 3, head.1 - 3, head.0 + 2, head.1 - 4, IRON + 7);
            a.line(head.0 - 4, head.1, head.0 + 4, head.1, IRON + 1);
            if mounted {
                a.line(head.0, head.1 - 4, head.0 - fx * 3 - 1, head.1 - 8, team + 6);
                a.dot(head.0 - fx * 3 - 2, head.1 - 7, team + 3);
            }
        }
    }
    let weapon_x = x + if fx == 0 { 6 } else { fx * 6 };
    let combat = !dig && pose >= 6;
    let lift = if combat { [7, 4, 0, -2, 2, 0, 3][pose - 6] } else { 0 };
    let hand = (weapon_x + if combat { fx * 2 } else { 0 }, shoulder + 7 - lift + if dig { 2 } else { 0 });
    stroke(&mut a, (weapon_x - fx * 2, shoulder + 2), hand, team + 5);
    a.dot(hand.0, hand.1, COPPER + 6);
    if mail || troop == Troop::Macemen {
        let sx = x - if fx == 0 { 6 } else { fx * 5 };
        leonine_shield(&mut a, sx, shoulder + 8, team, !mail);
    }
    let end = (hand.0 + fx * 9, hand.1 + fy * 7 - 9);
    if dig {
        let tip = (hand.0 + fx * 10 + if fx == 0 { 4 } else { 0 }, hand.1 + 8 - pose as i32 * 3);
        stroke(&mut a, hand, tip, WOOD + 5);
        a.ellipse(tip.0, tip.1, 3, 2, IRON + 4);
        a.dot(tip.0 - 1, tip.1 - 1, IRON + 7);
        if pose == 2 {
            a.dot(tip.0 + fx * 3, tip.1 - 4, PARCHMENT + 3);
        }
    } else {
        match troop {
            Troop::Archers => {
                let bow = (hand.0 + fx * 4 + if fx == 0 { 2 } else { 0 }, hand.1 - 3);
                let side = if fx < 0 { -1 } else { 1 };
                a.line(bow.0 - side, bow.1 - 7, bow.0 + side * 3, bow.1 - 3, WOOD + 4);
                a.line(bow.0 + side * 3, bow.1 - 3, bow.0 + side * 3, bow.1 + 3, GOLD + 4);
                a.line(bow.0 + side * 3, bow.1 + 3, bow.0 - side, bow.1 + 7, WOOD + 4);
                a.line(bow.0 - side, bow.1 - 7, bow.0 - side, bow.1 + 7, CREAM + 3);
                if pose >= 10 { a.line(bow.0 - side * 5, bow.1, bow.0 + side * 7, bow.1 + fy, IRON + 6); }
            }
            Troop::Crossbowmen => {
                let stock = (hand.0 + fx * 10, hand.1 + fy * 5 - 4);
                let l = (stock.0 - fy * 6, stock.1 - fx * 4 - 2);
                let r = (stock.0 + fy * 6, stock.1 + fx * 4 + 2);
                stroke(&mut a, hand, stock, WOOD + 5);
                a.line(l.0, l.1, stock.0, stock.1, IRON + 4);
                a.line(stock.0, stock.1, r.0, r.1, IRON + 6);
                a.line(l.0, l.1, hand.0, hand.1, PARCHMENT + 3);
                a.line(hand.0, hand.1, r.0, r.1, PARCHMENT + 3);
            }
            Troop::Pikemen => {
                let tip = (hand.0 + fx * 13, hand.1 + fy * 9 - if combat { 5 } else { 13 });
                let butt = (hand.0 - fx * 7, hand.1 - fy * 5 + 8);
                stroke(&mut a, butt, tip, WOOD + 5);
                a.line(tip.0 - fx * 3, tip.1 + 3, tip.0 + fx * 3, tip.1 - 3, IRON + 7);
                a.poly(&[(tip.0-fx*4,tip.1+4),(tip.0-fx*6-4,tip.1+5),(tip.0-fx*5,tip.1+8)], team + 6);
            }
            Troop::Macemen => {
                stroke(&mut a, hand, end, WOOD + 5);
                a.ellipse(end.0, end.1, 4, 4, IRON + 1);
                a.ellipse(end.0, end.1, 3, 3, IRON + 4);
                for (dx, dy) in [(-3,0), (0,-4), (3,1), (0,3)] {
                    a.dot(end.0+dx, end.1+dy, IRON + 7);
                }
            }
            Troop::Swordsmen | Troop::Knights => {
                stroke(&mut a, hand, end, IRON + 5);
                a.line(hand.0 + fx, hand.1 - 2, end.0, end.1 - 1, IRON + 7);
                a.line(hand.0 - 3, hand.1 - 1, hand.0 + 3, hand.1 - 1, GOLD + 5);
                a.dot(hand.0 - fx, hand.1 + 2, GOLD + 4);
            }
            _ => {
                stroke(&mut a, hand, end, WOOD + 5);
                a.line(end.0 - 3, end.1, end.0 + 3, end.1, IRON + 5);
                a.line(end.0 - 3, end.1, end.0 - 2, end.1 + 3, IRON + 6);
            }
        }
    }
    a
}

fn missile(i: usize) -> Art {
    match i {
        0..=15 => {
            let (fx, fy) = FACING_DELTA[i % 8];
            let mut a = Art::new(15, 15);
            let (x, y) = (6, 6);
            let head = (x + fx * 5, y + fy * 5);
            let tail = (x - fx * 4, y - fy * 4);
            stroke(&mut a, tail, head, if i < 8 { WOOD + 6 } else { IRON + 5 });
            a.line(head.0 - fx - fy, head.1 - fy + fx, head.0, head.1, CREAM + 6);
            a.line(head.0 - fx + fy, head.1 - fy - fx, head.0, head.1, IRON + 7);
            a.dot(tail.0 + fy, tail.1 - fx, if i < 8 { CREAM + 4 } else { COPPER + 6 });
            a
        }
        16..=24 => {
            let mut a = Art::new(20, 16);
            let r = if i == 16 { 5 } else { 5 - ((i - 17) / 2) as i32 };
            a.ellipse(8, 10, r, r / 2, INK + 4);
            a.poly(&[(3,8),(4,5),(8,6-r),(8+r,4),(10+r,9),(8,10),(4,10)], STONE + 1);
            a.poly(&[(4,7),(8,7-r),(7+r,5),(7+r,8),(7,9)], STONE + 5);
            a.dot(7, 7-r, STONE + 7);
            if i > 16 { a.dot(12 + i as i32 % 5, 7 + i as i32 % 6, PARCHMENT + 4); }
            a
        }
        25..=32 => {
            let mut a = Art::new(26, 24);
            let phase = i as i32 - 25;
            a.ellipse(9, 14, 8, 5, INK + 3);
            for k in 0..4 {
                let x = 3 + k * 4;
                let top = 1 + (k * 7 + phase * 3) % 8;
                let lean = (phase + k) % 4 - 1;
                a.poly(&[(x-2,16), (x-3,11), (x+lean,top), (x+2,10), (x+4,14), (x+2,18)], RED + 3);
                a.poly(&[(x,17), (x-1,12), (x+lean,top+3), (x+2,13), (x+2,17)], GOLD + 6);
                a.dot(x + lean, top.saturating_sub(2), COPPER + 6);
            }
            a
        }
        _ => {
            let mut a = Art::new(40, 39);
            let team = LIVERY[(i - 33) / 8];
            let phase = ((i - 33) % 8) as i32;
            let tail = [1, 0, -1, -1, 0, 1, 2, 2][phase as usize];
            let tip = 34 - (phase % 4);
            // The first flag's white and amber fringe is also indexed by
            // the late fire band. Both callers use the same PL8 records.
            a.line(19, 34, 19, 2, WOOD + 1);
            a.line(20, 32, 20, 2, GOLD + 4);
            a.ellipse(19, 2, 2, 2, GOLD + 6);
            a.poly(&[(20,5),(26,4),(tip,6+tail),(tip-3,11+tail),(tip,17+tail),(26,16),(20,18)], INK + 1);
            a.poly(&[(21,6),(26,5),(tip-1,7+tail),(tip-4,11+tail),(tip-1,16+tail),(26,15),(21,17)], team + 4);
            a.line(21, 7, tip - 2, 8 + tail, team + 7);
            a.line(21, 16, tip - 2, 15 + tail, GOLD + 5);
            a.dot(26, 9, GOLD + 6);
            a.line(24, 12, 27, 10, CREAM + 6);
            a.dot(27, 13, GOLD + 3);
            a
        }
    }
}

fn wheel(a: &mut Art, x: i32, y: i32, r: i32, thin: bool) {
    let rx = if thin { r * 2 / 3 } else { r };
    a.ellipse(x, y, rx + 1, r + 1, INK + 1);
    a.ellipse(x, y, rx, r, WOOD + 4);
    a.ellipse(x, y, rx - 3, r - 3, WOOD + 1);
    for (dx, dy) in [(rx - 2, 0), (0, r - 2), (rx / 2 + 1, r / 2), (rx / 2 + 1, -r / 2)] {
        a.line(x - dx, y - dy, x + dx, y + dy, WOOD + 6);
    }
    a.ellipse(x, y, 2, 2, IRON + 3);
    a.dot(x - 1, y - 1, IRON + 7);
}

fn catapult_frame(facing: usize) -> Art {
    let (fx, fy) = FACING_DELTA[facing];
    let mut a = Art::new(128, 120);
    let (x, y) = (64, 68);
    let p = |along: i32, cross: i32| {
        (x + fx * along - fy * cross, y + fy * along / 2 + fx * cross / 2)
    };
    a.ellipse(x + 3, y + 6, 32, 19, INK + 4);
    for (along, cross) in [(-23, -17), (21, -17)] {
        let (wx, wy) = p(along, cross);
        wheel(&mut a, wx, wy + 6, 10, fy != 0);
    }
    let corners = [p(-27, -15), p(26, -15), p(26, 15), p(-27, 15)];
    a.poly(&corners, WOOD + 1);
    for w in (-12..=12).step_by(4) {
        let (from, to) = (p(-26, w), p(25, w));
        stroke(&mut a, from, to, WOOD + 5);
    }
    for (l, w) in [(-23, 13), (21, 13), (-16, -11), (20, -11)] {
        let at = p(l, w);
        a.dot(at.0, at.1, IRON + 7);
    }
    for side in [-1, 1] {
        let bottom = p(-16, side * 11);
        let top = p(-5, side * 11);
        let end = p(17, side * 11);
        stroke(&mut a, bottom, (top.0, top.1 - 25), WOOD + 4);
        stroke(&mut a, end, (top.0, top.1 - 25), WOOD + 6);
        a.ellipse(top.0, top.1 - 24, 4, 4, IRON + 2);
        a.dot(top.0 - 1, top.1 - 25, GOLD + 6);
    }
    for (along, cross) in [(-23, 17), (21, 17)] {
        let (wx, wy) = p(along, cross);
        wheel(&mut a, wx, wy + 6, 11, fy != 0);
    }
    a
}

fn catapult_arm(facing: usize, pose: usize) -> Art {
    let (fx, fy) = FACING_DELTA[facing];
    let mut a = Art::new(128, 120);
    let pivot = (64 - fx * 5, 43 - fy * 5 / 2);
    let reach = [27, 20, 8, -4, -13][pose];
    let up = [1, 13, 24, 29, 33][pose];
    let tip = (pivot.0 - fx * reach, pivot.1 - fy * reach / 2 - up);
    let floor = (pivot.0 + fx * reach / 3, pivot.1 + fy * reach / 6 + up / 3);
    stroke(&mut a, floor, tip, WOOD + 6);
    a.line(floor.0, floor.1 + 2, tip.0, tip.1 + 2, WOOD + 4);
    a.ellipse(pivot.0, pivot.1, 3, 3, GOLD + 4);
    a.ellipse(tip.0, tip.1, 5, 3, WOOD + 2);
    a.line(tip.0 - 4, tip.1 - 1, tip.0 + 4, tip.1 - 1, IRON + 5);
    if pose == 0 {
        a.ellipse(tip.0, tip.1 - 4, 4, 3, STONE + 5);
        a.dot(tip.0 - 1, tip.1 - 5, STONE + 7);
    }
    a
}

fn siege_tower(facing: usize, docked: bool) -> Art {
    let mut a = Art::new(96, 96);
    let (fx, fy) = FACING_DELTA[facing];
    let y = if docked { 30 } else { 36 };
    if !docked {
        a.ellipse(50, 75, 32, 13, INK + 4);
        for x in [25, 37, 60, 72] {
            wheel(&mut a, x, 70, 10, false);
        }
    }
    a.poly(&[(19,y-11),(32,y-19),(72,y-18),(78,y-10),(75,y+31),(68,y+35),(21,y+29)], INK + 1);
    plank(&mut a, 23, y - 10, 50, 40, facing as u32);
    a.poly(&[(22,y-10),(33,y-19),(71,y-18),(76,y-10),(67,y-6),(30,y-6)], WOOD + 6);
    a.poly(&[(33,y-17),(68,y-16),(71,y-11),(64,y-8),(31,y-8)], WOOD + 1);
    for x in [23, 37, 59, 71] {
        a.rect(x, y - 11, 3, 40, WOOD + 2);
        a.line(x, y - 11, x, y + 28, WOOD + 6);
    }
    for (x0, x1) in [(25, 45), (48, 70)] {
        stroke(&mut a, (x0, y - 6), (x1, y + 23), WOOD + 5);
        stroke(&mut a, (x1, y - 6), (x0, y + 23), WOOD + 5);
    }
    a.rect(20, y - 10, 55, 5, WOOD + 3);
    a.line(20, y - 10, 74, y - 10, GOLD + 5);
    for x in [22, 32, 44, 55, 69] {
        a.rect(x, y - 15, 6, 7, WOOD + 2);
        a.line(x, y - 15, x + 5, y - 15, WOOD + 6);
        a.dot(x + 1, y - 11, GOLD + 4);
    }
    let gate = 48 + fx * 20;
    a.rect(gate - 6, y - 13, 12, 12, WOOD + 1);
    if docked {
        let end = (gate + fx * 17, y - 4 + fy * 20);
        for n in -5i32..=5 {
            let c = if n.abs() == 5 { WOOD + 1 } else { WOOD + 5 };
            a.line(gate + n, y - 9, end.0 + n, end.1, c);
        }
        a.line(end.0 - 4, end.1, end.0 + 4, end.1, GOLD + 5);
    } else {
        for k in 0..4 {
            a.line(gate - 4, y - 11 + k * 3, gate + 4, y - 11 + k * 3, WOOD + 5);
        }
    }
    a
}

fn oil_pot(f: usize) -> Art {
    let mut a = Art::new(32, 32);
    a.ellipse(16, 24, 12, 4, INK + 4);
    for x in [7, 23] {
        a.line(x, 15, x, 24, IRON + 2);
        a.dot(x, 23, IRON + 5);
    }
    a.ellipse(15, 17, 9, 8, INK + 1);
    a.ellipse(15, 17, 8, 7, IRON + 3);
    a.ellipse(12, 17, 4, 6, IRON + 5);
    a.ellipse(15, 12, 9, 4, IRON + 6);
    a.ellipse(15, 11, 7, 2, GOLD + 2);
    a.ellipse(15, 11, 6, 1, GOLD + 5);
    if f < 42 {
        for k in 0..3 {
            let x = 9 + k * 6 + ((f + k as usize) % 3) as i32;
            a.line(x, 8 - k, x + 2, 3 - k, STONE + 5 + ((f + k as usize) % 3) as u8);
        }
    } else {
        let (fx, fy) = FACING_DELTA[(f - 42) * 2];
        let (x, y) = (15 + fx * 8, 13 + fy * 5);
        let tip = (x + fx * 7, y + fy * 6 + 4);
        a.poly(&[(x-2,y),(x+2,y-1),(tip.0+2,tip.1),(tip.0-1,tip.1+3)], GOLD + 3);
        a.line(x, y, tip.0, tip.1, GOLD + 7);
        a.dot(tip.0 - fy, tip.1 + 3, COPPER + 6);
    }
    a
}

fn engine(i: usize) -> Art {
    match i {
        0 => {
            let mut a = Art::new(96, 96);
            a.ellipse(49, 62, 28, 18, INK + 4);
            for (x, y) in [(28, 45), (66, 45), (29, 67), (65, 67)] {
                wheel(&mut a, x, y, 9, true);
            }
            a.poly(&[(26,29),(45,17),(67,28),(68,66),(47,75),(24,64)], INK + 1);
            a.poly(&[(26,30),(45,18),(67,29),(46,42)], RED + 3);
            a.poly(&[(26,30),(46,42),(47,73),(26,63)], WOOD + 3);
            a.poly(&[(46,42),(67,30),(66,65),(47,73)], WOOD + 5);
            for y in [42, 50, 58] { a.line(27, y, 46, y + 9, WOOD + 1); }
            for y in [44, 53, 62] { a.line(48, y, 65, y - 8, WOOD + 3); }
            a.line(26, 30, 45, 18, GOLD + 6);
            a.line(45, 18, 65, 28, COPPER + 5);
            a.rect(44, 38, 5, 30, IRON + 3);
            a.line(46, 39, 46, 65, IRON + 6);
            a
        }
        1..=4 => siege_tower((i - 1) * 2, false),
        5..=12 => catapult_frame(i - 5),
        13..=30 => {
            let mut a = Art::new(96, 76);
            let upper = i >= 22;
            let pose = if upper { i - 22 } else { i - 13 } as i32;
            let y = if upper { 45 + pose * 2 } else { 2 + pose };
            plank(&mut a, 43, y, 10, 17, 2);
            a.rect(42, y + if upper { 0 } else { 14 }, 12, 4, IRON + 4);
            a.dot(43, y + if upper { 1 } else { 15 }, GOLD + 6);
            a
        }
        31..=34 => siege_tower((i - 31) * 2, true),
        _ => oil_pot(i),
    }
}

fn village_grid() -> Art {
    let mut a = Art::new(village::GRID_COLS as i32, village::GRID_ROWS as i32);
    for y in 0..a.h {
        for x in 0..a.w {
            let (px, py) = (x * village::GRID_CELL + 4, y * village::GRID_CELL + 4);
            let mut pick = (i32::MAX, 0u8);
            for (id, &(ox, oy)) in village::ORIGINS.iter().enumerate() {
                if (ox - 5..ox + 88).contains(&px) && (oy - 23..oy + 59).contains(&py) {
                    let dx = px - (ox + 43);
                    let dy = py - (oy + 17);
                    let score = dx * dx + dy * dy;
                    if score < pick.0 {
                        pick = (score, id as u8 + 1);
                    }
                }
            }
            a.dot(x, y, pick.1);
        }
    }
    a
}

fn cottage(a: &mut Art, x: i32, y: i32, w: i32, h: i32, roof: u8) {
    let (left, right) = (x - w / 2, x + w / 2);
    let side = x + w / 6;
    let eave = y - h;
    let peak = eave - w / 3;
    a.ellipse(x + 3, y, w / 2 + 5, h / 3, PINE + 3);
    a.poly(&[(left, eave), (side, eave), (side, y), (left, y - 2)], CREAM + 5);
    a.poly(&[(side, eave), (right, eave - 6), (right, y - 6), (side, y)], STONE + 5);
    a.poly(&[(left - 2, eave), (x - w / 6, peak), (side + 1, eave)], roof + 5);
    a.poly(&[(x - w / 6, peak), (right - w / 4, peak - 6), (right + 3, eave - 6), (side + 1, eave)], roof + 3);
    a.line(x - w / 6, peak, right - w / 4, peak - 6, roof + 6);
    a.line(left - 2, eave, side + 1, eave, roof + 1);
    a.rect(x - 4, y - h / 2, 4, h / 2, WOOD + 2);
    a.rect(side + 3, eave + 2, 3, 3, INK + 3);
    a.dot(side + 4, eave + 3, GOLD + 6);
}

fn atlas_plots(a: &mut Art, x: i32, y: i32, w: i32, h: i32, tint: u8) {
    a.poly(&[(x, y), (x + w - 15, y - 8), (x + w, y + h - 9), (x + 14, y + h)], tint + 2);
    for row in (3..h - 2).step_by(5) {
        let start = x + row * 14 / h + 2;
        a.line(start, y + row - 1, start + w - 20, y + row - 9, tint + 4);
        a.line(start, y + row, start + w - 18, y + row - 8, tint + 6);
    }
}

fn fence(a: &mut Art, start: Point, end: Point) {
    let (x, y) = start;
    let (u, v) = end;
    a.line(x, y - 2, u, v - 2, WOOD + 6);
    let length = (u - x).abs().max((v - y).abs()).max(1);
    for i in (0..=length).step_by(9) {
        let (px, py) = (x + (u - x) * i / length, y + (v - y) * i / length);
        a.line(px, py - 4, px, py + 1, WOOD + 3);
    }
}

fn atlas_grove(a: &mut Art, trees: &[(i32, i32, i32)]) {
    for &(x, y, r) in trees {
        a.ellipse(x + 2, y, r, r / 3, PINE + 2);
        a.rect(x, y - r, 1, r + 1, WOOD + 3);
        a.ellipse(x, y - r + 1, r / 2 + 1, r, PINE + 4);
        a.ellipse(x - r / 5, y - r - 1, r / 3 + 1, r * 2 / 3, PINE + 6);
    }
}

fn atlas_road(a: &mut Art, points: &[Point]) {
    for (dy, color) in [(3, MOSS + 4), (-2, PARCHMENT + 4), (-1, PARCHMENT + 5), (0, PARCHMENT + 5), (1, PARCHMENT + 4), (2, PARCHMENT + 4)] {
        for pair in points.windows(2) {
            let ((ax, ay), (bx, by)) = (pair[0], pair[1]);
            a.line(ax, ay + dy, bx, by + dy, color);
        }
    }
}

fn atlas_sky(a: &mut Art, sky: u8, far: u8, near: u8, light: bool) {
    a.rect(0, 0, a.w, 47, sky);
    a.rect(0, 20, a.w, 28, sky + 1);
    if light {
        a.ellipse(281, 18, 18, 18, sky + 2);
        a.ellipse(281, 18, 12, 12, CREAM + 7);
    }
    for (x, y, w) in [(27, 17, 49), (146, 8, 37), (310, 19, 32)] {
        a.line(x, y, x + w, y, sky + 2);
        a.line(x + 7, y + 3, x + w / 2, y + 3, sky + 2);
    }
    a.poly(&[(0,31),(32,20),(65,25),(106,37),(148,23),(184,17),(227,28),(260,38),(309,29),(342,19),(362,22),(362,48),(0,48)], far);
    a.poly(&[(0,35),(31,36),(80,30),(122,28),(161,38),(188,41),(229,34),(264,31),(290,35),(323,33),(362,28),(362,48),(0,48)], near);
}

fn village_scene() -> Art {
    let mut a = Art::new(village::SCENE_W, village::SCENE_H);
    a.rect(0, 40, a.w, a.h - 40, GRASS + 4);
    atlas_sky(&mut a, PARCHMENT + 5, SLATE + 5, PINE + 5, true);
    a.poly(&[(0,40),(84,35),(151,45),(183,56),(242,49),(304,37),(362,38),(362,75),(271,96),(187,83),(124,69),(55,70),(0,88)], GRASS + 5);
    a.poly(&[(0,111),(60,95),(126,97),(196,112),(257,120),(362,103),(362,181),(290,195),(189,180),(88,166),(0,177)], MOSS + 5);
    a.poly(&[(0,168),(101,156),(182,174),(286,197),(362,182),(362,251),(248,267),(158,232),(76,239),(0,251)], PINE + 5);
    a.poly(&[(0,241),(81,220),(157,226),(245,263),(362,240),(362,319),(0,319)], GRASS + 5);
    for (x, y, w, h, tint) in [(7,84,71,35,GOLD),(17,125,81,38,PARCHMENT),(111,108,67,37,PARCHMENT),(290,58,62,29,MOSS),(267,135,65,42,GOLD),(34,220,86,41,MOSS),(164,256,88,54,GOLD)] {
        atlas_plots(&mut a, x, y, w, h, tint);
    }

    a.poly(&[(248,36),(235,47),(225,57),(227,66),(243,77),(260,88),(268,101),(266,113),(246,136),(237,160),(245,182),(263,197),(281,224),(271,255),(243,276),(216,319),(253,319),(272,293),(301,265),(314,226),(296,191),(271,172),(262,153),(274,127),(295,108),(292,88),(275,77),(254,70),(240,61),(240,56),(248,46),(254,36)], PINE + 3);
    a.poly(&[(250,37),(238,52),(231,61),(234,68),(270,87),(279,101),(275,115),(253,141),(245,161),(253,180),(277,202),(289,226),(281,257),(252,280),(227,319),(243,319),(266,288),(292,263),(305,225),(286,194),(263,173),(254,154),(267,126),(287,107),(284,91),(271,81),(246,70),(236,62),(243,50),(253,37)], WATER + 5);
    for (x, y, w) in [(236,59,7), (250,76,8), (274,99,8), (254,147,8), (258,181,11), (287,222,9), (273,273,11)] {
        a.line(x, y, x + w, y, WATER + 7);
    }

    atlas_road(&mut a, &[(0,113),(52,101),(106,82),(141,81),(184,89),(213,88),(247,91),(275,86),(319,74),(362,75)]);
    atlas_road(&mut a, &[(63,98),(94,116),(111,143),(99,180),(58,212),(62,261),(124,319)]);
    atlas_road(&mut a, &[(211,88),(190,106),(174,138),(183,166),(206,198),(197,245),(160,319)]);
    atlas_road(&mut a, &[(362,153),(312,147),(291,163),(262,185),(224,195),(206,198)]);
    for (x, y, w) in [(253,84,31), (247,175,25)] {
        a.poly(&[(x, y + 3), (x + w, y - 3), (x + w, y + 4), (x, y + 9)], WOOD + 4);
        a.line(x, y + 3, x + w, y - 3, PARCHMENT + 5);
        a.line(x, y + 8, x + w, y + 3, PARCHMENT + 5);
    }

    atlas_grove(&mut a, &[(8,64,8),(19,66,10),(29,71,7),(39,68,11),(51,69,8),(309,48,7),(320,45,9),(333,47,6),(343,51,10),(348,58,8),(353,108,11),(343,115,9),(355,126,7),(121,155,9),(134,159,13),(145,166,9),(13,204,12),(26,205,9),(37,212,14),(211,222,13),(224,226,11),(233,234,9),(331,271,12),(340,287,13),(353,290,8)]);
    for (x, y, w, h, roof) in [(123,71,21,10,RED),(150,78,35,16,COPPER),(177,82,22,11,SLATE),(200,81,25,11,RED),(141,97,22,11,SLATE),(171,106,34,14,RED),(202,107,26,12,COPPER),(307,104,30,12,SLATE),(85,128,22,12,RED),(142,191,38,21,RED),(171,190,24,13,COPPER),(218,212,26,12,SLATE),(75,268,29,12,COPPER),(159,281,32,18,RED),(327,244,29,13,SLATE)] {
        cottage(&mut a, x, y, w, h, roof);
    }
    // A pale tower over the rooftops anchors the atlas at a glance.
    a.rect(159, 46, 10, 24, CREAM + 5);
    a.rect(166, 45, 4, 22, STONE + 5);
    a.poly(&[(157,46),(165,34),(172,45)], SLATE + 3);
    a.line(157, 46, 165, 34, SLATE + 6);
    a.rect(162, 49, 2, 5, SLATE + 2);
    a.dot(164, 33, GOLD + 7);
    a
}

fn village_weather(f: usize) -> Art {
    let mut a = Art::new(village::SCENE_W, 70);
    let (sky, far, near) = [
        (ICE + 4, SLATE + 6, ICE + 4),
        (PARCHMENT + 5, COPPER + 4, PARCHMENT + 3),
        (ICE + 4, WATER + 4, PINE + 5),
        (SLATE + 5, SLATE + 4, PINE + 5),
        (SLATE + 2, VIOLET + 3, SLATE + 3),
        (WATER + 3, SLATE + 5, WATER + 5),
    ][f];
    a.rect(0, 45, a.w, 25, near);
    atlas_sky(&mut a, sky, far, near, f == 1 || f == 2);
    a.poly(&[(0,70),(0,51),(58,44),(120,52),(173,64),(228,65),(280,57),(335,45),(362,50),(362,70)], near + 1);
    a.poly(&[(0,70),(48,64),(108,62),(158,70),(237,70),(306,63),(362,60),(362,70)], GRASS + 4);
    if f == 4 {
        a.line(291, 39, 284, 52, GOLD + 7);
        a.line(284, 52, 290, 50, GOLD + 7);
        a.line(290, 50, 284, 60, CREAM + 7);
    }
    a
}

fn mine_animation(f: usize) -> Art {
    let mut a = Art::new(32, 23);
    let phase = (f % 9) as i32;
    let x = if f < 9 { phase } else { 8 - phase };
    a.line(0, 18, 27, 18, WOOD + 2);
    a.line(0, 20, 28, 20, IRON + 4);
    for k in (0..29).step_by(5) { a.line(k, 18, k, 21, WOOD + 4); }
    a.poly(&[(x,10),(x+14,10),(x+11,17),(x+3,17)], WOOD + 2);
    a.line(x + 1, 11, x + 13, 11, WOOD + 6);
    a.ellipse(x + 4, 17, 2, 2, IRON + 2);
    a.ellipse(x + 10, 17, 2, 2, IRON + 4);
    a.dot(x + 5, 9, IRON + 6);
    a.dot(x + 8, 8, STONE + 5);
    a
}

fn village_animation(i: usize) -> Art {
    match i {
        0..=6 => {
            let mut a = Art::new(28, 33);
            let phase = (i as i32 - 3).abs();
            let tip = (8 + phase * 3, 6 + i as i32);
            stroke(&mut a, (4, 30), (7, 9), WOOD + 4);
            stroke(&mut a, (3, 13), tip, WOOD + 6);
            a.line(tip.0, tip.1, tip.0, 22 + phase, IRON + 3);
            a.ellipse(tip.0, 24 + phase, 3, 2, STONE + 5);
            a.dot(tip.0 - 1, 23 + phase, STONE + 7);
            a
        }
        7..=14 => {
            let mut a = Art::new(33, 26);
            let x = 7 + ((i - 7) as i32 - 3).abs() * 3;
            a.ellipse(14, 20, 13, 4, MOSS + 1);
            plank(&mut a, 2, 14, 27, 7, 12);
            a.line(x, 11, x + 9, 11, IRON + 6);
            for k in 0..4 { a.dot(x + k * 2, 12, IRON + 7); }
            let glow = x + ((i % 3) as i32) * 4;
            a.dot(glow, 13, PARCHMENT + 7);
            a
        }
        15..=24 => {
            let mut a = Art::new(27, 23);
            let x = 9 + ((i - 15) as i32 - 4).abs() / 2;
            wheel(&mut a, x, 10, 7, true);
            a.line(2, 18, 22, 18, WATER + 5);
            a.line(6, 20, 24 - (i % 4) as i32, 20, WATER + 6);
            a.dot(x + 2, 4 + (i % 5) as i32, GOLD + 5);
            a
        }
        25..=32 => {
            let mut a = Art::new(29, 23);
            let w = (i - 25) as i32;
            for k in 0..4 {
                let x = 12 + k * 2 + (w + k) % 4;
                let y = 19 - k * 4 - w % 3;
                a.ellipse(x, y, 2 + k / 2, 1 + k / 2, STONE + 4 + (k % 2) as u8);
                a.dot(x - 1, y - 1, STONE + 6);
            }
            a
        }
        33..=39 => {
            let mut a = Art::new(24, 25);
            let turn = (i - 33) % 4;
            for k in 0..4 {
                let (x, y) = FACING_DELTA[(turn + k * 2) % 8];
                let tip = (12 + x * 10, 12 + y * 10);
                a.poly(&[(12,12), tip, (tip.0-y*3,tip.1+x*3), (12-y*2,12+x*2)], PARCHMENT + 5);
                a.line(12, 12, tip.0, tip.1, WOOD + 3);
                a.line(tip.0, tip.1, tip.0-y*3, tip.1+x*3, WOOD + 5);
            }
            a.ellipse(12, 12, 2, 2, WOOD + 2);
            a.dot(12, 11, GOLD + 6);
            a
        }
        40 => {
            let mut a = Art::new(111, 46);
            a.ellipse(54, 31, 47, 13, MOSS + 4);
            for (x, y, r) in [(17, 28, 8), (39, 24, 12), (61, 27, 10), (82, 24, 12)] {
                a.poly(&[(x - r, y + 4), (x - r / 2, y - r), (x + r / 2, y - r - 2), (x + r, y + 4)], STONE + 4);
                a.poly(&[(x - r, y + 4), (x - r / 2, y - r), (x + r / 2, y - r - 2), (x + 1, y)], STONE + 6);
            }
            fence(&mut a, (11, 35), (90, 35));
            a
        }
        41 => {
            let mut a = Art::new(122, 73);
            atlas_grove(&mut a, &[(13, 30, 9), (31, 28, 10), (70, 24, 11), (113, 52, 9), (97, 53, 8)]);
            cottage(&mut a, 51, 51, 40, 22, WOOD);
            for k in 0..5 {
                let y = 47 + k * 3;
                a.line(66, y, 88, y, WOOD + 2);
                a.line(66, y - 1, 88, y - 1, WOOD + 5);
                a.ellipse(88, y - 1, 2, 2, PARCHMENT + 5);
            }
            a
        }
        43 => {
            let mut a = Art::new(111, 52);
            a.poly(&[(9,39),(22,24),(45,16),(70,18),(86,12),(100,26),(104,40)], STONE + 4);
            a.poly(&[(9,39),(22,24),(45,16),(70,18),(63,26),(34,30)], STONE + 6);
            a.poly(&[(69,36),(86,12),(100,26),(104,40)], STONE + 3);
            a.ellipse(53, 28, 11, 10, INK + 1);
            a.rect(42, 28, 23, 13, INK + 1);
            a.line(42, 40, 42, 22, WOOD + 4);
            a.line(65, 40, 65, 22, WOOD + 4);
            a.line(41, 22, 66, 22, WOOD + 6);
            a.line(48, 40, 102, 21, IRON + 5);
            a.line(58, 42, 102, 25, IRON + 4);
            a
        }
        // The single unreferenced resource frame sits between timber and iron.
        _ => {
            let mut a = Art::new(38, 31);
            cottage(&mut a, 18, 27, 28, 14, COPPER);
            a
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_field_and_castle_layouts_have_deployment_and_terrain() {
        use l2_sim::{castle, terrain};
        let field_file = asset("batfield.pl8").unwrap();
        let fields = terrain::FieldSheets::parse(&field_file).unwrap();
        assert_eq!(fields.len(), terrain::FieldSheets::PLAYLIST);
        for i in 0..fields.len() {
            let field = terrain::build_field(fields.get(i).unwrap(), 1);
            assert!(field.deploy_side0.iter().chain(&field.deploy_side4).all(|&(x, y)| {
                x != 0 && y != 0 && !field.at(x as usize, y as usize).impassable()
            }));
            assert!(field.cells.iter().any(|c| c.terrain != terrain::id::OPEN));
        }

        let castle_file = asset("stnfield.pl8").unwrap();
        let castles = l2_sim::CastleSheets::parse(&castle_file).unwrap();
        assert_eq!(castles.len(), 20);
        for i in 0..castles.len() as u8 {
            let sheet = castles.get(i).unwrap();
            let field = castle::build(i, sheet);
            let tables = castle::tables(sheet);
            assert!(tables.deploy_side0.iter().chain(&tables.deploy_side4).all(|&(x, y)| x != 0 && y != 0));
            assert!(field.cells.iter().any(|c| c.flags & siege::FLAG_KEEP != 0));
            assert!(field.cells.iter().any(|c| c.flags & siege::FLAG_WALL != 0));
            if i >= 2 {
                assert!(field.cells.iter().any(|c| c.surface == siege::SURFACE_WATER));
                assert!(field.cells.iter().any(|c| c.elevation == siege::DOCK_WALL_ELEVATION));
            }
        }
    }

    #[test]
    fn original_village_icons_land_in_their_own_drop_regions() {
        let art = village::VillageArt::load(|name| asset(name).ok_or_else(|| name.to_owned())).unwrap();
        assert_eq!(art.tops_frames(), 6);
        assert!(art.has_grid() && art.has_villani1());
        for cluster in 0..village::CLUSTER_COUNT {
            for slot in 0..village::ICONS_PER_CLUSTER {
                let top = village::SCENE_Y_ADVANCED;
                let (x, y) = village::icon_hit_point(cluster, slot, top);
                assert_eq!(art.cluster_at(x, y, top), cluster + 1, "cluster {cluster}, slot {slot}");
            }
        }
    }
}
