# 0079. Tile maps reached through their own Lust files

- **Status:** Accepted (carries out 0065's rule 4 for tile maps; builds on 0067 and 0078)
- **Date:** 2026-11-05

## Problem

Game code couldn't read or change a tile map. The farming game needs it to: tilling, watering, planting and growing are cells' terrains and values. The API guidelines (decision 0065, rule 4) say anything made in an editor reaches code through a file the editor writes.

## Decision

- **Each tile map file gets `solis/maps/<path>.lst`** (`script::map_code`, from `SpriteSheets::map_names`), written again whenever the map or its tileset changes; one no longer wanted is taken away. It holds states of the map's layers (`FarmLayer`), its tileset's terrains (`FarmTerrain`, `Empty` first) and each choice value's options (`FarmCrop`, `None` first), and `class FarmMap` with `width()`, `height()`, `terrain(layer, x, y)`, `setTerrain(layer, x, y, to)` and, per value, a reader and a setter (`growth`/`setGrowth`, `watered`/`setWatered`, `crop`/`setCrop`).
- **A component holds it as a field Solis fills in** (`FarmMap farm;`), as for animators (0078).
- **The class's bodies call the engine's `Maps` object** with the map's path and places by number (`Maps:setSwitch("maps/farm", 1, x, y, on)`). The map is the first entity, by ID, showing that file.
- **Reading is the map as the tick started** (`WorldView::maps`, made again only when a map changed: `World::maps_revision`), and changes land at the end of the tick (`Change::MapTile`, `Change::MapValue`) on the world's map, so they're in the checksum and saves, and solid terrains become walls at once. A cell off the map reads as empty and changing it does nothing.
- Names that can't be Lust names, or a value whose methods the class has already (`width`), are left out with a note in the Console.

## Consequences

- The farming game's crops can be written now.
- Turning a place in the world into a cell (which cell the player stands on or faces) waits for the 2D camera's milestone, where world and pixel places meet.
- `script` depends on `tiles` (both layer 1) to read maps.
