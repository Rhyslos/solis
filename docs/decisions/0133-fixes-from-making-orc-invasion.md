# 0133: Fixes from making Orc Invasion, before dialogue

- **Status:** Accepted (the project lead, 2026-11-20)
- **Date:** 2026-11-20

## Problem

A session made Orc Invasion, a small top-down action RPG, with Solis and Lust, and wrote down 27 hurdles ([the report](../reports/orc-invasion-hurdles.md)):
- 5 bugs;
- 8 missing game-code calls;
- 4 rough edges in Lust;
- 5 gaps in the documentation;
- 5 in the tools and testing.

Two of them break a game that passes the project check (screens failing on Lua's 200-locals limit, and a check that never starts the game). Others made the game's code much longer:
- 300 lines of clip lookups;
- events for every question to another entity;
- string names for one-off entities.

The project lead asked for fixing every one of them to be in the plan.

## Options

1. All four milestones now, before dialogue.
2. Only the bugs now, and the rest after dialogue.
3. Everything after dialogue.

## Decision

**All of it now, before dialogue** (the project lead's answer). Four milestones fix Solis's 23 hurdles ([the plan](../plans/game-stages.md#fixes-from-making-orc-invasion)):
1. bugs, and a project check that starts the game;
2. the missing game-code calls (reading another entity's component, reaching a one-off entity without a string, spawning with values, `exists`, turning a collider off, hiding an entity);
3. clips by action and direction, and a character's other look;
4. Solis's files written down, and playing a game from the command line.

Lust's four go on the requests for Lust.

## Consequences

- Stage 1 gains four milestones before dialogue, and dialogue starts after them.
- Dialogue can then use reading components and entities reached without strings.
- `World:find("…")` with a string name is retired from the manual once a checked way exists.
- Each milestone is planned in detail when it's next, with UI mocked up first and choices asked, as usual.
