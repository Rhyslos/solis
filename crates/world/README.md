# world

**Layer 1 · Engine**

The game's fast storage while it runs ([decision 0024](../../docs/decisions/0024-game-storage-is-hecs.md)).

## Purpose

Hold the running game's entities in a form a game can run over quickly. The area (`area::Area`) stays the document the editor edits, saves and undoes; `World::from_area` copies it into `hecs` when Play is pressed:

- built-in components as their Rust types (`Transform`, `Light`, `Model`, `Environment`, `Camera`);
- anything else (game code's components, unknown ones) as plain values, in `Values`;
- each entity's area ID (`Id`), name (`Name`) and parent (`Parent`), and where it is in the world (`Global`), worked out parents first.

`step` moves the clock on in fixed ticks (1/60 s, at most 8 a call, so a stall skips ahead instead of racing). Entities are found by their area ID (`entity`, `get`, `global`), and `placed` lists every entity with a given component, in the area's tree order. `update` takes in changes to a few entities' components and names (what the editor does while playing, such as each frame of a drag), keeping where the game put an entity unless its Transform itself changed, and `rebuild` builds the world again from a changed area; both keep the clock. Drawing between ticks: `remember` keeps where every entity is as a tick starts, `progress` says how far the clock is into the next tick, and `placed_between` and `camera_between` give places part way between (`between`, numbers moved in straight lines); `step_one` runs exactly one tick (Step, while paused). `add_subtree` adds a tree of entities at the end (what game code spawned) and `remove_subtrees` takes trees out, without building the world again; `subtree` lists an entity and everything under it, and `is_inside` says whether one entity is under another with one look at the tree order (each subtree is one run of it). Physics (decision 0036): every entity with a `RigidBody` or `Collider` has a body in `physics()` (made at `from_area`, joined by `add_subtree`, left by `remove_subtrees`, remade by `update` keeping its speed, and kept with its speed through `rebuild`); `push` and `set_velocity` wait for the next `step_physics`, which also moves bodies to where game code put their entities, steps, and moves the dynamic bodies' entities after them. `seed` is the game's random seed (`set_seed`); a save game is put back with `set_clock`, `restore_physics` and `Projectiles::restore`; `due` says how many ticks a frame makes due and `begin_tick` moves the clock on as each begins (`step` does both), and `checksum` sums the world's state for determinism checks. `changes` gives a number that changes whenever anything that can be seen does (places, entities, the clock, projectiles), so what's drawn of the world is made again only then. Both keep where game code put an entity: the world remembers the area's Transform for each entity game code moved, and keeps the game's place while the area still has that one (an undo or edit that moves it takes the area's). Game code moves and turns entities with `set_transform` or `change_transform` (one lookup, given the parent's place: the entity's place follows at once, those under it at `settle`, which works out only the subtrees of entities that moved and have children), and finds them by name with `find`, from an index of each name's first entity (kept up to date by `update` and `rebuild`). Stepping doesn't work out places: nothing moves by itself yet. Projectiles (`projectiles.rs`): a pool of things fired by game code that aren't entities, each with a place, velocity, time left and colour, capped at `MAX_PROJECTILES` (a million; shots past it are counted in `refused`). `Projectiles::tick` moves them in jobs and lets expired ones go, in the order fired; the editor ticks them once per world tick, before game code. They survive the world being built again. `revision` changes whenever the entities, their order or their names change (never repeating, even in a world built again), so game code's view of who's who (`entities`, `globals`) is made again only then. `placed` and `first` ask `hecs` for the entities with one component only (each carries its place in the tree order, to keep that order), not walking all of them.

## Must never

- Change the area: Stop throws the world away and the area is put back.
- Know about the editor, the window or the GPU.
- Hang or panic on a damaged area (a parent loop, a missing parent).

## Depends on

`area`, `reflect`, `physics`, glam and hecs.

## Status

Phase 3: built from an area, stepped, read by ID. Phase 4: game code moves entities between ticks. Phase 6: drawing between ticks, physics, the seed and checksums. Phase 7, S5: `audio_sources` lists every `AudioSource` (by ID, from a query, so a world without them costs nothing) and `set_sound_playing` switches one (game code's `setSoundPlaying`); `sounding_audio_sources` copies only those playing with an event named (what the app plays each frame); which are on is in the checksum, read without copying the sources. S6: `ReverbZone`s are kept as components, for `placed_between`.
