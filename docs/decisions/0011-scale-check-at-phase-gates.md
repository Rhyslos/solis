# 0011. A scale check at every phase gate

- **Status:** Accepted
- **Date:** 2026-10-21

## Problem

Solis has a scale target (DESIGN.md §2): Empire at War-style battles, 2,000 units and 20,000 projectiles at 60 frames a second. Slowdowns creep in a little at a time, and are cheapest to fix when they are new. Aventail guards against the same thing with its heavy-mesh check (Aventail decision 0033). Solis needs its own guard, from the start.

## Options

1. **Timings in every pull request.** Catches a slowdown at once, but timings on CI machines are noisy, and decision 0001 keeps CI small.
2. **A scale check at the end of every phase, as part of the gate, and whenever the project lead asks.** Ordinary pull requests keep their tests and benchmarks for code on a performance path.

## Decision

Option 2, as in Aventail.

`tools/scale/scale-check.sh` builds in release mode and prints a table of timings against targets, for the pull request. Steps that miss their target are marked SLOW and the check fails.

What it times grows with the engine:

- **Now:** one frame of the editor's interface with a full console (target: 4 ms), and one viewport frame at 1920×1080 (reported only, since it depends on the graphics card).
- **From Phase 1:** models and lights in the viewport.
- **From Phase 4:** the scale target itself, as a scene the check builds and runs.

## Consequences

- Each phase's gate pull request carries the scale check's table.
- The first run found the console laying out every message each frame, about 10 ms with a full console. The console now lays out only the rows in view (0.3 ms).
