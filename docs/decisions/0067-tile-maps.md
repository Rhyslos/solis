# 0067. Tile maps: meaning in cells, a file per map, values every cell has

- **Status:** Accepted (builds on 0059, 0063 and 0065)
- **Date:** 2026-10-28

## Problem

A farming game's world is a grid: grass that's tilled, then planted, watered and grown. Phase 9's D4 brings tile maps that change as you play. It had to settle:
- where a project's terrains are listed;
- where a map lives, and how it's placed in an area;
- whose layers a map has;
- what a cell can hold besides its terrain;
- how the editor shows a map before the tile painter (D6) and tileset art (D5) arrive.

## Options

- **Terrains:** in the project's settings; in a tileset file (`.soltileset`) that D5's art joins later; or in each map.
- **A map:** inside the area file; its own file, placed on an entity by a component; or a whole area that is a map.
- **Layers:** a fixed set every map shares; the project's list; or each map's own.
- **Cell values:** numbers only; number, switch and choice; or any engine value.
- **The editor:** the first mock-ups (a list under Properties) were turned down as too plain. Three others were drawn on a real screenshot: A, a Map tab beside Properties; B, a layers strip in the viewport; C, a panel of its own under the Area panel.

## Decision

Chosen by the project lead: every recommended option, and mock-up A.

- **Terrains live in a tileset file** (`.soltileset`). Each terrain has a stable ID, a name, the one character a map file writes for it (`.` grass, `t` tilled, `~` water) and a colour. The first terrain fills new maps. D5 adds each terrain's art and its autotiling kind to the same file. Read within limits (256 KB, 64 terrains, names of 32 characters, codes from a fixed set of plain characters), fuzzed, version 1 with an example.
- **A map is its own file** (`.soltilemap`), placed by an entity's **Tile Map** component. Its top-left corner is at the entity, rows run down the screen, and a cell is the project's tile size (2D settings). The file holds:
  - its tileset and size (each side 1 to 4,096 cells);
  - its layers;
  - its values;
  - its cells, in chunks of 32 × 32.

  Each chunk of a layer is a block of rows of codes, with a space for an empty cell. Each chunk's values are a block of only the cells not at their start (`[x, y, value]`). A Git diff shows exactly the cells that changed, and two people's changes to different chunks merge. Read as a hostile file within limits (64 MB, 300,000 blocks, 16 layers, 32 values, 64 options), fuzzed, version 1 with an example.
- **Layers are the map's own.** A new map has Ground and Decoration (under sprites) and Above (over sprites: roofs, treetops). A layer under sprites is drawn first, under everything; one over sprites is drawn last, over everything. The renderer draws each layer as one picture with a texel per cell, in its terrains' colours until D5 brings art. A picture is made again only when its layer changes.
- **Cell values are the map's own**, each a number (a whole number), a switch or a choice from named options. Every cell has each one, starting at zero, off or none. So "crop: corn, growth: 3, watered" needs no entity per plant.
- **In the running game,** each entity with a Tile Map runs its own copy of its file's map, which the game changes cell by cell. Its cells and values are game state:
  - in the checksum, chunk by chunk, worked out again only where they changed;
  - in saves, as a section of the map file's own text per entity (`solis save 5`, still reading 1 to 4).

  A map whose file changes while playing starts again from it. The world's calls (`tile_map`, `set_tile`, `set_cell_value`, `tile_maps`) are what game code's calls will use.
- **The Map tab** (mock-up A) opens beside Properties while a map is selected, and closes when it isn't. It shows:
  - the layers, top first, each with a small picture, its name, under or over sprites, an eye (shown or hidden in the editor only, never saved) and a menu (move up, move down, delete);
  - the tileset's terrains as swatches with how many cells use each;
  - the values, with their kinds and a choice's options;
  - the cell under the pointer in the viewport: every layer's terrain and every value.

  Each change is one undo step (Ctrl+Z over the tab) and is saved at once. While playing, the tab shows the game's map.
- **Over the viewport,** while a map is selected:
  - a **Values** menu lays chosen values over the map (a switch as a blue tint, a number or a choice written on the cell);
  - a **Chunks** switch outlines its chunks;
  - the cell under the pointer is outlined.
- **The Library** makes new tilesets and maps (Import > New tileset, New tile map), places a map with a double-click or a drag, and shows a tileset's terrains or a map file's tileset and size in Properties.
- **Game code reaches a map through a Lust file the editor writes for it** (rule 4 of the API guidelines, decision 0065): a state of its terrains, layers and values, and a class with a method per value. It arrives with the API milestone, like animators. No string-named calls are added meanwhile.

## Consequences

- **Game code can't read or change maps yet.** The plan's `tileAt`, `setTile`, `cellValue`, `setCellValue` and `cellsWhere` become methods of each map's generated class with the API milestone.
- **Cells are painted with the tile painter (D6).** Until then a map's cells come from its file, edited by hand, or from the game.
- **Terrains show as flat colours** until D5's art and autotiling. A cell whose code isn't in the tileset is drawn pink, and the Map tab counts such cells.
- **Budgets met:** 10,000 cell changes take 0.2 ms (the budget is 1 ms), and a 1,000 × 1,000 map reads in 62 ms (the budget is 1 s).
- **Selecting a map is done in the Area panel:** a click on a map in the viewport passes through to what stands on it, because the painter will use clicks on the map.
