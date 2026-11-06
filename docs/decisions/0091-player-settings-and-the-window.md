# 0091. Player settings and the game's window

- **Status:** Accepted
- **Date:** 2026-11-06

## Problem

Decision 0089 settled text size and UI size as player settings on `Screens`, and a screen's glow, outline and pixel size on its Screen component. Two things were still open before the UI sample's pause menu could be built:

- how a game offers fullscreen and a window size;
- where the player's choices are kept.

A playtest export (built later in the same session) needs both.

## Options

1. **The game's own menu, plus Alt+Enter.** Game code gets calls for the window as it has for text size, so a pause menu offers them. Alt+Enter switches fullscreen in every game.
2. **A settings window made by Solis**, the same in every game: no game code, but a game can't look like itself.
3. **Only Alt+Enter**, and the window size from the export.

## Decision

Option 1, the project lead's choice (2026-11-06, "Game's own menu + Alt+Enter").

- **Calls**, following the game API guidelines:
  - text and UI size: `Screens:setTextSize(size)`, `Screens:textSize()`, `Screens:setUiSize(size)` and `Screens:uiSize()` (decision 0089);
  - the window: `Window:setFullscreen(on)`, `Window:fullscreen()`, `Window:setResolution(width, height)`, `Window:resolution()` (a `vec3`, x across and y down), and `Window:useNextResolution()` for a menu's Resolution button, stepping through the usual sizes.
  - In Lua: `solis.Screens:…` and `solis.Window:…`. In Rust: `set_text_size`, `set_fullscreen` and the rest on the host.
  - Changes land at the end of the tick, like every change game code makes.
- **Screens read them and never change them.** A screen that calls a setter stops, saying to send an event instead (decision 0088's rule).
- **The settings are the player's, not the game's.** They're not in save games, recordings or the checksum, and a replay plays the same whatever they are.
- **Where they're kept:**
  - While playing in the editor, they're kept from play to play, for as long as the editor is open. The editor's window isn't the game's, so fullscreen and the window size are only said in the Console.
  - The exported game saves them in the player's saved-games folder, beside its saves, and uses them as it starts.
- **Alt+Enter** switches the fullscreen setting while the game has the keys. The game's code never sees that Enter.
- **The ranges:**
  - text size 0.5 to 3;
  - UI size 0.5 to 2 (a whole number of the game's pixels on a pixel-art screen);
  - each side of the window 320 to 16,384 pixels.
  - A number that isn't one changes nothing.
- **Text size grows the room round text.** It multiplies every text's `size` as the view is laid out, so columns and rows make room and long lines wrap. A piece given a fixed size keeps it, so the canvas previews text at 150% and 200% to check.

## Consequences

- The UI sample's pause menu offers Text size, Fullscreen and Resolution, with buttons sending events to the Pauser.
- `Window` joins the engine objects (`Input`, `World`, `Physics`, `Audio`, `Saves`, `Screens`). It's not one of Lust's reserved names.
- The look from decision 0089 is built as decided:
  - the Screen component has Glow (in each piece's own colour), Outline, Outline colour and Pixel size;
  - Lust's `shadow` on a piece is a soft glow in its colour;
  - Lust is unchanged.
  
  All of it is drawn in the UI shader with no extra pass: each shape's quad grows by its glow and outline, and a pixel size works each block out at its middle.
