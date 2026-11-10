# Fuzzing

Fuzz targets for every reader of a file a project can ship ([decision 0042](../docs/decisions/0042-a-hardening-milestone-before-phase-7.md)). A fuzzer feeds a reader millions of changed copies of real files, looking for a crash, a hang or a broken promise (a file written by Solis must read back). Anything a project ships can be anyone's, so none of these may ever crash Solis.

| Target | What it reads |
|---|---|
| `save_game` | A save game, loaded into an editor and played on a few ticks |
| `recording` | A recording of play; read back after writing |
| `input_actions` | `input.solinput`; read back the same after writing |
| `area_file` | An area or assembly file, its assemblies expanded (it may place itself) |
| `aseprite` | An Aseprite file, read within the limits and packed into a sprite sheet whose frames draw back the same |
| `model` | A glb model, imported within limits; every number finite |
| `import_cache` | An import cache entry (`.solis/cache/*.solbin`) |
| `sidecar` | A `.solmeta` sidecar |
| `project_file` | `project.solis` |
| `game_code` | A Lust file and a Lua file (split by a 0x01 byte), checked, loaded and run for a tick |
| `update_answer` | GitHub's answer about the latest release and the installer's checksum file (split by a 0x01 byte); a release read only ever points at the repository's own downloads |
| `sound_event` | A sound event file (`.solsound`); an event read is within its limits, writes back to itself, and every pick is one of its takes |
| `mixer` | The project's buses (`mixer.solmix`); buses read make a sound tree within the limits, write back to themselves, and give levels and ducking that are numbers |
| `pixels` | The project's 2D settings (`pixels.solpix`); settings read are within the limits and write back to themselves |
| `frames` | A sprite's clips file (`.solframes`); clips read are within the limits, write back to themselves, and play without panicking |
| `animator` | An animator file (`.solanim`); an animator read is within the limits, writes back to itself, and runs without panicking whatever game code sets |
| `atlas` | An atlas file (`.atlas.lst`, Atlas Crop); crops read are within the limits, each name good and used once, and write back to themselves |
| `text_file` | A text file shown in Properties: any bytes become Markdown blocks and inline runs, never a panic |
| `screen_source` | A screen's Lust file as the UI editor reads it; pieces' places inside the text, and its edits never panic |
| `tileset` | A tileset (`.tileset.lst`, or an older `.soltileset`); terrains read are within the limits, each name and code used once, and write back to themselves |
| `tile_art` | A tileset's picture (PNG, JPEG or Aseprite, the first byte choosing); a picture read is within the limits, every frame its size, and its grid and kind are found without panicking |
| `tile_map` | A tile map file (`.tilemap.lst`, or an older `.soltilemap`); a map read is within the limits, writes back to itself, is autotiled, painted (a fill, a stamp flipped and turned, exact tiles) and changed cell by cell, resized and summed without panicking |
| `sound_file` | A WAV or Ogg Vorbis sound file (`sound::read_bytes`); a sound read is within the limits, as long as its overview, and every sample a number between -1 and 1 |

## Running

Every Sunday the weekly deep checks ([`deep-checks.yml`](../.github/workflows/deep-checks.yml)) fuzz every target for 20 minutes, each in a job of its own. They read the targets from the `[[bin]]` entries in [`Cargo.toml`](Cargo.toml), so a new target needs no change to the workflow. A crash or hang there is kept with the run and opens an issue.

You need [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) (`cargo install cargo-fuzz@<version> --locked`) and a nightly compiler. The weekly run pins both, as `CARGO_FUZZ_VERSION` and `FUZZ_NIGHTLY` at the top of [`deep-checks.yml`](../.github/workflows/deep-checks.yml): use those two to get exactly what it gets. From the repository's top folder:

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

## The lock file

`fuzz/` is a workspace of its own, so the engine's build never builds the fuzzers. That gives it its own `Cargo.lock`, and nothing in Cargo keeps that file in step with the main one. Fuzzing is only worth its time if it tries the library versions the program ships, so a check holds the two together.

[`tools/ci/fuzz-lock-check.sh`](../tools/ci/fuzz-lock-check.sh) fails, naming each package, when `fuzz/Cargo.lock` has a version of a package that the main `Cargo.lock` has at other versions only. With `--locked` it also asks Cargo whether a build of the fuzzers would change the file: `cargo fuzz` can't be told `--locked` itself, and would quietly add whatever the file lacks at the newest version. It runs with `--locked` on every pull request ([`ci.yml`](../.github/workflows/ci.yml)) and again before the weekly fuzzing, which doesn't start if it fails.

It fails after any change to the main `Cargo.lock` that touches a library the fuzzers build, Dependabot's weekly pull request included. To fix it, from the repository's top folder:

```
tools/ci/fuzz-lock-check.sh --fix
```

Then commit `fuzz/Cargo.lock`. The fix copies the main lock over the fuzzers' and has Cargo take out what they don't build and add what only they use: `libfuzzer-sys` and `arbitrary`, at their newest versions. That is how those two are kept up to date, so Dependabot isn't pointed at this folder. Never update this lock file by itself (`cargo update` in `fuzz/`): the check would fail.

The weekly security check ([`security.yml`](../.github/workflows/security.yml)) reads this lock file as well as the main one, so a newly found problem in a library only the fuzzers use shows up too.
