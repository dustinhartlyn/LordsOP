//! Indexed illustrations for the built-in front end and management rooms.
use super::*;

const LIVERY: [u8; 6] = [RED, GOLD, SLATE, VIOLET, BLUE, MOSS];

pub(super) fn asset(name: &str) -> Option<Vec<u8>> {
    let raster = match name {
        "gateway.pl8" => landscape(640, 480, 0),
        "custom.pl8" => setup_board(),
        "skirmish.pl8" => muster(false),
        "skircust.pl8" => muster(true),
        "merchant.pl8" => market(),
        "armoury.pl8" => armoury(),
        "cas_back.pl8" => castle_catalogue(),
        "score1.pl8" => scorecard(),
        "grtnoble.pl8" => standards(),
        "smithy.pl8" => smithy(),
        _ => return sheet_asset(name),
    };
    Some(sheet([raster]))
}

fn grain(a: &mut Art, x: i32, y: i32, w: i32, h: i32, base: u8, seed: u32) {
    for j in y.max(0)..(y + h).min(a.h) {
        for i in x.max(0)..(x + w).min(a.w) {
            let n = noise(i, j * 11, seed);
            let fleck = match n % 23 { 0 => 2, 1..=5 => 1, _ => 0 };
            a.dot(i, j, base + fleck);
        }
    }
}

fn trim(a: &mut Art, x: i32, y: i32, w: i32, h: i32, edge: u8, face: u8, seed: u32) {
    a.rect(x, y, w, h, INK);
    a.rect(x + 1, y + 1, w - 2, h - 2, edge + 4);
    a.line(x + 2, y + 2, x + w - 3, y + 2, edge + 6);
    a.line(x + 2, y + h - 3, x + w - 3, y + h - 3, edge + 1);
    grain(a, x + 4, y + 4, w - 8, h - 8, face, seed);
}

fn diamond(a: &mut Art, x: i32, y: i32, r: i32, c: u8) {
    a.poly(&[(x, y - r), (x + r, y), (x, y + r), (x - r, y)], c);
}

fn stonework(a: &mut Art, x: i32, y: i32, w: i32, h: i32, r: i32, light: u8) {
    grain(a, x, y, w, h, light, 54);
    for j in (0..h).step_by(r as usize) {
        a.line(x, y + j, x + w - 1, y + j, light.saturating_sub(2));
        for k in (0..w).step_by((r * 2) as usize) {
            let i = k + if j / r % 2 == 1 { r } else { 0 };
            if i < w {
                a.line(x + i, y + j, x + i, y + (j + r - 1).min(h - 1), light.saturating_sub(1));
            }
        }
    }
}

fn fir(a: &mut Art, x: i32, y: i32, h: i32, c: u8) {
    a.line(x, y - h / 2, x, y + h / 9, WOOD + 1);
    for (lift, breadth) in [(0, 4), (h / 5, 5), (h * 2 / 5, 7)] {
        a.poly(&[(x, y - h + lift), (x - h / breadth, y - h / 3 + lift / 2), (x + h / breadth, y - h / 3 + lift / 2)], c);
        a.line(x, y - h + lift + 2, x - h / (breadth + 2), y - h / 3 + lift / 2 - 1, c + 1);
    }
}

fn tower(a: &mut Art, x: i32, y: i32, w: i32, h: i32, c: u8, banner: usize) {
    a.rect(x, y, w, h, INK);
    stonework(a, x + 1, y + 1, w - 2, h - 2, (w / 8).max(3), c);
    a.rect(x + w * 3 / 4, y + 4, w / 4 - 1, h - 5, c.saturating_sub(2));
    let tooth = (w / 7).max(2);
    for i in [0, (w - tooth) / 2, w - tooth] {
        a.rect(x + i, y - tooth + 1, tooth, tooth + 2, c + 1);
        a.line(x + i, y - tooth + 1, x + i + tooth - 1, y - tooth + 1, c + 2);
    }
    a.rect(x + w / 2 - 1, y + h / 4, (w / 8).max(2), (h / 6).max(4), INK + 1);
    if w > 13 {
        let mid = x + w / 2;
        a.line(mid, y - w / 2, mid, y - 3, GOLD + 3);
        a.poly(&[(mid + 1, y - w / 2), (mid + w / 2, y - w / 2 + 2), (mid + 1, y - w / 4)], LIVERY[banner % 6] + 4);
    }
}

fn keep(a: &mut Art, x: i32, y: i32, w: i32, h: i32, kind: usize) {
    let c = if matches!(kind, 0 | 1) { WOOD + 4 } else { STONE + 4 };
    let t = (w / 5).max(8);
    a.ellipse(x + w / 2, y + h, w * 3 / 5, h / 8, MOSS + 1);
    if kind == 0 {
        for i in (x..x + w).step_by((w / 15).max(3) as usize) {
            a.line(i, y + h / 2, i, y + h, WOOD + 5);
            a.line(i + 1, y + h / 2 + 1, i + 1, y + h, WOOD + 1);
        }
    } else {
        a.rect(x, y + h / 2, w, h / 2, INK);
        stonework(a, x + 1, y + h / 2 + 2, w - 2, h / 2 - 2, (w / 18).max(3), c);
        for i in (x..x + w - 4).step_by((w / 14).max(4) as usize) {
            a.rect(i, y + h / 2 - 4, (w / 22).max(2), 5, c + 1);
        }
    }
    tower(a, x + (w - t) / 2, y + h / 8, t, h * 5 / 8, c, kind);
    if kind > 0 {
        let th = if kind >= 3 { h * 3 / 4 } else { h / 2 + 4 };
        tower(a, x - 1, y + h - th, t, th, c, kind + 1);
        tower(a, x + w - t + 1, y + h - th, t, th, c, kind + 2);
    }
    if kind >= 4 {
        tower(a, x + w / 3 - t / 2, y + h / 3, t, h * 2 / 3, c + 1, kind);
        tower(a, x + w * 2 / 3 - t / 2, y + h / 3, t, h * 2 / 3, c, kind);
    }
    let gx = x + w / 2;
    let gy = y + h - h / 6;
    a.ellipse(gx, gy, (w / 13).max(2), (h / 14).max(2), INK);
    a.rect(gx - w / 13, gy, (w / 13 * 2 + 1).max(5), h / 6, INK);
    for q in -2..=2 { a.line(gx + q * (w / 80).max(1), gy, gx + q * (w / 80).max(1), y + h - 1, WOOD + 3); }
}

fn landscape(w: i32, h: i32, variant: usize) -> Art {
    let mut a = Art::new(w, h);
    let anchor = w * 3 / 4;
    let moon_y = h * 7 / 20;
    let ground = h * 4 / 5;
    for y in 0..h {
        for x in 0..w {
            let haze = ((w - (x - anchor).abs()) * 3 / w + y * 4 / h).min(5) as u8;
            let dither = u8::from(noise(x, y, 42) % 9 < 2);
            a.dot(x, y, if variant == 0 && h > 250 { SLATE + haze + dither } else { BLUE + haze + dither });
        }
    }
    a.ellipse(anchor, moon_y, h / 8, h / 8, COPPER + 2);
    a.ellipse(anchor, moon_y, h / 9, h / 9, GOLD + 6);
    for i in 0..19 {
        let x = (noise(i, variant as i32, 44) % w as u32) as i32;
        let y = h / 4 + (noise(i, 8, 23) % (h / 3) as u32) as i32;
        let span = 11 + i * 4 % 49;
        a.line(x, y, x + span, y, if i % 4 == 0 { COPPER + 3 } else { SLATE + 6 });
    }
    for layer in 0..3 {
        let y = h * (4 + layer) / 9;
        let mut points = Vec::with_capacity(22);
        for i in 0..=18 {
            let bump = (noise(i, layer, 84) % (h / 6) as u32) as i32;
            points.push((w * i / 18, y + bump));
        }
        points.extend([(w, h - 1), (0, h - 1)]);
        a.poly(&points, [BLUE + 2, SLATE + 2, PINE + 1][layer as usize]);
    }
    if h > 250 {
        let size = w * 3 / 8;
        keep(&mut a, anchor - size / 2, ground - size / 2, size, size / 2, variant.max(2));
    }
    for i in 0..48 {
        let x = (noise(i, 5, 43) % w as u32) as i32;
        let y = ground + (noise(i, 8, 99) % (h - ground) as u32) as i32;
        let n = (noise(i, 6, 13) % (h / 10) as u32) as i32;
        fir(&mut a, x, y, h / 12 + n, PINE + (i % 3) as u8);
    }
    if h > 250 {
        for x in [10, w - 11] {
            a.line(x, 7, x, h - 9, GOLD + 1);
            a.line(x + 2, 9, x + 2, h - 11, INK + 2);
            for y in [13, h - 15] { diamond(&mut a, x, y, 4, GOLD + 5); }
        }
        a.line(14, h - 13, w - 14, h - 13, GOLD + 1);
    }
    a
}

fn setup_board() -> Art {
    let mut a = Art::new(640, 480);
    grain(&mut a, 0, 0, 640, 480, WOOD + 1, 38);
    for y in (0..480).step_by(67) {
        a.line(0, y, 639, y, INK + 2);
        a.line(0, y + 1, 639, y + 1, WOOD + 4);
    }
    trim(&mut a, 150, 9, 338, 225, GOLD, INK + 2, 18);
    trim(&mut a, 153, 237, 483, 238, GOLD, WOOD + 2, 33);
    trim(&mut a, 5, 2, 145, 475, COPPER, WOOD, 42);
    for y in [21, 112, 206, 300, 394] {
        a.line(8, y, 144, y, COPPER + 1);
        diamond(&mut a, 78, y, 4, GOLD + 4);
    }
    for (x, _) in crate::screens::setup::CUSTOM_BUTTONS {
        trim(&mut a, x, 194, 76, 24, GOLD, INK + 2, x as u32);
    }
    a
}

fn muster(custom: bool) -> Art {
    let mut a = landscape(640, 480, 2);
    trim(&mut a, 11, 8, 618, 66, GOLD, INK + 2, 45);
    label(&mut a, 170, 22, "THE MUSTER FIELD", 2, CREAM + 7);
    label(&mut a, 233, 49, "A COMPANY OF BANNERS", 1, GOLD + 5);
    for x in [32, 605] { jewel(&mut a, x, 40, 8, BLUE); }
    if custom {
        trim(&mut a, 13, 81, 205, 389, GOLD, SLATE + 1, 38);
        trim(&mut a, 222, 82, 211, 385, GOLD, SLATE + 1, 12);
        for y in (130..419).step_by(30) { a.line(26, y, 203, y, GOLD + 1); }
        for (row, weapon) in [6, 8, 9, 10, 11, 12, 13].into_iter().enumerate() {
            stamp(&mut a, &good(weapon, 23), 21, 103 + row as i32 * 30);
        }
        for y in [150, 373] { trim(&mut a, 280, y, 124, 26, GOLD, INK + 2, y as u32); }
        let mut swap = plate(32, 32, INK + 2, 12);
        swap.line(8, 12, 24, 12, CREAM + 5);
        swap.line(8, 20, 24, 20, CREAM + 5);
        swap.poly(&[(19, 8), (25, 12), (19, 16)], CREAM + 5);
        swap.poly(&[(13, 16), (7, 20), (13, 24)], CREAM + 5);
        stamp(&mut a, &swap, 267, 423);
    } else {
        trim(&mut a, 11, 80, 425, 389, GOLD, SLATE + 1, 39);
        for x in [11, 278] {
            trim(&mut a, x, 259, 158, 45, GOLD, INK + 2, x as u32);
            for dx in [15, 143] { jewel(&mut a, x + dx, 281, 4, RED); }
        }
        for x in [20, 275] {
            trim(&mut a, x, 350, 160, 92, GOLD, SLATE, x as u32);
            let banner = realm_flag(if x < 200 { 0 } else { 4 }, 61, 67, 4);
            stamp(&mut a, &banner, x + 51, 357);
        }
    }
    trim(&mut a, 444, 80, 194, 389, GOLD, INK + 2, 67);
    label(&mut a, 489, 117, "BATTLE LEDGER", 1, GOLD + 6);
    a.line(462, 133, 619, 133, GOLD + 3);
    trim(&mut a, 463, 174, 167, 117, GOLD, SLATE, 13);
    for (i, y) in [291, 322, 353, 384].into_iter().enumerate() {
        trim(&mut a, 463, y, 167, 30, GOLD, SLATE + 1, y as u32);
        let mut emblem = Art::new(25, 25);
        match i {
            0 => keep(&mut emblem, 3, 6, 19, 14, 3),
            1 => emblem = good(10, 25),
            2 => emblem = good(8, 25),
            _ => emblem = resized(faces(15), 23, 22),
        }
        stamp(&mut a, &emblem, 466, y + 2);
    }
    stamp(&mut a, &resized(system(35), 20, 20), 604, 181);
    stamp(&mut a, &resized(system(37), 20, 20), 604, 266);
    for (i, x) in [461, 519, 577].into_iter().enumerate() {
        trim(&mut a, x, 422, 56, 52, GOLD, WOOD + 1, x as u32);
        jewel(&mut a, x + 28, 434, 6, [RED, BLUE, GOLD][i]);
    }
    a
}

fn wood_room(w: i32, h: i32, masonry: bool) -> Art {
    let mut a = Art::new(w, h);
    let sill = h * 3 / 4;
    if masonry {
        stonework(&mut a, 0, 0, w, sill, 27, STONE + 2);
    } else {
        grain(&mut a, 0, 0, w, sill, WOOD + 1, 9);
        for x in (0..w).step_by(22) { a.line(x, 0, x, sill, WOOD); }
    }
    grain(&mut a, 0, sill, w, h - sill, WOOD + 1, 25);
    for i in -9..=9 {
        a.line(w / 2 + i * 21, sill, w / 2 + i * 59, h - 1, WOOD);
    }
    for y in [12, sill - 4] {
        a.rect(0, y, w, 10, WOOD);
        a.line(0, y + 1, w - 1, y + 1, WOOD + 6);
        a.line(0, y + 9, w - 1, y + 9, INK);
    }
    a
}

fn market() -> Art {
    use crate::screens::merchant::STALL;
    let mut a = wood_room(640, 480, false);
    for x in (0..640).step_by(40) {
        let c = if x % 80 == 0 { RED } else { PARCHMENT };
        a.poly(&[(x, 0), (x + 39, 0), (x + 35, 58), (x + 20, 68), (x + 4, 58)], c + 4);
        a.line(x + 3, 4, x + 8, 56, c + 6);
        a.line(x + 32, 48, x + 28, 59, c + 2);
    }
    for y in [161, 273, 374] { trim(&mut a, 0, y, 640, 13, WOOD, WOOD + 2, y as u32); }
    for row in STALL.iter().filter(|row| row.x != 0 || row.y != 0) {
        trim(&mut a, row.x + 18, row.y + 3, 92, 57, GOLD, WOOD + 1, row.good as u32);
        let item = trade_icon(row.good - 1, 48);
        stamp(&mut a, &item, row.x + 39, row.y + 7);
        for x in [row.x + 24, row.x + 103] { diamond(&mut a, x, row.y + 30, 2, GOLD + 5); }
    }
    trim(&mut a, 243, 18, 153, 36, GOLD, INK + 2, 8);
    label(&mut a, 269, 29, "THE MARKET", 1, CREAM + 6);
    for x in [12, 619] {
        a.rect(x, 66, 9, 386, WOOD);
        a.line(x + 1, 67, x + 1, 447, WOOD + 5);
    }
    a
}

fn arch(a: &mut Art, x: i32, y: i32, w: i32, h: i32) {
    a.ellipse(x + w / 2, y + w / 2, w / 2, w / 2, STONE + 5);
    a.rect(x, y + w / 2, w, h - w / 2, STONE + 4);
    a.ellipse(x + w / 2, y + w / 2 + 3, w / 2 - 6, w / 2 - 6, INK + 1);
    a.rect(x + 6, y + w / 2 + 3, w - 12, h - w / 2 - 3, INK + 1);
    for i in [w / 3, w * 2 / 3] { a.line(x + i, y + w / 2, x + i, y + h - 1, STONE + 2); }
    a.rect(x - 3, y + h, w + 6, 5, STONE + 5);
}

fn armoury() -> Art {
    let mut a = wood_room(640, 480, true);
    for x in [48, 206, 364, 522] { arch(&mut a, x, 49, 72, 115); }
    trim(&mut a, 242, 16, 156, 32, GOLD, INK + 2, 22);
    label(&mut a, 268, 27, "THE ARMOURY", 1, CREAM + 6);
    for y in [181, 289] {
        a.rect(17, y, 606, 13, WOOD);
        a.line(18, y + 1, 621, y + 1, WOOD + 5);
        for x in (23..619).step_by(24) { diamond(&mut a, x, y + 6, 2, GOLD + 4); }
    }
    trim(&mut a, 4, 391, 632, 88, WOOD, WOOD + 1, 21);
    for (x, y, w, h) in [(542, 396, 92, 28), (542, 425, 92, 25), (542, 450, 92, 29)] {
        trim(&mut a, x, y, w, h, GOLD, INK + 2, y as u32);
    }
    a
}

fn castle_catalogue() -> Art {
    use crate::screens::castle::{PICTURE, TYPE_BOUNDS};
    let mut a = wood_room(640, 480, true);
    trim(&mut a, PICTURE.x - 4, PICTURE.y - 4, PICTURE.w + 8, PICTURE.h + 8, GOLD, INK + 2, 39);
    let motte = castle_picture(1);
    stamp(&mut a, &motte, PICTURE.x, PICTURE.y);
    trim(&mut a, 489, 8, 150, 244, GOLD, SLATE + 1, 8);
    for (i, &(x, y, r, b)) in TYPE_BOUNDS.iter().enumerate() {
        trim(&mut a, x, y, r - x + 1, b - y, GOLD, SLATE + 2, i as u32);
        let size = (r - x - 17).min(110);
        keep(&mut a, x + (r - x - size) / 2, y + 55, size, 42 + i as i32 * 5, i);
        label(&mut a, (r + x) / 2 - 2, b - 18, ["I", "II", "III", "IV", "V"][i], 1, GOLD + 6);
    }
    a
}

fn scorecard() -> Art {
    let mut a = landscape(640, 480, 3);
    trim(&mut a, 81, 45, 463, 375, GOLD, SLATE + 1, 14);
    for y in [100, 260] {
        trim(&mut a, 99, y, 425, 139, GOLD, INK + 2, y as u32);
        for yy in [y + 74, y + 96, y + 116] { a.line(109, yy, 514, yy, STONE + 2); }
    }
    a
}

fn standards() -> Art {
    use crate::screens::nobles::{COLUMN_X, TABS};
    let mut a = wood_room(640, 480, true);
    for x in COLUMN_X {
        arch(&mut a, x - 10, 26, 68, 328);
        a.line(x + 4, 247, x + 55, 247, GOLD + 2);
    }
    for (i, tab) in TABS.iter().enumerate() {
        trim(&mut a, tab.x, tab.y, tab.w, tab.h, GOLD, SLATE + 1, i as u32);
        let icon = good([0, 5, 7, 8, 10, 13, 1][i], 24);
        stamp(&mut a, &icon, tab.x + 6, tab.y + 4);
    }
    trim(&mut a, 315, 431, 282, 42, GOLD, INK + 2, 10);
    a
}

fn smithy() -> Art {
    let mut a = wood_room(480, 400, true);
    arch(&mut a, 79, 75, 86, 127);
    a.rect(19, 201, 181, 13, STONE + 4);
    a.line(20, 201, 198, 201, STONE + 6);
    for (i, x) in [225, 301, 377].into_iter().enumerate() {
        let item = good(8 + i, 62);
        stamp(&mut a, &item, x, 49);
        a.rect(x - 4, 108, 68, 5, WOOD + 2);
    }
    a.poly(&[(211, 235), (389, 235), (440, 274), (185, 274)], WOOD + 4);
    a.rect(185, 275, 255, 18, WOOD + 2);
    a.rect(193, 292, 15, 54, WOOD + 1);
    a.rect(413, 292, 16, 54, WOOD + 1);
    a.poly(&[(257, 211), (351, 211), (332, 227), (315, 228), (314, 249), (333, 251), (333, 257), (276, 257), (275, 251), (294, 248), (294, 231), (272, 231)], IRON + 3);
    a.line(260, 212, 348, 212, IRON + 7);
    trim(&mut a, 8, 349, 462, 48, GOLD, WOOD, 22);
    a
}

fn stamp(dst: &mut Art, src: &Art, x: i32, y: i32) {
    for (i, &color) in src.pixels.iter().enumerate().filter(|&(_, c)| *c != 0) {
        dst.dot(x + i as i32 % src.w, y + i as i32 / src.w, color);
    }
}

fn resized(src: Art, w: i32, h: i32) -> Art {
    if (src.w, src.h) == (w, h) { return src; }
    let mut out = Art::new(w, h);
    for y in 0..h {
        for x in 0..w {
            out.dot(x, y, src.pixels[((y * src.h / h) * src.w + x * src.w / w) as usize]);
        }
    }
    out
}

fn stroke(a: &mut Art, x0: i32, y0: i32, x1: i32, y1: i32, c: u8) {
    a.line(x0 - 1, y0, x1 - 1, y1, INK);
    a.line(x0 + 1, y0, x1 + 1, y1, INK);
    a.line(x0, y0 - 1, x1, y1 - 1, INK);
    a.line(x0, y0 + 1, x1, y1 + 1, INK);
    a.line(x0, y0, x1, y1, c);
}

fn jewel(a: &mut Art, x: i32, y: i32, r: i32, c: u8) {
    diamond(a, x, y, r + 1, INK);
    diamond(a, x, y, r, c + 3);
    a.poly(&[(x, y - r), (x + r, y), (x, y + 1), (x - r, y)], c + 6);
    a.dot(x - 1, y - 1, CREAM + 7);
}

fn crest(i: usize, w: i32, h: i32, bright: bool) -> Art {
    let c = LIVERY[i % 6];
    let mut a = Art::new(48, 52);
    a.poly(&[(1, 4), (24, 1), (46, 4), (44, 31), (36, 43), (24, 51), (11, 43), (3, 31)], INK);
    a.poly(&[(3, 6), (24, 3), (44, 6), (42, 30), (34, 42), (24, 48), (13, 41), (5, 30)], GOLD + if bright { 7 } else { 4 });
    a.poly(&[(6, 8), (24, 6), (41, 8), (39, 29), (32, 39), (24, 44), (15, 39), (8, 29)], c + 3);
    a.poly(&[(24, 6), (41, 8), (39, 29), (32, 39), (24, 44)], c + 1);
    for y in 8..42 {
        for x in 8..41 {
            if a.pixels[(y * 48 + x) as usize] == c + 3 && noise(x, y, 50) % 5 == 0 { a.dot(x, y, c + 4); }
        }
    }
    let metal = CREAM + 4;
    match i % 6 {
        0 => {
            for (x, y) in [(15, 20), (33, 20), (24, 32)] {
                a.poly(&[(x - 5, y - 5), (x, y - 2), (x + 5, y - 5), (x + 3, y + 3), (x - 3, y + 3)], metal);
                a.line(x - 3, y + 2, x + 3, y + 2, GOLD + 3);
            }
        }
        1 => {
            a.line(24, 12, 24, 37, metal);
            for y in [16, 22, 28] {
                a.poly(&[(24, y + 3), (13, y - 4), (16, y + 2), (24, y + 7)], metal);
                a.poly(&[(24, y + 3), (35, y - 4), (32, y + 2), (24, y + 7)], metal);
            }
        }
        2 => {
            a.line(15, 33, 33, 14, metal);
            a.line(15, 14, 33, 33, metal);
            a.poly(&[(16, 17), (25, 10), (31, 17), (24, 28)], metal);
            a.poly(&[(17, 32), (23, 28), (29, 30), (32, 36), (24, 38)], metal);
            a.dot(25, 17, INK);
        }
        3 => {
            a.poly(&[(24, 10), (38, 24), (24, 38), (10, 24)], metal);
            diamond(&mut a, 24, 24, 8, c + 3);
            jewel(&mut a, 24, 24, 5, c);
        }
        4 => {
            a.poly(&[(24, 14), (37, 11), (33, 25), (27, 28), (24, 37), (21, 28), (15, 25), (11, 11)], metal);
            a.line(14, 16, 22, 23, c + 2);
            a.line(34, 16, 26, 23, c + 2);
            a.dot(25, 17, INK);
        }
        _ => {
            a.ellipse(24, 23, 9, 9, metal);
            a.ellipse(27, 20, 7, 7, c + 1);
            diamond(&mut a, 32, 27, 3, GOLD + 6);
        }
    }
    for (x, y) in [(6, 8), (41, 8), (24, 46)] { a.dot(x, y, GOLD + 7); }
    resized(a, w, h)
}

fn good(i: usize, size: i32) -> Art {
    let mut a = Art::new(48, 48);
    let metal = IRON + 6;
    a.ellipse(24, 41, 19, 4, INK + 1);
    match i {
        0 => {
            for x in [13, 20, 26, 33, 38] {
                stroke(&mut a, 24, 40, x, 8 + x % 5, GOLD + 5);
                for y in [12, 17, 22] {
                    a.ellipse(x - 2, y, 3, 2, GOLD + 3);
                    a.ellipse(x + 2, y + 2, 3, 2, PARCHMENT + 6);
                }
            }
            a.line(18, 32, 30, 30, RED + 6);
            a.line(18, 33, 30, 31, RED + 4);
        }
        1 | 2 => {
            let wool = if i == 1 { WOOD + 5 } else { CREAM + 4 };
            for x in [12, 18, 31, 36] {
                stroke(&mut a, x, 27, x - 1, 39, WOOD + 2);
            }
            a.ellipse(23, 24, 19, 11, INK);
            a.ellipse(23, 23, 17, 9, wool);
            for (x, y) in [(15, 20), (27, 22), (19, 27), (31, 28)] {
                a.ellipse(x, y, 4, 3, if i == 1 { WOOD + 1 } else { CREAM + 6 });
            }
            a.ellipse(39, 24, 7, 7, WOOD + 2);
            a.ellipse(43, 27, 5, 3, PARCHMENT + 3);
            if i == 1 {
                a.line(38, 18, 34, 13, CREAM + 6);
                a.line(42, 18, 45, 13, CREAM + 6);
            }
            a.dot(41, 22, CREAM + 7);
        }
        3 => {
            a.ellipse(34, 27, 10, 11, INK);
            a.ellipse(34, 27, 7, 8, GOLD + 3);
            a.ellipse(34, 27, 4, 5, 0);
            a.poly(&[(7, 10), (34, 10), (33, 35), (28, 41), (13, 41), (8, 36)], INK);
            a.poly(&[(9, 13), (32, 13), (30, 36), (26, 39), (14, 39), (10, 35)], COPPER + 3);
            a.line(14, 14, 15, 36, GOLD + 6);
            a.line(23, 14, 22, 37, COPPER + 1);
            for (x, y) in [(10, 11), (16, 8), (23, 10), (30, 9)] {
                a.ellipse(x, y, 5, 4, CREAM + 6);
                a.dot(x - 1, y - 1, CREAM + 7);
            }
        }
        4 => {
            for (x, y, r) in [(13, 23, 9), (22, 16, 11), (32, 20, 12), (30, 32, 11), (17, 33, 10)] {
                a.ellipse(x, y, r + 1, r + 1, INK);
                a.ellipse(x, y, r, r, CREAM + 3);
                a.ellipse(x - 2, y - 3, r * 3 / 5, r / 3, CREAM + 5);
                a.line(x, y + r / 3, x + 4, y, PARCHMENT + 2);
            }
        }
        5 => {
            for (x, y) in [(2, 31), (18, 30), (8, 19), (24, 17), (15, 8)] {
                a.poly(&[(x, y + 3), (x + 5, y), (x + 16, y), (x + 20, y + 6), (x + 15, y + 11), (x + 1, y + 11)], INK);
                a.poly(&[(x + 1, y + 4), (x + 6, y + 1), (x + 15, y + 1), (x + 18, y + 6), (x + 14, y + 8), (x + 2, y + 8)], STONE + 4);
                a.line(x + 6, y + 2, x + 14, y + 2, STONE + 7);
            }
        }
        6 => {
            for (x, y) in [(11, 31), (26, 34), (19, 23)] {
                a.poly(&[(x - 7, y - 4), (x + 15, y - 18), (x + 22, y - 11), (x + 6, y + 4)], WOOD + 2);
                a.line(x, y - 4, x + 18, y - 15, WOOD + 6);
                a.ellipse(x, y, 8, 7, WOOD);
                a.ellipse(x, y, 6, 5, PARCHMENT + 5);
                a.ellipse(x, y, 4, 3, WOOD + 4);
                a.ellipse(x, y, 2, 1, PARCHMENT + 4);
            }
        }
        7 => {
            for (x, y) in [(4, 30), (22, 31), (14, 18)] {
                a.poly(&[(x, y + 4), (x + 5, y), (x + 20, y), (x + 24, y + 5), (x + 20, y + 10), (x + 3, y + 10)], INK);
                a.poly(&[(x + 1, y + 4), (x + 6, y + 1), (x + 19, y + 1), (x + 22, y + 5), (x + 19, y + 7), (x + 3, y + 7)], IRON + 4);
                a.line(x + 7, y + 2, x + 18, y + 2, IRON + 7);
            }
        }
        8 => {
            stroke(&mut a, 20, 40, 24, 9, WOOD + 6);
            a.poly(&[(4, 24), (9, 12), (38, 12), (44, 24), (39, 21), (24, 17), (9, 21)], IRON + 4);
            a.line(4, 24, 24, 15, CREAM + 4);
            a.line(24, 15, 44, 24, CREAM + 4);
            a.line(4, 23, 40, 23, WOOD + 4);
            stroke(&mut a, 24, 25, 26, 5, metal);
        }
        9 => {
            stroke(&mut a, 12, 40, 34, 13, WOOD + 5);
            a.ellipse(34, 12, 9, 9, INK);
            a.ellipse(34, 12, 7, 7, IRON + 3);
            for (x, y) in [(34, 4), (40, 8), (38, 14), (31, 16), (29, 9)] {
                diamond(&mut a, x, y, 3, metal);
            }
            a.line(8, 36, 15, 42, GOLD + 4);
        }
        10 => {
            a.poly(&[(13, 30), (32, 8), (43, 4), (40, 15), (20, 36)], INK);
            a.poly(&[(15, 30), (33, 9), (41, 6), (38, 15), (20, 34)], IRON + 4);
            a.line(17, 31, 40, 7, metal);
            stroke(&mut a, 17, 32, 8, 42, WOOD + 5);
            stroke(&mut a, 10, 27, 24, 40, GOLD + 5);
            jewel(&mut a, 8, 42, 3, GOLD);
        }
        11 => {
            stroke(&mut a, 10, 41, 37, 9, WOOD + 5);
            a.poly(&[(40, 3), (38, 15), (32, 18), (31, 13)], metal);
            a.line(34, 15, 40, 3, CREAM + 7);
            a.line(15, 33, 19, 37, GOLD + 3);
        }
        12 => {
            a.poly(&[(12, 2), (22, 6), (32, 13), (40, 23), (41, 34), (37, 45), (34, 44), (37, 33), (34, 24), (27, 15), (18, 8)], WOOD + 6);
            a.line(13, 4, 36, 43, CREAM + 4);
            stroke(&mut a, 5, 36, 32, 18, IRON + 5);
            a.poly(&[(31, 15), (38, 14), (35, 20), (29, 21)], metal);
            a.line(7, 39, 11, 33, RED + 4);
        }
        _ => {
            a.poly(&[(12, 7), (20, 5), (24, 10), (28, 5), (36, 7), (46, 19), (40, 27), (35, 24), (36, 40), (11, 40), (12, 24), (5, 27), (1, 19)], INK);
            a.poly(&[(12, 9), (19, 7), (24, 13), (29, 7), (35, 9), (44, 19), (39, 24), (33, 20), (34, 38), (13, 38), (14, 20), (6, 24), (3, 19)], IRON + 3);
            for y in (11..38).step_by(3) {
                for x in (5..44).step_by(3) {
                    let i = x + y % 2;
                    if a.pixels[(y * 48 + i) as usize] == IRON + 3 { a.dot(i, y, metal); }
                }
            }
            a.line(11, 32, 34, 32, WOOD + 7);
            jewel(&mut a, 24, 32, 3, GOLD);
        }
    }
    resized(a, size, size)
}

fn portrait(i: usize, w: i32, h: i32) -> Art {
    let mut a = Art::new(56, 65);
    let livery = LIVERY[i % 6];
    let skin = if i % 3 == 1 { COPPER + 4 } else { PARCHMENT + 4 };
    let hair = [WOOD, INK + 3, COPPER + 1, STONE + 5, PARCHMENT + 3][i % 5];
    a.ellipse(28, 33, 27, 31, INK);
    a.ellipse(28, 32, 25, 29, GOLD + 4);
    a.ellipse(28, 32, 23, 27, livery + 1);
    a.poly(&[(8, 49), (17, 42), (23, 39), (33, 39), (47, 46), (52, 55), (43, 61), (13, 61)], livery + 4);
    a.poly(&[(17, 43), (26, 40), (32, 41), (40, 47), (29, 61)], livery + 6);
    a.ellipse(27, 27, 15, 18, hair);
    a.poly(&[(17, 20), (27, 14), (35, 16), (39, 24), (37, 34), (32, 43), (24, 43), (16, 35)], skin);
    a.poly(&[(30, 16), (37, 20), (39, 26), (35, 39), (32, 43), (28, 42), (32, 34)], COPPER + 3);
    a.line(18, 25, 25, 23, hair);
    a.line(31, 23, 36, 25, hair);
    a.line(20, 27, 23, 27, INK);
    a.line(32, 27, 35, 27, INK);
    a.line(28, 25, 26, 32, skin + 1);
    a.line(26, 32, 31, 33, COPPER + 2);
    a.line(23, 37, 32, 37, RED + 2);
    if i % 4 != 1 {
        a.poly(&[(16, 30), (22, 36), (27, 34), (33, 36), (38, 29), (36, 41), (28, 48), (21, 42)], hair);
        a.line(25, 38, 32, 38, skin);
    }
    match i % 4 {
        0 => {
            a.poly(&[(12, 10), (19, 14), (23, 6), (28, 13), (35, 6), (38, 13), (44, 9), (40, 21), (16, 21)], GOLD + 5);
            jewel(&mut a, 28, 17, 3, livery);
        }
        1 => {
            a.poly(&[(12, 29), (11, 17), (15, 9), (27, 5), (39, 10), (44, 19), (41, 30), (34, 16), (24, 15), (18, 20)], livery + 5);
            a.line(15, 15, 30, 7, livery + 7);
        }
        2 => {
            a.ellipse(28, 16, 15, 9, IRON + 4);
            a.line(17, 13, 34, 9, IRON + 7);
            a.rect(13, 18, 30, 4, IRON + 2);
        }
        _ => {
            a.poly(&[(13, 12), (20, 8), (36, 7), (44, 14), (39, 18), (17, 21)], livery + 5);
            a.line(16, 18, 40, 15, GOLD + 4);
        }
    }
    jewel(&mut a, 28, 55, 4, GOLD);
    resized(a, w, h)
}

fn plate(w: i32, h: i32, face: u8, seed: u32) -> Art {
    let mut a = Art::new(w, h);
    trim(&mut a, 0, 0, w, h, GOLD, face, seed);
    a
}

fn gateway_tile(i: usize) -> Art {
    use l2_view::chrome::panels::*;
    let mut a = Art::new(CELL, CELL);
    grain(&mut a, 0, 0, CELL, CELL, INK + 2, i as u32);
    let left = i == CORNER_TL || i == CORNER_BL || (EDGE_LEFT..EDGE_RIGHT).contains(&i);
    let right = i == CORNER_TR || i == CORNER_BR || (EDGE_RIGHT..TEXTURE).contains(&i);
    let top = i == CORNER_TL || i == CORNER_TR || (EDGE_TOP..EDGE_BOTTOM).contains(&i);
    let bottom = i == CORNER_BR || i == CORNER_BL || (EDGE_BOTTOM..EDGE_LEFT).contains(&i);
    for x in [left.then_some(2), right.then_some(CELL - 3)].into_iter().flatten() {
        a.line(x - 1, 0, x - 1, 15, WOOD + 2);
        a.line(x, 0, x, 15, GOLD + 4);
        a.line(x + 1, 0, x + 1, 15, GOLD + 1);
        if i % 3 == 1 { diamond(&mut a, x + if left { 4 } else { -4 }, 8, 3, GOLD + 3); }
    }
    for y in [top.then_some(2), bottom.then_some(CELL - 3)].into_iter().flatten() {
        a.line(0, y - 1, 15, y - 1, WOOD + 3);
        a.line(0, y, 15, y, GOLD + 5);
        a.line(0, y + 1, 15, y + 1, GOLD + 1);
        if i % 3 == 2 { diamond(&mut a, 8, y + if top { 4 } else { -4 }, 3, GOLD + 3); }
    }
    if i < 4 { jewel(&mut a, 8, 8, 5, RED); }
    a
}

fn gateway_panel(i: usize) -> Art {
    match i {
        0..=195 => gateway_tile(i),
        196..=203 => plate(24, 16, SLATE + 1, i as u32),
        204 => plate(224, 32, SLATE + 1, 24),
        205..=214 => crest((i - 205) / 2, 60, 65, i % 2 == 0),
        _ => {
            let mut a = plate(54, 27, WOOD + 1, 0);
            jewel(&mut a, 27, 13, 5, BLUE);
            for (x, dx) in [(17, -1), (37, 1)] {
                a.line(x, 13, x + dx * 7, 13, GOLD + 5);
            }
            a
        }
    }
}

fn livery(i: usize) -> usize {
    i.saturating_sub(1).min(5)
}

fn realm_flag(i: usize, w: i32, h: i32, phase: usize) -> Art {
    let mut a = Art::new(58, 68);
    let c = LIVERY[i % 6];
    a.line(3, 5, 3, 65, WOOD + 3);
    a.line(4, 5, 4, 65, GOLD + 5);
    jewel(&mut a, 4, 5, 3, GOLD);
    let sway = (phase.min(12) as i32 - 6).abs() - 3;
    a.poly(&[(5, 10), (21, 8), (45, 11 + sway), (57, 7 + sway), (50, 24 + sway), (56, 40 + sway), (42, 42 + sway), (24, 37), (5, 41)], INK);
    a.poly(&[(6, 12), (21, 10), (44, 13 + sway), (54, 10 + sway), (48, 24 + sway), (53, 39 + sway), (42, 40 + sway), (24, 35), (6, 39)], c + 4);
    a.poly(&[(34, 12), (45, 14 + sway), (52, 11 + sway), (48, 24 + sway), (51, 38 + sway), (40, 39 + sway), (36, 34)], c + 2);
    a.line(8, 13, 21, 11, c + 7);
    a.line(8, 38, 24, 34, GOLD + 4);
    let badge = crest(i, 23, 25, true);
    stamp(&mut a, &badge, 13, 12);
    resized(a, w, h)
}

fn setup_cards(i: usize) -> Art {
    match i {
        0..=9 => {
            let mut a = plate(72, 68, LIVERY[i / 2] + 1, i as u32);
            stamp(&mut a, &crest(i / 2, 54, 57, i % 2 == 1), 9, 5);
            a
        }
        10..=14 => portrait(i - 10, 65, 64),
        15 => {
            let mut a = plate(320, 36, INK + 2, 87);
            label(&mut a, 80, 12, "CHARTER OF CONQUEST", 1, GOLD + 7);
            for x in [38, 282] { jewel(&mut a, x, 17, 5, RED); }
            a
        }
        _ => {
            let mut a = plate(134, 225, INK + 2, 20);
            trim(&mut a, 0, 0, 132, 132, GOLD, SLATE + 1, 42);
            stamp(&mut a, &system(35), 105, 131);
            stamp(&mut a, &system(37), 105, 199);
            a
        }
    }
}

fn skirmish_icons(i: usize) -> Art {
    match i {
        33..=44 => realm_flag(4, 88, 66, i - 33),
        14..=18 => crest(livery(i - 13), 36, 44, true),
        19..=23 => crest(livery(i - 18), 23, 28, true),
        1..=5 => crest(i - 1, 48, 53, true),
        6..=10 => crest(i - 6, 34, 39, true),
        _ => {
            let mut a = plate(38, 32, SLATE + 1, i as u32);
            stamp(&mut a, &good(8 + i % 6, 24), 7, 4);
            a
        }
    }
}

fn system(i: usize) -> Art {
    let (w, h) = match i {
        1..=8 => (16, 16),
        29..=32 | 57..=73 => (32, 32),
        76 => (10, 24),
        _ => (24, 24),
    };
    let pressed = i > 0 && i % 2 == 0 && !matches!(i, 16 | 74 | 76 | 78 | 80 | 82);
    let mut a = plate(w, h, if pressed { WOOD + 1 } else { SLATE + 2 }, i as u32);
    let x = w / 2 + i32::from(pressed);
    let y = h / 2 + i32::from(pressed);
    if (74..=76).contains(&i) {
        a.rect(0, 0, w, h, 0);
        if i == 76 {
            a.rect(1, 2, 8, 20, WOOD + 1);
            for r in 3..21 { a.line(2, r, 7, r, GOLD + (r % 3) as u8 + 2); }
            diamond(&mut a, 5, 11, 3, GOLD + 7);
        } else {
            let off = if i == 74 { 3 } else { 0 };
            trim(&mut a, off, 7, w - 3, 11, GOLD, INK + 2, i as u32);
        }
        return a;
    }
    let c = if matches!(i, 31 | 32) { RED + 7 } else { CREAM + 6 };
    match i {
        1..=8 | 25..=28 | 57..=62 | 65..=73 => {
            let r = if w < 20 { 3 } else { 6 };
            let minus = matches!(i, 3 | 4 | 7 | 8 | 27 | 28 | 57 | 58 | 65 | 66 | 70 | 71);
            a.line(x - r, y, x + r, y, c);
            a.line(x - r, y + 1, x + r, y + 1, c);
            if !minus {
                a.line(x, y - r, x, y + r, c);
                a.line(x + 1, y - r, x + 1, y + r, c);
            }
            if matches!(i, 5..=8 | 57..=60 | 70..=73) {
                a.line(x - r, y + r + 2, x + r, y + r + 2, GOLD + 5);
            }
        }
        16 | 29 | 30 | 51 | 52 => {
            stroke(&mut a, x - 6, y, x - 1, y + 5, MOSS + 7);
            stroke(&mut a, x - 1, y + 5, x + 7, y - 6, MOSS + 7);
        }
        31 | 32 => {
            stroke(&mut a, x - 6, y - 6, x + 6, y + 6, c);
            stroke(&mut a, x + 6, y - 6, x - 6, y + 6, c);
        }
        21..=24 | 35..=42 => {
            let dir = if matches!(i, 23 | 24 | 37 | 38 | 41 | 42) { -1 } else { 1 };
            a.poly(&[(x, y - 6 * dir), (x + 7, y + 4 * dir), (x - 7, y + 4 * dir)], c);
            a.line(x - 4, y + 2 * dir, x + 4, y + 2 * dir, GOLD + 3);
        }
        77 | 78 => {
            a.rect(x - 5, y - 6, 4, 12, c);
            a.rect(x + 2, y - 6, 4, 12, c);
        }
        _ => {
            jewel(&mut a, x, y, 6, [BLUE, RED, GOLD, VIOLET][i % 4]);
            a.dot(x - 5, y, GOLD + 7);
            a.dot(x + 5, y, GOLD + 7);
        }
    }
    a
}

fn flame(w: i32, h: i32, phase: i32, torch: bool) -> Art {
    let mut a = Art::new(w, h);
    let x = w / 2;
    let y = h - if torch { 10 } else { 6 };
    let s = if torch { 11 } else { w / 4 };
    let lean = (phase * 3 % 7) - 3;
    if torch {
        a.poly(&[(x - 9, y - 3), (x + 9, y - 3), (x + 4, y + 4), (x + 2, y + 9), (x - 3, y + 9), (x - 4, y + 4)], IRON + 4);
        a.line(x - 7, y, x + 6, y, WOOD);
    } else {
        a.ellipse(x, y + 1, w / 2 - 4, 5, RED + 2);
        a.line(x - s * 2, y + 2, x + s * 2, y + 2, COPPER + 2);
    }
    a.poly(&[(x - s, y - 2), (x - s - 1, y - 13), (x - s / 2, y - h / 2), (x + lean, y - 15), (x + lean * 2, 3 + phase % 6), (x + s / 2, y - 21), (x + s, y - 11), (x + s + 1, y - 3), (x + s / 2, y + 2), (x - s / 2, y + 2)], RED + 5);
    a.poly(&[(x - s + 5, y - 2), (x - s / 2, y - 17), (x + lean, y - 9), (x + lean * 2, 12 + phase % 7), (x + s / 2, y - 12), (x + s - 4, y - 2)], GOLD + 6);
    a.poly(&[(x - s / 3, y - 1), (x + lean, y - 12), (x + s / 3, y - 2)], CREAM + 7);
    for k in 0..9 {
        let sx = x + (noise(k, phase, 97) % (s * 3) as u32) as i32 - s * 3 / 2;
        let sy = 3 + (noise(k, phase, 17) % (h / 2) as u32) as i32;
        a.dot(sx, sy, GOLD + (k % 3) as u8 + 4);
        if k % 4 == 0 { a.dot(sx, sy + 1, RED + 6); }
    }
    a
}

fn arm_item(i: usize, team: usize) -> Art {
    if i < 6 {
        let mut a = Art::new(62, 66);
        a.rect(7, 3, 47, 8, WOOD + 2);
        a.line(8, 3, 51, 3, WOOD + 6);
        a.line(12, 7, 12, 14, GOLD + 4);
        a.line(49, 7, 49, 14, GOLD + 4);
        stamp(&mut a, &good(8 + i, 57), 2, 8);
        return a;
    }
    if i >= 15 { return good(8 + i - 15, 30); }
    let mut a = plate(72, 78, WOOD + 2, i as u32);
    a.rect(5, 5, 62, 46, LIVERY[team] + 1);
    a.line(5, 50, 66, 50, GOLD + 4);
    match i {
        6 => stamp(&mut a, &portrait(8 + team, 43, 44), 14, 5),
        13 => stamp(&mut a, &crest(team, 35, 41, true), 18, 8),
        _ => {
            let weapon = [8, 9, 11, 13, 12, 10][i - 7];
            stamp(&mut a, &good(weapon, 45), 14, 6);
        }
    }
    a.rect(5, 53, 62, 21, INK + 2);
    diamond(&mut a, 61, 62, 3, GOLD + 3);
    a
}

fn trade_icon(i: usize, size: i32) -> Art {
    good([0, 1, 2, 3, 4, 7, 5, 6, 11, 12, 9, 8, 10, 13][i], size)
}

fn arm_turn(slot: usize, phase: usize) -> Art {
    let mut a = plate(100, 100, WOOD + 1, 12);
    a.ellipse(50, 54, 39, 37, INK + 1);
    a.ellipse(50, 54, 37, 36, SLATE + 1);
    let edge_on = (phase % 12).abs_diff(6) as i32;
    let w = 28 + edge_on * 10;
    let weapon = resized(good(8 + slot, 48), w, 86);
    stamp(&mut a, &weapon, (100 - w) / 2, 7);
    if phase < 13 { a.dot(61 + edge_on * 2, 18 + edge_on, CREAM + 7); }
    a
}

fn walker(team: usize, weapon: usize, phase: usize) -> Art {
    let mut a = Art::new(48, 88);
    let step = [1, 4, 7, 5, 1, -3, -6, -4][phase];
    let bob = phase as i32 % 4 / 2;
    let c = LIVERY[team];
    a.ellipse(24, 82, 18, 4, INK + 1);
    a.poly(&[(25, 56), (33, 59), (32 + step, 73), (36 + step, 79), (35 + step, 82), (26 + step, 81), (23 + step / 2, 72)], INK + 4);
    a.poly(&[(17, 56), (24, 59), (26 - step, 73), (31 - step, 78), (31 - step, 81), (22 - step, 81), (15 - step / 2, 72)], WOOD + 4);
    a.poly(&[(17, 25 + bob), (30, 26 + bob), (36, 47), (34, 61), (15, 60), (12, 51)], c + 3);
    a.line(15, 49, 33, 51, GOLD + 6);
    a.poly(&[(19, 26), (20, 20), (29, 21), (31, 29), (25, 32)], COPPER + 5);
    a.ellipse(26, 15 + bob, 9, 11, IRON + 4);
    a.poly(&[(19, 11), (28, 9), (38, 14), (34, 19), (20, 16)], IRON + 6);
    a.line(25, 17 + bob, 33, 17 + bob, INK);
    a.line(32, 16 + bob, 32, 22 + bob, INK);
    a.poly(&[(27, 30), (33, 31), (33 + step / 2, 43), (39 + step / 2, 45), (38 + step / 2, 50), (29 + step / 2, 47)], IRON + 5);
    stamp(&mut a, &good(8 + weapon, 24), 18 + step / 2, 32);
    a
}

fn castle_picture(kind: usize) -> Art {
    let mut a = landscape(320, 200, kind);
    grain(&mut a, 0, 148, 320, 52, MOSS + 2, 7);
    a.poly(&[(134, 200), (185, 139), (204, 140), (201, 200)], PARCHMENT + 3);
    for (x, h) in [(30, 45), (61, 32), (87, 38), (260, 43), (289, 55), (310, 29)] {
        fir(&mut a, x, 182, h, PINE + 2);
    }
    keep(&mut a, 88, 50, 149, 102, kind);
    a
}

fn castle_bits(i: usize) -> Art {
    match i {
        0..=4 => {
            let mut a = plate(117, 40, INK + 2, i as u32);
            label(&mut a, 9, 9, "BUILDING", 1, CREAM + 6);
            label(&mut a, 9, 22, "CHARTER", 1, GOLD + 6);
            stamp(&mut a, &crest(i, 24, 28, true), 82, 6);
            a
        }
        5..=9 => {
            let mut a = Art::new(22, 18);
            jewel(&mut a, 10, 9, 7, BLUE);
            a
        }
        10 => good(5, 28),
        11 => good(6, 28),
        _ => {
            let mut a = Art::new(21, 21);
            jewel(&mut a, 10, 10, 6, GOLD);
            a.line(10, 4, 10, 16, CREAM + 7);
            a.line(5, 10, 15, 10, CREAM + 7);
            a
        }
    }
}

fn plan(i: usize) -> Art {
    let mut a = plate(135, 111, PARCHMENT + 2, i as u32);
    for x in (11..130).step_by(9) { a.line(x, 6, x, 103, PARCHMENT + 1); }
    for y in (6..104).step_by(9) { a.line(8, y, 128, y, PARCHMENT + 1); }
    for r in 0..=i / 2 {
        let p = 10 + r as i32 * 14;
        a.poly(&[(p, 65), (64, p), (128 - p, 45), (78, 100 - p / 3)], INK + 4);
        a.poly(&[(p + 6, 64), (64, p + 5), (121 - p, 46), (77, 94 - p / 3)], PARCHMENT + 4);
    }
    keep(&mut a, 46, 36, 44, 34, i);
    label(&mut a, 10, 92, "N", 1, INK);
    diamond(&mut a, 16, 80, 4, INK);
    a
}

fn graph(i: usize) -> Art {
    if i < 2 {
        let mut a = plate(400, 153, INK + 1, i as u32);
        for r in 0..6 {
            let y = 154 - r * 24;
            a.line(5, y - 2, 394, y - 2, SLATE + 2);
            for x in (8..398).step_by(8) { a.dot(x, y - 2, STONE + 5); }
        }
        label(&mut a, 10, 8, if i == 0 { "POPULATION" } else { "CONTENTMENT" }, 1, GOLD + 5);
        return a;
    }
    let widths = [40, 20, 12, 10, 8, 6, 5, 4, 3, 2, 1];
    let w = widths[(i - 2) / 2];
    let mut a = Art::new(w, 100);
    let c = if i % 2 == 0 { COPPER } else { GOLD };
    a.rect(0, 0, w, 100, c + 3);
    a.line(0, 0, 0, 99, c + 6);
    if w > 2 { a.line(w - 1, 0, w - 1, 99, c + 1); }
    a
}

fn medallion(i: usize) -> Art {
    if (49..=54).contains(&i) {
        return crest(i - 49, 42, 43, true);
    }
    let side = if i == 55 || i == 56 { 80 } else { 58 };
    let mut a = Art::new(side, side);
    let r = side / 2 - 1;
    a.ellipse(r, r, r, r, INK);
    a.ellipse(r, r, r - 2, r - 2, GOLD + 5);
    a.ellipse(r, r, r - 4, r - 4, WOOD + 1);
    a.ellipse(r, r, r - 6, r - 6, SLATE + 2);
    for (x, y) in [(r, 4), (r, side - 6), (4, r), (side - 6, r)] { a.dot(x, y, GOLD + 7); }
    match i {
        25 => {
            a.poly(&[(9, 41), (25, 14), (35, 25), (40, 18), (50, 41)], STONE + 4);
            a.poly(&[(18, 26), (25, 14), (34, 25), (26, 22)], ICE + 6);
        }
        23 | 24 | 26 => {
            for (x, y, h) in [(17, 44, 23), (27, 41, 31), (40, 42, 27)] { fir(&mut a, x, y, h, PINE + 3); }
            if i == 24 { a.poly(&[(12, 48), (24, 25), (29, 22), (26, 32), (23, 54)], PARCHMENT + 4); }
        }
        27 | 28 | 56 => {
            let k = if i == 27 { 0 } else { 4 };
            keep(&mut a, side / 6, side / 3 + 5, side * 2 / 3, side / 3, k);
        }
        29 => {
            a.poly(&[(11, 38), (45, 38), (40, 42), (18, 42)], WOOD + 6);
            a.line(26, 15, 26, 39, GOLD + 4);
            a.poly(&[(27, 16), (42, 35), (27, 35)], CREAM + 6);
            for y in [44, 47] { a.line(15, y, 42, y, WATER + 5); }
        }
        36 => {
            a.rect(13, 19, 31, 20, WOOD + 4);
            a.line(13, 20, 43, 20, GOLD + 3);
            a.ellipse(19, 40, 7, 6, INK);
            a.ellipse(38, 40, 7, 6, INK);
            jewel(&mut a, 19, 40, 3, GOLD);
            jewel(&mut a, 38, 40, 3, GOLD);
        }
        37 => stamp(&mut a, &portrait(9, 37, 43), 10, 7),
        38 | 39 => stamp(&mut a, &crest(if i == 38 { 0 } else { 4 }, 37, 41, true), 10, 7),
        40..=43 => {
            let (x, y) = (28, 28);
            a.poly(&[(x - 13, y - 3), (x + 1, y - 3), (x + 1, y - 11), (x + 14, y), (x + 1, y + 11), (x + 1, y + 3), (x - 13, y + 3)], GOLD + 6);
            if i > 40 { a.line(x - 13, y + 12, x + 12, y + 12, if i == 42 { RED + 6 } else { BLUE + 6 }); }
        }
        55 => {
            let sword = good(10, 65);
            for y in 0..sword.h {
                for x in 0..sword.w {
                    let c = sword.pixels[(y * sword.w + x) as usize];
                    if c != 0 { a.dot(7 + sword.w - x, 7 + y, c); }
                }
            }
            stamp(&mut a, &sword, 7, 7);
        }
        _ => {
            let good_idx = match i {
                0 | 1 | 17 => 6,
                13 => 0,
                18 | 31 => 7,
                22 => 1,
                30 => 5,
                32 => 6,
                35 => 3,
                _ => i % 14,
            };
            stamp(&mut a, &good(good_idx, 46), 6, 5);
        }
    }
    a
}

fn faces(i: usize) -> Art {
    match i {
        13..=15 => {
            let mut a = Art::new(25, 20);
            if i == 15 {
                trim(&mut a, 2, 3, 21, 15, GOLD, PARCHMENT + 3, 20);
                a.line(4, 5, 13, 13, WOOD + 3);
                a.line(13, 13, 20, 5, WOOD + 3);
            } else {
                let c = if i == 13 { MOSS } else { RED };
                jewel(&mut a, 12, 9, 7, c);
                if i == 13 {
                    a.line(8, 9, 11, 12, CREAM + 7);
                    a.line(11, 12, 17, 6, CREAM + 7);
                } else {
                    a.line(9, 6, 15, 12, CREAM + 7);
                    a.line(15, 6, 9, 12, CREAM + 7);
                }
            }
            a
        }
        29 => {
            let mut a = Art::new(48, 47);
            a.poly(&[(18, 23), (13, 44), (22, 40), (28, 45), (32, 24)], RED + 3);
            a.ellipse(24, 20, 18, 16, RED);
            a.ellipse(24, 18, 15, 13, RED + 4);
            a.ellipse(24, 18, 11, 9, RED + 2);
            jewel(&mut a, 24, 18, 5, RED);
            a
        }
        _ => portrait(i, 76, 73),
    }
}

fn village_icon(i: usize) -> Art {
    let item = [0, 0, 1, 0, 0, 5, 6, 7, 0, 7, 9, 10, 11, 12, 13, 8, 10][i];
    let mut a = plate(56, 56, SLATE + 2, i as u32);
    if i == 8 {
        arch(&mut a, 12, 9, 32, 31);
    } else {
        stamp(&mut a, &good(item, 48), 4, 4);
    }
    a
}

fn battle_column(i: usize) -> Art {
    if i >= 46 {
        let width = (i as i32 - 45).min(64);
        let mut a = Art::new(width + 2, 5);
        a.rect(0, 0, width + 2, 5, INK);
        a.rect(1, 1, width, 3, MOSS + 6);
        a.line(1, 1, width, 1, CREAM + 7);
        return a;
    }
    if i >= 13 {
        use crate::battlefield::{BANNERS_FEW, BANNERS_MANY, BANNERS_SOME};
        let layout = if i < 24 { BANNERS_FEW } else if i < 35 { BANNERS_SOME } else { BANNERS_MANY };
        let (w, h) = layout.size;
        let slot = i - layout.frame;
        let mut a = plate(w, h, SLATE + 1, 51);
        let size = if w < 30 { 17 } else { (h - 15).min(34) };
        let dy = if w < 30 { 0 } else if h < 40 { 12 } else { 14 };
        let dx = (w - size) / 2;
        if slot == 0 {
            stamp(&mut a, &portrait(8, size, size), dx, dy);
        } else {
            stamp(&mut a, &good([8, 9, 10, 11, 12, 13, 7, 6, 6, 3][slot - 1], size), dx, dy);
        }
        return a;
    }
    match i {
        0 => {
            let mut a = plate(160, 296, WOOD + 1, 87);
            for x in (8..159).step_by(21) { a.line(x, 5, x, 253, WOOD + 2); }
            for (x, y) in [(80, 92), (80, 188)] {
                a.ellipse(x, y, 59, 30, WOOD + 1);
                a.ellipse(x, y, 56, 28, WOOD + 2);
                stamp(&mut a, &crest(4, 46, 50, false), x - 23, y - 25);
            }
            a
        }
        1 => {
            let mut a = plate(160, 32, SLATE + 1, 50);
            stamp(&mut a, &battle_control(false, false), 1, 1);
            stamp(&mut a, &battle_control(true, false), 33, 1);
            label(&mut a, 78, 11, "WAR COUNCIL", 1, GOLD + 5);
            a
        }
        2 => {
            let mut a = plate(160, 36, INK + 2, 21);
            trim(&mut a, 25, 4, 52, 28, GOLD, SLATE + 1, 16);
            trim(&mut a, 102, 4, 52, 28, GOLD, SLATE + 1, 26);
            a
        }
        5 => battle_control(false, true),
        6 => battle_control(true, true),
        7..=12 => crest(livery(i - 6), 25, 32, true),
        _ => {
            let mut a = Art::new(24, 24);
            jewel(&mut a, 12, 12, 7, if i == 3 { GOLD } else { RED });
            a
        }
    }
}

fn battle_control(retreat: bool, lit: bool) -> Art {
    let mut a = plate(30, 30, if lit { COPPER + 1 } else { INK + 2 }, 91);
    if retreat {
        a.poly(&[(7, 14), (14, 7), (14, 11), (23, 11), (23, 18), (14, 18), (14, 22)], CREAM + 6);
    } else {
        a.rect(9, 8, 4, 14, CREAM + 6);
        a.rect(17, 8, 4, 14, CREAM + 6);
    }
    a
}

fn site_grid(merchant: bool) -> Art {
    let mut a = Art::new(80, 60);
    for y in 0..60 {
        for x in 0..80 {
            let (mx, my) = (x * 8 + 4, y * 8 + 4);
            let id = if merchant {
                crate::screens::merchant::STALL.iter()
                    .find(|r| (r.x != 0 || r.y != 0) && (r.x..r.x + 128).contains(&mx) && (r.y..r.y + 64).contains(&my))
                    .map_or(0, |r| r.good as u8)
            } else {
                crate::screens::armoury::RACK_HOTSPOTS.iter()
                    .find(|&&(l, t, r, b, _)| (l..r).contains(&mx) && (t..b).contains(&my))
                    .map_or(0, |&(_, _, _, _, id)| id)
            };
            a.dot(x, y, id);
        }
    }
    a
}

fn sheet_asset(name: &str) -> Option<Vec<u8>> {
    Some(match name {
        "panels2.pl8" => sheet((0..216).map(gateway_panel)),
        "misc_sel.pl8" => sheet((0..17).map(setup_cards)),
        "misc_ske.pl8" => sheet((0..45).map(skirmish_icons)),
        "system.pl8" | "system2.pl8" => sheet((0..84).map(system)),
        "misc_bat.pl8" => sheet((0..303).map(battle_column)),
        "mercgrid.pl8" => sheet([site_grid(true)]),
        "arm_grid.pl8" => sheet([site_grid(false)]),
        "icontrad.pl8" => sheet((0..25).map(|i| if i < 14 { trade_icon(i, 58) } else {
            good([0, 1, 7, 5, 6, 8, 9, 10, 11, 12, 13][i - 14], 24)
        })),
        "armtorch.pl8" => sheet((0..26).map(|i| flame(77, 49 - i / 13, i % 13, true))),
        "faces.pl8" => sheet((0..30).map(faces)),
        "caspics.pl8" => sheet([0, 2, 3, 4].into_iter().map(castle_picture)),
        "cas_bits.pl8" => sheet((0..13).map(castle_bits)),
        "flags.pl8" => sheet((0..6).map(|i| if i < 5 { realm_flag(i, 58, 92, 4) } else {
            let mut a = Art::new(23, 13);
            jewel(&mut a, 11, 6, 5, GOLD);
            a
        })),
        "sgeplans.pl8" => sheet((0..5).map(plan)),
        "icon_tmp.pl8" => sheet((0..57).map(medallion)),
        "graphs.pl8" => sheet((0..23).map(graph)),
        "hearth.pl8" => sheet((0..17).map(|i| if i < 11 { flame(69, 52, i as i32, false) } else {
            let mut a = Art::new(173, [136, 142, 119, 125, 124, 182][i - 11]);
            let h = a.h;
            trim(&mut a, 2, h / 3, 168, h - h / 3, WOOD, SLATE + 2, i as u32);
            stamp(&mut a, &good(8 + i - 11, h.min(125)), (173 - h.min(125)) / 2, h / 4 - 2);
            a
        })),
        "iconvill.pl8" => sheet((0..17).map(village_icon)),
        "fntl2_14.pl8" => typeset(Face::Body),
        "fntl2_22.pl8" => typeset(Face::Heading),
        "fntl2_9.pl8" => typeset(Face::Small),
        "font_10.pl8" => typeset(Face::Numerals),
        "fnt_8.pl8" => typeset(Face::Tiny),
        "arm_cros.pl8" | "arm_mace.pl8" | "arm_swor.pl8" | "arm_pike.pl8" | "arm_bow.pl8" | "arm_mail.pl8" => {
            let slot = ["arm_cros.pl8", "arm_mace.pl8", "arm_swor.pl8", "arm_pike.pl8", "arm_bow.pl8", "arm_mail.pl8"]
                .iter().position(|&file| file == name).unwrap();
            sheet((0..24).map(|phase| arm_turn(slot, phase)))
        }
        _ => {
            let stem = name.strip_suffix(".pl8")?;
            let (kind, team) = stem.rsplit_once('_')?;
            let team = ["r", "y", "k", "p", "b"].iter().position(|&c| c == team)?;
            if kind == "arm_it" {
                sheet((0..21).map(|i| if i == 14 { crest(team, 31, 34, false) } else { arm_item(i, team) }))
            } else {
                let weapon = ["xb", "ma", "sw", "pi", "ar", "kn"].iter().position(|&c| Some(c) == kind.strip_prefix("trp_"))?;
                sheet((0..8).map(|phase| walker(team, weapon, phase)))
            }
        }
    })
}

// Letterpress Roman constructed as seven rows of five pixels, independent of
// the stock faces. The display cut has expanded strokes and bracketed feet;
// the body cut stays narrow enough for the original control geometry.
const CAPITALS: [[u8; 7]; 26] = [
    [14,17,17,31,17,17,17], [30,17,17,30,17,17,30], [15,16,16,16,16,16,15],
    [30,17,17,17,17,17,30], [31,16,16,30,16,16,31], [31,16,16,30,16,16,16],
    [15,16,16,23,17,17,15], [17,17,17,31,17,17,17], [7,2,2,2,2,2,7],
    [7,2,2,2,2,18,12], [17,18,20,24,20,18,17], [16,16,16,16,16,16,31],
    [17,27,21,21,17,17,17], [17,25,21,19,17,17,17], [14,17,17,17,17,17,14],
    [30,17,17,30,16,16,16], [14,17,17,17,21,18,13], [30,17,17,30,20,18,17],
    [15,16,16,14,1,1,30], [31,4,4,4,4,4,4], [17,17,17,17,17,17,14],
    [17,17,17,17,10,10,4], [17,17,17,21,21,27,17], [17,17,10,4,10,17,17],
    [17,17,10,4,4,4,4], [31,1,2,4,8,16,31],
];

// Lowercase includes two descender rows; a caps line ends at row 7.
const LOWER: [[u8; 9]; 26] = [
    [0,0,0,14,1,15,17,15,0], [16,16,16,30,17,17,17,30,0], [0,0,0,15,16,16,16,15,0],
    [1,1,1,15,17,17,17,15,0], [0,0,0,14,17,31,16,15,0], [6,9,8,28,8,8,8,28,0],
    [0,0,0,15,17,17,15,1,14], [16,16,16,30,17,17,17,17,0], [4,0,0,12,4,4,4,14,0],
    [2,0,0,6,2,2,2,18,12], [16,16,16,18,20,24,20,18,0], [12,4,4,4,4,4,4,6,0],
    [0,0,0,26,21,21,21,21,0], [0,0,0,30,17,17,17,17,0], [0,0,0,14,17,17,17,14,0],
    [0,0,0,30,17,17,30,16,16], [0,0,0,15,17,17,15,1,1], [0,0,0,22,25,16,16,16,0],
    [0,0,0,15,16,14,1,30,0], [0,8,8,30,8,8,9,6,0], [0,0,0,17,17,17,17,15,0],
    [0,0,0,17,17,10,10,4,0], [0,0,0,17,17,21,21,10,0], [0,0,0,17,10,4,10,17,0],
    [0,0,0,17,17,17,15,1,14], [0,0,0,31,2,4,8,31,0],
];

const SIGNS: [[u8; 7]; 27] = [
    // 1..9 and 0
    [4,12,4,4,4,4,14], [14,17,1,6,8,16,31], [30,1,1,14,1,1,30],
    [2,6,10,18,31,2,2], [31,16,16,30,1,1,30], [7,8,16,30,17,17,14],
    [31,1,2,4,8,8,8], [14,17,17,14,17,17,14], [14,17,17,15,1,2,28],
    [14,17,19,21,25,17,14],
    // ! " % * ( ) - + = : ; ' ? / , .
    [4,4,4,4,4,0,4], [10,10,20,0,0,0,0], [25,25,2,4,8,19,19],
    [0,4,21,14,21,4,0], [2,4,8,8,8,4,2], [8,4,2,2,2,4,8],
    [0,0,0,14,0,0,0], [0,4,4,31,4,4,0], [0,0,31,0,31,0,0],
    [0,0,4,0,0,4,0], [0,0,4,0,0,4,8], [4,4,8,0,0,0,0],
    [14,17,1,6,4,0,4], [1,1,2,4,8,16,16], [0,0,0,0,0,4,8],
    [0,0,0,0,0,0,4], [0,0,4,14,4,0,0],
];

fn glyph_rows(i: usize) -> [u8; 9] {
    if i < 26 { return LOWER[i]; }
    if (79..=104).contains(&i) {
        let j = i - 79;
        let base = b"aaaaaaeeeeiiiioooouuuunnos"[j];
        let mut rows = LOWER[usize::from(base - b'a')];
        rows[0] = if j % 3 == 0 { 8 } else { 4 };
        rows[1] = if j % 3 == 0 { 4 } else { 10 };
        return rows;
    }
    let mut rows = [0; 9];
    let source = match i {
        26..=51 => CAPITALS[i - 26],
        52..=78 => SIGNS[i - 52],
        _ => SIGNS[26],
    };
    rows[1..8].copy_from_slice(&source);
    rows
}

#[derive(Clone, Copy)]
enum Face { Heading, Body, Small, Tiny, Numerals }

fn typeset(face: Face) -> Vec<u8> {
    sheet((0..106).map(|i| {
        let rows = glyph_rows(i);
        let mask = rows.iter().fold(0, |acc, &r| acc | r);
        let full = matches!(face, Face::Numerals) && (52..=61).contains(&i);
        let left = if full { 0 } else { (0..5).find(|&b| mask & (16 >> b) != 0).unwrap_or(0) };
        let right = if full { 4 } else { (0..5).rfind(|&b| mask & (16 >> b) != 0).unwrap_or(4) };
        let (sx, h) = match face {
            Face::Heading => (2, 21),
            Face::Body => (1, 13),
            Face::Small => (1, 10),
            Face::Tiny => (1, 9),
            Face::Numerals => (1, 11),
        };
        let mut a = Art::new((right - left + 1) * sx, h);
        for y in 0..h {
            let r = rows[(y * 9 / h) as usize];
            for x in left..=right {
                if r & (16 >> x) != 0 {
                    a.rect((x - left) * sx, y, sx, 1, 1);
                    if matches!(face, Face::Heading) && y > 2 && y % 3 == 0 {
                        a.dot((x - left) * sx - 1, y, 1);
                    }
                }
            }
        }
        a
    }))
}

fn label(a: &mut Art, x: i32, y: i32, text: &str, size: i32, colour: u8) {
    let mut pen = x;
    for c in text.bytes() {
        if let b'A'..=b'Z' = c {
            for (r, row) in CAPITALS[usize::from(c - b'A')].iter().enumerate() {
                for k in 0..5 {
                    if row & (16 >> k) != 0 { a.rect(pen + k * size, y + r as i32 * size, size, size, colour); }
                }
            }
        }
        pen += 6 * size;
    }
}

#[cfg(test)]
mod tests {
    use crate::shell::{ShellAssets, SHEETS};
    use l2_mods::vfs::Vfs;

    #[test]
    fn original_shell_has_every_named_sheet_and_face_without_stock_text() {
        let a = ShellAssets::load_original(&Vfs::new(), crate::original::asset);
        assert!(a.eng.is_none());
        assert!(a.has_artwork());
        assert!(a.missing_fonts().is_empty());
        assert_eq!(a.text(11, 47), "About");
        for name in SHEETS {
            let image = a.sheet(name).and_then(|s| s.frame(0)).unwrap_or_else(|| panic!("{name}"));
            assert!(image.opaque.iter().any(|&pixel| pixel), "{name}");
        }
        let label = a.heading.as_ref().unwrap().width(a.text(11, 0));
        assert!(label > 60 && label < 384);
    }
}
