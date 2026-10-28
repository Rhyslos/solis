# tools/ci

Small scripts the CI workflows in [`.github/workflows`](../../.github/workflows/README.md) use.

| File | What it does |
| --- | --- |
| `failed-tests.sh` | Turns a saved `cargo test` output into one GitHub annotation listing the failed tests and why, so the reason shows on the run's page without downloading the whole log. Usage: `failed-tests.sh test-output.txt`. Used by `ci.yml`. |
