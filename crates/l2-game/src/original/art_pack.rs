//! Shared embedded QOI artwork, seamless painterly terrain textures, and sprite blitters.
//! Every asset decodes lazily once into RAM and keeps the binary fully self-contained.

use std::sync::LazyLock;
use tiny_skia::Pixmap;

pub(crate) struct RgbaImage {
    pub width: usize,
    pub height: usize,
    /// Premultiplied RGBA pixels (`width * height * 4`).
    pub rgba: Vec<u8>,
}

pub(crate) struct RawImage {
    pub width: usize,
    pub height: usize,
    /// Straight (un-premultiplied) RGBA pixels (`width * height * 4`).
    pub rgba: Vec<u8>,
}

pub(crate) struct Texture {
    pub width: usize,
    pub height: usize,
    /// Packed RGB pixels (`width * height * 3`) for fast periodic world sampling.
    pub rgb: Vec<u8>,
    pub mean: [f32; 3],
}

impl RgbaImage {
    pub(crate) fn qoi(bytes: &[u8]) -> Self {
        let (width, height, rgba) = decode_qoi(bytes, true);
        Self { width, height, rgba }
    }
}

impl RawImage {
    pub(crate) fn qoi(bytes: &[u8]) -> Self {
        let (width, height, rgba) = decode_qoi(bytes, false);
        Self { width, height, rgba }
    }
}

impl Texture {
    pub(crate) fn qoi(bytes: &[u8]) -> Self {
        let (width, height, rgba) = decode_qoi(bytes, false);
        let mut rgb = Vec::with_capacity(width * height * 3);
        let mut sum = [0u64; 3];
        for px in rgba.chunks_exact(4) {
            rgb.push(px[0]);
            rgb.push(px[1]);
            rgb.push(px[2]);
            sum[0] += u64::from(px[0]);
            sum[1] += u64::from(px[1]);
            sum[2] += u64::from(px[2]);
        }
        let n = (width * height).max(1) as f32;
        let mean = [sum[0] as f32 / n, sum[1] as f32 / n, sum[2] as f32 / n];
        Self { width, height, rgb, mean }
    }

    /// Periodic bilinear RGB sample in texture pixel coordinates `(u, v)`.
    #[inline]
    pub(crate) fn sample_f32(&self, u: f32, v: f32) -> [f32; 3] {
        let (w, h) = (self.width as i32, self.height as i32);
        let x0 = u.floor() as i32;
        let y0 = v.floor() as i32;
        let fx = u - x0 as f32;
        let fy = v - y0 as f32;
        let ix0 = x0.rem_euclid(w) as usize;
        let iy0 = y0.rem_euclid(h) as usize;
        let ix1 = (ix0 + 1) % self.width;
        let iy1 = (iy0 + 1) % self.height;
        let at = |x: usize, y: usize| {
            let i = (y * self.width + x) * 3;
            [
                f32::from(self.rgb[i]),
                f32::from(self.rgb[i + 1]),
                f32::from(self.rgb[i + 2]),
            ]
        };
        let a = at(ix0, iy0);
        let b = at(ix1, iy0);
        let c = at(ix0, iy1);
        let d = at(ix1, iy1);
        let mut out = [0.0; 3];
        for ch in 0..3 {
            let top = a[ch] + (b[ch] - a[ch]) * fx;
            let bot = c[ch] + (d[ch] - c[ch]) * fx;
            out[ch] = top + (bot - top) * fy;
        }
        out
    }

    /// Fast periodic nearest sample as `[u8; 3]`.
    #[inline]
    pub(crate) fn sample_u8(&self, x: i32, y: i32) -> [u8; 3] {
        let ix = x.rem_euclid(self.width as i32) as usize;
        let iy = y.rem_euclid(self.height as i32) as usize;
        let i = (iy * self.width + ix) * 3;
        [self.rgb[i], self.rgb[i + 1], self.rgb[i + 2]]
    }

    /// Zero-mean painterly detail offset `sample(u, v) - mean`, ideal for
    /// modulating seasonal or elevation-shaded surfaces without shifting hue.
    #[inline]
    pub(crate) fn detail(&self, u: f32, v: f32) -> [f32; 3] {
        let s = self.sample_f32(u, v);
        [s[0] - self.mean[0], s[1] - self.mean[1], s[2] - self.mean[2]]
    }
}

/// Decode a QOI byte slice into either premultiplied or straight RGBA.
pub(crate) fn decode_qoi(bytes: &[u8], premul: bool) -> (usize, usize, Vec<u8>) {
    fn take<const N: usize>(input: &mut &[u8]) -> [u8; N] {
        let (bytes, rest) = input.split_at(N);
        *input = rest;
        bytes.try_into().unwrap()
    }
    let mut input = bytes.strip_prefix(b"qoif").expect("valid QOI header");
    let width = u32::from_be_bytes(take(&mut input)) as usize;
    let height = u32::from_be_bytes(take(&mut input)) as usize;
    let _format = take::<2>(&mut input);
    let mut rgba = Vec::with_capacity(width * height * 4);
    let mut cache = [[0u8; 4]; 64];
    let mut pixel = [0u8, 0, 0, 255];
    while rgba.len() < width * height * 4 {
        let [byte] = take(&mut input);
        let mut count = 1usize;
        match byte {
            0xfe => pixel[..3].copy_from_slice(&take::<3>(&mut input)),
            0xff => pixel = take(&mut input),
            0x00..=0x3f => pixel = cache[byte as usize],
            0x40..=0x7f => {
                for (c, shift) in pixel[..3].iter_mut().zip([4, 2, 0]) {
                    *c = c.wrapping_add((byte >> shift & 3).wrapping_sub(2));
                }
            }
            0x80..=0xbf => {
                let [diff] = take(&mut input);
                let g = (byte & 63).wrapping_sub(32);
                pixel[0] = pixel[0].wrapping_add(g).wrapping_add((diff >> 4).wrapping_sub(8));
                pixel[1] = pixel[1].wrapping_add(g);
                pixel[2] = pixel[2].wrapping_add(g).wrapping_add((diff & 15).wrapping_sub(8));
            }
            _ => count += (byte & 63) as usize,
        }
        let [r, g, b, a] = pixel.map(usize::from);
        cache[(r * 3 + g * 5 + b * 7 + a * 11) % 64] = pixel;
        let mut sample = pixel;
        if premul && a < 255 {
            for c in &mut sample[..3] {
                *c = ((usize::from(*c) * a + 127) / 255) as u8;
            }
        }
        for _ in 0..count {
            rgba.extend_from_slice(&sample);
        }
    }
    assert_eq!(rgba.len(), width * height * 4);
    assert_eq!(input, b"\0\0\0\0\0\0\0\x01");
    (width, height, rgba)
}

/// Composite a straight-RGBA sprite into a premultiplied `tiny_skia::Pixmap`
/// using bilinear sampling, centered horizontally at `cx` with bottom at `foot_y`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn stamp_sprite(
    dst: &mut Pixmap,
    src: &RawImage,
    cx: f32,
    foot_y: f32,
    dest_w: f32,
    dest_h: f32,
    tint: [f32; 3],
    snow: f32,
    flip_x: bool,
) {
    if dest_w <= 1.0 || dest_h <= 1.0 {
        return;
    }
    let dw = dst.width() as i32;
    let dh = dst.height() as i32;
    let x0 = (cx - dest_w * 0.5).floor() as i32;
    let y0 = (foot_y - dest_h).floor() as i32;
    let x1 = (cx + dest_w * 0.5).ceil() as i32;
    let y1 = foot_y.ceil() as i32;
    let cx0 = x0.max(0);
    let cy0 = y0.max(0);
    let cx1 = x1.min(dw);
    let cy1 = y1.min(dh);
    if cx0 >= cx1 || cy0 >= cy1 {
        return;
    }
    let inv_w = src.width as f32 / dest_w;
    let inv_h = src.height as f32 / dest_h;
    let sw = src.width;
    let sh = src.height;
    let data = dst.data_mut();
    let pitch = dw as usize;

    for py in cy0..cy1 {
        let sv = ((py as f32 + 0.5) - (foot_y - dest_h)) * inv_h - 0.5;
        let iy0 = (sv.floor() as i32).clamp(0, sh as i32 - 1) as usize;
        let iy1 = (iy0 + 1).min(sh - 1);
        let fy = (sv - sv.floor()).clamp(0.0, 1.0);
        let row_dst = py as usize * pitch;
        for px in cx0..cx1 {
            let mut su = ((px as f32 + 0.5) - (cx - dest_w * 0.5)) * inv_w - 0.5;
            if flip_x {
                su = (sw as f32 - 1.0) - su;
            }
            let ix0 = (su.floor() as i32).clamp(0, sw as i32 - 1) as usize;
            let ix1 = (ix0 + 1).min(sw - 1);
            let fx = (su - su.floor()).clamp(0.0, 1.0);

            let p00 = (iy0 * sw + ix0) * 4;
            let p10 = (iy0 * sw + ix1) * 4;
            let p01 = (iy1 * sw + ix0) * 4;
            let p11 = (iy1 * sw + ix1) * 4;
            let s = &src.rgba;
            let bilerp = |ch: usize| {
                let top = f32::from(s[p00 + ch]) + (f32::from(s[p10 + ch]) - f32::from(s[p00 + ch])) * fx;
                let bot = f32::from(s[p01 + ch]) + (f32::from(s[p11 + ch]) - f32::from(s[p01 + ch])) * fx;
                top + (bot - top) * fy
            };
            let a = bilerp(3);
            if a < 1.0 {
                continue;
            }
            let mut r = bilerp(0) * tint[0];
            let mut g = bilerp(1) * tint[1];
            let mut b = bilerp(2) * tint[2];
            if snow > 0.0 {
                // Soft winter frost dusting on upper sunlit pixels
                let upper = (1.0 - (sv / sh as f32).clamp(0.0, 1.0)).powf(1.5);
                let luma = (r * 0.3 + g * 0.59 + b * 0.11) / 255.0;
                let frost = (snow * upper * (0.35 + 0.65 * luma)).clamp(0.0, 0.72);
                r = r + (226.0 - r) * frost;
                g = g + (235.0 - g) * frost;
                b = b + (234.0 - b) * frost;
            }
            let af = (a / 255.0).clamp(0.0, 1.0);
            let sr = (r.clamp(0.0, 255.0) * af).round() as u8;
            let sg = (g.clamp(0.0, 255.0) * af).round() as u8;
            let sb = (b.clamp(0.0, 255.0) * af).round() as u8;
            let sa = a.round().clamp(0.0, 255.0) as u8;
            let di = (row_dst + px as usize) * 4;
            if sa == 255 {
                data[di..di + 4].copy_from_slice(&[sr, sg, sb, 255]);
            } else {
                let inv = 255 - u16::from(sa);
                data[di] = sr.saturating_add(((u16::from(data[di]) * inv + 127) / 255) as u8);
                data[di + 1] = sg.saturating_add(((u16::from(data[di + 1]) * inv + 127) / 255) as u8);
                data[di + 2] = sb.saturating_add(((u16::from(data[di + 2]) * inv + 127) / 255) as u8);
                data[di + 3] = sa.saturating_add(((u16::from(data[di + 3]) * inv + 127) / 255) as u8);
            }
        }
    }
}

static TEX_GRASS: LazyLock<Texture> = LazyLock::new(|| Texture::qoi(include_bytes!("art/tex-grass.qoi")));
static TEX_ROCK: LazyLock<Texture> = LazyLock::new(|| Texture::qoi(include_bytes!("art/tex-rock.qoi")));
static TEX_WATER: LazyLock<Texture> = LazyLock::new(|| Texture::qoi(include_bytes!("art/tex-water.qoi")));
static TEX_EARTH: LazyLock<Texture> = LazyLock::new(|| Texture::qoi(include_bytes!("art/tex-earth.qoi")));
static TEX_WHEAT: LazyLock<Texture> = LazyLock::new(|| Texture::qoi(include_bytes!("art/tex-wheat.qoi")));
static TEX_PLANKS: LazyLock<Texture> = LazyLock::new(|| Texture::qoi(include_bytes!("art/tex-planks.qoi")));
static TEX_MASONRY: LazyLock<Texture> = LazyLock::new(|| Texture::qoi(include_bytes!("art/tex-masonry.qoi")));

static VILLAGE_PANORAMA: LazyLock<RawImage> =
    LazyLock::new(|| RawImage::qoi(include_bytes!("art/village-panorama.qoi")));
static SOLDIERS: LazyLock<RawImage> =
    LazyLock::new(|| RawImage::qoi(include_bytes!("art/soldiers.qoi")));
static SOLDIERS_MASK: LazyLock<RawImage> =
    LazyLock::new(|| RawImage::qoi(include_bytes!("art/soldiers-mask.qoi")));
static SIEGE_ENGINES: LazyLock<RawImage> =
    LazyLock::new(|| RawImage::qoi(include_bytes!("art/siege-engines.qoi")));
static PROP_CASTLE_KEEP: LazyLock<RawImage> =
    LazyLock::new(|| RawImage::qoi(include_bytes!("art/prop-castle-keep.qoi")));
static PROP_OAK: LazyLock<RawImage> =
    LazyLock::new(|| RawImage::qoi(include_bytes!("art/prop-oak.qoi")));

pub(crate) fn grass() -> &'static Texture { &TEX_GRASS }
pub(crate) fn rock() -> &'static Texture { &TEX_ROCK }
pub(crate) fn water() -> &'static Texture { &TEX_WATER }
pub(crate) fn earth() -> &'static Texture { &TEX_EARTH }
pub(crate) fn wheat() -> &'static Texture { &TEX_WHEAT }
pub(crate) fn planks() -> &'static Texture { &TEX_PLANKS }
pub(crate) fn masonry() -> &'static Texture { &TEX_MASONRY }

pub(crate) fn village_panorama() -> &'static RawImage { &VILLAGE_PANORAMA }
pub(crate) fn soldiers() -> &'static RawImage { &SOLDIERS }
pub(crate) fn soldiers_mask() -> &'static RawImage { &SOLDIERS_MASK }
pub(crate) fn siege_engines() -> &'static RawImage { &SIEGE_ENGINES }
pub(crate) fn prop_castle_keep() -> &'static RawImage { &PROP_CASTLE_KEEP }
pub(crate) fn prop_oak() -> &'static RawImage { &PROP_OAK }
