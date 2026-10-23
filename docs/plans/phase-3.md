# Phase 3 plan: the editor

The working plan for Phase 3 (ROADMAP.md, "Phase 3: The editor"). It records the choices made before any code, the controls the project lead chose, and a plan for every milestone, so a later session can carry on where this one stopped. Update it as milestones are built: tick them off here and in the README tracker, and change a milestone's plan if its design changes.

**Gate (unchanged):** build a small area from scratch in the editor, press Play, stop, and everything is back as it was.

## How the plan was made

- **The game's fast storage first,** as decision 0019 asks: `hecs` against a storage of Solis's own, timed at the scale target (`tools/storage-bench`). Both are far inside the budget; `hecs` was chosen because it means less code to look after ([decision 0024](../decisions/0024-game-storage-is-hecs.md)).
- **Aventail's code is reused** where it fits (decision 0007): its gizmo maths (`tools::gizmo`, with its tests) and its rules for moving several objects, snapping and cancelling ([Aventail decision 0030](https://github.com/Rhyslos/aventail-3d/blob/main/docs/decisions/0030-transform-gizmos.md)). Its picking reads depth only; Solis needs to know *which entity* is under the mouse, so it gets an ID image instead.
- **What already exists is extended, not replaced:** the Area panel, Properties, the Library, the console and the viewport's corner controls were built in Phases 0–2; each milestone fills in what they still lack.

## Controls (chosen 2026-10-23, CONTRIBUTING rule 17)

| Action | Control |
|---|---|
| Select in the viewport | Click |
| Add to or take from the selection | Ctrl+click |
| Select with a box | Drag on empty space (Ctrl+drag adds) |
| Select nothing | Esc |
| Select everything | Ctrl+A |
| Gizmo snapping | The buttons in the viewport's bottom-right corner; holding Ctrl flips snapping for that drag |
| Snap steps | Moves: the grid step (1 m, 50 cm, 25 cm or 10 cm); turns: 15°; scaling: 10% |
| Ten times finer | Hold Shift while dragging |
| Cancel a drag | Esc, or a right click |
| Lock an axis (drags leave it alone; press again to free it) | X, Y or Z, or the buttons in the corner |
| World or object axes | L |
| Play, Pause, Stop | The buttons at the top only (no keys) |
| Jump to the console's command line | The `'` key: on a Norwegian keyboard, the key with ' and * next to Æ |
| Earlier commands | Up and Down in the command line |
| Complete a command or name | Tab |
| Flat views (Top, Front, Right) | Right or middle drag slides the view, the wheel zooms (Unreal's; recommended, as the project lead didn't choose) |

**No clashes:**
- The tool keys (Q W R S T), F, F2, Delete, X, Y, Z, L and the fly keys (W A S D Q E while the right mouse button is held) take no modifiers, so Ctrl+Z (undo) and Ctrl+A stay apart from Z and A.
- Esc cancels a drag first, and only clears the selection when nothing is being dragged.
- `'` only jumps to the command line when no text field has the keyboard; inside one it types a `'`.
- None of these fire while typing in a field.

## The shape of the code

| Crate | Layer | What it gains | Milestone |
|---|---|---|---|
| `area` | 0 | The `Camera` component | M5 |
| `world` (new) | 1 | The game's fast storage on `hecs`: built from an area, stepped, read by entity ID | M5 |
| `render` | 1 | An ID image for picking, read back without waiting; thumbnails; flat (orthographic) views and a grid that faces them | M1, M3, M6 |
| `asset_db` | 1 | Every file in the project listed (areas, textures, code), renaming and moving inside Solis with IDs kept | M3 |
| `editor` | 2 | Picking, box selection, gizmos and snapping; Area panel search, hiding and locking; Properties for several entities; the Library's folders and thumbnails; console commands; Problems; play mode; several viewports | M1–M6 |
| `app` | 3 | Picking and thumbnails on the GPU, a logger feeding the console, the game's world while playing, one image per viewport | M1–M6 |

**What changes least:**
- `ViewportRenderer` (the editor's only way to draw) gains picking in M1 and an image per view in M5; tests keep drawing nothing.
- Every change to the area still goes through `Document::edit`, so gizmo drags, console commands and Library drops are all undoable for free.

## Milestones

### M1. Picking and gizmos ☑

- **Picking:**
  - `render` draws each placement's number into an ID image (one 32-bit number per pixel, with depth) alongside the frame, only when asked. A click or box asks for a rectangle of it; the answer is read back on a later frame (usually the next), so the frame never waits (rule 10).
  - Lamps, the sun and (from M5) cameras have no surface: they're picked by their markers on screen, in the editor.
  - **Assemblies:** a click selects the outermost copy the entity belongs to; a second click on something inside an already-selected copy selects the entity itself (as Unity does with prefabs).
  - A box selects everything with a visible pixel inside it, plus markers inside it.
- **Gizmos** (adapted from Aventail):
  - Move: an arrow per axis, a square per plane, the centre (facing you). Rotate: a ring per axis and an outer ring around the line of sight. Scale: a box per axis, a square per plane, the centre. Transform: all three.
  - Drawn over the image a fixed size on screen.
  - The gizmo sits at the selected entity, or the middle of several. Entities inside another selected one move with it, not twice.
  - **One drag is one undo step:** each frame sets every moved entity's Transform from where it was when the drag began, with the gesture's merge key, so the steps merge (decision 0022). Cancelling (Esc, right click) takes the step back.
  - Values are worked out in the parent's space; scaling a turned entity along the world's axes keeps the nearest shape without skew (as Aventail does).
- **Snapping:** the bottom-right corner's grid, angle and scale buttons become real (on and off, and the step), saved per computer.
- **Tests:** the gizmo maths (Aventail's tests, adapted); picking against a GPU image; selection rules with a stand-in renderer; a drag is one undo step and cancel leaves none; snapping steps; Esc and Ctrl+A.
- **Manual:** a new "Selecting and moving things" page; keyboard shortcuts; concepts (gizmo, snapping, pivot).
- **Built** ([decision 0025](../decisions/0025-picking-and-gizmos.md)): as planned. The axis locks are switches, as in Aventail, shown as X, Y and Z buttons in the corner (red while locked), rather than only working during a drag. The corner is two rows so it stays clear of the tools. Snapping to other objects' surfaces (Aventail has it) waits; moves snap to the grid.

### M2. The entity list and the inspector ☐

- **Area panel:**
  - The search box filters by name (matches shown with their parents).
  - Its menu row becomes real menus: Edit (undo, redo, duplicate, delete, rename, select all, select none), View (expand all, collapse all) and Entity (the same as the top bar's Entity menu).
  - **Hide** (an eye) and **lock** (a padlock) per entity, shown on hover. A hidden entity isn't drawn in the editor; a locked one can't be picked in the viewport. Both are the editor's view only: they're not saved and not undo steps, as with the selection (decision 0022).
- **Properties:**
  - **Several entities:** the components they all have, with fields that differ shown as "—". Changing a field changes it on all of them, as one undo step.
  - Its menu row becomes real: Edit (copy a component's values, paste them, reset to defaults) and Units (centimetres or metres; the rotation in degrees always). Mode is dropped; there is only one so far.
- **Layers:** the Layers panel's note is corrected. Layers aren't in any phase of the roadmap yet; the project lead is asked when they should come.
- **Tests:** filter, hide and lock with picking; multi-edit is one step and undoes on every entity; copy and paste values.

### M3. The library: assets and code ☐

- **`asset_db`** lists every file under `assets/`: models and assemblies (with IDs, as now), plus areas, textures and code (`.lst`, `.lua`, `.rs`), which are listed but don't need IDs yet.
- **The Library panel:**
  - A folder tree on the left that can be collapsed to make room (the "collapsible panel"), the items on the right, and a search box.
  - Items as a grid of thumbnails with a coloured stripe per kind (decision 0004), or a list (View menu).
  - Thumbnails are drawn on the GPU by the app, a few per frame, and kept in `.solis/cache/thumbnails`; textures use the picture itself.
- **Placing by dragging** (decision 0004): drag a model or assembly onto the viewport to place it where the mouse points (on the surface under it, or on the ground), or onto an entity in the Area panel to place it inside that entity. Double-click places it where the camera looks (as now), opens an area, or opens code in the system's editor.
- **Right-click:** Place, Open, Rename (F2), Show in Explorer, Delete (to the Recycle Bin), New folder. Renaming and moving (dragging onto a folder) keep the asset's ID: the sidecar moves with it.
- **Code** (the Library's second tab, the Code panel): the project's code files, the same way. Writing code in Solis comes in Phase 10; until then double-click opens the file in the system's editor.
- **Tests:** listing kinds, rename and move keep IDs, drop placement with a stand-in pick, search, thumbnails on the GPU.

### M4. The console: commands as well as messages ☐

- **A command line** under the messages. `'` jumps to it, Up and Down go through earlier commands (kept between sessions), Tab completes names, Enter runs.
- **Commands** use the same editor commands as the menus, so they're undoable: `help`, `clear`, `select`, `find`, `frame`, `delete`, `duplicate`, `new` (an entity), `get` and `set` (any component field), `undo`, `redo`, `save`, `look`, `tool`, `stats`. Play mode adds `play`, `pause` and `stop` in M5.
- **Messages from the engine:** the app passes warnings and errors from Solis's crates and its libraries (wgpu, for example) to the console through the `log` facade.
- **Problems:** a list of what's wrong in the open area, worked out when it changes: models or assemblies missing from the project, components nobody registered, overrides whose target is gone. Clicking one selects its entity.
- **Tests:** every command, with good and bad arguments (bad ones give a plain-language error, never a crash); history and completion; a fuzz test of the command reader; Problems for each kind.

### M5. Play mode ☐

- **The `world` crate** ([decision 0024](../decisions/0024-game-storage-is-hecs.md)): `World::from_area` builds the game's storage from the area (built-in components as Rust types; anything else as plain values), `step` runs a tick (world matrices from parents; game code joins in Phase 4), and entities are found by their `EntityId`.
- **Play, Pause and Stop** at the top. Pressing Play:
  1. keeps a copy of the area, its history and selection;
  2. builds the world;
  3. brings the Game panel to the front and marks the editor as playing (the Play button lit, a coloured border around the viewport and Game panels).
- **While playing:** everything in the editor still works and shows the game's state. Changes made in the editor reach the world too. Saving, opening and new areas wait until Stop ("Stop playing first").
- **Stop** throws the world away and puts back the copy: the area, its undo history and the selection, exactly as they were. Assemblies or models changed on disk while playing are applied again afterwards.
- **The `Camera` component:** field of view, near and far. The Game panel shows the view from the first camera in the area (or the editor's camera, with a note, if there's none). Cameras get a marker and are added from the Entity menu.
- **Tests:** world building for every built-in component and unknown ones; Play, edit, Stop leaves the area and history identical (the gate, as a test); pause; blocked saving; the Game view uses the camera.
- **Manual:** a new "Play mode" page.

### M6. Several viewports ☐

- **Layouts** in the viewport's ≡ menu: one view, two side by side, two above each other, or four. Each view has its own camera, look and kind (Perspective, Top, Front, Right), and a button to maximise it. Saved per computer.
- **Flat views** (orthographic): right or middle drag slides, the wheel zooms towards the mouse. The grid lies in the view's own plane.
- Picking, gizmos, dropping from the Library and F work in whichever view the mouse is over.
- **Tests:** layouts round-trip through saved settings and fuzzing; flat-view maths; the grid in each plane on the GPU; picking in a flat view.

## After Phase 3

- **The scale check** runs at the end of the phase (decision 0011), with a new entry for several viewports.
- **Phase 4 (Lust)** builds on the `world` crate: Lust components become `world` components, and hot reload snapshots the world into the document's values (decision 0019).
