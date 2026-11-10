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
| `settings.rs` | Settings files in the same text style: a header, then one `key = value;` line per setting (sound events, the mixer); and with blocks of fields, `word ID { … }` (animators, tilesets, tile maps; `read_block_settings_within` caps the blocks for big files). | `read_settings`, `write_settings`, `read_block_settings`, `read_block_settings_within`, `write_block_settings`, `Settings`, `BlockSettings`, `Field` |
| `lust_data.rs` | Lust data files (decision 0081): what every format shares to write and read one, through `lust::read_values`, with exact numbers, and settings blocks kept as Lust things. `read` is the one place every data file passes on its way to Lust: it refuses first what `lust_guard.rs` finds, then whatever `lust_check.rs` doesn't let through, and afterwards a file whose text holds a control character `text` would leave out, so everything read is written back as itself. | `read`, `write_blocks`, `read_blocks`, `Shape`, `DataError` |
| `lust_guard.rs` | Guards for text on its way to the Lust compiler, shared by data files (`lust_data.rs`) and game code (`script::code`): the text split as Lust's own reader splits it, and what Lust can't finish checking found in it (a declared class or the like, a long run of `!`, text holding a value). The data reader's tests sweep thousands of mixed-up files through it and Lust. | `scan`, `Found`, `MAX_PREFIX_RUN` |
| `lust_check.rs` | Checks a data file before Lust does: Lust has no budget, and a file full of mistakes takes it time that grows with their square. Each piece of the file (split by `lust_guard.rs`) is checked against the file's own classes in one pass, so only a `fetch` of them, `lock` values of the right types and `end` are let through, and what reaches Lust it reads in time that grows with the file's length. A file with more than 12 million pieces is refused as too big to read: Lust keeps about 100 bytes for each. Value names are asked of Lust on their own first: one of its own words would set it finding a mistake in every thing of the value. Tests in `lust_check_tests.rs`: each shape let through or refused with its line, the files Lust took minutes over, and a sweep of thousands of muddled files through the check and Lust. | `Classes`, `MAX_VALUES`, `MAX_DEPTH`, `MAX_PIECES` |
| `pixels.rs` | A project's 2D settings (Phase 9, D1): base resolution, pixels per unit, tile size and scaling, in `pixels.solpix`. Reading is fuzz-tested. | `PixelSettings`, `Scaling`, `read`, `write` |
| `text.rs` | Areas as text (decision 0020): one block per entity, sorted by ID, one line per field, so Git merges cleanly. Reading is fuzz-tested. | `write`, `read`, `AreaFile`, `ParseError` |
| `assembly.rs` | Assemblies placed in areas: written as the root plus overrides, expanded into ordinary entities while edited, with IDs derived from the copy's root. | `Expander`, `derive`, `AssemblyError` |
| `gap_tests.rs` | Tests added where mutation testing found the crate's own tests missed something. | |
| [`builtin/`](builtin/README.md) | The built-in components: `Transform`, `Light`, `Environment`, `Model`, `Camera`, and the physics ones. | `registry`, `starter_area` |

## How the pieces fit

An `Area` holds the entities. The editor never changes it directly: it builds `Edit`s, applies them, and records the inverses as one `Step` in `History`. `text.rs` turns an `Area` into a file and back; `assembly.rs` expands placed assemblies on load and folds them back into overrides on save. Component values are `reflect` values, described by the schemas in `builtin/`.

**Read next:** the crate's [README](../README.md), and [ARCHITECTURE.md](../../../ARCHITECTURE.md), "The area".
