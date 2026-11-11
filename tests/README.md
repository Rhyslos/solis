# tests

Test data for the whole program, read by the crates' tests.

## fixtures/

Example files of every version of every file format Solis has ever released (Phase 8, C1; [decision 0053](../docs/decisions/0053-compatibility-before-2d.md)). A test in the app (`crates/app/src/fixtures_tests.rs`) opens every one with the current build, which upgrades it in memory, so a build that can't open an older file can't be merged. They're never changed: when a format gets a new version, its new folder is added beside the old ones.

| Folder | Format | Versions |
| --- | --- | --- |
| `area/` | Areas (`.solarea`) and assemblies (`.solassembly`) | v1, v2 (a placed assembly's part moved out to the top level, `parent = "";`) |
| `sound-event/` | Sound events (`.solsound`) | v1 |
| `mixer/` | The project's buses (`mixer.solmix`) | v1 |
| `pixels/` | The project's 2D settings (`pixels.solpix`) | v1 |
| `frames/` | A sprite's clips (`.solframes`) | v1 |
| `animator/` | An animator (`.solanim`) | v1 |
| `tileset/` | A tileset (`.soltileset`, then `.tileset.lst`) | v1, v2 (its art: picture, kinds, pieces, rules), v3 (solid terrains), v4 (as Lust, `farm.tileset.lst`) |
| `tilemap/` | A tile map (`.soltilemap`, then `.tilemap.lst`) | v1, v2 (exact tiles), v3 (as Lust, `farm.tilemap.lst`) |
| `input-actions/` | Input actions (`input.solinput`) | v1 (no version line; the arrow keys written `ArrowUp`), v2 |
| `recording/` | Recordings of a play (`.solrec`) | v1, v2 (names the area it was recorded in), v3 (what screens' buttons sent) |
| `atlas/` | A picture's crops (`.atlas.lst`, Atlas Crop) | v1 |
| `project/` | The project file (`project.solis`) | v1 |
| `sidecar/` | Sidecars beside assets (`.solmeta`) | v1 |
| `save/` | Save games (`.solsave`), opened by `crates/editor/src/save_game_tests.rs`, which loads each and plays on | v1 (physics as Rapier's bytes, never read now), v2 (the game's places in the area; one with the first builds' checksum of the body only), v3, v4 (animators' parameters), v5 (tile maps as the game changed them), v6 (the game-wide objects), v7 (the game's speed), v8 (the hours game code moved the day on by) |
| `editor-settings/` | The editor's own settings on this computer (eframe's `app.ron`: a map of keys to values), which have no version number: each key is read by itself, with a fallback. Opened by `crates/app/src/saved_look_tests.rs`. | `before-g-layout/`: three files as the last build before the editor's look was remade wrote them (decision 0100): `standard-blue.ron` (a first run), `graphite-rounded-plum.ron` (the Graphite theme, rounded panels, the Plum accent, a rearranged layout, everything else changed too) and `before-accent2.ron` (only the keys of the time before decision 0070: `accent`, `panel_layout`, `recent_projects`) |

Each `v<N>/` folder holds files as that version wrote them, byte for byte: the repository's `.gitattributes` stops Git changing their line endings on Windows (a save carries a checksum). `fixtures/` and its folders are data, so they have no README of their own.
