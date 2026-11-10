# tiles/tests

Test files for the `tiles` crate. The tests themselves are beside the code they try, in [`../src`](../src/README.md).

| Folder | What it holds |
| --- | --- |
| `data/` | Files the tests read that aren't examples of a format (those are in the repository's `tests/fixtures`): `two-classes-one-name.tileset.lst`, a tileset as the fuzzer's first run left one, made small, with two classes of one name, which the reader took for ever over and now refuses (`tileset_tests.rs`). It is a fuzzing seed too ([`fuzz/make-seeds.sh`](../../../fuzz/make-seeds.sh)). `data/` has no README of its own, because the tests read its files as data. |
