# 0082. The 2D follow camera, and cells at places

- **Status:** Accepted (the 2D camera has no keys: the project lead's choice; builds on 0062, 0065 and 0079)
- **Date:** 2026-11-05

## Problem

A 2D game needs a camera that follows the player calmly and stays inside the map ("usable first", step 6), and game code needs to know which cell of a tile map a place is on (the farmer's hoe, a sprinkler's reach). Decision 0079 left the second for this milestone, where world and pixel places meet.

## Decision

- **The kit's `FollowCamera2D`** (`solis/kit/cameras.lst`, Lust like the other ready-made cameras, so it can be read and copied): a **target**, a **dead zone** the target can move in before the camera follows, **smoothing** as a critically damped spring (it eases in and never goes past), and **bounds**: an entity showing a tile map, whose box it never shows past (a map smaller than the view stays in the middle). It keeps its height. No keys or mouse actions, as the project lead chose. Snapping to whole pixels stays the renderer's job (0062), so the camera itself moves smoothly.
- **Cells at places, on the map's own class** (0079's file): `farm:cellAt(place)` gives a `MapCell` (`inside`, `x`, `y`: one class for the answers, by the API guidelines), and `farm:placeOf(x, y)` the place at a cell's middle. They call the engine's `Maps:cellAt` and `Maps:placeOf` with the map's path. Cells count from the map's top left, x across and y down, wherever its entity is (moved or turned); a place off the map still gets its cell, with `inside` false.
- **An entity's map box:** `entity:mapBounds()` gives a `MapBounds` (`found`, `low`, `high`): what the camera's bounds read.
- **The 2D view's size:** `World:viewSize()`, in metres (the project's 2D settings: base resolution over pixels per metre).
- All of it reads the world as the tick started (`WorldView::map_places`, `map_bounds`, `view_size`); the editor works out where each map lies each tick (`MapPlace`), since game code may move a map's entity.

## Consequences

- A farm game's camera and hoe can be written now; the manual has both.
- Screen shake, room transitions and the other feel tools of the 2D plan's D8 come later, when a game needs them.
