# 0097: Three tracks: 2D first, shared once, 3D after

- **Status:** Accepted
- **Date:** 2026-11-10

## Problem

The next phase was light, sky and atmosphere: 20 milestones, almost all for 3D, standing between the project lead and finished 2D games. Phases had grown uneven (3 milestones to 20), which made progress hard to follow, and some systems were filed under 3D though 2D games need them too (bloom, grading, weather).

## Decision

The project lead's order (2026-11-10):

- **The rest of the plan runs in three tracks.** **2D** is finished first, so whole 2D games can be made. **Shared** holds what isn't purely 2D or 3D, built **once, for both**, so no system is made twice. **3D** follows.
- **The order:** 2D 1 and 2 (was Phase 10, with 2D bounced light from Phase 13); Shared 1 to 3 (materials and shaders, effects, then weather and the finished picture, both taken from Phase 13); the rethink and the big testing pass; Shared 4 (scripting, the full export and Git, so finished games can ship); 3D 1 (animation) and 3D 2 to 5 (light in four steps); Shared 5 (netcode) and 6 (Solis 0.1).
- **Big phases are split** into phases of two to six milestones. The milestones themselves are unchanged: 57 left, regrouped.
- Materials now come before light (decision 0048 kept materials between light and effects): materials take the light around them as an input, so they work with today's lights and gain the new ones as they arrive.
- The progress tracker's count is corrected to every milestone listed: 92 of 149, with Phase 9's two moved milestones counted as built.

## Consequences

- 2D is complete, effects included, before any of the 20 light milestones, which Phase 13 had put first.
- Each shared milestone is built and tested for both sprites and models.
- Phase numbers stop at 12; later phases are named by track (`2D 1`, `Shared 3`, `3D 2`). Older documents that say "Phase 13" to "Phase 19" mean the phases these came from, listed as "was" in ROADMAP.md.
- Decision 0059 still holds: steps after the rethink are written down, not planned in detail.
