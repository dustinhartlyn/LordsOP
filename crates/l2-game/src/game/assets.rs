#![allow(unused_imports)]
use super::*;

use unit_frames::*;
use levy::*;
use game_methods::*;
use l2_formats::maps::{MapSet, MapSlot};
use l2_formats::Palette;
use l2_kingdom::county::{LABOUR_CEILING_IGNORED, MAX_COUNTIES};
use l2_kingdom::realm::MAX_REALMS;
use l2_kingdom::tables::JOB_IDLE_TOWNSFOLK;
use l2_kingdom::{Kingdom, SeasonReport};
use l2_mods::vfs::Vfs;
use l2_view::campaign::{self, MapAssets};
use l2_view::chrome::{Chrome, Minimap};
use l2_view::village::VillageArt;
use l2_view::Ink;
use crate::shell::ShellAssets;
use l2_kingdom::tables::MAX_TAX_RATE;
use l2_kingdom::county::MAX_RATION_SPLIT;

pub struct Assets {
    pub palette: Palette,
    pub ink: Ink,
    pub map: MapAssets,
    pub quirks: Quirks,
    pub wall_clock: Option<i64>,
    pub chrome: Option<Chrome>,
    pub village: Option<VillageArt>,
    /// `T32_bat1.pl8`, its palette and six colours of seven troop sheets — the
    /// battlefield's own artwork, loaded by `Battle_LoadAssets` (`0x004987B7`)
    /// and by nothing else.
    ///
    /// `None` on a partial install, and the battlefield then draws its own flat
    /// ground and a block for each man. That keeps every input arm testable
    /// without the install, and it is safe here for a reason the campaign map's
    /// hit test was not (`docs/decisions.md` C61): **every hotspot on this
    /// screen is a constant out of the binary**, not a consequence of the
    /// artwork,
    pub battle: Option<l2_view::scene::BattleAssets>,
    /// What the shell screens draw with: `L2.eng`, the two panel fonts, and
    /// the per-screen artwork the front end and the management screens load.
    pub shell: ShellAssets,
    pub films: crate::movie::FilmFiles,
    /// `L2help.hlp`, which `Opt_GameHelpContents` (`0x00434942`) hands to `WinHelpA`; `None` on the DOS release.
    pub help: Option<l2_formats::hlp::Hlp>,
    maps: Vec<u8>,
    minimap_files: Vec<Option<Vec<u8>>>,
    original: bool,
}

impl Assets {
    pub fn load(vfs: &Vfs) -> Result<Assets, String> {
        Self::load_from(vfs, false)
    }

    /// The built-in world and artwork, with explicitly enabled mods layered above it.
    pub fn load_original(vfs: &Vfs) -> Result<Assets, String> {
        Self::load_from(vfs, true)
    }

    fn load_from(vfs: &Vfs, original: bool) -> Result<Assets, String> {
        let read = |name: &str| match vfs.read(name) {
            Ok(bytes) => Ok(bytes),
            Err(l2_mods::vfs::VfsError::NotFound(_)) if original => {
                crate::original::asset(name).ok_or_else(|| format!("{name}: no bundled asset"))
            }
            Err(e) => Err(format!("{name}: {e}")),
        };
        let maps = read("L2_maps.dat")?;
        let maps_set = MapSet::parse(&maps).map_err(|e| format!("L2_maps.dat: {e}"))?;
        let palette = Palette::from_bytes(&read(campaign::PALETTE)?)
            .map_err(|e| format!("{}: {e}", campaign::PALETTE))?;
        let map = MapAssets::load(read)?;
        let chrome = optional_art(original, Chrome::load(read))?;
        let village = optional_art(original, VillageArt::load(read))?;
        let battle = optional_art(original, l2_view::scene::BattleAssets::load(
            read,
            l2_view::figures::Colour::Red,
            l2_view::figures::Colour::Blue,
        ))?;
        let minimap_files = (0..16)
            .map(|n| vfs.read(&Minimap::file_for_slot(n * 4)).ok().or_else(|| {
                if original { crate::original::minimaps(&maps_set, n) } else { None }
            }))
            .collect();
        crate::castle::publish_from(|name| read(name).ok());
        crate::batfield::publish_from(|name| read(name).ok());
        Ok(Assets {
            ink: Ink::for_palette(&palette),
            quirks: Quirks::default(),
            wall_clock: None,
            palette,
            map,
            chrome,
            village,
            battle,
            shell: if original { ShellAssets::load_original(vfs, crate::original::asset) }
                else { ShellAssets::load(vfs) },
            films: crate::movie::FilmFiles::index(vfs),
            help: vfs.read("L2help.hlp").ok().and_then(|b| l2_formats::hlp::Hlp::parse(&b)),
            maps,
            minimap_files,
            original,
        })
    }

    pub fn slot(&self, index: usize) -> Option<MapSlot<'_>> {
        MapSet::parse(&self.maps).ok()?.slot(index).ok()
    }

    /// `FUN_0046A101` `0x0046A101`: the slots whose `MAPnn.PL8` opens, packed
    /// into `DAT_0050A460`; its length is `DAT_00554018`. [V]
    pub fn scenario_slots(&self) -> Vec<usize> {
        (0..60).filter(|&s| {
            matches!(self.minimap_files.get(s >> 2), Some(Some(_)))
                && (!self.original || self.slot(s).is_some_and(|slot| !slot.is_empty()))
        }).collect()
    }

    pub fn minimap(&self, slot: usize) -> Option<Minimap> {
        let bytes = self.minimap_files.get(slot >> 2)?.as_ref()?;
        Minimap::load(bytes, slot).ok()
    }

    pub fn is_original(&self) -> bool {
        self.original
    }

    pub fn cursor_pictures(&self, vfs: &Vfs) -> Vec<l2_formats::cursors::Picture> {
        if self.original {
            crate::original::pictures(&self.palette)
        } else {
            vfs.read(crate::scenario::EXECUTABLE)
                .ok()
                .and_then(|b| l2_formats::cursors::read(&b).ok())
                .unwrap_or_default()
        }
    }

    pub fn placeholder() -> Assets {
        let mut palette_bytes = vec![0u8; Palette::FILE_LEN];
        for i in 0..256usize {
            let v = (i / 4) as u8;
            palette_bytes[i * 3] = v;
            palette_bytes[i * 3 + 1] = v;
            palette_bytes[i * 3 + 2] = v;
        }
        let palette = Palette::from_bytes(&palette_bytes).expect("768 bytes");

        let mut pl8 = vec![0u8; 8 + 16];
        pl8[2] = 1; // one frame
        pl8[8] = 2; // width
        pl8[10] = 2; // height
        pl8[12..16].copy_from_slice(&24u32.to_le_bytes());
        pl8.extend_from_slice(&[1, 2, 3, 4]);
        let map = MapAssets::load(|_| Ok(pl8.clone())).expect("a synthetic sheet parses");

        Assets {
            ink: Ink::for_palette(&palette),
            quirks: Quirks::default(),
            wall_clock: None,
            palette,
            map,
            chrome: None,
            village: None,
            battle: None,
            shell: ShellAssets::empty(),
            films: crate::movie::FilmFiles::default(),
            help: None,
            maps: vec![0u8; l2_formats::maps::SLOT_LEN],
            minimap_files: vec![None; 16],
            original: false,
        }
    }
}

fn optional_art<T>(original: bool, result: Result<T, String>) -> Result<Option<T>, String> {
    if original { result.map(Some) } else { Ok(result.ok()) }
}

