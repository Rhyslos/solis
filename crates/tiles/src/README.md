# tiles/src

| File | What it does | Main items |
| --- | --- | --- |
| `lib.rs` | The crate's map. Start here. | |
| `tileset.rs` | A tileset (`ground.tileset.lst`, version 4, a Lust data file; older `.soltileset` files too): its picture and tile size, terrains (a name, a code, a colour, a kind), pieces (a tile of the picture filling a slot, its weight and animation) and rules; read within limits (version 1 too) and written. Tests in `tileset_tests.rs`. | `Tileset`, `Terrain`, `TileKind`, `Piece`, `Animation`, `Rule`, `Cond`, `read`, `write`, `usable_code` |
| `map.rs` | A tile map: layers of codes, values per cell, chunks marked as they change, a checksum per chunk. Tests in `map_tests.rs`. | `TileMap`, `Layer`, `ValueDef`, `ValueKind`, `CellValue`, `MapError` |
| `autotile.rs` | Autotiling: each kind's slots from a cell's neighbours (16-tile on the dual grid, 47-tile, edges, rules), variations picked by weight from the cell's place, a chunk's tiles placed (`chunk_tiles`), missing slots, animation frames, templates, and the picture's grid and kind guessed. Tests in `autotile_tests.rs`. | `chunk_tiles`, `Placed`, `Art`, `pick`, `missing_slots`, `slot_count`, `template_rgba`, `detect_tile`, `guess_kind`, `mark_up` |
| `map_file.rs` | A tile map file (`farm.tilemap.lst`, or an older `.soltilemap`): chunks of rows of codes and sparse values; read within limits, and written only when the file would be read back (`write` refuses a map whose file is past what the reader takes). Tests in `map_file_tests.rs`. | `read`, `write`, `write_within`, `ReadMap`, `MapFileError`, `STANDARD_SIZE` |
| `map_lust.rs` | A tile map as a Lust data file (decision 0081): its classes (`CLASSES`), written from the map's parts and read back into them: no more things than the biggest map has, and each chunk's values gathered by looking the chunk up, however many a file names. Tests in `map_lust_tests.rs`. | `CLASSES`, `KIND`, `read`, `write` |
