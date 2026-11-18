# 0120: The order after the rethink: open worlds join the 3D track, time and feel before 2D 2

- **Status:** Accepted (the project lead, 2026-11-18)
- **Date:** 2026-11-18

## Problem

The project lead asked for research on everything left after the editor rethink, and a detailed plan from it ([the plan](../plans/after-the-rethink.md), [the research](../reports/after-the-rethink/README.md)). The research found problems in ROADMAP.md's order:
- Open worlds, five large 3D-only milestones, came straight after Tools 1b, ahead of all the remaining 2D, against decision 0097's "2D first". Its far ground only looks right once 3D 2 and 3D 3 bring haze over distance and far shadows.
- 2D 2's polish needed Shared 3's timers and tweens.
- 2D 1's typed fields were mostly Shared 4's work.
- Nothing covered foliage, feet on slopes, or a check that a game replays the same on two PCs.
- The renderer needed restructuring before materials and light pile on.

## Options

Each was asked as a question with a recommended answer and alternatives. The alternatives were:
- Open worlds straight after Tools 1b;
- small versions of hit-stop and shake in 2D 2;
- field hints pulled into 2D 1;
- the renderer restructured at 3D 2, or deferred shading;
- the determinism check later, before netcode;
- foliage only when asked;
- foot IK later;
- every world baked fully into files.

## Decision

- **Open worlds move into the 3D track,** after 3D 1 and before or alongside 3D 2 and 3D 3. Tools 1b stays next.
- **Shared 3 (time and feel) comes before 2D 2.**
- **2D 1's entities with typed fields move into Shared 4.**
- **The renderer is restructured at the start of Shared 7:**
  - shader modules;
  - a small frame graph of passes;
  - a depth-and-normals pass first, with many lamps per pixel ("forward+");
  - MSAA kept.
- **Shared 2 ends with a check that a recorded game replays the same on two PCs,** checksums compared tick by tick.
- **A milestone for foliage and scattering** follows Open worlds.
- **3D 1 gains foot IK,** so feet sit on sculpted slopes.
- **Open worlds' recipe:**
  - **by default:** it is local (erosion-like noise and a height-map picture, no simulated erosion), its versions are kept forever, and ground the game reads is made exactly on the processor;
  - **as a world setting:** the whole world is baked into files instead, for worlds that must never depend on the recipe.
- **Older projects keep their `assets` folder** as their files folder. Solis doesn't offer to move their files out.
- The code review at the end of the editor rethink waits until the project lead asks for it.

## Consequences

- ROADMAP.md's order and sections change: Open worlds and Foliage after 3D 1, Shared 3 before 2D 2, and the new lines in Shared 2, 2D 1, Shared 4, Shared 7 and 3D 1.
- The plan after the rethink is no longer a draft. Its other questions are asked when each phase starts.
- The 2D track's gate (2D complete) now comes after Shared 3, a phase later than before.
