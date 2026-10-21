# editor

**Layer 2 · Editor**

The editor's interface, as decided in [decision 0004](../../docs/decisions/0004-editor-layout.md): the one-row top bar (menus, document tabs, Play and Build), the dockable panels (Viewport and Game, Scene and Layers, Properties, Library and Code, Console and Problems), the controls in the viewport's corners, the icons (drawn in code, in `icons.rs`), and the theme. Interface text is Inter and monospace text JetBrains Mono (both OFL, in `assets/fonts`, licences in `licenses/fonts`).

## Purpose

Draw the editor with egui and egui_dock, keep its state between frames (the panel layout, the chosen tool, the accent colour, the camera, the console), and tell the app what to do through `WorkbenchAction`s.

`camera.rs` holds the viewport camera and how it flies ([decision 0009](../../docs/decisions/0009-fly-camera-controls.md)). The viewport turns each frame's mouse and keys into a `FlyInput`, and `Camera::fly` does the maths, so tests can fly the camera without a window.

## Must never

- Be needed by a game: games ship without the editor
- Touch the window, the GPU or files directly: the app does that, through actions and the `ViewportRenderer` it hands in
- Hard-code the program's name: it comes from `AppInfo`

## Depends on

egui, egui_dock, glam and serde. No Solis crates yet; the engine's crates join as there is something to edit.

## Status

Phase 1, milestone 1: the shell and a flying camera. The panels other than the viewport and the console say what arrives in which phase.
