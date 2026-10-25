# 0010. A Windows EXE is built when code reaches main

- **Status:** Accepted (loosens [0001](0001-builds-on-demand.md) for one build); [0043](0043-installed-and-updated-from-github.md) adds an installer and a release per build
- **Date:** 2026-10-21

## Problem

Decision 0001 keeps GitHub builds rare, because the repo is private and Actions minutes are limited, and Windows minutes count double. But the project lead tries every phase gate on a Windows PC, and building from source there takes Rust and several minutes. A ready-made `solis.exe` makes trying each change a download.

## Options

1. **By hand only:** the lead starts the build from the Actions tab when wanted.
2. **On every merge into main, and by hand.** Merges are rare (one per milestone), so this costs little, and the latest EXE is always waiting.
3. **On every pull request.** Many more Windows builds, mostly of work in progress.

## Decision

Option 2. The `Windows EXE` workflow (`.github/workflows/windows-exe.yml`) builds `solis.exe` in release mode:

- when code is merged into `main` (not for changes to documentation only);
- when started by hand.

It only builds the program: no tests (the pull request already ran them), and no installer (installers come later). The download is a zip with `solis.exe` and its licence files, kept for 30 days.

Everything else in decision 0001 stands.

## Consequences

- About 10 to 15 Windows minutes per merge (20 to 30 of the monthly allowance, since Windows counts double).
- Release builds on Windows open without a console window behind the editor, so messages printed by `solis --version` aren't seen there. The editor's own Console panel shows its messages as before.
