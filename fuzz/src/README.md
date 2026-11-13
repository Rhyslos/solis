# fuzz/src

| File | What it does | Main items |
| --- | --- | --- |
| `lib.rs` | What every fuzz target shares, as the small library `fuzz_support`: `quiet_caught_panics`, which each target calls first, so a panic inside a library from crates.io (one Solis catches on purpose and turns into a refused file) isn't printed, while a target's own `assert!` message, or a panic in Solis's code, is. | `quiet_caught_panics` |

**Read next:** the targets, in [`../fuzz_targets`](../fuzz_targets/README.md).
