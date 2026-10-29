# fuzz/seeds

Real files the fuzzers start from, one folder per target in [`../fuzz_targets`](../fuzz_targets/README.md). A fuzzer changes copies of these, so starting from real files reaches deep into each reader quickly. The folders hold only seed files and have no README of their own, because the fuzzers read everything in them.

| Folder | What it holds |
| --- | --- |
| `area_file/` | Area files (`.solarea`), including the turret sample's. |
| `frames/` | A sprite's clips file (`.solframes`), as the fixture of version 1. |
| `game_code/` | Each sample's `main.lst`, and the turret's Lust and Lua joined by a 0x01 byte. |
| `import_cache/` | Import cache entries (`.solbin`) of the sample models. |
| `input_actions/` | Input action files (`.solinput`), with one of format 1 (no version line). |
| `model/` | The sample models (`.glb`). |
| `project_file/` | A small project file. |
| `recording/` | Recordings of play (`.solrec`): a long one, and short ones of formats 1 and 2, so the upgrader is fuzzed too. |
| `save_game/` | Save games (`.solsave`): the current version and versions 1 and 2, so the upgraders are fuzzed too. |
| `sidecar/` | Sidecars (`.solmeta`) of the sample models. |
| `sound_event/` | A full sound event, an empty one and a looping 2D one. |
| `sound_file/` | WAV files of each kind the reader takes, and Ogg Vorbis files from `crates/sound/tests/data`. |
| `tile_art/` | The logo's PNG and the walker's Aseprite file, each after the byte that chooses its reader. |
| `tile_map/` | Tile map files (`.soltilemap`), as the fixtures of version 1 and of version 2 (with exact tiles). |
| `tileset/` | Tilesets (`.soltileset`): the fixtures of version 1 and of version 2 (art, every kind, rules). |
| `update_answer/` | A release answer and its checksum file, joined by a 0x01 byte. |

They are made by [`../make-seeds.sh`](../make-seeds.sh) from the editor's tests, the sample models and the samples; run it again after a file format changes. What each fuzzing run finds goes in `fuzz/corpus/`, which isn't committed.
