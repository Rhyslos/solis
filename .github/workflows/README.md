# .github/workflows

The checks and builds GitHub runs on its own machines ([decision 0018](../../docs/decisions/0018-checks-for-a-public-repo.md)). Changes that touch only documentation (`*.md`, `docs/`, licences) skip the build. Actions are pinned by commit and the tools workflows install by version (see [below](#tools-the-workflows-install)); older runs are cancelled and builds are cached, to keep runs cheap.

| File | What it does | When |
| --- | --- | --- |
| `ci.yml` | Formatting, lints and licences first; then tests and API docs on Linux, the GPU tests and the window test on Linux, and every test (GPU ones through Windows' software renderer) on Windows. Failed tests are summarised by `tools/ci/failed-tests.sh`. | Every update to a non-draft pull request, merges into main, release tags, and by hand |
| `changelog.yml` | Fails a pull request that changes code (`crates/`, `tools/`, `Cargo.*`) without a `CHANGELOG.md` entry, unless it is labelled `internal`. | Every update to a non-draft pull request, and when labels change |
| `security.yml` | Checks dependencies for newly published security advisories (`cargo deny check advisories`). | Mondays, and by hand |
| `deep-checks.yml` | What's too slow for every pull request (decision 0042): every fuzz target for a time box, starting from `fuzz/seeds`, and the whole-game invariant tests with many more games. A failure in the Sunday run opens an issue. | Sundays, and by hand |
| `windows-exe.yml` | Builds the program for Windows, stamps its build number, makes the installer from `installer/setup.iss`, and publishes a `build-<number>` release that installed copies update from (decisions 0010 and 0043). | Code merged into main, and by hand |
| `sources.yml` | Keeps a copy of every library's source code as a pre-release, so a library deleted from the internet can't stop the build (decision 0052). | When `Cargo.lock` changes on main, and by hand |

A step that must fail when a command before a pipe fails (`cargo test ... | tee file`) sets `shell: bash`. That turns on bash's `pipefail`; GitHub's default shell on Linux leaves it off, and the step would pass whatever the first command did.

## Tools the workflows install

A tool a workflow downloads is pinned to one exact version, like the actions, so a new release of it can't change or break a build with no change in this repository. The installer matters most: every installed copy downloads and runs it without asking.

| Tool | What it's for | Where its version is set |
| --- | --- | --- |
| Inno Setup | Makes the installer | `INNO_SETUP_VERSION` at the top of `windows-exe.yml` |
| cargo-about | Writes `THIRD-PARTY-NOTICES.txt` | The "Third-party notices" step of `windows-exe.yml` (`cargo-about@<version>`) |
| cargo-fuzz | Runs the fuzz targets | `CARGO_FUZZ_VERSION` at the top of `deep-checks.yml` |
| The nightly compiler | Builds the fuzz targets (never Solis itself) | `FUZZ_NIGHTLY` at the top of `deep-checks.yml`, a dated nightly |

- **Inno Setup is checked before it is used.** GitHub's Windows machines come with an Inno Setup, and which one changes when the machines are rebuilt. The "Make the installer" step asks the compiler for its version; if it isn't the pinned one, the step installs that version with Chocolatey and asks again, and the build fails if the answer is still anything else. No installer is ever made by a compiler nobody chose.
- **Dependabot doesn't update these.** It reads `Cargo.lock` and the actions' commits, not versions written inside a step. A person changes them, in a pull request of its own, and then starts the workflow by hand (the Actions tab, "Run workflow") to see it pass: `windows-exe.yml` for Inno Setup and cargo-about, `deep-checks.yml` for cargo-fuzz and the nightly compiler, which change together. The end-of-phase look at the libraries ([docs/dependencies.md](../../docs/dependencies.md), safeguard 8) looks at these too.
- **Not pinned:** the packages `ci.yml` and `deep-checks.yml` take from Ubuntu with `apt-get` (the sound library's headers, and the virtual screen and its helpers for the window test). They come from the archive of the Ubuntu version GitHub's machine runs, and nothing from them ends up in the program. The stable compiler that builds Solis is pinned in `rust-toolchain.toml`.

## The rest of `.github`

The folder above has no README of its own, because GitHub would show a README there as the repository's front page in place of the main one. Its other files:

| File | What it does |
| --- | --- |
| `../dependabot.yml` | Opens grouped pull requests weekly for outdated Rust libraries and GitHub Actions. |
| `../pull_request_template.md` | The form every pull request fills in: what changed, test results, documentation updated, how to test. |

**Read next:** ARCHITECTURE.md, "Checks", and CONTRIBUTING.md, "Pull requests".
