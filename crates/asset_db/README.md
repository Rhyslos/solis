# asset_db

**Layer 1 · Engine**

The project folder and its assets ([decision 0021](../../docs/decisions/0021-asset-database-and-models.md)).

## Purpose

Know every asset in a project (models, and assemblies: `.solassembly`) by an ID that survives renaming, read them quickly, and notice when they change; list every file and folder for the Library, and rename, move and delete them with their sidecars (`files.rs`; deleting goes to the Recycle Bin through `trash`).

- **The project** (`project.rs`): `project.solis` (TOML: name, format), `assets/` with everything the user owns, and `.solis/`, a cache that can be deleted at any time. New projects get a `.gitignore` and a `.gitattributes` (Solis's text files with Unix line endings; models, images and sound through Git LFS).
- **Sidecars** (`sidecar.rs`): `<file>.solmeta` beside each asset, in TOML: its `AssetId`, its content hash (to find it again after a rename outside Solis), importer settings, and stable part IDs for its nodes, meshes and materials. Parts are matched on re-import by name path, then name and order, then index; lost parts are kept, marked missing. Written only when something changes, through a temporary file.
- **The database** (`db.rs`): worked out from the folder. A renamed file finds its orphaned sidecar by content; a copy that brought its sidecar gets a new ID; sidecars are never deleted. `rescan` reports what changed: added, changed (the file or one it reads, such as a `.gltf`'s `.bin`), moved, removed.
- **The import cache** (`cache.rs`): imported models in `.solis/cache/*.solbin`, keyed by content and import limits, so renaming never imports again. Entries are read as hostile and fuzzed.
- **Watching** (`watch.rs`): `notify` with a debouncer. Events are only hints to rescan.
- **Workers** (`jobs.rs`): a small thread pool; an older result for the same asset is dropped.

## Must never

- Delete a user's file, sidecars included.
- Trust a sidecar or cache file: both are read within limits, and a damaged one is explained or ignored, never a crash.
- Know about the editor or the GPU.

## Depends on

`assets`, `reflect`, serde and toml, notify and notify-debouncer-mini, glam.

## Status

Phase 2, milestone 5: projects, sidecars, the cache, watching and workers. Phase 3: listing every file, and renaming, moving, new folders and deleting from the Library.
