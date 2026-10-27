# 0054. First-class 2D, with game UI and a playtest export right after it

- **Status:** Accepted
- **Date:** 2026-10-27

## Problem

DESIGN.md §1 said "3D only: the only 2D is UI", so that Solis stays focused. The project lead is restarting their own 2D game, a top-down farming RPG in pixel art with frame-by-frame sprite-sheet animation, and wants to make it in Solis, in Lust. They asked for 2D that is **great, not half done**.

A game like that also needs:
- real menus (inventory, shop, dialogue);
- a way to send builds to playtesters.

In the roadmap, both of those came much later.

## Options

**How 2D is built:**
1. **A separate 2D engine** inside Solis, with its own lighting, effects and physics. Two ways of working, and a 2D lighting model that doesn't match the 3D one: the split that troubles engines that did it.
2. **2D inside the same world:** sprites and tiles are flat things seen by a flat camera, so lamps, the sky, effects, materials, sound, saving and netcode work on 2D too. Real 2D tools on top: a pixel-perfect view, a sprite and tile renderer, tile maps, a painter, and 2D physics.

**2D physics:**
- Rapier2D: the 2D twin of the 3D physics Solis uses, with the same determinism mode.
- Box2D v3: a C library with a strong determinism story.
- Only simple tile collision.

**Autotiling:** one default kind of tileset, or each tileset choosing its own.

## Decision

Chosen by the project lead:

- **Option 2: 2D inside the same world, with real 2D tools on top.** DESIGN.md §1 now reads "3D, and first-class 2D".
- **In two parts, both finished:**
  - Phase 9 is what a 2D game needs to move in;
  - Phase 10 completes 2D.

  Each milestone must pass the bar in the plan before it counts:
  - a speed budget with a scale case;
  - the old tools' known problems, each a test;
  - fuzz tests for every file read;
  - a manual page;
  - the project lead's own round of use.
- **2D physics is Rapier2D,** walled into the physics crate beside the 3D physics.
- **Each tileset chooses its own kind** (16-tile, 47-tile, edge sets or custom rules), set in the tileset wizard. There's no project-wide default. Maps store meaning, so a tileset's kind can change without repainting.
- **Bone (skeletal) 2D animation waits** until after 0.1. The project lead's game uses frames.
- **A playtest export follows 2D** (Phase 11): a game program without the editor, with Lust and Lua run as in Play, for sending to playtesters. The full export, with the game's own Rust built in, stays with Git in Phase 17.
- **Game UI moves up to right after the export** (Phase 12), so a 2D game gets real menus sooner.
- **The effects suite gets a pixel mode** (Phase 15): pixel-shaped particles snapped to the grid, the game's palette, and a ready-made set (wind streaks, fireflies, bubbles and more).
- **The light phase brings 2D bounced light** (radiance cascades, which were made for 2D and are practical there), and the real sky and time of day over 2D worlds.

The order becomes:

| Phase | Content |
| --- | --- |
| 7 | Sound and animation (its state machine drives sprite frames too) |
| 8 | Compatibility (decision 0053) |
| 9 | 2D, part 1 |
| 10 | 2D, part 2 |
| 11 | Playtest export |
| 12 | Game UI |
| 13 | Light, sky and atmosphere |
| 14 | Materials and shaders |
| 15 | Effects |
| 16 | Netcode |
| 17 | Scripting, full export and Git |
| 18 | Solis 0.1 |

The plan, with the research behind it, is [docs/plans/2d.md](../plans/2d.md).

## Consequences

- **2D is the largest addition so far:** about 40,000 lines across its two parts, roughly half of Solis's size today. Its pace is set by the project lead's rounds of use, because "great" comes from use.
- **About a tenth of today's code changes.** Most of 2D is new:
  - the renderer gains a pixel stage;
  - the editor gains a 2D mode;
  - the physics crate gains a 2D world;
  - new asset kinds and components are added.

  The 3D side keeps working, guarded by its tests.
- **Plans for phases not started are named by subject** (`light.md`, `materials.md`, `effects.md`, `game-ui.md`, `2d.md`, `compatibility.md`), since the order can still change.
- **The Phase 7 animation plan** builds its state machine to drive any clip with a length, events and frames, not only 3D skeletons.
