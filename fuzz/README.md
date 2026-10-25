# Fuzzing

Fuzz targets for every reader of a file a project can ship ([decision 0042](../docs/decisions/0042-a-hardening-milestone-before-phase-7.md)). A fuzzer feeds a reader millions of changed copies of real files, looking for a crash, a hang or a broken promise (a file written by Solis must read back). Anything a project ships can be anyone's, so none of these may ever crash Solis.

| Target | What it reads |
|---|---|
| `save_game` | A save game, loaded into an editor and played on a few ticks |
| `recording` | A recording of play; read back after writing |
| `input_actions` | `input.solinput`; read back the same after writing |
| `area_file` | An area or assembly file, its assemblies expanded (it may place itself) |
| `model` | A glb model, imported within limits; every number finite |
| `import_cache` | An import cache entry (`.solis/cache/*.solbin`) |
| `sidecar` | A `.solmeta` sidecar |
| `project_file` | `project.solis` |
| `game_code` | A Lust file and a Lua file (split by a 0x01 byte), checked, loaded and run for a tick |
| `update_answer` | GitHub's answer about the latest release and the installer's checksum file (split by a 0x01 byte); a release read only ever points at the repository's own downloads |

## Running

You need [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) (`cargo install cargo-fuzz`) and the nightly compiler. From the repository's top folder:

```
fuzz/make-seeds.sh              # real files to start from, into fuzz/seeds/
fuzz/run-all.sh                 # every target for 10 minutes, two at a time
SECONDS_EACH=3600 fuzz/run-all.sh
cd fuzz && cargo +nightly fuzz run save_game corpus/save_game seeds/save_game
```

On a machine that can't download nightly, `RUSTC_BOOTSTRAP=1` lets the pinned stable compiler build the targets (only ever for fuzzing).

What each run finds is kept in `fuzz/corpus/<target>` (not committed). A crash is saved in `fuzz/artifacts/<target>/`; replay it with `cargo fuzz run <target> <file>`, then fix it and add a regression test in the crate that reads the file.

Each target only counts a panic that escapes Solis: libraries' panics that Solis catches on purpose, and turns into a refused file, aren't failures. For example, the `gltf` crate (1.4.1) panics on a mesh whose POSITION names an accessor that isn't there.

While fuzzing, Solis is built with `--cfg fuzzing`. The only difference is that a save game's checksum isn't checked, because a fuzzer can't make a matching one, so the rest of the reader gets tried.
