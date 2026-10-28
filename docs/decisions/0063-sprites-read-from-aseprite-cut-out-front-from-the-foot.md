# 0063. Sprites are read from Aseprite files, cut out, with front and behind from where they stand

- **Status:** Accepted (builds on 0054 and 0062)
- **Date:** 2026-10-28

## Problem

D2 brings sprites: pictures drawn in Aseprite, placed in a 2D world in their tens of thousands (the plan's budget is 100,000). It had to settle how Solis gets the pictures out of Aseprite, how a frame reaches the graphics card, how front and behind are decided without sorting every sprite every frame, and how pixel art keeps its exact colours in a renderer that lights and tone maps everything.

## Options

- **Reading the pictures:** ask the user to export a sheet (PNG and JSON) from Aseprite, run Aseprite's command line, use a library crate, or read `.aseprite` files with our own reader.
- **Front and behind:** sort every sprite by its foot each frame (and draw them blended), or draw them cut out, writing depth, with depth worked out from where each stands.
- **Colours:** draw sprites through lighting and tone mapping like models, or show them exactly as painted.

## Decision

- **Solis reads `.aseprite` files itself** (`assets::aseprite`): the format is documented and small, there's nothing to export or forget to export, and saving in Aseprite updates Solis at once. The reader treats every file as hostile (limits on size, sides, frames, layers and pixels; fuzzed), and its only new dependency is `flate2` (pure Rust) for compressed cels. Our own reader also lets every limit and message be ours, in plain words. Visible layers are flattened with their opacity; other blend modes are drawn as Normal, with a note.
- **Each file is one atlas**, packed when it's read: frames trimmed to their drawn pixels, identical frames kept once, each frame's edge pixels repeated one pixel out and a pixel's gap round it, at most 4096 × 4096.
- **Sprites are cut out:** each pixel is drawn or not (alpha of a half or more), so they write depth and are drawn in any order, a run per sheet in one call. **Front and behind come from where each stands:** the shader draws a sprite a hair nearer the lower its foot is on screen (`FOOT_DEPTH`, under a metre of height across the view), so a whole metre of height always wins. Only sprites marked **See-through** are blended, drawn last and sorted back to front.
- **Colours are drawn exactly as painted:** never lit, shaded or tone mapped (sprites write alpha 0, which the last pass shows untouched). Lamps and day and night on sprites come in D9, on purpose.
- **A placed sprite stands as drawn** (not flipped; chosen by the project lead), on a whole pixel, on its sheet's *pivot* or *feet* slice.
- **Picking uses the sprite's own pixels,** so the clear space round a sprite selects what's behind.

## Consequences

- Animation (D3) changes a Sprite's frame; tags, durations and events are already read.
- Keeping layers separate (clothes and tools) and palette swaps need more per sprite: they come with the parts that use them, and the 88-byte instance has room to grow.
- A sheet must fit one 4096 atlas; a bigger one isn't read (Properties says why), and the user splits it.
- Lighting sprites (D9) will change the "as painted" rule for sprites that ask for it, not for all.
