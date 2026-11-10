# animation/tests

Test files for the `animation` crate. The tests themselves are beside the code they try, in [`../src`](../src/README.md).

| Folder | What it holds |
| --- | --- |
| `data/` | Files the tests read that aren't examples of a format (those are in the repository's `tests/fixtures`): `unseen-character.atlas.lst`, an atlas as the fuzzer's first run left one, made small, whose picture's name holds a control character (number 23, where its dot was). It was read with the character and written without it, so the atlas didn't write back to itself; now the reader refuses it (`atlas_tests.rs`). It is a fuzzing seed too ([`fuzz/make-seeds.sh`](../../../fuzz/make-seeds.sh)). `data/` has no README of its own, because the tests read its files as data. |
