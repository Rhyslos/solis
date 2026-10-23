# 0019. A scene document now; the game's fast storage in Phase 3

- **Status:** Accepted
- **Date:** 2026-10-23

## Problem

ROADMAP Phase 2 says to decide first how entities and components are stored: `bevy_ecs`, or Solis's own. The answer has to serve several needs at once:

- **the editor;**
- **text scenes** that merge in Git;
- **prefab overrides** by field;
- **undo** for every edit;
- **Lua components** defined while the program runs (Phase 4);
- **Lust code built as a Rust library** and swapped while playing (decision 0016). Its Rust types change identity on every rebuild.
- **lockstep netcode** (Phase 6), which needs the same order every time;
- **the scale target** of 2,000 units and 20,000 projectiles (DESIGN.md §2);
- **no `unsafe` code** without a decision record.

## Options

1. **bevy_ecs with bevy_reflect.**
   - Fast and well known.
   - Version 0.20 (October 2026) breaks its API about three times a year.
   - Components defined at run time (Lua) need its `unsafe` functions (`insert_by_id`, `ComponentDescriptor::new_with_layout`).
   - Its reflection is keyed by Rust type, so it can't follow Lust types across a rebuild. Scenes, prefab overrides and merging would still need a Solis layer on top: two reflection systems.
2. **A small storage crate (`hecs`) plus Solis's own reflection.** Stable and simple, but has no components defined at run time.
3. **Solis's own fast storage now.** Builds the fast part two phases before anything can measure it; Phase 2 runs no simulation.
4. **A scene document now, the game's storage later.**
   - The editor, saving, prefabs, undo and the merge tool all work on one plain-data document of entities and component values, keyed by stable IDs.
   - The running game's packed storage is built from the document when Play is pressed and thrown away on Stop, which is Phase 3's "resets when you stop".

## Decision

Option 4 (the project lead's choice, 2026-10-23).

- **`reflect` (layer 0)** holds:
  - IDs: `EntityId` and `AssetId`, random 64-bit, written as 13 Crockford base-32 characters.
  - `Value`.
  - Component schemas: named fields with kinds, defaults and hints.
  - A registry, looked up by component **name**, never by Rust type.
  - Property paths.
  - A `Component` trait for typed access.

  The few built-in components implement it by hand. A derive macro waits until there are enough components to need one. Lust generates the same code (decision 0016), and Lua builds schemas at run time.
- **`scene` (layer 0)** holds the document.
  - Each entity has a stable ID, a name, a parent, a sibling order, and its components as values.
  - Components the registry doesn't know (a newer version, a Lust script not built yet) are kept untouched and written back.
- **Parenting is part of the entity, not a component.** World transforms are worked out from the hierarchy and never saved.
- **Hand-written forms are replaced:** the editor's inspector is generated from schemas, and only drag ranges come from hints. Values are never clamped to the field's range. That fixes review item 7 for good.
- **The game's fast storage is chosen at the start of Phase 3** with a benchmark: `hecs` against Solis's own safe storage, at the scale target. The projectiles get a packed pool of their own (decision 0016), not one entity each.
- **Hot reload of Lust** (Phase 4) becomes:
  1. snapshot the running game into the document's values;
  2. unload the old library, so no old types are left alive;
  3. load the new one and re-register its components;
  4. rebuild the game from the snapshot.

  Loading a library needs `unsafe`, which gets its own decision then.

## Consequences

- Phase 2 has no storage engine to wait on, and nothing in the document is rewritten later.
- There are two representations once games run (Phase 3): the document and the game's storage. The game is only ever built from the document; while playing, the inspector reads the game's values through the same schemas.
- Built-in components need hand-written reflection code until a derive macro arrives. Each is about 40 lines and tested.
