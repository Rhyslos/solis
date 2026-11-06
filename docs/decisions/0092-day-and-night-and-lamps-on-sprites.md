# 0092. Day and night, and lamps on sprites

- **Status:** Accepted
- **Date:** 2026-11-06

## Problem

The usable-first build order (decision 0070) puts day and night with lamps after game UI. The 2D plan's D9 asks for:

- sprites and tiles lit by lamps;
- light worked out at the game's resolution, so it stays on the pixel grid;
- a "light steps" setting for a banded look;
- a day-and-night tint that follows the time of day, which is game state, so a game's clock and sleeping work.

Four choices were open: where the clock lives, how game code reaches it, whether old 2D projects change, and how lamps know it's night.

## Options

For the clock:

1. **A setting on the Environment, with calls.** The area's Environment has a Day and night section, and game code reads and sets the time.
2. A clock written only in game code (a game-wide object), with Solis given a colour each tick.
3. A real-time clock, the same in every game.

For lighting sprites:

1. **A switch on the Environment**, on in new 2D projects and off in older ones.
2. Always on.
3. Per sprite.

## Decision

The project lead's choices (2026-11-06): **Environment setting + calls**, and **a switch, on for new 2D**. The mock-up (`runmock/a.png`) was approved, with one change asked for: more options for the light's colours. So the colours are a list of stops, not four fixed ones.

- **Day and night**, under the Environment's settings:
  - **On**;
  - **A day lasts** (minutes of play, 24 by default);
  - **Starts at** (the hour the game starts, 08:00);
  - **Colours**: time→colour stops, to begin with dawn 06:00, day 09:00, dusk 18:00 and night 21:00, the night held until 04:30 (five stops), with **+ Add a colour** and a way to take one away. The tint runs smoothly from stop to stop and round midnight.
  - **Preview time**: a slider showing any hour in the viewport. It's the editor's, never saved.
- **The time of day is game state.** It runs with game time (a paused game's clock stands still), and save games keep it (format 8).
  - `World:timeOfDay()` gives the hour, 0 up to 24 (19.5 is half past seven in the evening).
  - `World:setTimeOfDay(h)` sets it at the end of the tick, so sleeping is `World:setTimeOfDay(6)`.
  - In Lua: `solis.World:timeOfDay()`. In Rust: `time_of_day`, `set_time_of_day`.
- **Lighting sprites:**
  - **Light sprites** on the Environment lights sprites and tile maps. It's on in every new area (a project has no 2D-or-3D switch, and with no lamps and day and night off a lit sprite looks exactly as drawn), and off in older areas (an Environment without the field), so nothing already drawn changes.
  - Each sprite pixel's light is the day-and-night tint plus the lamps. A lamp shines on the ground beneath it from its height, so a lamp 1 m up lights a pool round its foot.
  - The light is worked out per pixel of the game's own small picture, so it falls on the pixel grid.
  - **Light steps** (0 smooth; 2 to 8) bands it.
  - Sprites keep their exact colours where the light is 1.
- **Lamps at night:** every Light gets **Only at night**. Such a lamp is on while the tint is darker than half the brightest colour of the day, so it comes on at dusk and goes off at dawn, whatever colours are chosen.
- **3D:** the tint and Only at night work in 3D too: the sun and the sky's light are tinted, and night lamps switch. A real sky with sunsets is Phase 13's. Phase 13 also gets the "more colour options" asked for, colour stops per light, for 3D.
- **Budget:** lamps on sprites use the 16 lamps a frame already carries (decision 0014). Phase 13 lifts that limit for 3D and 2D together; until then, the nearest 16 to the camera are used.

## Consequences

- The Environment component gains Day and night and Light-on-sprites fields. An older area reads with day and night off and sprites unlit, so it looks as it did.
- Save games are format 8 (the time of day). Format 7 saves still load, with the day at its starting hour.
- The viewport, the Game panel and the canvas's preview all draw the tint. The viewport uses the preview time when one is set, and the start time otherwise.
- The 2D sample planned for later (a farm with sleeping) builds on `setTimeOfDay`.
