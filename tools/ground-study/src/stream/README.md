# ground-study/src/stream

The open-worlds study's streaming: only the chunks wanted near the camera and units kept in memory, read on a worker, let go of when memory runs short, and sculpted chunks written back first. Findings in [the report](../../../../docs/reports/open-worlds-study.md).

| File | What it does | Main items |
| --- | --- | --- |
| `mod.rs` | Where chunks come from: region files in a folder, or the world's formulas keeping only what was sculpted. Start here. | `Source`, `Files`, `Made`, `made_region` |
| `region.rs` | One region's file (256 × 256 m, levels 0 to 3): a table first, so one chunk can be read alone; each chunk's runs deflated on its own; a reader that checks everything. | `write`, `read`, `Index`, `pack`, `unpack` |
| `streamer.rs` | What is kept: two ways of choosing what to read (whole regions, or chunks by priority), a memory budget, edits averaged up the levels and written back before they're let go. | `Streamer`, `Policy`, `Wanted`, `wanted_for` |
| `*_tests.rs` | Tests: region files round trip, refuse other regions and damaged files (fuzzed), never inflate past a chunk; the streamer reads what's wanted, lets go of the least wanted first, never loses an edit, averages edits up. | |
