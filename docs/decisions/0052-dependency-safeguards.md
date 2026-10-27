# 0052. Safeguards against dependency trouble

- **Status:** Accepted
- **Date:** 2026-10-27

## Problem

Solis's crates use 35 outside libraries directly, and about 560 counting the libraries those use. The project lead asked how Solis is protected from "dependency hell":
- libraries becoming a crutch that is too expensive to fix or remove;
- a library changing its licence;
- legal trouble.

Some safeguards already existed:
- `cargo deny` checks licences, sources and advisories on every build;
- `Cargo.lock` pins versions;
- Dependabot proposes updates as pull requests;
- a weekly security check runs;
- the layer rules keep most libraries inside one crate.

Three things were missing:
- a record of what each library is for and how to leave it;
- a check that libraries stay inside their walls;
- a copy of the source in case a library disappears.

## Decision

Chosen by the project lead:

1. **A dependency register,** `docs/dependencies.md`. Every library Solis's crates use is listed with:
   - what it does for Solis;
   - its licence;
   - the crates allowed to use it;
   - how deep it reaches (walled, shared, foundation, tests or build);
   - an exit plan.
2. **A test holds the register to the truth** (`tools/layer-check`, in CI). It fails if a crate:
   - uses an unlisted library;
   - uses a library outside its allowed crates;
   - leaves a row or an allowance that is no longer used;
   - has a row without an exit plan.
3. **A source archive.** The Source archive workflow saves the source of every library (`cargo vendor`, about 60 MB compressed) whenever `Cargo.lock` changes on main. It is published as a pre-release named after the lock file's hash, and the newest five are kept. It is marked as a pre-release so installed copies, which ask GitHub for the latest release, never see it as an update.
4. **A reason for every new library:** a register row, and a line in the pull request on why Solis shouldn't write it.
5. **A look at each phase's end:** the phase review reads the register for libraries that are unmaintained, growing in reach, or now cheap to replace.

**The foundations are accepted risks:** wgpu, egui with eframe, glam, and serde. Each is large, maintained and permissively licensed, and the exit plan for each is to stay on the last good version and maintain a fork if needed.

## Consequences

- **Adding a library touches the register,** and CI says so if it's forgotten.
- **Licence changes are caught by `cargo deny`;** the register's licence column is for people to read.
- **Solis can be built from the archive** with nothing downloaded (docs/dependencies.md, "Building from the archive").
- **The tools and the fuzzing workspace aren't in the register:** they never ship.
