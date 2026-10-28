# tiles/src

| File | What it does | Main items |
| --- | --- | --- |
| `lib.rs` | The crate's map. Start here. | |
| `tileset.rs` | A tileset (`.soltileset`): terrains with a name, a code and a colour; read within limits and written. Tests in `tileset_tests.rs`. | `Tileset`, `Terrain`, `read`, `write`, `usable_code` |
| `map.rs` | A tile map: layers of codes, values per cell, chunks marked as they change, a checksum per chunk. Tests in `map_tests.rs`. | `TileMap`, `Layer`, `ValueDef`, `ValueKind`, `CellValue`, `MapError` |
| `map_file.rs` | A tile map file (`.soltilemap`): chunks of rows of codes and sparse values; read within limits and written. Tests in `map_file_tests.rs`. | `read`, `write`, `ReadMap`, `STANDARD_SIZE` |
