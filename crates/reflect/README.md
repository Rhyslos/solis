# reflect

**Layer 0 · Core**

How components describe themselves ([decision 0019](../../docs/decisions/0019-scene-document-now-game-storage-later.md)): stable IDs for entities and assets, plain values, component schemas (named fields with kinds, defaults and hints), and a registry looked up by component name.

## Purpose

Give every part of Solis one way to see any component, whether it's built in, written in Lust or defined in Lua:

- the inspector draws fields from a schema;
- area files write values in a schema's field order;
- undo records values;
- assembly overrides name a component and a field.

Nothing here knows Rust types beyond the [`Component`] trait for typed access, so components defined while the program runs work the same way as built-in ones.

## Must never

- Identify a component by its Rust type (`TypeId`): names only, so Lust libraries can be rebuilt and Lua components defined at run time.
- Clamp a stored value to a field's range: ranges are hints for dragging only.
- Depend on any other Solis crate, or on the GPU, the window or files.

## Depends on

glam (vectors and rotations). Random IDs use only the standard library.

## Status

Phase 2, milestone 1. Phase 5: `IdMap`, a map keyed by entity IDs with `IdHasher` (one multiplication: IDs are random already), many times quicker than the standard hasher on the game's hot paths.
