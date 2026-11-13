# samples

The game code and files of the ready-made projects in the Help menu. Each sample is laid out as a game project: its files live in an `assets/` folder, as in a real project. The program builds these files in (`crates/app/src`), and writes them into a new project when the user picks the Help menu item. The `assets/` folders have no README of their own, because the Library would list it as one of the project's files.

| Folder | Help menu item | What's in its `assets/` |
| --- | --- | --- |
| `turret/` | Make the sample game | The first game, in all three languages: `main.lst` (the turret and the drone's health, in Lust), `armour.rs` (the drone's armour, in Rust, built into the program until the full export, Shared 11 in ROADMAP.md, builds a game's own Rust; decision 0029), `sparks.lua` (sparks, in Lua), `ui/droneHealth.lst` (the drone's health bar, a screen reading its Health; decision 0087) and `turret.solarea` (the area, whose Drone has a Screen showing it). Made by `crates/app/src/sample_game.rs`. |
| `ui/` | Make the UI sample | A game's screens (game UI M7): `main.lst` (a turret the player fires, the drone's health, and the Pauser, which changes the player's settings when the menu asks), `ui/hud.lst`, `ui/droneHealth.lst`, `ui/pauseMenu.lst` (Resume, Text size, Fullscreen, Resolution, and Quit, which calls `Window:quit()`), and `ui.solarea`. Made by `crates/app/src/ui_sample.rs`. |
| `benchmark/` | Make the benchmark | `main.lst`: two fleets of ships flying loops and firing lasers, at three sizes (decision 0030). Made by `crates/app/src/benchmark.rs`, which also builds its areas. |
| `playground/` | Make the playground | `main.lst`: quick saves, the moving platform and footsteps for a level to walk around in, using the ready-made character and camera. Made by `crates/app/src/playground.rs`, which also builds the level. |

These files are also read by tests (`crates/editor/src/scale_levels_tests.rs` and the app's tests) and copied into fuzz seeds by `fuzz/make-seeds.sh`. `turret.solarea` is written by an ignored test in `crates/app/src/sample_game_tests.rs`; run it again if the area format changes.
