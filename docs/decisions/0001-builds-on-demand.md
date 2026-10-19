# 0001. Builds run on demand, not on every update

- **Status:** Accepted
- **Date:** 2026-10-20

## Problem

The repo is private. On GitHub's free plan, private repos get a limited number of Actions minutes each month, and Windows and macOS runners use them up faster than Linux. An engine is slow to compile, so building on every push would burn through the allowance quickly and mostly build work-in-progress nobody needed checked.

## Decision

No workflow runs on every push. When engine development begins, CI follows these rules:

1. **Pull requests only,** and only when marked ready for review. Draft pull requests and branch pushes don't build.
2. **Manual runs** (`workflow_dispatch`) for anything else.
3. **Docs-only changes never build.** Workflows ignore changes to Markdown files and `docs/`.
4. **Cheap checks first:** formatting and lints on Linux; the full build and tests only if those pass.
5. **Linux by default.** Windows and macOS builds run only manually or for a release tag.
6. **A newer run cancels an older one** on the same pull request, and build caches are kept between runs.
7. **No scheduled or nightly builds.**

Checks are run locally before pushing (Claude runs the full set in its session and lists the results in each pull request), so CI is a second line, not the first.

## Consequences

- A broken build can reach a branch unnoticed until its pull request is ready. That's acceptable: branches are work in progress.
- If the repo becomes public, Actions minutes are free and these rules can loosen. Revisit then.
