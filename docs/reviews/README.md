# docs/reviews

Code reviews written up at the end of a phase (decision 0027). Each review reads the code since the last one, checks its findings against the code, and sorts them into what to fix now, before the next phase, and later. Fixes go in separate pull requests.

| File | What it holds |
| --- | --- |
| `2026-10-phase-1-review.md` | The end of Phase 1: performance, code and decisions across every crate, CI and tools. Its section 2 fed the Phase 2 plan ([`../plans/phase-2.md`](../plans/phase-2.md)). |
| `2026-11-full-review.md` | A review of the whole codebase asked for by the project lead (10 November 2026): 114 problems, by kind and severity, found by fourteen reviewers, the open issues and the weekly checks run by hand. |
| `2026-11-full-review-status.md` | Where each of that review's 125 findings stands (120 fixed, 5 fixed in part, in pull requests 228 to 250); the notes from every fix; what the project lead has to confirm; and the problems found in the Lust compiler. |
| `2026-11-full-review-second-pass.md` | The final report: the second pass over what the fixes and the editor's UI remake changed (three reviewers: stability, speed, parts meeting), 27 new problems scored 1 to 10 with proposed fixes, the list of everything the review fixed, and the end-of-phase scale check. Nothing in it is fixed yet. |

Name new reviews `YYYY-MM-phase-N-review.md`, or `YYYY-MM-<what>.md` for one outside a phase.
