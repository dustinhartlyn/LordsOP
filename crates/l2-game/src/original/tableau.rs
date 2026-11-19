//! Painted event cards in the standalone edition. Like the rest of the art,
//! these scenes are drawn from geometry and a palette, not stock screenshots.

use super::{noise, Art, COPPER, CREAM, GOLD, GRASS, INK, IRON, PARCHMENT, PINE, RED, SLATE, STONE, WATER, WOOD};
use crate::movie::Film;
use l2_formats::DecodedFrame;

pub(crate) fn tableau(film: Film) -> Option<(DecodedFrame, i32, i32)> {
    let (x, y, w, h, mode) = match film {
        Film::Castle(level) => (158, 20, 320, 190, Scene::Keep(level)),
        Film::Capture { take, .. } => (43, 108, 394, 184, Scene::Standard(take)),
        Film::Ending { record, .. } => (93, 108, 288, 178,
            if record.group == l2_kingdom::victory::MSG_VICTORY { Scene::Crown } else { Scene::Fall }),
        Film::Battle { .. } => (39, 73, 320, 176, Scene::Battle),
        _ => return None,
    };
    let mut art = landscape(w, h, mode);
    match mode {
        Scene::Keep(level) => keep(&mut art, w / 2, h * 3 / 4, 2 + i32::from(level) / 2),
        Scene::Standard(take) => {
            keep(&mut art, w / 4, h * 2 / 3, 2);
            standard(&mut art, w * 2 / 3, 20, h - 20, [RED, WATER, COPPER][usize::from(take).min(2)]);
        }
        Scene::Crown => crown(&mut art, w / 2, h / 2 + 28),
        Scene::Fall => {
            art.poly(&[(36,h-12),(w-34,h-12),(w-64,h-81),(63,h-81)], STONE);
            art.rect(w / 2 - 16, 40, 32, h - 65, INK);
            art.rect(w / 2 - 13, 44, 25, h - 74, WOOD + 3);
            for i in [6, 15, 22] { art.line(w/2-13+i, 46, w/2-13+i, h-34, WOOD + 1); }
            art.poly(&[(w/2-48,69),(w/2+50,79),(w/2+31,112),(w/2-33,103)], RED + 2);
            art.line(w/2-35, 74, w/2+42, 82, GOLD + 4);
        }
        Scene::Battle => {
            for (cx, cy, colour, dir) in [(w/3,h-20,RED,1),(w*2/3,h-20,WATER,-1)] {
                art.line(cx-50*dir, cy, cx+50*dir, 30, INK);
                art.poly(&[(cx+51*dir,30),(cx+16*dir,48),(cx+30*dir,54)], IRON+6);
                art.poly(&[(cx-31,cy-59),(cx+32,cy-59),(cx+29,cy-20),(cx,cy),(cx-28,cy-20)], INK);
                art.poly(&[(cx-25,cy-54),(cx+26,cy-54),(cx+22,cy-22),(cx,cy-7),(cx-22,cy-22)], colour+4);
                art.poly(&[(cx,cy-51),(cx+13,cy-36),(cx,cy-18),(cx-13,cy-36)], GOLD+5);
            }
        }
    }
    for margin in [1, 4] {
        art.line(margin, margin, w-1-margin, margin, GOLD + 4);
        art.line(margin, h-1-margin, w-1-margin, h-1-margin, WOOD + 3);
        art.line(margin, margin, margin, h-1-margin, GOLD + 4);
        art.line(w-1-margin, margin, w-1-margin, h-1-margin, WOOD + 3);
    }
    Some((DecodedFrame {
        width: w as u16, height: h as u16,
        opaque: vec![true; art.pixels.len()], indices: art.pixels,
    }, x, y))
}

#[derive(Clone, Copy)]
enum Scene { Keep(u8), Standard(u8), Crown, Fall, Battle }

fn landscape(w: i32, h: i32, scene: Scene) -> Art {
    let mut art = Art::new(w, h);
    let (sky, ground, sun) = match scene {
        Scene::Keep(..) => (SLATE, GRASS, CREAM + 7),
        Scene::Standard(..) => (COPPER, PINE, GOLD + 7),
        Scene::Crown => (PARCHMENT, WOOD, GOLD + 6),
        Scene::Fall => (SLATE, STONE, STONE + 5),
        Scene::Battle => (RED, PINE, GOLD + 6),
    };
    for y in 0..h {
        for x in 0..w {
            let n = noise(x, y, 47);
            let bands = ((y * 7 / h) + (n % 4) as i32 / 2).min(7) as u8;
            art.dot(x, y, sky + bands);
        }
    }
    art.ellipse(w * 3 / 5, h / 3, h / 7, h / 7, sun);
    for layer in 0..3 {
        let b = h / 2 + layer * h / 10;
        let step = w / 8;
        let mut points = Vec::with_capacity(11);
        points.push((0,h));
        for i in 0..=8 {
            let top = (noise(i, layer, 19) % (h as u32 / 5)) as i32;
            points.push((if i == 8 { w-1 } else { i*step },b-top));
        }
        points.push((w-1,h));
        art.poly(&points, ground + (6 - layer * 2) as u8);
    }
    for i in 0..1600 {
        let n = noise(i, h, 32);
        let x = (n % w as u32) as i32;
        let y = h / 2 + (((n >> 13) % (h as u32 / 2)) as i32);
        if (ground..ground+8).contains(&art.pixels[(y * w + x) as usize]) {
            art.line(x, y, x + (i % 5), y, ground + 3 + (i % 2) as u8);
        }
    }
    art
}

fn keep(art: &mut Art, cx: i32, y: i32, scale: i32) {
    let w = 27 * scale;
    let h = 14 * scale;
    art.ellipse(cx, y+1, w+12, 12, PINE);
    art.rect(cx-w, y-h, w*2, h, STONE+4);
    art.poly(&[(cx+w,y-h),(cx+w+10,y-h-7),(cx+w+10,y-6),(cx+w,y)], STONE+2);
    for x in [cx-w, cx, cx+w] {
        art.rect(x-5*scale,y-h-12*scale,10*scale,h+12*scale,STONE+5);
        art.rect(x+3*scale,y-h-12*scale,3*scale,h+12*scale,STONE+2);
        for a in [-6, -1, 4] { art.rect(x+a*scale, y-h-16*scale, 3*scale, 5*scale, STONE+6); }
        art.rect(x-scale,y-h-8*scale,2*scale,4*scale,INK);
        art.line(x-4*scale,y-h-11*scale,x+2*scale,y-h-11*scale,STONE+7);
    }
    for yy in (y-h+5..y-2).step_by(7) { art.line(cx-w, yy, cx+w, yy, STONE+3); }
    art.ellipse(cx, y-h/2, 4*scale, 4*scale, INK);
    art.rect(cx-4*scale,y-h/2,8*scale,h/2,INK);
    art.rect(cx-3*scale,y-h/2,6*scale,h/2,WOOD+2);
    standard(art,cx,y-h-30*scale,y-h-15*scale,RED);
}

fn standard(art: &mut Art, x: i32, top: i32, bottom: i32, cloth: u8) {
    let width = ((bottom-top) / 2).max(14);
    let height = (width * 4 / 7).max(8);
    art.rect(x-2,top,bottom.min(top+5)-top,bottom-top,INK);
    art.line(x,top,x,bottom,GOLD+5);
    art.ellipse(x,top,3,4,GOLD+6);
    art.poly(&[(x+2,top+6),(x+width/2,top+4),(x+width,top+11),(x+width*3/4,top+height/2+9),
        (x+width,top+height+5),(x+width/2,top+height-1),(x+2,top+height+5)], INK);
    art.poly(&[(x+3,top+8),(x+width/2,top+6),(x+width-4,top+12),(x+width*3/4-3,top+height/2+9),
        (x+width-4,top+height+2),(x+width/2,top+height-3),(x+3,top+height+3)], cloth+4);
    let (a,b) = (x+width/3,top+height/2+6);
    art.poly(&[(a,b-height/4),(a+height/5,b),(a,b+height/4),(a-height/5,b)], GOLD+6);
}

fn crown(art: &mut Art, x: i32, y: i32) {
    art.ellipse(x,y+12,94,22,WOOD);
    art.ellipse(x,y+9,82,16,RED+3);
    art.poly(&[(x-62,y-49),(x-34,y-32),(x-18,y-57),(x,y-39),(x+18,y-57),(x+35,y-32),(x+62,y-49),
        (x+45,y+4),(x+26,y+12),(x-28,y+12),(x-45,y+4)], INK);
    art.poly(&[(x-56,y-43),(x-34,y-28),(x-18,y-51),(x,y-36),(x+18,y-51),(x+35,y-28),(x+57,y-44),
        (x+41,y+1),(x+26,y+8),(x-27,y+8),(x-42,y+1)], GOLD+5);
    art.line(x-43,y-5,x+43,y-5,GOLD+7);
    for (off,col) in [(-29,RED),(0,WATER),(29,RED)] {
        art.ellipse(x+off,y-13,7,9,WOOD+2);
        art.ellipse(x+off,y-13,5,7,col+4);
        art.dot(x+off-1,y-16,CREAM+7);
    }
}
