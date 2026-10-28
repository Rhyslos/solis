# crates

The engine and editor's code, split into small crates (Rust packages), one job each. Lower layers never depend on higher ones, so games can ship without the editor ([ARCHITECTURE.md](../ARCHITECTURE.md), "Layers").

| Folder | Layer | What it does |
| --- | --- | --- |
| [`app`](app/README.md) | 3 App | The program itself: opens the window, owns the frame loop, wires everything together. |
| [`updater`](updater/README.md) | 3 App | Finds, downloads and checks new builds on GitHub. |
| [`editor`](editor/README.md) | 2 Editor | The editor's interface: top bar, dockable panels, viewport tools, play mode. Games never need it. |
| [`render`](render/README.md) | 1 Engine | Draws viewports on the GPU: sky, models, lights, shadows, grid, projectiles. |
| [`assets`](assets/README.md) | 1 Engine | Reads model files (glTF) and textures into plain data, safely. |
| [`asset_db`](asset_db/README.md) | 1 Engine | The project folder: its assets by stable ID, sidecars, the import cache, watching for changes. |
| [`world`](world/README.md) | 1 Engine | The running game's fast storage, built from the area when Play is pressed. |
| [`script`](script/README.md) | 1 Engine | Game code: Lust, Lua and Rust components, run safely on every core. |
| [`physics`](physics/README.md) | 1 Engine | Physics, the same result on every PC. |
| [`input`](input/README.md) | 1 Engine | Keys and buttons turned into named actions, and recordings of play. |
| [`sound`](sound/README.md) | 1 Engine | Reads WAV and Ogg Vorbis files and plays sound. |
| [`area`](area/README.md) | 0 Core | The area document the editor edits, its undo, its text files, and the built-in components. |
| [`reflect`](reflect/README.md) | 0 Core | Stable IDs, plain values, component schemas and the registry. |

Each crate's folder holds `Cargo.toml` (its name, layer and dependencies), `README.md` (its purpose and what it must never do) and `src/` (its code, with a README listing every file); some also have `tests/` or `examples/`. `tools/layer-check` reads every crate here and fails the tests if one breaks the layer rules or lacks a README with `## Purpose` and `## Must never`.

**Adding a crate:** propose it first (CONTRIBUTING.md, rule 1), then add it to the table above, to `ARCHITECTURE.md`, and to `[workspace.dependencies]` in the root `Cargo.toml`.

**Read next:** [ARCHITECTURE.md](../ARCHITECTURE.md) for how a frame runs through these crates, then the README of the crate you will change.
