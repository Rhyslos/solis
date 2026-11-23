# fuzz/seeds

Real files the fuzzers start from, one folder for each target in [`../fuzz_targets`](../fuzz_targets/README.md) that has any. A fuzzer changes copies of these, so starting from real files reaches deep into each reader quickly; a target with no folder here is still fuzzed, starting from nothing. The folders hold only seed files and have no README of their own, because the fuzzers read everything in them.

| Folder | What it holds |
| --- | --- |
| `animator/` | An animator file (`.solanim`), as the fixture of version 1. |
| `level_file/` | Level files (`.sollevel`): the turret sample's, the trigger zones' example (`zones.sollevel`: a box, a circle and a freeform L), and two of version 1 (so the upgrader is fuzzed too), one of them one whose entities are inside each other in loops (one its own parent, two and three in a ring, a chain hanging under a ring) or under a parent that isn't in the file. |
| `translation_file/` | A translation file: the Norwegian shop words the `translations` crate's tests read (`crates/translations/tests/data/shop.ftl`), with values, a plural, an attribute and a term. |
| `tags_file/` | The project's tags: the example of version 1 (`tags.lst`). |
| `module_file/` | Module files: the example of version 1 (`.module.lst`), and an assembly's older file of layouts 1 and 4 (`.solassembly`). |
| `aseprite/` | Two small Aseprite files written by the `assets` crate's tests: the walker, and one in indexed colour (a palette). |
| `atlas/` | Atlas files (`.atlas.lst`): the fixture of version 1, and a hostile one from `crates/animation/tests/data` that the reader refuses (a control character in its picture's name, which was read but not written back: the file the fuzzer's first run found, made small). |
| `frames/` | A sprite's clips file (`.solframes`), as the fixture of version 1. |
| `clips_file/` | A picture's clips file (`.clips.lst`), as the fixture of version 2. |
| `brush/` | A level of brushes: the level fixture of version 3 (a box, a ramp and stairs). |
| `game_code/` | Each sample's `main.lst`, and the turret's Lust and Lua joined by a 0x01 byte. |
| `import_cache/` | Import cache entries (`.solbin`) of the sample models. |
| `input_actions/` | Input action files (`.solinput`), with one of format 1 (no version line). |
| `mixer/` | Bus files (`.solmix`): the standard buses, nested ones with ducking, and one whose buses are inside each other in a loop. |
| `model/` | The sample models (`.glb`), a cube whose 64 extra buffers all claim the file's binary part (refused), and a triangle with a corner placed past what numbers can hold (refused; the file the fuzzer found, issue #157, made small). |
| `pixels/` | 2D settings files (`.solpix`): the standard ones, ones that fill the window, and ones with values the reader must put right or skip. |
| `playtest_file/` | Playtest files: a usual one, one the reader puts right (Windows line endings, a name with characters a file can't have, a window too big and too small, and the `folder` line older copies held), and one it refuses (a level outside the assets). |
| `project_file/` | A small project file. |
| `recording/` | Recordings of play (`.solrec`): a long one, and short ones of formats 1 and 2, so the upgrader is fuzzed too. |
| `save_game/` | Save games (`.solsave`): the current version and versions 1 and 2, so the upgraders are fuzzed too, and one with a single very long line (thousands of words where a body has sixteen), which the reader refuses without listing them all. |
| `sidecar/` | Sidecars (`.solmeta`) of the sample models. |
| `sound_event/` | A full sound event, an empty one and a looping 2D one. |
| `sound_file/` | WAV files of each kind the reader takes, and Ogg Vorbis files from `crates/sound/tests/data`: good ones (one of many pages), and hostile ones the reader refuses (a second stream after a good sound, one of them hidden behind a stray byte). |
| `tile_art/` | The logo's PNG and the walker's Aseprite file, each after the byte that chooses its reader. |
| `tile_map/` | Tile map files: the fixtures of version 1 and of version 2 (with exact tiles) as `.soltilemap`, and of version 3 as `.tilemap.lst`. |
| `font_file/` | JetBrains Mono, one of the fonts in the program (the smaller one), as a project's own font; the target lays it out beside Inter. |
| `text_file/` | A Markdown file using every mark Properties draws, and the project's CONTRIBUTING.md. |
| `tileset/` | Tilesets: the fixtures of version 1 and of version 2 (`.soltileset`; art, every kind, rules) and of version 4 (`.tileset.lst`), and a hostile one from `crates/tiles/tests/data` that the reader refuses (two classes with one name, which it never finished reading: the file the fuzzer's first run found, made small). |
| `update_answer/` | A release answer and its checksum file, joined by a 0x01 byte. |

Most are made by [`../make-seeds.sh`](../make-seeds.sh) from the crates' tests, the sample models, the samples and the example files in `tests/fixtures`; run it again after a file format changes. `mixer/`, `pixels/`, `playtest_file/`, `sound_event/`, `text_file/` and `level_file/parent-loops.sollevel` were written by hand, and the script leaves them alone. What each fuzzing run finds goes in `fuzz/corpus/`, which isn't committed.
