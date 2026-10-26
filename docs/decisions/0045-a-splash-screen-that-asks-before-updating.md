# 0045. A splash screen that asks before updating

- **Status:** Accepted (changes how [0043](0043-installed-and-updated-from-github.md) updates; its installer and releases stay)
- **Date:** 2026-10-26

## Problem

Since decision 0043, an installed Solis checked GitHub when it opened and every six hours, downloaded a newer build quietly, and installed it when it closed. The project lead asked for something more visible, like Aventail's splash screen ([Aventail decision 0079](https://github.com/Rhyslos/aventail-3d/blob/main/docs/decisions/0079-splash-screen.md)):

- a splash screen that shows for at least 0.6 s and runs the update check;
- when the check finishes, Solis opens, or a window says an update is available and offers to update now or open without updating;
- a "Skip this version" tick, after which that build isn't offered again until a newer one comes out;
- a setting that stops the checking altogether.

## Decision

**The splash screen** (`editor::splash::show`) is Aventail's, with Solis's logo:

- the logo, the name, the version, a thin bar in the accent colour, and what's happening, on the dark background;
- it shows for at least 0.6 s (`startup::SPLASH_AT_LEAST`), so a quick start doesn't just flash it;
- it covers the window's first frames, not a second window;
- the editor is made behind it as before.

**While it shows,** Solis asks GitHub for the newest build (`updater::start`, which now only checks and never downloads).

- **Up to date, built from source, or checking off:** Solis opens once the 0.6 s are up.
- **Offline or failing:** it opens, with a quiet line in the Console.
- **GitHub hasn't answered within 5 s** (`startup::CHECK_WAIT`): it opens anyway. The answer, when it comes, shows under Help and in the Console; it never pops up in the middle of work.

**A newer build** brings up the question, *Update available*: "Build N of Solis is available (you have build M)."

- **Update now:**
  1. the splash says "Downloading build N…";
  2. the download is checked against its SHA-256 as before;
  3. Solis closes, and the installer runs quietly and opens the new build.
  - If the download fails, this build opens and says why.
- **Open without updating** opens this build. With **Skip this version** ticked, that build is remembered (`skipped_build` in the settings) and not offered at start again. A newer build is offered as usual.
- **A copy that wasn't installed with the installer** can't replace itself. It gets **Open the download page** (the Releases page) instead of Update now, and a line saying why.

**The setting:** Help > **Check for updates when starting** (on unless turned off). Off: nothing is asked of GitHub unless the person chooses Help > Check for updates.

**Help > Check for updates** asks GitHub now and shows the same question for a newer build, even a skipped one: asking is asking.

- **Update now** downloads it while the person works.
- **Restart to update** (top right) then installs it and opens it again, or it installs when Solis closes.

**The checks every six hours are gone.** Checking happens when Solis starts, and when asked.

## Consequences

- Updating is always the person's choice, and never needs more than one click.
- Starting takes at least 0.6 s, and longer only while waiting on GitHub, for at most 5 s.
- Copies installed from builds before this (up to the one with this change) behave as decision 0043 says until they update to it.
- No new keys or mouse actions (CONTRIBUTING rule 17).
