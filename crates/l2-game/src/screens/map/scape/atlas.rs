use l2_view::detail::SCALE;
use l2_view::{Canvas, Clip};
use tiny_skia::Pixmap;

use super::new_image;

// One historical near scroll step in each axis. The far view uses the same
// chunks, with partial chunks clipped by the normal detail blit.
const W: i32 = 60;
const H: i32 = 30;
// The largest viewport touches at most 9 × 16 chunks. A modulo grid keeps
// additional recently seen chunks around without a hash table or LRU updates.
const COLS: usize = 12;
const ROWS: usize = 20;

type Cell = (i32, i32);

pub(super) struct Atlas {
    chunks: Vec<Option<Chunk>>,
}

struct Chunk {
    cell: Cell,
    image: Pixmap,
}

impl Atlas {
    pub(super) fn new() -> Self {
        Self { chunks: (0..COLS * ROWS).map(|_| None).collect() }
    }

    pub(super) fn ensure(&mut self, clip: Clip, shift: (i32, i32), mut paint: impl FnMut(Clip) -> Pixmap) {
        let bounds = needed(clip, shift);
        for y in bounds.y0..bounds.y1 {
            let mut x = bounds.x0;
            while x < bounds.x1 {
                if self.has((x, y)) {
                    x += 1;
                    continue;
                }
                // Paint adjacent misses as one region. Opening or jumping the
                // map is a single paint; a one-step scroll only paints an
                // entering strip (or two strips when moving diagonally).
                let x1 = (x + 1..bounds.x1).find(|&i| self.has((i, y))).unwrap_or(bounds.x1);
                let y1 = (y + 1..bounds.y1)
                    .find(|&j| (x..x1).any(|i| self.has((i, j))))
                    .unwrap_or(bounds.y1);
                let (left, top) = origin(clip, (x, y));
                let (right, bottom) = origin(clip, (x1, y1));
                let region = paint(Clip::new(left, top, right, bottom));
                if (x1 - x, y1 - y) == (1, 1) {
                    self.chunks[index((x, y))] = Some(Chunk { cell: (x, y), image: region });
                } else {
                    self.retain(&region, Clip::new(x, y, x1, y1));
                }
                x = x1;
            }
        }
    }

    fn retain(&mut self, region: &Pixmap, bounds: Clip) {
        let stride = region.width() as usize * 4;
        let chunk_stride = W as usize * SCALE * 4;
        for y in bounds.y0..bounds.y1 {
            for x in bounds.x0..bounds.x1 {
                let cell = (x, y);
                let chunk = self.chunks[index(cell)]
                    .get_or_insert_with(|| Chunk { cell, image: new_image(W, H) });
                chunk.cell = cell;
                let left = (x - bounds.x0) as usize * chunk_stride;
                let top = (y - bounds.y0) as usize * H as usize * SCALE;
                for (i, row) in chunk.image.data_mut().chunks_exact_mut(chunk_stride).enumerate() {
                    let start = (top + i) * stride + left;
                    row.copy_from_slice(&region.data()[start..start + chunk_stride]);
                }
            }
        }
    }

    pub(super) fn blit(&self, canvas: &mut Canvas, clip: Clip, shift: (i32, i32)) {
        let bounds = needed(clip, shift);
        for y in bounds.y0..bounds.y1 {
            for x in bounds.x0..bounds.x1 {
                let chunk = self.chunks[index((x, y))].as_ref().expect("visible scenery was painted");
                debug_assert_eq!(chunk.cell, (x, y));
                let (left, top) = origin(clip, (x, y));
                canvas.blit_detail_opaque(chunk.image.data(), chunk.image.width() as usize,
                    left - shift.0, top - shift.1, clip);
            }
        }
    }

    fn has(&self, cell: Cell) -> bool {
        self.chunks[index(cell)].as_ref().is_some_and(|chunk| chunk.cell == cell)
    }
}

fn index((x, y): Cell) -> usize {
    y.rem_euclid(ROWS as i32) as usize * COLS + x.rem_euclid(COLS as i32) as usize
}

fn origin(clip: Clip, (x, y): Cell) -> (i32, i32) {
    (clip.x0 + x * W, clip.y0 + y * H)
}

fn needed(clip: Clip, (x, y): (i32, i32)) -> Clip {
    Clip::new(x.div_euclid(W), y.div_euclid(H),
        (x + clip.width() - 1).div_euclid(W) + 1, (y + clip.height() - 1).div_euclid(H) + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use l2_view::campaign::{FAR, NEAR};

    #[test]
    fn scroll_paints_only_entering_chunks_and_backtracking_hits_the_atlas() {
        let mut atlas = Atlas::new();
        let clip = NEAR.clip();
        let mut regions = Vec::new();
        let mut paint = |region: Clip| {
            regions.push(region);
            new_image(region.width(), region.height())
        };
        atlas.ensure(clip, (W, H), &mut paint);
        atlas.ensure(clip, (W * 2, H * 2), &mut paint);
        atlas.ensure(clip, (W, H), &mut paint);
        assert_eq!(regions, [
            Clip::new(W, clip.y0 + H, W * 9, clip.y0 + H * 16),
            Clip::new(W * 9, clip.y0 + H * 2, W * 10, clip.y0 + H * 17),
            Clip::new(W * 2, clip.y0 + H * 16, W * 9, clip.y0 + H * 17),
        ]);
    }

    #[test]
    fn far_pan_keeps_partially_intersecting_chunks_and_world_pixels() {
        let mut atlas = Atlas::new();
        let clip = FAR.clip();
        let shift = (FAR.pitch, FAR.row_step);
        let paint = |region: Clip| {
            let mut image = new_image(region.width(), region.height());
            let width = image.width() as usize;
            for (i, sample) in image.data_mut().chunks_exact_mut(4).enumerate() {
                let x = region.x0 as usize * SCALE + i % width;
                let y = region.y0 as usize * SCALE + i / width;
                sample.copy_from_slice(&[x as u8, y as u8, 0, 255]);
            }
            image
        };
        atlas.ensure(clip, shift, paint);
        let mut actual = Canvas::screen();
        actual.enable_detail();
        atlas.blit(&mut actual, clip, shift);

        let image = paint(Clip::new(clip.x0 + shift.0, clip.y0 + shift.1,
            clip.x1 + shift.0, clip.y1 + shift.1));
        let mut expected = Canvas::screen();
        expected.enable_detail();
        expected.blit_detail_opaque(image.data(), image.width() as usize, clip.x0, clip.y0, clip);
        assert!(actual == expected, "partial atlas chunks must not shift pixels or overpaint the clip");
    }
}
