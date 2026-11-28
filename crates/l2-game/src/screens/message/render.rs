use super::{face_frame, peasant_face_frame, BOX_SET, FACES, FACE_AT, FACE_WELL, SHIELD_BASE};
use super::copy::{body_at, body_text, nonblank, title, words};

use l2_view::Canvas;
use crate::input::Rect;
use crate::press::Press;
use crate::message::{self, category, Prompt, Record, Shape};
use crate::screen::Ctx;
use crate::shell::{self, font, Pen};


pub(crate) fn county_name(ctx: &Ctx, id: u8) -> String {
    let name = ctx.assets.shell.text(message::GROUP_COUNTY, ctx.game.map_slot * 20 + id as usize);
    if name.trim().is_empty() {
        "This county".into()
    } else {
        name.to_string()
    }
}

/// `g_playerNames[realm]`, with `L2.eng` group 7 standing in for a realm whose
/// name
/// place.
///
/// **Every screen that names a lord comes here**, and that is the point of it
/// being one function. `Ui_DrawText(&g_playerNames + realm * 0x2C, …)` is the
/// original's draw at all of them — the court, the battle prompt, the county
/// strip's third line (`CountyStrip_Draw` `0x0040F7D3`, whose call is
/// `FUN_004025D7(&g_playerNames + owner * 0x2C, 0x1E0, 0x118, 0xA0, …)`),
/// `Diplo_DrawScreen`'s heading, `Diplo_DrawLordCard`'s caption and the three
/// compose dialogs —
/// Three of them carried their own copy and each invented `REALM n` for a world
/// that never came through the front end; `docs/decisions.md` C189 fixed the
/// first two and this is the rest.
///
/// **Group 7 is indexed by the `lord`, not by the realm.** `Eng_Seek(7,
/// realm[+0x07])` is what new-game setup calls before copying sixteen bytes
/// into `g_playerNames`, so falling back to it *is* reconstructing what the
/// front end would have put there. **[V]**
pub fn lord_name(ctx: &Ctx, realm: u8) -> String {
    let named = ctx.game.player_names.get(realm as usize).map(|n| n.as_str()).unwrap_or_default();
    if !named.trim().is_empty() {
        return named;
    }
    let lord = ctx.game.kingdom.realms.get(realm as usize).map_or(0, |r| r.lord);
    words(&ctx.assets.shell, 7, lord.min(4) as usize, "Unknown ruler").to_owned()
}

pub(crate) fn message_pen<'a>(ctx: &'a Ctx) -> Pen<'a> {
    Pen {
        assets: &ctx.assets.shell,
        ink: &ctx.assets.ink,
        chrome: ctx.assets.chrome.as_ref(),
        shadow: (!ctx.assets.is_original()).then_some(font::SHADOW),
        caps: None,
    }
}

const INK: u8 = 128;
const SLATE: u8 = 9;
const RIM: u8 = 11;
const TEAL: u8 = 70;
const GOLD: u8 = 39;
const CREAM: u8 = 127;

pub(crate) fn draw_surface(pen: &Pen, ctx: &Ctx, canvas: &mut Canvas, f: message::Frame, record: &Record) {
    if !ctx.assets.is_original() {
        pen.window(canvas, f.x, f.y, f.w / 16, f.h / 16, BOX_SET);
        return;
    }
    let accent = match record.shape() {
        Shape::Capture | Shape::Ending | Shape::Prompt | Shape::Garrison => GOLD,
        _ => TEAL,
    };
    canvas.fill_rect(f.x + 5, f.y + 6, f.w, f.h, INK);
    canvas.fill_rect(f.x, f.y, f.w, f.h, SLATE);
    pen.outline(canvas, f.x, f.y, f.w, f.h, RIM);
    canvas.fill_rect(f.x + 1, f.y + 1, f.w - 2, 2, accent);
}

pub(crate) fn draw_ok(pen: &Pen, ctx: &Ctx, canvas: &mut Canvas, f: message::Frame) {
    let (x, y) = f.ok_button();
    if ctx.assets.is_original() {
        canvas.fill_rect(x - 8, y - 2, 40, 28, GOLD);
        pen.body_centred(canvas, x - 8, y + 5, 40, "OK", INK);
    } else {
        pen.ok_button(canvas, x, y, 0);
    }
}

pub(super) fn has_ok_button(ctx: &Ctx, record: &Record) -> bool {
    record.shape().has_ok_button()
        || (ctx.assets.is_original() && record.shape() == Shape::Unhandled)
}


/// So `+0x13` is not spare: it puts a **second realm's** name where the group's
/// own label would have gone.
pub(super) fn draw_notice(pen: &Pen, ctx: &Ctx, canvas: &mut Canvas, record: &Record, f: message::Frame) {
    if ctx.assets.is_original() {
        draw_atlas_notice(pen, ctx, canvas, record, f);
        return;
    }
    let heading = if record.county != 0 {
        county_name(ctx, record.county)
    } else if record.spare != 0 {
        lord_name(ctx, record.spare)
    } else {
        title(ctx, record).to_owned()
    };
    if record.county == 0 && record.spare != 0 {
        pen.heading(canvas, f.x + 0x20, f.y + 0x20, &heading, font::TEXT);
    } else {
        pen.heading_centred(canvas, f.x + 0x10, f.y + 0x20, f.w - 0x20, &heading, font::TEXT);
    }
    pen.body_wrapped(canvas, f.x + 0x20, f.y + 0x50, f.w - 0x40, body_text(ctx, record), font::TEXT);
}

fn draw_atlas_notice(pen: &Pen, ctx: &Ctx, canvas: &mut Canvas, record: &Record, f: message::Frame) {
    let context = if record.county != 0 {
        Some(county_name(ctx, record.county))
    } else if record.spare != 0 {
        Some(lord_name(ctx, record.spare))
    } else {
        None
    };
    let (x, y, w) = (f.x + 24, f.y, f.w - 48);
    pen.heading(canvas, x, y + 26, title(ctx, record), font::TEXT);
    let top = if let Some(context) = context {
        pen.body(canvas, x, y + 55, &context, TEAL);
        y + 80
    } else {
        y + 68
    };
    pen.body_wrapped(canvas, x, top, w, body_text(ctx, record), font::TEXT);
}

/// **Category `0x0D`, a county taken, with animations off** — `Msg_DrawWindow`'s
/// unanimated capture branch, which is not category 0's layout. `[V]`:
///
/// ```c
/// FUN_004093e0(0x20, 0xa0, 0x1a, 0xc);
/// Ui_OkButton(x + w - 0x30, h + y - 0x30, 0);
/// Ui_DrawCentred(100, g_scenarioIndex * 0x14 + county, x + 0x10, y + 0x20, w - 0x20, heading);
/// FUN_0040328e(g_messageGroup, 1, x + 0x20, y + 0x40, w - 0x40, 400, 0, 0, body);
/// ```
pub(super) fn draw_capture(pen: &Pen, ctx: &Ctx, canvas: &mut Canvas, record: &Record, f: message::Frame) {
    if ctx.assets.is_original() {
        draw_atlas_notice(pen, ctx, canvas, record, f);
        return;
    }
    let heading = county_name(ctx, record.county);
    pen.heading_centred(canvas, f.x + 0x10, f.y + 0x20, f.w - 0x20, &heading, font::TEXT);
    pen.body_wrapped(canvas, f.x + 0x20, f.y + 0x40, f.w - 0x40, body_text(ctx, record), font::TEXT);
}

/// `Msg_DrawDiplomacy` (`0x00475E07`) and, for category `0x14`, `Msg_DrawBeyondLetter`
/// (`0x00476488`): the literal "Beyond" for the name, arms 1 and 2 only. [V]
/// `FUN_00410C71` (`0x00410C71`) `(county, x + 0x100, y + 0x40)` in arms 5..10 is not drawn. [D]
pub(super) fn draw_diplomacy(pen: &Pen, ctx: &Ctx, canvas: &mut Canvas, record: &Record, f: message::Frame) {
    let beyond = record.category == category::BEYOND;
    draw_portrait(pen, ctx, canvas, record.from, f);
    let from = words(&ctx.assets.shell, message::GROUP_FROM, 0, "From");
    let x = pen.heading(canvas, f.x + 0x6C, f.y + 0x20, from, font::TEXT);
    let name = if beyond { "Beyond".to_string() } else { lord_name(ctx, record.from) };
    pen.heading(canvas, x, f.y + 0x20, &name, font::TEXT);
    pen.body(canvas, f.x + 0x6C, f.y + 0x40, title(ctx, record), font::TEXT);
    let summary = body_at(ctx, record, 1);
    let arm = record.group.checked_sub(0xF5);
    let draft = ctx.game.messages.letters.get(usize::from(record.from)).and_then(|s| nonblank(s));
    let free_text = |canvas: &mut Canvas, top: i32, text: &str| {
        let well = Rect::new(f.x + 0x20, f.y + top, f.w - 0x40, 0x5A);
        if ctx.assets.is_original() {
            canvas.fill_rect(well.x, well.y, well.w, well.h, INK);
            pen.outline(canvas, well.x, well.y, well.w, well.h, RIM);
        } else {
            pen.inset(canvas, well);
        }
        // `FUN_0040352F` (`0x0040352F`)
        pen.buffer_wrapped(canvas, f.x + 0x28, f.y + top + 4, f.w - 0x50, text, font::TEXT);
    };
    match arm {
        Some(1 | 2) => free_text(canvas, 0x68, draft.unwrap_or(summary)),
        _ if beyond => {
            pen.body_wrapped(canvas, f.x + 0x20, f.y + 0x68, f.w - 0x40, summary, font::TEXT);
        }
        Some(0) => {
            let x = pen.body(canvas, f.x + 0x20, f.y + 0x68, summary, font::TEXT);
            let crowns = if record.variant == 1 { "crown" } else { "crowns" };
            let noun = words(&ctx.assets.shell, 8, usize::from(record.variant != 1), crowns);
            pen.count_with_noun(shell::Face::Body, canvas, x, f.y + 0x68, record.variant, noun, font::TEXT);
        }
        Some(3) => {
            if ctx.assets.is_original() {
                pen.body_wrapped(canvas, f.x + 0x20, f.y + 0x68, f.w - 0x40, summary, font::TEXT);
                if let Some(draft) = draft {
                    free_text(canvas, 0x88, draft);
                }
            } else {
                pen.body(canvas, f.x + 0xA0, f.y + 0x68, summary, font::TEXT);
                free_text(canvas, 0x88, draft.unwrap_or_default());
            }
        }
        Some(5..=10) => {
            let top = f.y + 0x68;
            let height = if ctx.assets.is_original() {
                pen.body_wrapped(canvas, f.x + 0x20, top, f.w - 0x40, summary, font::TEXT)
            } else {
                pen.body(canvas, f.x + 0x20, top, summary, font::TEXT);
                16
            };
            let county = county_name(ctx, record.county);
            pen.body(canvas, f.x + 0x20, top + height.max(16), &county, font::TEXT);
        }
        // Arm 4 and the final `else` (00470000.c:2359), which draws groups 0x100..=0x102. [V]
        _ => {
            if ctx.assets.is_original() {
                pen.body_wrapped(canvas, f.x + 0x20, f.y + 0x68, f.w - 0x40, summary, font::TEXT);
            } else {
                pen.body(canvas, f.x + 0x20, f.y + 0x68, summary, font::TEXT);
            }
        }
    }
}

pub(super) fn draw_letter(pen: &Pen, ctx: &Ctx, canvas: &mut Canvas, record: &Record, f: message::Frame) {
    draw_portrait(pen, ctx, canvas, record.from, f);
    let from = words(&ctx.assets.shell, message::GROUP_FROM, 0, "From");
    let x = pen.heading(canvas, f.x + 0x6C, f.y + 0x20, from, font::TEXT);
    pen.heading(canvas, x, f.y + 0x20, &lord_name(ctx, record.from), font::TEXT);

    let x = pen.body(canvas, f.x + 0x6C, f.y + 0x40, title(ctx, record), font::TEXT);
    if ctx.assets.is_original() {
        let mut context = if record.county != 0 {
            county_name(ctx, record.county)
        } else if record.spare != 0 {
            lord_name(ctx, record.spare)
        } else {
            String::new()
        };
        if record.category == category::PAY_PROMPT {
            use std::fmt::Write;
            if !context.is_empty() {
                context.push_str("  /  ");
            }
            let price = ctx.game.kingdom.diplomacy.help_price;
            let _ = write!(context, "{price} crowns");
        }
        if !context.is_empty() {
            pen.body(canvas, f.x + 0x6C, f.y + 0x54, &context, TEAL);
        }
    } else {
        if record.category == category::PAY_PROMPT {
            let price = ctx.game.kingdom.diplomacy.help_price;
            pen.count(canvas, x, f.y + 0x40, price, 0, font::TEXT);
        } else if record.spare != 0 {
            pen.body(canvas, x, f.y + 0x40, &lord_name(ctx, record.spare), font::TEXT);
        } else if record.county != 0 {
            pen.body(canvas, x, f.y + 0x40, &county_name(ctx, record.county), font::TEXT);
        }
    }
    pen.body_wrapped(canvas, f.x + 0x20, f.y + 0x70, f.w - 0x40, body_text(ctx, record), font::TEXT);
}

/// **Category `0x02`** — the portrait panel with `L2.eng` 109/1 over the county
/// name, both centred in 0x140.
///
/// The well is **not** empty: the arm blits a face into it like a lord's letter
/// does, only the frame is [`peasant_face_frame`]'s and not a realm's. The
/// county the picture answers to is the record's — `DAT_0055CE54`, the same
/// byte the heading's county name is read with two lines below.
pub(super) fn draw_county_portrait(
    pen: &Pen,
    ctx: &Ctx,
    canvas: &mut Canvas,
    record: &Record,
    f: message::Frame,
) {
    let population =
        ctx.game.kingdom.counties.get(record.county as usize).map_or(0, |c| c.population);
    draw_face(pen, canvas, peasant_face_frame(population), f);
    let heading = if ctx.assets.is_original() {
        title(ctx, record)
    } else {
        words(&ctx.assets.shell, message::GROUP_FROM, 1, "An envoy from")
    };
    pen.heading_centred(canvas, f.x + 0x6C, f.y + 0x18, 0x140, heading, font::TEXT);
    let county = county_name(ctx, record.county);
    pen.heading_centred(canvas, f.x + 0x6C, f.y + 0x38, 0x140, &county, font::TEXT);
    pen.body_wrapped(canvas, f.x + 0x20, f.y + 0x70, f.w - 0x40, body_text(ctx, record), font::TEXT);
}

/// **Category `0x0E`, the ending.** The name at the top is the *local player's*
/// for group 225 and the *sender's* for everything else —
/// one layout serve *"Victory!"*, *"Defeat!"* and an AI's obituary.
///
/// The well is **not** empty: the arm calls `FUN_00475D73(DAT_00553EE0)` and
/// blits the frame like every other portrait layout. `DAT_00553EE0` is
/// `g_messageFrom`, so the face is the **sender's**, whoever the heading names
/// — the dead lord for an obituary, the player himself for group 224 *Defeat!*
///
/// (`from == g_localPlayer` is what makes it his), and the realm-0 end of the
/// ladder for group 225, which `Msg_Enqueue(0, g_localPlayer, 0xE1, …)` posts
/// with `from = 0`.
pub(super) fn draw_ending(pen: &Pen, ctx: &Ctx, canvas: &mut Canvas, record: &Record, f: message::Frame) {
    draw_face(pen, canvas, realm_face_frame(ctx, record.from), f);
    let who = if record.group == l2_kingdom::victory::MSG_VICTORY {
        ctx.game.player
    } else {
        record.from
    };
    pen.heading_centred(canvas, f.x + 0x20, f.y + 0x20, 0x1A0, &lord_name(ctx, who), font::TEXT);
    pen.body_centred(
        canvas,
        f.x + 0x20,
        f.y + 0x48,
        f.w - 0x40,
        title(ctx, record),
        font::TEXT,
    );
    pen.body_wrapped(canvas, f.x + 0x20, f.y + 0x70, f.w - 0x40, body_text(ctx, record), font::TEXT);
}

pub(super) fn draw_garrison(pen: &Pen, ctx: &Ctx, canvas: &mut Canvas, record: &Record, f: message::Frame) {
    pen.heading_centred(
        canvas,
        f.x + 0x10,
        f.y + 0x20,
        f.w - 0x20,
        title(ctx, record),
        font::TEXT,
    );
    let top = if ctx.assets.is_original() {
        pen.body(canvas, f.x + 0x20, f.y + 0x38, &county_name(ctx, record.county), TEAL);
        f.y + 0x50
    } else {
        f.y + 0x40
    };
    pen.body_wrapped(canvas, f.x + 0x20, top, f.w - 0x40, body_text(ctx, record), font::TEXT);

    let county = ctx.game.kingdom.counties.get(record.county as usize);
    let garrison = county
        .map(|c| c.garrison_unit)
        .filter(|u| *u != 0)
        .and_then(|u| ctx.game.kingdom.campaign.units.get(u))
        .map_or(0, |u| u.men);
    // `*(int *)(&DAT_004D8A0C + castleType * 4)` — which is
    // `CASTLE_GARRISON_CAP[castleType - 1]`, because the table at `0x004D8A10`
    // is indexed by castle types **1..=5**. It looks like an off-by-one and is
    // not one; the entry it would read at type 0 is the tail of
    // `CASTLE_WORKFORCE` and a county with no castle cannot be garrisoned.
    let capacity =
        county.map_or(0, |c| l2_kingdom::industry::garrison_cap(&ctx.game.kingdom.tables, c.castle_type));
    let room = capacity - garrison;
    // `Msg_DrawWindow`: `Ui_DrawNumber(room, '@', &DAT_004D700C, x + 0x20,
    // y + 0x70)` and then the noun at `x + g_penAdvance + 0x20`. The suffix is
    // **one space**, so the old no-lead-plus-space had the digits and the noun
    // both four pixels left. **[V]**
    let body = shell::Face::Body;
    let x = pen.number_in(body, canvas, f.x + 0x20, f.y + 0x70, room, '@', " ", font::TEXT);
    let line = |index, fallback: &'static str| words(&ctx.assets.shell, usize::from(record.group), index, fallback);
    pen.body(canvas, x, f.y + 0x70, line(2, "places left in the garrison"), font::TEXT);

    let x = pen.body(canvas, f.x + 0x20, f.y + 0x80, line(3, "Troops in this army:"), font::TEXT);
    let men = ctx
        .game
        .kingdom
        .campaign
        .units
        .get(record.variant as usize)
        .map_or(0, |u| u.men) as i32;
    // `Ui_DrawNumber(menTotal, '@', &DAT_004D7010, …)`, one space. **[V]**
    pen.number_in(body, canvas, x, f.y + 0x80, men, '@', " ", font::TEXT);
    pen.body(canvas, f.x + 0x20, f.y + 0xB0, line(4, "Split off a detachment?"), font::TEXT);
}

/// **Category `0x0F`, a county's random event** — `Msg_DrawWindow`'s arm at
/// `00470000.c:2067`, whose record `FUN_00448D7E` posts as
/// `Msg_Enqueue(0, g_localPlayer, county.eventId, 0, 0x0F, county, 0, 0)`.
///
/// ```c
/// Ui_DrawCentred(group, 0, x + 0x10, y + 0x20, w - 0x20, &g_fontHeading, 0x3F);
/// FUN_0040328E(group, 1, x + 0x20, y + 0x40, w - 0x40, 400, 0, 0, &g_fontBody, 0x3F);
/// g_penAdvance = 0;
/// switch (g_counties[county].eventId) {       /* the county's, not the record's */
///   0x87 Rats:          Ui_DrawCount(+0x278, 2, x + 0x20, y + 0x90) + 77/0x19
///   0x8B Grain found:   Ui_DrawCount(+0x278, 2, …)                  + 77/0x1A
///   0x88 Mad cows:      Ui_DrawCount(+0x274, 4, …)                  + 77/0x14
///   0x89 Wolves:        Ui_DrawCount(+0x274, 4, …)                  + 77/0x15
///   0x8C Bad cattle:    Ui_DrawCount(+0x274, 4, …)                  + 77/0x16
///   0x8D Cow bonanza:   Ui_DrawCount(+0x274, 4, …)                  + 77/0x17
///   0x8A Plague:        Ui_DrawNumber(+0x2F8, '@', " ", …)          + 77/0x1D
///   0x8E Wedding fever: Ui_DrawNumber(+0x2F8, '@', " ", …)          + 77/0x1E
/// }
/// ```
///
/// **All eight number lines are drawn.** The last two were not: *Plague* and
/// *Wedding fever* print county `+0x2F8`, `Population_UpdateAll`'s event swing,
/// which nothing of ours carried. It is carried now —
/// `County::event_population_swing`, written by the population pass and imported
/// from the save (`docs/stored-fields.json`, `County+0x2F8`) — so the two draw
/// `Ui_DrawNumber(+0x2F8, '@', " ", …)` and 77/`0x1D` *"extra deaths."* or
/// 77/`0x1E` *"extra births."* after it.
///
/// `DAT_004D7050` and `DAT_004D7054` are both `" "`, dumped.
///
/// **The record is posted by [`message::post_event`]**, the port of
/// `FUN_00448D7E`, which `Machine::update` runs once a frame for
/// `Game::selected`
pub(super) fn draw_event(pen: &Pen, ctx: &Ctx, canvas: &mut Canvas, record: &Record, f: message::Frame) {
    if ctx.assets.is_original() {
        draw_atlas_notice(pen, ctx, canvas, record, f);
    } else {
        pen.heading_centred(canvas, f.x + 0x10, f.y + 0x20, f.w - 0x20, title(ctx, record), font::TEXT);
        pen.body_wrapped(canvas, f.x + 0x20, f.y + 0x40, f.w - 0x40, body_text(ctx, record), font::TEXT);
    }

    let Some(c) = ctx.game.kingdom.counties.get(record.county as usize) else { return };
    // A queued report must not show the figures from a newer, different county event.
    if c.event_id != record.group {
        return;
    }
    enum Line {
        Count(i32, usize),
        Number(i32),
    }
    let (line, word, fallback) = match record.group {
        0x87 => (Line::Count(c.grain_event_change, EVENT_NOUN_SACK), 0x19, "eaten by rats."),
        0x8B => (Line::Count(c.grain_event_change, EVENT_NOUN_SACK), 0x1A, "of grain found."),
        0x88 => (Line::Count(c.herd_event_change, EVENT_NOUN_ANIMAL), 0x14, "of cattle died of disease."),
        0x89 => (Line::Count(c.herd_event_change, EVENT_NOUN_ANIMAL), 0x15, "of cattle killed by wolves."),
        0x8C => (Line::Count(c.herd_event_change, EVENT_NOUN_ANIMAL), 0x16, "of cattle lost."),
        0x8D => (Line::Count(c.herd_event_change, EVENT_NOUN_ANIMAL), 0x17, "of cattle added."),
        0x8A => (Line::Number(c.event_population_swing), 0x1D, "extra deaths."),
        0x8E => (Line::Number(c.event_population_swing), 0x1E, "extra births."),
        _ => return,
    };
    let (x, y) = (f.x + 0x20, f.y + 0x90);
    let next = match line {
        Line::Count(value, kind) => {
            let noun = match kind {
                EVENT_NOUN_SACK if value == 1 => "sack",
                EVENT_NOUN_SACK => "sacks",
                _ => "head",
            };
            let index = kind + usize::from(value != 1);
            let noun = words(&ctx.assets.shell, 8, index, noun);
            pen.count_with_noun(shell::Face::Body, canvas, x, y, value, noun, font::TEXT)
        }
        Line::Number(value) => {
            pen.number_in(shell::Face::Body, canvas, x, y, value, '@', " ", font::TEXT)
        }
    };
    pen.body(canvas, next, y, words(&ctx.assets.shell, EVENT_GROUP, word, fallback), font::TEXT);
}

/// `L2.eng` group 77, whose indices `0x14` … `0x1A` are the event letters'
/// last words: *"died of disease."*, *"eaten by rats."* and the rest.
const EVENT_GROUP: usize = 77;
const EVENT_NOUN_SACK: usize = 2;
const EVENT_NOUN_ANIMAL: usize = 4;

pub(super) fn draw_tip(pen: &Pen, ctx: &Ctx, canvas: &mut Canvas, record: &Record, pointer: (i32, i32)) {
    let (mx, my) = pointer;
    let x = if mx < 0xF1 { mx + 0x20 } else { mx - 0xA8 }.clamp(0x50, 0xF0);
    let y = if my < 0xF1 { my + 0x20 } else { my - 0x50 }.clamp(0x50, 0xF0);
    let heading = title(ctx, record);
    if ctx.assets.is_original() {
        let width = ctx.assets.shell.body.as_ref().map_or_else(
            || l2_view::text::width(heading), |font| font.width(heading),
        );
        let w = (width + 32).max(160);
        let f = message::Frame { x, y, w, h: 40 };
        draw_surface(pen, ctx, canvas, f, record);
        pen.body_centred(canvas, x, y + 12, w, heading, font::TEXT);
    } else {
        pen.window(canvas, x, y, 0xB, 3, BOX_SET);
        pen.body_centred(canvas, x, y + 0x10, 0xB0, heading, font::TEXT);
    }
}


fn draw_portrait_well(pen: &Pen, canvas: &mut Canvas, f: message::Frame) {
    let (dx, dy, w, h) = FACE_WELL;
    pen.inset(canvas, Rect::new(f.x + dx, f.y + dy, w, h));
}

/// The well and one `Faces.pl8` frame in it — `Ui_DrawInsetRect(x + 0xF, y +
/// 0x11, 0x52, 0x4E)` then `Blit_Raster(0x4EEB80, x + 0x10, y + 0x12, 0x50,
/// 0x4C)`. `0x4EEB80` is the buffer `FUN_00475D73` decoded the frame into, so
/// the pair is *one* portrait draw, and every arm of `Msg_DrawWindow` that has
/// a portrait writes it out identically.
fn draw_face(pen: &Pen, canvas: &mut Canvas, frame: usize, f: message::Frame) {
    draw_portrait_well(pen, canvas, f);
    if let Some(sheet) = pen.assets.sheet(FACES) {
        if let Some(bitmap) = sheet.frame(frame) {
            canvas.blit(&bitmap, f.x + FACE_AT.0, f.y + FACE_AT.1);
        }
    }
}

fn draw_portrait(pen: &Pen, ctx: &Ctx, canvas: &mut Canvas, realm: u8, f: message::Frame) {
    let r = ctx.game.kingdom.realms.get(realm as usize);
    if let (Some(r), Some(chrome)) = (r, pen.chrome) {
        let shield = SHIELD_BASE + r.shield_index.clamp(0, 5) as usize;
        chrome.draw_panel_frame(canvas, shield, f.x + f.w - 0x1E, f.y + 0x12);
    }
    draw_face(pen, canvas, realm_face_frame(ctx, realm), f);
}

/// `FUN_00475D73(realm)` — the argument every portrait arm passes is a realm,
/// and the ladder reads that realm's lord and whether a person plays him.
fn realm_face_frame(ctx: &Ctx, realm: u8) -> usize {
    let r = ctx.game.kingdom.realms.get(realm as usize);
    face_frame(r.map_or(0, |r| r.lord), r.is_some_and(|r| r.is_human), realm)
}

/// It is the build stamp's assertion, and it is here
/// because the test may not know the geometry — computing the probe from the
/// same constants the code reads is what makes an ablation prove nothing.
pub fn repaint_clickables(ctx: &Ctx, canvas: &mut Canvas, record: &Record) {
    let pen = message_pen(ctx);
    if let Some(prompt) = record.answer_widgets() {
        draw_prompt(&pen, ctx, canvas, prompt, &Press::new());
    }
    if has_ok_button(ctx, record) {
        if let Some(frame) = window_frame(ctx, record) {
            draw_ok(&pen, ctx, canvas, frame);
        }
    }
}


pub fn window_frame(ctx: &Ctx, record: &Record) -> Option<message::Frame> {
    match record.shape() {
        Shape::Paragraphs(n) => Some(tip_layout(ctx, record, n).0.frame),
        Shape::Tip => None,
        // Unknown categories and out-of-range help/letter groups can appear in an imported save.
        _ => message::frame_of(record).or_else(|| {
            message::frame_of(&Record { category: category::NOTICE, ..*record })
        }),
    }
}

/// **The tip window's paragraphs, broken into lines the way `FUN_0040328E`
/// breaks them**, and the layout those line counts give.
///
/// Installed words and their English fallback are measured using the same body
/// font that paints each line.
pub fn tip_layout(ctx: &Ctx, record: &Record, n: usize) -> (message::Paragraphs, Vec<Vec<String>>) {
    let texts: Vec<Vec<String>> = (1..=n)
        .map(|i| {
            message::break_lines(body_at(ctx, record, i), message::PARAGRAPH_WIDTH, |c| glyph_width(ctx, c))
        })
        .collect();
    let lines: Vec<usize> = texts.iter().map(Vec::len).collect();
    (message::paragraph_layout(&lines), texts)
}

/// `FUN_004015B9(c, &g_fontBody)` — one glyph's advance. `[I]` for a character
/// the font has no frame for: the original answers 0 and
/// [`crate::shell::font::Font::width`] answers a space's advance; no tip string
/// has been found to contain one.
fn glyph_width(ctx: &Ctx, c: char) -> i32 {
    let mut buf = [0; 4];
    let s = c.encode_utf8(&mut buf);
    match &ctx.assets.shell.body {
        Some(f) => f.width(s),
        None => l2_view::text::width(s),
    }
}

pub(super) fn draw_paragraphs(pen: &Pen, ctx: &Ctx, canvas: &mut Canvas, record: &Record, n: usize) {
    let (layout, texts) = tip_layout(ctx, record, n);
    let f = layout.frame;
    draw_surface(pen, ctx, canvas, f, record);
    pen.heading(canvas, layout.heading.0, layout.heading.1, title(ctx, record), font::TEXT);
    for (lines, top) in texts.iter().zip(&layout.tops) {
        for (k, line) in lines.iter().enumerate() {
            pen.body(canvas, f.x + 0x10, top + 0x10 * k as i32, line, font::TEXT);
        }
    }
    draw_ok(pen, ctx, canvas, f);
}

pub(super) fn draw_prompt(pen: &Pen, ctx: &Ctx, canvas: &mut Canvas, prompt: Prompt, press: &Press) {
    let [yes, no] = prompt.widgets();
    if ctx.assets.is_original() {
        for (i, ((x, y), label)) in [(yes, "YES"), (no, "NO")].into_iter().enumerate() {
            let (fill, text) = if press.is_pressed(i) {
                (RIM, CREAM)
            } else if i == 0 {
                (GOLD, INK)
            } else {
                (INK, CREAM)
            };
            canvas.fill_rect(x, y, Prompt::SIDE, Prompt::SIDE, fill);
            pen.outline(canvas, x, y, Prompt::SIDE, Prompt::SIDE, RIM);
            pen.body_centred(canvas, x, y + 9, Prompt::SIDE, label, text);
        }
        return;
    }
    // `Widget_Draw` adds one to the frame while `+0x0D` runs.
    for (i, (at, frame)) in [(yes, Prompt::FRAME_YES), (no, Prompt::FRAME_NO)].into_iter().enumerate() {
        let frame = if press.is_pressed(i) { frame + 1 } else { frame };
        if !pen.system_frame(canvas, frame, at.0, at.1) {
            crate::shell::button_recess(canvas, at.0, at.1, Prompt::SIDE, Prompt::SIDE);
        }
    }
}


