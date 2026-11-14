# docs/user-guide

`index.html` is the **user guide**: the user manual ([`../manual`](../manual/SUMMARY.md)) built into one page to read in a browser, with a search and a contents list ([decision 0112](../decisions/0112-the-user-guide-is-the-manual-built.md)). Download it and open it, or see it in the program: the Windows build ships the same file beside the program as `user-guide/index.html`, and the editor's **Documentation** button opens it.

**Never edit it by hand.** It is made by [`tools/user-guide`](../../tools/user-guide/README.md) from the manual:

```
cargo run -p user-guide
```

Change the manual, run that, and commit both together. A test (`tools/user-guide/tests/manual.rs`) fails when this file isn't what the manual builds now. Git keeps it byte for byte and folds it away in pull requests' changes (`.gitattributes`), where the manual's own change is the one to read.
