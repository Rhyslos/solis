# user-guide/tests

| File | What it checks |
| --- | --- |
| `manual.rs` | The real manual: every page is in `SUMMARY.md`; it builds, with every link going to a page, heading or file that exists, every picture shown there and shown by a page, and every game-code example in all three languages; the guide kept in `docs/user-guide/index.html` is exactly what it builds now; the guide loads nothing from the network; the pictures are as new as the editor's interface (their stamp, `tools/guide-pictures/interface.txt`, matches the approved picture-test images in `crates/editor/tests/pictures`); and each example's Rust balances its brackets and its Lua and Rust call only what `docs/game-code-interface.txt` lists. The examples' Lust and Lua are checked against the engine itself in `crates/script/src/manual_examples_tests.rs`. It runs with `cargo test --workspace`, so CI fails when a pull request changes the manual without building the guide again. |

The pieces' own tests (the Markdown reader, the links, the contents, a fuzz test of damaged pages) are beside them in [`../src`](../src/README.md).
