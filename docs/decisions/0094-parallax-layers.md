# 0094. Parallax layers: a Parallax component on a sprite, drawn behind or in front, each layer sliding smoothly on its own

- **Status:** Accepted
- **Date:** 2026-11-08

## Problem

Phase 10's polish milestone plans "parallax backgrounds, each layer with its own scroll speed and its own sub-pixel slide" (docs/plans/2d.md, D15). The project lead is making a 2D platformer, where backgrounds that move slower than the world (far hills, clouds) and foregrounds that move faster (leaves, posts) are what make a flat level feel deep. They asked for it now, in full: both the simple version and the smooth one.

Two questions: how a layer is set up, and how it's drawn without breaking the pixel-perfect look (decision 0062).

## Options

**Setting a layer up:**

1. **A component on a sprite** (Godot's Parallax2D, Unity's usual scripts): any sprite becomes a layer, so its picture, tint, lamps and clips keep working.
2. A layer object holding several pictures (Godot's older ParallaxBackground): one more kind of thing to learn, and pictures that aren't sprites.

**Drawing it smoothly:** a 2D game is drawn small, on whole pixels, then scaled up; the camera's bit of a pixel slides the finished picture (decision 0062). A layer moving at half the camera's speed has a bit of a pixel of its own, so it can't share that slide.

1. **Each layer drawn into its own small picture** and slid by its own bit of a pixel as the pictures are scaled up and laid on each other, far to near.
2. Drawing layers straight onto the screen at full size: their pixels could then be put anywhere, but they would bypass the lamps, day and night and the small picture's crispness.
3. Whole pixels only: simple, but a slow layer steps a whole pixel at a time, which shows in a slow pan.

## Decision

The project lead asked for both steps (2026-11-08); the choices above marked 1.

- **`Parallax`, a built-in component,** turns the entity's Sprite into a layer:
  - **Scroll across** and **Scroll up**: how far it moves with the camera. 0 stays still on the screen (the sky), 0.5 moves half as far (far hills), 1 moves with the world, and above 1 faster (a foreground). From 0 to 4; 0.5 to start.
  - **Repeat across** and **Repeat up**: drawn again and again side by side (or above and below), at its sheet's size, so it never runs out. Across on to start.
  - **In front**: drawn over everything instead of behind it.
- **Where it's drawn:** exactly where it's placed while the game's camera is where it starts (the first camera in the area, when Play is pressed). As the camera moves, the whole layer goes along by `(1 − scroll) × how far the camera moved`, across and up separately, so on the screen it moves `scroll` times as far as the world. So the editor shows the game's first picture, and a layer made of several sprites (a row of trees) keeps their spacing. (Anchoring each sprite at its own place was tried first: it squeezed a layer's sprites together.)
- **In what order:** layers behind are drawn before everything else (tile maps included), the slowest first; layers in front after everything, the slowest first. Layers with the same speeds go lowest first, then by entity.
- **In the editor:** drawn where placed (the editor's camera isn't the game's), repeated across the view, behind or in front as in the game. A click picks a layer behind only where nothing else is.
- **In the game:** layers follow the camera the game looks through (or the editor's, when there is none, measured from the world's middle).
- **Smooth (step 2):** through a 2D camera, each layer (each different set of speeds, behind or in front) is drawn into a small picture of its own, and the pixel stage lays them on each other, each slid by its own bit of a pixel. Each layer is seen by a camera of its own, which has moved its scroll's share of the way from where the game's camera started, so its sprites stay on whole pixels where they were placed and its camera's bit of a pixel does the smooth sliding. The farthest layer holds the sky; the world's picture and nearer layers are clear where nothing is drawn. At most 8 such layers; with more, every layer is drawn on whole pixels in the world's picture, as in step one.
- **Not yet:** tile maps as layers, and changing a layer's speeds from game code. A Parallax is only how a sprite looks: a Collider2D on the same entity stays where the entity is.

## Consequences

- Any sprite can be a layer, with everything sprites do: clips, tint, lamps and day and night.
- Each smooth layer costs one more small picture a frame (480 × 270 is about half a megabyte): cheap, measured by a scale case.
- The world's picture is drawn with a clear background when layers lie behind it, so the pixel stage blends it over them (by its alpha, which the last pass writes for this). The picture's alpha usually says which pixels are tone mapped; in a layer's picture it says how covered each pixel is instead, so 3D models in such a picture are shown as drawn, not tone mapped. 2D games rarely have models; tone-mapped models among layers would be a change of its own.
