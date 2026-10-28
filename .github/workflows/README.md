# .github/workflows

The checks and builds GitHub runs on its own machines ([decision 0018](../../docs/decisions/0018-checks-for-a-public-repo.md)). Changes that touch only documentation (`*.md`, `docs/`, licences) skip the build. Actions are pinned by commit, older runs are cancelled, and builds are cached, to keep runs cheap.

| File | What it does | When |
| --- | --- | --- |
| `ci.yml` | Formatting, lints and licences first; then tests and API docs on Linux, the GPU tests and the window test on Linux, and every test (GPU ones through Windows' software renderer) on Windows. Failed tests are summarised by `tools/ci/failed-tests.sh`. | Every update to a non-draft pull request, merges into main, release tags, and by hand |
| `changelog.yml` | Fails a pull request that changes code (`crates/`, `tools/`, `Cargo.*`) without a `CHANGELOG.md` entry, unless it is labelled `internal`. | Every update to a non-draft pull request, and when labels change |
| `security.yml` | Checks dependencies for newly published security advisories (`cargo deny check advisories`). | Mondays, and by hand |
| `deep-checks.yml` | What's too slow for every pull request (decision 0042): every fuzz target for a time box, starting from `fuzz/seeds`, and the whole-game invariant tests with many more games. A failure opens an issue. | Sundays, and by hand |
| `windows-exe.yml` | Builds the program for Windows, stamps its build number, makes the installer from `installer/setup.iss`, and publishes a `build-<number>` release that installed copies update from (decisions 0010 and 0043). | Code merged into main, and by hand |
| `sources.yml` | Keeps a copy of every library's source code as a pre-release, so a library deleted from the internet can't stop the build (decision 0052). | When `Cargo.lock` changes on main, and by hand |

## The rest of `.github`

The folder above has no README of its own, because GitHub would show a README there as the repository's front page in place of the main one. Its other files:

| File | What it does |
| --- | --- |
| `../dependabot.yml` | Opens grouped pull requests weekly for outdated Rust libraries and GitHub Actions. |
| `../pull_request_template.md` | The form every pull request fills in: what changed, test results, documentation updated, how to test. |

**Read next:** ARCHITECTURE.md, "Checks", and CONTRIBUTING.md, "Pull requests".
