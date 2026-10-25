# app

**Layer 3 · App**

The `solis` program: reads the command line, opens the editor window with eframe (winit and wgpu), restores the settings saved last time (panel layout, accent colour, viewport controls), and hands each frame to the editor's workbench.

## Purpose

Wire the engine and the editor together. It owns the window and the GPU device, connects the `render` crate's viewport images to egui (`gpu.rs`, one image per view: the viewport's and the Game panel's), reads model files on background threads with `assets` (`import.rs`), and carries out the editor's actions (quitting, importing, saving and opening areas, the Library's file changes, opening files in their own programs with `opener`). `thumbnails.rs` makes the Library's thumbnails: pictures shrunk on a worker, models drawn once on the GPU. `logger.rs` passes what the libraries write through the `log` facade to the Console: every library's warnings and errors, information only from Solis's own crates. The console's earlier commands are saved with the other settings.

## Must never

- Hold feature logic: that belongs in the engine or the editor
- Hard-code the program's name outside `branding.rs`

## Depends on

`editor`, `render`, `assets`, `asset_db`, eframe, rfd (the system's Open window), opener and log.

## Status

Phase 1: the editor with a drawn viewport, a flying camera, and imported models. Phase 2: projects, area files and assemblies. Phase 3: picking, the Library's thumbnails and file actions, and library messages in the Console.
