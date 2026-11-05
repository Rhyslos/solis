# 0081. What Solis makes is Lust

- **Status:** Accepted (the project lead chose the names and the order, 2026-11-05; carries out 0070's second step; builds on 0020 and 0053)
- **Date:** 2026-11-05

## Problem

Decision 0070 set "one project file, the rest as Lust": what the editor makes should be Lust data files, read with Lust's own `lust::read_values`, so a project is all Lust and every file can be read in Ledger. Until now each kind of file had its own Lust-like layout (`tilemap version 2;`, blocks of `key = value;`), close to Lust but not Lust: Ledger couldn't open them as Lust, and `lust check` couldn't check them.

## Options

- **File names:** keep the old endings with Lust inside (`farm.soltilemap`); plain `farm.lst`, with the kind worked out from what's inside; or `farm.tilemap.lst`.
- **What moves:** everything but the project file, saves and recordings; or everything, the project file too.
- **Order:** every file first; the base and the farm game's files first, then the 2D camera and the UI, then the rest; or the UI first.

## Decision

- **Every file Solis makes is a Lust data file named for what it holds**, `<name>.<kind>.lst`: `farm.tilemap.lst`, `ground.tileset.lst`. Ledger opens and colours them all; the word before `.lst` says what each is (`reflect::data_files`, the one list of kinds). The project lead chose this.
- **Only `project.solis` stays as it is**: the one file that opens a project. Everything else becomes Lust, sidecars, save games and recordings too (the project lead first said everything, then kept the project file).
- **A data file is ordinary Lust**: a `fetch` of its classes from `solis/data/<kind>`, `lock int version = N;`, then `lock` values of fixed values (things in lists, one a line), and `end`. Lust's reader runs nothing and checks every type, field and option, with the mistake's line. The classes are Lust too, kept with the crate that reads the format (`tiles::map_lust::CLASSES`) and written into every project's `solis/data/` with the engine's other files, so `lust check` and Ledger can follow the fetch; reading hands Lust those classes itself, so a file never depends on what's beside it on disk.
- **Lust lists hold one kind of thing**, so each format gets its own shape rather than a line-by-line translation (a tile map's chunk values become `Numbers`, `Switches` and `Choices`). Formats whose parts are blocks of settings share `area::lust_data::{write_blocks, read_blocks}`: the top-level settings as one thing, each kind of block as a list, read back into the same blocks so one set of checks reads either layout.
- **Numbers are exact:** an `f32` is written as its own shortest text (`0.42`), which reads back as exactly that `f32`; other numbers as their shortest `f64` text.
- **Older files move when a project opens** (`app::to_lust`): each is read, written as Lust beside it, checked (it must read back and write the same), its sidecar moves with it (so its ID, and everything pointing at it, stays), and the older file goes into the backups (`asset_db::backup::move_to_new_format`), never simply deleted. One that can't move stays, read as before, and the Console says why. Every reader still reads the older layout too, so `--check-project` and older example files keep working.
- **Order** (the project lead's choice): the base and tile maps and tilesets first; then the 2D camera and the UI, with UI screens Lust from the start; then the other files, one pull request each (plan: [docs/plans/lust-data.md](../plans/lust-data.md)).
- Data files are never part of the game's program: the code gatherer leaves `*.<kind>.lst` out.

## Consequences

- A tile map is version 3 and a tileset version 4, as Lust; the older layouts are read and moved.
- Reading goes through Lust's checker, about 10 µs per thing in a release build (100,000 entities in a second): fine for every file today. Big areas and saves are measured when they move.
- The engine's code folder name now appears in data files' `fetch` lines (`area::lust_data::ENGINE_FOLDER`, kept equal to `branding::CODE_FOLDER` by a test; RENAMING.md).
