# fuzz/fuzz_targets

One fuzz target per reader of a file a project can ship (decision 0042). Each takes the fuzzer's bytes, hands them to the engine's own reader, and fails only on a crash, a hang or a broken promise (such as a file that doesn't read back the same after writing). Each is listed as a `[[bin]]` in [`../Cargo.toml`](../Cargo.toml).

| File | What it reads |
| --- | --- |
| `area_file.rs` | An area or assembly file: refused or read, its assemblies expanded, written and read back. |
| `aseprite.rs` | An Aseprite file (`.aseprite`): refused or read within the limits, every frame the canvas's size, tags within the frames, then packed into a sheet whose frames draw back the same. |
| `atlas.rs` | An atlas file (`.atlas.lst`): refused or read, within the limits, each crop's name good and used once, and written back to itself. |
| `animator.rs` | An animator file (`.solanim`): refused or read, within the limits, written back to itself, and run with every parameter set every way. |
| `game_code.rs` | One Lust file and one Lua file (split by a 0x01 byte): checked, then loaded and run for a tick within the time limits. |
| `import_cache.rs` | An import cache entry (`.solis/cache/*.solbin`): refused, or read with every index pointing somewhere. |
| `text_file.rs` | A text file shown in Properties (`.md`, `.txt` and the like): read as the app reads it, then made into Markdown blocks and inline runs, never a panic. |
| `input_actions.rs` | A project's input actions (`input.solinput`): refused or read, then written and read back the same. |
| `mixer.rs` | The project's buses (`mixer.solmix`): refused or read, always a sound tree within the limits (Master first), written back to themselves, with levels and ducking that are numbers. |
| `model.rs` | A model file (glb or glTF): refused with why, or imported within the limits, its numbers finite. |
| `pixels.rs` | The project's 2D settings (`pixels.solpix`): refused or read, always within the limits, and written back to themselves. |
| `frames.rs` | A sprite's clips file (`.solframes`): refused or read, within the limits, written back to itself, and its clips played without panicking. |
| `project_file.rs` | A project file: refused or read. |
| `recording.rs` | A recording of play: refused or read, then written and read back the same. |
| `save_game.rs` | A save game loaded into a fresh editor: refused with why, or loaded and played on a few ticks. |
| `screen_source.rs` | A screen's Lust file as the UI editor reads it: refused or read, every piece's place inside the text, and the editor's changes (a style, words, a piece taken out or moved) never a panic. |
| `sidecar.rs` | A sidecar (`*.solmeta`): refused or read, then written and read back. |
| `sound_event.rs` | A sound event file (`.solsound`): refused or read, always within its limits, written back to itself, and every pick one of its takes. |
| `sound_file.rs` | A WAV or Ogg Vorbis file: refused or read, always within the limits, every sample between -1 and 1. |
| `tileset.rs` | A tileset (`.tileset.lst`, or an older `.soltileset`): refused or read, within the limits, each terrain's name and code used once, and written back to itself. |
| `tile_art.rs` | A tileset's picture (the first byte chooses PNG or JPEG, or Aseprite): refused or read whole within the limits, every frame its size, then its grid found and its tiles weighed. |
| `tile_map.rs` | A tile map file (`.tilemap.lst`, or an older `.soltilemap`): refused or read, within the limits, written back to itself (or refused by the writer, when it would be more than is read back), then changed cell by cell (inside and past its edges), resized and summed. |
| `playtest_file.rs` | A playtest file (`playtest.txt` beside an exported game, or a project's `.solis/last export.txt`): refused or read, every part put right (a usable name and area, a window Solis opens), and written back to itself. |
| `update_answer.rs` | GitHub's answer about the latest release and the installer's checksum file (split by a 0x01 byte): a release read only ever points at the repository's own downloads. |

Each target starts with `fuzz_support::quiet_caught_panics()` (in [`../src`](../src/README.md)), so a library's panic that Solis catches isn't printed but the target's own `assert!` messages are.

**Adding a target:** add the file here (calling `fuzz_support::quiet_caught_panics()` first), a `[[bin]]` of the same name in `../Cargo.toml`, a row in the table above and in [`../README.md`](../README.md), and starting files in `../make-seeds.sh` with a row in [`../seeds/README.md`](../seeds/README.md). The weekly run ([`deep-checks.yml`](../../.github/workflows/deep-checks.yml)) reads its targets from `../Cargo.toml`, so it fuzzes the new one with no change to the workflow, and stops with the reason if a file here has no `[[bin]]`.
