# tools/ci

Small scripts the CI workflows in [`.github/workflows`](../../.github/workflows/README.md) use.

| File | What it does |
| --- | --- |
| `breaking-check.sh` | Phase 8, C5 (decision 0053): a pull request that adds a new version of a file format (its newest `tests/fixtures/<format>/v<N>/` goes up) or takes something away from what game code can call (a line removed or changed in `docs/game-code-interface.txt`) must add a line under `### Breaking` in CHANGELOG.md's Unreleased part; says why and fails otherwise. Usage: `breaking-check.sh <base> <head>`. Used by `changelog.yml`, even on pull requests labelled internal. |
| `failed-tests.sh` | Turns a saved `cargo test` output into one GitHub annotation listing the failed tests and why, so the reason shows on the run's page without downloading the whole log. Usage: `failed-tests.sh test-output.txt`. Used by `ci.yml`. |
