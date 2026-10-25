# app

**Layer 3 · App**

The `solis` program: reads the command line, opens the editor window with eframe (winit and wgpu), restores the settings saved last time (panel layout, accent colour, viewport controls), and hands each frame to the editor's workbench.

## Purpose

Wire the engine and the editor together. It owns the window and the GPU device, connects the `render` crate's viewport images to egui (`gpu.rs`, one image per view: the viewport's and the Game panel's), reads model files on background threads with `assets` (`import.rs`), and carries out the editor's actions (quitting, importing, saving and opening areas, the Library's file changes, opening files in their own programs with `opener`). `thumbnails.rs` makes the Library's thumbnails: pictures shrunk on a worker, models drawn once on the GPU. `logger.rs` passes what the libraries write through the `log` facade to the Console: every library's warnings and errors, information only from Solis's own crates. The console's earlier commands are saved with the other settings.

## Must never

- Hold feature logic: that belongs in the engine or the editor
- Hard-code the program's name outside `branding.rs`

## Depends on

`editor`, `render`, `assets`, `asset_db`, `input`, `updater`, eframe, rfd (the system's Open window), opener, log, and on Windows gilrs (gamepads).

## Status

Phase 1: the editor with a drawn viewport, a flying camera, and imported models. Phase 2: projects, area files and assemblies. Phase 3: picking, the Library's thumbnails and file actions, and library messages in the Console. Phase 5: the benchmark (`benchmark.rs`, Help > Make the benchmark…): a project of two fleets of ships (the ship model from `assets::sample::ship_glb`, its sidecar with a fixed ID, an assembly, and `samples/benchmark/assets/main.lst`) in three areas; every assembly in a project is read when the Library is listed, so game code can spawn any; the Game panel's projectiles are handed to the renderer (`render_with_bolts`). Phase 6: the project's input actions read from and saved to `input.solinput` and recordings saved in `recordings/` and opened for replay, and save games written to and read from `saves/` (`play_files.rs`); the first gamepad read each frame with gilrs on Windows only (`gamepad.rs`; elsewhere none, so the Linux checks need no system libraries); the playground (`playground.rs`, Help > Make the playground…): a level made in code (steps, ramps, crates, a moving platform, a character with the ready-made controller and third-person camera), its models from `assets::sample::playground_glb` with fixed IDs, `samples/playground/assets/main.lst`, and its input actions with QuickSave (F6) and QuickLoad (F7). Updates (decision 0043): `updates.rs` starts a check with `updater` when the window opens and every six hours (unless Help > Update automatically is off; saved with the other settings), tells the editor how it's going (`Workbench::set_update`) and the Console when there's news, and keeps a downloaded installer in a slot that `window::run` starts once the window has closed (relaunching after Restart to update). `version()` adds the build number to the version.
