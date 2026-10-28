# storage-bench/src

| File | What it does |
| --- | --- |
| `main.rs` | The benchmark: 2,000 units at peak, and ten times that for headroom, doing a game's work each tick in `hecs` and in the candidate storage, printing the median time of each. Start here. |
| `own.rs` | The candidate storage, in safe Rust: one sparse set per component type, each component's values packed in a vector, an entity's slot found in one step. |

**Read next:** [decision 0024](../../../docs/decisions/0024-game-storage-is-hecs.md) for what the numbers decided.
