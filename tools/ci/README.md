# tools/ci

Small scripts the CI workflows in [`.github/workflows`](../../.github/workflows/README.md) use.

| File | What it does |
| --- | --- |
| `breaking-check.sh` | Phase 8, C5 (decision 0053): a pull request that adds a new version of a file format (its newest `tests/fixtures/<format>/v<N>/` goes up) or takes something away from what game code can call (a line removed or changed in `docs/game-code-interface.txt`) must add a line under `### Breaking` in CHANGELOG.md's Unreleased part; says why and fails otherwise. Usage: `breaking-check.sh <base> <head>`. Used by `changelog.yml`, even on pull requests labelled internal. |
| `fuzz-lock-check.sh` | Holds the fuzzers to the library versions the program ships. `fuzz/` is its own workspace with its own `Cargo.lock`, which nothing in Cargo keeps in step with the main one. Fails, naming each package and how to fix it, when `fuzz/Cargo.lock` has a version of a package that the main `Cargo.lock` has at other versions only (packages only the fuzzers use are left alone); needs no Cargo. `--locked` then asks Cargo whether a build of the fuzzers would change the file, without building anything (`cargo fuzz` has no `--locked` of its own). `--fix` copies the main lock over the fuzzers' and has Cargo take out what they don't build and add what only they use; commit the result. Usage: `fuzz-lock-check.sh [--locked \| --fix]`. Used with `--locked` by `ci.yml` and `deep-checks.yml`; see [fuzz/README.md](../../fuzz/README.md#the-lock-file). |
| `failed-tests.sh` | Turns a saved `cargo test` output into one GitHub annotation listing the failed tests and why, so the reason shows on the run's page without downloading the whole log. Usage: `failed-tests.sh test-output.txt`. Used by `ci.yml`. |
