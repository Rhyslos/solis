# world

**Layer 1 · Engine**

The game's fast storage while it runs ([decision 0024](../../docs/decisions/0024-game-storage-is-hecs.md)).

## Purpose

Hold the running game's entities in a form a game can run over quickly. The area (`area::Area`) stays the document the editor edits, saves and undoes; `World::from_area` copies it into `hecs` when Play is pressed:

- built-in components as their Rust types (`Transform`, `Light`, `Model`, `Environment`, `Camera`);
- anything else (game code's components, unknown ones) as plain values, in `Values`;
- each entity's area ID (`Id`), name (`Name`) and parent (`Parent`), and where it is in the world (`Global`), worked out parents first.

`step` moves the clock on in fixed ticks (1/60 s, at most 8 a call, so a stall skips ahead instead of racing). Entities are found by their area ID (`entity`, `get`, `global`), and `placed` lists every entity with a given component, in the area's tree order. `update` takes in changes to a few entities' components and names (what the editor does while playing, such as each frame of a drag), keeping where the game put an entity unless its Transform itself changed, and `rebuild` builds the world again from a changed area; both keep the clock. Both keep where game code put an entity: the world remembers the area's Transform for each entity game code moved, and keeps the game's place while the area still has that one (an undo or edit that moves it takes the area's). Game code moves and turns entities with `set_transform` or `change_transform` (one lookup, given the parent's place: the entity's place follows at once, those under it at `settle`, which works out only the subtrees of entities that moved and have children), and finds them by name with `find`, from an index of each name's first entity (kept up to date by `update` and `rebuild`). Stepping doesn't work out places: nothing moves by itself yet. `revision` changes whenever the entities, their order or their names change (never repeating, even in a world built again), so game code's view of who's who (`entities`, `globals`) is made again only then. `placed` and `first` ask `hecs` for the entities with one component only (each carries its place in the tree order, to keep that order), not walking all of them.

## Must never

- Change the area: Stop throws the world away and the area is put back.
- Know about the editor, the window or the GPU.
- Hang or panic on a damaged area (a parent loop, a missing parent).

## Depends on

`area`, `reflect`, glam and hecs.

## Status

Phase 3: built from an area, stepped, read by ID. Phase 4: game code moves entities between ticks.
