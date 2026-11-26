# 0138: Tiny first, and the done list's games in order of cost

- **Status:** Accepted (the project lead, 2026-11-26)
- **Date:** 2026-11-26

## Problem

The game stages ([decision 0122](0122-the-game-stages.md)) held about 170 pull requests of 2D work, with dialogue next and in full. The project lead weighed whether the project was worth going on with. With the done list written ([the scope reset](../plans/scope-reset.md#done-means)): a farming game like Stardew Valley, a platformer and a 2D combat game. Most of that work turned out not to be needed for those three games.

## Options

- Keep the stages' order, with dialogue next and in full, then the rest of stage 1 (the farming game in about 80 pull requests, the platformer after it).
- **Build the smallest version of each feature the three games need, and make them possible cheapest first.**

## Decision

The project lead, 2026-11-26:
- **Tiny first.** Each feature is built in the smallest version that makes the three games possible, and grows later when a game needs more. Anything game code can do reasonably is a Lust recipe in the manual, not an engine feature.
- **Dialogue is the exception: it's built in full** ([the outline](../plans/dialogue/README.md)), as the one part worth going all out on.
- **Cheapest game first,** so a game can be made as soon as possible instead of waiting for the most expensive one ("it makes more sense to let me write platformers today, and Stardew Valley in 2 weeks"):
  1. the "Can I make this?" check (the manual page);
  2. the platformer;
  3. the action platformer;
  4. the farming game's small pieces;
  5. dialogue, in full.

The list of tiny versions and their order is in [the scope reset](../plans/scope-reset.md#the-target-tiny-versions-cheapest-game-first).

## Consequences

- About 25 pull requests of tiny features and about 35 for dialogue: the three games are possible in about 60, against about 170 before.
- The platformer is possible after about 3 pull requests and the action platformer after about 3 more, before dialogue starts.
- The game stages' later work (puzzle tools, the renderer rework, mods, 3D) waits until a game asks for it. [The game stages](../plans/game-stages.md) stay the catalogue.
- The estimates rest on parts believed to exist (knockback, a ground check, loading a level from game code). The "Can I make this?" check confirms them, and its "not yet" rows become the milestones.
