# 0071. 2D physics and top-down movement: solid terrains, walls merged per chunk, a Collider2D, and a walker in the kit

- **Status:** Accepted (builds on 0036, 0054, 0062, 0067 and 0070)
- **Date:** 2026-11-04

## Problem

"Usable first" (decision 0070) puts 2D movement fifth: the project lead's farming game needs a farmer who walks the farm in 8 directions, NPCs in 4, and fences, water and buildings that stop them. Solis had tile maps, sprites and 3D physics, but nothing in 2D that blocks or slides. It's built ahead of the milestones that wait on Lust's changes, because it needs none of them (the project lead, 4 November).

Two lines had to be drawn. First, between the engine and the game: the engine gives tools any game can use, and the game decides what they mean (the project lead's question, 4 November). Second, the 2D plan ([docs/plans/2d.md](../plans/2d.md), D7) asked for Rapier2D beside the 3D world; 2D in Solis already lies on the ground of the 3D world (decision 0062), so reusing the 3D world was possible too.

## Options

- **2D in the 3D physics, flat.** Less new code, and 2D and 3D things could bump into each other; but Rapier's 3D character controller is made for floors and slopes, and a top-down game has neither.
- **A separate 2D world (Rapier2D), as the plan said.** A second world to build, check and keep the same on every PC, but made for the job, and faster for big 2D games.
- **Tile collision as one shape per tile.** Simple, but a character sliding along a fence catches on the seam between every two tiles, and a big map makes hundreds of thousands of shapes.

## Decision

- **A separate 2D world** (the project lead chose it): Rapier2D in its strict mode, in `physics::flat`, walled in like the 3D world. Places are Solis's X and Y on the ground; Z, the layer, is left alone.
- **Solid is the game's choice, per terrain.** A tileset's terrain has a `solid` setting (written `solid = true;` only when on, so older tilesets read as before). Right-click a terrain in the Map tab and tick **Solid**; a small wall mark shows on solid terrains' swatches. Called "Solid", not "Blocks walking": it stops anything with a body. A cell is solid if any of its layers is a solid terrain.
- **Walls merged per chunk.** Each map's solid cells become walls, merged into as few rectangles as a greedy sweep finds (`tiles::walls`), never leaving their 32 × 32 chunk; a changed cell builds only its own chunk's walls again. Never one shape per tile.
- **The mover is Solis's own:** a shape cast to what it meets, a skin (0.006 m) back out along the surface, then a slide along it, a few times over. Kept a skin away, a character passes the corners where two walls meet without touching, so a seam never catches it (tested over 1,000 seams). A circle slips round corners it clips.
- **`Collider2D`, in pixels:** a circle or a box with an offset (a character's feet), turned into metres with the project's 2D settings. On its own it's solid and still (a house, a bin); ticked **Character**, game code walks it with `moveCharacter`, which a 2D character takes on the ground. The viewport outlines it when selected.
- **The viewport's Walls switch** (beside Chunks) shows what's solid on the selected map, as its merged rectangles, in the collider green (mock-up u5, approved with the two changes above).
- **A walker in the kit:** `solis/kit/topdown.lst`'s `TopDownController`, open Lust any game reads, copies or replaces. Speed, acceleration and deceleration; 8 directions or 4 (a setting the game chooses, never "players 8, NPCs 4"); never faster diagonally; `usesInput` off and `wanted` set walks it from game code (an NPC). It needs no Lust changes.
- **The arrow keys walk too:** the ready-made Move action is WASD, the arrow keys and a controller's left stick (the project lead's choice). The arrows also still pan the RTS camera, as the stick already did both; a game uses one or the other.
- **In the checksum, not in saves:** 2D physics is in the world's checksum; a save keeps only where entities are, and 2D physics is built again from them and the maps, so a save's format doesn't change.
- **Triggers wait** for the API milestone (the project lead's choice): telling game code that something walked into a door needs game code to hear new events, which touches Lust.

## Consequences

- Game-specific rules stay in games: Solis never knows "water", "fence" or "farmer".
- `Collider2D` has no rotation and doesn't scale with its entity; 2D bodies that physics moves (a ball rolling across the floor) and moving platforms come with Phase 10's other movers.
- The 2D camera (the next milestone in this order that doesn't wait on Lust) can follow a `TopDownController`.
