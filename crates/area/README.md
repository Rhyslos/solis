# area

**Layer 0 · Core**

The area document ([decision 0019](../../docs/decisions/0019-scene-document-now-game-storage-later.md)): entities with stable IDs, names, parents and sibling order, and their components as plain values described by schemas (the `reflect` crate). Also the built-in components: `Transform`, `Light`, `Environment`, `Model`, `Camera` (what the game looks through: its View, perspective or 2D, field of view, near and far, with its projection), and from Phase 6 `RigidBody` (dynamic, kinematic or fixed; mass, damping, gravity, upright, fast) `Collider` (box, sphere or capsule; size, radius, height, centre, friction, bounciness) and `CharacterBody` (a standing capsule moved by game code: height, radius, step height, steepest slope, keeps to the ground, mass; `collider` gives its capsule), which the game's world turns into physics.

## Purpose

Be the one place an area lives while it's edited. The editor, saving and loading, assemblies and undo all work on this document. The running game's fast storage (Phase 3) is built from it when Play is pressed.

- **Built-in components** (`builtin/`) are typed Rust structs that turn into values and back through `reflect::Component`.
- **Components nobody registered** (a newer version, a script that isn't built) are kept untouched, never dropped.
- **Edits** (`Edit`) are the only way the editor changes an area: set a field, add or remove a component, spawn, despawn (a whole subtree), rename, reparent. Applying one returns its inverse ([decision 0022](../../docs/decisions/0022-undo-as-reversible-edits.md)); a list that fails part-way is rolled back.
- **Tree** (`Tree`): the area's tree worked out once (children in order, tree order, the first of some entities, every world matrix in one pass), for a caller to keep until entities are added, deleted or reparented.
- **History** (`History`) keeps undo steps of edits, with the selection before and after (of the caller's type). Steps with the same `MergeKey` in a row merge (a whole drag), keeping only each field's first and last value; a merged step that changes nothing disappears. Steps are shared, so copying a history is cheap, and each step's selections count toward its limit (`Weigh`). History is limited by memory, not by count.
- **The tree:** `hierarchy()` and `depth_first()` give every entity's children in one pass; `order_after` and `order_at_end` make sibling orders (short base-36 counters) so new entities go last; `world_matrix` adds up parents' transforms.
- **Text** (`text.rs`): areas as Lust-style data blocks ([decision 0020](../../docs/decisions/0020-scene-files-in-lust-style.md)): `write` and `read`, one block per entity sorted by ID, numbers as the shortest text that reads back exactly, unknown components kept, a version line, limits checked before keeping anything, every mistake reported with its line. No file access: the app reads and writes the files.
- **Assemblies** (`assembly.rs`): a placed assembly is its root entity (with an `Origin`) plus overrides in the file, and ordinary entities while edited. `Expander` expands copies (IDs from `derive(copy root, path)`, so the same file gives the same IDs), collapses them back to what differs, counts entities before making any, refuses loops (naming the chain) and nesting past 16. Overrides whose part is gone are kept. `set_in_file` is Apply to assembly.
- **2D settings** (`pixels.rs`, Phase 9, D1): a project's base resolution, pixels per unit, tile size and scaling, read from and written to `pixels.solpix` (a settings file, format 1), values out of range brought in with a note.
- **A new area** has a Sun and an Environment (the sun, sky and fog live in each area).

## Must never

- Hold code from a game or a script library: only values, so swapping a library never leaves stale types behind.
- Drop data it doesn't understand.
- Depend on the GPU, the window, files or the editor.

## Depends on

`reflect`, and glam.

## Status

Phase 2 complete: the document, the built-in components, edits and undo history, the tree, and the text format. Phase 7, S2: settings files (`settings.rs`) in the same text style, for sound events and later the mixer.
