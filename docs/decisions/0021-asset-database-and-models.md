# 0021. The asset database, and models as node trees

- **Status:** Accepted
- **Date:** 2026-10-23

## Problem

ROADMAP Phase 2 asks for an asset database where renaming a file breaks nothing and assets reload when changed. Decision 0006 says each asset's ID lives in a small text file beside it, and generated files live in a cache that's never committed.

The Phase 1 review also found that imported models are flattened into world space:

- hierarchy, names and shared meshes are lost;
- a mesh used ten times is stored ten times;
- nothing can be moved;
- prefabs and stable sub-asset IDs are impossible.

## Options

**Sidecars:**
- one database file: ruled out by decision 0006;
- Unity-style `.meta`;
- Godot-style `.import`;
- a Solis-named file.

**Sub-asset IDs:**
- by index (Blender's re-export renumbers them);
- by name only (fails on duplicate names);
- a stored table.

**Models:**
- keep the flat bake and add one transform per model (blocks prefabs);
- one asset file per mesh (too many files in Git);
- meshes in local space with a node tree and instances.

## Decision

- **The project folder:**
  - `project.solis` (TOML: name, format version);
  - `assets/` holds everything the user owns, and is watched;
  - `.solis/` is a rebuildable cache, never committed.

  New projects get a `.gitignore` (ignoring `/.solis/` and builds) and a `.gitattributes`: Solis's text files `eol=lf`, and models, images and sound through Git LFS. LFS locking waits for the Git panel (Phase 10).
- **Sidecars:** `<file>.solmeta` beside each asset, in TOML. It holds:
  - the asset's `AssetId`;
  - its importer settings;
  - a table of sub-assets (nodes, meshes, materials, textures), each with a small stable `PartId`. Parts are matched on re-import by their name path first, then by name and order among duplicates, then by index. A part that disappears is kept and marked missing, so references say "missing" rather than pointing at the wrong thing.

  Sidecars are written only when something changes, through a temporary file.
- **Renames:**
  - Renaming inside Solis moves the sidecar too.
  - A rename in Explorer leaves an orphaned sidecar. It's paired with a new file of identical content, if exactly one matches.
  - A copy that brought its sidecar along gets a fresh ID.
  - Orphaned sidecars are never deleted automatically.
- **The import cache** in `.solis/cache/` is keyed by a hash of the content and settings. Renaming or switching branches never re-imports. Cache files are read as hostile files and fuzzed.
- **Watching files:**
  - `notify` with a debouncer.
  - Events are only hints: changed files are checked again by size, modification time and hash. That copes with programs that save by renaming.
  - Solis's own writes are ignored, and the folder is rescanned after an overflow.
- **Threads:** a small worker pool replaces one thread per import. Results come back as messages, and an older result for the same asset is dropped.
- **Hot reload:** a re-imported asset is swapped in by ID. If it fails, the old version stays, and the console says why.
- **Models:**
  - A glTF becomes unique meshes in local space and a node tree: name, parent, translation, rotation, scale, meshes, lamp.
  - The Y-up to Z-up turn moves into the transforms, as C·T·C⁻¹, which gives exactly the old result.
  - Mirrored placements are drawn with a front-face-flipped pipeline instead of flipped triangles.
  - The renderer gets per-instance transforms through an instance buffer, which still works on OpenGL.
  - `Model::placed_meshes()` keeps the old flat view, so the Phase 1 tests still check the same thing.
- **A model in a scene is a read-only prefab** (decision 0020). Placing one makes one instance: one object by default, with Unpack for its parts. Entities inside an instance get IDs derived from the instance and the node's `PartId`, so overrides survive re-imports.

## Consequences

- `.solmeta` files must be committed with their assets. RENAMING.md lists the extension.
- Instancing for Phase 5 builds on the instance buffer without touching the vertex path again.
- The cache can be deleted at any time; it rebuilds.
