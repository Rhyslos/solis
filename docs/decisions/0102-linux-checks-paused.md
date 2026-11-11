# 0102: The Linux checks on GitHub are paused

- **Status:** Accepted (changes [0018](0018-checks-for-a-public-repo.md))
- **Date:** 2026-11-11

## Problem

Decision 0018 runs every pull request's checks on Linux and on Windows. The Linux jobs install Ubuntu packages before they build (the sound library's headers; a virtual screen and its helpers for the window test). On 11 November 2026 Ubuntu's package servers couldn't be reached from GitHub's machines: the first job stopped before any of Solis was looked at, the jobs that wait for it were skipped, and a pull request whose code was fine couldn't be merged. Solis is made for Windows only ([decision 0012](0012-windows-only.md)); the Linux runs were extra checks.

## Decision

The project lead's call (2026-11-11): the Linux package is ignored for now, paused until the engine is much further along.

- **Paused:** the two Linux jobs of `ci.yml` that build Solis, "Tests and documentation (Linux)" and "Window and GPU tests (Linux)". They stay in the file, switched off with `if: false`.
- **Moved to Windows:** the lints (`cargo clippy`) and the API docs (`cargo doc`), which also build Solis, now run in the Windows job with the tests. The lints therefore read the code as Windows builds it, which they never did before.
- **Kept on Linux:** formatting and the licence check. Neither builds Solis, so neither needs a package.
- **Still run on Linux by hand** before every push, as CLAUDE.md already asks: the whole test suite, the docs, the GPU tests on the software driver, and the window test (`tools/smoke/window-smoke.sh`). GitHub no longer repeats them.
- **Not changed:** the Sunday deep checks (`deep-checks.yml`), whose fuzz jobs install the same packages. If Ubuntu's servers are down on a Sunday that run fails and opens an issue, as any failure of it does.

To bring the jobs back, delete the two `if: false` lines in `ci.yml` (each says so) and move the lints and docs back if wanted.

## Consequences

- A pull request can be merged while Ubuntu's servers are down.
- GitHub no longer runs the window test or the GPU tests on a second driver (Mesa); they depend on being run by hand. A change pushed without the local checks could break them unseen until someone runs them.
- The Windows job does more (lints and docs as well as tests), so a run takes longer; its time limit is 75 minutes.
- The first Windows lint run may find warnings in code that only Windows builds.
