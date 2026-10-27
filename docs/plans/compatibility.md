# Phase 8 plan: Compatibility (draft)

**Draft for the project lead.** Before the project lead's own game moves to Solis, updates must never lose work or break a game ([decision 0053](../decisions/0053-compatibility-before-2d.md)). This is the light version the project lead chose: **safety that works without anyone operating it.**

**Gate:**
- open a copy of a project saved by an older build, and see Solis back it up and upgrade it;
- a save game made before an update loads after it;
- a deliberately broken pull request (a renamed native, a changed format without its upgrader) is stopped by the checks.

## The guarantee

**You never lose work.** These hold whatever happens:
- every file Solis writes is written whole or not at all (atomic writes, already built);
- before Solis changes any file in a project, it saves a dated copy;
- the project is in Git, an undo that doesn't depend on Solis.

On top of that, most breakage is caught before it reaches a project, and the rest can be undone by installing the previous build (the last ten are kept).

## Where things stand

- **Atomic writes:** `asset_db::project::write_atomically` writes a temporary file and renames it over the old one. It's used for the project file and sidecars; this phase makes every writer use it.
- **Area files** start with `area version 1;` (`area::text::VERSION`). A newer version is refused with a clear message, but there is no code to upgrade an older one: there has only been version 1.
- **Save games** start with `solis save N`. An older one is refused ("it was made by an earlier Solis, whose saves this one can't read"), which would wipe a shipped game's players' progress on update.
- **The game-code interface** is written out as Lust text (`script::describe`, the `solis/*.lst` files), so it can be compared.
- **Recordings** (`.solrec`) replay a game exactly, and state checksums compare results (Phase 6). Whole-game tests already play the samples.

## Milestones

### C1. Backups and upgraders for every file format
- **One shared upgrade path:** each format has a version, and a chain of upgraders (1 → 2 → 3) that each turn one version into the next. Opening an older file upgrades it in memory first, so the file on disk isn't touched until it's saved.
- **Before Solis writes an upgraded file back:**
  1. it saves a copy to `.solis/backups/<date and time>/`, keeping its folder layout;
  2. it reads the new file back and checks it gives the same project;
  3. only then does it swap the file in.

  If any step fails, the old file stays as it was, and the Console says why. Old backups are trimmed by age and total size, never the newest.
- **Old example files, kept forever:** each format gets a folder of example files, one per version ever released, in `tests/fixtures/<format>/v<N>/`. A test opens every one with the current build and checks the result. A build that can't open an older file can't be merged.
- **Formats covered:** areas and assemblies, the project file, settings, input actions, recordings. Later formats (tile maps, materials, effects, UI screens) join as they're made.
- **A newer file than the build understands** is still refused with a clear message, as now, and never changed.
- **Fuzz tests** for each upgrader: a damaged old file is an error, never a crash or a half-upgrade.

### C2. Save games that survive updates
- **Old saves load:** the save format joins the upgrade path, so a save made by an earlier build loads in a later one. A save is never refused just for being older.
- **What a save holds** (decision 0038) is upgraded field by field. A component a game no longer has is reported, not a crash.
- **Tests:** saves made by each released save version, kept as examples, load and play on.
- **Games stay in charge:** an exported game can say "saves older than version N of *my game* can't be loaded" for its own reasons. That's its choice, never Solis's.

### C3. The game-code interface, guarded
- **A snapshot of the interface:** what game code can call (every native, its arguments and their types, for Lust, Lua and Rust) is written to `docs/game-code-interface.txt` by a test. If the interface changes and the file wasn't updated in the same pull request, the test fails and shows the difference.
- **Renames keep working.** A native renamed on purpose keeps its old name as an alias for a set number of phases. The Problems panel warns on each use ("`spawnAt` is now `spawn`; the old name stops working in Phase N") and offers **Fix**, which rewrites the call, or **Fix all** for the project.
- **Removals** follow the same path: a warning for a set number of phases first.

### C4. Real projects tested before an update ships
- **`solis --check-project <folder>`:**
  1. opens a project without the editor's window;
  2. reads every file (reporting any that fail to open or upgrade);
  3. checks the game code;
  4. plays each recording in the project's `recordings/` folder, comparing its final state checksum with the one saved beside it.

  It exits with a plain report, and a failure code if anything changed.
- **The samples, in CI:** the turret sample, the benchmark and the playground (and the 2D sample, when it exists) are checked on every pull request.
- **The project lead's game,** in their own private repository: a small workflow there runs each new Solis build's `--check-project` against the game and emails on a failure. Solis's public repository never holds it. Or the project lead runs the same check by hand before updating.
- **Making a recording to check against** uses what exists: **Play > Play and record inputs** (Phase 6). A new **Keep as a check** option saves the recording with its final checksum in `recordings/`, which makes it part of the check.

### C5. Breaking changes, labelled
- **CHANGELOG.md gets a "Breaking" section.**
- **A CI check:** if a pull request changes a format's version, or the game-code interface snapshot, it must have a "Breaking" entry and an example file for the old version. Otherwise it fails with a message saying what's missing.
- **A box in the pull request template:** "Does this change a file format, a save, or what game code can call?"

### C6. Lua's speed, measured and explained (decision 0055)
- **Scale cases:** the same work in a Rust, a Lust and a Lua component.
- **The native-code experiment:** Luau's native code generation (`mlua`'s `luau-jit`) on and off. It's kept only if it is faster **and** the determinism checks give identical results; then the sandbox tests also run with it on.
- **The manual's "which language when" page,** with the measured numbers.
- **The profiler's hint** on Lua components among a frame's biggest costs.

### End of the phase
- The three reviewers (decision 0027).
- The gate.

## Size

About 6,000–8,000 lines, mostly tests, example files and upgraders: the least likely things to cause harm themselves.

## Later, when many people's games depend on Solis

Kept here so nothing is designed against them:
- builds installed side by side, each project choosing its own;
- Preview features, off unless a project turns them on;
- behaviour versions, so a project keeps old game behaviour until it opts in;
- Stable and Preview update channels.
