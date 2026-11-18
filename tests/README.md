# tests

Test data for the whole program, read by the crates' tests.

## fixtures/

Example files of every version of every file format Solis has ever released (Phase 8, C1; [decision 0053](../docs/decisions/0053-compatibility-before-2d.md)). A test in the app (`crates/app/src/fixtures_tests.rs`) opens every one with the current build, which upgrades it in memory, so a build that can't open an older file can't be merged. They're never changed: when a format gets a new version, its new folder is added beside the old ones.

| Folder | Format | Versions |
| --- | --- | --- |
| `area/` | Levels (`.sollevel`; areas, `.solarea`, until v4; the folder keeps the old name, since released examples never move) and an assembly's older file (`.solassembly`, what prefabs were until format 1 of `prefab/`) | v1, v2 (a placed prefab's part moved out to the top level, `parent = "";`), v3 (brushes: a box, a ramp and stairs, their points and faces a few numbers to a line), v4 (areas renamed levels, decision 0118: the header says `level`; `blockout.sollevel` is v3's `blockout.solarea` as this build writes it), v5 (assemblies renamed prefabs: a copy says `prefab = "ID";`; `yard.sollevel` places one; `zones.sollevel`, trigger zones (stage 1): a box, a circle that fires once and a freeform L, a new component that needed no new version) |
| `prefab/` | Prefabs (`.prefab.lst`, Lust, decision 0081) | v1 (`crate.prefab.lst` is `area/v4/crate.solassembly` as this build moves it) |
| `sound-event/` | Sound events (`.solsound`) | v1 |
| `mixer/` | The project's buses (`mixer.solmix`) | v1 |
| `pixels/` | The project's 2D settings (`pixels.solpix`) | v1 |
| `tags/` | The project's tags (`tags.lst`, Lust, stage 1) | v1 |
| `frames/` | A sprite's clips (`.solframes`) | v1 |
| `animator/` | An animator (`.solanim`) | v1 |
| `tileset/` | A tileset (`.soltileset`, then `.tileset.lst`) | v1, v2 (its art: picture, kinds, pieces, rules), v3 (solid terrains), v4 (as Lust, `farm.tileset.lst`) |
| `tilemap/` | A tile map (`.soltilemap`, then `.tilemap.lst`) | v1, v2 (exact tiles), v3 (as Lust, `farm.tilemap.lst`) |
| `input-actions/` | Input actions (`input.solinput`) | v1 (no version line; the arrow keys written `ArrowUp`), v2 |
| `recording/` | Recordings of a play (`.solrec`) | v1, v2 (names the area it was recorded in), v3 (what screens' buttons sent), v4 (the line says `level` and its file ends `.sollevel`, decision 0118) |
| `atlas/` | A picture's crops (`.atlas.lst`, Atlas Crop) | v1 |
| `project/` | The project file (`project.solis`); `v1/kind-2d.solis` is one with the optional `kind` key (decision 0118), which needed no new version | v1, v2 (a project whose levels are `.sollevel` files, decision 0118: nothing in the file changes, but a build from before refuses it as newer), v3 (a project whose prefabs are `.prefab.lst` files, likewise) |
| `playtest/` | How a game is exported (`playtest.txt` beside an exported game, and `.solis/last export.txt`) | v1 (its level line said `area`, ending `.solarea`), v2 (`level`, `.sollevel`; decision 0118) |
| `sidecar/` | Sidecars beside assets (`.solmeta`) | v1 |
| `save/` | Save games (`.solsave`), opened by `crates/editor/src/save_game_tests.rs`, which loads each and plays on | v1 (physics as Rapier's bytes, never read now), v2 (the game's places in the level; one with the first builds' checksum of the body only), v3, v4 (animators' parameters), v5 (tile maps as the game changed them), v6 (the game-wide objects), v7 (the game's speed), v8 (the hours game code moved the day on by), v9 (the level's section is called `level`, decision 0118) |
| `editor-settings/` | The editor's own settings on this computer (eframe's `app.ron`: a map of keys to values), which have no version number: each key is read by itself, with a fallback. Opened by `crates/app/src/window_saved_tests.rs`. | `before-g-layout/`: three files as the last build before the editor's look was remade wrote them (decision 0100): `standard-blue.ron` (a first run), `graphite-rounded-plum.ron` (the Graphite theme, rounded panels, the Plum accent, a rearranged layout, everything else changed too) and `before-accent2.ron` (only the keys of the time before decision 0070: `accent`, `panel_layout`, `recent_projects`); `before-frame/side-layouts.ron`: as the rethink's R2 wrote it, before one frame on every tab (decision 0118): decision 0109's "Level and Assets on the left" layout and the shelf's tabs, folds and sizes in points, read into their nearest |

## fixtures/ui-sample/

Not a file format's examples but a whole small project, "Harbor_Prototype", named after the sample content of the editor's design (`docs/design/g-layout/`): one level (`docks_night.sollevel`: a crane, two stacks of crates and a row of bollards under **Props**, the sun and a lamp under **Lighting**, a camera under **Gameplay**), and a handful of files of different kinds for the Library (the level, game code with one component, a note, a sound, and in `props/` a model and a picture, each asset with the `.solmeta` file Solis keeps beside it). About 10 KB in all. The editor's picture tests open it (`crates/editor/src/picture_support.rs`, `sample_project`), so that their approved picture of the editor shows real content and a later milestone's changes are seen on it.

Unlike the versioned examples it may grow when a milestone needs more to show, and a picture approved from it is then approved again. It is checked the way any project is before a release: `solis --check-project` on a copy of it reports that it opens, its files read and its code checks. **Open a copy, never the folder itself:** Solis writes its own files into a project it opens (the engine's Lust files, its cache).

Each `v<N>/` folder holds files as that version wrote them, byte for byte: the repository's `.gitattributes` stops Git changing their line endings on Windows (a save carries a checksum). `fixtures/` and its folders are data, so they have no README of their own.

## two-pcs/

The two-PC check's game (stage 1): a small project with a little of every kind of game state, ten seconds of recorded play, and the state it must come out as at every tick, made on Linux. `crates/app/src/two_pcs.rs` plays it back in every test run (on GitHub's Windows machine too) on one core and on four, and fails at the first tick that differs, naming the parts. Its own [README](two-pcs/README.md) says what's in it and how to make it again.
