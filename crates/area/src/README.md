# area/src

The area document (decision 0019): entities with stable IDs, their components as plain values, changed only through reversible edits, which undo and redo are made of (decision 0022). Also how areas are written as text, and how assemblies are placed in them.

**Start reading at** `document.rs`, then `edit.rs`.

| File | What it does | Main items |
| --- | --- | --- |
| `lib.rs` | Lists the modules and what the crate offers. | |
| `document.rs` | The document itself: entities by ID, each with a name, a parent, a sibling order and components. | `Area`, `EntityData`, `Origin` |
| `edit.rs` | Reversible edits: every change is one, and applying one gives back its inverse, which is how undo works. | `Edit`, `EditError` |
| `history.rs` | Undo and redo: steps of edits with the selection each step had; drags merge into one step. | `History`, `Step`, `MergeKey`, `Weigh` |
| `tree.rs` | The area's tree worked out once: children in order, tree order, where each entity is in the world. | `Tree` |
| `settings.rs` | Settings files in the same text style: a header, then one `key = value;` line per setting (sound events, later the mixer). | `read_settings`, `write_settings`, `Settings`, `Field` |
| `pixels.rs` | A project's 2D settings (Phase 9, D1): base resolution, pixels per unit, tile size and scaling, in `pixels.solpix`. Reading is fuzz-tested. | `PixelSettings`, `Scaling`, `read`, `write` |
| `text.rs` | Areas as text (decision 0020): one block per entity, sorted by ID, one line per field, so Git merges cleanly. Reading is fuzz-tested. | `write`, `read`, `AreaFile`, `ParseError` |
| `assembly.rs` | Assemblies placed in areas: written as the root plus overrides, expanded into ordinary entities while edited, with IDs derived from the copy's root. | `Expander`, `derive`, `AssemblyError` |
| `gap_tests.rs` | Tests added where mutation testing found the crate's own tests missed something. | |
| [`builtin/`](builtin/README.md) | The built-in components: `Transform`, `Light`, `Environment`, `Model`, `Camera`, and the physics ones. | `registry`, `starter_area` |

## How the pieces fit

An `Area` holds the entities. The editor never changes it directly: it builds `Edit`s, applies them, and records the inverses as one `Step` in `History`. `text.rs` turns an `Area` into a file and back; `assembly.rs` expands placed assemblies on load and folds them back into overrides on save. Component values are `reflect` values, described by the schemas in `builtin/`.

**Read next:** the crate's [README](../README.md), and [ARCHITECTURE.md](../../../ARCHITECTURE.md), "The area".
