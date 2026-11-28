open-lords2
===========

Open Lords II is a standalone medieval strategy game inspired by Lords of
the Realm II (Sierra / Impressions, 1996). The included worlds and art are
original to this project. No publisher assets are distributed. See NOTICE
for the standalone interface font license.

Start playing
-------------

Unpack the archive and launch l2-game.exe on Windows or ./l2-game on Linux.
No installation directory or extra asset download is needed.
Windows uses Direct3D 12 by default. No graphics environment variable is needed.

Standalone fits a 4:3 picture to the window. Campaign terrain, the village
panorama and battle maps render at up to 1920 x 1440 with scalable text.
Classic mode and indexed graphics mods keep pixel-perfect scaling.

For optional classic mode, supply your own read-only Lords of the Realm II
folder as a command-line argument or set LORDS2_DIR. The folder must contain
L2_maps.dat and Lords2.exe (or L2.eng). Leave both unset for standalone mode.
Standalone is silent without a sound mod; illustrated events replace the
classic films.

Where your files go
-------------------

Saves, settings (prefs.ini) and crash reports (crash-<date>-<time>.log) are in

  %APPDATA%\open-lords2\standalone\saves             on Windows
  ~/.local/share/open-lords2/standalone/saves        on Linux

Classic mode uses the same paths without the standalone subdirectory. The
two modes' saves are not interchangeable. Set LORDS2_SAVES to use a different
directory. If the game stops unexpectedly, attach the crash report from there
to a bug report.

Keys
----

  Alt+Enter   switch between full screen (the default) and a window
  F5          in a window: snap it to a whole multiple of 640 x 480
  Ctrl+D      show or hide our debug overlay

The in-game Help menu opens the standalone field guide.

On the campaign map, the right-hand dashboard shows county status, policy,
and actions. Choose Village to see what each job does. Click a job card for
details. In your own county, drag a box around worker groups on one card,
release, and click a destination card to reassign them. Right-click cancels
a held selection.

Multiplayer
-----------

Multiple players opens the lobby: Host a game, or Join a game.

  Host     pick your shield and the map, press Start. The room shows a
           six-letter code; tell it to your friends. Start game lights up
           once everyone has pressed I'm ready.
  Join     type the code your host gave you and press Join, then I'm ready.

Codes go through our join-code service, which only pairs the two
connections; no ports to open. Without it, "use an address instead" on the
join page takes the host's address (TCP port 4712 by default; the game asks
the host's router to open it, or forward it by hand). --relay host:port or
LORDS2_RELAY picks another service.

Use the same build, asset mode and mods on every player's machine.

Command line
------------

  l2-game [<classic game folder>] [--mods <dir>] [--no-sound]
          [--host [port] | --join <addr|code>] [--relay <host:port>]
          [--players <n>] [--name <text>] [--shield <1-5>]

  --host opens the lobby hosting at once; --join opens it joining that host,
  by address or by code. With --players, a host presses Start when that many
  seats are ready and a joiner presses Ready at once, so a shortcut can carry
  a friend straight in.

Licence
-------

MIT; see LICENSE. See NOTICE for the rights in the original game.
