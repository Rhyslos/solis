# 0108: Shape brushes and world sculpting come next

- **Status:** Accepted (the project lead, 2026-11-13; moves two tools ahead of [0097](0097-three-tracks.md)'s order and takes the sculpting part of terrain out of "Not in 0.1")
- **Date:** 2026-11-13

## Problem

Decision 0097 finishes 2D first, then 3D, and terrain was "Not in 0.1". On 13 November 2026, after the editor's new interface and the full review were done, the project lead asked for two 3D building tools: **shape brushes** (simple shapes made in the editor to build a level with, as level editors of the brush kind do) and a **world sculpting tool** (shaping the ground), both fast, and asked when they should come.

## Options

1. **After the layout fixes and the second pass's 27 problems, as the next phase** (recommended): the interface fixes and the found problems first, so the new tools are built on a sound base; then the tools, ahead of the rest of 2D.
2. **Right after the layout fixes**, before the 27 problems.
3. **Later, on the 3D track**, after 2D is complete.

## Decision

**Option 1.** The order is now:

1. The layouts as a setting, and every edge resizable (decision 0109).
2. The 27 problems of the second pass (`docs/reviews/2026-11-full-review-second-pass.md`).
3. **Tools 1: shape brushes and world sculpting**, then the rest of the plan as it was.

**The brush list's keys** (the project lead's answer, rule 17): **B** over the 3D or 2D view opens the list of brushes, and each further **B** moves one down it; **Space** confirms the one picked, but only while the list is open (otherwise Space still opens the search); **Esc** closes it. While a tile map is picked, B stays the tile map's Brush tool (decision 0069).

Both tools are mocked up and approved before they are built, have a plan of their own, and have scale cases, as every new tool does.

## Consequences

- Sculpting the ground is no longer "Not in 0.1"; open-world streaming still is.
- The rest of 2D (2D 1 and 2D 2) moves back by one phase.
