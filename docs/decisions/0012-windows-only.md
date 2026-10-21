# 0012. Solis is made for Windows

- **Status:** Accepted (narrows "PC only" in DESIGN.md §14 and the export plan in Phase 10)
- **Date:** 2026-10-21

## Problem

The plan said Solis would build for Windows and Linux, with macOS later, and CI could test on all three. The project lead uses Windows and will never test Linux or macOS. Every platform costs Actions minutes, and a platform nobody tries is a platform whose bugs nobody sees.

## Decision

**Solis is made for Windows:** the editor, and the games it exports. Builds for Linux and macOS are dropped.

- The Windows EXE (decision 0010) is the one build.
- CI's automatic checks still run on a **Linux machine**. That isn't a Linux build: Linux machines are the cheapest on GitHub (Windows minutes count double), and they have the virtual screen the window test uses. The engine's code is the same on both.
- A **Windows test run** happens on release tags, and when started by hand ("also test on Windows").
- macOS is dropped from CI entirely.

Nothing in the code is made Windows-only on purpose: the libraries Solis uses work everywhere, so Linux or macOS could come back later if someone wants them.

## Consequences

- Fewer Actions minutes, and no time spent on platforms nobody uses.
- Phase 10 exports games for Windows only.
- Bugs that only happen on Windows are caught by the project lead's own testing, and by the Windows test run on release tags.
