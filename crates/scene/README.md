# scene

**Layer 0 · Core**

The scene document ([decision 0019](../../docs/decisions/0019-scene-document-now-game-storage-later.md)): entities with stable IDs, names, parents and sibling order, and their components as plain values described by schemas (the `reflect` crate). Also the built-in components: `Transform`, `Light`, `Environment` and `Model`.

## Purpose

Be the one place a scene lives while it's edited. The editor, saving and loading, prefabs and undo all work on this document. The running game's fast storage (Phase 3) is built from it when Play is pressed.

- **Built-in components** (`builtin/`) are typed Rust structs that turn into values and back through `reflect::Component`.
- **Components nobody registered** (a newer version, a script that isn't built) are kept untouched, never dropped.
- **A new scene** has a Sun and an Environment (the sun, sky and fog live in each scene).

## Must never

- Hold code from a game or a script library: only values, so swapping a library never leaves stale types behind.
- Drop data it doesn't understand.
- Depend on the GPU, the window, files or the editor.

## Depends on

`reflect`, and glam.

## Status

Phase 2, milestone 1: the document and the built-in components.
