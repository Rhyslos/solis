# 0027. Regular code reviews for stability and speed

- **Status:** Accepted
- **Date:** 2026-10-24

## Problem

After Phase 3, the project lead asked for three separate reviewers to read all the code for stability and efficiency. They found about 60 problems. Among them were crashes, ways to lose work, places where two parts of Solis disagreed, and work repeated every frame. Tests and the scale check hadn't caught these, because they test what was thought of. The project lead wants this done regularly, so the engine stays stable and fast.

## Decision

- **At the end of every phase,** before its gate, and whenever the project lead asks, three reviewers read the code built since the last review. Each works on its own, without seeing the code's author's reasoning:
  1. **Stability:** crashes, hangs, lost work, damaged or hostile files, limits.
  2. **Efficiency:** work repeated every frame, costs that grow faster than the area does, memory that's never freed, work on the frame thread that belongs on a worker.
  3. **Correctness where parts meet:** places where two parts of Solis (editor and game, app and editor, disk and database) can get out of step.
- **Every finding is checked before it's fixed.** A finding that turns out wrong is dropped, with the reason noted.
- **Fixes go in pull requests grouped by kind,** the usual way: tests, documentation and a changelog entry for each. Anything left for later is listed in the pull request, with the phase it's left to.
- **The scale check runs with the efficiency fixes,** and grows a check for each slow case found.

## Consequences

- Each phase ends with a review batch as well as the scale check (decision 0011).
- The first review (after Phase 3) is pull requests #46–#48.
