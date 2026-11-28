use std::sync::OnceLock;
use l2_view::{Canvas, Clip, detail::SCALE};
use tiny_skia::{Color, FillRule, GradientStop, LineCap, LinearGradient, Paint, Path, PathBuilder,
    Pixmap, Point, RadialGradient, Rect, SpreadMode, Stroke, Transform};

type Pt = (f32, f32);
type Rgb = [u8;3];
const BANNER: usize = 25;
const CART: usize = 44;
static FLAGS: [OnceLock<Pixmap>;40] = [const { OnceLock::new() };40];
static CARTS: [OnceLock<Pixmap>;2] = [const { OnceLock::new() };2];

pub(super) fn flag(canvas: &mut Canvas, (x,y): (i32,i32), frame: usize, clip: Clip) -> bool {
    let Some(swatch) = FLAGS.get(frame) else { return false };
    let image = swatch.get_or_init(|| banner(frame));
    canvas.blit_detail(image.data(), BANNER * SCALE, x, y, clip);
    true
}

pub(super) fn cart(canvas: &mut Canvas, (x,y): (i32,i32), left: bool, clip: Clip) {
    let image = CARTS[left as usize].get_or_init(|| wagon(left));
    canvas.blit_detail(image.data(), CART * SCALE, x - CART as i32 / 2, y - 25, clip);
}

fn scale() -> Transform { Transform::from_scale(SCALE as f32, SCALE as f32) }
fn canvas(w: usize, h: usize) -> Pixmap { Pixmap::new((w*SCALE) as u32,(h*SCALE) as u32).unwrap() }
fn rgb([r,g,b]:Rgb) -> Color { Color::from_rgba8(r,g,b,255) }

fn banner(frame: usize) -> Pixmap {
    let mut p = canvas(BANNER, 39);
    let palette = [([205,84,60], [135,52,42]), ([223,177,76],[142,105,46]),
        ([137,159,164],[68,89,98]), ([175,121,162],[102,65,111]), ([104,154,187],[61,94,131])];
    let (sun, dark) = palette[frame / 8];
    let wave = [0.0, 0.5, 1.0, 0.58, 0.0, -0.58, -1.0, -0.48][frame % 8];
    line(&mut p,(3.3,2.3),(3.3,34.2),1.1,[116,85,51]);
    line(&mut p,(2.95,2.3),(2.95,33.7),0.44,[211,179,117]);
    let mut b=PathBuilder::new();
    b.move_to(3.9,3.5); b.cubic_to(10.,1.8+wave,16.,6.8-wave,22.,4.8+wave*0.6);
    b.line_to(19.7,9.1+wave*0.6); b.line_to(22.8,13.+wave*0.6);
    b.cubic_to(15.,14.1-wave,11.,9.7+wave,3.9,13.); b.close();
    shape(&mut p,&b.finish().unwrap(),(5.,3.),(19.,13.),sun,dark,scale());
    line(&mut p,(9.8,5.5+wave*0.4),(10.0,11.2+wave*0.4),0.56,[236,213,157]);
    line(&mut p,(7.7,8.1+wave*0.4),(12.7,8.0+wave*0.4),0.56,[236,213,157]);
    p
}

fn wagon(left: bool) -> Pixmap {
    let mut p=canvas(CART,30);
    let s = SCALE as f32;
    let size = 0.63;
    let center = CART as f32 * 0.5;
    let dir = if left { -size } else { size };
    let t = Transform::from_row(dir * s, 0., 0., size * s,
        (center - dir * center) * s, (23. - 23. * size) * s);
    let shadow=PathBuilder::from_circle(0.,0.,1.).unwrap();
    let c=|a| Color::from_rgba8(35,49,39,a);
    let paint=Paint { shader: RadialGradient::new(Point::zero(),Point::zero(),1.,vec![
        GradientStop::new(0.,c(140)),GradientStop::new(0.62,c(68)),GradientStop::new(1.,c(0)),
    ],SpreadMode::Pad,Transform::identity()).unwrap(),..Paint::default() };
    p.fill_path(&shadow,&paint,FillRule::Winding,Transform::from_row(19.,0.,0.,4.,23.,23.).post_concat(t),None);
    let ellipse=|p:&mut Pixmap,(x,y):Pt,(rx,ry):Pt, lit,dark| {
        let path=Rect::from_xywh(x-rx,y-ry,rx*2.,ry*2.).and_then(PathBuilder::from_oval).unwrap();
        shape(p,&path,(x-rx,y-ry),(x+rx,y+ry),lit,dark,t);
    };
    let panel=|p:&mut Pixmap, coords:&[Pt], a,b| {
        let mut path=PathBuilder::new(); path.move_to(coords[0].0,coords[0].1);
        for &(x,y) in &coords[1..]{path.line_to(x,y);} path.close();
        shape(p,&path.finish().unwrap(),coords[0],coords[2],a,b,t);
    };
    let rail=|p:&mut Pixmap,a,b,w,c| line_at(p,a,b,w,c,t);
    ellipse(&mut p,(31.2,17.3),(7.7,3.85),[148,104,71],[88,61,46]);
    panel(&mut p,&[(32.,17.),(34.,11.),(37.6,10.),(36.4,17.)],[147,104,68],[91,65,49]);
    ellipse(&mut p,(38.0,12.3),(3.75,2.0),[145,105,72],[82,58,42]);
    for (x,y) in [(28.,19.6),(32.,19.6),(35.2,18.)] { rail(&mut p,(x,y),(x+1.6,y+4.),0.94,[76,57,44]); }
    rail(&mut p,(19.,19.),(34.,16.7),1.,[121,89,55]);
    panel(&mut p,&[(4.,14.),(18.2,13.),(23.,20.5),(5.7,21.4)],[167,122,73],[101,70,45]);
    let mut hood=PathBuilder::new();
    hood.move_to(3.,14.8); hood.quad_to(4.,5.4,9.,5.7); hood.line_to(17.,5.8);
    hood.quad_to(20.8,6.7,22.,14.3); hood.quad_to(12.,16.0,3.,14.8);hood.close();
    shape(&mut p,&hood.finish().unwrap(),(9.,6.),(17.,15.),[234,215,170],[173,142,91],t);
    for x in [8.2,18.7] {
        ellipse(&mut p,(x,21.4),(3.3,3.55),[107,86,65],[66,49,39]);
        ellipse(&mut p,(x,21.4),(2.42,2.52),[147,117,76],[90,68,48]);
        rail(&mut p,(x-2.1,21.4),(x+2.1,21.4),0.55,[184,140,90]);
        rail(&mut p,(x,19.4),(x,23.4),0.55,[182,140,93]);
    }
    p
}

fn shape(p:&mut Pixmap,path:&Path,from:Pt,to:Pt,a:Rgb,b:Rgb,t:Transform){
    let paint=Paint{shader:LinearGradient::new(Point::from_xy(from.0,from.1),Point::from_xy(to.0,to.1),
        vec![GradientStop::new(0.,rgb(a)),GradientStop::new(1.,rgb(b))],SpreadMode::Pad,Transform::identity()).unwrap(),..Paint::default()};
    p.fill_path(path,&paint,FillRule::Winding,t,None);
}
fn line(p:&mut Pixmap,a:Pt,b:Pt,w:f32,c:Rgb){line_at(p,a,b,w,c,scale());}
fn line_at(p:&mut Pixmap,a:Pt,b:Pt,w:f32,c:Rgb,t:Transform){
    let mut bld=PathBuilder::new(); bld.move_to(a.0,a.1);bld.line_to(b.0,b.1);
    let Some(path)=bld.finish() else { return };let mut paint=Paint::default();paint.set_color(rgb(c));
    p.stroke_path(&path,&paint,&Stroke{width:w,line_cap:LineCap::Round,..Stroke::default()},t,None);
}
