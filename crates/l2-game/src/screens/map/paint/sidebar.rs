//! The standalone county dashboard. Paint into the campaign's native input
//! rectangles so the classic map rules and shortcut tooltips still apply.

use std::{fmt, io::Write as _};

use l2_kingdom::{
    county::County,
    tables::{JOB_COUNT, JOB_IDLE_TOWNSFOLK, RATION_NAMES},
};
use l2_view::{
    chrome::{self, MinimapMode},
    text, Canvas,
};

use crate::{
    input::Rect,
    screen::Ctx,
    screens::county,
    shell::font::{Font, Style},
    turn,
};

use super::super::{
    END_TURN_BUTTON, MINIMAP_MODE_BUTTONS, MapScreen, PANEL, SIDEBAR_BUTTONS, SPLIT_SLIDER,
    SidebarAction,
};

// The standalone art palette in original/mod.rs, intentionally used only here
// and only when Assets::is_original() is true.
const INK: u8 = 128;
const SLATE: u8 = 8;
const CARD: u8 = SLATE + 1;
const EDGE: u8 = SLATE + 2;
const MUTED: u8 = SLATE + 6;
const WATER: u8 = 64;
const HOVER: u8 = WATER + 2;
const TEAL: u8 = WATER + 6;
const GOLD: u8 = 39;
const CREAM: u8 = 127;
const CORAL: u8 = 87;
const LILAC: u8 = 102;

const LEFT: i32 = PANEL.x + 10;
const RIGHT: i32 = PANEL.x + PANEL.w - 10;

pub(in crate::screens::map) const VILLAGE_SHORTCUT: Rect = Rect::new(RIGHT - 50, 160, 50, 20);

pub(in crate::screens::map) fn village_shortcut_target(ctx: &Ctx) -> Option<usize> {
    let selected = ctx.game.selected;
    if !ctx.assets.is_original() || !ctx.game.is_players(selected) {
        return None;
    }
    let map = &ctx.game.kingdom.campaign.map;
    // Same first town tile as MapScreen::town, without collecting a Vec every frame.
    map.county.iter().zip(&map.flags).position(|(&county, &flags)| {
        county == selected && flags & l2_kingdom::map::flags::CASTLE != 0
    })
}

const JOB_NAMES: [&str; JOB_COUNT] = [
    "Grain", "Cattle", "Reclaim", "Castle", "Iron", "Stone", "Timber", "Smithy", "Idle",
];

pub(super) fn draw(screen: &MapScreen, canvas: &mut Canvas, ctx: &Ctx) {
    let ui = Dashboard {
        hover: (screen.pointer_in && screen.fading.is_none() && !turn::map_input_shut(ctx.game))
            .then_some(screen.pointer),
        small: ctx.assets.shell.small.as_ref(),
        body: ctx.assets.shell.body.as_ref(),
        title: ctx.assets.shell.heading.as_ref(),
    };
    fill(canvas, PANEL, SLATE);
    canvas.fill_rect(PANEL.x, PANEL.y, 1, PANEL.h, INK);
    ui.minimap(screen, canvas, ctx);
    ui.county(canvas, ctx);
    ui.actions(canvas, ctx);

    if ctx.game.prefs.debug_overlay {
        fit(canvas, ui.small, Rect::new(LEFT, 414, RIGHT - LEFT, 10), &screen.status, MUTED);
    }
}

struct Dashboard<'a> {
    hover: Option<(i32, i32)>,
    small: Option<&'a Font>,
    body: Option<&'a Font>,
    title: Option<&'a Font>,
}

impl Dashboard<'_> {
    fn hovered(&self, r: Rect) -> bool {
        self.hover.is_some_and(|(x, y)| r.contains(x, y))
    }

    fn minimap(&self, screen: &MapScreen, canvas: &mut Canvas, ctx: &Ctx) {
        let hit = chrome::minimap_hit_area();
        let side = chrome::MINIMAP_DIM;
        // The dashboard raster and the county picker use the same origin.
        if let Some(map) = &screen.minimap {
            let game = &ctx.game;
            let mode = screen.minimap_mode;
            // Resolve each county's display colour once, not each raster pixel.
            let mut ramps = [map_ramp(EDGE, false); 256];
            for (id, c) in game.kingdom.counties.iter().enumerate().skip(1) {
                let base = if mode == MinimapMode::Owner {
                    const REALMS: [u8; 6] = [EDGE, 82, 34, 59, 98, 90];
                    let realm = if c.owner == 0 {
                        0
                    } else {
                        chrome::realm_colour(game.realm_colour.get(c.owner as usize).copied().unwrap_or(0))
                    };
                    REALMS[realm as usize]
                } else if c.owner == game.player {
                    let bands = c.minimap_bands_with(game.debug_keys.food_bands);
                    match mode {
                        MinimapMode::Labour => match bands.labour {
                            0 => 84, // shortage: red
                            5 => 100, // unused labour: purple
                            _ => EDGE,
                        },
                        MinimapMode::Food => rating_base(bands.food),
                        MinimapMode::Happiness => rating_base(bands.happiness),
                        MinimapMode::Owner => EDGE,
                    }
                } else {
                    EDGE
                };
                ramps[id] = map_ramp(base, id == game.selected as usize);
            }
            for (i, (&county, &shade)) in map.counties.iter().zip(&map.shades).enumerate() {
                let colour = match shade {
                    0 => WATER,
                    chrome::MINIMAP_SHADE_LO..=chrome::MINIMAP_SHADE_HI =>
                        ramps[county as usize][(shade - chrome::MINIMAP_SHADE_LO) as usize],
                    66 | 67 if county == 0 => WATER + 1,
                    _ => shade,
                };
                let (x, y) = (i % side as usize, i / side as usize);
                canvas.set(hit.x0 as usize + x, hit.y0 as usize + y, colour);
            }
        } else {
            canvas.fill_rect(hit.x0, hit.y0, side, side, WATER);
            centre(canvas, self.body, hit.x0 + side / 2, 81, "Map unavailable", MUTED);
        }
        canvas.fill_rect(hit.x0, hit.y1, side, 1, EDGE);

        for (i, (label, symbol)) in [
            ("Work", WORK), ("Food", FOOD), ("Mood", MOOD),
            if screen.minimap_mode == MinimapMode::Owner { ("Zoom", ZOOM) } else { ("Land", LAND) },
        ].into_iter().enumerate() {
            let r = MINIMAP_MODE_BUTTONS[i];
            let active = MinimapMode::from_button(i) == Some(screen.minimap_mode);
            let available = i == 3 || screen.minimap_mode == MinimapMode::Owner;
            let card = Rect::new(r.x + 1, r.y + if i == 2 { 3 } else { 1 }, r.w - 2, 25);
            let bg = if active || available && self.hovered(r) { HOVER } else { CARD };
            let pen = if active || available { CREAM } else { MUTED };
            tile(canvas, card, bg, if active { TEAL } else { EDGE });
            icon(canvas, card.centre_x(), card.y + 6, symbol, if available || active { TEAL } else { MUTED });
            centre(canvas, self.small, card.centre_x(), card.y + 14, label, pen);
        }
    }

    fn county(&self, canvas: &mut Canvas, ctx: &Ctx) {
        let selected = ctx.game.selected;
        let county = ctx.game.kingdom.counties.get(selected as usize).filter(|_| selected != 0);
        let owned = ctx.game.is_players(selected);
        let (kicker, colour) = match county {
            Some(_) if owned => ("YOUR COUNTY", TEAL),
            Some(c) if c.owner == 0 => ("INDEPENDENT COUNTY", MUTED),
            Some(_) => ("FOREIGN COUNTY", MUTED),
            None => ("COUNTY ATLAS", TEAL),
        };
        canvas.fill_rect(PANEL.x + 4, 159, 2, 19, colour);
        label(canvas, self.small, LEFT, 154, kicker, colour);
        let mut buf = [0; 24];
        let name = if county.is_some() {
            let index = ctx.game.map_slot * 20 + selected as usize;
            let name = ctx.assets.shell.text(crate::message::GROUP_COUNTY, index);
            if name.is_empty() { formatted(&mut buf, format_args!("County {selected}")) } else { name }
        } else {
            "Select a county"
        };
        let name_right = if village_shortcut_target(ctx).is_some() {
            let r = VILLAGE_SHORTCUT;
            let over = self.hovered(r);
            tile(canvas, r, if over { HOVER } else { CARD }, TEAL);
            label(canvas, self.small, r.x + 5, r.y + 5, "Village", CREAM);
            icon(canvas, r.x + r.w - 6, r.y + 10, CHEVRON, TEAL);
            r.x - 5
        } else {
            RIGHT
        };
        if text_width(self.title, name) <= name_right - LEFT {
            label(canvas, self.title, LEFT, 161, name, CREAM);
        } else {
            fit(canvas, self.body, Rect::new(LEFT, 166, name_right - LEFT, 13), name, CREAM);
        }

        let Some(c) = county.filter(|_| owned) else {
            let r = Rect::new(LEFT, 187, RIGHT - LEFT, 55);
            tile(canvas, r, CARD, EDGE);
            let owner = county.and_then(|c| (c.owner != 0).then(|| crate::screens::message::lord_name(ctx, c.owner)));
            let (headline, hint) = match (&owner, county) {
                (Some(owner), _) => (owner.as_str(), "This county is not yours."),
                (_, Some(_)) => ("No ruling lord", "This county is independent."),
                _ => ("No county selected", "Choose one on the minimap."),
            };
            fit(canvas, self.body, Rect::new(r.x + 8, r.y + 8, r.w - 16, 13), headline, CREAM);
            label(canvas, self.small, r.x + 8, r.y + 29, hint, MUTED);
            return;
        };

        self.stat(canvas, county::Panel::Population, "Population", formatted(&mut buf, format_args!("{}", c.population)), CREAM);
        self.stat(canvas, county::Panel::Happiness, "Happiness", formatted(&mut buf, format_args!("{}%", c.happiness)), CREAM);
        self.stat(canvas, county::Panel::Tax, "Tax rate", formatted(&mut buf, format_args!("{}%", c.tax_rate)), CREAM);
        let ration = c.ration_achieved.clamp(0, RATION_NAMES.len() as i32 - 1) as usize;
        let ration_label = ctx.assets.shell.text(21, ration);
        self.stat(canvas, county::Panel::Ration, "Rations",
            if ration_label.is_empty() { RATION_NAMES[ration] } else { ration_label },
            if c.ration_achieved < c.ration_wanted { CORAL } else { CREAM });
        for i in 0..5 {
            let (x, y) = (555, 190 + (4 - i) * 9);
            canvas.fill_rect(x, y, 6, 5, if i <= i32::from(c.health_band) { TEAL } else { EDGE });
        }
        self.workforce(canvas, c);
    }

    fn stat(&self, canvas: &mut Canvas, panel: county::Panel, caption: &str, value: &str, pen: u8) {
        let hit = panel.strip_hotspot();
        let r = Rect::new(hit.x, hit.y + 1, hit.w, hit.h - 2);
        let over = self.hovered(hit);
        tile(canvas, r, if over { HOVER } else { CARD }, if over { TEAL } else { EDGE });
        label(canvas, self.small, r.x + 5, r.y + 3, caption, MUTED);
        fit(canvas, self.body, Rect::new(r.x + 5, r.y + 13, r.w - 14, 13), value, pen);
        icon(canvas, r.x + r.w - 6, r.y + 17, CHEVRON, if over { CREAM } else { TEAL });
    }

    fn workforce(&self, canvas: &mut Canvas, c: &County) {
        let mut buf = [0; 24];
        let idle = c.labour[JOB_IDLE_TOWNSFOLK];
        label(canvas, self.body, LEFT, 243, "Workforce", CREAM);
        right(canvas, self.small, RIGHT, 245, formatted(&mut buf, format_args!("{idle} idle")),
            if idle > 0 { LILAC } else { MUTED });

        let slider = Rect::new(LEFT, SPLIT_SLIDER.y, RIGHT - LEFT, SPLIT_SLIDER.h - 1);
        let minus = Rect::new(LEFT + 3, slider.y + 3, 33, 20);
        let plus = Rect::new(594, slider.y + 3, RIGHT - 597, 20);
        let share = c.industry_share.clamp(0, 100);
        let pressed = self.hovered(SPLIT_SLIDER);
        tile(canvas, slider, CARD, if pressed { TEAL } else { EDGE });
        for (r, mark) in [(minus, MINUS), (plus, PLUS)] {
            let over = self.hovered(r);
            tile(canvas, r, if over { HOVER } else { SLATE }, EDGE);
            icon(canvas, r.centre_x(), r.y + 10, mark, if over { CREAM } else { TEAL });
        }
        // The endpoints agree with split_from_click: x 531 represents 0%, x 581 100%.
        let thumb = 531 + share / 2;
        canvas.fill_rect(531, slider.y + 12, 51, 3, EDGE);
        canvas.fill_rect(531, slider.y + 12, thumb - 531, 3, TEAL);
        tile(canvas, Rect::new(thumb - 3, slider.y + 8, 7, 11), GOLD, CREAM);
        label(canvas, self.small, slider.x + 5, slider.y + 26,
            formatted(&mut buf, format_args!("Farm {}%", 100 - share)), CREAM);
        right(canvas, self.small, slider.x + slider.w - 5, slider.y + 26,
            formatted(&mut buf, format_args!("Industry {share}%")), CREAM);

        let farm = county::farm_rows(c);
        let industry = county::industry_rows(c);
        self.jobs(canvas, c, &farm, county::farm_pitch(farm.len()),
            Rect::new(PANEL.x, chrome::PANEL_OWN_C_Y, 560 - PANEL.x, chrome::PANEL_STATUS_Y - chrome::PANEL_OWN_C_Y),
            "Farm jobs");
        self.jobs(canvas, c, &industry, county::industry_pitch(industry.len()),
            Rect::new(560, chrome::PANEL_OWN_C_Y, PANEL.x + PANEL.w - 560, chrome::PANEL_STATUS_Y - chrome::PANEL_OWN_C_Y),
            "Industry jobs");
    }

    fn jobs(&self, canvas: &mut Canvas, c: &County, slots: &[usize], pitch: i32, col: Rect, heading: &str) {
        let x = col.x + 6;
        label(canvas, self.small, x, col.y - 5, heading, TEAL);
        if slots.is_empty() {
            label(canvas, self.small, x, col.y + 11, "No jobs", MUTED);
        }
        let mut buf = [0; 24];
        for (i, &slot) in slots.iter().enumerate() {
            let y = col.y + i as i32 * pitch;
            let hit = Rect::new(col.x, y, col.w, pitch.min(col.y + col.h - y));
            let over = self.hovered(hit);
            let workers = c.labour[slot];
            let accent = if workers > c.labour_useful[slot] { LILAC }
                else if workers < c.labour_wanted[slot] { CORAL }
                else { TEAL };
            let bg = if over { HOVER } else { CARD };
            if hit.h < 12 {
                // Only the fifth industry job can land in the original's 8px final row.
                fill(canvas, Rect::new(x, y, col.w - 12, hit.h), bg);
                label(canvas, self.small, x + 5, y - 1, JOB_NAMES[slot], CREAM);
                icon(canvas, x + col.w - 19, y + 4, CHEVRON, accent);
            } else {
                let r = Rect::new(x, y + 5, col.w - 12, (hit.h - 7).min(34));
                tile(canvas, r, bg, if over { TEAL } else { EDGE });
                let mid = r.y + r.h / 2;
                canvas.fill_rect(r.x + 2, mid - 5, 2, 11, accent);
                label(canvas, self.body, r.x + 7, mid - 11, JOB_NAMES[slot], CREAM);
                fit(canvas, self.small, Rect::new(r.x + 7, mid + 1, r.w - 13, 10),
                    formatted(&mut buf, format_args!("{workers} staff")), MUTED);
                icon(canvas, r.x + r.w - 6, mid, CHEVRON, if over { CREAM } else { accent });
            }
        }
    }

    fn actions(&self, canvas: &mut Canvas, ctx: &Ctx) {
        let own = ctx.game.is_players(ctx.game.selected);
        for (button, (caption, symbol)) in SIDEBAR_BUTTONS.into_iter().zip([
            ("Army", ARMY), ("Court", COURT), ("Send", SUPPLY), ("Keep", CASTLE), ("Lords", LORDS),
        ]) {
            let SidebarAction::Screen(id) = button.action;
            let enabled = own || !matches!(id, 0x17 | 0x18 | 0x1B);
            let hit = button.rect();
            let r = Rect::new(hit.x + 1, hit.y + 1, hit.w - 2, hit.h - 2);
            let over = enabled && self.hovered(hit);
            tile(canvas, r, if over { HOVER } else { CARD }, if over { TEAL } else { EDGE });
            icon(canvas, r.centre_x(), r.y + 8, symbol, if enabled { TEAL } else { MUTED });
            centre(canvas, self.small, r.centre_x(), r.y + 16, caption, if enabled { CREAM } else { MUTED });
        }
        let hit = END_TURN_BUTTON;
        let r = Rect::new(hit.x + 3, hit.y + 1, hit.w - 5, hit.h - 2);
        let waiting = turn::turn_in_flight(ctx.game);
        tile(canvas, r, if waiting { CARD } else { GOLD }, if self.hovered(hit) { CREAM } else if waiting { EDGE } else { GOLD });
        centre(canvas, self.body, r.centre_x(), r.y + 2,
            if waiting { "Season ending" } else { "End season" }, if waiting { MUTED } else { INK });
        if !waiting {
            icon(canvas, r.x + r.w - 14, r.y + 8, ARROW, INK);
        }
    }
}

fn map_ramp(base: u8, selected: bool) -> [u8; 4] {
    [if selected { GOLD } else { base - 1 }, base, base + 1, base + 2]
}

fn rating_base(band: u8) -> u8 {
    [84, 108, 35, 67, 68, 69].get(band as usize).copied().unwrap_or(EDGE)
}

fn formatted<'a>(buf: &'a mut [u8], args: fmt::Arguments<'_>) -> &'a str {
    let cap = buf.len();
    let mut tail = &mut *buf;
    tail.write_fmt(args).expect("sidebar number fits in its stack buffer");
    let len = cap - tail.len();
    std::str::from_utf8(&buf[..len]).expect("formatting yields UTF-8")
}

fn text_width(face: Option<&Font>, caption: &str) -> i32 {
    face.map_or_else(|| text::width(caption), |font| font.width(caption))
}

fn label(canvas: &mut Canvas, face: Option<&Font>, x: i32, y: i32, caption: &str, colour: u8) {
    if let Some(font) = face {
        let style = Style { colour, shadow: None, caps: None };
        if font.draw_outline(canvas, x, y, caption, style, None).is_none() {
            font.draw(canvas, x, y, caption, &style);
        }
    } else {
        text::draw(canvas, x, y + 1, caption, colour);
    }
}

fn centre(canvas: &mut Canvas, face: Option<&Font>, x: i32, y: i32, caption: &str, colour: u8) {
    label(canvas, face, x - text_width(face, caption) / 2, y, caption, colour);
}

fn right(canvas: &mut Canvas, face: Option<&Font>, x: i32, y: i32, caption: &str, colour: u8) {
    label(canvas, face, x - text_width(face, caption), y, caption, colour);
}

fn fit(canvas: &mut Canvas, face: Option<&Font>, area: Rect, caption: &str, colour: u8) {
    if text_width(face, caption) <= area.w {
        label(canvas, face, area.x, area.y, caption, colour);
        return;
    }
    let max = area.w - text_width(face, "...");
    let mut width = 0;
    for (i, c) in caption.char_indices() {
        let advance = text_width(face, &caption[i..i + c.len_utf8()]);
        if width + advance > max {
            label(canvas, face, area.x, area.y, &caption[..i], colour);
            label(canvas, face, area.x + width, area.y, "...", colour);
            return;
        }
        width += advance;
    }
}

fn fill(canvas: &mut Canvas, r: Rect, ink: u8) {
    canvas.fill_rect(r.x, r.y, r.w, r.h, ink);
}

fn tile(canvas: &mut Canvas, r: Rect, bg: u8, edge: u8) {
    canvas.fill_rect(r.x + 1, r.y, r.w - 2, 1, edge);
    canvas.fill_rect(r.x + 1, r.y + r.h - 1, r.w - 2, 1, edge);
    canvas.fill_rect(r.x, r.y + 1, 1, r.h - 2, edge);
    canvas.fill_rect(r.x + r.w - 1, r.y + 1, 1, r.h - 2, edge);
    canvas.fill_rect(r.x + 1, r.y + 1, r.w - 2, r.h - 2, bg);
}

type Icon = &'static [(i32, i32, i32, i32)];
const WORK: Icon = &[(-4, 3, 4, 3), (-3, 1, -3, -1), (0, 1, 0, -4), (3, 1, 3, -2)];
const FOOD: Icon = &[(-5, 2, -3, -2), (-3, -2, 3, -2), (3, -2, 5, 2), (-5, 2, 5, 2), (-1, -1, 0, 0), (2, -1, 3, 0)];
const MOOD: Icon = &[(-4, -2, -4, 0), (-4, -2, -2, -3), (-2, -3, 0, -1), (0, -1, 2, -3), (2, -3, 4, -2), (4, -2, 4, 0), (-4, 0, 0, 4), (0, 4, 4, 0)];
const ZOOM: Icon = &[(-4, -4, 0, -4), (0, -4, 0, 0), (0, 0, -4, 0), (-4, 0, -4, -4), (1, 1, 4, 4)];
const LAND: Icon = &[(-3, 4, -3, -4), (-3, -4, 4, -4), (4, -4, 2, -2), (2, -2, 4, 0), (4, 0, -3, 0)];
const ARMY: Icon = &[(-4, 4, 5, -5), (-2, 0, 2, 4), (2, -5, 5, -5), (5, -5, 5, -2)];
const COURT: Icon = &[(-5, -1, 0, -5), (0, -5, 5, -1), (-5, -1, 5, -1), (-5, 4, 5, 4), (-3, 0, -3, 3), (0, 0, 0, 3), (3, 0, 3, 3)];
const SUPPLY: Icon = &[(-5, -3, 5, -3), (-5, -3, -5, 4), (5, -3, 5, 4), (-5, 4, 5, 4), (-5, 0, 5, 0), (-1, 1, 1, 1)];
const CASTLE: Icon = &[(-5, 4, -5, -4), (-5, -4, -2, -4), (-2, -4, -2, -1), (-2, -1, 2, -1), (2, -1, 2, -4), (2, -4, 5, -4), (5, -4, 5, 4), (-5, 4, 5, 4), (0, 2, 0, 4)];
const LORDS: Icon = &[(-5, -3, -5, 2), (-5, -3, -2, -3), (-5, 2, -2, 4), (-2, 4, 0, 2), (1, -4, 5, -4), (1, -4, 1, 1), (5, -4, 5, 1), (1, 1, 3, 3), (3, 3, 5, 1)];
const MINUS: Icon = &[(-3, 0, 3, 0)];
const PLUS: Icon = &[(-3, 0, 3, 0), (0, -3, 0, 3)];
const CHEVRON: Icon = &[(-1, -2, 1, 0), (1, 0, -1, 2)];
const ARROW: Icon = &[(-4, 0, 4, 0), (1, -3, 4, 0), (4, 0, 1, 3)];

fn icon(canvas: &mut Canvas, ox: i32, oy: i32, segments: Icon, colour: u8) {
    for &(mut x, mut y, x1, y1) in segments {
        let (dx, dy) = ((x1 - x).abs(), -(y1 - y).abs());
        let (sx, sy) = ((x1 - x).signum(), (y1 - y).signum());
        let mut err = dx + dy;
        loop {
            canvas.set((ox + x) as usize, (oy + y) as usize, colour);
            if (x, y) == (x1, y1) { break; }
            let twice = 2 * err;
            if twice >= dy { err += dy; x += sx; }
            if twice <= dx { err += dx; y += sy; }
        }
    }
}
