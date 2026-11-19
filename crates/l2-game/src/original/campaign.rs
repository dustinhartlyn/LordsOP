//! Original relief-map tiles, campaign figures and county chrome.
use super::*;
use l2_view::campaign::{FAR, NEAR};

pub(super) fn asset(name: &str) -> Option<Vec<u8>> {
    // Use the renderer's filename table: far zoom deliberately shares one bank
    // across all seasons; sprite and flag banks are likewise not seasonal.
    for zoom in [NEAR, FAR] {
        let far = zoom.id == FAR.id;
        for (season, banks) in zoom.banks.iter().enumerate() {
            if let Some(bank) = banks.iter().position(|file| file.eq_ignore_ascii_case(name)) {
                let count = [140, 25, 140, 61, 100][bank];
                return Some(sheet((0..count).map(|frame| tile(bank, frame, season, far))));
            }
        }
        if let Some(index) = zoom.sprites.iter().position(|file| file.eq_ignore_ascii_case(name)) {
            return Some(sheet((0..if index == 0 { 168 } else { 48 }).map(|frame| unit(frame, index == 1, far))));
        }
        if zoom.flags.eq_ignore_ascii_case(name) {
            return Some(sheet((0..131).map(|frame| marker(frame, far))));
        }
    }
    match name {
        "panels.pl8" => Some(sheet((0..262).map(panel))),
        "misc_cty.pl8" => Some(sheet((0..97).map(county))),
        _ => None,
    }
}

const LAND: [u8; 4] = [GRASS, MOSS, COPPER, ICE];
const LEAF: [u8; 4] = [PINE, MOSS, COPPER, PINE];
const REALM: [u8; 5] = [RED, GOLD, SLATE, VIOLET, BLUE];

// Neighbours are NE, E, SE, S, SW, W, NW, N, matching the coast dresser.
// Edge neighbours reach diagonally across an entire shore; vertex neighbours
// leave a tongue of sand at their common corner instead.
const COAST: &[(i32, i32, u8)] = &[
    (38,41,1),(42,45,4),(46,49,16),(50,53,64),
    (54,55,17),(56,57,68),(58,61,5),(62,65,20),(66,69,80),(70,73,65),
    (74,74,69),(75,75,21),(76,76,84),(77,77,81),(78,79,85),
    (80,80,9),(81,81,33),(82,82,41),(83,83,36),(84,84,132),(85,85,164),
    (86,86,144),(87,87,18),(88,88,146),(89,89,66),(90,90,72),(91,91,74),
    (92,92,37),(93,93,148),(94,94,82),(95,95,73),
    (96,99,32),(100,103,128),(104,107,2),(108,111,8),
    (112,112,160),(113,113,130),(114,114,10),(115,115,40),(116,116,34),
    (117,117,136),(118,118,162),(119,119,138),(120,120,42),(121,121,168),(122,122,170),
];

// Track spokes by frame. Boundary tracks repeat the same run after frame 45.
const TRACK: [u8; 30] = [5, 10, 3, 6, 12, 9, 1, 2, 4, 8, 7, 11, 13, 14, 15,
                         5, 10, 3, 6, 12, 9, 1, 2, 4, 8, 7, 11, 13, 14, 15];

fn tile(bank: usize, frame: i32, season: usize, far: bool) -> Art {
    let (w, h) = (NEAR.tile_w, NEAR.tile_h);
    let forest = bank == 2 && matches!(frame, 30..=37 | 41..=45);
    let lift = match bank {
        1 => 25,
        2 if forest => 12,
        3 => 22,
        4 => 33,
        _ => 0,
    };
    let mut a = Art::new(w, h + lift);
    let coast = if bank == 0 { COAST.iter().find(|&&(lo, hi, _)| (lo..=hi).contains(&frame)).map_or(0, |&(_, _, mask)| mask) } else { 0 };
    let sea = bank == 0 && matches!(frame, 22..=129);
    let road = if bank == 2 {
        match frame {
            0..=29 => TRACK[frame as usize],
            46..=75 => TRACK[(frame - 46) as usize],
            128..=131 => 5,
            132..=135 => 10,
            136..=139 => 15,
            _ => 0,
        }
    } else {
        0
    };
    let field = (bank == 2 && (80..=123).contains(&frame)) || (bank == 0 && frame >= 130);
    let boundary = if bank != 2 {
        0
    } else if (124..=139).contains(&frame) {
        1 << ((frame - 124) % 4)
    } else if matches!(frame, 38..=75 | 81..=83) {
        [1, 2, 4, 8][frame as usize % 4]
    } else {
        0
    };
    let land = LAND[season];
    let seed = frame as u32 * 29 + bank as u32 * 119;
    for y in 0..h {
        let span = 1 + 2 * y.min(h - 1 - y);
        for x in w / 2 - span..w / 2 + span {
            let u = x * 2 + 1 - w;
            let v = (y * 2 + 1 - h) * 2;
            let n = noise(x, y, seed);
            let comb = noise(x / 6, y / 3, seed + 1);
            let grain = ((n % 5 == 0) as u8) + ((comb % 5 == 0) as u8);
            let mut c = land + 3 + grain;
            if bank == 0 && frame < 6 {
                c = INK + 1 + grain;
            } else if sea {
                let distances = [60 - u + v, 58 - u, 60 - u - v, 58 - v,
                                 60 + u - v, 58 + u, 60 + u + v, 58 + v];
                let edge = distances.into_iter().enumerate().filter(|(i, _)| coast & (1 << i) != 0)
                    .map(|(i, d)| d / if i % 2 == 0 { 2 } else { 1 }).min().unwrap_or(60);
                let edge = edge + (comb % 5) as i32;
                c = if edge < 7 { land + 2 + grain }
                    else if edge < 12 { PARCHMENT + 3 + grain }
                    else if edge < 16 || (n % 13 < 4 && (x + y * 3 + frame) % 19 < 5) { WATER + 5 + grain.min(1) }
                    else { WATER + 2 + grain };
            } else if field {
                let furrow = (u + v + 118 + frame * 2) % 15;
                let inside = u.abs() + v.abs() < 50;
                if inside {
                    c = match frame {
                        104..=107 => if furrow < 4 { GRASS + 5 } else { land + 3 + grain },
                        88..=103 if furrow < 5 + (frame - 88) / 4 => if n % 5 == 0 { GOLD + 6 } else { GOLD + 3 + grain },
                        130..=139 if furrow < 5 => PARCHMENT + 3 + grain,
                        108..=123 if n % 16 < ((frame - 108) / 4 + 1) as u32 * 3 => land + 4 + grain,
                        _ if furrow < 4 => WOOD + 2 + grain,
                        _ => COPPER + 2 + grain,
                    };
                }
            }
            let along = [u - v, u + v, v - u, -u - v];
            let mut track = false;
            if road != 0 {
                let distances = [(u + v).abs(), (u - v).abs(), (u + v).abs(), (u - v).abs()];
                let dist = distances.into_iter().zip(along).enumerate()
                    .filter(|(i, (_, d))| road & (1 << i) != 0 && *d >= -4)
                    .map(|(_, (d, _))| d).min().unwrap_or(60);
                track = dist < 10;
                if track { c = if dist < 6 { PARCHMENT + 3 + grain } else { WOOD + 3 + grain }; }
            }
            if boundary != 0 && !track && along.into_iter().enumerate().any(|(i, d)| boundary & (1 << i) != 0 && d >= 50) {
                c = if dither(x, y) { GOLD + 5 } else { WOOD + 2 };
            }
            a.dot(x, y + lift, c);
        }
    }
    let middle = lift + h / 2;
    if forest {
        for (i, (x, dy)) in [(20, -5), (34, -3), (13, 2), (27, 4), (43, 4), (33, 9)].into_iter().enumerate() {
            let v = noise(x, dy, seed + i as u32);
            pine(&mut a, x + (v % 5) as i32 - 2, middle + dy, 5 + (v % 3) as i32, 12 + (v % 7) as i32, season);
        }
    }
    match bank {
        1 => {
            let rock = if season == 3 { ICE } else { STONE };
            let rise = if frame >= 16 { 19 + (4 - (frame - 20).abs()).max(0) * 3 } else { 18 + (frame * 7) % 14 };
            peak(&mut a, 17, middle + 2, 14, rise * 2 / 3, rock, seed);
            peak(&mut a, 34, middle + 6, 21, rise, rock, seed + 1);
        }
        3 => site(&mut a, frame, middle, season),
        4 => castle(&mut a, frame, middle, season),
        _ => (),
    }
    if !far { return a; }
    // Sample the relief at the actual 5:1 projection. The base is rebuilt
    // on the far lattice to close the corners after sampling.
    let mut small = Art::new(FAR.tile_w, FAR.tile_h + (lift + 4) / 5);
    let top = small.h - FAR.tile_h;
    for y in 0..small.h {
        let sy = (lift + (y - top) * 5 + 2).max(0);
        for x in 0..small.w {
            let sx = x * 5 + 6;
            let c = a.pixels[(sy * a.w + sx) as usize];
            let span = 1 + 2 * (y - top).min(small.h - 1 - y);
            let on_diamond = y >= top && (small.w / 2 - span..small.w / 2 + span).contains(&x);
            if on_diamond || lift != 0 && c != 0 {
                let fallback = if sea { WATER + 3 } else if bank == 0 && frame < 6 { INK + 2 } else { land + 3 };
                small.dot(x, y, if c != 0 { c } else { fallback });
            }
        }
    }
    small
}

fn dither(x: i32, y: i32) -> bool {
    (x + 2 * y) % 8 < 5
}

fn pine(a: &mut Art, x: i32, y: i32, w: i32, h: i32, season: usize) {
    let green = LEAF[season];
    a.ellipse(x + 2, y, w, 2, LAND[season] + 1);
    a.line(x, y - 5, x, y + 1, WOOD + 2);
    for i in (0..3).rev() {
        let top = y - h + i * h / 5;
        let low = top + h / 2;
        let spread = w - 2 + i;
        a.poly(&[(x, top), (x - spread, low - 1), (x - 2, low + 1), (x + spread, low)], PINE);
        a.poly(&[(x, top + 2), (x - spread + 1, low - 1), (x - 1, low), (x + 1, low - 2)], if season == 3 { ICE + 4 } else { green + 5 });
        a.line(x + 1, top + 3, x + spread - 1, low - 1, green + 2);
    }
}

fn peak(a: &mut Art, x: i32, y: i32, w: i32, h: i32, rock: u8, seed: u32) {
    let p = [(x - w, y - 1), (x - w / 2, y - h / 2), (x - 3, y - h), (x + 5, y - h * 3 / 4), (x + w, y), (x + 1, y + 4)];
    a.poly(&p, SLATE + 1);
    a.poly(&[(x - w + 2, y - 1), p[1], (x - 3, y - h + 1), (x + 1, y - h / 3), (x - 3, y + 2)], rock + 4);
    a.poly(&[p[2], p[3], (x + w - 2, y - 1), (x + 1, y + 3), (x + 2, y - h / 3)], SLATE + 4);
    a.poly(&[p[1], p[2], (x + 2, y - h * 2 / 3), (x - 3, y - h * 3 / 4), (x - 7, y - h / 2)], ICE + 5);
    for i in 0..12 {
        let xx = x - w + 5 + (noise(i, x, seed) % (2 * w - 10) as u32) as i32;
        let yy = y - 2 - (noise(i, y, seed + 3) % (h / 3) as u32) as i32;
        if a.pixels[(yy * a.w + xx) as usize] == rock + 4 {
            a.line(xx, yy, xx + 1, yy - 1, rock + 3);
        }
    }
}

fn house(a: &mut Art, x: i32, y: i32, w: i32, h: i32, roof: u8, snow: bool) {
    let l = x - w;
    let r = x + w;
    let eave = y - h;
    let high = eave - w + 1;
    a.ellipse(x + 3, y + 1, w + 2, 2, INK + 4);
    a.poly(&[(l, eave), (x, eave - 3), (r, eave), (r, y - 3), (x, y + 2), (l, y - 1)], INK + 1);
    a.poly(&[(l + 1, eave + 1), (x, eave - 1), (x, y + 1), (l + 1, y - 2)], PARCHMENT + 5);
    a.poly(&[(x + 1, eave - 1), (r - 1, eave + 1), (r - 1, y - 4), (x + 1, y + 1)], WOOD + 4);
    a.rect(x - 4, y - 6, 3, 6, INK + 2);
    a.dot(x - 3, y - 4, GOLD + 4);
    a.rect(x + 3, y - 8, 2, 3, INK + 1);
    a.dot(x + 3, y - 8, GOLD + 4);
    a.poly(&[(l - 2, eave), (x - 3, high), (x + 4, high - 2), (r + 1, eave - 1), (x, eave + 2)], INK + 2);
    a.poly(&[(l, eave - 1), (x - 3, high + 1), (x + 4, high - 1), (x, eave + 1)], if snow { ICE + 6 } else { roof + 5 });
    a.poly(&[(x + 4, high), (r, eave - 1), (x + 1, eave + 1)], if snow { ICE + 3 } else { roof + 2 });
    for i in (2..w).step_by(3) {
        a.line(l + i + 1, eave - i, x - 1 + i / 2, high + i / 2, if snow { ICE + 4 } else { roof + 3 });
    }
}

fn site(a: &mut Art, frame: i32, y: i32, season: usize) {
    let snow = season == 3;
    match frame {
        47..=58 => {
            let part = (frame - 47) % 4;
            let tier = (frame - 47) / 4;
            let x = [30, 35, 23, 29][part as usize];
            a.ellipse(x, y + 2, 19, 8, PARCHMENT + 3);
            if part == 0 {
                house(a, x, y + 4, 10, 12 + tier * 2, SLATE, snow);
                if tier > 0 {
                    a.rect(x - 6, y - 23, 5, 15, STONE + 4);
                    a.poly(&[(x - 8, y - 22), (x - 4, y - 32), (x + 1, y - 22)], SLATE + 2);
                    a.dot(x - 4, y - 32, GOLD + 6);
                }
            } else {
                house(a, x - 8, y, 7, 7, RED, snow);
                if tier > 0 || part < 3 {
                    house(a, x + 5, y + 5, 8, 8, COPPER, snow);
                }
                if tier == 2 {
                    house(a, x - 6, y + 10, 5, 6, RED, snow);
                } else if part == 3 {
                    a.ellipse(x + 4, y + 5, 5, 3, STONE + 1);
                    a.ellipse(x + 4, y + 4, 3, 2, WATER + 3);
                }
            }
        }
        20..=29 => {
            pine(a, 17, y + 1, 8, 23, season);
            pine(a, 36, y - 3, 8, 22, season);
            house(a, 27, y + 5, 7, 7, WOOD, snow);
            for j in 0..4 {
                let dx = (frame + j) % 4;
                a.line(33 + dx, y + 6 + j, 47 + dx, y + 3 + j, WOOD + 2);
                a.dot(33 + dx, y + 6 + j, GOLD + 4);
            }
            if frame == 29 {
                a.line(26, y + 1, 33, y - 11, INK + 2);
            }
        }
        30..=46 => {
            peak(a, 25, y + 3, 17, 20, STONE, frame as u32);
            a.ellipse(25, y + 1, 8, 8, WOOD + 1);
            a.ellipse(25, y + 2, 6, 7, INK + 1);
            a.line(22, y + 5, 41, y + 10, WOOD + 3);
            a.line(28, y + 4, 46, y + 7, WOOD + 4);
            let x = 34 + (frame - 30) % 11;
            a.poly(&[(x - 4, y + 4), (x + 4, y + 3), (x + 3, y + 8), (x - 3, y + 8)], IRON + 3);
            a.dot(x, y + 3, IRON + 6);
            a.dot(x - 2, y + 9, INK + 1);
            a.dot(x + 2, y + 8, INK + 1);
            if frame == 46 { a.line(18, y + 5, 32, y - 4, WOOD + 4); }
        }
        10..=19 => {
            house(a, 27, y + 2, 12, 9, SLATE, snow);
            a.rect(36, y - 22, 4, 11, STONE + 2);
            if frame != 19 {
                let phase = (frame - 10) % 3;
                a.ellipse(37 + phase, y - 24 - phase, 4, 2, SLATE + 5);
                a.dot(27, y - 2, RED + 7);
            }
            a.rect(40, y + 1, 7, 3, IRON + 2);
            a.rect(42, y, 4, 2, IRON + 6);
        }
        0..=9 => {
            a.ellipse(28, y + 2, 22, 10, WOOD + 3);
            a.ellipse(27, y, 19, 8, STONE + 2);
            a.ellipse(26, y - 1, 15, 6, STONE + 5);
            a.poly(&[(11, y - 1), (21, y - 9), (35, y - 7), (34, y), (23, y + 3)], STONE + 4);
            a.line(14, y - 2, 25, y - 7, STONE + 6);
            let x = 37 + (frame % 5);
            a.line(x, y + 4, x - 4, y - 14, WOOD + 1);
            a.line(x - 4, y - 14, x + 6, y - 11, WOOD + 4);
            a.line(x + 5, y - 11, x + 5, y + 2, GOLD + 4);
            if frame != 9 { a.rect(x + 4, y + 1, 4, 3, STONE + 6); }
        }
        _ => house(a, 29, y + 2, 8, 6, RED, snow),
    }
    if matches!(frame, 9 | 19 | 29 | 46) {
        a.poly(&[(15, y + 5), (21, y - 1), (32, y + 4), (43, y + 2), (46, y + 8), (27, y + 12)], INK + 4);
        for (dx, dy, c) in [(18, 5, WOOD + 2), (24, 7, STONE + 3), (31, 7, WOOD + 2), (41, 5, SLATE + 4), (36, 9, STONE + 3)] {
            a.line(dx, y + dy, dx + 4, y + dy - 3, c);
            a.dot(dx + 1, y + dy - 1, STONE + 5);
        }
    }
}

fn tower(a: &mut Art, x: i32, y: i32, w: i32, h: i32, snow: bool) {
    let t = y - h;
    a.poly(&[(x - w, t), (x, t - 4), (x + w, t), (x + w, y - 2), (x, y + 2), (x - w, y - 1)], INK + 1);
    a.poly(&[(x - w + 1, t + 1), (x - 1, t + 4), (x - 1, y + 1), (x - w + 1, y - 2)], STONE + 5);
    a.poly(&[(x, t + 4), (x + w - 1, t + 1), (x + w - 1, y - 3), (x, y + 1)], SLATE + 4);
    a.poly(&[(x - w + 1, t), (x, t - 4), (x + w - 1, t), (x, t + 4)], if snow { ICE + 5 } else { STONE + 3 });
    for i in (t + 7..y).step_by(4) {
        a.line(x - w + 1, i - 3, x - 2, i, STONE + 3);
        a.line(x + 1, i, x + w - 1, i - 3, SLATE + 2);
    }
    a.rect(x - 4, t + 7, 2, 4, INK + 1);
    a.dot(x - 3, t + 7, GOLD + 3);
    for (dx, dy) in [(-w, 0), (-w / 2, 2), (0, 4), (w / 2, 2), (w - 2, 0)] {
        a.rect(x + dx, t + dy - 3, 3, 4, INK + 1);
        a.rect(x + dx, t + dy - 3, 2, 3, if snow { ICE + 6 } else { STONE + 6 });
    }
}

fn castle(a: &mut Art, frame: i32, y: i32, season: usize) {
    let part = frame % 4;
    let snow = season == 3;
    let x = [29, 34, 24, 29][part as usize];
    a.ellipse(x, y + 3, 20, 8, PARCHMENT + 2);
    match frame {
        40..=59 => {
            a.poly(&[(x - 18, y), (x - 1, y - 8), (x + 17, y), (x, y + 9)], STONE + 4);
            for dx in [-13, -5, 5, 13] {
                a.line(x + dx, y + 3, x + dx, y - 15, WOOD + 1);
                a.line(x + dx + 1, y + 2, x + dx + 1, y - 14, WOOD + 5);
            }
            a.line(x - 15, y - 12, x + 14, y - 12, WOOD + 4);
            a.line(x - 13, y + 2, x + 12, y - 13, WOOD + 4);
        }
        _ => {
            let tier = (frame % 20) / 4;
            let height = if (60..80).contains(&frame) { 8 + tier } else { 12 + tier * 3 };
            a.poly(&[(x - 21, y), (x, y - 10), (x + 21, y), (x, y + 10)], STONE + 3);
            if part == 0 {
                tower(a, x, y + 1, 10 + tier / 2, height + 8, snow);
                if tier >= 2 { tower(a, x - 13, y + 4, 6, height - 2, snow); }
            } else {
                tower(a, x - 9, y + 1, 7, height, snow);
                tower(a, x + 9, y + 3, 7, height + 2, snow);
                a.poly(&[(x - 8, y - 9), (x + 10, y - 9), (x + 10, y + 3), (x, y + 7), (x - 8, y + 4)], STONE + 4);
                if part == 3 {
                    a.ellipse(x, y + 1, 4, 5, INK + 1);
                    a.rect(x - 4, y + 1, 9, 6, INK + 1);
                    a.line(x - 3, y + 8, x + 6, y + 8, WOOD + 5);
                } else {
                    a.line(x - 8, y - 9, x + 8, y - 9, STONE + 6);
                    a.rect(x - 3, y - 5, 2, 3, INK + 2);
                }
            }
        }
    }
}

fn person(a: &mut Art, x: i32, y: i32, facing: usize, phase: i32, coat: u8, armed: bool) {
    let turn = [0, 1, 1, 1, 0, -1, -1, -1][facing];
    let gait = [0, 1, 2, 1, 0, -1][phase as usize];
    a.ellipse(x, y, 6, 2, INK + 4);
    a.line(x - 2, y - 7, x - 3 - gait, y, INK + 1);
    a.line(x + 1, y - 7, x + 2 + gait, y - 1, WOOD + 1);
    a.dot(x - 3 - gait, y, WOOD + 4);
    a.poly(&[(x - 4, y - 14), (x + 3, y - 15), (x + 5, y - 6), (x - 5, y - 5)], INK + 1);
    a.poly(&[(x - 3, y - 13), (x, y - 14), (x + 1, y - 7), (x - 4, y - 6)], coat + 5);
    a.poly(&[(x + 1, y - 13), (x + 3, y - 14), (x + 4, y - 7), (x + 1, y - 7)], coat + 3);
    a.line(x - 4, y - 8, x + 4, y - 9, WOOD + 2);
    a.dot(x, y - 9, GOLD + 6);
    a.ellipse(x + turn, y - 17, 3, 3, INK + 1);
    a.rect(x - 1 + turn, y - 17, 3, 3, PARCHMENT + 6);
    a.poly(&[(x - 4 + turn, y - 17), (x - 2 + turn, y - 21), (x + 1 + turn, y - 21), (x + 4 + turn, y - 17)],
        if armed { IRON + 4 } else { WOOD + 4 });
    a.line(x - 2 + turn, y - 20, x + turn, y - 20, if armed { IRON + 7 } else { GOLD + 4 });
    if armed {
        let dir = if turn < 0 { -1 } else { 1 };
        a.line(x + dir * 5, y - 3, x + dir * 7, y - 23, WOOD + 3);
        a.line(x + dir * 7, y - 23, x + dir * 7, y - 26, IRON + 7);
        a.poly(&[(x - dir * 7, y - 12), (x - dir * 3, y - 11), (x - dir * 5, y - 5), (x - dir * 8, y - 7)], SLATE + 2);
        a.dot(x - dir * 5, y - 9, GOLD + 6);
    }
}

fn unit(frame: usize, transport: bool, far: bool) -> Art {
    let caravan = frame < 48 || transport;
    if !caravan && frame < 72 { return Art::new(3, 4); } // Unreachable in Map_DrawArmies.
    let class = if caravan { 0 } else { (frame - 72) / 24 };
    let stride = if caravan { 6 } else { 3 };
    let facing = (frame / stride) & 7;
    let phase = (frame % stride) as i32;
    if far {
        // The renderer puts a far army's owner pennant 24 px right of
        // its frame origin. Transparent margins keep it over the formation.
        // Unit picking checks opacity, so the margins are not clickable.
        let (w, h) = if caravan { (12, 10) } else if class == 3 { (14, 13) } else { (44, 13) };
        let mut a = Art::new(w, h);
        let x = a.w / 2;
        let y = a.h - 2;
        a.ellipse(x, y, 5, 1, INK + 3);
        if caravan {
            a.ellipse(x, y - 2, 4, 3, WOOD + 2);
            a.poly(&[(x - 4, y - 3), (x - 2, y - 7), (x + 3, y - 6), (x + 4, y - 3)], if transport { BLUE + 5 } else { PARCHMENT + 6 });
            a.dot(x - 2, y, WOOD + 5);
            a.dot(x + 2, y - 1, WOOD + 5);
        } else {
            for i in 0..(class.min(2) + 2) as i32 {
                let xx = x + i * 3 - 5;
                let yy = y - i % 2 - (phase + i) % 2;
                a.line(xx, yy, xx, yy - 4, if class == 3 { COPPER + 3 } else { SLATE + 4 });
                a.dot(xx, yy - 5, if class == 3 { PARCHMENT + 6 } else { IRON + 7 });
                a.line(xx + 1, yy - 2, xx + 1, yy - 6, WOOD + 5);
            }
        }
        return a;
    }
    let mut a = if caravan { Art::new(40, 32) } else { Art::new(53, 44) };
    if caravan {
        let y = 26;
        let dir = if facing > 4 { -1 } else { 1 };
        let x = 20 - dir * 6;
        let rise = phase % 3 / 2;
        a.ellipse(21, y, 16, 3, INK + 4);
        a.poly(&[(x - 10, y - 8), (x - 7, y - 15 + rise), (x + 6, y - 14 + rise), (x + 11, y - 8), (x + 7, y - 4), (x - 7, y - 4)], INK + 1);
        a.poly(&[(x - 9, y - 9), (x - 6, y - 15 + rise), (x + 5, y - 14 + rise), (x + 9, y - 9), (x + 5, y - 7), (x - 7, y - 7)],
            if transport { BLUE + 5 } else { PARCHMENT + 6 });
        a.line(x - 7, y - 11, x + 7, y - 10, if transport { BLUE + 3 } else { PARCHMENT + 3 });
        a.ellipse(x - 5, y - 4, 4, 4, WOOD + 1);
        a.ellipse(x + 5, y - 4, 4, 4, WOOD + 1);
        for xx in [x - 5, x + 5] {
            a.ellipse(xx, y - 4, 2, 2, WOOD + 5);
            a.dot(xx + phase % 2, y - 4, IRON + 7);
        }
        let ox = 21 + dir * 9;
        a.ellipse(ox, y - 6, 6, 4, WOOD + 3);
        a.ellipse(ox + dir * 4, y - 9, 4, 3, WOOD + 5);
        a.line(ox + dir * 3, y - 11, ox + dir * 4, y - 14, PARCHMENT + 6);
        a.line(ox - dir * 3, y - 4, ox - dir * (2 + phase % 3), y, INK + 1);
        a.line(ox + dir * 2, y - 4, ox + dir * (2 - phase % 3), y - 1, INK + 1);
        a.dot(ox + dir * 5, y - 10, INK + 1);
    } else {
        let arm = class != 3;
        let crowd = [2, 3, 5, 4][class];
        for i in 0..crowd {
            let (x, y) = [(25, 29), (18, 34), (34, 35), (12, 40), (27, 41)][i];
            person(&mut a, x, y, facing, (phase + i as i32) % 6, if arm { [SLATE, IRON, BLUE][i % 3] } else { [WOOD, COPPER, MOSS][i % 3] }, arm);
        }
    }
    a
}

fn pennant(a: &mut Art, x: i32, top: i32, w: i32, h: i32, realm: usize, phase: usize) {
    let color = REALM[realm % REALM.len()];
    let flap = [0, 1, 2, 3, 2, 1, 0, -1][phase % 8];
    let tail = top + 2 + flap;
    a.line(x, top, x, top + h, WOOD + 1);
    a.line(x + 1, top, x + 1, top + h, GOLD + 4);
    a.dot(x, top - 1, GOLD + 7);
    a.poly(&[(x + 1, top), (x + w, tail), (x + w - 3, tail + 4), (x + w, tail + 8), (x + 1, top + 11)], INK + 1);
    a.poly(&[(x + 2, top + 1), (x + w - 2, tail + 1), (x + w - 4, tail + 4), (x + w - 2, tail + 7), (x + 2, top + 10)], color + 4);
    a.line(x + 2, top + 2, x + w - 3, tail + 2, color + 6);
    heraldry(a, x + w / 2, top + 5, realm, CREAM + 6);
}

fn heraldry(a: &mut Art, x: i32, y: i32, realm: usize, c: u8) {
    match realm % 5 {
        0 => {
            a.line(x, y - 3, x, y + 3, c);
            a.line(x - 2, y - 1, x + 2, y - 1, c);
        }
        1 => a.poly(&[(x, y - 3), (x - 3, y + 1), (x, y), (x + 3, y + 1), (x, y + 3)], c),
        2 => {
            a.poly(&[(x - 3, y + 2), (x - 3, y - 2), (x - 1, y), (x, y - 3), (x + 1, y), (x + 3, y - 2), (x + 3, y + 2)], c);
        }
        3 => {
            a.ellipse(x, y, 2, 2, c);
            a.dot(x, y - 4, c);
            a.dot(x, y + 4, c);
            a.dot(x - 4, y, c);
            a.dot(x + 4, y, c);
        }
        _ => {
            a.line(x - 2, y - 3, x + 2, y + 3, c);
            a.line(x + 2, y - 3, x - 2, y + 3, c);
            a.dot(x, y - 3, c);
        }
    }
}

fn marker(frame: usize, far: bool) -> Art {
    match frame {
        0..=39 | 121..=128 => {
            let (realm, phase) = if frame < 40 { (frame / 8, frame % 8) } else { (1, frame - 121) };
            if far {
                let top = if frame < 40 { 10 } else { 1 };
                let mut a = Art::new(9, top + 15);
                a.line(1, top, 1, top + 13, GOLD + 5);
                a.poly(&[(1, top + 1), (8, top + 1 + (phase % 3) as i32), (6, top + 5), (1, top + 7)], REALM[realm] + 5);
                a.dot(3, top + 3, CREAM + 6);
                a
            } else {
                let mut a = Art::new(32, 24);
                pennant(&mut a, 2, 3, 22, 20, realm, phase);
                a
            }
        }
        40..=55 => {
            let mut a = Art::new(26, 25);
            let phase = (frame - 40) as i32;
            for i in 0..5 {
                let age = (phase + i * 3) % 16;
                let x = 12 + age / 2 - i * 2;
                a.ellipse(x, 23 - age, 2 + age / 7, 2 + age / 8, SLATE + 4 - (age / 6) as u8);
            }
            a.poly(&[(4, 24), (8, 18), (10, 21), (14, 14 + phase % 4), (17, 24)], RED + 4);
            a.line(10, 24, 13, 19, GOLD + 6);
            a
        }
        56..=78 => {
            let w = if far { 5 } else { 15 };
            let mut a = Art::new(w, w);
            let r = w / 2;
            let (color, shine) = match frame {
                56 => (IRON, 3),
                78 => (GOLD, 5),
                _ => (BLUE, 3 + ((frame - 57) / 7) as u8),
            };
            a.ellipse(r, r, r, r, INK + 1);
            a.ellipse(r, r, r - 1, r - 1, color + shine);
            a.dot(r - 1, r - 1, color + 7);
            if !far && frame != 56 {
                a.ellipse(r - 2, r - 2, 2, 1, color + 6);
                if frame == 78 {
                    a.line(r - 2, r + 1, r, r + 3, GOLD + 7);
                    a.line(r, r + 3, r + 3, r - 1, GOLD + 7);
                }
            }
            a
        }
        85..=120 => {
            let mut a = Art::new(46, 27);
            let dead = frame >= 103;
            let start = if dead { 103 } else { 85 };
            let phase = ((frame - start) % 6) as i32;
            for i in 0..((frame - start) / 6 + 1) as i32 {
                let x = 12 + i * 10 + phase / 2;
                let y = 14 + i * 3;
                a.ellipse(x, y, 7, 2, INK + 4);
                a.ellipse(x, y - if dead { 1 } else { 4 }, 6, 3, PARCHMENT + 4);
                a.ellipse(x + 2, y - if dead { 1 } else { 4 }, 3, 3, WOOD + 2);
                if !dead {
                    a.line(x - 3, y - 3, x - 4 + phase % 2, y, INK + 1);
                    a.line(x + 3, y - 3, x + 3 - phase % 2, y, INK + 1);
                    a.ellipse(x + 5, y - 2 - phase / 3, 2, 2, CREAM + 3);
                }
            }
            a
        }
        129 => {
            let (w, h) = if far { (11, 24) } else { (24, 24) };
            let mut a = Art::new(w, h);
            let x = w / 2;
            let y = if far { 16 } else { 10 };
            a.poly(&[(x, y - 6), (x + 5, y - 3), (x + 4, y + 3), (x, y + 6), (x - 4, y + 3), (x - 5, y - 3)], INK + 1);
            heraldry(&mut a, x, y, 4, GOLD + 7);
            a
        }
        130 => {
            let mut a = Art::new(24, 28);
            a.poly(&[(2, 13), (10, 2), (15, 3), (22, 12), (14, 17)], INK + 1);
            a.poly(&[(4, 13), (10, 4), (13, 15)], PARCHMENT + 5);
            a.poly(&[(12, 4), (15, 4), (21, 12), (14, 15)], RED + 4);
            a.line(10, 5, 9, 14, WOOD + 1);
            a.dot(10, 2, GOLD + 7);
            a
        }
        _ => Art::new(2, 2), // Unreferenced marker slots.
    }
}

fn finish(a: &mut Art, base: u8, level: u8, seed: u32) {
    for y in 0..a.h {
        for x in 0..a.w {
            let q = noise(x, y, seed);
            let streak = noise(x / 9 + y / 13, y, seed + 2);
            let value = (u8::from(q % 19 == 0) + u8::from(streak % 11 == 0)) as i32;
            a.dot(x, y, base + (i32::from(level) + value - 1).clamp(0, 7) as u8);
        }
    }
}

fn border(a: &mut Art, x: i32, y: i32, w: i32, h: i32, dark: u8, light: u8) {
    a.line(x, y, x + w - 1, y, light);
    a.line(x, y, x, y + h - 1, light);
    a.line(x + w - 1, y, x + w - 1, y + h - 1, dark);
    a.line(x, y + h - 1, x + w - 1, y + h - 1, dark);
}

fn stud(a: &mut Art, x: i32, y: i32) {
    a.ellipse(x, y + 1, 2, 2, WOOD);
    a.ellipse(x, y, 2, 2, GOLD + 3);
    a.dot(x - 1, y - 1, GOLD + 7);
}

fn shield(a: &mut Art, x: i32, y: i32, realm: usize) {
    let color = REALM[realm % 5];
    a.poly(&[(x - 6, y - 7), (x + 6, y - 7), (x + 5, y + 2), (x, y + 7), (x - 5, y + 2)], INK + 1);
    a.poly(&[(x - 5, y - 6), (x + 5, y - 6), (x + 4, y + 2), (x, y + 6), (x - 4, y + 2)], GOLD + 4);
    a.poly(&[(x - 4, y - 5), (x + 4, y - 5), (x + 3, y + 1), (x, y + 5), (x - 3, y + 1)], color + 4);
    heraldry(a, x, y - 1, realm, CREAM + 5);
}

fn panel(frame: usize) -> Art {
    match frame {
        52..=195 => {
            let mut a = Art::new(16, 16);
            finish(&mut a, INK, 3, frame as u32);
            a
        }
        196..=203 => {
            let mut a = Art::new(24, 24);
            finish(&mut a, SLATE, 2, frame as u32);
            for (y, color) in [(0, GOLD + 3), (1, GOLD + 6), (21, WOOD + 4), (22, GOLD + 5), (23, WOOD + 1)] {
                a.line(0, y, 23, y, color);
            }
            a
        }
        256..=260 => {
            let mut a = Art::new(13, 16);
            shield(&mut a, 6, 8, frame - 256);
            a
        }
        261 => {
            let mut a = Art::new(48, 48);
            a.ellipse(24, 24, 22, 22, INK + 2);
            a.ellipse(24, 24, 20, 20, GOLD + 5);
            a.ellipse(24, 24, 17, 17, SLATE + 3);
            a.poly(&[(18, 11), (30, 11), (28, 17), (25, 22), (28, 28), (30, 36), (18, 36), (20, 28), (23, 22), (20, 17)], ICE + 3);
            a.poly(&[(21, 32), (24, 27), (27, 32), (28, 35), (20, 35)], GOLD + 5);
            a.line(16, 11, 32, 11, GOLD + 7);
            a.line(16, 36, 32, 36, GOLD + 6);
            a
        }
        _ => {
            let mut a = Art::new(16, 16);
            finish(&mut a, WOOD, if frame >= 204 { 4 } else { 3 }, frame as u32);
            let i = frame % 204;
            let side = match i { 4..=15 => 1, 16..=27 => 2, 28..=39 => 4, 40..=51 => 8, 0 => 5, 1 => 9, 2 => 10, _ => 6 };
            for j in 0..16 {
                for (k, c) in [GOLD + 2, GOLD + 6, WOOD + 2, GOLD + 4].into_iter().enumerate() {
                    let k = k as i32;
                    if side & 3 != 0 { a.dot(j, k, c); a.dot(j, 15 - k, c); }
                    if side & 12 != 0 { a.dot(k, j, c); a.dot(15 - k, j, c); }
                }
            }
            if i < 4 {
                stud(&mut a, 8, 8);
                a.line(4, 12, 12, 4, GOLD + 4);
            } else if (i - 4) % 4 == 2 {
                stud(&mut a, 8, 8);
            }
            a
        }
    }
}

#[derive(Clone, Copy)]
enum Token { Swords, Crown, Cargo, Keep, Envoys, Hammer, Wheat, Joy, Zoom, Stone, Iron, Wood, Cattle }

fn token(a: &mut Art, x: i32, y: i32, icon: Token) {
    use Token::*;
    match icon {
        Swords => {
            a.line(x - 7, y - 7, x + 6, y + 6, INK + 2);
            a.line(x - 6, y - 7, x + 4, y + 3, IRON + 6);
            a.line(x + 7, y - 7, x - 6, y + 7, INK + 1);
            a.line(x + 6, y - 7, x - 4, y + 3, IRON + 7);
            a.line(x - 3, y + 6, x - 7, y + 2, GOLD + 5);
            a.line(x + 3, y + 6, x + 7, y + 2, GOLD + 5);
        }
        Crown => {
            a.poly(&[(x - 8, y - 5), (x - 4, y - 2), (x, y - 7), (x + 4, y - 2), (x + 8, y - 5), (x + 6, y + 6), (x - 6, y + 6)], WOOD);
            a.poly(&[(x - 7, y - 3), (x - 4, y), (x, y - 6), (x + 4, y), (x + 7, y - 3), (x + 5, y + 5), (x - 5, y + 5)], GOLD + 5);
            a.line(x - 4, y + 3, x + 4, y + 3, GOLD + 7);
            a.dot(x, y + 1, RED + 5);
        }
        Cargo => {
            a.rect(x - 7, y - 6, 15, 13, WOOD + 1);
            a.rect(x - 6, y - 5, 13, 10, WOOD + 5);
            for i in [-4, 4] { a.line(x + i, y - 5, x + i, y + 5, GOLD + 4); }
            a.line(x - 5, y, x + 5, y, WOOD + 2);
            a.rect(x - 1, y - 1, 3, 3, GOLD + 7);
        }
        Keep => {
            a.rect(x - 8, y - 5, 17, 12, SLATE + 1);
            a.rect(x - 7, y - 4, 14, 10, STONE + 5);
            for i in [-7, -1, 5] { a.rect(x + i, y - 7, 3, 4, STONE + 6); }
            a.line(x - 6, y + 1, x + 6, y + 1, STONE + 3);
            a.ellipse(x, y + 4, 2, 3, INK + 1);
        }
        Envoys => {
            shield(a, x - 4, y, 0);
            shield(a, x + 4, y, 4);
        }
        Hammer => {
            a.line(x - 6, y + 7, x + 4, y - 4, WOOD + 1);
            a.line(x - 5, y + 7, x + 5, y - 4, WOOD + 5);
            a.poly(&[(x - 6, y - 3), (x - 1, y - 8), (x + 8, y - 1), (x + 4, y + 3)], IRON + 2);
            a.line(x - 1, y - 7, x + 7, y - 1, IRON + 7);
        }
        Wheat => {
            for k in -1..=1 {
                let u = x + k * 4;
                a.line(x, y + 8, u, y - 5, WOOD + 2);
                for i in 0..3 {
                    a.ellipse(u - 1, y - i * 3, 2, 1, GOLD + 4);
                    a.dot(u, y - i * 3 - 1, GOLD + 7);
                }
            }
            a.line(x - 2, y + 5, x + 2, y + 5, RED + 4);
        }
        Joy => {
            a.ellipse(x, y, 7, 7, WOOD + 2);
            a.ellipse(x, y - 1, 6, 6, GOLD + 5);
            a.dot(x - 2, y - 3, INK + 1);
            a.dot(x + 2, y - 3, INK + 1);
            a.line(x - 3, y, x - 1, y + 2, WOOD + 2);
            a.line(x - 1, y + 2, x + 1, y + 2, WOOD + 2);
            a.line(x + 1, y + 2, x + 3, y, WOOD + 2);
        }
        Zoom => {
            a.line(x + 1, y + 1, x + 7, y + 7, WOOD + 1);
            a.line(x + 3, y + 1, x + 8, y + 6, GOLD + 5);
            a.ellipse(x - 2, y - 2, 6, 6, WOOD + 1);
            a.ellipse(x - 2, y - 2, 5, 5, GOLD + 6);
            a.ellipse(x - 2, y - 2, 3, 3, WATER + 4);
            a.dot(x - 3, y - 3, ICE + 7);
        }
        Stone | Iron => {
            let c = if matches!(icon, Iron) { IRON } else { STONE };
            a.poly(&[(x - 8, y + 4), (x - 6, y - 3), (x, y - 6), (x + 5, y - 3), (x + 8, y + 3), (x + 5, y + 6), (x - 5, y + 6)], SLATE + 2);
            a.poly(&[(x - 7, y + 3), (x - 5, y - 3), (x, y - 5), (x + 4, y - 3), (x + 1, y + 2)], c + 5);
            a.poly(&[(x - 6, y + 4), (x + 1, y + 2), (x + 7, y + 3), (x + 4, y + 5), (x - 5, y + 5)], c + 3);
            a.dot(x - 3, y - 2, c + 7);
        }
        Wood => {
            for d in [0, 3, -3] {
                a.line(x - 6 + d, y + 5, x + 3 + d, y - 4, WOOD + 2);
                a.line(x - 6 + d, y + 4, x + 3 + d, y - 5, WOOD + 5);
                a.ellipse(x - 6 + d, y + 5, 2, 2, GOLD + 4);
                a.dot(x - 6 + d, y + 5, WOOD + 3);
            }
        }
        Cattle => {
            a.ellipse(x - 1, y - 1, 8, 5, WOOD + 2);
            a.ellipse(x - 2, y - 2, 6, 4, PARCHMENT + 5);
            a.ellipse(x + 6, y - 1, 4, 3, WOOD + 4);
            a.dot(x + 7, y - 2, INK + 1);
            a.line(x + 6, y - 3, x + 8, y - 6, PARCHMENT + 5);
            a.line(x - 5, y + 2, x - 5, y + 6, WOOD + 1);
            a.line(x + 2, y + 2, x + 3, y + 6, WOOD + 1);
        }
    }
}

fn county(frame: usize) -> Art {
    match frame {
        0..=22 => {
            if matches!(frame, 5 | 6 | 11 | 12) { return Art::new(2, 2); }
            let mut a = Art::new(16, 32);
            let (coat, selected) = match frame {
                0 => (RED, false),
                1 | 2 => (SLATE, frame == 2),
                3 | 4 => (GOLD, frame == 4),
                7 | 8 => (MOSS, frame == 8),
                9 | 10 => (PARCHMENT, frame == 10),
                13 | 14 => (BLUE, frame == 14),
                15 | 16 => (IRON, frame == 16),
                17 | 18 => (STONE, frame == 18),
                19 | 20 => (WOOD, frame == 20),
                _ => (COPPER, frame == 22),
            };
            if selected { a.ellipse(8, 28, 7, 3, GOLD + 6); }
            person(&mut a, 8, 27, 3, 0, if selected { GOLD } else { coat }, false);
            a
        }
        0x36..=0x3B | 0x42 => {
            let h = match frame { 0x36 => 132, 0x37 => 94, 0x38 => 128, 0x39 => 30, 0x3A => 274, 0x3B => 20, _ => 52 };
            let mut a = Art::new(162, h);
            if frame == 0x39 {
                finish(&mut a, PARCHMENT, 5, frame as u32);
            } else {
                finish(&mut a, SLATE, 2, frame as u32);
            }
            border(&mut a, 0, 0, 162, h, WOOD + 1, WOOD + 3);
            border(&mut a, 1, 1, 160, h - 2, GOLD + 2, GOLD + 5);
            if frame == 0x36 {
                a.rect(0, 3, 129, 129, SLATE + 1);
                a.rect(128, 3, 2, 129, GOLD + 3);
            } else if frame == 0x39 {
                for (i, icon) in [Token::Swords, Token::Crown, Token::Cargo, Token::Keep, Token::Envoys].into_iter().enumerate() {
                    let x = i as i32 * 32 + 2;
                    border(&mut a, x, 2, 30, 26, WOOD + 2, GOLD + 6);
                    token(&mut a, x + 15, 15, icon);
                }
            } else if frame == 0x3B {
                stud(&mut a, 10, 10);
                stud(&mut a, 151, 10);
            } else if frame == 0x37 {
                a.line(12, 26, 149, 26, GOLD + 4);
                a.line(12, 52, 68, 52, GOLD + 2);
                a.line(92, 52, 149, 52, GOLD + 2);
                token(&mut a, 17, 41, Token::Envoys);
                token(&mut a, 110, 41, Token::Joy);
            } else if frame == 0x38 {
                a.line(81, 8, 81, h - 9, GOLD + 2);
            } else if frame == 0x42 {
                a.rect(59, 16, 50, 20, PARCHMENT + 1);
                border(&mut a, 58, 15, 52, 22, GOLD + 5, WOOD + 3);
                token(&mut a, 29, 26, Token::Wheat);
                token(&mut a, 131, 26, Token::Hammer);
            } else {
                a.line(14, 42, 147, 42, GOLD + 4);
                a.line(14, 210, 147, 210, GOLD + 4);
                shield(&mut a, 80, 195, 2);
            }
            a
        }
        0x46..=0x4A => {
            let mut a = Art::new(14, 59);
            a.rect(4, 1, 6, 50, WOOD + 1);
            a.rect(5, 2, 4, 51, STONE + 4);
            let n = 12 + 8 * (frame - 0x46) as i32;
            a.rect(5, 51 - n, 4, n, RED + 4);
            a.ellipse(7, 51, 6, 6, WOOD + 1);
            a.ellipse(7, 51, 5, 5, RED + 4);
            a.dot(5, 49, RED + 7);
            for y in (5..46).step_by(8) { a.dot(10, y, GOLD + 4); }
            a
        }
        0x4B..=0x55 => {
            let plain = [0x21, 0x26, 0x3F, 0x40, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x3D][frame - 0x4B];
            let b = county(plain);
            let (w, h) = if frame == 0x4E { (32, 34) } else { (b.w + 4, b.h + 4) };
            let mut a = Art::new(w, h);
            let ox = (w - b.w) / 2;
            let oy = (h - b.h) / 2;
            for y in 0..b.h {
                for x in 0..b.w {
                    let c = b.pixels[(y * b.w + x) as usize];
                    if c != 0 {
                        a.ellipse(x + ox, y + oy, 2, 2, BLUE + 3);
                    }
                }
            }
            for y in 0..b.h {
                for x in 0..b.w {
                    let c = b.pixels[(y * b.w + x) as usize];
                    if c != 0 { a.dot(x + ox, y + oy, c); }
                }
            }
            a
        }
        0x56..=0x5A => {
            let mut a = Art::new(13, 16);
            shield(&mut a, 6, 8, frame - 0x56);
            a
        }
        0x5B | 0x5C => {
            let mut a = Art::new(29, 123);
            finish(&mut a, WOOD, 3, frame as u32);
            border(&mut a, 0, 0, 29, 123, WOOD, GOLD + 4);
            for (i, icon) in [Token::Hammer, Token::Wheat, Token::Joy, Token::Zoom].into_iter().enumerate() {
                let y = i as i32 * 32 + 2;
                if frame == 0x5B && i < 3 {
                    let c = [RED, COPPER, GOLD, MOSS, GRASS, PINE];
                    for j in 0..2 {
                        a.rect(6, y + j * 15, 17, 13, c[i * 2 + j as usize] + 5);
                        border(&mut a, 6, y + j * 15, 17, 13, WOOD + 1, GOLD + 5);
                    }
                } else {
                    a.rect(2, y, 25, 23, PARCHMENT + 5);
                    border(&mut a, 2, y, 25, 23, WOOD + 1, GOLD + 6);
                    token(&mut a, 14, y + 11, icon);
                }
            }
            a
        }
        0x5D..=0x5F => {
            let mut a = Art::new(22, 22);
            a.ellipse(11, 11, 10, 10, WOOD + 1);
            a.ellipse(11, 11, 9, 9, PARCHMENT + 5);
            token(&mut a, 11, 11, [Token::Hammer, Token::Joy, Token::Wheat][frame - 0x5D]);
            a
        }
        0x60 => {
            let mut a = Art::new(74, 30);
            finish(&mut a, PARCHMENT, 5, frame as u32);
            border(&mut a, 0, 0, 74, 30, WOOD + 1, GOLD + 6);
            token(&mut a, 12, 15, Token::Zoom);
            a
        }
        0x3D => {
            let mut a = Art::new(9, 33);
            a.poly(&[(4, 0), (8, 5), (8, 27), (4, 32), (0, 27), (0, 5)], WOOD + 1);
            a.poly(&[(4, 2), (7, 6), (7, 26), (4, 30), (1, 26), (1, 6)], GOLD + 5);
            a.line(3, 6, 3, 26, GOLD + 7);
            a.line(5, 7, 5, 26, WOOD + 4);
            a
        }
        0x3E => {
            let mut a = Art::new(60, 77);
            a.ellipse(29, 59, 25, 10, PARCHMENT + 3);
            for (x, y) in [(15, 48), (25, 55), (41, 47), (45, 61)] {
                a.ellipse(x, y, 9, 5, WOOD + 2);
                a.ellipse(x, y - 2, 8, 4, GOLD + 5);
                a.ellipse(x, y - 2, 4, 2, GOLD + 7);
            }
            a.poly(&[(11, 43), (16, 25), (29, 17), (41, 26), (43, 43), (30, 49)], WOOD + 2);
            a.poly(&[(13, 42), (19, 26), (29, 21), (37, 28), (40, 42), (29, 47)], PARCHMENT + 4);
            a.line(22, 22, 38, 24, GOLD + 4);
            token(&mut a, 29, 35, Token::Crown);
            a
        }
        _ => {
            let (w, h, glyph) = match frame {
                0x17 => (20, 18, Token::Joy),
                0x18 => {
                    let mut a = Art::new(8, 18);
                    a.ellipse(4, 3, 2, 3, WOOD + 4);
                    a.rect(2, 6, 4, 7, SLATE + 4);
                    a.line(3, 11, 2, 17, WOOD + 2);
                    a.line(5, 11, 5, 17, WOOD + 2);
                    return a;
                }
                0x19 => (23, 26, Token::Stone),
                0x1A => (23, 26, Token::Wood),
                0x1B => (23, 26, Token::Cargo),
                0x20..=0x24 => (36, 27, Token::Wheat),
                0x25..=0x29 => (37, 24, Token::Cattle),
                0x2A | 0x2B => (23, 19, Token::Cattle),
                0x2C => (21, 22, Token::Iron),
                0x2D => (26, 22, Token::Stone),
                0x2E => (26, 22, Token::Wood),
                0x30..=0x35 => (26, 26, [Token::Swords, Token::Hammer, Token::Keep, Token::Swords, Token::Iron, Token::Envoys][frame - 0x30]),
                0x3F => (28, 25, Token::Hammer),
                0x40 => (23, 26, Token::Keep),
                _ => (22, 22, [Token::Cargo, Token::Iron, Token::Wheat, Token::Stone, Token::Wood, Token::Crown][frame % 6]),
            };
            let mut a = Art::new(w, h);
            token(&mut a, (w - 1) / 2, h / 2, glyph);
            if matches!(frame, 0x23 | 0x29) {
                a.ellipse(w / 2 + 9, h / 2 + 5, 4, 4, RED + 4);
                a.line(w / 2 + 9, h / 2 + 2, w / 2 + 9, h / 2 + 5, CREAM + 6);
                a.dot(w / 2 + 9, h / 2 + 7, CREAM + 6);
            }
            a
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_tiles_cover_each_pixel_of_the_staggered_lattice_once() {
        for (zoom, far) in [(NEAR, false), (FAR, true)] {
            for (bank, frame) in [(0, 0), (0, 6), (0, 22), (0, 78), (2, 14), (2, 127)] {
                let a = tile(bank, frame, 0, far);
                assert_eq!((a.w, a.h), (zoom.tile_w, zoom.tile_h));
                let filled = |x, y| x < a.w && a.pixels[(y * a.w + x) as usize] != 0;
                for y in 0..zoom.tile_h {
                    let other_y = (y + zoom.row_step) % zoom.tile_h;
                    for x in 0..zoom.pitch {
                        let other_x = (x + zoom.half_pitch) % zoom.pitch;
                        assert_ne!(filled(x, y), filled(other_x, other_y), "zoom {} frame {frame} at ({x},{y})", zoom.id);
                    }
                }
            }
        }
    }
}
