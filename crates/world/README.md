# world

**Layer 1 · Engine**

The game's fast storage while it runs ([decision 0024](../../docs/decisions/0024-game-storage-is-hecs.md)).

## Purpose

Hold the running game's entities in a form a game can run over quickly. The area (`area::Area`) stays the document the editor edits, saves and undoes; `World::from_area` copies it into `hecs` when Play is pressed:

- built-in components as their Rust types (`Transform`, `Light`, `Model`, `Environment`, `Camera`);
- anything else (game code's components, unknown ones) as plain values, in `Values`;
- each entity's area ID (`Id`), name (`Name`) and parent (`Parent`), and where it is in the world (`Global`), worked out parents first.

`step` moves the clock on in fixed ticks (1/60 s, at most 8 a call, so a stall skips ahead instead of racing). Entities are found by their area ID (`entity`, `get`, `global`), and `placed` lists every entity with a given component, in the area's tree order. `rebuild` builds the world again from a changed area, keeping the clock.

## Must never

- Change the area: Stop throws the world away and the area is put back.
- Know about the editor, the window or the GPU.
- Hang or panic on a damaged area (a parent loop, a missing parent).

## Depends on

`area`, `reflect`, glam and hecs.

## Status

Phase 3: built from an area, stepped, read by ID. Game code joins the ticks in Phase 4.
