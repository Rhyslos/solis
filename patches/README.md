# patches

The engine's own stand-ins for libraries it can't use as they are. When a library the engine needs depends on something whose licence isn't allowed, and that dependency can't be switched off, a small permissive stand-in with the same interface is written here, from the interface alone (no code taken from the original), with its own tests. The root `Cargo.toml`'s `[patch.crates-io]` points the library at it. See [`docs/dependencies.md`](../docs/dependencies.md), "Patches".

| Folder | Stands in for | Needed by |
| --- | --- | --- |
| [`triple_buffer/`](triple_buffer/README.md) | `triple_buffer` (MPL-2.0) | Kira, the sound library (decision 0056) |

**Adding one:** add a folder here with its own `Cargo.toml` and README, add it to the workspace `members` and `[patch.crates-io]` in the root `Cargo.toml`, and add a row to the table above and to `docs/dependencies.md`. If the library drops the dependency, the patch is removed.
