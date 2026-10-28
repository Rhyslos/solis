# assets/src

Reads model files (glTF 2.0: `.gltf` or `.glb`) and their PNG and JPEG textures into plain data the renderer can upload. Every file is treated as hostile: sizes are checked before anything big is allocated, and bad files give a plain-language `ImportError`, never a crash.

**Start reading at** `model.rs`, then `reader.rs`.

| File | What it does | Main items |
| --- | --- | --- |
| `lib.rs` | Lists the modules and what the crate offers. | `is_model_file` |
| `model.rs` | A model as plain data: meshes, a tree of nodes placing them, materials, textures, bounds, the size limits a file must fit, and the errors a read can give. | `Model`, `Node`, `Mesh`, `Lod`, `Vertex`, `Material`, `Texture`, `Limits`, `ImportError` |
| `reader.rs` | Reads a glTF file into a `Model`, checking every buffer and count before reading it; lists the other files a model refers to. | `load`, `load_bytes`, `referenced_files` |
| `reader_tests.rs` | The reader's tests, including fuzz tests with thousands of damaged files. Kept apart to keep `reader.rs` small. | |
| `geometry.rs` | Turns glTF meshes into triangles and a tree of nodes, turns the file's Y-up into Z-up, fills in missing normals, and splits meshes too big for the GPU. | (internal) |
| `lods.rs` | Levels of detail: simpler versions of each mesh for when a copy is small on screen, made by simplifying or taken from the model's own `_LOD1` meshes (decision 0032). Tests in `lods_tests.rs`. | `add_lods`, `simplified` |
| `aseprite.rs` | Reads Aseprite files (Phase 9, D2) within limits: frames flattened from the visible layers, tags, slices, layers, palette and user data. Tests in `aseprite_tests.rs`, which also writes test files and the fuzz seeds. | `read_aseprite`, `Aseprite`, `AseLimits` |
| `sprite_sheet.rs` | Turns an Aseprite file into a sprite sheet: frames trimmed, identical ones kept once, packed into one atlas with padded edges; where a sprite stands (`pivot`). | `SpriteSheet`, `build_sheet` |
| `texture.rs` | Decodes PNG and JPEG images within limits, and works out their mip levels (smaller copies for distant surfaces). | `decode`, `with_mips`, `solid` |
| `tile_art.rs` | A tileset's picture (Phase 9, D5): a PNG, JPEG or Aseprite file read whole within limits (never scaled; an Aseprite file's every frame and its time); and a picture written as a PNG (templates). Tests in `tile_art_tests.rs`. | `TileArt`, `read_tile_art`, `png_of` |
| `lights.rs` | Reads the lights a file carries (suns, point lights, spotlights) and converts their strength into the units the editor shows. | `Lamp`, `LampKind` |
| `sample.rs` | Builds models in code: the textured cube, the showcase scene, and the ships and playground pieces the ready-made projects use. Used by tests and the window test too. | `sample_glb`, `showcase_glb`, `ship_glb`, `playground_glb`, `Builder` |

## How the pieces fit

`reader::load` opens the file and checks it against `Limits`; `geometry` turns its meshes into `Mesh`es and `Node`s, `texture` decodes its images, `lights` reads its lamps, and `lods` adds simpler levels, all into one `Model`. Nothing here touches the GPU: the `render` crate uploads the `Model`, and `asset_db` keeps it in the import cache.

**Read next:** the crate's [README](../README.md).
