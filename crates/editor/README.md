# editor

**Layer 2 · Editor**

The editor's interface, as decided in [decision 0004](../../docs/decisions/0004-editor-layout.md): the one-row top bar (menus, document tabs, Play and Build), the dockable panels (Viewport and Game, Scene and Layers, Properties, Library and Code, Console and Problems), the controls in the viewport's corners, the icons (drawn in code, in `icons.rs`), and the theme. Interface text is Inter and monospace text JetBrains Mono (both OFL, in `assets/fonts`, licences in `licenses/fonts`).

## Purpose

Draw the editor with egui and egui_dock, keep its state between frames (the panel layout, the chosen tool, the accent colour, the camera, the console), and tell the app what to do through `WorkbenchAction`s.

**Where things are:**

- `workbench.rs`: the editor's state, and `show`, which draws a frame and returns the actions.
- `top_bar.rs`: the menus, document tabs, Play and Build.
- `panels.rs`: draws each docked panel and the bits panels share.
- `scene_panel.rs`, `properties.rs`, `console.rs`: those panels' contents.
- `viewport.rs`: the viewport and its corner controls.
- `layout.rs`: the presets, and restoring a saved layout.

Tests of the whole editor (drawn with egui_kittest, without a GPU) are in `workbench_tests.rs`.

`look.rs` holds the viewport's look (standard, stylized or unlit). `document.rs` holds the open scene (a `scene::Scene`), its component registry and the selection. `inspector.rs` draws any component's fields from its schema; `properties.rs` uses it and returns `PropertyRequest`s that the workbench applies, so panels never change the scene themselves. `scene_view.rs` works out what the viewport draws from the scene each frame (models, the sun, lamps placed in the world, the environment). `models.rs` holds what the editor knows about imported models, by asset ID (the app holds them on the GPU), and the lights a model's file carries. `camera.rs` holds the viewport camera and how it flies ([decision 0009](../../docs/decisions/0009-fly-camera-controls.md)). The viewport turns each frame's mouse and keys into a `FlyInput`, and `Camera::fly` does the maths, so tests can fly the camera without a window.

## Must never

- Be needed by a game: games ship without the editor
- Touch the window, the GPU or files directly: the app does that, through actions and the `ViewportRenderer` it hands in
- Hard-code the program's name: it comes from `AppInfo`

## Depends on

egui, egui_dock, glam and serde. No Solis crates yet; the engine's crates join as there is something to edit.

## Status

Phase 1 complete: the shell, a flying camera, imported models, looks, lights, sky and fog. The panels other than the viewport and the console say what arrives in which phase.
