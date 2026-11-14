# ground-study/src/mesh

Each look's competing ways of making a chunk's triangles from its samples. Every way meshes a chunk from its own samples and two more all round, so neighbouring chunks make the same points where they meet and no crack opens.

| File | What it does |
| --- | --- |
| `mod.rs` | `ChunkMesh` (points, facings, materials, triangles) and `Method`, the six ways and which look each draws. Start here. |
| `cases.rs` | Marching cubes' table of 256 cases, worked out from the cube's faces rather than typed in. |
| `marching.rs` | Smooth: marching cubes. |
| `dual.rs` | Smooth and Sharp: surface nets, dual contouring and sharpened surface nets, which differ only in where each cube's point goes. |
| `qef.rs` | Where several planes come closest to meeting (dual contouring's small least-squares problem). |
| `normals.rs` | Hard edges for the Sharp look: facings worked out again from the triangles, split where faces meet steeply. |
| `blocky.rs` | Blocky: cube faces, plain or merged into rectangles. |
| `mesh_tests.rs` | Closed surfaces across chunks, facing out, the same voxels giving the same mesh, merged faces covering exactly the plain ones, sharp edges kept. |
