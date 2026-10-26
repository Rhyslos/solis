# updater

**Layer 3 · App**

Keeps installed copies of the program up to date from GitHub ([decision 0043](../../docs/decisions/0043-installed-and-updated-from-github.md)).

## Purpose

Find out whether a newer build has been published, download its installer, make sure it arrived whole, and start it.

- `release`: reads GitHub's answer about the latest release (`read_release`) and the checksum file beside the installer (`read_checksum`), within limits (`MAX_RELEASE_TEXT`, `MAX_ASSETS`, `MAX_INSTALLER_SIZE`). A release's tag is `build-` and its number (`build_of_tag`). Drafts and pre-releases are refused, and download addresses must be the repository's own releases on github.com (`is_release_download`).
- `Source`: where to ask (the repository, the installer's name, the User-Agent). Its `latest` and `download` go through ureq over HTTPS checked by the system's certificates, with time and size limits. A download streams into a `.part` file while its SHA-256 is worked out, and is kept only if it matches.
- `this_build`: the build number stamped in through `BUILD_NUMBER` when GitHub built this copy (none otherwise). `is_installed`: whether the installer's `installed.txt` sits beside the program. `updates_folder`: where downloads go.
- `run` and `start`: one check, deciding from this copy (`Copy`) and what the `Remote` says. It never downloads, and ends in a `Status`: up to date, a newer build available (with what GitHub says of it), built from source, or failed (in plain words). `start` runs it on a thread and reports each step on a channel.
- `download` and `start_download`: downloading a release the person chose (Update now), ending ready (with the installer) or failed (decision 0045).
- `should_offer`: whether to ask about a build when starting: not the one skipped with Skip this version (asking always does).
- `start_installer`: runs the installer quietly (`installer_arguments`), and with `/RELAUNCH=1` after Restart to update.

## Must never

- Run on the window's thread: every check goes through `start`.
- Install anything it hasn't checked against its published SHA-256, or download from anywhere but the repository's releases.
- Update a copy that wasn't installed by the installer, or one built from source.
- Send anything about the user: only the request, with a User-Agent naming the program.
- Panic on any answer from the network.

## Depends on

ureq (with rustls, ring and rustls-platform-verifier), sha2, serde and serde_json, log.

## Status

Decision 0043: checks, downloads, checksums, and starting the installer. Tests cover reading releases and checksums (with random and damaged answers), every way a check can end (with a stand-in for GitHub), and, when asked (`-- --ignored`), a real question to GitHub.
