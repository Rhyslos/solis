# fuzz/seeds

Real files the fuzzers start from, one folder per target in [`../fuzz_targets`](../fuzz_targets/README.md). A fuzzer changes copies of these, so starting from real files reaches deep into each reader quickly. The folders hold only seed files and have no README of their own, because the fuzzers read everything in them.

| Folder | What it holds |
| --- | --- |
| `area_file/` | Area files (`.solarea`), including the turret sample's. |
| `game_code/` | Each sample's `main.lst`, and the turret's Lust and Lua joined by a 0x01 byte. |
| `import_cache/` | Import cache entries (`.solbin`) of the sample models. |
| `input_actions/` | Input action files (`.solinput`). |
| `model/` | The sample models (`.glb`). |
| `project_file/` | A small project file. |
| `recording/` | A recording of play (`.solrec`). |
| `save_game/` | A save game (`.solsave`). |
| `sidecar/` | Sidecars (`.solmeta`) of the sample models. |
| `sound_event/` | A full sound event, an empty one and a looping 2D one. |
| `sound_file/` | WAV files of each kind the reader takes, and Ogg Vorbis files from `crates/sound/tests/data`. |
| `update_answer/` | A release answer and its checksum file, joined by a 0x01 byte. |

They are made by [`../make-seeds.sh`](../make-seeds.sh) from the editor's tests, the sample models and the samples; run it again after a file format changes. What each fuzzing run finds goes in `fuzz/corpus/`, which isn't committed.
