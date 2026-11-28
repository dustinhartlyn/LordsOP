use l2_kingdom::tables::Commodity;

use super::{Brush, C, L, Pt, Q, Rgb, Scene, Terrain, curve, oval, polygon};

pub(super) fn settlement(p: &mut Brush, scene: Scene, t: &Terrain) {
    // Walk the valley back to front. A handful of recognizable silhouettes
    // break up the canopy; the bridge and the chapel anchor the composition.
    for (x, y, size) in [
        (5.0, 53.0, 7.0), (12.0, 51.4, 8.5), (20.0, 54.0, 7.9), (26.0, 51.0, 9.7),
        (33.0, 52.6, 8.4), (46.0, 51.8, 7.0), (54.0, 54.5, 7.4), (60.0, 53.0, 5.8),
        (95.0, 55.1, 4.8), (101.0, 55.4, 5.7), (114.0, 56.0, 4.8),
        (214.0, 49.9, 6.4), (224.0, 51.2, 5.4), (251.0, 49.2, 6.5), (257.0, 50.6, 7.2),
        (347.0, 51.2, 8.8), (354.0, 50.4, 7.1), (360.0, 53.1, 9.8),
    ] {
        broadleaf(p, t, x, y, size);
    }
    if scene.resource(Commodity::Wood) {
        for (x, y, size) in [
            (1.8, 59.0, 13.8), (12.0, 58.6, 12.8), (21.0, 61.1, 15.0), (33.0, 60.2, 12.5),
            (44.0, 62.8, 14.2), (55.0, 59.2, 9.6),
        ] {
            pine(p, t, x, y, size);
        }
        for (x, y, size) in [(8.0, 64.8, 12.4), (29.0, 65.0, 12.9), (49.0, 65.8, 12.3)] {
            broadleaf(p, t, x, y, size);
        }
        timber(p, 49.5, 68.2);
    } else {
        broadleaf(p, t, 17.5, 58.8, 11.5);
        broadleaf(p, t, 43.5, 60.8, 10.5);
    }

    for (x, y, w, h, roof) in [
        (89.5, 61.2, 12.1, 4.5, Roof::Straw), (108.8, 62.1, 12.9, 5.4, Roof::Clay),
        (121.0, 63.4, 13.2, 5.2, Roof::Slate), (143.2, 57.9, 9.1, 4.0, Roof::Clay),
        (207.0, 60.3, 9.0, 3.8, Roof::Slate), (219.2, 62.7, 13.2, 5.2, Roof::Clay),
        (254.0, 58.8, 16.3, 6.0, Roof::Straw),
    ] {
        cottage(p, t, x, y, w, h, roof);
    }
    windmill(p, t, 241.0, 59.5);
    if scene.resource(Commodity::Stone) {
        quarry(p, t);
    }
    if scene.resource(Commodity::Iron) {
        mine(p, t);
    }
    for (x, y, h) in [(310.0, 60.7, 9.5), (344.0, 65.2, 14.4), (355.5, 65.7, 13.0)] {
        pine(p, t, x, y, h);
    }

    fence(p, t, (64.5, 79.5), (102.0, 68.7), 7);
    fence(p, t, (266.0, 68.0), (298.0, 74.0), 6);
    cattle(p, 272.3, 76.8, 0.83, 0x7a573d);
    cattle(p, 289.7, 82.4, 1.02, 0x5d463c);
    cattle(p, 277.6, 83.8, 1.0, 0xe2cfab);
    broadleaf(p, t, 94.2, 76.1, 13.9);
    cottage(p, t, 101.0, 72.9, 15.8, 7.3, Roof::Clay);
    broadleaf(p, t, 156.1, 67.6, 10.8);
    cottage(p, t, 152.2, 73.8, 18.4, 6.6, Roof::Straw);
    chapel(p, t, 127.0, 75.2);
    cottage(p, t, 108.8, 80.3, 19.0, 7.6, Roof::Slate);
    if scene.resource(Commodity::Weapons) {
        p.line((113.4, 74.7), (113.4, 82.1), 0x472f26, 0.64);
        p.poly(&[(111.2, 77.2), (115.9, 77.0), (115.8, 79.8), (111.3, 80.1)], 0x623d29);
        p.line((112.2, 78.4), (114.9, 78.2), 0xd7bd82, 0.45);
        chimney(p, 123.0, 68.2, 2.2);
    }
    cottage(p, t, 129.0, 84.5, 18.3, 8.1, Roof::Clay);
    chimney(p, 140.7, 71.0, 1.7);
    broadleaf(p, t, 102.5, 84.7, 11.6);
    cottage(p, t, 216.0, 73.8, 17.7, 7.5, Roof::Clay);
    broadleaf(p, t, 231.0, 72.1, 10.2);

    bridge(p, t);
    mill(p, t);
    broadleaf(p, t, 235.0, 86.2, 14.6);
    pine(p, t, 253.0, 91.4, 18.0);
    broadleaf(p, t, 243.6, 90.0, 11.3);

    for (x, y, h) in [
        (2.0, 88.3, 4.8), (9.5, 86.0, 4.1), (21.0, 83.0, 2.8), (73.0, 83.7, 2.7),
        (137.0, 101.0, 5.7), (145.0, 100.3, 3.9), (141.6, 110.1, 6.5), (152.0, 111.3, 4.6),
        (206.0, 96.1, 3.3), (216.0, 100.0, 4.8), (229.5, 107.7, 5.0), (241.5, 110.8, 5.5),
        (306.7, 85.0, 4.7), (359.6, 94.2, 6.9),
    ] {
        reeds(p, t, x, y, h);
    }
    boat(p, 183.5, 106.4);
}

fn broadleaf(p: &mut Brush, t: &Terrain, x: f32, y: f32, h: f32) {
    let [shade, green, lit] = t.leaf;
    let at = |a, b| -> Pt { (x + h * a, y + h * b) };
    p.tint(&oval(x + h * 0.24, y - h * 0.01, h * 0.48, h * 0.13), shade, 118);
    p.stroke(&curve(at(-0.02, 0.0), &[Q(x + h * 0.025, y - h * 0.24, x - h * 0.08, y - h * 0.55)], false), 0x6b5946, 245, h * 0.079);
    let crown = curve(at(-0.40, -0.35), &[
        C(x - h * 0.61, y - h * 0.53, x - h * 0.48, y - h * 0.74, x - h * 0.30, y - h * 0.76),
        C(x - h * 0.29, y - h * 0.93, x - h * 0.07, y - h * 1.03, x + h * 0.07, y - h * 0.88),
        C(x + h * 0.29, y - h * 0.98, x + h * 0.44, y - h * 0.80, x + h * 0.40, y - h * 0.70),
        C(x + h * 0.55, y - h * 0.60, x + h * 0.58, y - h * 0.41, x + h * 0.42, y - h * 0.34),
        C(x + h * 0.34, y - h * 0.19, x + h * 0.18, y - h * 0.23, x + h * 0.10, y - h * 0.25),
        C(x - h * 0.05, y - h * 0.15, x - h * 0.13, y - h * 0.29, x - h * 0.22, y - h * 0.23),
        Q(x - h * 0.43, y - h * 0.23, x - h * 0.40, y - h * 0.35),
    ], true);
    p.gradient(&crown, at(-0.25, -0.89), at(0.32, -0.31), &[(0.0, lit), (0.49, green), (1.0, shade)]);
    let light = curve(at(-0.46, -0.48), &[
        Q(x - h * 0.45, y - h * 0.72, x - h * 0.27, y - h * 0.72),
        Q(x - h * 0.26, y - h * 0.91, x - h * 0.08, y - h * 0.88),
        Q(x + h * 0.01, y - h * 0.85, x + h * 0.12, y - h * 0.77),
        Q(x - h * 0.04, y - h * 0.76, x - h * 0.11, y - h * 0.67),
        Q(x - h * 0.29, y - h * 0.67, x - h * 0.33, y - h * 0.54),
    ], true);
    p.tint(&light, if t.snow { 0xe4e4da } else { lit }, 154);
    if h >= 9.0 {
        p.tint(&oval(x + h * 0.21, y - h * 0.68, 0.14 * h, 0.071 * h), lit, 105);
        p.stroke(&curve(at(0.23, -0.42), &[Q(x + h * 0.13, y - h * 0.49, x + h * 0.01, y - h * 0.41)], false), lit, 124, h * 0.035);
    }
}

fn pine(p: &mut Brush, t: &Terrain, x: f32, y: f32, h: f32) {
    let [shade, green, lit] = t.leaf;
    let at = |a, b| -> Pt { (x + h * a, y + h * b) };
    p.tint(&oval(x + h * 0.22, y - h * 0.02, h * 0.48, h * 0.089), shade, 125);
    p.line(at(0.0, -0.37), at(0.02, 0.0), 0x695742, h * 0.061);
    let boughs = polygon(&[
        at(-0.025, -1.0), at(0.15, -0.63), at(0.09, -0.66), at(0.24, -0.46), at(0.15, -0.48),
        at(0.32, -0.24), at(0.16, -0.27), at(0.22, -0.17), at(0.02, -0.2), at(-0.22, -0.15),
        at(-0.17, -0.24), at(-0.34, -0.18), at(-0.19, -0.44), at(-0.26, -0.42),
        at(-0.12, -0.65), at(-0.17, -0.62),
    ]);
    p.gradient(&boughs, at(-0.22, -0.8), at(0.17, -0.29), &[(0.0, lit), (0.61, green), (1.0, shade)]);
    for (tier, w) in [(0.67, 0.14), (0.46, 0.21), (0.23, 0.28)] {
        p.tint(&polygon(&[at(-0.01, -tier - 0.22), at(-w, -tier), at(-w * 0.24, -tier - 0.06), at(w * 0.27, -tier)]),
            if t.snow { 0xe2e3d9 } else { lit }, if t.snow { 226 } else { 130 });
    }
}

#[derive(Clone, Copy)]
enum Roof {
    Clay,
    Slate,
    Straw,
}

impl Roof {
    fn colors(self) -> [Rgb; 3] {
        match self {
            Self::Clay => [0xd49b6c, 0xaa684f, 0x6f4a40],
            Self::Slate => [0xa4b0af, 0x6c8589, 0x445a61],
            Self::Straw => [0xdbc17d, 0xb59655, 0x796642],
        }
    }
}

fn cottage(p: &mut Brush, t: &Terrain, x: f32, y: f32, w: f32, h: f32, roof: Roof) {
    let [lit, body, dark] = roof.colors();
    let front = w * 0.72;
    let ridge = w * 0.30;
    let eave = y - h;
    let back = w * 0.17;
    let wall = polygon(&[(x, y), (x, eave), (x + front, eave), (x + front, y + 0.45)]);
    let gable = polygon(&[
        (x + front, y + 0.45), (x + front, eave), (x + w * 0.81, eave - ridge),
        (x + w, eave - back), (x + w, y - back),
    ]);
    p.tint(&oval(x + w * 0.73, y - 0.3, w * 0.73, w * 0.14), 0x43534b, 115);
    p.gradient(&wall, (x, eave), (x + front, y), &[(0.0, 0xf2debc), (0.54, 0xd3bb95), (1.0, 0xaa8e72)]);
    p.gradient(&gable, (x + front, eave), (x + w, y), &[(0.0, 0xc0a080), (1.0, 0x887564)]);
    p.stroke(&curve((x + front, y + 0.2), &[L(x + front, eave)], false), 0x796954, 230, 0.55);
    p.line((x + front, y - h * 0.32), (x + w, y - back - h * 0.28), 0x93755b, 0.43);
    if w > 13.0 {
        let door = x + front * 0.62;
        p.poly(&[(door - 0.85, y), (door - 0.85, y - h * 0.60), (door + 0.88, y - h * 0.59), (door + 0.88, y)], 0x66504a);
        p.line((door - 0.49, y - 0.2), (door - 0.49, y - h * 0.49), 0x957558, 0.37);
        let sill = y - h * 0.49;
        let sill_x = x + front * 0.26;
        p.poly(&[(sill_x - 1.15, sill), (sill_x - 1.15, sill - 1.88), (sill_x + 1.14, sill - 1.88), (sill_x + 1.14, sill)], 0x725d4f);
        p.line((sill_x - 0.61, sill - 0.6), (sill_x + 0.51, sill - 0.6), 0xe7bc78, 0.47);
    }
    p.line((x + w * 0.84, eave + 0.46), (x + w * 0.84, eave + 1.9), 0x6c584e, 0.69);

    let ridge_y = eave - ridge;
    let front_roof = polygon(&[
        (x - 1.0, eave + 0.64), (x + w * 0.16, ridge_y),
        (x + w * 0.82, ridge_y - 0.24), (x + front + 0.75, eave + 0.64),
    ]);
    p.poly(&[(x + front + 0.75, eave + 0.64), (x + w * 0.82, ridge_y - 0.24), (x + w + 0.75, eave - back + 0.25)], dark);
    p.gradient(&front_roof, (x + w * 0.33, ridge_y), (x + w * 0.40, eave + 0.5), &[(0.0, lit), (0.64, body), (1.0, dark)]);
    for (i, r) in [0.26, 0.49, 0.73].into_iter().enumerate() {
        let a = x + w * 0.16 - (w * 0.16 + 1.0) * r;
        let b = x + w * 0.82 - (w * 0.10 - 0.75) * r;
        let slope_y = ridge_y + (eave + 0.49 - ridge_y) * r;
        p.stroke(&curve((a + 0.1, slope_y), &[Q((a + b) * 0.5, slope_y + 0.32, b - 0.1, slope_y - 0.12)], false), dark, 138, 0.35);
        if w > 15.0 {
            for j in 0..5 {
                let u = (j as f32 + 0.52 + (i % 2) as f32 * 0.34) / 5.8;
                let px = a + (b - a) * u;
                p.stroke(&curve((px, slope_y + 0.10), &[L(px - 0.25, slope_y + ridge * 0.16)], false), lit, 139, 0.33);
            }
        }
    }
    p.line((x + w * 0.16, ridge_y), (x + w * 0.82, ridge_y - 0.24), lit, 0.64);
    if t.snow {
        p.gradient(&curve((x + w * 0.16, ridge_y), &[
            L(x + w * 0.82, ridge_y - 0.24), L(x + w * 0.77, ridge_y + ridge * 0.62),
            Q(x + w * 0.67, ridge_y + ridge * 0.73, x + w * 0.58, ridge_y + ridge * 0.63),
            Q(x + w * 0.47, ridge_y + ridge * 0.55, x + w * 0.39, ridge_y + ridge * 0.67),
            Q(x + w * 0.22, ridge_y + ridge * 0.81, x + w * 0.06, ridge_y + ridge * 0.59),
        ], true), (x, ridge_y), (x + w * 0.30, eave), &[(0.0, 0xf4eee2), (1.0, 0xb7c6ca)]);
    }
}

fn chimney(p: &mut Brush, x: f32, y: f32, width: f32) {
    p.poly(&[(x, y - width * 1.7), (x + width, y - width * 1.8), (x + width, y), (x, y + 0.1)], 0x9b7663);
    p.line((x, y - width * 1.8), (x + width, y - width * 1.88), 0xc19b80, 0.6);
    let plume = curve((x + width * 0.4, y - width * 2.0), &[
        C(x + 0.8, y - 7.0, x - 1.4, y - 7.4, x - 5.0, y - 8.6),
        C(x - 8.2, y - 10.1, x - 9.9, y - 12.2, x - 9.6, y - 14.0),
    ], false);
    p.stroke(&plume, 0xe5ded0, 133, 1.1);
}

fn chapel(p: &mut Brush, t: &Terrain, x: f32, y: f32) {
    cottage(p, t, x + 3.0, y - 1.1, 24.3, 9.6, Roof::Slate);
    let front = polygon(&[(x, y), (x, y - 17.0), (x + 7.7, y - 16.9), (x + 7.7, y + 0.4)]);
    p.gradient(&front, (x, y - 16.0), (x + 7.7, y), &[(0.0, 0xe0d8bc), (1.0, 0xb3a589)]);
    p.poly(&[(x + 7.7, y + 0.4), (x + 7.7, y - 16.9), (x + 10.1, y - 18.1), (x + 10.1, y - 1.1)], 0x8c8479);
    let spire = polygon(&[(x - 0.95, y - 16.95), (x + 4.55, y - 31.3), (x + 8.1, y - 16.8)]);
    p.poly(&[(x + 4.55, y - 31.3), (x + 10.65, y - 17.95), (x + 8.1, y - 16.8)], 0x4c6471);
    p.gradient(&spire, (x + 1.2, y - 25.6), (x + 7.5, y - 17.2), &[(0.0, 0x93a9ae), (0.8, 0x647e88), (1.0, 0x5a707a)]);
    p.line((x + 4.55, y - 32.4), (x + 4.55, y - 29.8), 0xa58558, 0.61);
    p.line((x + 3.8, y - 31.9), (x + 5.27, y - 31.9), 0xc6b07a, 0.43);
    for by in [y - 14.7, y - 8.8] {
        p.line((x + 0.1, by), (x + 7.7, by + 0.2), 0xe0d3b1, 0.61);
    }
    for wx in [x + 2.2, x + 5.35] {
        let opening = curve((wx - 0.76, y - 10.15), &[L(wx - 0.76, y - 12.52), Q(wx, y - 14.07, wx + 0.74, y - 12.52), L(wx + 0.74, y - 10.15)], true);
        p.fill(&opening, 0x665f55);
    }
    let door = curve((x + 2.1, y + 0.1), &[L(x + 2.1, y - 3.6), Q(x + 3.55, y - 6.0, x + 5.1, y - 3.6), L(x + 5.1, y + 0.27)], true);
    p.fill(&door, 0x716353);
    p.stroke(&door, 0xb9ad90, 245, 0.36);
    if t.snow {
        p.poly(&[(x + 4.55, y - 30.8), (x + 3.9, y - 25.0), (x + 1.4, y - 23.0)], 0xe3e6dd);
    }
}

fn windmill(p: &mut Brush, t: &Terrain, x: f32, y: f32) {
    p.tint(&oval(x + 2.7, y - 0.25, 8.1, 1.7), t.leaf[0], 116);
    let wall = curve((x - 4.7, y), &[L(x - 3.15, y - 11.5), Q(x, y - 12.5, x + 4.0, y - 11.4), L(x + 5.1, y - 0.5), Q(x, y + 1.3, x - 4.7, y)], true);
    p.gradient(&wall, (x - 3.7, y - 8.0), (x + 4.7, y - 4.7), &[(0.0, 0xe0cfab), (0.55, 0xbd9d7a), (1.0, 0x887464)]);
    let top = curve((x - 4.0, y - 10.95), &[Q(x - 1.1, y - 13.8, x, y - 17.0), Q(x + 2.0, y - 15.2, x + 5.0, y - 11.3), Q(x + 1.1, y - 10.4, x - 4.0, y - 10.95)], true);
    p.gradient(&top, (x - 2.5, y - 14.7), (x + 3.6, y - 10.5), &[(0.0, 0xd3a978), (1.0, 0x957057)]);
    let hub = (x - 1.42, y - 10.72);
    for (vx, vy) in [(-7.3, -8.24), (8.02, -6.25), (7.3, 8.24), (-8.02, 6.25)] {
        let tip = (hub.0 + vx, hub.1 + vy);
        let (side_x, side_y) = (-vy * 0.19, vx * 0.19);
        p.line(hub, tip, 0x705747, 0.78);
        let sail = polygon(&[
            (hub.0 + vx * 0.43, hub.1 + vy * 0.43), tip,
            (tip.0 + side_x, tip.1 + side_y), (hub.0 + vx * 0.43 + side_x, hub.1 + vy * 0.43 + side_y),
        ]);
        p.fill(&sail, if t.snow { 0xe2ded0 } else { 0xe4d3ae });
        for k in [0.49, 0.68, 0.87] {
            let at = (hub.0 + vx * k, hub.1 + vy * k);
            p.line(at, (at.0 + side_x, at.1 + side_y), 0xab9071, 0.28);
        }
    }
    p.fill(&oval(hub.0, hub.1, 0.83, 0.7), 0x77604c);
    p.line((x + 1.6, y - 0.3), (x + 1.6, y - 3.2), 0x705e52, 1.2);
}

fn bridge(p: &mut Brush, t: &Terrain) {
    let stone = curve((168.0, 78.2), &[
        C(180.0, 74.4, 188.9, 69.8, 203.4, 69.5), L(206.0, 73.9), L(202.3, 75.6),
        L(201.7, 78.8), L(198.4, 79.6), Q(198.7, 74.8, 195.3, 75.4),
        Q(191.1, 75.8, 190.6, 81.65), L(186.9, 82.65),
        Q(187.1, 77.7, 183.3, 78.8), Q(179.6, 79.7, 178.6, 85.3),
        L(175.6, 85.8), Q(175.6, 81.6, 173.7, 82.3), Q(171.6, 82.9, 170.5, 87.0),
        L(166.0, 88.1), L(166.4, 83.2),
    ], true);
    p.tint(&oval(186.0, 83.9, 17.0, 2.4), t.stream[2], 96);
    p.gradient(&stone, (176.0, 74.0), (182.6, 88.0), &[(0.0, 0xe4cea6), (0.42, 0xb8a084), (1.0, 0x837365)]);
    p.stroke(&curve((166.8, 81.5), &[C(177.8, 78.7, 192.4, 72.2, 204.3, 72.1)], false), 0x796f61, 183, 0.77);
    p.stroke(&curve((167.1, 80.65), &[C(177.8, 77.4, 191.5, 71.4, 204.1, 71.1)], false), 0xe0c6a0, 247, 0.85);
    for (x, y) in [(169.0, 81.7), (177.6, 79.35), (189.2, 75.4), (199.6, 73.9)] {
        p.line((x - 0.48, y - 0.25), (x - 0.55, y + 1.1), 0x9a8873, 0.41);
    }
    for (x, y, w, h) in [(170.5, 85.0, 4.1, 3.0), (178.65, 83.6, 8.2, 4.22), (190.8, 80.1, 7.8, 4.56)] {
        let arch = curve((x, y), &[C(x + w * 0.18, y - h, x + w * 0.94, y - h * 1.38, x + w, y - 1.92)], false);
        p.stroke(&arch, 0xd5b894, 242, 0.57);
    }
    if t.snow {
        p.stroke(&curve((167.6, 79.31), &[C(180.5, 74.8, 189.5, 70.3, 203.6, 69.75)], false), 0xe7e6db, 255, 0.94);
    }
}

fn mill(p: &mut Brush, t: &Terrain) {
    cottage(p, t, 140.6, 93.8, 15.4, 6.5, Roof::Straw);
    let (x, y) = (154.1, 90.8);
    p.tint(&oval(x + 1.1, y + 4.38, 4.95, 1.25), t.stream[2], 129);
    let wheel = oval(x, y, 4.62, 4.78);
    p.stroke(&wheel, 0x514a3d, 255, 1.02);
    p.stroke(&oval(x - 0.12, y - 0.18, 4.25, 4.41), 0xa4815d, 255, 0.47);
    for i in 0..8 {
        let a = i as f32 * std::f32::consts::FRAC_PI_4 + 0.20;
        let (s, c) = a.sin_cos();
        let to = (x + c * 4.5, y + s * 4.6);
        p.line((x, y), to, 0x705a43, 0.6);
        p.line((to.0 - s * 0.7, to.1 + c * 0.7), (to.0 + s * 0.67, to.1 - c * 0.67), 0x62503d, 0.73);
    }
    p.fill(&oval(x, y, 0.86, 0.82), 0xa78661);
    p.stroke(&curve((153.6, 96.3), &[Q(157.8, 95.4, 160.0, 96.4), L(162.7, 96.2)], false), 0xc4d5bf, 173, 0.52);
}

fn quarry(p: &mut Brush, t: &Terrain) {
    let face = curve((314.0, 62.1), &[
        L(316.8, 56.5), L(323.3, 56.5), L(325.9, 52.0), L(335.4, 51.2), L(339.8, 54.4),
        L(347.0, 54.2), L(349.2, 60.3), L(346.2, 66.0), Q(332.0, 65.1, 314.0, 66.2),
    ], true);
    p.gradient(&face, (322.0, 53.5), (341.8, 65.4), &[(0.0, 0xe0caa6), (0.6, 0xb4a48b), (1.0, 0x908a7b)]);
    p.poly(&[(318.5, 57.2), (328.3, 56.7), (335.9, 53.0), (337.1, 55.1), (330.1, 59.1), (317.7, 60.7)], 0xb0a18a);
    p.poly(&[(325.8, 61.0), (337.6, 59.7), (347.6, 57.8), (348.0, 60.4), (337.9, 62.6), (324.2, 63.9)], 0x938674);
    p.stroke(&curve((318.8, 56.7), &[L(326.9, 55.7), L(328.8, 52.0), L(334.0, 51.7)], false), if t.snow { 0xf1ede2 } else { 0xefdbb9 }, 248, 0.61);
    for (x, y, w) in [(312.4, 65.0, 2.6), (322.0, 65.6, 3.4), (335.2, 67.1, 3.0), (340.6, 65.7, 3.3)] {
        p.poly(&[(x, y), (x + w * 0.4, y - 1.4), (x + w, y - 0.6), (x + w * 1.15, y + 0.4)], 0xd1bb98);
        p.line((x + 0.25, y + 0.31), (x + w, y + 0.43), 0x8c8170, 0.43);
    }
}

fn mine(p: &mut Brush, t: &Terrain) {
    let bluff = polygon(&[(282.5, 62.2), (287.4, 58.8), (288.0, 55.0), (295.0, 52.8), (301.8, 53.7), (307.0, 57.5), (307.8, 62.3), (299.7, 64.0)]);
    p.gradient(&bluff, (286.2, 54.0), (301.3, 62.0), &[(0.0, 0xbeb39b), (1.0, 0x827a6c)]);
    let tunnel = curve((291.4, 62.9), &[L(291.9, 58.4), Q(295.1, 53.85, 298.75, 57.6), L(299.35, 62.9)], true);
    p.gradient(&tunnel, (295.0, 56.7), (295.5, 62.6), &[(0.0, 0x3f4543), (1.0, 0x5c4c40)]);
    p.stroke(&tunnel, 0x917356, 255, 0.92);
    p.line((292.65, 63.0), (292.8, 57.4), 0xc0a27b, 0.75);
    p.line((291.6, 57.8), (299.5, 57.4), 0x9c8464, 0.88);
    let track = if t.snow { 0x8c8f8a } else { 0x8b795c };
    p.stroke(&curve((293.3, 62.0), &[Q(293.0, 65.1, 285.4, 67.2)], false), track, 234, 0.43);
    p.stroke(&curve((295.8, 62.0), &[Q(295.1, 66.3, 287.7, 68.2)], false), track, 235, 0.43);
    p.line((288.45, 65.7), (291.35, 66.25), 0x786953, 0.44);
    p.line((290.93, 64.55), (293.65, 64.9), 0x786953, 0.43);
}

fn timber(p: &mut Brush, x: f32, y: f32) {
    for (dy, dx) in [(0.8, -0.5), (0.05, 1.25), (-1.29, 0.15)] {
        p.line((x + dx - 3.8, y + dy - 1.25), (x + dx, y + dy), 0x725f45, 1.34);
        p.fill(&oval(x + dx, y + dy, 0.61, 0.67), 0xceb184);
        p.fill(&oval(x + dx - 0.04, y + dy + 0.04, 0.24, 0.26), 0xaa8d64);
    }
}

fn fence(p: &mut Brush, t: &Terrain, a: Pt, b: Pt, count: u32) {
    let face = if t.snow { 0xd8cfbd } else { 0xc3a77a };
    p.line((a.0, a.1 - 2.1), (b.0, b.1 - 1.72), 0x84795d, 0.67);
    p.line((a.0, a.1 - 2.43), (b.0, b.1 - 2.05), face, 0.35);
    p.line((a.0, a.1 - 1.0), (b.0, b.1 - 0.87), 0x94846a, 0.49);
    for i in 0..=count {
        let r = i as f32 / count as f32;
        let (x, y) = (a.0 + r * (b.0 - a.0), a.1 + r * (b.1 - a.1));
        p.line((x, y), (x - 0.26, y - 3.25 + r * 0.32), 0x8b7a60, 0.66);
        p.line((x - 0.2, y - 0.75), (x - 0.4, y - 3.22 + r * 0.32), face, 0.34);
    }
}

fn cattle(p: &mut Brush, x: f32, y: f32, s: f32, color: Rgb) {
    p.tint(&oval(x + 0.9 * s, y + 0.3 * s, 3.1 * s, 0.68 * s), 0x506047, 101);
    p.fill(&oval(x, y - 2.65 * s, 2.31 * s, 1.31 * s), color);
    p.poly(&[(x + 1.4 * s, y - 2.9 * s), (x + 2.9 * s, y - 2.55 * s), (x + 3.1 * s, y - 1.72 * s), (x + 1.99 * s, y - 1.42 * s)], color);
    for (dx, lean) in [(-1.15, -0.26), (1.16, 0.3)] {
        p.line((x + dx * s, y - 2.19 * s), (x + (dx + lean) * s, y - 0.17 * s), 0x6e5945, 0.49 * s);
    }
    p.stroke(&curve((x - 1.5 * s, y - 3.18 * s), &[Q(x - 0.61 * s, y - 4.0 * s, x + 1.19 * s, y - 3.48 * s)], false), 0xe4c7a0, 192, 0.37 * s);
}

fn reeds(p: &mut Brush, t: &Terrain, x: f32, y: f32, h: f32) {
    for (dx, lift, lean) in [(-0.4, 0.72, -0.38), (-0.22, 0.93, -0.13), (0.0, 1.0, 0.21), (0.26, 0.68, 0.28), (0.44, 0.50, 0.33)] {
        let foot = x + h * dx;
        let reed = curve((foot, y + h * 0.15 * dx), &[Q(foot + lean * h * 0.16, y - lift * h * 0.62, foot + lean * h, y - h * lift)], false);
        p.stroke(&reed, if t.snow { 0x938670 } else { t.bank[1] }, 198, 0.26 + h * 0.026);
    }
    p.stroke(&curve((x + h * 0.3, y - h * 0.35), &[Q(x + h * 0.47, y - h * 0.63, x + h * 0.59, y - h * 0.74)], false), if t.snow { 0xe6ddca } else { t.field[3] }, 218, 0.41);
}

fn boat(p: &mut Brush, x: f32, y: f32) {
    p.tint(&oval(x + 6.1, y + 0.8, 8.07, 1.13), 0x274d58, 123);
    let hull = curve((x - 1.8, y + 0.22), &[
        Q(x + 4.5, y - 3.54, x + 14.7, y - 2.58),
        Q(x + 10.15, y + 1.21, x + 0.6, y + 1.67), L(x - 1.8, y + 0.22),
    ], true);
    p.fill(&hull, 0x685648);
    p.stroke(&curve((x - 1.4, y + 0.16), &[Q(x + 6.0, y - 2.51, x + 14.5, y - 2.6)], false), 0xd7b384, 250, 0.59);
    p.line((x + 5.3, y - 1.41), (x + 6.8, y - 0.16), 0xb18e68, 0.55);
    p.line((x + 8.8, y - 3.06), (x + 11.5, y + 1.28), 0xd6ba90, 0.35);
}
