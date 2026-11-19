#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! The rule here was `next_tick = Instant::now() + TICK` read *after* the wait,
//! which carried every overshoot forward and made a 16 ms tick 16.4 ms of wall
//! clock — two percent slow, compounding, and audible the moment a film's sound
//! track (played out by the device in real time) ran against a picture stepped
//! by tick count. `docs/decisions.md` C193.

mod app_frame;
mod events;

mod app;
mod launch;
mod os;
mod remote;
use app::*;

use std::sync::Arc;
use std::time::{Duration, Instant};

use l2_game::audio::Audio;
use l2_game::game::Assets;
use l2_game::screen::{Machine, ScreenId};
use l2_game::screens::setup::SetupPage;
use l2_game::{scenario, Game};
use l2_view::Canvas;

use pixels::Pixels;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{CustomCursor, Window};

const CANVAS_W: u32 = l2_view::canvas::WIDTH as u32;
const CANVAS_H: u32 = l2_view::canvas::HEIGHT as u32;

const DOUBLE_CLICK: Duration = Duration::from_millis(500);

const DOUBLE_CLICK_SLOP: i32 = 4;

struct App {
    game: Game,
    assets: Assets,
    machine: Machine,
    canvas: Canvas,
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    started: Instant,
    ticker: l2_game::clock::Ticker,
    last_cursor: (i32, i32),
    audio: Audio,
    director: l2_game::audio::Director,
    /// `DAT_004DF3A8` — whether Control is held. The window procedure keeps the
    /// same latch and its digit arm dispatches on it: with Control, store a
    /// battle control group; without, recall one.
    ctrl: bool,
    alt: bool,
    /// What `prefs.ini` last held; `persist_prefs` writes on a difference.
    saved_prefs: l2_game::game::Prefs,
    last_press: Option<(Instant, (i32, i32))>,
    /// **`DAT_004EABC2`'s left bit**
    /// derives from it. In the library so a test can drive it without a window;
    /// see [`l2_game::input::LeftButton`].
    left: l2_game::input::LeftButton,
    pointer: l2_game::cursor::Pointer,
    /// Original cursor artwork, or cursor resources in explicit classic mode.
    pictures: Vec<l2_formats::cursors::Picture>,
    cursors: Vec<(u16, CustomCursor)>,
    cursor_scale: u32,
    /// The lobby's sockets; `l2_game::net::service` pumps them between ticks (D-5).
    net: Option<l2_game::net::Driver>,
    /// The lockstep session after `Start`, pumped before each tick (D-5).
    live: Option<l2_game::net::Live>,
    remote: Option<remote::Remote>,
}

fn main() {
    launch::panic_hook();
    if let Err(e) = run() {
        launch::fatal(&e.to_string());
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let launch::Args { dir, mods, sound, verify, net } = launch::args();
    let dir = launch::install_dir(dir);
    if let Some(path) = verify {
        // [D] headless: a replay panic goes to stderr, not the crash box, C240.
        drop(std::panic::take_hook());
        app::verify_replay(dir, mods, path);
    }
    let classic = dir.is_some();
    let _save_scope = if classic { None } else { l2_game::saves::standalone_scope() };
    let prefs = l2_game::prefs::load();
    let platform = app::platform(dir.as_deref(), mods.as_deref())?;
    for m in &platform.load_order {
        println!("mod: {} {}", m.id, m.version);
    }
    let tables = platform.kingdom_tables()?;

    let assets = app::load_assets(&platform, classic)?;
    // The front end is up from the first frame and `Setup`'s own *Start* builds
    // the real game
    // until then. `docs/decisions.md` C117.
    let game = scenario::new_game(
        &assets,
        0,
        &l2_game::setup::SetupOptions::new().commit(1, l2_net::Quirks::default()),
        1,
        // Red, which is `FUN_004978AD`'s seed and what page 4 shows before
        // anybody clicks. This world is only what stands behind the title page.
        1,
        scenario::SEED,
        tables,
    )?;
    // `App_WinMain`'s `Options_Load()` (`00400000.c:8221`), after `L2.eng` and
    // before `App_InitWindow`: `Options_Load` `0x004AE1B8`. `l2_game::prefs`.
    let mut game = game;
    game.prefs = prefs;
    launch::install_id(&mut game.prefs);
    println!(
        "{} counties, {} owned by realm {}, {} {} - a NEW world; the front end starts the real one",
        game.kingdom.county_count,
        game.owned_by(game.player),
        game.player,
        l2_game::screens::map::season_name(game.kingdom.season),
        game.kingdom.year
    );

    let audio = if sound && (classic || !platform.load_order.is_empty()) {
        Audio::open(&platform.vfs)
    } else {
        Audio::silent()
    };
    println!(
        "sound: {} ({} wav files found)",
        if audio.is_live() { "on" } else { "off" },
        audio.file_count()
    );

    let mut machine = Machine::new(ScreenId::Setup(SetupPage::Title));
    // **And over it, the intro** — `App_WinMain`'s `FUN_004B3571(0)`, which
    // chains to the Impressions logo and the credits before the title page is
    // seen. A missing film goes straight to the title page.
    match &net {
        Some(flags) => {
            launch::open_lobby(&mut game, flags);
            machine.push(ScreenId::Lobby);
        }
        None if classic => l2_game::movie::start_up(&mut machine),
        None => {}
    }

    let mut canvas = Canvas::screen();
    if game.prefs.sharp_text {
        canvas.enable_sharp();
    }
    let pictures = assets.cursor_pictures(&platform.vfs);
    let mut app = App {
        game,
        assets,
        audio,
        director: l2_game::audio::Director::new(),
        machine,
        canvas,
        window: None,
        pixels: None,
        started: Instant::now(),
        ticker: l2_game::clock::Ticker::new(),
        last_cursor: (0, 0),
        ctrl: false,
        alt: false,
        saved_prefs: prefs,
        last_press: None,
        left: l2_game::input::LeftButton::new(),
        pointer: l2_game::cursor::Pointer::Arrow,
        pictures,
        cursors: Vec::new(),
        cursor_scale: 0,
        net: None,
        live: None,
        remote: remote::Remote::from_env(),
    };

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    event_loop.run_app(&mut app)?;
    // `FUN_0049767D`'s `Options_Save()` (`00490000.c:3084`), reached only from
    // `0x0040EBFE` after the message loop ends.
    if let Err(e) = l2_game::prefs::save(&app.game.prefs) {
        eprintln!("preferences: {e}");
    }
    Ok(())
}

