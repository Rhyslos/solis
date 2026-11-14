# tools

Checks and helpers that run beside the code: they test the engine and the editor from outside, rather than being part of them. None ships with the program (the user guide that `user-guide/` makes does).

| Folder | What it does | When it runs |
| --- | --- | --- |
| [`layer-check/`](layer-check/README.md) | A small Rust crate whose tests check every crate follows the layer rules in `ARCHITECTURE.md`, has a proper README, and uses only libraries in the dependency register. | With `cargo test --workspace`, on every pull request |
| [`user-guide/`](user-guide/README.md) | A small Rust crate that builds the user guide (one offline HTML page) from the user manual, and whose tests fail when the guide kept in `docs/user-guide` is out of date, a page is missing from the manual's contents, or a link in the manual is broken (decision 0112). What it makes ships with the program. | `cargo run -p user-guide` after changing the manual; its tests with `cargo test --workspace`, and on documentation-only pull requests through `.github/workflows/user-guide.yml` |
| [`scale/`](scale/README.md) | `scale-check.sh`: times the editor, game code and the viewport against their targets and prints a table. | At the end of each phase and when the project lead asks (decision 0011) |
| [`smoke/`](smoke/README.md) | `window-smoke.sh`: opens the real editor on a virtual screen and clicks through it. | In CI on Linux, and locally before pushing |
| [`ci/`](ci/README.md) | Small scripts the CI workflows use. | In CI |
| [`storage-bench/`](storage-bench/README.md) | The benchmark behind decision 0024 (the game's storage). Kept for the record, outside the workspace. | By hand only |

**Read next:** CONTRIBUTING.md, "Setting up", for the commands, and ARCHITECTURE.md, "Checks", for how these fit with CI.
