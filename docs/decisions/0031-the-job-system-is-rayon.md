# 0031. The job system is rayon

- **Status:** Accepted
- **Date:** 2026-10-24

## Problem

Until Phase 5, the world, game code and drawing all run on the frame's thread, so a PC with eight cores uses one. Reaching the stress level (decision 0030) needs the work spread over every core.

## Options

1. **`rayon`:** the standard Rust library for this (work stealing, parallel iterators, scoped jobs). MIT or Apache, widely used and tested.
2. **Our own job system:** full control over scheduling, but a large, subtle piece of code to write and keep correct.
3. **A game-engine scheduler** (Bevy's tasks, say): built for one engine's shape.

## Decision

Option 1. One pool for the whole program, sized to the cores and leaving one for the window. Work is split into jobs that write only their own slice; anything that adds results together does it in a fixed order, so a frame gives the same result on one core or sixteen. A panic in a job is caught where it's joined and reported.

## Consequences

- The first jobs (Phase 5): gathering and culling instances, placing entities in the world, moving projectiles, and Rust components that touch only their own entity.
- Lust and Lua stay on one core while editing (Luau is single-threaded); built games run Lust as Rust, which can be spread out later.
- Phase 6's determinism builds on "same result on any number of cores".
