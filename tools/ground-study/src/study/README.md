# ground-study/src/study

The measurements behind [the report](../../../../docs/reports/ground-study.md). `mod.rs` runs them in the report's order; each prints a Markdown table.

| File | What it measures |
| --- | --- |
| `mod.rs` | The order. Start here. |
| `stats.rs` | Timing, spreads (mean, median, 95th, worst) and the tables. |
| `scale.rs` | The target ground (1,024 × 1,024 × 128 voxels): memory and the stores' own costs, the file in each form, and meshing every surface chunk each way. |
| `strokes.rs` | Brush strokes of 60 dabs over a corner where four chunk columns meet: editing in each store, and meshing again and colliders each way. |
| `solid.rs` | Each chunk's collider, and 2,000 characters walking over the whole ground. |
| `frames.rs` | A dab's chunks made on the frame against on a worker, and the frame's own work while workers mesh. |
| `gpu.rs` | With the `gpu` feature: uploads through the engine's renderer, and the report's pictures. |
