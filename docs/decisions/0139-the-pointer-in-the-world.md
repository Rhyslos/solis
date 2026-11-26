# 0139: The pointer in the world, shown when game code asks

- **Status:** Accepted (the project lead, 2026-11-26)
- **Date:** 2026-11-26

## Problem

The action game aims with the mouse (the scope reset's action platformer, A1). Game code could read only the mouse's movement, and while a game has the mouse Solis hides the pointer and holds it in the window, so moving the mouse turns 3D cameras. A 2D shooter needs the pointer seen and free, and the place in the world under it.

## Options

How a game chooses a pointer that's shown and free:
- **Game code sets it** with a field, like the other settings.
- A project setting in Project settings › Input, fixed for the whole game.
- Both: the setting starts it, game code changes it.

## Decision

The project lead, 2026-11-26: game code sets it.
- `Input.pointerShown = true;` shows the pointer and leaves it free while the game has the mouse; `false` hides and holds it again. It starts false, so 3D games are unchanged, and lands at the end of the tick.
- `vec3 Input:pointer()` is the place in the world under the pointer, over a 2D game's picture, as the tick started: the camera looked through's place plus the pointer's share of the view the project's 2D settings give.
- Where the pointer is goes into each tick's input as a fraction of the game's picture (kept while it's away), so it's in the tick's checksum and in recordings, which are version 6 (a `p` line when it moves).

## Consequences

- Replays aim the same; a recording from before version 6 reads as before (no pointer).
- In a 3D game `Input:pointer()` isn't where the pointer points: it's the 2D answer at the camera's height. A ray from the camera through the pointer waits until a 3D game asks.
- Whether the pointer is shown isn't saved: game code sets it again as it starts.
