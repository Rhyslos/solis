# ground-study/src/study/worlds

The open-worlds study's measurements (`cargo run --release -p ground-study -- worlds`), behind [the report](../../../../../docs/reports/open-worlds-study.md). `mod.rs` runs them in the report's order; each prints a Markdown table.

| File | What it measures |
| --- | --- |
| `mod.rs` | The order, the measured places, and making and meshing chunks for a layout. Start here. |
| `seams.rs` | On a square kilometre with every level averaged: popping between levels, each way of closing seams in each look, and what is made again when the camera moves. |
| `far_edits.rs` | Remaking a chunk at each level, and sculpting where the ground is drawn coarse. |
| `view.rs` | The 20 km view each way: triangles, graphics memory, time to make it, cracks seen from the camera, and flying across it. |
| `streaming.rs` | A 2 km set of region files: sizes, reads, flying across it with each way of reading, a far edit written back, and streaming from formulas. |
| `units.rs` | Colliders in two halves, a character running onto ground not yet solid, and 2,000 units spread out or together. |
| `disk.rs` | The 20 km file set from a sample, opening a project, making a starting ground, and how far it goes (20 to 164 km). |
| `pictures.rs` | With the `gpu` feature: the report's pictures through the engine's renderer, and the region map. |
