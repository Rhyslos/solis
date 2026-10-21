# app

**Layer 3 · App**

The `solis` program: reads the command line, opens the editor window with eframe (winit and wgpu), restores the settings saved last time (panel layout, accent colour, viewport controls), and hands each frame to the editor's workbench.

## Purpose

Wire the engine and the editor together. It owns the window and the GPU device, connects the `render` crate's viewport images to egui (`gpu.rs`), and carries out the editor's actions (such as Quit).

## Must never

- Hold feature logic: that belongs in the engine or the editor
- Hard-code the program's name outside `branding.rs`

## Depends on

`editor`, `render`, eframe.

## Status

Phase 0: opens the editor shell with a drawn viewport.
