# ground-study/src/lod

The open-worlds study's levels of detail: far ground drawn coarser. Level 0 is the ground as sculpted (a voxel a metre); each level up has voxels twice as wide, made by averaging, and a level's sample `i` sits at `i · 2^level` metres, so every level's chunk edges line up. Findings in [the report](../../../../docs/reports/open-worlds-study.md).

| File | What it does | Main items |
| --- | --- | --- |
| `mod.rs` | `Levels` (what meshers read), the tent-filtered `average`, a `Pyramid` of every level kept up to date when sculpted, and `Kept`, just the chunks a streamer holds. Start here. | `Levels`, `Pyramid`, `Kept`, `average` |
| `layout.rs` | Which size of chunk is drawn where: nested rings of fixed-size chunks, or an octree split by distance and balanced. | `Layout`, `Column`, `new_columns` |
| `nets.rs` | Smooth and Sharp at any level, and four ways of meeting a neighbour of another level: nothing, skirts, a transition band, stitched seams. | `mesh`, `border`, `Seams`, `Look` |
| `blocky.rs` | Blocky at any level: cubes that nest, with walls between sizes or skirts. | `mesh` |
| `skirts.rs` | Strips hung from a mesh's open edges along its sides. | `hang`, `open_edges` |
| `cracks.rs` | Finding cracks: open edges after welding, and rays from a camera that see inside the ground. | `open_edges`, `Scene`, `deep` |
| `*_tests.rs` | Tests: averaging and sculpting the pyramid; the meshers at one level equal to the ground study's, the transition band and stitches closed in Smooth and Sharp, Blocky's walls leaving nothing to see through, the same voxels giving the same mesh. | |
