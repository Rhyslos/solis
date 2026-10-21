# Architecture

Solis is a Rust program split into small, single-purpose crates. **Lower layers never depend on higher ones**, so each piece can be built, tested and replaced on its own, and games can ship without the editor. The `layer-check` tool enforces this on every change.

## Layers

| Layer | Name | Crates | Rule |
| --- | --- | --- | --- |
| 3 | App | `app` | Opens the window, owns the frame loop, wires the rest together. No feature logic. |
| 2 | Editor | `editor` | The editor's interface: top bar, dockable panels, theme. Never needed by a game. |
| 1 | Engine | `render` | What games run on. Knows nothing about the editor. |
| 0 | Core | none yet | Shared building blocks with no GPU, window or interface (math, IDs, components). |

Each crate's `README.md` states its purpose and what it **must never** do. Read it before changing the crate.

Planned: `world` (entities and components, layer 0 or 1) and `assets` (layer 1) in Phase 2; physics, sound and networking crates in later phases. See ROADMAP.md.

## How a frame runs

1. **Input:** winit delivers mouse and keyboard events to egui (through eframe).
2. **Editor:** the workbench draws the top bar and the docked panels and returns any actions (such as Quit) for the app to carry out.
3. **Camera:** the viewport turns the mouse and keys into a `FlyInput`, and `Camera::fly` (in `editor/src/camera.rs`) moves the camera.
4. **Viewport:** the viewport panel asks a `ViewportRenderer` for an image of its size. The app's implementation renders it with `render` on the window's own GPU device and hands it to egui as a texture.
5. **Closing:** eframe saves the window size, the panel layout and the camera in a per-user folder named after `APP_ID`.

## Colour in the viewport

The viewport draws into a plain 8-bit RGBA image, which egui shows as it is. Shaders work in linear colour and convert to sRGB themselves as they write (`encode` in `crates/render/src/viewport.wgsl`). Letting the GPU convert would need a second "view" of the image in another format, which older and software GPUs (OpenGL) can't make, and the editor has to open on those too. Phase 1's lit renderer will draw into a linear HDR image and convert in one final pass.

## Checks

- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` and `cargo deny check` run locally before every push.
- `crates/render/tests/gpu.rs` renders real frames. Without a GPU it skips; with `REQUIRE_GPU=1` a missing GPU is a failure.
- `tools/smoke/window-smoke.sh` opens the real editor on a virtual screen, flies the camera, drags a panel, quits and reopens, and checks the layout and camera came back.
- CI (`.github/workflows/ci.yml`) runs all of these on ready pull requests and by hand only ([decision 0001](docs/decisions/0001-builds-on-demand.md)).
- `.github/workflows/windows-exe.yml` builds a downloadable `solis.exe` when code is merged into main, and by hand ([decision 0010](docs/decisions/0010-windows-exe-on-merge.md)).

## Conventions

- **Z is up.** X is red, Y green and Z blue wherever axes are shown.
- **Units are metres.**
- **The engine never waits on the GPU or on files in the frame loop;** slow work runs on other threads and reports back with messages.
