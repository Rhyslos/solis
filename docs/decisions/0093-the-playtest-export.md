# 0093. The playtest export

- **Status:** Accepted
- **Date:** 2026-11-06

## Problem

The usable-first build order (decision 0070) ends with the playtest export (Phase 11, brought forward): "a build a friend can play" is part of the gate. ROADMAP.md asks for File > Export for playtesting: a folder to zip and send, with a game program without the editor and the project's files packed beside it; Lust and Lua running as in Play, so there's no long compile; windowed, fullscreen and resolution settings; saves in the player's saved-games folder; and a crash log playtesters can send back. Building a game's own Rust code, an installer and Steam wait for the full export (Phase 18).

The question was what the program in the folder is.

## Options

1. **Solis, renamed.** The folder holds this same program, named after the game, with a small file beside it saying what to play. Started there, it opens straight into the game: no launcher, no editor.
2. A separate player program, built alongside Solis, holding only what playing needs.
3. Solis as it is, opened with a command-line option.

## Decision

The project lead's choice (2026-11-06): **Solis, renamed (Recommended)**, and the export window as mocked up and approved (`runmock/b.png`).

- **File > Export for playtesting…** (above Quit, and in the search) asks for:
  - **Game's name:** the program's name, the window's title and the saves folder's. Characters Windows refuses in file names are taken out.
  - **Starts in:** one of the project's areas, the open one at first.
  - **Into the folder:** by default `Documents/<Game> exports/<Game> playtest <date>`, a new dated folder each time so old builds stay. A folder that has files in it already is never used, and nor is one inside the project.
  - **Window:** windowed or fullscreen, and its size, as the game first opens.
  
  The project remembers these for next time (in its own `.solis` folder, not shared in Git).
- **What the folder holds:**
  - `<Game>.exe`: this program, renamed;
  - `playtest.txt`: the game's name, the area it starts in, and its first window;
  - `game/`: the project's files. Left out: the cache (`.solis`), `builds`, the editor's saves and recordings, backups, Git's history, and links to elsewhere.
  
  It's copied on a thread of its own, so the editor keeps drawing; the Console and the window say where it went, with **Open folder**.
- **The exported game.** Started from the folder, the program finds `playtest.txt` beside itself and plays.
  - It opens the project in `game/` and plays its area as soon as its code is read, filling the window.
  - The game has the mouse and keys from the start, Esc included, and keeps them when the window loses focus.
  - It never asks GitHub about updates and never shows the editor. A playtest file that can't be read is explained in a message, not ignored.
  - Code with mistakes shows them in the window instead of starting.
- **`Window:quit()`** (new, in `solis/window`) closes the game at the end of the tick, for a menu's Quit button. While playing in the editor, play stops and the Console says the exported game would close.
- **The player's settings** (decision 0091) are real in an exported game:
  - **Alt+Enter**, `Window:setFullscreen` and `Window:setResolution` change the window;
  - the settings are kept in `settings.txt` in the player's folder;
  - the next start uses them, or the export's window the first time.
- **The player's folder** is `Saved Games/<Game>` on Windows (the usual data folder elsewhere). It holds:
  - the saves (`saves/`);
  - the settings;
  - `log.txt` (the game's warnings and mistakes);
  - `crash log.txt`, written if the program stops unexpectedly. It says what happened and the last 40 things the game said.

## Consequences

- The program has two modes, chosen by what's beside it. A stray `playtest.txt` next to the editor's own program would make it a game, so the installer's folder must never hold one.
- The export is as big as the program and the project (the program alone is tens of megabytes in a release build).
- Exported games run Lust and Lua as Play does. A game's own Rust components wait for Phase 18. The turret sample's built-in ones work, since they're part of the program.
- Everything game code can do while playing in the editor works the same in the export, so a playtest finds what the editor's Play would.
