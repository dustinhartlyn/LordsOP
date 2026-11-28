//! True-color artwork at three samples per logical axis. Indexed painters still
//! own layout, clipping and hit testing; their writes cover the detail below.

use crate::{Canvas, Clip};

pub const SCALE: usize = crate::sharp::SCALE;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Detail {
    pub(crate) pixels: Vec<u8>,
    pub(crate) coverage: Vec<Coverage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Coverage {
    Clear,
    Partial,
    Opaque,
}

impl Canvas {
    pub fn enable_detail(&mut self) {
        if self.detail.is_none() {
            let area = self.width * self.height;
            self.detail = Some(Box::new(Detail {
                pixels: vec![0; area * SCALE * SCALE * 4],
                coverage: vec![Coverage::Clear; area],
            }));
        }
    }

    pub fn has_detail(&self) -> bool {
        self.detail.is_some()
    }

    /// Draw premultiplied RGBA art in 3x pixels at a logical origin and clip.
    pub fn blit_detail(&mut self, rgba: &[u8], physical_width: usize, x: i32, y: i32, clip: Clip) {
        self.blit_detail_impl(rgba, physical_width, x, y, clip, false);
    }

    /// Draw an opaque image whose width and height are whole logical pixels.
    pub fn blit_detail_opaque(&mut self, rgba: &[u8], physical_width: usize, x: i32, y: i32, clip: Clip) {
        self.blit_detail_impl(rgba, physical_width, x, y, clip, true);
    }

    fn blit_detail_impl(&mut self, rgba: &[u8], physical_width: usize, x: i32, y: i32, clip: Clip, opaque: bool) {
        if physical_width == 0 {
            return;
        }
        let Some(detail) = self.detail.as_mut() else { return };
        let s = SCALE as i32;
        let (ox, oy) = (x * s, y * s);
        let x0 = ox.max(clip.x0.max(0) * s);
        let y0 = oy.max(clip.y0.max(0) * s);
        let x1 = (ox + physical_width as i32).min(clip.x1.min(self.width as i32) * s);
        let y1 = (oy + (rgba.len() / 4 / physical_width) as i32).min(clip.y1.min(self.height as i32) * s);
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        if opaque {
            let width = (x1 - x0) as usize * 4;
            for py in y0..y1 {
                let src = ((py - oy) as usize * physical_width + (x0 - ox) as usize) * 4;
                let dst = (py as usize * self.width * SCALE + x0 as usize) * 4;
                detail.pixels[dst..dst + width].copy_from_slice(&rgba[src..src + width]);
            }
            for y in y0 / s..y1 / s {
                let start = y as usize * self.width;
                detail.coverage[start + (x0 / s) as usize..start + (x1 / s) as usize].fill(Coverage::Opaque);
            }
            return;
        }
        for py in y0..y1 {
            let src = (py - oy) as usize * physical_width;
            for px in x0..x1 {
                let i = (src + (px - ox) as usize) * 4;
                let sample = [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]];
                if sample[3] != 0 {
                    detail.over(self.width, px as usize, py as usize, sample);
                }
            }
        }
    }

    /// One coverage sample, for vector glyphs drawn directly at the same density.
    pub fn detail_sample(&mut self, x: i32, y: i32, rgb: [u8; 3], alpha: u8) {
        let (x, y) = (x as usize, y as usize);
        if alpha == 0 || x >= self.width * SCALE || y >= self.height * SCALE {
            return;
        }
        if let Some(detail) = self.detail.as_mut() {
            let [r, g, b] = rgb.map(|ch| mul(ch, alpha));
            detail.over(self.width, x, y, [r, g, b, alpha]);
        }
    }
}

fn mul(v: u8, a: u8) -> u8 {
    ((u16::from(v) * u16::from(a) + 127) / 255) as u8
}

impl Detail {
    pub(crate) fn dim(&mut self, width: usize) {
        let pitch = width * SCALE;
        for (i, &coverage) in self.coverage.iter().enumerate() {
            if coverage == Coverage::Clear {
                continue;
            }
            let origin = ((i / width) * pitch + i % width) * SCALE;
            for row in 0..SCALE {
                let at = (origin + row * pitch) * 4;
                for pixel in self.pixels[at..at + SCALE * 4].chunks_exact_mut(4) {
                    for channel in &mut pixel[..3] {
                        *channel = (u16::from(*channel) * 9 / 16) as u8;
                    }
                }
            }
        }
    }

    fn over(&mut self, width: usize, x: usize, y: usize, src: [u8; 4]) {
        let pitch = width * SCALE;
        let block = (y / SCALE) * width + x / SCALE;
        if self.coverage[block] == Coverage::Clear {
            self.coverage[block] = Coverage::Partial;
            let top = (y / SCALE) * SCALE * pitch + (x / SCALE) * SCALE;
            for row in 0..SCALE {
                let row = (top + row * pitch) * 4;
                self.pixels[row..row + SCALE * 4].fill(0);
            }
        }
        let at = (y * pitch + x) * 4;
        let dest = &mut self.pixels[at..at + 4];
        if src[3] == 255 || dest[3] == 0 {
            dest.copy_from_slice(&src);
        } else {
            let back = 255 - src[3];
            for ch in 0..4 {
                dest[ch] = src[ch] + mul(dest[ch], back);
            }
        }
    }

    /// Area average for a fully opaque 3× block. A 2× output overlaps two
    /// samples per axis with integer weights 2:1, so sum before dividing.
    #[inline]
    pub(crate) fn resample_opaque(&self, x: usize, y: usize, width: usize, k: usize, out: &mut [u8]) {
        let pitch = width * 12;
        let top = (y * width * 3 + x) * 12;
        let a = &self.pixels[top..top + 12];
        let b = &self.pixels[top + pitch..top + pitch + 12];
        let c = &self.pixels[top + pitch * 2..top + pitch * 2 + 12];
        if k == 1 {
            let px = &mut out[(y * width + x) * 4..][..4];
            for v in 0..3 {
                let sum = [a, b, c].into_iter().map(|r| {
                    u16::from(r[v]) + u16::from(r[v + 4]) + u16::from(r[v + 8])
                }).sum::<u16>();
                px[v] = (sum / 9) as u8;
            }
            px[3] = 255;
            return;
        }
        let dst = (y * width * 2 + x) * 8;
        let (rows, last) = out[dst..].split_at_mut(width * 8);
        let (upper, lower) = (&mut rows[..8], &mut last[..8]);
        for v in 0..3 {
            let plus = |r: &[u8], i| u16::from(r[v + i]);
            let center = plus(b, 4);
            let (n, s, w, e) = (2 * plus(a, 4), 2 * plus(c, 4), 2 * plus(b, 0), 2 * plus(b, 8));
            upper[v] = ((4 * plus(a, 0) + n + w + center) / 9) as u8;
            upper[v + 4] = ((4 * plus(a, 8) + n + e + center) / 9) as u8;
            lower[v] = ((4 * plus(c, 0) + s + w + center) / 9) as u8;
            lower[v + 4] = ((4 * plus(c, 8) + s + e + center) / 9) as u8;
        }
        upper[3] = 255; upper[7] = 255;
        lower[3] = 255; lower[7] = 255;
    }

    #[inline]
    pub(crate) fn sample(&self, i: usize, rgb: [u8; 4], phase: Option<u8>) -> [u8; 4] {
        let [r, g, b, alpha] = self.pixels[i * 4..i * 4 + 4].try_into().unwrap();
        if alpha == 0 {
            return rgb;
        }
        let mut foreground = [r, g, b];
        if let Some(phase) = phase {
            // Fade the unassociated color then restore premultiplication.
            foreground = foreground.map(|v| mul(crate::fade::channel_at(
                ((u16::from(v) * 255 / u16::from(alpha)).min(255)) as u8, phase), alpha));
        }
        if alpha != 255 {
            for (v, back) in foreground.iter_mut().zip(rgb) {
                *v += mul(back, 255 - alpha);
            }
        }
        [foreground[0], foreground[1], foreground[2], 255]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use l2_formats::Palette;

    #[test]
    fn artwork_keeps_subpixels_through_cache_and_indexed_occlusion() {
        let mut base = Canvas::new(2, 1);
        base.enable_detail();
        let mut src = [0; 6 * 3 * 4];
        src[..4].copy_from_slice(&[240, 0, 0, 255]);
        src[12..16].copy_from_slice(&[0, 0, 240, 255]);
        base.blit_detail(&src, 6, 0, 0, Clip::WHOLE);
        let mut frame = Canvas::new(2, 1);
        frame.enable_detail();
        frame.copy_plane(&base);
        frame.fill_rect(1, 0, 1, 1, 0);
        let mut output = [0; 6 * 3 * 4];
        let palette = Palette::from_bytes(&[0; 768]).unwrap();
        frame.to_rgba_at(&palette, 3, &mut output);
        assert_eq!(&output[..16], &[240, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255]);
        frame.to_rgba_at(&palette, 1, &mut output[..8]);
        assert_eq!(&output[..4], &[26, 0, 0, 255]);

        frame.shade(&palette);
        frame.detail_sample(3, 0, [0, 0, 240], 255);
        frame.to_rgba_at(&palette, 3, &mut output);
        assert_eq!(&output[..4], &[135, 0, 0, 255]);
        assert_eq!(&output[12..16], &[0, 0, 240, 255]);
    }

    #[test]
    fn opaque_spans_match_per_sample_compositing_at_all_native_densities() {
        let mut src = [0; 4 * 3 * SCALE * SCALE * 4];
        for (i, pixel) in src.chunks_exact_mut(4).enumerate() {
            pixel.copy_from_slice(&[(i * 37) as u8, (i * 71) as u8, (i * 113) as u8, 255]);
        }
        let palette = Palette::from_bytes(&[0; 768]).unwrap();
        let mut fast = Canvas::new(4, 3);
        let mut reference = fast.clone();
        fast.enable_detail();
        reference.enable_detail();
        fast.blit_detail_opaque(&src, 4 * SCALE, 0, 0, Clip::WHOLE);
        reference.blit_detail(&src, 4 * SCALE, 0, 0, Clip::WHOLE);
        for canvas in [&mut fast, &mut reference] {
            canvas.shade(&palette);
            canvas.detail_sample(1, 2, [120, 130, 180], 151);
            canvas.fill_rect(2, 1, 1, 1, 0);
            canvas.detail_sample(7, 4, [140, 90, 30], 173);
            canvas.enable_sharp();
            canvas.sharp_seed(0, 2, 1, 3);
            canvas.sharp_claim(0, 2, 1, 3);
            canvas.sharp_set(1, 7, 255);
        }
        for k in 1..=SCALE {
            let n = 4 * 3 * k * k * 4;
            let (mut output, mut expected) = (vec![0; n], vec![0; n]);
            fast.to_rgba_at(&palette, k, &mut output);
            reference.to_rgba_at(&palette, k, &mut expected);
            assert_eq!(output, expected, "output scale {k}");
        }
    }
}
