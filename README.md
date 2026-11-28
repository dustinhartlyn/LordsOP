# open-lords2

**Open Lords II** is a standalone medieval strategy game inspired by **Lords of
the Realm II** (Sierra/Impressions, 1996). No stock installation is needed.
The executable includes original artwork, maps and battlefield layouts, plus
scalable interface fonts. No publisher assets are distributed; see [NOTICE](NOTICE).

## What works

A single-player game plays from the front end to victory or defeat:

- **Kingdom** — the whole end-of-season economy: taxes, rations, grain, cattle, weather,
  industry, wages, ale, health, happiness, births, migration, events, revolts.
- **Armies** — raising, the armoury, mercenaries, marching, splitting, garrisons, foraging,
  taking counties, fog of war.
- **Battles and sieges** — field battles with the battle AI, auto-calculated results,
  castle building, laying siege, storming, moats and burning oil.
- **AI lords** — farming, war, sieges, diplomacy and alliances, trade.
- **Merchants** — routes, trading and the neutral market.
- **Artwork and worlds** — 14 provinces across two campaign tracks and custom
  games, 35 skirmish presets, seasonal maps, village and management screens,
  animated units, and illustrated battle and campaign events.
- **Saving** — save, load and autosave.

[`docs/status.html`](docs/status.html) grades every feature against the original, with the
evidence for each.

## Network play

*Multiple players* opens the lobby. The host presses *Host a game* and the lobby shows the
address others type (its LAN address and port 4712). Windows asks once whether to let the
game through its firewall: allow it on private networks, or open TCP port 4712. A peer who
leaves or goes silent stops the game with their name; save it to go on alone against the
computer. Peers should use the same build, asset mode and mods.

## What does not, yet

- **Network play** — deterministic lockstep is built and tested; the lobby and game over a
  network are in progress.
- A few gaps are named in the status page: closing archers in battle, one AI attack, some
  narration and cursor pictures.

## Playing

Build and run with Rust and Cargo:

```
cargo run --release -p l2-game
```

Pass `-- --mods <dir>` to enable mods. Standalone games use `open-lords2/standalone/saves`
under the platform's data directory.

Standalone uses an original ray-traced castle title, illustrated heraldry, and
full-color campaign and battle art at up to 1920×1440. Seasonal ground,
pasture walls, fields, ridge faces and siege-yard approaches are drawn from
the map. The campaign renderer retains world-space scenery while scrolling.
Cattle walk and graze independently within their pastures. These graphics
don't change movement rules.
The title's POV-Ray scene and regeneration script are in
[`crates/l2-game/src/original/art`](crates/l2-game/src/original/art).
It fits the available window at 4:3 without changing the logical control
layout. Classic mode and indexed artwork/palette mods retain their original
rendering.

In standalone mode, the campaign dashboard shows the selected county's
population, food, tax, and workforce alongside the minimap and county actions.
Open **Village** there to inspect job sites. To move workers in your own
county, drag a box around the groups on one job card, release, then click
the destination card. Right-click to cancel a held selection.

Proposed strategy changes and playtest goals are tracked in the
[standalone mechanics backlog](docs/standalone-mechanics.md).

**Optional classic mode:** pass a Lords of the Realm II install directory after `--`,
or set `LORDS2_DIR`, to use your own stock artwork, music, narration and films.
The engine only reads that installation. Classic saves stay in `open-lords2/saves`
and are not interchangeable with standalone saves. The standalone edition is silent
without a sound mod; its event illustrations replace the classic films.

## How it is built

`Lords2.exe` is an oracle, not a target: we read the original binary to answer what the game
does, and where it disagrees with us it is right. Of its
<!--fig:binary-functions-->2,452<!--/fig--> functions we have named
**<!--fig:functions-->1,464<!--/fig-->**, and **<!--fig:tests-->3,345<!--/fig--> tests pass**.

## Release archive

`powershell -File tools/run/package.ps1` builds a release and writes
`dist/open-lords2-<commit>.zip`: the executable, LICENSE, NOTICE and
[`tools/run/README-player.txt`](tools/run/README-player.txt), which covers launching,
saves and controls. The archive needs no separate artwork download.

## Contributing

Developers start at [CLAUDE.md](CLAUDE.md), the index to `docs/`. The most valuable thing
anyone can offer is **memory of playing the game**: [`docs/mechanics.md`](docs/mechanics.md)
exists to be read by a player and contradicted. If you remember a mechanic it does not
list, please open an issue.

## Licence

MIT — see [LICENSE](LICENSE) and [NOTICE](NOTICE).

This is an independent reimplementation containing no code, art, sound or data from the
original. Lords of the Realm II is copyright Sierra On-Line / Impressions Games and its
present rights holders, who are not affiliated with this project and do not endorse it.
