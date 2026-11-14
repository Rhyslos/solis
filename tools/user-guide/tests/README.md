# user-guide/tests

| File | What it checks |
| --- | --- |
| `manual.rs` | The real manual: every page is in `SUMMARY.md`; it builds, with every link going to a page, heading or file that exists; the guide kept in `docs/user-guide/index.html` is exactly what it builds now; and the guide loads nothing from the network. It runs with `cargo test --workspace`, so CI fails when a pull request changes the manual without building the guide again. |

The pieces' own tests (the Markdown reader, the links, the contents, a fuzz test of damaged pages) are beside them in [`../src`](../src/README.md).
