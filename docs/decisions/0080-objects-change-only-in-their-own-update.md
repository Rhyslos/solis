# 0080. Game-wide objects change only in their own update

- **Status:** Accepted (the project lead chose the safer option, 2026-11-05; builds on 0033 and 0038)
- **Date:** 2026-11-05

## Problem

Some of a game's state belongs to no entity: the time of day, the money, the weather. Lust has `object` for this, and its engine section says the engine ticks and saves a passed object. Solis ran neither: objects never ticked, and saves lost them.

Solis runs game code on several cores at once (decision 0033), each core with a Luau of its own and its own copy of the code. An object that any component could change would differ from core to core: a component on core 2 adding money wouldn't be seen on core 1, and the game would come out differently on a computer with more cores.

## Options

1. **Objects change anywhere, and changes are merged** at the end of the tick. Two components adding money in the same tick need a rule for merging ("last one wins" loses money), and every read mid-tick would see only its own core's changes.
2. **Objects change only in their own update** (and other objects'); components read them. Simple, the same on any number of cores, and loud when broken. Telling an object something from a component (selling a crop) needs another way, added later.
3. **All game code on one core** when there are objects. Correct, but gives up the scale target (DESIGN.md §2) for any game with a clock.

## Decision

Option 2, which the project lead chose as the easier and safer start, to improve later if needed.

- **Ticked first, on the first shard:** each passed object's `update(float dt)` runs once a tick, objects in name order, before any component (`Job::Objects`, with its own phase number for spawns, sharing the tick's time budget). Then its fields are copied to every other shard (`Job::SetObjects`), so every component reads the same values.
- **Changed only there:** when the Lust starts, each object's fields move into a store of their own and the object gets a metatable that reads the store and writes it only while the objects' updates run (`script::objects`). A write anywhere else is a Lua error at that line: the component stops, with "Clock.hour can't be changed here: an object changes only in its own update…". The metatable keeps the class's metamethods and `__parent`, so `like` still knows the object.
- **What's kept and copied:** fields of a component's kinds. Lists, maps and things stay as each core has them and aren't saved; the Console names them at Play.
- **Kept in saves** (`solis save 6`, an `objects` section: area text of one entity whose components are the objects, written without a registry), given back on a load by name and kind, with a warning for an object or field the code no longer has. A save from version 5 gets an empty section.
- **Kept across new code** by name and kind, with a note for a field whose kind changed.
- **In the checksum** once (the first shard's), so replays check them.
- An object with no fields and no `update` (a helper, or one of the engine's own, such as `World`) is left alone.

## Consequences

- A game can have a clock, a calendar and weather that every entity reads alike on any number of cores.
- A component can't yet tell an object something (sell a crop, add money). A later milestone adds a way, such as events sent to the game that objects handle at the start of the next tick.
- Lists and maps in objects aren't saved or shared yet; that needs Lust's schemas to describe them.
- Objects are Lust only: Lua and Rust components can't read them yet.
