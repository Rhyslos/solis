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

As the one crate every reader of files already depends on, it also holds the one way a file is opened (`ordinary_file`): only an ordinary file, never a folder, a device or a pipe (a project can ship a link to one, named like any picture or sound, and reading it would never end), and never more of it than asked. The model, picture, sound, project and text readers all open their files there.

For every crate's tests, behind the `test-support` feature (only ever turned on by a dev-dependency, never in a build of Solis): `test_support::TempFolder`, the one way a test makes a folder in the system's temporary folder. It is always new (`solis-<name>-<process id>-<n>`, never an earlier run's) and is removed with everything in it when dropped, so runs leave nothing behind.

## Must never

- Identify a component by its Rust type (`TypeId`): names only, so Lust libraries can be rebuilt and Lua components defined at run time.
- Clamp a stored value to a field's range: ranges are hints for dragging only.
- Depend on any other Solis crate, or on the GPU or the window. It knows no file's layout either: the one thing it does with files is open them safely (`ordinary_file`), for every crate that reads one.

## Depends on

glam (vectors and rotations) and serde (IDs saved as text). Random IDs use only the standard library.

## Status

Phase 2, milestone 1. Phase 5: `IdMap`, a map keyed by entity IDs with `IdHasher` (one multiplication: IDs are random already), many times quicker than the standard hasher on the game's hot paths. Phase 9, D2: a field's hint can say what it `offers` (`Offer`): the project's assets of a kind (a Sprite's Sheet) or the slices of the sheet in another field (its Pivot), so Properties shows a list to pick from; D3 adds the clips of the sheet in another field (a Sprite's Clip).

Decision 0089: `atlas` joins the data kinds (`tools.atlas.lst`, Atlas Crop). Usable first, L1 (decision 0081): `data_files` names the Lust data files Solis makes (`farm.tilemap.lst`): the one list of kinds, a file's kind and its name without it.
