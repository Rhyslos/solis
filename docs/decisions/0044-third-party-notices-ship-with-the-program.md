# 0044. The libraries' notices ship with the program

- **Status:** Accepted
- **Date:** 2026-10-26

## Problem

The project lead asked whether Solis's licence and copyright notice are correct. Solis's own `LICENSE` is the standard MIT text with "Copyright (c) 2026 Rhyslos". That's right, and so is copying Aventail's code in ([decision 0007](0007-mit-licence-and-separate-from-aventail.md)).

But every build includes about 310 open-source libraries (egui, wgpu, Rapier, Luau's bindings and the rest). Most of their licences (MIT, BSD, ISC and Apache 2.0) ask that their copyright notice and licence text go with every copy of the program, including a compiled one. Until now only the fonts' licences shipped, in `licenses/`. `cargo deny` checks that every library's licence allows use in Solis, but nothing put their notices in the download or the installer.

## Options

1. **Write the list by hand.** It would be wrong after the next dependency update.
2. **Generate it for every build** from the exact libraries built in, with cargo-about (Embark Studios, MIT or Apache 2.0). It's the usual tool for Rust programs, and reads the same licence data `cargo deny` checks.
3. **Show the notices inside the program** (an About box page). This can come later; the file comes first, since it's what the licences ask for.

## Decision

Option 2. The Windows EXE workflow runs cargo-about (pinned to 0.9.2) for the program, for Windows, without build or test-only libraries. It writes `THIRD-PARTY-NOTICES.txt`, which goes:

- next to the program in the download;
- into the installed folder.

The text says what's in it, lists how many libraries use each licence, then gives each licence's full text once, with the libraries under it.

- `about.toml` holds the settings, with the same accepted licences as `deny.toml`.
- `licenses/third-party.hbs` is the layout.

## Consequences

- Each build takes about a minute longer on Windows, mostly installing cargo-about.
- A library under a licence `about.toml` doesn't accept stops the build. `deny.toml` would already have refused it in the pull request, so both lists must change together.
- The installer still shows Solis's MIT licence before installing. MIT doesn't ask anyone to accept anything; showing it is the usual courtesy.
- When games can be exported (Phase 10), they need the same file for the engine's libraries. The same command, run for the game, gives it.
