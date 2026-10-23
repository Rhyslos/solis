# Architecture

Solis is a Rust program split into small, single-purpose crates. **Lower layers never depend on higher ones**, so each piece can be built, tested and replaced on its own, and games can ship without the editor. The `layer-check` tool enforces this on every change.

## Layers

| Layer | Name | Crates | Rule |
| --- | --- | --- | --- |
| 3 | App | `app` | Opens the window, owns the frame loop, wires the rest together. No feature logic. |
| 2 | Editor | `editor` | The editor's interface: top bar, dockable panels, theme. Never needed by a game. |
| 1 | Engine | `render`, `assets`, `asset_db` | What games run on. Knows nothing about the editor. `assets` reads model files; `render` draws; `asset_db` keeps the project's assets by stable ID. |
| 0 | Core | `reflect`, `area` | Shared building blocks with no GPU, window or interface. `reflect` holds IDs, values and component schemas; `area` holds the area document and the built-in components. |

Each crate's `README.md` states its purpose and what it **must never** do. Read it before changing the crate.

Planned: the game's own fast storage in Phase 3, on `hecs` in a `world` crate ([decision 0024](docs/decisions/0024-game-storage-is-hecs.md)); physics, sound and networking crates in later phases. See ROADMAP.md.

## The area

The editor edits a **area document** (`area::Area`): entities by stable random ID, each with a name, a parent, an order among its siblings, and components stored as plain values (`reflect::ComponentData`). A component's fields are described by a **schema** looked up by name in a `reflect::Registry`, so Properties draws any component (built-in now, Lust and Lua later, registered through `Workbench::register_component`) without knowing it, and a component nobody registered is kept untouched and shown as missing ([decision 0019](docs/decisions/0019-scene-document-now-game-storage-later.md)). Built-in components (`Transform`, `Light`, `Model`, `Environment`) are Rust structs that convert to and from their values.

Panels never change the document: Properties returns `PropertyRequest`s and the Area panel `AreaRequest`s, and the workbench turns them into commands. What only changes how the editor shows the area (the Area panel's search, folded, hidden and locked entities, like the selection) lives in the editor and is never saved or undone; hidden entities are left out of the viewport's `AreaView` and locked ones get no pick number. A command is a list of reversible `area::Edit`s applied through `Document::edit`, the only way the area changes, which records it as one undo step in `area::History` ([decision 0022](docs/decisions/0022-undo-as-reversible-edits.md)). A drag is one step: changes carry a merge key of (what's changed, which mouse press), and steps with the same key in a row merge. Each frame the workbench turns the document into an `AreaView` (models to draw, the sun, lamps placed in the world, the environment), which the app maps to the renderer's types: each model becomes a `render::Placement` (the uploaded model and its entity's world matrix). The renderer gathers every mesh copy of every placement into one instance buffer per frame, grouped so each mesh is drawn once for all its copies, mirrored copies with front-face-flipped pipelines ([decision 0021](docs/decisions/0021-asset-database-and-models.md)). A model's own node tree places its meshes inside it; one imported model is one entity until it's unpacked (with assemblies).

## How a frame runs

1. **Input:** winit delivers mouse and keyboard events to egui (through eframe).
2. **Editor:** the workbench draws the top bar and the docked panels and returns any actions (such as Quit) for the app to carry out.
3. **Camera:** the viewport turns the mouse and keys into a `FlyInput`, and `Camera::fly` (in `editor/src/camera.rs`) moves the camera.
4. **Imports:** File > Import model (or a file dropped on the window) asks the app to import; the app reads the file with `assets` on a thread of its own, then uploads it with `render` and tells the workbench, which lists it (with any lamps the file carries) and frames the camera on it.
5. **Viewport:** the viewport panel asks a `ViewportRenderer` for an image of its size, with the sun's settings and the lamps. The app's implementation renders it with `render` on the window's own GPU device (first the sun's shadow map, then the view) and hands it to egui as a texture.
6. **Closing:** eframe saves the window size, the panel layout, the camera, the look and the sun's settings in a per-user folder named after `APP_ID`.

## Colour in the viewport

The world is drawn in linear HDR colour (brighter than white allowed) into a 16-bit floating-point image, with multisampling. A last pass (`post_fs` in `crates/render/src/viewport.wgsl`) applies exposure and tone mapping (AgX, ACES or none) and converts to sRGB into a plain 8-bit RGBA image, which egui shows as it is ([decision 0017](docs/decisions/0017-hdr-tone-mapping-sky-and-fog.md)). The shader converts to sRGB itself, because letting the GPU do it would need a second "view" of the image in another format, which older and software GPUs (OpenGL) can't make, and the editor has to open on those too.

The HDR image's alpha channel is a mask, not opacity: 1 where the lit world is (tone mapped), 0 for the editor's backdrop and the lines over it (shown exactly as the theme has them). Lines and the grid therefore blend their colour only and leave alpha alone, and they pick an opacity that looks the same as blending in sRGB (`overlay_alpha`), as the mockup was drawn.

## Checks

- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` and `cargo deny check` run locally before every push.
- `crates/render/tests/gpu.rs` renders real frames. Without a GPU it skips; with `REQUIRE_GPU=1` a missing GPU is a failure.
- Fuzz tests feed thousands of randomly damaged inputs (saved layouts, saved cameras, command lines) to everything that reads them; none may crash.
- `tools/scale/scale-check.sh` times the editor and the viewport against their targets, at every phase gate and on request ([decision 0011](docs/decisions/0011-scale-check-at-phase-gates.md)).
- `tools/smoke/window-smoke.sh` opens the real editor on a virtual screen, flies the camera, drags a panel, quits and reopens, and checks the layout and camera came back.
- CI (`.github/workflows/ci.yml`) runs all of these on every update to a pull request and on merges into main, on Linux and on Windows (Direct3D 12 through WARP for the GPU tests) ([decision 0018](docs/decisions/0018-checks-for-a-public-repo.md)). `.github/workflows/security.yml` checks for security advisories weekly, and Dependabot opens grouped update pull requests weekly.
- `.github/workflows/changelog.yml` fails a ready pull request that changes code without a `CHANGELOG.md` entry, unless it is labelled `internal`.
- `.github/workflows/windows-exe.yml` builds a downloadable `solis.exe` when code is merged into main, and by hand ([decision 0010](docs/decisions/0010-windows-exe-on-merge.md)).

## Conventions

- **Z is up.** X is red, Y green and Z blue wherever axes are shown.
- **Units are metres.**
- **The engine never waits on the GPU or on files in the frame loop;** slow work runs on other threads and reports back with messages.

## Picking and gizmos

Each model the viewport draws carries a pick number (`editor::ViewModel::pick`). A click or a box in the viewport asks the renderer about a rectangle of the image (`ViewportView::pick`); `render` draws the placements once more into a small ID image (each pixel: the placement's number and its depth), inside that rectangle only, and copies it back without waiting (`ViewportRenderer::request_pick`, `ViewportTarget::poll_pick`). The answer reaches the editor a frame or so later (`ViewportRenderer::picked`), which turns numbers back into entities with the list it kept when it asked, and applies the selection rules in `editor::picking`. Lamps are picked by their markers on screen instead.

The gizmo's maths (`editor::gizmo`, from Aventail) turns mouse movement into a change; `editor::transforming` works out each moved entity's new Transform from where it was when the drag began, and the workbench writes it as `SetField` edits merged by the drag's gesture, so a drag is one undo step and a cancelled drag leaves none ([decision 0025](docs/decisions/0025-picking-and-gizmos.md)). The viewport only reports what the mouse did (`ViewportEvent`s); the workbench changes the area.

## Projects and assets

A project is a folder: `project.solis`, `assets/` and the `.solis/` cache. `asset_db::AssetDb` is worked out from `assets/` (the folder is the truth); each asset's `AssetId` lives in a `.solmeta` sidecar beside it, so renaming, moving or switching Git branches breaks nothing. The app (`crates/app/src/project.rs`) holds the database and a watcher; when the watcher reports something, the database rescans and the app reloads changed models in use on worker threads, through the import cache, and swaps them in by ID. A model imported while a project is open is copied into `assets/` first. The editor only sees `ProjectInfo` and the Library's list ([decision 0021](docs/decisions/0021-asset-database-and-models.md)).

## Area files

An area is saved as a `.solarea` text file: `area::text::write` and `read` turn an `Area` into Lust-style blocks and back, made to merge in Git ([decision 0020](docs/decisions/0020-scene-files-in-lust-style.md)). The editor never touches files: it asks (`SaveArea`, `OpenArea`…) and the app (`crates/app/src/area_file.rs`) shows the system's windows, writes through a temporary file and a swap, and checks the open file once a second for changes made elsewhere, which the editor loads as one undoable step. `area::History::state` tells whether there are unsaved changes: it comes back to the saved value when undo or redo return to the save.

## Assemblies

An assembly is a `.solassembly` file (the area format with one root). In an area it's written as the copy's root entity naming the assembly plus overrides (`root/inner` blocks with only what differs); while edited, copies are ordinary entities carrying an `Origin` (assembly, copy root, path inside), with IDs worked out from the copy's root and the path (`area::assembly::derive`), so the same file always expands to the same IDs ([decision 0020](docs/decisions/0020-scene-files-in-lust-style.md)). `area::assembly::Expander` expands on opening and collapses on saving, comparing each copy with its assembly. The editor holds the assembly files (`Workbench::set_assembly`); when one changes, every copy is collapsed against the old contents and expanded against the new, so its changes survive. A placed model is one entity; Unpack splits it into entities that each draw one node (`render::Placement::node`).

