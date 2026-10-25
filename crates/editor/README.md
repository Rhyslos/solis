# editor

**Layer 2 · Editor**

The editor's interface, as decided in [decision 0004](../../docs/decisions/0004-editor-layout.md): the one-row top bar (menus, document tabs, Play and Build), the dockable panels (Viewport and Game, Area and Layers, Properties, Library and Code, Console and Problems), the controls in the viewport's corners, the icons (drawn in code, in `icons.rs`), and the theme. Interface text is Inter and monospace text JetBrains Mono (both OFL, in `assets/fonts`, licences in `licenses/fonts`).

## Purpose

Draw the editor with egui and egui_dock, keep its state between frames (the panel layout, the chosen tool, the accent colour, the camera, the console), and tell the app what to do through `WorkbenchAction`s.

**Where things are:**

- `workbench.rs`: the editor's state, and `show`, which draws a frame and returns the actions.
- `top_bar.rs`: the menus, document tabs, Play and Build.
- `panels.rs`: draws each docked panel and the bits panels share.
- `area_panel.rs`, `properties.rs`, `console.rs`: those panels' contents.
- `viewport.rs`: the viewport and its corner controls.
- `layout.rs`: the presets, and restoring a saved layout.

Tests of the whole editor (drawn with egui_kittest, without a GPU) are in `workbench_tests.rs`.

`look.rs` holds the viewport's look (standard, stylized or unlit). `document.rs` holds the open area (a `area::Area`), its component registry, the selection and the undo history; the area can only be changed through `Document::edit`. `Workbench::register_component` is the public way to add a component schema (game code in Phase 4, plugins); Properties' Add component menu lists every registered one. The Area panel draws only the rows in view. Assemblies the area uses are held as their files (`Workbench::set_assembly`); copies are expanded and collapsed with `area::assembly`, and Properties shows each copy's changes in bold with Revert and Apply. The Area panel (`area_panel.rs`) keeps its own view state (`AreaPanelState`: search, folded, hidden and locked entities), never saved and never an undo step; `menu_items.rs` holds the Edit and Entity menus' items, shared by the top bar and the Area panel. `commands_many.rs` holds Properties' commands for several entities at once and copying and pasting components. `library.rs` draws the Library and Code panels (folder tree, thumbnails or a list, search, drag and drop) and returns `LibraryRequest`s; `workbench_library.rs` turns them into actions for the app and places dropped assets where they were dropped. `picking.rs` decides what a click or a box in the viewport selects; `gizmo.rs` (adapted from Aventail) is the move, rotate and scale gizmos' maths; `transforming.rs` turns a drag into new transforms, with snapping; `screen.rs` holds the rays and projections between the screen and the world; `viewport_tools.rs` handles the mouse in the viewport (clicks, boxes, gizmo drags), draws the gizmo and the selection's outline, and holds the snapping corner; `workbench_tools.rs` is the workbench's side of it ([decision 0025](../../docs/decisions/0025-picking-and-gizmos.md)). `commands.rs` holds everything the editor does to the area (import, add, delete, duplicate, reparent keeping the world position, move here, frame, Properties and Area panel changes), each turned into edits as one undo step. `inspector.rs` draws any component's fields from its schema; `properties.rs` uses it and returns `PropertyRequest`s (and the Area panel `AreaRequest`s) that the workbench applies, so panels never change the area or the selection themselves. `area_view.rs` works out what the viewport draws from the area each frame (models, the sun, lamps placed in the world, the environment). `models.rs` holds what the editor knows about imported models, by asset ID (the app holds them on the GPU), and the lights a model's file carries. `console_commands.rs` reads and runs the commands typed in the Console's command line (`console.rs` draws it, with its history and Tab), through the same editor commands as the menus, so each is an undo step. `problems.rs` finds the area's mistakes for the Problems panel (missing models and assemblies, unknown components, missing parents), again only when something they depend on changes. `play.rs` runs the area as a game: Play keeps a copy of the document and builds a `world::World`, Stop puts the copy back, and saving and opening wait meanwhile; `game_panel.rs` draws the Game panel through the first camera. `camera.rs` holds the viewport camera and how it flies ([decision 0009](../../docs/decisions/0009-fly-camera-controls.md)). The viewport turns each frame's mouse and keys into a `FlyInput`, and `Camera::fly` does the maths, so tests can fly the camera without a window.

## Must never

- Be needed by a game: games ship without the editor
- Touch the window, the GPU or files directly: the app does that, through actions and the `ViewportRenderer` it hands in
- Hard-code the program's name: it comes from `AppInfo`

## Depends on

`reflect` and `area` (the document it edits), `world` (the running game), egui, egui_dock, glam, serde and chrono. Never `render` or `asset_db`: the app connects those.

## Status

Phase 1 complete: the shell, a flying camera, imported models, looks, lights, sky and fog. Phase 2: entities, undo, projects, area files and assemblies. Phase 3: picking and gizmos in the viewport, the Area panel and Properties for several entities, the Library, console commands and Problems, play mode and the Game panel. The panels not built yet say what arrives in which phase.
