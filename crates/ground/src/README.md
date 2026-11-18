# ground/src

| File | What it does | Main items |
| --- | --- | --- |
| `lib.rs` | The crate's map. Start here. | |
| `voxel.rs` | One voxel (distance in 1/64ths of a voxel, ground kind: two bytes), where it sits in a chunk, and the chunk a world place is in. | `Voxel`, `SIZE`, `VOLUME`, `index`, `place`, `split` |
| `store.rs` | Chunks (one voxel if the same all through, else shared voxels) behind the `Store` interface, which can say a chunk isn't loaded; fixed chunks. Tests in `store_tests.rs`. | `Chunk`, `Store`, `Slot`, `Fixed`, `keys`, `origin` |
| `ground.rs` | A ground of chunks: a voxel anywhere (the edges carried on), "unknown" for chunks not loaded, whether a chunk is bare, and a chunk's samples copied for meshing. Tests in `ground_tests.rs`. | `Ground`, `Answer`, `Samples`, `BORDER` |
| `smooth.rs` | The Smooth look: surface nets, each crossed edge once, facings from the voxels, chunk-local places. Tests in `smooth_tests.rs` (same bytes twice, no cracks, seams shared, facing out). | `ChunkMesh`, `Mesher`, `mesh` |
| `query.rs` | The height at a place, the kind at a place, a ray against the ground. Tests in `query_tests.rs`. | `Hit`, `Ground::height_at`, `Ground::kind_at`, `Ground::ray` |
| `start.rs` | A new ground's starting shape: flat or gently rolling. | `Start` |
| `noise.rs` | Value noise, the same on every PC. | `noise3`, `noise2`, `layered2` |
| `settings.rs` | The settings file (`.ground.lst`, Lust): look, size, height, palette, recipe. Tests in `settings_tests.rs`. | `Settings`, `GroundKind`, `Look`, `read`, `write`, `FORMAT`, `CLASSES` |
| `region.rs` | The binary region files: a table of chunks with room for levels 1 to 3, deflated runs. Tests in `region_tests.rs`. | `write`, `read`, `load_into`, `file_name`, `keys` |
