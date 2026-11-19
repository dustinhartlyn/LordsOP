use super::{Art, CREAM, GOLD, INK, RED};
use crate::cursor::RESOURCES;
use l2_formats::{cursors::Picture, Palette};

pub(crate) fn pictures(palette: &Palette) -> Vec<Picture> {
    RESOURCES.into_iter().map(|id| {
        let mut image = Art::new(24, 24);
        let edge = INK;
        let bright = CREAM + 7;
        let brass = GOLD + 5;
        let hotspot = match id {
            102 | 103 => {
                let mark = if id == 103 { RED + 6 } else { bright };
                for (x, y, w, h) in [(10, 1, 5, 9), (10, 15, 5, 8), (1, 10, 9, 5), (15, 10, 8, 5)] {
                    image.rect(x, y, w, h, edge);
                    image.rect(x+1, y+1, w-2, h-2, mark);
                }
                if id == 103 { image.dot(12, 12, mark); }
                (12, 12)
            }
            104 => {
                image.ellipse(12, 12, 10, 10, edge);
                image.ellipse(12, 12, 8, 8, brass);
                image.ellipse(12, 12, 5, 5, edge);
                image.ellipse(12, 12, 3, 3, 0);
                image.line(6, 7, 10, 4, bright);
                (12, 12)
            }
            105 => {
                image.poly(&[(1,0),(1,20),(7,15),(11,22),(16,20),(12,13),(20,13)], edge);
                image.poly(&[(3,4),(3,16),(7,12),(12,20),(13,19),(9,11),(16,11)], bright);
                (1, 0)
            }
            110 => {
                image.ellipse(11, 11, 10, 10, edge);
                image.ellipse(11, 11, 8, 8, brass);
                image.line(8, 7, 14, 7, edge);
                image.line(14, 7, 14, 10, edge);
                image.line(14, 10, 11, 12, edge);
                image.rect(10, 12, 2, 2, edge);
                image.rect(10, 15, 2, 2, edge);
                (11, 11)
            }
            111 => {
                image.poly(&[(4,14),(5,9),(8,9),(8,2),(13,2),(13,8),(19,8),(21,13),(18,20),(16,22),(9,22)], edge);
                image.poly(&[(6,14),(7,11),(10,14),(10,4),(11,4),(11,10),(18,10),(19,13),(16,20),(10,20)], bright);
                image.line(13, 11, 13, 14, brass);
                image.line(16, 11, 16, 14, brass);
                (10, 2)
            }
            113 => {
                image.poly(&[(0,6),(6,1),(14,1),(20,5),(20,10),(10,5),(4,6)], edge);
                image.poly(&[(3,5),(8,2),(14,3),(18,6),(13,4),(7,4)], bright);
                image.poly(&[(15,5),(18,5),(9,22),(5,22)], edge);
                image.line(16, 7, 8, 20, brass);
                (0, 6)
            }
            _ => unreachable!("unknown pointer id"),
        };
        let rgba = image.pixels.into_iter().flat_map(|index| {
            let [r, g, b] = palette.rgb(index);
            [r, g, b, if index == 0 { 0 } else { 255 }]
        }).collect();
        Picture { id, width: 24, height: 24, hot_x: hotspot.0, hot_y: hotspot.1, rgba }
    }).collect()
}
