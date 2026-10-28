# layer-check/tests

| File | What it does |
| --- | --- |
| `workspace.rs` | Runs the checks on the real workspace: the architecture rules hold, every planned crate exists and is registered in the workspace, only the app and its updater are in the top layer, the engine never depends on the editor, and every outside library is in the dependency register. If one fails, its message says which crate broke which rule. |

Run alone with `cargo test -p layer-check`. The checks themselves are in [`../src`](../src/README.md).
