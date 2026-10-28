# fuzz/fuzz_targets

One fuzz target per reader of a file a project can ship (decision 0042). Each takes the fuzzer's bytes, hands them to the engine's own reader, and fails only on a crash, a hang or a broken promise (such as a file that doesn't read back the same after writing). Each is listed as a `[[bin]]` in [`../Cargo.toml`](../Cargo.toml).

| File | What it reads |
| --- | --- |
| `area_file.rs` | An area or assembly file: refused or read, its assemblies expanded, written and read back. |
| `game_code.rs` | One Lust file and one Lua file (split by a 0x01 byte): checked, then loaded and run for a tick within the time limits. |
| `import_cache.rs` | An import cache entry (`.solis/cache/*.solbin`): refused, or read with every index pointing somewhere. |
| `input_actions.rs` | A project's input actions (`input.solinput`): refused or read, then written and read back the same. |
| `model.rs` | A model file (glb or glTF): refused with why, or imported within the limits, its numbers finite. |
| `project_file.rs` | A project file: refused or read. |
| `recording.rs` | A recording of play: refused or read, then written and read back the same. |
| `save_game.rs` | A save game loaded into a fresh editor: refused with why, or loaded and played on a few ticks. |
| `sidecar.rs` | A sidecar (`*.solmeta`): refused or read, then written and read back. |
| `sound_file.rs` | A WAV or Ogg Vorbis file: refused or read, always within the limits, every sample between -1 and 1. |
| `update_answer.rs` | GitHub's answer about the latest release and the installer's checksum file (split by a 0x01 byte): a release read only ever points at the repository's own downloads. |

**Adding a target:** add the file here, a `[[bin]]` in `../Cargo.toml`, a row in [`../README.md`](../README.md), and starting files in `../make-seeds.sh`.
