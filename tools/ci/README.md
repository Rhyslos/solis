# tools/ci

Small scripts the CI workflows in [`.github/workflows`](../../.github/workflows/README.md) use.

| File | What it does |
| --- | --- |
| `breaking-check.sh` | Phase 8, C5 (decision 0053): a pull request that adds a new version of a file format (its newest `tests/fixtures/<format>/v<N>/` goes up) or takes something away from what game code can call (a line removed or changed in `docs/game-code-interface.txt`) must add a line under `### Breaking` in CHANGELOG.md's Unreleased part; says why and fails otherwise. Usage: `breaking-check.sh <base> <head>`. Used by `changelog.yml`, even on pull requests labelled internal. |
| `fuzz-lock.sh` | Writes the fuzzers' lock file (`fuzz/Cargo.lock`, not kept in the repository) from the main `Cargo.lock`, so the fuzzers build the library versions the program ships: a copy of the main lock, from which Cargo takes out what the fuzzers don't build and to which it adds the two libraries only they use. Starts from the main lock every time, so it is safe to run again. Fails, saying why and leaving no lock file, if Cargo can't work the file out or the result has another version of a library than the main lock. Usage: `fuzz-lock.sh`, no arguments. Used by every fuzz job of `deep-checks.yml` and by `fuzz/run-all.sh`; see [fuzz/README.md](../../fuzz/README.md#the-lock-file). |
| `failed-tests.sh` | Turns a saved `cargo test` output into one GitHub annotation listing the failed tests and why, so the reason shows on the run's page without downloading the whole log. Usage: `failed-tests.sh test-output.txt`. Used by `ci.yml`. |
