# 0098: The ideas become milestones, and the editor's UI is remade first

- **Status:** Accepted
- **Date:** 2026-11-10

## Problem

The ideas list (docs/plans/ideas.md) held 34 tools that help make games, written down on 8 November with no place in the plan. The project lead wants them in the engine, and wants the editor's interface rebuilt from scratch.

## Decision

The project lead's order (2026-11-10):

- **The editor's UI, remade** is the first phase (Shared 1), mocked up first and planned in detail when it starts.
- **Every idea becomes a milestone,** placed by what it needs and who uses it, built once for 2D and 3D (decision 0097):
  - Shared 2, game code: tile map questions, tags and world questions, spawning with values, game tests (the rest of the playtest fixes), and a note for code files nothing fetches.
  - Shared 3, time and feel: timers, tween, steps in order, states that know when they start and stop, shake, flash, fade and rumble, smooth randomness and weighted random.
  - Shared 4, data and the editor: data tables, save slots, field hints, more kinds of settings, components that draw in the editor, paths.
  - Shared 5, screens, words and sound: text effects, button pictures, tooltips and drag and drop, translations and subtitles, music that follows the game.
  - Shared 6, worlds and testing: rooms, pools, debug drawing and a cheat box, bug reports with recordings.
  - Shared 10, after effects: a cutscene timeline, and behaviour for characters drawn as diagrams.
  - Shared 11 gains mods in LustScript; 3D 1 gains navigation (navmeshes).
- **Not planned:** photo mode and screenshots, and store features built in (Steam's achievements, cloud saves, Workshop, and its networking as part of netcode). Those are for each game's developer, not the engine.
- The shared phases after Shared 2 are renumbered: materials and shaders are now Shared 7, effects 8, weather and the finished picture 9, scripting, export, Git and mods 11, netcode 12, Solis 0.1 13.

## Consequences

- 31 milestones join the plan: 180 in all, 92 built.
- 2D stays first among the tracks; the idea tools follow it, so games gain them before the rethink and the big testing pass.
- The ideas list stays as the record of what each idea means, each marked with its phase.
