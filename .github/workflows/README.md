# .github/workflows

The checks and builds GitHub runs on its own machines ([decision 0018](../../docs/decisions/0018-checks-for-a-public-repo.md)). Changes that touch only documentation (`*.md`, `docs/`, licences) skip the build; the user guide's own small check still runs on changes to the manual. Actions are pinned by commit and the tools workflows install by version (see [below](#tools-the-workflows-install)); older runs are cancelled and builds are cached, to keep runs cheap.

| File | What it does | When |
| --- | --- | --- |
| `ci.yml` | Formatting and licences first (on Linux; neither builds Solis); then on Windows the lints, every test (GPU ones through Windows' software renderer) and the API docs. Failed tests are summarised by `tools/ci/failed-tests.sh`. Its two Linux build jobs (tests and docs; the GPU tests and the window test) are paused with `if: false` ([decision 0102](../../docs/decisions/0102-linux-checks-paused.md)): they install Ubuntu packages first and fail when Ubuntu's servers can't be reached. | Every update to a non-draft pull request, merges into main, release tags, and by hand |
| `changelog.yml` | Fails a pull request that changes code (`crates/`, `tools/`, `Cargo.*`) without a `CHANGELOG.md` entry, unless it is labelled `internal`. | Every update to a non-draft pull request, and when labels change |
| `security.yml` | Checks dependencies for newly published security advisories (`cargo deny check advisories`). | Mondays, and by hand |
| `deep-checks.yml` | What's too slow for every pull request (decision 0042): every fuzz target for a time box (20 minutes each, one job per target, side by side), starting from `fuzz/seeds`, the whole-game invariant tests with many more games, and the updater's two tests that need the internet. The targets are read from `fuzz/Cargo.toml` by the run's first job, never listed in the workflow, so a new target is fuzzed with no change here; that job fails if a file in `fuzz/fuzz_targets` has no `[[bin]]` or a `[[bin]]` no file. Each fuzz job first writes the fuzzers' lock file from the main `Cargo.lock` (`tools/ci/fuzz-lock.sh`), so what is fuzzed is the library versions the program ships; no lock file is kept in `fuzz/`. The updater's job (`updater`) runs on Windows, where installed copies update and where the system's own certificates are what the updater trusts; the rest run on Linux. It runs exactly two tests, by name (`github_answers` and `the_newest_installer_downloads_whole_and_matches` in `crates/updater/src/tests.rs`): they ask the real GitHub for the newest build and download its installer, checked against its published SHA-256, and never start it. A failing try is repeated, three in all, two minutes apart, because the network can fail when the updater hasn't; the step also fails if fewer than two tests ran, so a renamed test can't quietly drop out. A failure in the Sunday run opens an issue. | Sundays, and by hand |
| `user-guide.yml` | The user guide's checks (`cargo test -p user-guide`, decision 0112): the guide kept in `docs/user-guide` is what the manual builds now, every page is in `SUMMARY.md`, every link goes somewhere. Builds only that small tool, so it needs no Ubuntu packages. | Pull requests and merges that change the manual, the guide or what it is made from, documentation-only ones included (which `ci.yml` skips) |
| `windows-exe.yml` | Builds the program for Windows, stamps its build number, puts the user guide (`docs/user-guide/index.html`) beside it as `user-guide/index.html`, makes the installer from `installer/setup.iss`, and publishes a `build-<number>` release that installed copies update from (decisions 0010 and 0043). | Code merged into main, and by hand |
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
| `libfuzzer-sys` and `arbitrary` | The library the fuzz targets are built against, and the one it needs; only the fuzzers use them | Exact versions (`=`) under `[dependencies]` in `fuzz/Cargo.toml` |

- **Inno Setup is checked before it is used.** GitHub's Windows machines come with an Inno Setup, and which one changes when the machines are rebuilt. The "Make the installer" step asks the compiler for its version; if it isn't the pinned one, the step installs that version with Chocolatey and asks again, and the build fails if the answer is still anything else. No installer is ever made by a compiler nobody chose.
- **Dependabot doesn't update these.** It reads `Cargo.lock` and the actions' commits, not versions written inside a step. A person changes them, in a pull request of its own, and then starts the workflow by hand (the Actions tab, "Run workflow") to see it pass: `windows-exe.yml` for Inno Setup and cargo-about, `deep-checks.yml` for cargo-fuzz, the nightly compiler and the fuzzers' two libraries, which change together. The end-of-phase look at the libraries ([docs/dependencies.md](../../docs/dependencies.md), safeguard 8) looks at these too.
- **Not pinned:** the packages `ci.yml` and `deep-checks.yml` take from Ubuntu with `apt-get` (the sound library's headers, and the virtual screen and its helpers for the window test). There is one list of them, in [`tools/ci/linux-packages.sh`](../../tools/ci/README.md), and every Linux job that builds a crate runs it before building: without the sound library's headers nothing that uses the `sound` crate builds, the editor included. They come from the archive of the Ubuntu version GitHub's machine runs, and nothing from them ends up in the program. The stable compiler that builds Solis is pinned in `rust-toolchain.toml`.

## The rest of `.github`

The folder above has no README of its own, because GitHub would show a README there as the repository's front page in place of the main one. Its other files:

| File | What it does |
| --- | --- |
| `../dependabot.yml` | Opens grouped pull requests weekly for outdated Rust libraries and GitHub Actions, skipping versions held back on purpose (each also denied in `deny.toml`). It reads the main `Cargo.lock`, the only lock file kept: the fuzzers' is written from it ([fuzz/README.md](../../fuzz/README.md#the-lock-file)). |
| `../pull_request_template.md` | The form every pull request fills in: what changed, test results, documentation updated, how to test. |

**Read next:** ARCHITECTURE.md, "Checks", and CONTRIBUTING.md, "Pull requests".
