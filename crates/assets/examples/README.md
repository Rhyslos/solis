# assets/examples

Small programs that use the `assets` crate, run with `cargo run -p assets --example <name>`.

| File | What it does |
| --- | --- |
| `sample.rs` | Writes a sample model to a `.glb` file for trying the importer: `cargo run -p assets --example sample -- out.glb` writes the small one (a textured cube on a red base); add `--scene` before the file name for the bigger sample scene. CI uses it to make the model the window test imports. |

The models themselves are built in [`../src/sample.rs`](../src/sample.rs).
