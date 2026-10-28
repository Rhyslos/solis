# layer-check/src

| File | What it does | Main items |
| --- | --- | --- |
| `lib.rs` | The architecture checks: reads the workspace (`load_workspace`), parses each crate's manifest and README into a `CrateInfo` (`parse_crate`), finds the program's own crates (`internal_crate_names`), and lists every broken rule (`check`, returning `Violation`s). Start here. | `CrateInfo`, `Violation`, `LoadError`, `MAX_LAYER` |
| `deps.rs` | The dependency register check: reads which outside libraries each crate uses (`library_uses`) and the register's table (`parse_register`), and says where they disagree (`check`, returning `Problem`s). | `Use`, `Entry`, `Kind`, `Problem` |

`load_workspace` reads only the folders in `crates/` that hold a `Cargo.toml`, so other files there (such as `crates/README.md`) are ignored.

**Read next:** [`../tests`](../tests/README.md), which runs these checks on the real workspace.
