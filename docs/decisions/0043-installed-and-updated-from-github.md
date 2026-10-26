# 0043. Installed on the computer, and updated from GitHub

- **Status:** Accepted (adds to [0010](0010-windows-exe-on-merge.md)); how updates are offered is changed by [0045](0045-a-splash-screen-that-asks-before-updating.md)
- **Date:** 2026-10-25

## Problem

The project lead tries each change by downloading a zip from GitHub's Actions tab, unpacking it and running the program from wherever it landed. Every merge means doing that again, and the zip disappears after 30 days. The lead asked for Solis to be properly installed, and to update itself from GitHub whenever something new is merged into main.

## Options

**How it's installed**

1. **The program copies itself** somewhere on first run. It can't add an entry in Settings > Apps for uninstalling without a lot of Windows-specific code.
2. **An installer made with Inno Setup** on GitHub's Windows machines. It's free, widely used, and makes a normal Windows installer: Start menu entry, uninstall entry, and an optional desktop shortcut. It's a build tool, not part of the program, so its licence doesn't reach Solis.
3. **MSI (WiX)**. More enterprise features, but harder to write, and it needs an administrator for most setups.

**Where builds are published**

1. **Actions artifacts** (today). These need signing in to GitHub to download, and expire after 30 days.
2. **A GitHub release per build**, tagged `build-<number>`. Anyone can download one, it's permanent, and GitHub offers a "latest release" address to ask.

**When an update installs**

1. **As soon as it's downloaded.** This interrupts work, and could lose it.
2. **When the program closes, or now with a Restart to update button.** Work is never interrupted, and the update is never more than one close away.
3. **Only when asked.** Easy to forget.

## Decision

**Installer:** option 2, Inno Setup (`installer/setup.iss`).

- It installs for the current user only, so no administrator is needed, into `%LOCALAPPDATA%\Programs\<name>` or a folder the user chooses (asked for by the project lead). The installer refuses a folder the user can't write to (Program Files, say), since the program must be able to replace itself when it updates. Updates go to the same folder: Inno Setup remembers it.
- It adds a Start menu entry and an uninstall entry, and offers a desktop shortcut.
- It leaves `installed.txt` beside the program. Only a copy with that file updates itself, so a copy run from a zip, or built from source, is left alone.

**Publishing:** option 2, a release per build. The Windows EXE workflow, on every merge into main:

1. stamps the program with its build number (the workflow's run number, through `BUILD_NUMBER`);
2. makes the installer;
3. writes its SHA-256 beside it;
4. publishes both as release `build-<number>`;
5. keeps the newest ten releases.

The name, ID and repository come from `branding.rs`, so they aren't written in the workflow or the installer.

**Updating:** option 2. The new `updater` crate (layer 3, beside `app`) does the work, on a thread of its own, so the window never waits (CONTRIBUTING rule 10).

- **When it checks:** when the window opens, then every six hours.
- **Turning it off:** Help > Update automatically switches the checking off. Help > Check for updates checks now.
- **How it checks:** it asks GitHub's API for the latest release, and compares its build number with its own.
- **Downloading:** when the release is newer and this copy was installed, it downloads the installer and its checksum file over HTTPS. The connection is checked with Windows' own certificates.
- **Checking the download:** it checks the SHA-256, and throws away a download that doesn't match.
- **Installing:** a ready update installs quietly when the window closes. "Restart to update" in the top bar closes the window (asking about unsaved work first), installs, and opens the program again.

The reading is strict, like any file the program reads (rule 9):

- answers are size-limited;
- drafts and pre-releases are ignored;
- download addresses must be this repository's releases on github.com;
- the installer has a size limit.

## Consequences

- The program now talks to the internet. It sends no information about the user, only the request (with a User-Agent naming the program and its version, as GitHub requires).
- Copies built from source have no build number. They never check, and say so under Help.
- New dependencies, all MIT or Apache: `ureq` (HTTPS) with `rustls`, `ring` and `rustls-platform-verifier`; `sha2`; and `serde_json`. There's no bundled certificate list: Windows' own is kept up to date by Windows. One crate under a data licence (`webpki-root-certs`, CDLA-Permissive-2.0) is listed only for WebAssembly builds, which Solis never makes, so `deny.toml` makes an exception for it alone.
- **Trust:** the checksum protects against a broken or cut-short download, not against someone who controls the GitHub repository. Signing the program with a code-signing certificate would add that, and also stop Windows SmartScreen warning on the first install. It costs money each year, so it's left for later.
- The workflow needs permission to write releases (`contents: write`) for its one job.
- Each merge publishes a release, so GitHub's Releases page lists builds rather than hand-written versions. When the project starts making numbered releases, they can sit beside these, or replace them.
- The first install is still by hand: download the setup file from the Releases page and run it. Windows may warn that it doesn't know the publisher (no signing yet): click More info, then Run anyway.
