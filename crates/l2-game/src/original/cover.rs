//! The gateway's illustrated panorama: first light on a fortified river crossing.
use super::*;

const SKY: u8 = 144;
const DISTANCE: u8 = 184;

pub(super) fn palette(bytes: &mut [u8]) {
    for (start, count, high, low) in [
        (SKY, 40, [9, 25, 48], [235, 174, 124]),
        (DISTANCE, 12, [65, 94, 114], [143, 154, 152]),
    ] {
        for shade in 0..count {
            for c in 0..3 {
                let rgb = (high[c] * (count - 1 - shade) + low[c] * shade) / (count - 1);
                bytes[(start as usize + shade) * 3 + c] = (rgb / 4) as u8;
            }
        }
    }
}

pub(super) fn gateway() -> Art {
    let mut a = Art::new(640, 480);
    for y in 0..356 {
        let gradient = ((y + 12) * (y + 12) * 40 / (356 * 356)).min(39) as u8;
        a.rect(0, y, 640, 1, SKY + gradient);
    }
    for i in 0..50 {
        let x = (noise(i, 2, 17) % 638) as i32 + 1;
        let y = (noise(i, 2, 31) % 115) as i32 + 4;
        a.dot(x, y, SKY + (5 + i % 5) as u8);
    }
    let (sx, sy) = (518, 249);
    for (r, shade) in [(64, 25), (52, 29), (42, 32)] {
        a.ellipse(sx, sy, r, r, SKY + shade);
    }
    for y in -35i32..=35 {
        let radius = (35 * 35 - y * y).isqrt();
        a.rect(sx - radius, sy + y, radius * 2 + 1, 1, if y < 16 { CREAM + 7 } else { GOLD + 7 });
    }
    for (x,y,w) in [(4,222,95),(77,246,153),(44,264,112),(180,236,36),(534,214,96),(498,272,141),(247,255,60)] {
        a.poly(&[(x,y),(x+w/4,y-2),(x+w*2/3,y-1),(x+w,y),(x+w/2,y+2)], SKY + 20 + (y % 7) as u8);
    }

    let backdrop: &[&[(i32,i32)]] = &[
        &[(0,290),(36,274),(71,282),(105,258),(123,267),(145,263),(170,289),(192,280),(222,303),(273,289),(310,310),(346,296),(396,304),(422,284),(454,290),(468,270),(506,286),(533,270),(554,285),(590,249),(615,258),(640,293)],
        &[(0,306),(35,297),(69,307),(108,290),(150,311),(191,305),(236,325),(263,318),(307,331),(356,323),(407,313),(441,301),(467,309),(502,291),(542,307),(576,283),(607,286),(640,308)],
        &[(0,317),(42,313),(87,329),(128,326),(177,350),(230,342),(283,357),(341,349),(380,331),(420,326),(449,315),(490,318),(533,300),(580,309),(640,304)],
    ];
    for (ridge, colour) in backdrop.iter().zip([DISTANCE+8, DISTANCE+4, DISTANCE]) {
        let mut p = ridge.to_vec(); p.extend([(640,480),(0,480)]);
        a.poly(&p, colour);
    }
    for (x,y) in [(105,258),(590,249),(533,270),(468,270)] {
        a.poly(&[(x-12,y+9),(x,y),(x+12,y+6),(x+6,y+6),(x+2,y+3),(x-3,y+6)], DISTANCE+11);
    }

    let river = [(0,376),(95,370),(183,376),(300,363),(351,346),(390,344),(422,351),(413,360),(383,372),(349,387),(372,407),(462,432),(559,480),(0,480)];
    a.poly(&river, WATER+2);
    a.poly(&[(0,383),(101,377),(181,381),(300,372),(361,352),(398,350),(408,354),(365,374),(330,389),(360,411),(451,443),(512,479),(0,479)], WATER+3);
    for i in 0..114 {
        let y = 353 + (noise(i, 0, 43) % 127) as i32;
        let x = (noise(i, 1, 43) % 626) as i32;
        if matches!(a.pixels[(y * a.w + x) as usize], c if c == WATER+2 || c == WATER+3) {
            let width = 2 + (y - 349) / 12 + i % 8;
            let colour = if (x - (378 - (y - 349) * 2)).abs() < 38 { GOLD + 4 } else { WATER + 4 };
            for px in x..(x + width).min(a.w) {
                let at = (y * a.w + px) as usize;
                if a.pixels[at] == WATER + 2 || a.pixels[at] == WATER + 3 { a.pixels[at] = colour; }
            }
        }
    }
    a.poly(&[(640,321),(559,320),(520,330),(458,335),(422,348),(420,362),(457,367),(434,378),(388,382),(359,391),(391,406),(458,421),(523,458),(550,479),(639,479)], PINE+2);
    a.poly(&[(640,335),(557,340),(526,350),(460,347),(428,357),(473,363),(469,376),(423,385),(386,392),(420,404),(499,424),(569,462),(590,479),(639,479)], PINE+3);
    a.poly(&[(640,353),(599,350),(566,366),(530,359),(492,369),(485,387),(446,397),(418,397),(508,419),(574,450),(639,462)], PINE+4);
    a.poly(&[(0,432),(44,423),(102,438),(160,441),(210,457),(270,464),(314,479),(0,479)], PINE+1);
    a.poly(&[(0,448),(32,446),(84,451),(128,461),(169,479),(0,479)], PINE);

    // Road, bridge and its paired supports use the same highlight direction as the keep.
    a.poly(&[(639,375),(598,366),(554,367),(508,355),(475,356),(444,350),(452,354),(479,363),(511,361),(550,376),(600,372),(639,386)], MOSS+4);
    a.line(465, 359, 408, 370, STONE+2);
    for (x,y) in [(425,367),(443,363)] {
        a.rect(x,y,5,10,STONE+2); a.line(x,y,x,y+9,STONE+5);
    }
    a.poly(&[(406,365),(460,352),(466,358),(409,372)], STONE+5);
    a.line(406,365,460,352,CREAM+4);

    fortress(&mut a, 306, 337, 118);
    for i in 0..59 {
        let side = i % 2 == 0;
        let x = if side { (noise(i, 4, 9) % 177) as i32 - 8 } else { 542 + (noise(i, 4, 11) % 107) as i32 };
        let y = if side { 445 + (noise(i, 2, 25) % 60) as i32 } else { 395 + (noise(i, 4, 23) % 98) as i32 };
        let h = 9 + (y - 383).max(0) / 5 + (noise(i, 2, 11) % 24) as i32;
        fir(&mut a, x, y, h, if i % 3 == 0 { PINE+2 } else { PINE });
    }
    a
}

fn fortress(a: &mut Art, x: i32, y: i32, w: i32) {
    a.poly(&[(x-22,y+10),(x+12,y-5),(x+w-21,y-8),(x+w+22,y+7),(x+w-5,y+16),(x+19,y+18)], SLATE+2);
    a.poly(&[(x-12,y+9),(x+11,y-2),(x+w-18,y-6),(x+w+13,y+5),(x+w-11,y+9),(x+12,y+14)], STONE+3);
    fort_tower(a,x+41,y-17,29,45,true);
    fort_tower(a,x+73,y-14,21,31,false);
    a.poly(&[(x+9,y-17),(x+w-14,y-23),(x+w-14,y+1),(x+9,y+7)], STONE+5);
    a.line(x+9,y-17,x+w-14,y-23,PARCHMENT+5);
    a.line(x+9,y-4,x+w-14,y-10,STONE+4);
    a.line(x+9,y+5,x+w-14,y-1,STONE+2);
    for i in (12..w-12).step_by(8) {
        a.rect(x+i,y-20-i/16,4,5,STONE+6);
    }
    a.ellipse(x+63,y-5,9,9,STONE+3);
    a.ellipse(x+63,y-4,7,7,INK+1);
    a.rect(x+56,y-4,15,8,INK+1);
    a.line(x+56,y+4,x+70,y+4,WOOD+3);
    fort_tower(a,x+3,y+4,23,29,false);
    fort_tower(a,x+w-23,y-2,23,33,false);
}

fn fort_tower(a: &mut Art, x: i32, base: i32, w: i32, h: i32, spire: bool) {
    let top = base - h;
    a.poly(&[(x,top),(x+w/2,top-5),(x+w,top-1),(x+w,base-4),(x+w/2,base),(x,base-3)], SLATE+3);
    a.poly(&[(x,top),(x+w/2,top-4),(x+w/2,base),(x,base-3)], STONE+6);
    for j in (top+5..base-3).step_by(6) {
        a.line(x,j,x+w/2,j+2,STONE+4);
        a.line(x+w/2,j+2,x+w,j-1,SLATE+4);
    }
    if spire {
        a.poly(&[(x-4,top),(x+w/2,top-28),(x+w/2+1,top+1)], BLUE+4);
        a.poly(&[(x+w/2,top-28),(x+w+3,top-2),(x+w/2+1,top+1)], BLUE+2);
        a.line(x+w/2,top-41,x+w/2,top-26,GOLD+6);
        a.poly(&[(x+w/2,top-41),(x+w/2+20,top-39),(x+w/2+14,top-34),(x+w/2,top-35)], RED+5);
    } else {
        a.line(x-1,top,x+w/2,top-4,CREAM+5);
        for offset in [0,w/3,w*2/3] { a.rect(x+offset,top-6-offset/8,5,6,STONE+6); }
    }
    a.rect(x+w/4,top+10,3,7,INK+1);
    a.rect(x+w*3/4,top+11,2,7,INK+1);
    a.dot(x+w/4,top+15,GOLD+5);
}

fn fir(a: &mut Art, x: i32, y: i32, h: i32, c: u8) {
    a.line(x,y-h,x,y,WOOD+1);
    for row in (0..3).rev() {
        let top = y-h + h*row/6;
        let base = (top + h/2).min(y);
        let w = 1 + (base-top)*(3+row)/10;
        a.poly(&[(x,top),(x-w,base),(x+1,base+2),(x+w,base-1)], c);
        a.poly(&[(x,top+2),(x-w+1,base-1),(x,base+1)], c+1);
    }
}
