# 0035. Game code spawns into the area while playing

- **Status:** Accepted
- **Date:** 2026-10-25

## Problem

The benchmark's ceiling run (Phase 5, M7) adds ships while it plays, so game code needs to make entities (`spawn`) and take them away (`despawn`), brought forward from Phase 6. Copies of assemblies are made in the area document today, and the game's world is built from the area. Where do entities game code makes live?

## Options

1. **In the world only.** The world grows entities of its own. But the world is rebuilt from the area whenever the area changes (an entity added in the editor, an undo), so it would have to remember and rebuild what game code made, and the Area panel and Properties wouldn't show them.
2. **In the area, as the editor's own changes are,** but not as undo steps: Stop puts the area back as it was before Play anyway.

## Decision

Option 2. `spawn(file, place)` makes a copy of an assembly (named by its file in the assets, that without the extension, or its name) where asked; `despawn(entity)` takes one out with everything under it. Like a move, they land after the tick's code (decision 0033): the new entity's ID comes back at once, made from where the spawn was in the tick's order and the time, so the same game makes the same IDs on any number of cores; spawns and despawns land at the end of the tick in the order asked for, joining the area (at the end of its tree) and the game's world at once, so the entity's components run from the next tick, even within one frame. The editor adds them to the area with `Document::edit_by_game`, which changes the area without an undo step, and to the world with `World::add_subtree` and `World::remove_subtrees`, without building the world again (the Phase 5 review moved this from the end of the frame).

## Consequences

- What game code makes shows in the Area panel and Properties while playing, and goes away on Stop.
- Undoing an editor step while playing still works, unless it touches an entity game code took out (that step can't be undone, and says so).
- Spawning many entities rebuilds the world from the area that frame. Fine for adding fleets in steps; Phase 6, when games spawn every tick (bullets that are entities, effects), may need a quicker path.
- Every assembly in a project is read when the project opens, so any can be spawned.
