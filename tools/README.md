# tools

Checks and helpers that run beside the code: they test the engine and the editor from outside, rather than being part of them. None ships with the program.

| Folder | What it does | When it runs |
| --- | --- | --- |
| [`layer-check/`](layer-check/README.md) | A small Rust crate whose tests check every crate follows the layer rules in `ARCHITECTURE.md`, has a proper README, and uses only libraries in the dependency register. | With `cargo test --workspace`, on every pull request |
| [`scale/`](scale/README.md) | `scale-check.sh`: times the editor, game code and the viewport against their targets and prints a table. | At the end of each phase and when the project lead asks (decision 0011) |
| [`smoke/`](smoke/README.md) | `window-smoke.sh`: opens the real editor on a virtual screen and clicks through it. | In CI on Linux, and locally before pushing |
| [`ci/`](ci/README.md) | Small scripts the CI workflows use. | In CI |
| [`storage-bench/`](storage-bench/README.md) | The benchmark behind decision 0024 (the game's storage). Kept for the record, outside the workspace. | By hand only |

**Read next:** CONTRIBUTING.md, "Setting up", for the commands, and ARCHITECTURE.md, "Checks", for how these fit with CI.
