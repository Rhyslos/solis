# Architecture

Solis is a Rust program split into small, single-purpose crates. **Lower layers never depend on higher ones**, so each piece can be built, tested and replaced on its own, and games can ship without the editor. The `layer-check` tool enforces this on every change.

## Layers

| Layer | Name | Crates | Rule |
| --- | --- | --- | --- |
| 3 | App | `app` | Opens the window, owns the frame loop, wires the rest together. No feature logic. |
| 2 | Editor | `editor` | The editor's interface: top bar, dockable panels, theme. Never needed by a game. |
| 1 | Engine | `render`, `assets` | What games run on. Knows nothing about the editor. `assets` reads model files; `render` draws. |
| 0 | Core | none yet | Shared building blocks with no GPU, window or interface (math, IDs, components). |

Each crate's `README.md` states its purpose and what it **must never** do. Read it before changing the crate.

Planned: `world` (entities and components, layer 0 or 1) in Phase 2; physics, sound and networking crates in later phases. See ROADMAP.md.

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
- CI (`.github/workflows/ci.yml`) runs all of these on ready pull requests and by hand only ([decision 0001](docs/decisions/0001-builds-on-demand.md)).
- `.github/workflows/changelog.yml` fails a ready pull request that changes code without a `CHANGELOG.md` entry, unless it is labelled `internal`.
- `.github/workflows/windows-exe.yml` builds a downloadable `solis.exe` when code is merged into main, and by hand ([decision 0010](docs/decisions/0010-windows-exe-on-merge.md)).

## Conventions

- **Z is up.** X is red, Y green and Z blue wherever axes are shown.
- **Units are metres.**
- **The engine never waits on the GPU or on files in the frame loop;** slow work runs on other threads and reports back with messages.
