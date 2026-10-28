# tools/storage-bench

The benchmark behind [decision 0024](../../docs/decisions/0024-game-storage-is-hecs.md): the `hecs` library against a small storage written for the engine, on what a game at the scale target does each tick (DESIGN.md section 2). It is its own little workspace, not part of the main one, so it never enters the engine's build; it is kept so the decision can be checked again.

| File or folder | What it holds |
| --- | --- |
| `Cargo.toml`, `Cargo.lock` | The benchmark's own package and pinned libraries. |
| [`src/`](src/README.md) | The benchmark and the candidate storage it was measured against. |

Run with `cargo run --release --manifest-path tools/storage-bench/Cargo.toml`. Each line it prints is the median of many runs, in microseconds.
