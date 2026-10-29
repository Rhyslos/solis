# tiles

**Layer 1 · Engine**

Tile maps (Phase 9, D4 to D6; decisions [0067](../../docs/decisions/0067-tile-maps.md), [0068](../../docs/decisions/0068-tile-art-and-autotiling.md) and [0069](../../docs/decisions/0069-the-tile-painter.md)): grids that store meaning, not art, and change as the game plays. A tileset lists the terrains a cell can be and their art; a tile map is cells of those terrains in layers, with values every cell has; autotiling works out each cell's art from its neighbours.

## Purpose

- **Tilesets** (`tileset.rs`): `.soltileset`, a settings file with a block per terrain, each by a stable random ID: a name, the one character a map file writes for it (`.` grass, `t` tilled, `~` water) and a colour until tilesets' art arrives (D5). The first terrain fills new maps. Read as a hostile file within limits (256 KB, 64 terrains, 32-character names, codes from a fixed set of plain characters); fuzzed (`fuzz/fuzz_targets/tileset.rs`).
- **Tileset art** (`tileset.rs`, version 2, D5): the picture its art comes from (a PNG or Aseprite asset) and the tile size; each terrain's kind (plain, 16-tile, 47-tile, edges, rules); pieces, each a tile of the picture filling one of a terrain's slots (a second in a slot is a variation, by weight; animated by columns or the picture's frames); and a rule terrain's patterns (up to 9 × 9 of `? = ! _` or a terrain's code). Version 1 reads as plain terrains; the pattern characters can't be codes.
- **Autotiling** (`autotile.rs`, D5): which piece each cell shows, worked out by plain arithmetic from its neighbours (past the map's edge counts as the same). `chunk_tiles` places a chunk's tiles in passes (later terrains over earlier), a 16-tile terrain's pieces half a cell off at the corners where four cells meet (each corner in one chunk only); a slot with no piece is `Art::Missing`, never another; a terrain without art is `Art::Colour`. A cell with an exact tile (D6) shows its piece as it is, over the rest of its layer. Variations are picked from a hash of the cell's place, the same every time. Also each kind's template (`template_rgba`), the picture's grid (`detect_tile`) and kind (`guess_kind`) guessed, and marking up from a template (`mark_up`).
- **Tile maps** (`map.rs`): a size (each side 1 to 4,096), a tileset's asset ID, layers (each a byte per cell, the terrain's code or 0 for empty, drawn under or over sprites) and values (number, switch or choice, each a column with a cell's value in it). Layers and columns are `Arc`'d, so a copy (an undo step, the running game's) shares them until changed. A change marks its 32 × 32 chunk (`take_dirty` for whoever redraws), and `checksum` works out only dirty chunks again. `cells_where` finds every cell whose value holds ("every watered crop"); `counts` says how many cells use each code.
- **Exact tiles** (`map.rs`, D6): per layer, the cells painted to show one piece as it is (`set_exact`), kept by cell index; art only, so not in the checksum. A cell's code changing clears its exact tile.
- **Painting** (`paint.rs`, D6): what the editor's tile painter touches, apart from the editor: a line's cells (Bresenham, no gaps), a rectangle clipped to the map, a fill's reach (cells joined by their sides with the same terrain), and a `Stamp` copied from an area, flipped, turned and put down.
- **Tile map files** (`map_file.rs`): `.soltilemap`, a settings file: the tileset and size, a block per layer and value, a block per layer's chunk of rows of codes (a space for empty), and a block per chunk of only the values not at their start (`[x, y, value]`). Version 2 (D6) adds a block per layer's chunk of its exact tiles (`[x, y, "piece"]`); version 1 reads with none. A Git diff shows exactly the changed cells. Read as a hostile file within limits (64 MB, 300,000 blocks, 16 layers, 32 values, 64 options); fuzzed (`fuzz/fuzz_targets/tile_map.rs`).

## Must never

- Crash, hang or read past a limit on a damaged tileset or map file.
- Change a cell without marking its chunk: the checksum and the drawing rely on it.
- Know about the GPU, the window or the editor.

## Depends on

`area` (the settings-file reader), `reflect` (asset and stable IDs, checksums). No pictures are read here: the art's pixels come from `assets::tile_art`, and only arrays of them are looked at (grid and kind guesses).

## Status

Phase 9, D4: tilesets with colours, maps with layers and values, their files. D5: tilesets' art and autotiling. D6: exact tiles, the painter's shapes and stamps (the painter itself is the editor's).
