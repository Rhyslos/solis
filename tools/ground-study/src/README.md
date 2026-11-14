# ground-study/src

The library (`lib.rs`) is the prototypes; `main.rs` and `study/` are the program that measures them.

| File or folder | What it does | Main items |
| --- | --- | --- |
| `lib.rs` | Names the modules below. Start here. | |
| `voxel.rs` | One voxel (distance from the surface in 1/64ths of a voxel, and a material, two bytes) and a chunk of 32 × 32 × 32 laid out in full. | `Voxel`, `Dense`, `SIZE` |
| `store.rs` | The three stores compared, each keeping a chunk the surface passes through. | `Store`, `Full`, `Sparse`, `Octree` |
| `ground.rs` | A box of chunks (a chunk the same all through is one voxel in every store), a chunk's samples with two more all round, and a brush dab applied across chunks, saying which chunks to mesh again. | `Ground`, `Chunk`, `Samples` |
| `brush.rs` | One dab of the round brush: Add, Dig, Smooth, Paint. | `Brush`, `Action`, `Region` |
| [`mesh/`](mesh/README.md) | The looks' ways of meshing. | `Method`, `ChunkMesh` |
| `collision.rs` | A chunk's mesh as the physics crate's still triangle collider (points welded), and a character's walk. | `solid`, `add_chunk`, `walk` |
| `file.rs` | The ground's file as text and as bytes (runs of equal voxels), squeezing, and readers that check every count. | `text`, `binary`, `read_text`, `read_binary` |
| `shapes.rs` | The test grounds: the report's hill with a cave and an overhang, the target's rolling square kilometre, the busy worst-case chunk. | `hill_with_cave`, `rolling_chunk`, `busy` |
| `noise.rs` | Smooth random numbers over space, the same every run. | `noise3`, `layered2` |
| `*_tests.rs` | Tests of the module of the same name. | |
| `main.rs` | The measuring program's commands. | |
| [`study/`](study/README.md) | The measurements, each printing a Markdown table. | |

**Read next:** [the report](../../../docs/reports/ground-study.md).
