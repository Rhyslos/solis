# tests

Test data for the whole program, read by the crates' tests.

## fixtures/

Example files of every version of every file format Solis has ever released (Phase 8, C1; [decision 0053](../docs/decisions/0053-compatibility-before-2d.md)). A test in the app (`crates/app/src/fixtures_tests.rs`) opens every one with the current build, which upgrades it in memory, so a build that can't open an older file can't be merged. They're never changed: when a format gets a new version, its new folder is added beside the old ones.

| Folder | Format | Versions |
| --- | --- | --- |
| `area/` | Areas (`.solarea`) and assemblies (`.solassembly`) | v1 |
| `sound-event/` | Sound events (`.solsound`) | v1 |
| `mixer/` | The project's buses (`mixer.solmix`) | v1 |
| `input-actions/` | Input actions (`input.solinput`) | v1 (no version line; the arrow keys written `ArrowUp`), v2 |
| `recording/` | Recordings of a play (`.solrec`) | v1 |
| `project/` | The project file (`project.solis`) | v1 |
| `sidecar/` | Sidecars beside assets (`.solmeta`) | v1 |
| `save/` | Save games (`.solsave`), opened by `crates/editor/src/save_game_tests.rs`, which loads each and plays on | v1 (physics as Rapier's bytes, never read now), v2 (the game's places in the area; one with the first builds' checksum of the body only), v3 |

Each `v<N>/` folder holds files as that version wrote them. `fixtures/` and its folders are data, so they have no README of their own.
