# translations/tests

Test files for the `translations` crate. The tests themselves are beside the code they try, in [`../src`](../src/README.md).

| Folder | What it holds |
| --- | --- |
| `data/` | Files the tests read that aren't examples of a format: `shop.ftl`, the shop's words in Norwegian (values, a plural, a line with an attribute, a term). It is the translation fuzzer's seed too ([`fuzz/make-seeds.sh`](../../../fuzz/make-seeds.sh)). `data/` has no README of its own, because the tests read its files as data. |
