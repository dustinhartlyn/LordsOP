//! Selecting an optional classic installation and failing where a player can see it.
//! a release build has no console (`windows_subsystem`), so stderr alone is lost.

use std::path::PathBuf;

use l2_game::install;

use crate::os;

const TITLE: &str = "open-lords2";

pub(crate) struct Args {
    pub dir: Option<PathBuf>,
    pub mods: Option<PathBuf>,
    pub sound: bool,
    pub verify: Option<PathBuf>,
    pub net: Option<NetFlags>,
}

/// `--host [port]` or `--join <addr>` opens the lobby at once (C253).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NetFlags {
    pub host: Option<u16>,
    pub join: Option<String>,
    /// Host: Start when this many seats are ready. Joiner: Ready at once.
    pub players: Option<u8>,
    pub name: Option<String>,
    pub shield: Option<u8>,
}

pub(crate) fn args() -> Args {
    let mut a = Args { dir: None, mods: None, sound: true, verify: None, net: None };
    let mut n = NetFlags { host: None, join: None, players: None, name: None, shield: None };
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let mut it = argv.iter().peekable();
    while let Some(s) = it.next() {
        match s.as_str() {
            "--mods" => a.mods = Some(PathBuf::from(it.next().unwrap_or_else(|| crate::usage()))),
            "--no-sound" => a.sound = false,
            "--host" => {
                let port = it.peek().and_then(|p| p.parse::<u16>().ok());
                if port.is_some() {
                    it.next();
                }
                n.host = Some(port.unwrap_or(l2_game::net::DEFAULT_PORT));
            }
            "--join" => n.join = Some(it.next().unwrap_or_else(|| crate::usage()).clone()),
            // `--relay host:port` is read where `Driver` asks for it (C254).
            "--relay" => l2_game::net::configure_relay(it.next().unwrap_or_else(|| crate::usage())),
            "--players" => {
                n.players = it.next().and_then(|v| v.parse().ok()).or_else(|| crate::usage())
            }
            "--name" => n.name = Some(it.next().unwrap_or_else(|| crate::usage()).clone()),
            "--shield" => {
                n.shield = it.next().and_then(|v| v.parse().ok()).filter(|v| (1..=5).contains(v)).or_else(|| crate::usage())
            }
            "--verify-replay" => {
                a.verify = Some(PathBuf::from(it.next().unwrap_or_else(|| crate::usage())))
            }
            "-h" | "--help" => crate::usage(),
            other => a.dir = Some(PathBuf::from(other)),
        }
    }
    if n.host.is_some() && n.join.is_some() {
        crate::usage();
    }
    if n.host.is_some() || n.join.is_some() {
        a.net = Some(n);
    }
    a
}

/// The lobby as the flags asked for it, instead of the title page and the intro.
pub(crate) fn open_lobby(game: &mut l2_game::Game, flags: &NetFlags) {
    use l2_game::net::{Identity, Request, Setup};
    let name = flags
        .name
        .clone()
        .unwrap_or_else(|| game.prefs.kept.player_name.as_str().to_string());
    game.net.identity = Some(Identity {
        name,
        shield: flags.shield.unwrap_or(1),
        setup: Setup { map: 0, options: game.prefs.kept.setup },
    });
    game.net.auto = flags.players;
    let request = match (&flags.host, &flags.join) {
        (Some(port), _) => Request::Host { addr: Request::every_interface(*port) },
        (None, Some(address)) => Request::Join { address: address.clone() },
        (None, None) => return,
    };
    game.net.requests.push(request);
}

pub(crate) fn fatal(text: &str) -> ! {
    eprintln!("{text}");
    os::error(TITLE, text);
    std::process::exit(1)
}

/// A directory or explicit environment setting opts into classic mode.
/// Never search the user's computer for stock assets.
pub(crate) fn install_dir(arg: Option<PathBuf>) -> Option<PathBuf> {
    let dir = arg.or_else(|| std::env::var_os(install::DIR_VAR)
        .filter(|d| !d.is_empty()).map(PathBuf::from))?;
    if !install::is_install(&dir) {
        fatal(&format!(
            "{} is not a Lords of the Realm II folder: it needs {} and one of {}.",
            dir.display(), install::REQUIRED, install::ANY_OF.join(", ")
        ));
    }
    Some(dir)
}

/// C257: this install's id, made once and kept in `prefs.ini`. Not a secret
/// (`docs/netcode.md` §8): the process's hash seed, the clock and the process id.
pub(crate) fn install_id(prefs: &mut l2_game::game::Prefs) {
    use std::hash::{BuildHasher, Hasher};
    if !prefs.install.is_nil() {
        return;
    }
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
    let mut id = [0u8; 16];
    for (i, half) in id.chunks_mut(8).enumerate() {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u128(now);
        h.write_u32(std::process::id());
        h.write_usize(i);
        half.copy_from_slice(&h.finish().to_le_bytes());
    }
    prefs.install = l2_game::net::InstallId(id);
}

/// Crash log beside the saves, then a box. `crate::crash` formats it.
pub(crate) fn panic_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        default(info);
        let bt = std::backtrace::Backtrace::force_capture().to_string();
        let screen = l2_game::crash::screen();
        let text = l2_game::crash::report(&info.to_string(), screen.as_deref(), &bt);
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let where_ = match l2_game::saves::dir().map(|d| l2_game::crash::write(&d, secs, &text)) {
            Some(Ok(p)) => format!("A report was written to\n{}", p.display()),
            _ => "No report could be written.".to_string(),
        };
        os::error(TITLE, &format!("open-lords2 stopped: {info}\n\n{where_}"));
    }));
}
