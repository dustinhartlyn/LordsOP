//! Standalone village layout. A card's painted rectangle is also its click
//! and drop target; the centres of the painted groups are the band hit points.

use l2_kingdom::county::County;
use l2_kingdom::tables::{Commodity, JOB_IDLE_TOWNSFOLK, JOB_IRON_MINING};
use l2_view::{Canvas, village as vill};
use super::{Phase, VillageScreen};
use crate::input::Rect;
use crate::screen::Ctx;
use crate::shell::{Face, Pen};

#[path = "panorama.rs"]
mod panorama;

const INK: u8 = 129;
const PANEL: u8 = 9;
const CARD: u8 = 10;
const EDGE: u8 = 12;
const MUTED: u8 = 15;
const PAPER: u8 = 127;
const TEAL: u8 = 71;
const GOLD: u8 = 39;

const PANEL_RECT: Rect = Rect::new(16, 36, 448, 428);
const FIRST_X: i32 = 30;
const PITCH_X: i32 = 106;
const PITCH_Y: i32 = 81;
const CARD_W: i32 = 98;
const CARD_H: i32 = 73;
const FIRST_Y: i32 = 240;
const WORK_AREA: Rect = Rect::new(FIRST_X, FIRST_Y, 3 * PITCH_X + CARD_W, PITCH_Y + CARD_H);
const SCENE_TOP: i32 = 100;
const SCENE_BOTTOM: i32 = 219;
pub(super) const CLOSE: Rect = Rect::new(376, 56, 72, 26);

const ORDER: [usize; vill::CLUSTER_COUNT] = [1, 2, 7, 6, 4, 0, 5, 3];
const RECTS: [Rect; vill::CLUSTER_COUNT] = {
    let mut rects = [Rect::new(0, 0, 0, 0); vill::CLUSTER_COUNT];
    let mut i = 0;
    while i < ORDER.len() {
        rects[ORDER[i]] = Rect::new(
            FIRST_X + (i % 4) as i32 * PITCH_X,
            FIRST_Y + (i / 4) as i32 * PITCH_Y,
            CARD_W,
            CARD_H,
        );
        i += 1;
    }
    rects
};

pub(super) fn site_at(x: i32, y: i32) -> Option<usize> {
    RECTS.iter().position(|r| r.contains(x, y))
}

pub(super) fn site_rect(cluster: usize) -> Rect {
    RECTS[cluster]
}

pub(super) fn in_work_area(x: i32, y: i32) -> bool {
    WORK_AREA.contains(x, y)
}

pub(super) fn group_point(cluster: usize, slot: usize) -> (i32, i32) {
    let r = RECTS[cluster];
    (r.x + 58 + 7 * (slot % 5) as i32, r.y + 27 + 5 * (slot / 5) as i32)
}

fn industry(cluster: usize, slot: usize) -> Option<Commodity> {
    match cluster {
        0 if slot == JOB_IRON_MINING => Some(Commodity::Iron),
        0 => Some(Commodity::Stone),
        4 => Some(Commodity::Wood),
        5 => Some(Commodity::Weapons),
        _ => None,
    }
}

fn name(c: &County, cluster: usize, slot: usize) -> &'static str {
    match cluster {
        0 if slot == JOB_IRON_MINING => "Iron Mine",
        0 if c.industry[Commodity::Stone.index()].has_resource => "Stone Quarry",
        0 => "Mineral Site",
        1 => "Grain Fields",
        2 => "Cattle Farm",
        3 => "Castle Works",
        4 => "Forest",
        5 => "Smithy",
        6 => "Town Square",
        _ => "Land Clearing",
    }
}

fn fill(canvas: &mut Canvas, r: Rect, color: u8) {
    canvas.fill_rect(r.x, r.y, r.w, r.h, color);
}

impl VillageScreen {
    pub(super) fn draw_planner(&self, ctx: &Ctx, canvas: &mut Canvas) {
        let pen = Pen {
            assets: &ctx.assets.shell,
            ink: &ctx.assets.ink,
            chrome: ctx.assets.chrome.as_ref(),
            shadow: None,
            caps: None,
        };
        let r = PANEL_RECT;
        fill(canvas, Rect::new(r.x + 3, r.y + 4, r.w, r.h), INK);
        fill(canvas, r, PANEL);
        let owned = ctx.game.is_players(self.county);
        let county = self.county_ref(ctx);
        let has_grid = ctx.assets.village.as_ref().is_some_and(vill::VillageArt::has_grid);
        if let Some(c) = county.filter(|_| canvas.has_detail()) {
            let weather = ctx.game.kingdom.options.advanced_farming.then_some(c.weather);
            panorama::draw(
                canvas,
                SCENE_TOP,
                ctx.game.kingdom.season,
                weather,
                Self::resource_flags(c),
            );
        } else if let (Some(c), Some(art)) = (county, ctx.assets.village.as_ref()) {
            art.draw_scene(canvas, SCENE_TOP);
            if ctx.game.kingdom.options.advanced_farming {
                art.draw_tops(canvas, c.weather.index() as usize);
            }
            let resources = Self::resource_flags(c);
            art.draw_resources(canvas, resources, SCENE_TOP);
            art.draw_animations(canvas, resources, SCENE_TOP, &self.clock);
        }
        // Crop the scene to its panoramic strip. The sides connect the inset
        // painting to the rest of the planner without hiding the campaign UI.
        fill(canvas, Rect::new(r.x, r.y, r.w, SCENE_TOP - r.y), PANEL);
        fill(canvas, Rect::new(r.x, SCENE_BOTTOM, r.w, r.y + r.h - SCENE_BOTTOM), PANEL);
        canvas.fill_rect(FIRST_X, SCENE_TOP, vill::SCENE_X - FIRST_X, SCENE_BOTTOM - SCENE_TOP, INK);
        canvas.fill_rect(vill::SCENE_X + vill::SCENE_W, SCENE_TOP, 21, SCENE_BOTTOM - SCENE_TOP, INK);
        // A small compass on the dark atlas rail.
        pen.body(canvas, 44, 121, "N", MUTED);
        canvas.fill_rect(46, 140, 1, 24, MUTED);
        canvas.fill_rect(37, 152, 19, 1, MUTED);
        canvas.fill_rect(45, 148, 3, 9, TEAL);
        canvas.fill_rect(42, 151, 9, 3, TEAL);
        canvas.fill_rect(46, 152, 1, 1, PAPER);
        pen.outline(canvas, FIRST_X, SCENE_TOP - 1, WORK_AREA.w + 2, SCENE_BOTTOM - SCENE_TOP + 2, EDGE);
        pen.outline(canvas, r.x, r.y, r.w, r.h, EDGE);
        canvas.fill_rect(r.x + 1, r.y + 1, r.w - 2, 2, TEAL);

        pen.body(canvas, FIRST_X, 47, if owned { "VILLAGE / WORKFORCE" } else { "VILLAGE / VIEW ONLY" }, TEAL);
        let title = crate::screens::county::county_name(ctx, self.county);
        pen.heading(canvas, FIRST_X, 65, &title, PAPER);
        let hovered = CLOSE.contains(self.pointer.0, self.pointer.1);
        fill(canvas, CLOSE, if hovered { GOLD } else { EDGE });
        pen.body_centred(canvas, CLOSE.x, CLOSE.y + 7, CLOSE.w, "Close", if hovered { INK } else { PAPER });
        pen.body(canvas, FIRST_X, 224, "Work sites", PAPER);
        if let Some(c) = county {
            pen.number_in(Face::Body, canvas, 311, 224, c.pop_band, ' ', " workers per group", MUTED);
            fill(canvas, Rect::new(vill::SCENE_X + 7, 186, 128, 24), INK);
            pen.number_in(Face::Body, canvas, vill::SCENE_X + 11, 193, c.population, ' ', " residents", PAPER);
            if ctx.game.kingdom.options.advanced_farming {
                fill(canvas, Rect::new(323, 186, 97, 24), INK);
                pen.body_centred(canvas, 323, 193, 97, c.weather.name(), PAPER);
            }
            self.draw_sites(canvas, &pen, c, owned, has_grid);
        }
        if self.phase == Phase::Band {
            if let Some((x0, y0, x1, y1)) = self.band() {
                let x = x0.max(WORK_AREA.x);
                let y = y0.max(WORK_AREA.y);
                let right = x1.min(WORK_AREA.x + WORK_AREA.w - 1);
                let bottom = y1.min(WORK_AREA.y + WORK_AREA.h - 1);
                pen.outline(canvas, x, y, right - x + 1, bottom - y + 1, GOLD);
            }
        }
        self.draw_planner_help(canvas, &pen, county, owned, has_grid);
    }

    fn draw_sites(&self, canvas: &mut Canvas, pen: &Pen<'_>, c: &County, owned: bool, has_grid: bool) {
        let icons = Self::icons(c);
        let slots = Self::slots(c);
        let (quarry, mine) = Self::resources(c);
        for (cluster, &r) in RECTS.iter().enumerate() {
            let slot = slots[cluster];
            let available = vill::cluster_is_clickable(cluster, quarry, mine);
            let hot = r.contains(self.pointer.0, self.pointer.1) && has_grid
                && (self.phase == Phase::Carry || self.phase == Phase::Idle && available);
            let selected = self.drag_cluster == cluster + 1;
            let site = industry(cluster, slot).map(|i| &c.industry[i.index()]);
            let missing = site.is_some_and(|s| !s.has_resource);
            let workers = c.labour[slot];
            let (status, color) = if missing {
                ("No resource", MUTED)
            } else if site.is_some_and(|s| s.disabled_seasons > 0) {
                ("Paused", GOLD)
            } else if site.is_some_and(|s| !s.enabled) || cluster == 3 && !c.castle_switch {
                ("Inactive", GOLD)
            } else if slot == JOB_IDLE_TOWNSFOLK {
                ("Unassigned", TEAL)
            } else if workers < c.labour_wanted[slot] {
                ("Needs workers", GOLD)
            } else if workers > c.labour_useful[slot] {
                ("Spare workers", GOLD)
            } else if workers == 0 {
                ("No workers", MUTED)
            } else {
                ("Staffed", TEAL)
            };
            fill(canvas, r, if missing { INK } else { CARD });
            pen.outline(canvas, r.x, r.y, r.w, r.h, if selected || hot && self.phase == Phase::Carry { GOLD } else if hot { TEAL } else { EDGE });
            pen.body(canvas, r.x + 7, r.y + 6, name(c, cluster, slot), if missing { MUTED } else { PAPER });
            let count_face = if workers < 10_000 { Face::Heading } else { Face::Body };
            pen.number_in(count_face, canvas, r.x + 3, r.y + 26, workers, ' ', "", PAPER);
            canvas.fill_rect(r.x + 7, r.y + 54, r.w - 14, 1, EDGE);
            pen.body(canvas, r.x + 7, r.y + 58, status, color);
            for (group, &value) in icons[cluster].iter().enumerate() {
                let (x, y) = group_point(cluster, group);
                let color = match value {
                    0 => continue,
                    vill::ICON_SHORTFALL => {
                        pen.outline(canvas, x - 2, y - 1, 5, 3, EDGE);
                        continue;
                    }
                    _ if selected && self.selected[group] => GOLD,
                    _ if !owned || missing => MUTED,
                    _ => TEAL,
                };
                canvas.fill_rect(x - 2, y - 1, 5, 3, color);
            }
        }
    }

    fn draw_planner_help(&self, canvas: &mut Canvas, pen: &Pen<'_>, county: Option<&County>, owned: bool, has_grid: bool) {
        let r = Rect::new(FIRST_X, 403, WORK_AREA.w, 48);
        fill(canvas, r, INK);
        canvas.fill_rect(r.x, r.y, 2, r.h, if self.phase == Phase::Idle { TEAL } else { GOLD });
        let x = r.x + 9;
        if !has_grid {
            pen.body(canvas, x, 410, "Work site controls unavailable", GOLD);
            pen.body(canvas, x, 429, "The village work site data could not be loaded.", MUTED);
        } else if !owned {
            pen.body(canvas, x, 410, "This county is not yours to manage", GOLD);
            pen.body(canvas, x, 429, "Click an available site to inspect it.", PAPER);
        } else if self.phase == Phase::Carry {
            let count = county.map_or(0, |c| {
                let job = Self::slots(c)[self.drag_cluster - 1];
                c.labour[job].min(self.drag_count.saturating_mul(c.pop_band))
            });
            pen.number_in(Face::Body, canvas, x - 4, 409, count, ' ', " workers selected", GOLD);
            let detail = match site_at(self.pointer.0, self.pointer.1) {
                Some(cluster) if cluster + 1 == self.drag_cluster => "Click here to put them back.",
                Some(cluster) if county.is_some_and(|c| {
                    industry(cluster, Self::slots(c)[cluster]).is_some_and(|i| !c.industry[i.index()].has_resource)
                }) => "No resource here. Choose another site or right-click to cancel.",
                _ => "Click a destination to assign them. Right-click to cancel.",
            };
            pen.body(canvas, x, 428, detail, PAPER);
        } else if self.phase == Phase::Band {
            pen.body(canvas, x, 410, "Select groups from one work site", GOLD);
            pen.body(canvas, x, 429, "Release to pick them up, then click their destination.", PAPER);
        } else {
            let first = if self.status.is_empty() {
                "Click a site for details. Double-click to balance staffing."
            } else {
                &self.status
            };
            pen.body(canvas, x, 410, first, if self.status.is_empty() { PAPER } else { TEAL });
            pen.body(canvas, x, 429, "Drag around one site's groups, release, then click a destination.", MUTED);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Assets;
    use crate::input::Event;
    use crate::screen::{Screen, ScreenId, Transition};
    use l2_kingdom::tables::{JOB_CATTLE_FARMING, JOB_GRAIN_FARMING, JOB_WOOD_CUTTING};
    use l2_mods::vfs::Vfs;

    #[test]
    fn all_painted_groups_hit_their_own_cards() {
        for (cluster, r) in RECTS.iter().enumerate() {
            assert_eq!(site_at(r.x, r.y), Some(cluster));
            for group in 0..vill::ICONS_PER_CLUSTER {
                let (x, y) = group_point(cluster, group);
                assert!(in_work_area(x, y));
                assert_eq!(site_at(x, y), Some(cluster));
            }
        }
        assert!(site_at(CLOSE.x, CLOSE.y).is_none());
        assert!(site_at(FIRST_X + CARD_W, FIRST_Y).is_none(), "the gutter isn't a site");
    }

    #[test]
    fn standalone_card_clicks_and_bands_dispatch_existing_village_actions() {
        let (mut game, _) = crate::screens::village::tests::world();
        let assets = Assets::load_original(&Vfs::new()).unwrap();
        let mut ctx = Ctx { game: &mut game, assets: &assets };
        let mut screen = VillageScreen::new(1);

        let grain = (55, 252);
        screen.handle(Event::Click { x: grain.0, y: grain.1 }, &mut ctx);
        screen.handle(Event::Release { x: grain.0, y: grain.1 }, &mut ctx);
        for _ in 1..VillageScreen::CLICK_SETTLE_TICKS {
            assert_eq!(screen.update(&mut ctx), Transition::Stay);
        }
        assert_eq!(screen.update(&mut ctx), Transition::Push(ScreenId::Job(1, JOB_GRAIN_FARMING)));

        screen.handle(Event::Click { x: 189, y: 264 }, &mut ctx);
        screen.handle(Event::Pointer { x: 229, y: 291 }, &mut ctx);
        assert_eq!(screen.phase(), Phase::Band);
        screen.handle(Event::Release { x: 229, y: 291 }, &mut ctx);
        assert_eq!(screen.phase(), Phase::Carry);
        assert_eq!(ctx.game.kingdom.counties[1].labour[JOB_CATTLE_FARMING], 218);

        screen.handle(Event::Click { x: 54, y: 333 }, &mut ctx);
        assert_eq!(screen.phase(), Phase::Idle);
        assert_eq!(ctx.game.kingdom.counties[1].labour[JOB_CATTLE_FARMING], 0);
        assert_eq!(ctx.game.kingdom.counties[1].labour[JOB_WOOD_CUTTING], 435);
        assert_eq!(screen.handle(Event::Release { x: 412, y: 69 }, &mut ctx), Transition::Pop);
    }
}
