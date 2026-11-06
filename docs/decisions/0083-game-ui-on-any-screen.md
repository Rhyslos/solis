# 0083. Game UI on any screen, and pictures cut from sheets

- **Status:** Accepted (the project lead approved the mock-ups, 2026-11-05; builds on 0039 and the game UI plan's answers of 2026-10-25)
- **Date:** 2026-11-05

## Problem

Game UI must look right on every player's screen: Full HD, 1440p, 4K, ultrawide, a Steam Deck. The project lead asked how a UI designed on one screen fits the others, and asked for Godot's AtlasTexture: using one sprite, or a few, cut from a sprite sheet.

## Decision

The mock-ups (`ui3`, `ui4`, `ui5`, approved "Yeah looks great! Build it"):

- **Anchors:** every element is pinned to its parent at one of nine points (corners, edge middles, the middle), or stretched across, down or both, and keeps its offset from there. The UI editor's anchor picker is a 4 × 4 grid: left, middle, right and stretch, by top, middle, bottom and stretch. A stretched side's size is its gap at the far edge.
- **A scale rule per screen:**
  - **Grow with the screen** (the standard): sizes scale with the screen's height against the design's (1920 × 1080), so 1440p shows everything a third bigger and an ultrawide of the same height shows it the same size, with the extra width given to the game.
  - **Keep its size:** the same pixels everywhere.
  - **Whole steps** (recommended for pixel-art games, so the farm game's HUD): designed in the game's own pixels (the project's 2D base resolution, 480 × 270), scaled by the same whole number as the game's picture (×4 on 1080p), laid out inside the picture, with every edge on the game's pixel grid and drawn without smoothing.
- **A Screen list** in the UI editor previews common sizes (Full HD, QHD, 4K, HD, two ultrawides, Steam Deck, a 4:3 monitor) and a custom one (`ui::presets`). When playing, the game uses the player's real screen.
- **Pictures cut from sheets** (built in a later milestone; named **Atlas Crop** and kept in `tools.atlas.lst` by [decision 0089](0089-atlas-crop-and-the-ui-editor.md)): right-click a PNG or Aseprite file, **Cut out pictures…** opens it as a tab; drag boxes (snapped to the tile grid) and name them, or **Cut out every tile…**. They're kept in one Lust data file beside the picture (`tools.pictures.lst`, decision 0081), never changing the picture; anything that picks a picture lists them, and game code names them (`Picture.Sprites.Tools.Axe`).
- **The first milestone builds the layout and the shapes** (the game UI plan's M1, split): the `ui` crate (screens as element trees, anchors, the scale rules, layout into shapes, the preview sizes), and `render`'s UI layer: boxes with rounded corners, ellipses, lines and bars, each an instance whose edge is worked out from its distance in the shader, sharp at any size (or hard-edged for whole steps), drawn over the finished picture after tone mapping in one draw call. The Console's `ui test` shows a sample HUD over the Game panel until screens are made in Lust. Text is the next milestone.

## Consequences

- The plan's milestones stand, M1 split into M1a (layout and shapes, this) and M1b (text); cutting pictures from sheets is its own milestone, before the UI editor uses pictures.
- Game UI is drawn by Solis's own shader, as the plan's answer chose, so glow and pixelation can follow.
