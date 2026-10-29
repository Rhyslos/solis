# 0069. The tile painter: tools on the viewport's bar, the Map tab as palette, exact tiles in the map file

- **Status:** Accepted (builds on 0067 and 0068)
- **Date:** 2026-10-29

## Problem

D4's maps and D5's art could be made, but a map's cells could only be changed by hand in the file or by game code. D6 is the tile painter. It had to settle:
- where the tools and the palette sit in the editor;
- which keys and mouse actions paint;
- how a tile placed "as it is", without autotiling, is stored;
- what one undo step is;
- how painting stays smooth on a big map.

## Options

- **Where the tools sit:** three mock-ups drawn on a real screenshot.
  - **A:** the Map tab is the whole painter (tools, palette and layers).
  - **B:** the tools join the viewport's tool bar, with a palette floating on the viewport's left.
  - **C:** a separate Paint panel.

  The project lead chose B, with one change: the palette is the Map tab's terrain swatches on the right, not squares floating on the viewport (mock-up B2).
- **Keys:** B, U, N, G, I (brush, rectangle, line, fill, eyedropper), as in common paint programs and free of Solis's other keys, were chosen over 1 to 5 and over P, R, L, F, K (R is Rotate).
- **Exact tiles:** a separate "art" layer; a flag per cell in the cells blocks; or a list per chunk of the cells that show one piece as it is. The list was chosen: a map without exact tiles is unchanged, and a Git diff shows each one.

## Decision

- **The tools join the viewport's tool bar** while a tile map is selected, after Select, Move, Rotate, Scale and Transform: **Brush (B)**, **Rectangle (U)**, **Line (N)**, **Fill (G)** and **Eyedropper (I)**, then **Exact**, **Art or Meaning** and **the layer painted**. Q, W, R, S or T goes back to a transform tool.
- **The palette is the Map tab's terrain swatches:** click one to paint with it; the outlined one is painted. With Exact on, the terrain's tiles show under the swatches, to choose the one placed (the middle of a patch to start).
- **The mouse while painting:** left-drag paints, right-drag empties, Alt + click picks the terrain under the pointer, the middle button and the wheel still move the view. Right-drag doesn't slide or fly the view while a painting tool is in use.
- **The tools:**
  - the brush paints every cell the pointer crosses, so a fast drag leaves no gaps;
  - the rectangle and the line show their cells while dragged and paint when let go (a rectangle may start off the map: it's clipped);
  - the fill paints every cell joined by its sides with the same terrain as the one clicked: it keeps to one field;
  - the eyedropper copies the cells dragged over (a **copy**, or stamp); the brush then paints with it, repeated like a pattern without seams. **X** and **Y** flip it, **Z** turns it a quarter, **Esc** puts it away. One cell picks its terrain, like Alt + click.
- **Exact tiles:** Ctrl while painting, or Exact on, places the chosen piece as it is: autotiling leaves that cell alone, though its neighbours still see its terrain. Painting the cell again without Exact autotiles it again. Each layer keeps its exact tiles by cell; they're art only, so not in the checksum, and the game never sees them.
- **Tile map files are format 2**, with exact tiles in `exact` blocks per layer and chunk, each `[x, y, "piece ID"]`. Version 1 files read as maps with none; the upgrader changes nothing.
- **Tab** switches the selected map between its art and its meaning (each terrain in its colour, what the game reads).
- **Each stroke is one undo step**, from the button going down to it coming up, kept with the Map tab's steps (Ctrl + Z over the tab, or while a painting tool is in use). A stroke that changes nothing leaves no step. **Esc** during a stroke gives it up.
- **Smooth on big maps:** a stroke paints the map file's own copy, shown at once; only the chunks it touched are autotiled and drawn again; the file is written once, when the button comes up. Painting is off while the game runs: the map then is the game's.

## Consequences

- Budget met: a frame of brushing on a 1,000 × 1,000 map with art takes about 1 ms, and letting go (one undo step, the file saved) about 20 ms.
- Exact pieces are drawn as they are, not flipped: a flipped copy keeps each exact tile's piece and moves it with its cell.
- Tile map files written now aren't read by builds from before; the CHANGELOG labels it Breaking.
- Brush sizes, a random brush and painting values (crop, growth) can come later, through the same strokes.
