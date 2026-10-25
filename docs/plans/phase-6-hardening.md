# Phase 6 hardening plan

The working plan for the hardening milestone between Phase 6 and Phase 7 ([decision 0042](../decisions/0042-a-hardening-milestone-before-phase-7.md); ROADMAP.md, "Phase 6 hardening"). Update it as milestones are built: tick them off here and in the README tracker.

**Aim:** a fix that breaks something is caught by a test before review, and we can see how much the tests catch.

**Gate:** the project lead reads the hardening report (coverage per crate, mutation results, hours fuzzed and what was found, the invariants that hold) and tries the playground once more.

**Starts when:** the project lead has tested the open pull requests (#26 onwards) and they're merged into `main` in order.

## Milestones

### H1. Merge the stack

- [ ] Merge the open pull requests into `main` in order, oldest first, once tested.
- [ ] After each merge, retarget the next pull request onto `main`, then let CI pass before the next merge.
- [ ] Fix anything that only shows up once merged, in its own pull request.

Done when `main` has everything up to Phase 6's third review round, and CI is green on it.

### H2. Whole-game invariant tests

A `proptest` generator for **game scripts**: a random list of steps run on a small level. The level has a floor, steps, a ramp, a moving platform, crates, a character and components that spawn, despawn and use random numbers. The steps include:

- walk or jump for N ticks;
- push a crate;
- spawn or despawn;
- save to slot A or B, and load from slot A or B;
- edit during play (move, delete, add or change a component);
- pause and step;
- swap the code;
- rebind;
- start or stop recording.

Rules checked after every script:

- **Load is the same as never stopping:** save at tick T and carry on to tick U. A game loaded from the save and run to U has the same checksums (world and game code).
- **Replays match:** a recording replays with every tick's checksum matching, saves and loads included.
- **Batched equals one by one:** characters moved together end where one-by-one moves put them; likewise any number of cores.
- **Stop restores:** after Stop, the area, the undo history and the selection are as they were before Play.
- **Nothing broken:** no panics, no NaN anywhere in the world, physics state sensible, and every component's values of the kinds its schema says.

A failure is shrunk to the smallest script and saved as a fixed regression test. In CI the generator runs a fixed number of cases (a budget kept under about two minutes). The weekly job runs many more.

### H3. Fuzzing every reader

One `cargo-fuzz` target per reader of a file a project can ship:

| Reader | Where |
|---|---|
| Save games | `editor::save_game::Save::read`, then load |
| Recordings | `input::Recording::read` |
| Input actions | `input::Bindings::read` |
| Areas and assemblies | `area::text::read_file`, then expand |
| Models | `assets::load_bytes` (glb, and gltf with its files) |
| The import cache | `asset_db::cache::decode` |
| Sidecars and the project file | `asset_db::sidecar`, `Project::open` |
| Game code | `script::compile` (Lust and Lua text) |

- Each target is seeded with real files (the samples, the playground, saves and recordings from the tests).
- Each runs for several hours locally before the milestone ends.
- A weekly CI job runs each target for a time box on the nightly compiler and keeps a corpus between runs.
- Every crash or hang becomes a regression test and a fix.

### H4. Measuring the tests

- **Coverage** with `cargo-llvm-cov`: a per-crate table in the report. Untested code in the core crates either gets tests or a note saying why not (code that only drives the GPU, say).
- **Mutation testing** with `cargo-mutants` on `world`, `physics`, `script`, `area`, and `editor`'s `play`, `save_game`, `game_code` and `play_input`. Every surviving mutation either gets a test that catches it or a note on why it can't matter.
- The report goes in `docs/reports/hardening-phase-6.md`, written for the project lead.

### H5. Checks that stay

- CI runs H2's invariant tests on every pull request, within their budget.
- A weekly workflow runs fuzzing and the larger invariant run, and opens an issue on a failure.
- CONTRIBUTING.md gets two rules: **every fix comes with a test that fails without it**, and **each phase's review includes coverage and surviving mutations** (with decision 0027).
- ARCHITECTURE.md's Checks section lists the new checks and how to run them locally.

## Notes

- Bugs found are fixed within the milestone, in pull requests grouped by kind, each with the test that found it.
- Fuzzing needs the nightly compiler. It is used only for the fuzz targets, never for building Solis.
- To decide with the project lead: how many pull requests may wait unmerged at once from now on. Today's stack is 59.
