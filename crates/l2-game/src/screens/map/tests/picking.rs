#![allow(unused_imports)]
use super::*;
use super::layout::*;
use super::super::*;

#[test]
fn the_diamonds_leave_no_pixel_unpicked() {
    for zoom in [NEAR, FAR] {
        let mut s = MapScreen::new();
        s.zoom = zoom;
        s.view = Viewport::new(60, 30).clamped(&zoom);
        let (tx, ty) = (1..l2_kingdom::MAP_DIM - 1)
            .flat_map(|y| (1..l2_kingdom::MAP_DIM - 1).map(move |x| (x, y)))
            .find(|&(x, y)| {
                [(x, y), (x + 1, y), (x, y + 1), (x - 1, y), (x, y - 1)].iter().all(|&(a, b)| {
                    campaign::tile_centre(s.view, &zoom, a, b).is_some_and(|(cx, cy)| {
                        let c = s.map_clip();
                        cx - zoom.pitch > c.x0
                            && cx + zoom.pitch < c.x1
                            && cy - zoom.row_step * 2 > c.y0
                            && cy + zoom.row_step * 2 < c.y1
                    })
                })
            })
            .expect("a tile with room around it");
        let (cx, cy) = campaign::tile_centre(s.view, &zoom, tx, ty).expect("in view");

        let mut misses = Vec::new();
        for dy in -zoom.row_step..=zoom.row_step {
            for dx in -zoom.half_pitch..=zoom.half_pitch {
                if s.pick_tile(cx + dx, cy + dy).is_none() {
                    misses.push((dx, dy));
                }
            }
        }
        assert!(
            misses.is_empty(),
            "zoom {}: {} pixels around a tile centre pick nothing, first at {:?} \
             — the diamonds do not tile the plane",
            zoom.id,
            misses.len(),
            misses.first(),
        );
    }
}

#[test]
fn the_pick_diamond_is_the_lattice_and_not_the_picture() {
    for zoom in [NEAR, FAR] {
        assert_eq!(zoom.half_pitch * 2, zoom.pitch, "zoom {}", zoom.id);
    }
    assert_eq!(NEAR.tile_w / 2, 29);
    assert_eq!(NEAR.half_pitch, 30);
    assert_eq!(FAR.tile_w / 2, 5);
    assert_eq!(FAR.half_pitch, 6);
}

#[test]
fn bounded_picker_preserves_legacy_choices_at_tile_vertices_and_screen_edges() {
    let mut screen = MapScreen::new();
    for zoom in [NEAR, FAR] {
        screen.zoom = zoom;
        for view in [Viewport::new(60, 30), Viewport::new(0, 0), Viewport::new(120, 58)] {
            screen.view = view.clamped(&zoom);
            let clip = zoom.clip();
            let centres: Vec<_> = (0..l2_kingdom::MAP_DIM).flat_map(|y| {
                (0..l2_kingdom::MAP_DIM).filter_map(move |x| {
                    campaign::tile_centre(view.clamped(&zoom), &zoom, x, y)
                        .map(|p| (x as u8, y as u8, p))
                })
            }).collect();
            let points = centres.iter().flat_map(|&(_, _, (x, y))| {
                [(x + zoom.half_pitch, y), (x, y - zoom.row_step), (x, y),
                    (x + zoom.half_pitch / 2, y + zoom.row_step / 2)]
            }).chain([(clip.x0, clip.y0), (clip.x1 - 1, clip.y1 - 1), (clip.x1, clip.y0)]);
            for (x, y) in points {
                let expected = if clip.contains(x, y) {
                    centres.iter().find(|&&(_, _, (cx, cy))| {
                        (x - cx).abs() * zoom.row_step + (y - cy).abs() * zoom.half_pitch
                            <= zoom.row_step * zoom.half_pitch
                    }).map(|&(tx, ty, _)| (tx, ty))
                } else { None };
                assert_eq!(screen.pick_tile(x, y), expected, "zoom {}: ({x}, {y})", zoom.id);
            }
        }
    }
}

#[test]
fn the_screen_opens_at_the_originals_zoom_and_scroll_origin() {
    let s = MapScreen::new();
    assert_eq!(s.zoom().id, NEAR.id);
    assert_eq!(s.viewport(), Viewport::new(0x4A, 0x14));
    assert_eq!(s.map_clip(), NEAR.clip());
}

#[test]
fn the_zoom_toggle_saves_and_restores_the_near_views_position() {
    let assets = crate::game::Assets::placeholder();
    let mut game = crate::Game::new(1);
    let mut ctx = Ctx { game: &mut game, assets: &assets };
    let mut s = MapScreen::new();
    s.view = Viewport::new(60, 30);
    s.toggle_zoom(&mut ctx);
    assert_eq!(s.zoom().id, FAR.id);
    assert!(ctx.game.map_zoom_far, "the far zoom did not reach Game");
    assert_eq!(
        s.viewport(),
        Viewport::new(0, 14),
        "row clamps to 0, col 0x0E stands"
    );
    assert!(!s.scroll(Dir::E));
    assert!(!s.scroll(Dir::N));
    s.toggle_zoom(&mut ctx);
    assert_eq!(s.zoom().id, NEAR.id);
    assert!(!ctx.game.map_zoom_far, "the near zoom did not reach Game");
    assert_eq!(s.viewport(), Viewport::new(60, 30), "back where it was");
    assert!(s.scroll(Dir::E), "and the near view scrolls again");
    assert_eq!(s.viewport(), Viewport::new(60, 31));
}

// Map_ZoomInAtTile 00430000.c:2522 `DAT_0052afa4 = 1`; Map_Click :7770 swallows the release.
// Ablate: `return map_click_dismiss(ctx)…` in the zoom arm, second assertion red.
#[test]
fn the_far_view_zoom_click_leaves_an_open_message_alone() {
    let assets = crate::game::Assets::placeholder();
    let mut game = crate::Game::new(1);
    let rec = crate::message::Record { to: 0, group: 0x92, ..Default::default() };
    game.messages.enqueue(rec, 1);
    game.messages.pull();
    assert!(game.messages.is_open());
    let mut ctx = Ctx { game: &mut game, assets: &assets };
    let mut s = MapScreen::new();
    s.zoom = FAR;
    s.view = Viewport::new(0, 14).clamped(&FAR);
    let c = s.map_clip();
    let (x, y) = ((c.x0 + c.x1) / 2, (c.y0 + c.y1) / 2);
    s.handle(Event::Click { x, y }, &mut ctx);
    assert_eq!(s.zoom().id, NEAR.id, "the click zoomed in");
    assert!(ctx.game.messages.is_open(), "and did not dismiss the letter");
}

/// Ablation: make `mode_screen_id` answer `None`
/// goes red.
#[test]
fn picking_up_an_army_is_screen_0x10_and_putting_it_down_is_not() {
    let assets = crate::game::Assets::placeholder();
    let mut game = crate::Game::new(1);
    let unit = game
        .kingdom
        .campaign
        .units
        .spawn(l2_kingdom::unit::Unit::new(l2_kingdom::UnitKind::Army, 1, 10, 10))
        .expect("a free slot");
    let mut s = MapScreen::new();
    assert_eq!(s.mode_screen_id(), None, "the map is 0, which its ScreenId already says");
    let ctx = Ctx { game: &mut game, assets: &assets };
    s.begin_move_selection(&ctx, unit);
    assert_eq!(s.mode_screen_id(), Some(0x10));
    s.cancel_move_selection();
    assert_eq!(s.mode_screen_id(), None);
}


// FUN_004B0CB4 0x004B0CB4 `g_screenId = 0x23`; Season_FinishFade 0x0049A3E6 guards 0x24.
// Ablate: the `fading` arm of mode_screen_id -> None throughout, first assertion red.
#[test]
fn the_end_of_turn_fade_is_screens_0x23_then_0x24_then_0x23_then_the_map() {
    let mut s = MapScreen::new();
    s.fading = Some(Fading { phase: 0, status: String::new(), over: false });
    let mut seen = vec![s.mode_screen_id()];
    while s.fading.is_some() {
        s.tick_fade();
        seen.push(s.mode_screen_id());
    }
    seen.dedup();
    assert_eq!(seen, [Some(0x23), Some(0x24), Some(0x23), None], "newgame-original.orig f602..f607");
}

// Map_ZoomInAtTile 00430000.c:2522 sets `DAT_0052afa4`; a release off every tile leaves it,
// and FUN_0043CAF4 :7534 spends the next right click on it (peek 2026-09-24, army-panel-kinds).
// Ablate: `ctx.game.map_click_latch = true;` in the zoom arm, first assertion red.
#[test]
fn a_zoom_in_released_off_the_map_swallows_the_next_right_click() {
    let assets = crate::game::Assets::placeholder();
    let mut game = crate::Game::new(1);
    let mut ctx = Ctx { game: &mut game, assets: &assets };
    let mut s = MapScreen::new();
    s.zoom = FAR;
    s.view = Viewport::new(0, 14).clamped(&FAR);
    let c = s.map_clip();
    let (x, y) = ((c.x0 + c.x1) / 2, (c.y0 + c.y1) / 2);
    s.handle(Event::Click { x, y }, &mut ctx);
    s.handle(Event::Release { x: c.x1 + 20, y }, &mut ctx);
    let first = s.handle(Event::RightClick { x, y }, &mut ctx);
    assert!(matches!(first, Transition::Stay), "the first right click opened a box");
    let second = s.handle(Event::RightClick { x, y }, &mut ctx);
    assert!(matches!(second, Transition::Push(_)), "the second opens it");
}

// Battle_Frame 004b0000.c:6742 `DAT_005533b0 = Map_PickTile(0)`; Map_Click :7772 clears the latch.
// Ablate: the `Event::Release` arm's `ctx.game.map_click_latch = false;`, red.
#[test]
fn a_zoom_in_released_on_a_tile_leaves_the_next_right_click_alone() {
    let assets = crate::game::Assets::placeholder();
    let mut game = crate::Game::new(1);
    let mut ctx = Ctx { game: &mut game, assets: &assets };
    let mut s = MapScreen::new();
    s.zoom = FAR;
    s.view = Viewport::new(0, 14).clamped(&FAR);
    let c = s.map_clip();
    let (x, y) = ((c.x0 + c.x1) / 2, (c.y0 + c.y1) / 2);
    s.handle(Event::Click { x, y }, &mut ctx);
    s.handle(Event::Release { x, y }, &mut ctx);
    assert!(matches!(s.handle(Event::RightClick { x, y }, &mut ctx), Transition::Push(_)));
}
