# Phase 2 plan: the world

The working plan for Phase 2 (ROADMAP.md, "Phase 2: The world"). It records the choices made before any code, the controls the project lead chose, and a plan for every milestone, so a later session can carry on where this one stopped. Update it as milestones are built: tick them off here and in the README tracker, and change a milestone's plan if its design changes.

**Gate (unchanged):** save an area, change it in a text editor, and see it reload. Rename a texture and nothing breaks. Two people change different entities in the same area on two branches, and Git merges them without a conflict.

## How the plan was made

Before any code, three separate planning reviews each compared several options:

1. entity storage and reflection;
2. the area and assembly format;
3. the asset database, the model structure and undo.

Their recommendations, the Phase 1 review (`docs/reviews/2026-10-phase-1-review.md`, section 2) and the project lead's answers (2026-10-23) are settled in four decisions:

| Decision | What it settles | The project lead chose |
|---|---|---|
| [0019](../decisions/0019-scene-document-now-game-storage-later.md) | Entities, components and how they describe themselves; the fast game storage chosen in Phase 3 | Area document now, game storage in Phase 3 |
| [0020](../decisions/0020-scene-files-in-lust-style.md) | The `.solarea` / `.solassembly` format, IDs, assemblies and overrides | Lust-style data (tested against TOML style with 1,200 real Git merges each: equally mergeable) |
| [0021](../decisions/0021-asset-database-and-models.md) | The project folder, `.solmeta` sidecars, the import cache, watching files, models as node trees drawn with per-instance transforms | (from the reviews) |
| [0022](../decisions/0022-undo-as-reversible-edits.md) | Undo and redo as reversible edits to the area document | (from the reviews) |

The project lead also chose that **the sun, sky and fog live in each area** (saved with it), and the editing controls below.

## The shape of the code

| Crate | Layer | Job | New in |
|---|---|---|---|
| `reflect` | 0 | IDs (`EntityId`, `AssetId`), values, component schemas, the registry, property paths. Depends on glam only. | M1 |
| `area` | 0 | The area document (entities, names, parents, components as values), the built-in components, hierarchy and world transforms, reversible edits and history, the text format, assembly expansion. | M1 |
| `asset_db` | 1 | The project folder, sidecars and IDs, the import cache, file watching, a small worker pool. | M5 |
| `assets` | 1 | Readers only: glTF becomes meshes in local space and a node tree. | M3 |
| `render` | 1 | Gains per-instance transforms (an instance buffer). Never learns about areas. | M3 |
| `editor` | 2 | `Document` (area + history + selection), commands, the Area tree, the generated inspector. | M1–M7 |
| `app` | 3 | Wiring: asset IDs to GPU models, file dialogs, saving and loading. | M5–M6 |

**What changes least:** `render` keeps its API shape. It gains a list of instances per model, and the shader multiplies by the instance's matrix. `assets` keeps its readers and limits. It stops baking positions into world space, and `Model::placed_meshes()` keeps the old flat view, so the Phase 1 tests still check the same thing.

**What gets replaced:** the editor's hand-held Phase 1 state:

- `Viewport.sun`, `Viewport.lamps` and `Viewport.environment`;
- the per-computer `viewport_sun` and `viewport_environment` keys;
- `Selection`, `ModelInfo` and `LampId`;
- the hand-written Properties forms in `lighting.rs` and `environment.rs`.

All of these become entities and components in the document. They're temporary by design (their comments say "until entities arrive in Phase 2").

## Controls (chosen 2026-10-23, CONTRIBUTING rule 17)

| Action | Keys |
|---|---|
| Undo | Ctrl+Z |
| Redo | Ctrl+Y or Ctrl+Shift+Z |
| Delete the selection | Delete |
| Duplicate | Ctrl+D |
| Rename | F2, or double-click in the Area panel |
| Frame the selection | F |
| Save area / Save as | Ctrl+S / Ctrl+Shift+S |
| Open area / New area | Ctrl+O / Ctrl+N |
| Area panel: select | click |
| Area panel: add or remove from the selection | Ctrl+click |
| Area panel: select a range | Shift+click |
| Area panel: make a parent | drag an entity onto another |
| Area panel: move to the top level | drag an entity to empty space |

There are no clashes: the tool keys (Q W R S T) and the fly keys (W A S D Q E while the right mouse button is held) take no modifiers, and none of these keys fire while typing in a field.

## Milestones

The ROADMAP lists Phase 2's parts; they're built in this order. **Undo comes second,** so everything after it is undoable from the start, rather than being added at the end.

### M1. Entities and components ☑

- **`reflect`:**
  - `Id` (random 64-bit, written as 13 Crockford base-32 characters), with `EntityId` and `AssetId` built on it.
  - `Value`: bool, int, float, text, list, choice, asset reference, entity reference.
  - `FieldKind`, `FieldInfo` with hints (range for dragging only, step, unit, colour, angle), `ComponentSchema`, `Registry` (looked up by name, never by Rust type), `PropertyPath`, and the `Component` trait for typed access.
  - No derive macro yet: the handful of built-in components implement it by hand. Lust will generate the same code (decision 0016).
- **`area`:**
  - `Area` holds a `BTreeMap<EntityId, EntityData>`. Each `EntityData` has a name, a parent, a sibling order and components as values.
  - Unknown components are kept as they are, never dropped.
  - Built-in components:
    - `Transform` (position, rotation, scale);
    - `Light` (sun, point or spot, colour, strength, range, cone, shadows);
    - `Environment` (sky, fog, exposure, tone mapping);
    - `Model` (an asset reference and a node).
- **Editor:**
  - A `Document` holds the area and the selection (a set of entity IDs).
  - The sun, the environment, lamps and imported models become entities.
  - The Area panel lists entities.
  - Properties shows each component's fields generated from its schema. This replaces the hand-written forms.
  - The renderer is fed from the document.
- A new area has a Sun and an Environment entity. (Built: the old per-computer sun and environment settings are no longer read; seeding an area from them would have kept the old types alive for one release, and the defaults are what most people had.)
- **Tests:**
  - values and schemas;
  - the registry refuses duplicate names;
  - unknown components survive;
  - every built-in round-trips through values;
  - kittests that the Area panel and Properties show entities;
  - the GPU and smoke tests still pass.

### M2. Undo and redo ☑

- **`area::Edit`:** `SetField`, `AddComponent`, `RemoveComponent`, `Spawn`, `Despawn` (a whole subtree, saved in full), `Rename` and `Reparent`.
  - `apply` returns the inverse edit.
  - `History` holds undo steps. Each step has a label, its edits, a merge key, and the selection before and after.
  - History is limited by memory, not a count.
- **Editor:** every change goes through a command that produces edits. Properties shows a copy of each value and sends `SetField`.
  - A whole drag is one step, merged by (widget, start of interaction).
  - A drag that ends where it started leaves no step.
- **Controls:** Ctrl+Z, Ctrl+Y and Ctrl+Shift+Z. Edit > Undo and Redo, named after the step ("Undo Change Light.strength").
- **Selection:**
  - Not an undo step itself.
  - Undo and redo bring back the selection the step had.
  - Never saved to files.
- **Tests:**
  - every edit followed by its inverse leaves the area unchanged (property test over random edits);
  - drag merging;
  - the memory limit;
  - kittest: change a field, undo, redo.

### M3. Transforms and parenting ☑

- **`area`:** parents and sibling order on each entity, cycle checks, and world transforms worked out from the hierarchy (built: worked out when asked, and the tree in one pass per frame; a cache waits until the scale check in M4 shows it's needed).
- **`assets`:** a glTF becomes unique meshes in local space plus a node tree (name, parent, translation, rotation, scale, meshes, lamp).
  - Y-up to Z-up moves into the transforms, as C·T·C⁻¹.
  - A mesh used several times is stored once.
  - `Model::placed_meshes()` keeps the Phase 1 flat view for tests.
- **`render`:**
  - An instance buffer: each draw carries a world matrix.
  - Mirrored instances use a front-face-flipped pipeline.
  - The shadow box uses world bounds.
  - Grouping by mesh is already in place for Phase 5's instancing.
- **Editor:**
  - Importing a model makes one entity, with its node tree inside it (built: as the project lead asked, one object by default; one entity per node comes with Unpack in M7).
  - The Area panel becomes a tree with the controls above: drag to parent, Ctrl+click and Shift+click, F2, Delete, Ctrl+D.
  - F frames the selection.
  - All of it is undoable.
- **Tests:**
  - the C·T·C⁻¹ placement matches the old bake to 1e-5;
  - a shared mesh is stored once;
  - mirrored instances render like the old flipped bake;
  - reparent cycles are refused;
  - world transforms;
  - kittests for the tree.

### M4. Components that describe themselves ☑

- **The full inspector:**
  - Add component (a menu of registered ones) and Remove component.
  - Hints: slider, unit, colour picker, angles in degrees (rotation shown as degrees, stored as a quaternion).
  - Choices shown as drop-downs.
  - Missing components shown as "Missing component: Health" with Remove.
- **A public registration path** for game components, used by Lust in Phase 4 and by Lua.
- **A scale-check entry:** an area of 2,000 entities listed and inspected within the editor's frame target (rule 18). This needs the Area tree to draw only the rows in view.
- **Tests:** a made-up component registered at run time shows in the inspector and edits with undo; the scale check.

### M5. The asset database ☐

- **A project folder:**
  - `project.solis` (TOML: name, format version);
  - `assets/`, which is watched;
  - `.solis/`, a rebuildable cache that is never committed.
- **New project** writes `.gitignore` and `.gitattributes`, with Git LFS for models, textures and sound, and `text eol=lf` for Solis's own files.
- **File menu:** New project…, Open project…, and recent projects. The last project reopens on start.
- **Sidecars:**
  - Each asset has `<file>.solmeta` (TOML) beside it, holding its `AssetId`, its importer settings, and a table of stable sub-asset IDs (`PartId`) for nodes, meshes and materials, matched by name path, then by name and order, then by index.
  - Written only when something changes, through a temporary file (rule 11).
- **The import cache:**
  - Keyed by a hash of the content and settings, so renaming never re-imports.
  - `.solbin` files are read as hostile and fuzzed.
- **Renames:**
  - Renaming in Explorer leaves the sidecar behind. An orphaned sidecar is paired with a new file of the same content.
  - Sidecars are never deleted automatically.
  - Copies that brought their sidecar along get a fresh ID.
- **File watching:** `notify` with a debouncer. Events are only hints: changed files are checked again by size, modification time and hash.
  - Saves by rename, and Solis's own writes, are handled.
  - On overflow the folder is rescanned.
- **Threads:** a small worker pool replaces one thread per import. Results come back as messages; an older result is dropped if a newer one exists.
- **Hot reload:** a model changed on disk is re-imported and swapped in by `AssetId`. If the new version fails, the old one stays, and the console says why.
- **Tests:**
  - sidecar round trip and fuzzing;
  - renames and moves, part IDs kept through a re-export with nodes reordered;
  - the cache hit after a rename;
  - the watcher, with a simulated save by rename;
  - the template files.

### M6. Areas saved as text ☐

- **The format** (decision 0020): Lust-style data blocks, one per entity, sorted by ID, one line per field.
  - Floats written as the shortest text that reads back exactly.
  - Unknown components kept exactly as written.
  - A version line, and step-by-step migration of older versions.
  - Solis's own small reader, with limits checked before allocating and a fuzz test.
  - Saves go through a temporary file and a swap.
- **Editor:**
  - New, Open, Save and Save as, with Ctrl+N, Ctrl+O, Ctrl+S and Ctrl+Shift+S.
  - The tab and title show the area's name, with a dot while there are unsaved changes.
  - Closing with unsaved changes asks first.
  - An area changed on disk reloads as one undoable step, "Reloaded harbour.solarea from disk". With unsaved changes, it asks "Keep mine" or "Load the file".
  - References to assets are written as IDs, with the file's path as a comment. The comment is refreshed on save and never read.
- **Tests:**
  - round trips;
  - every f32 bit pattern reads back exactly (a long test, not run every time);
  - real `git merge-file` three-way merges: two adds, different entities side by side, different components of one entity, reordered children, a delete next to an edit;
  - fuzzing the reader;
  - a newer version is refused with a message;
  - reload on disk change.

### M7. Assemblies ☐

- **A `.solassembly`** is the same format with one root.
- **An instance in an area** is a reference plus overrides, never a copy.
  - Overrides are kept per field, by (entity inside the assembly, component, field).
  - Entities inside an instance are addressed as `instance/inner`. Their blocks sort straight after the instance's.
- **Nesting:** up to 16 levels. Expanding stops at the entity limit, which is counted before allocating, so a file can't explode.
- **Cycles** are refused, naming the chain.
- **An imported model is a read-only assembly.** Placing it makes one instance, as the project lead asked: one object by default.
  - **Unpack** turns an instance into plain entities.
  - Entities inside a model instance get IDs derived from (instance, node `PartId`), so overrides survive a re-import.
- **Editor:**
  - Entity > Create assembly from selection, and placing an assembly.
  - Overridden fields shown in bold, with Revert and Apply to assembly.
  - Unpack.
  - An override whose target is gone is shown as a warning with Remove, never dropped.
  - An assembly changed on disk rebuilds its instances.
- **Tests:**
  - nested resolution order;
  - overrides survive edits to the assembly;
  - cycle and size limits;
  - apply, revert and unpack round trips;
  - the gate's merge case with assemblies.

## After Phase 2

- **Phase 3 starts by choosing the game's fast storage** (decision 0019): a one-week benchmark of `hecs` against Solis's own safe storage, both built from the area document and thrown away when Play stops.
- **Picking in the viewport and gizmos** come in Phase 3 too. Every gizmo drag is one `SetField` step, merged like a Properties drag.
