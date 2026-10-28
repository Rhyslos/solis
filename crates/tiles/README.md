# tiles

**Layer 1 · Engine**

Tile maps (Phase 9, D4; [decision 0067](../../docs/decisions/0067-tile-maps.md)): grids that store meaning, not art, and change as the game plays. A tileset lists the terrains a cell can be; a tile map is cells of those terrains in layers, with values every cell has.

## Purpose

- **Tilesets** (`tileset.rs`): `.soltileset`, a settings file with a block per terrain, each by a stable random ID: a name, the one character a map file writes for it (`.` grass, `t` tilled, `~` water) and a colour until tilesets' art arrives (D5). The first terrain fills new maps. Read as a hostile file within limits (256 KB, 64 terrains, 32-character names, codes from a fixed set of plain characters); fuzzed (`fuzz/fuzz_targets/tileset.rs`).
- **Tile maps** (`map.rs`): a size (each side 1 to 4,096), a tileset's asset ID, layers (each a byte per cell, the terrain's code or 0 for empty, drawn under or over sprites) and values (number, switch or choice, each a column with a cell's value in it). Layers and columns are `Arc`'d, so a copy (an undo step, the running game's) shares them until changed. A change marks its 32 × 32 chunk (`take_dirty` for whoever redraws), and `checksum` works out only dirty chunks again. `cells_where` finds every cell whose value holds ("every watered crop"); `counts` says how many cells use each code.
- **Tile map files** (`map_file.rs`): `.soltilemap`, a settings file: the tileset and size, a block per layer and value, a block per layer's chunk of rows of codes (a space for empty), and a block per chunk of only the values not at their start (`[x, y, value]`). A Git diff shows exactly the changed cells. Read as a hostile file within limits (64 MB, 300,000 blocks, 16 layers, 32 values, 64 options); fuzzed (`fuzz/fuzz_targets/tile_map.rs`).

## Must never

- Crash, hang or read past a limit on a damaged tileset or map file.
- Change a cell without marking its chunk: the checksum and the drawing rely on it.
- Know about the GPU, the window or the editor.

## Depends on

`area` (the settings-file reader), `reflect` (asset and stable IDs, checksums).

## Status

Phase 9, D4: tilesets with colours, maps with layers and values, their files. D5 adds tilesets' art and autotiling; D6 the tile painter.
