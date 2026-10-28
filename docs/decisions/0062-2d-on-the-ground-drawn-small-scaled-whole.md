# 0062. 2D lies on the ground, drawn small and scaled up in whole steps

- **Status:** Accepted (builds on 0054)
- **Date:** 2026-10-28

## Problem

Decision 0054 puts 2D games in the same world as 3D, seen by a camera with no perspective, and the 2D plan (`docs/plans/2d.md`) asks for pixel art that never jitters or blurs. D1, the first 2D milestone, had to settle where flat things lie, how a game says it's 2D, where a project's 2D settings live, and how the small picture reaches the screen.

## Options

- **Which plane:** the ground (X and Y, seen from above), or upright (X and Z, seen from the front).
- **How a game is 2D:** a project-wide switch, or each camera saying how it sees.
- **Where the settings live:** in `project.solis` (a new version of its format, which older builds would refuse), or in a file of their own.
- **Scaling up:** the renderer drawing at screen size with a pixel-sized camera, or drawing small and scaling up.

## Decision

Chosen by the project lead from a mock-up drawn into a screenshot of the editor, with 480 × 270 and 16-pixel tiles as the start, and no new controls:

- **2D lies on the ground,** X right and Y up the screen, seen from straight above; height (Z) is the layer. The editor's Top view, the ground grid, the sun shining down and Y-sorting (D2) all already work that way.
- **A camera says how it sees:** the Camera component's new **View** is Perspective or **2D**. A 2D camera looks straight down from its entity's place (it never turns) at the project's base resolution. A project can have both; Entity → Add camera in the 2D view adds a 2D one.
- **The project's 2D settings** (base resolution, pixels per unit, tile size, scaling) are in their own file, `pixels.solpix`, a settings file in the Lust style (format 1, with an example in `tests/fixtures/pixels/`), shown on a **This project → 2D** page of Edit → Settings… (as decision 0061 foresaw). `project.solis` stays at format 1, so older builds still open a project with 2D settings.
- **The pixel stage:** the world is drawn at the base resolution plus a pixel each side, with the camera on whole pixels, then scaled up by a whole number of times with bars around it (**Whole steps**), or to fill the screen with only each pixel's last screen pixel blended (**Fill**). The bit of a pixel the camera has really moved slides the finished picture, in whole screen pixels, using the margin.
- **The 2D view** is a fifth kind of view: flat like Top, zooming in whole steps (25% to 2400%), with the world's pixels on whole screen pixels, the tile grid, moves snapping to whole pixels or tiles, X and Y only, and a dashed frame for what each 2D camera shows.

## Consequences

- Sprites (D2) are quads lying on the ground; their height orders them, and depth from Y (D2) adjusts it.
- D8 (the 2D camera's feel: following, dead zones, shake) adds settings to the 2D camera; shake and smoothing never move the game's own camera, only what's drawn.
- Text and menus drawn at full resolution come with game UI (Phase 12); the warning for a sprite resting between pixels comes with sprites (D2).
- The pixel stage's 0.2 ms budget at 4K is measured by the scale check on the reference card; software drivers only check it's right.
