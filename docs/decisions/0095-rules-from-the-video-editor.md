# 0095: Rules brought over from the video editor project

- **Status:** Accepted
- **Date:** 2026-11-10

## Problem

The project lead is also making a video editor, whose ground rules they find good for Solis too. Most overlap rules Solis already has, but each goes a little further.

## Decision

Solis takes the four rules, merged into its own instead of added beside them:

1. **No name-dropping.** CONTRIBUTING.md rule 20 widens from "don't name games" to every company and product: plans, decisions, the manual, code and the app describe features instead. Technical facts Solis needs (a licence, a paper's author, a file format, a tool it works with) can still be named.
2. **Documented folders, always current.** Solis already has a README in every folder; now `CLAUDE.md` is held to the same standard, updated with every change (the documentation table in CONTRIBUTING.md).
3. **Readable, clean, stable code.** Rule 3 grows from "small units" to code understandable without heavy commenting: clear names, small modules, consistent patterns.
4. **Research and compare for big systems.** Rule 22 (decision 0051, "understand, then build") adds: for large or advanced systems, build competing approaches, review them against each other and keep the best. "Works" is not the bar; fast, smooth and robust is.

## Consequences

- Big systems take longer to start, since more than one approach is built and measured; the study in `docs/reports/` records the comparison.
- Older documents may still name other products. They are fixed when next touched, not in one sweep.
