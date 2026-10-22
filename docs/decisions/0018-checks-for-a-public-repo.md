# 0018. Fuller checks, now the repository is public

- **Status:** Accepted (replaces decision 0001; keeps 0010)
- **Date:** 2026-10-22

## Problem

Decision 0001 kept GitHub's checks rare because the repository was private and Actions minutes were limited: checks ran only on pull requests marked ready, Windows tests only by hand, nothing on a schedule. On 2026-10-22 the repository became public, and GitHub's standard machines are free and unlimited for public repositories.

The Phase 1 review (`docs/reviews/2026-10-phase-1-review.md`) also found that Solis, made only for Windows (decision 0012), was only ever tested on Linux: the real graphics path on Windows (Direct3D 12) was never exercised.

## Decision

- **Checks run on every update to a pull request** that isn't a draft, and on every merge into `main`, plus by hand. Documentation-only changes still skip the build.
- **Windows tests on every pull request:** the whole test suite on a Windows machine, including the GPU tests on Direct3D 12 through Windows' software renderer (WARP), so the graphics path Solis actually ships on is checked.
- **A weekly security check:** `cargo deny check advisories` every Monday, so a newly found vulnerability in a dependency shows up without waiting for the next pull request.
- **Dependabot** opens pull requests for outdated dependencies and actions once a week, grouped so they don't flood the list.
- Everything else from 0001 stays: cheap checks first, a newer run cancels an older one, caches kept, actions pinned by their exact version, and every check run locally before pushing (CI is the second line, not the first).
- Decision 0010 stays: the Windows EXE is built on every merge.

## Consequences

- Broken builds show on a pull request straight away, and on Windows as well as Linux.
- Pull requests take a little longer to go green: the Windows run takes about 15–25 minutes.
- If the repository ever goes private again, minutes become limited again and these rules need revisiting.
