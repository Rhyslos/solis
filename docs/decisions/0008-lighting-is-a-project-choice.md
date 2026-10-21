# 0008. Lighting is a choice per project; real-time with sky light comes first

- **Status:** Accepted
- **Date:** 2026-10-21

## Problem

DESIGN.md §7 left the lighting approach open: baked lightmaps, light probes, or real-time global illumination. It shapes the renderer, the asset pipeline and the editor, so it had to be settled before Phase 1. Each suits different games, and Solis aims at several kinds (§1).

## Options

1. **Real-time lights with sky light.** The sun and lamps are lit live with shadow maps; soft ambient light comes from the sky. No baking, works with day and night, cheap enough for hundreds of units. Indirect light is approximate.
2. **Baked lightmaps.** The best-looking still scenes, but every level must be baked (slow), models need a second UV set, and moving lights and day/night are hard.
3. **Real-time global illumination.** Light bounces live. Looks best when things move, but heavy on the GPU and a large piece of work.

## Decision

**Each project chooses its lighting.** The renderer keeps direct light (sun, lamps, shadows) separate from indirect light (ambient and bounce), and the indirect part is a swappable source, so other approaches can be added without rewriting materials or the rest of the renderer.

**Option 1 is built first,** in Phase 1, and is the default. Baked lightmaps and light probes, and later real-time global illumination, are added as further sources after 0.1 (or earlier if a game needs them).

## Consequences

- Materials take indirect light from one place, whatever produces it.
- Phase 1 needs no bake step, no second UV set and no lightmap storage.
- Scenes look flatter indoors until probes or baking arrive.
- A project setting for the lighting approach appears when there is a second approach to choose.
